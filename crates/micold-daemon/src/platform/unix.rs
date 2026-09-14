//! Unix process-tree teardown via the child's process group (plan W5, FR-036).

use std::io;
use std::os::unix::process::ExitStatusExt;

use portable_pty::{Child, ExitStatus};

/// `si_code` values `waitid` reports for a child that has terminated — `<sys/signal.h>` defines them
/// identically on Linux and macOS, but the `libc` crate carries them for Linux only.
const CLD_EXITED: libc::c_int = 1;
const CLD_KILLED: libc::c_int = 2;
const CLD_DUMPED: libc::c_int = 3;

/// The process tree rooted at a session's PTY child, reachable through its process **group**.
///
/// `portable-pty` makes each PTY child its own session / process-group leader (it `setsid`s and
/// adopts the slave as its controlling terminal), so the child's pgid equals its pid — a session
/// leader cannot move to another group — and every descendant that stayed in the group is reachable
/// through it. Nothing has to happen at spawn.
///
/// **A pid is only a name for the child until the child is reaped.** Up to then the kernel keeps it
/// reserved — for a zombie as much as for a running process — so no other process can have it, and
/// no other group can be numbered by it. After it, the pid can be handed to any new process, and a
/// teardown that still signalled through it would `SIGKILL` whatever group that stranger leads.
///
/// Two consequences:
///
/// - Noticing the child has exited must not reap it ([`Self::exit_status`]). A child that ends on
///   its own can leave descendants running, and the zombie's pid is the only way left to reach
///   them; reaping at the liveness check would leave them for good, where a Windows job ends them at
///   teardown. So the zombie is kept until teardown, which signals the group and then reaps.
/// - After the reap the pid is forgotten ([`Self::leader_reaped`]), and a teardown after that
///   signals nothing.
pub struct ProcessTree {
    pid: Option<u32>,
}

impl ProcessTree {
    /// Remember the child's pid; the group is reached through it at teardown.
    pub fn adopt(pid: u32) -> Self {
        Self { pid: Some(pid) }
    }

    /// The child's exit status if it has exited, `None` while it runs — leaving an exited child
    /// **unreaped**, so its pid still names its group for [`Self::terminate`].
    ///
    /// `waitid(…, WNOWAIT)` reports the exit without collecting it; the eventual `wait` by the
    /// caller's teardown sees the same status. Once the child has been reaped there is no pid left,
    /// and the status comes from `child`, which caches what its own reap collected.
    pub fn exit_status(&self, child: &mut dyn Child) -> io::Result<Option<ExitStatus>> {
        let Some(pid) = self.pid else {
            return child.try_wait();
        };
        // SAFETY: `siginfo_t` is a plain C struct for which all-zero bytes are a valid value; it is
        // only an out-parameter here.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: `waitid` writes only into `info`, which lives for the whole call. `WNOHANG` makes
        // it return at once and `WNOWAIT` leaves the child waitable.
        let rc = unsafe {
            libc::waitid(
                libc::P_PID,
                pid as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if rc == -1 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: `info` was filled by a successful `waitid` (or left zeroed when nothing had
        // changed, which `WNOHANG` reports as a zero `si_pid`), so the child-status fields are the
        // ones in use.
        let (reporter, status) = unsafe { (info.si_pid(), info.si_status()) };
        if reporter == 0 {
            return Ok(None);
        }
        // Re-encode as the `wait` status `std` decodes, so the result is exactly what reaping would
        // have returned.
        let raw = match info.si_code {
            CLD_EXITED => (status & 0xff) << 8,
            CLD_KILLED => status,
            CLD_DUMPED => status | 0x80,
            other => {
                return Err(io::Error::other(format!(
                    "waitid reported si_code {other} for an exited child"
                )));
            }
        };
        Ok(Some(std::process::ExitStatus::from_raw(raw).into()))
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

    /// Signal the whole group: `kill(-pgid, SIGKILL)` ends the child *and* its descendants in one
    /// call — no orphaned helper processes when a session is closed or the daemon shuts down.
    ///
    /// The group is addressed by the pid directly rather than through `getpgid`: they are equal (see
    /// the type), and macOS does not answer `getpgid` for a zombie, which is exactly the child whose
    /// descendants still need reaching.
    ///
    /// Best-effort and idempotent: once the child is reaped there is no pid left to signal, and a
    /// group with nothing left in it just reports an error, ignored. `SIGKILL` matches the
    /// direct-child teardown (`Child::kill`) so a session close leaves nothing behind.
    pub fn terminate(&self) {
        let Some(pid) = self.pid else {
            return;
        };
        // SAFETY: `kill` with a negative pid signals the whole process group. It borrows no memory
        // and cannot corrupt process state; an error (group already empty) is intentionally ignored.
        unsafe {
            libc::kill(-(pid as libc::pid_t), libc::SIGKILL);
        }
    }
}
