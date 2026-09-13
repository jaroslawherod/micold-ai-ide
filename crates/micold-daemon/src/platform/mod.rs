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
//! teardown time, so its value is just the pid.

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

    /// Nothing beyond the direct child to terminate.
    pub fn terminate(&self) {}
}
