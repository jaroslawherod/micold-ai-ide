//! The one process-supervision platform abstraction (plan W5, FR-036, task T061).
//!
//! Tearing a session down must reap its **whole process tree**, not just the direct PTY child: a
//! `claude` (or shell) session routinely forks helpers, and orphaning them would leak processes on
//! every close, restart, or daemon shutdown. The mechanism is platform-specific — `killpg` on Unix,
//! a job object on Windows — so it lives behind this single seam (Constitution Principle VI:
//! platform differences confined, functional behaviour equivalent).
//!
//! It is a value rather than a free function because Windows needs one: a job object has to be
//! created and the child put in it *at spawn*, before the child has started anything worth reaping,
//! and the job is then held for the life of the session. Unix derives everything from the pid at
//! teardown time, so its value is just the pid — held only until the child is reaped, because after
//! that the pid is free for the kernel to give to someone else. For the same reason exit is observed
//! through it ([`ProcessTree::exit_status`]): on Unix that leaves the exited child unreaped, so its
//! descendants can still be reached at teardown, as a Windows job reaches them.
//!
//! [`PtySession`](crate::supervisor::PtySession) keeps it under the same lock as the child, so the
//! reap and the forgetting are one step and a teardown can never signal between them.

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::ProcessTree;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::ProcessTree;

/// Fallback for exotic targets with neither Unix signals nor Windows job objects: reaping the direct
/// child (the caller's own `child.kill()`) is the best available, so teardown is a no-op.
#[cfg(not(any(unix, windows)))]
pub struct ProcessTree;

#[cfg(not(any(unix, windows)))]
impl ProcessTree {
    /// Nothing to adopt.
    pub fn adopt(_pid: u32) -> Self {
        Self
    }

    /// The child's exit status if it has exited.
    pub fn exit_status(
        &self,
        child: &mut dyn portable_pty::Child,
    ) -> std::io::Result<Option<portable_pty::ExitStatus>> {
        child.try_wait()
    }

    /// Nothing to forget.
    pub fn leader_reaped(&mut self) {}

    /// Nothing beyond the direct child to terminate.
    pub fn terminate(&self) {}
}

/// Write `bytes` to `dir/file` so that only the user running the service can read it, creating
/// `dir` owner-only as well, and return the file's path (feature 034, FR-007).
///
/// A session's tool-server binding file carries its bearer credential, so another local account
/// must not be able to read it: `0700` / `0600` on Unix, a protected DACL with one ACE for the
/// current user on Windows. The bytes go to a temporary file that is made owner-only before it is
/// renamed over `file`, so the credential is never readable under a wider mode, and a rewrite
/// replaces the old content atomically. The work is `micold_core::owner_only::write`, which the
/// saved terminal history shares (feature 041, R8).
pub fn write_owner_only(
    dir: &std::path::Path,
    file: &str,
    bytes: &[u8],
) -> std::io::Result<std::path::PathBuf> {
    micold_core::owner_only::write(dir, file, bytes)
}
