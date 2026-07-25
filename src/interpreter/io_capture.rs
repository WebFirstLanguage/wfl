//! Output capture for nested `execute file` runs.
//!
//! `display` and `print` are plain fn pointers (`NativeFunction`) with no
//! access to interpreter state, so capture is routed through a thread-local
//! stack instead of an `Interpreter` field. This is sound for serial
//! execution because the interpreter — including nested child interpreters
//! started by `execute file` — runs on a single thread, and the parent is
//! suspended while a child runs. Only output produced on the interpreter
//! thread is captured, which is true for all program-output sites today.
//!
//! Concurrent handlers (`main loop concurrently:`) interleave on that one
//! thread instead of suspending each other, so the stack is part of the
//! per-handler `RunState` swap: `swap_stack` installs a handler's own stack
//! for the duration of each poll (#642). Guards remove their buffer by
//! identity rather than popping blindly, so a guard that drops while its
//! handler's stack is parked (handler future dropped mid-suspend) or after
//! out-of-order completion cannot remove another capture's buffer.

use std::cell::RefCell;
use std::rc::Rc;

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

/// Push a capture buffer; program output lines are appended to it until the
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
