//! Output capture for nested `execute file` runs.
//!
//! Native output functions are plain fn pointers (`NativeFunction`) with no
//! access to interpreter state, so capture is routed through a thread-local
//! stack instead of an `Interpreter` field. This is sound for serial
//! execution because the interpreter — including nested child interpreters
//! started by `execute file` — runs on a single thread, and the parent is
//! suspended while a child runs. Only output produced on the interpreter
//! thread is captured. Async output checks capture here before submitting
//! uncaptured text to the shared, bounded stdout worker.
//!
//! Concurrent handlers (`main loop concurrently:`) interleave on that one
//! thread instead of suspending each other, so the stack is part of the
//! per-handler `RunState` swap: `swap_stack` installs a handler's own stack
//! for the duration of each poll (#642). Guards remove their buffer by
//! identity rather than popping blindly, so a guard that drops while its
//! handler's stack is parked (handler future dropped mid-suspend) or after
//! out-of-order completion cannot remove another capture's buffer.

use std::cell::RefCell;
use std::future::Future;
use std::io::{self, Write};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, LazyLock};
use tokio::sync::{mpsc, oneshot};

use super::error::RuntimeError;
use super::value::Value;

#[derive(Debug)]
struct OutputWrite {
    text: Arc<str>,
    completed: oneshot::Sender<io::Result<()>>,
}

static STDOUT_WRITER: LazyLock<io::Result<mpsc::Sender<OutputWrite>>> =
    LazyLock::new(|| start_output_worker(stdout_writer()?));

fn stdout_writer() -> io::Result<Box<dyn Write + Send>> {
    let stdout = io::stdout();
    // A raw duplicate avoids holding Rust's global stdout lock while a pipe
    // stalls: process cleanup must not wait for this detached worker.
    #[cfg(unix)]
    {
        use std::os::fd::AsFd;
        Ok(Box::new(std::fs::File::from(
            stdout.as_fd().try_clone_to_owned()?,
        )))
    }
    #[cfg(windows)]
    {
        use std::io::IsTerminal;
        use std::os::windows::io::AsHandle;
        // Keep Rust's UTF-8 to UTF-16 conversion for Windows consoles.
        if stdout.is_terminal() {
            return Ok(Box::new(stdout));
        }
        Ok(Box::new(std::fs::File::from(
            stdout.as_handle().try_clone_to_owned()?,
        )))
    }
    #[cfg(not(any(unix, windows)))]
    {
        Ok(Box::new(stdout))
    }
}

fn start_output_worker(
    mut writer: impl Write + Send + 'static,
) -> io::Result<mpsc::Sender<OutputWrite>> {
    let (sender, mut receiver) = mpsc::channel::<OutputWrite>(1);
    // A single process-lifetime thread avoids unabortable Tokio blocking work
    // keeping runtime shutdown alive when the stdout reader stops draining.
    std::thread::Builder::new()
        .name("wfl-stdout".into())
        .spawn(move || {
            while let Some(output) = receiver.blocking_recv() {
                if output.completed.is_closed() {
                    continue;
                }
                let result = writer
                    .write_all(output.text.as_bytes())
                    .and_then(|()| writer.flush());
                let _ = output.completed.send(result);
            }
        })?;
    Ok(sender)
}

async fn emit_async(render: impl FnOnce() -> Arc<str>, capture: bool) -> io::Result<()> {
    if capture {
        let active_buffer = CAPTURE_STACK.with(|stack| stack.borrow().last().cloned());
        if let Some(buffer) = active_buffer {
            buffer.borrow_mut().push_str(&render());
            return Ok(());
        }
    }
    let sender = STDOUT_WRITER
        .as_ref()
        .map_err(|error| io::Error::new(error.kind(), error.to_string()))?;
    // Reserve before formatting/copying output, so backpressure also bounds
    // additional payload allocation to the active write and one queued write.
    let permit = sender.reserve().await.map_err(io::Error::other)?;
    let (completed, result) = oneshot::channel();
    permit.send(OutputWrite {
        text: render(),
        completed,
    });
    result.await.map_err(io::Error::other)?
}

pub(crate) type OutputFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, RuntimeError>> + 'a>>;

/// Route resolved output natives asynchronously; captures never leave this thread.
pub(crate) fn route<'a>(name: &str, args: &'a [Value]) -> Option<OutputFuture<'a>> {
    match name {
        "write_stdout" => Some(Box::pin(async move {
            let text = crate::stdlib::core::stdout_text(args)?;
            emit_async(|| Arc::clone(text), true)
                .await
                .map_err(|error| output_error("write_stdout", error))?;
            Ok(Value::Null)
        })),
        "print" | "display" => Some(Box::pin(async move {
            emit_async(
                || {
                    let mut text = crate::stdlib::core::format_print_args(args);
                    text.push('\n');
                    Arc::from(text)
                },
                true,
            )
            .await
            .map_err(|error| output_error("display/print", error))?;
            Ok(Value::Null)
        })),
        _ => None,
    }
}

/// Server notices keep their existing uncaptured stdout destination.
pub(crate) async fn stdout_line(line: &str) -> Result<(), RuntimeError> {
    emit_async(|| Arc::from(format!("{line}\n")), false)
        .await
        .map_err(|error| output_error("server notice", error))
}

fn output_error(name: &str, error: io::Error) -> RuntimeError {
    RuntimeError::new(format!("{name} could not write to stdout: {error}"), 0, 0)
}

