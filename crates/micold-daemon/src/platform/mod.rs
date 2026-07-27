//! The one process-supervision platform abstraction (plan W5, FR-036, task T061).
//!
//! Tearing a session down must reap its **whole process tree**, not just the direct PTY child: a
//! `claude` (or shell) session routinely forks helpers, and orphaning them would leak processes on
//! every close, restart, or daemon shutdown. The mechanism is platform-specific — `killpg` on Unix,
//! a job object on Windows — so it lives behind this single seam (Constitution Principle VI:
//! platform differences confined, functional behaviour equivalent).

#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use unix::terminate_process_tree;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::terminate_process_tree;

/// Fallback for exotic targets with neither Unix signals nor Windows job objects: reaping the direct
/// child (the caller's own `child.kill()`) is the best available, so this is a no-op.
#[cfg(not(any(unix, windows)))]
pub fn terminate_process_tree(_pid: u32) {}
