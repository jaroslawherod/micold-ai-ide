//! Unix process-tree teardown via the child's process group (plan W5, FR-036).

/// The process tree rooted at a session's PTY child, reachable through its process **group**.
///
/// `portable-pty` makes each PTY child its own session / process-group leader (it `setsid`s and
/// adopts the slave as its controlling terminal), so the child's pgid equals its pid and every
/// descendant that stayed in the group is reachable through it. Nothing has to happen at spawn.
pub struct ProcessTree {
    pid: u32,
}

impl ProcessTree {
    /// Remember the child's pid; the group is looked up at teardown.
    pub fn adopt(pid: u32) -> Self {
        Self { pid }
    }

    /// Signal the whole group: `kill(-pgid, SIGKILL)` reaps the child *and* its grandchildren in one
    /// call — no orphaned helper processes when a session is closed or the daemon shuts down.
    ///
    /// Best-effort and idempotent: a child that has already exited has no group, so `getpgid` fails
    /// and this is a harmless no-op. `SIGKILL` matches the direct-child teardown (`Child::kill`) so
    /// a session close leaves nothing behind.
    pub fn terminate(&self) {
        // SAFETY: `getpgid` takes no borrowed memory and only reads kernel process state; a failure
        // (the child is already gone) returns -1, which we filter out below.
        let pgid = unsafe { libc::getpgid(self.pid as libc::pid_t) };
        if pgid > 0 {
            // SAFETY: `kill` with a negative pid signals the whole process group. It borrows no
            // memory and cannot corrupt process state; an error (group already gone) is
            // intentionally ignored.
            unsafe {
                libc::kill(-pgid, libc::SIGKILL);
            }
        }
    }
}