thread_local! {
    static CAPTURE_STACK: RefCell<Vec<Rc<RefCell<String>>>> = const { RefCell::new(Vec::new()) };
}

/// RAII guard that removes its capture buffer when dropped, so capture ends
/// correctly even when execution unwinds through `?`.
pub(crate) struct CaptureGuard(Rc<RefCell<String>>);

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURE_STACK.with(|stack| {
            let mut stack = stack.borrow_mut();
            if let Some(pos) = stack.iter().rposition(|b| Rc::ptr_eq(b, &self.0)) {
                stack.remove(pos);
            }
        });
    }
}

/// Push a capture buffer; program output is appended to it until the
/// returned guard is dropped. Buffers nest: only the innermost one receives
/// output, giving correct semantics when a captured file itself captures.
pub(crate) fn push_capture(buffer: Rc<RefCell<String>>) -> CaptureGuard {
    CAPTURE_STACK.with(|stack| stack.borrow_mut().push(Rc::clone(&buffer)));
    CaptureGuard(buffer)
}

/// Swap this thread's capture stack with `other`. Used by the concurrent
/// handler `RunState` swap so each handler sees only its own capture stack
/// while polled, with the ambient stack parked (and restored) around it.
pub(crate) fn swap_stack(other: &mut Vec<Rc<RefCell<String>>>) {
    CAPTURE_STACK.with(|stack| std::mem::swap(&mut *stack.borrow_mut(), other));
}

/// Clone of the current capture stack: the initial capture context a new
/// concurrent handler inherits, so handler output still reaches an enclosing
/// `execute file` capture (the buffers are shared, the stack itself is not).
pub(crate) fn snapshot_stack() -> Vec<Rc<RefCell<String>>> {
    CAPTURE_STACK.with(|stack| stack.borrow().clone())
}

/// Emit exact text to the current capture, or write and flush stdout.
pub(crate) fn emit_text(text: &str) -> io::Result<()> {
    let active_buffer = CAPTURE_STACK.with(|stack| stack.borrow().last().cloned());
    if let Some(buffer) = active_buffer {
        buffer.borrow_mut().push_str(text);
        Ok(())
    } else {
        // Synchronous native-function API for embedders; WFL dispatch uses route().
        let mut stdout = io::stdout().lock();
        stdout.write_all(text.as_bytes())?;
        stdout.flush()
    }
}

/// Emit one line of program output: to the innermost active capture buffer on
/// this thread if there is one, otherwise to stdout.
pub(crate) fn emit_line(line: &str) {
    let active_buffer = CAPTURE_STACK.with(|stack| stack.borrow().last().cloned());
    if let Some(buffer) = active_buffer {
        let mut buffer = buffer.borrow_mut();
        buffer.push_str(line);
        buffer.push('\n');
    } else {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct GatedWriter {
        bytes: Arc<Mutex<Vec<u8>>>,
        started: Option<tokio::sync::oneshot::Sender<()>>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl Write for GatedWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if let Some(started) = self.started.take() {
                let _ = started.send(());
                self.release.recv().map_err(io::Error::other)?;
            }
            self.bytes.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn bounded_worker_skips_cancelled_queue_entries_after_the_writer_resumes() {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let (started, observed) = tokio::sync::oneshot::channel();
        let (release, gate) = std::sync::mpsc::channel();
        let sender = start_output_worker(GatedWriter {
            bytes: Arc::clone(&bytes),
            started: Some(started),
            release: gate,
        })
        .unwrap();
        let (completed, first) = tokio::sync::oneshot::channel();
        sender
            .send(OutputWrite {
                text: Arc::from("first"),
                completed,
            })
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(5), observed)
            .await
            .unwrap()
            .unwrap();

        let (completed, cancelled) = tokio::sync::oneshot::channel();
        sender
            .try_send(OutputWrite {
                text: Arc::from("abandoned"),
                completed,
            })
            .unwrap();
        assert!(matches!(
            sender.try_reserve(),
            Err(tokio::sync::mpsc::error::TrySendError::Full(()))
        ));
        drop(cancelled);
        let (completed, last) = tokio::sync::oneshot::channel();
        let final_write = tokio::spawn(async move {
            sender
                .send(OutputWrite {
                    text: Arc::from("last"),
                    completed,
                })
                .await
                .unwrap();
        });
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), first)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        final_write.await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), last)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(*bytes.lock().unwrap(), b"firstlast");
    }

    struct FlushFailure;

    impl Write for FlushFailure {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }

    #[tokio::test]
    async fn worker_reports_flush_failures_to_the_waiting_caller() {
        let sender = start_output_worker(FlushFailure).unwrap();
        let (completed, result) = tokio::sync::oneshot::channel();
        sender
            .send(OutputWrite {
                text: Arc::from("text"),
                completed,
            })
            .await
            .unwrap();
        let error = tokio::time::timeout(Duration::from_secs(5), result)
            .await
            .unwrap()
            .unwrap()
            .unwrap_err();
        assert_eq!(error.to_string(), "flush failed");
    }

    #[test]
    fn capture_collects_lines_and_nests() {
        let outer = Rc::new(RefCell::new(String::new()));
        let _outer_guard = push_capture(Rc::clone(&outer));
        emit_line("outer one");
        {
            let inner = Rc::new(RefCell::new(String::new()));
            let _inner_guard = push_capture(Rc::clone(&inner));
            emit_line("inner");
            assert_eq!(*inner.borrow(), "inner\n");
        }
        emit_line("outer two");
        assert_eq!(*outer.borrow(), "outer one\nouter two\n");
    }
}
