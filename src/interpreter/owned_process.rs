//! Platform ownership for subprocess trees. Opted-in ownership is configured
//! before the child can execute, including Windows suspended job assignment.
use process_wrap::tokio::{ChildWrapper, CommandWrap, KillOnDrop};
use std::io;
use std::ops::{Deref, DerefMut};

pub(super) fn executable_for_directory(
    program: &str,
    directory: Option<&str>,
) -> io::Result<std::path::PathBuf> {
    // Authorization resolves explicit executable paths against the parent's
    // cwd. Freeze that identity before Command::current_dir can change how a
    // relative path is resolved by the operating system.
    if directory.is_some()
        && (program.contains('/')
            || program.contains('\\')
            || program.as_bytes().get(1) == Some(&b':'))
    {
        std::fs::canonicalize(program)
    } else {
        Ok(program.into())
    }
}

pub(super) struct OwnedChild {
    inner: Box<dyn ChildWrapper>,
    owns_tree: bool,
    tree_closed: bool,
}

impl OwnedChild {
    // Wrapper waits include descendants. The WFL result belongs to the direct
    // child; once that child exits, close its remaining owned tree before
    // waiting for pipe EOF. Otherwise inherited pipes can keep completion open.
    pub(super) fn try_wait(&mut self) -> io::Result<Option<std::process::ExitStatus>> {
        let status = self.inner.inner_mut().try_wait()?;
        if status.is_some() {
            self.close_tree()?;
        }
        Ok(status)
    }

    pub(super) async fn wait(&mut self) -> io::Result<std::process::ExitStatus> {
        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    fn close_tree(&mut self) -> io::Result<()> {
        if self.owns_tree && !self.tree_closed {
            // An empty Unix group no longer has an ID to signal. Every
            // other termination failure must remain visible to callers.
            #[cfg(unix)]
            if let Err(error) = self.inner.start_kill()
                && error.raw_os_error() != Some(libc::ESRCH)
            {
                return Err(error);
            }
            #[cfg(not(unix))]
            self.inner.start_kill()?;
            self.tree_closed = true;
        }
        Ok(())
    }
}

impl Deref for OwnedChild {
    type Target = dyn ChildWrapper;
    fn deref(&self) -> &Self::Target {
        self.inner.as_ref()
    }
}

impl DerefMut for OwnedChild {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner.as_mut()
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.close_tree();
    }
}

pub(super) fn spawn(
    mut command: tokio::process::Command,
    owns_tree: bool,
    kill_on_drop: bool,
) -> io::Result<OwnedChild> {
    if !owns_tree {
        command.kill_on_drop(kill_on_drop);
        return command.spawn().map(|inner| OwnedChild {
            inner: Box::new(inner),
            owns_tree: false,
            tree_closed: false,
        });
    }

    #[cfg(target_os = "linux")]
    {
        // A nested WFL driver creates its own child group. Parent-death
        // signalling closes that nested ownership chain when the outer driver
        // is forcibly terminated before WFL finally clauses can execute.
        // SAFETY: this pre-exec hook calls only async-signal-safe syscalls,
        // performs no allocation, and guards the race with parent death.
        let parent_pid = std::process::id() as libc::pid_t;
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL) != 0 {
                    return Err(io::Error::last_os_error());
                }
                if libc::getppid() != parent_pid {
                    libc::_exit(1);
                }
                Ok(())
            });
        }
    }

    let mut command = CommandWrap::from(command);
    command.wrap(KillOnDrop);
    #[cfg(unix)]
    command.wrap(process_wrap::tokio::ProcessGroup::leader());
    #[cfg(windows)]
    command.wrap(process_wrap::tokio::JobObject);
    command.spawn().map(|inner| OwnedChild {
        inner,
        owns_tree: true,
        tree_closed: false,
    })
}
