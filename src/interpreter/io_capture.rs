//! Output capture for nested `execute file` runs.
//!
//! `display` and `print` are plain fn pointers (`NativeFunction`) with no
//! access to interpreter state, so capture is routed through a thread-local
//! stack instead of an `Interpreter` field. This is sound because the
//! interpreter — including nested child interpreters started by
//! `execute file` — runs on a single thread, and the parent is suspended
//! while a child runs. Only output produced on the interpreter thread is
//! captured, which is true for all program-output sites today.

use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CAPTURE_STACK: RefCell<Vec<Rc<RefCell<String>>>> = const { RefCell::new(Vec::new()) };
}

/// RAII guard that pops the capture buffer when dropped, so capture ends
/// correctly even when execution unwinds through `?`.
pub(crate) struct CaptureGuard(());

impl Drop for CaptureGuard {
    fn drop(&mut self) {
        CAPTURE_STACK.with(|stack| {
            stack.borrow_mut().pop();
        });
    }
}

/// Push a capture buffer; program output lines are appended to it until the
/// returned guard is dropped. Buffers nest: only the innermost one receives
/// output, giving correct semantics when a captured file itself captures.
pub(crate) fn push_capture(buffer: Rc<RefCell<String>>) -> CaptureGuard {
    CAPTURE_STACK.with(|stack| stack.borrow_mut().push(buffer));
    CaptureGuard(())
}

/// Emit one line of program output: to the innermost active capture buffer on
/// this thread if there is one, otherwise to stdout.
pub(crate) fn emit_line(line: &str) {
    let captured = CAPTURE_STACK.with(|stack| {
        if let Some(buffer) = stack.borrow().last() {
            let mut buffer = buffer.borrow_mut();
            buffer.push_str(line);
            buffer.push('\n');
            true
        } else {
            false
        }
    });
    if !captured {
        println!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
