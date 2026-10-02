# Contract: asking the session service to stop

How a stop becomes orderly, so that FR-002 holds for a restart, an update, a logout and a reboot.
Research: [R14](../research.md#r14-what-makes-a-stop-orderly).

## 1. The request

`platform::stop_requested()` is a future that completes once when the service is asked to stop.
Both accept loops select on it beside the idle timer; when it completes they run
`unwind(StopReason::Requested)` and the process exits as it does after an idle stop.

| Platform | What raises it |
|---|---|
| Linux, macOS, the container | `SIGTERM`, `SIGINT` or `SIGHUP` (`tokio::signal::unix`) |
| Windows | The named event `Local\Micold.Daemon.Stop.<user SID>` is set, or the service's hidden window receives `WM_ENDSESSION` |

A second request while unwinding changes nothing.

## 2. Who sends it

| Stop | Sender | Before this feature |
|---|---|---|
| **Restart service** on Linux, macOS | `terminate_daemon` sends `SIGTERM` (unchanged) | The process ended at once |
| **Restart service** on Windows | `terminate_daemon` sets the event, waits up to 5 s for the process to exit, then falls back to `TerminateProcess` | `TerminateProcess` at once |
| Update on Windows | The installer script sets the event and waits up to 5 s before `Stop-Process` | `Stop-Process -Force` |
| Logout, reboot on Linux, macOS | The session manager's `SIGTERM` | The process ended at once |
| Logout, reboot on Windows | `WM_ENDSESSION` to the hidden window, which waits for the unwind before returning | The process was ended |
| Sandbox stop | The runtime's `SIGTERM` to pid 1 | Ignored; killed after the grace period |

## 3. The Windows event

- Created by the service at start, manual-reset, with the same owner-only DACL as its pipe, so only
  the same user can set it.
- The name carries the user's SID, as the pipe's name does, so two users' services do not share it.
- A sender that cannot open the event (an older service, or none) goes straight to its fallback.

## 4. The save in `unwind`

A new step, before the live sessions are taken and dropped:

1. Capture every covered terminal from its live `Term`. The processes are not waited for: they are
   about to be killed. This differs from the capture at a process end (R4), which follows the
   reader's join and so holds every byte the process wrote. Here the process is still running, so
   the snapshot holds what the `Term` has parsed at that moment; bytes still in flight are not in
   it. That is what story 1 scenario 8 asks of an orderly stop, and nothing the user saw is lost:
   the client draws from the same `Term`.
2. With saving on, encode the snapshots in parallel on the blocking pool; each write takes the
   store's mutex in turn. A terminal whose content equals its last written file is not written
   (FR-004). The whole step is bounded at 3 s.
3. A terminal whose save did not finish in time keeps its previous file, which is at most 60 s old.

The endpoint is released after `unwind`, as today, so a new service cannot start, and so cannot
load a file, before the old one has finished saving.

The step runs for every reason `unwind` is called: the idle stop (story 1 scenario 8) and a
request.

## 5. What is not orderly

A crash, `SIGKILL`, a power loss, `TerminateProcess` after the 5 s wait, and a Windows logout that
ends the process before the window is told. The outcome is story 1 scenario 7: the history is
present up to at most 60 s before the kill (SC-002).

## 6. Tests

| Test | Platform |
|---|---|
| A service process gets `SIGTERM` while a fake CLI has printed 200 lines and no periodic save is due; a second service on the same directories restores all 200 | Unix |
| The same with the event set | Windows |
| `WM_ENDSESSION` sent to the window raises the request | Windows |
| `terminate_daemon` against a service that ignores the event falls back after 5 s | Windows |
| A save that blocks does not hold `unwind` longer than 3 s | all |
| `<runtime> stop` on a sandbox with a printing session; the host file holds the last lines | sandbox real-runtime suite |

A real Windows logout cannot be run in CI; it is listed in quickstart Part B as a manual check on a
Windows machine.
