//! Unix process-tree teardown via the child's process group (plan W5, FR-036).

/// The process tree rooted at a session's PTY child, reachable through its process **group**.
///
/// `portable-pty` makes each PTY child its own session / process-group leader (it `setsid`s and
/// adopts the slave as its controlling terminal), so the child's pgid equals its pid and every
/// descendant that stayed in the group is reachable through it. Nothing has to happen at spawn.
///
/// **A pid is only a name for the child until the child is reaped.** Up to then the kernel keeps it
/// reserved — for a zombie as much as for a running process. After it, the pid can be handed to any
/// new process, and a teardown that still signalled through it would `SIGKILL` whatever group that
/// stranger leads. So the pid is dropped at the reap ([`Self::leader_reaped`]), and a teardown after
/// that signals nothing. Descendants that outlived a reaped child are therefore left alone here;
/// before this they were left alone too, because `getpgid` on a reaped pid fails, except in exactly
/// the case where the pid had been reused and it succeeded.
pub struct ProcessTree {
    pid: Option<u32>,
}

impl ProcessTree {
    /// Remember the child's pid; the group is looked up at teardown.
    pub fn adopt(pid: u32) -> Self {
        Self { pid: Some(pid) }
    }

    /// The child has been reaped: forget its pid before anything can reuse it.
    ///
    /// The caller must call this under the same lock as the reap, and call [`Self::terminate`] under
    /// it too — otherwise a reap can land between reading the pid and signalling it.
    pub fn leader_reaped(&mut self) {
        self.pid = None;
    }

    /// The pid [`Self::terminate`] would signal the group of, if any.
    #[cfg(test)]
    pub(crate) fn signal_target(&self) -> Option<u32> {
        self.pid
    }

    /// Signal the whole group: `kill(-pgid, SIGKILL)` reaps the child *and* its grandchildren in one
    /// call — no orphaned helper processes when a session is closed or the daemon shuts down.
    ///
    /// Best-effort and idempotent: once the child is reaped there is no pid left to signal, and a
    /// child that has exited but not been reaped still names its own group. `SIGKILL` matches the
    /// direct-child teardown (`Child::kill`) so a session close leaves nothing behind.
    pub fn terminate(&self) {
        let Some(pid) = self.pid else {
            return;
        };
        // SAFETY: `getpgid` takes no borrowed memory and only reads kernel process state; a failure
        // returns -1, which we filter out below.
        let pgid = unsafe { libc::getpgid(pid as libc::pid_t) };
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
