<!-- claude-projects-attribution: {"github_login":"jaroslawherod"} -->
_Requested by **jaro** · [project thread](https://claude.ai/code/project/chan_01RyHdfvmnDGyHasLedRFLeQ?thread=cmsg_01RyHdfvmnDGyHasLedRFLeQVywo8xFvzy2nyDFzeziRa1)_

## Milestone M5 of 5 — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

**Deliverable**: killing the app's SSH connection shows the daemon `offline` within 10 s, and reopening it shows `online` within 16 s with the daemon pid unchanged, including after a `kill -9` and an immediate reopen.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1` (rootless `sshd`; quickstart §B steps 2, 4)
**Tasks**: T038–T040 · **Satisfies**: US3 scenarios 1–3; FR-009; SC-002

- Two `ssh_real_link` tests against a real `sshd`:
  - `SIGTERM` on the app's link `ssh`: the daemon is `offline` within 10 s and the directly linked host daemon stays `online`. While the forward is down, the remote `status.json` stays `not-linked` with one `since`, the log gains no line per retry, and the loss is logged once. After a reopen the daemon is `online` within 16 s, with the same pid and no duplicate record.
  - `SIGKILL` and an immediate reopen: the daemon links again within 16 s, and is still linked past the heartbeat deadline, so the link runs through the new forward. Same pid, one record.
- One in-process test (`orchestrator_dial/outage.rs`, paused clock). With a stale `orchestrator.sock`, the daemon is a quiet connect failure for 45 s: `NotLinked`, 0 attempts, and no log line at any level. A listener there then links within 16 s.
- The tests drive the harness orchestrator's hand-moved `TestClock` with real time (`RealTime`), so its heartbeat deadline runs out as a real one's does.
- T039 changed no product code. `prepare-link` already removes a stale socket (M4), and a refused connect was already quiet `NotLinked` with the 1, 2, 4, 8, 15 s backoff. See drift-M5.md and tdd/cycle-log.md cycle 15, which also records a mutation check.
- Docs: `docs/daemon.md` and `docs/user-guide/daemon-configuration.md` now give the offline and reconnect timings and the stale-forward troubleshooting.

## Agent review
- Review A (code-review, high, opus, headless): CLEAN with 3 MINOR, all fixed:
  - F1: the `SIGKILL` test now times the return on the daemon's side, as a new link with a later `since`.
  - F2: a comment now gives the right clock step.
  - F3: the docs no longer claim the test proves a live old session.
- Review B (conformance, sonnet): run by the orchestrator.

## Local gate
Local gate: scoped only, full gate left to CI. `scripts/gate.sh` passed fmt and clippy, then stopped at `cargo nextest`, which is not installed in the cloud container. Also run: `cargo test -p micold-daemon --test orchestrator_dial` passed 25. `--features ssh-real-runtime --test ssh_real ssh_real_link -- --test-threads=1` passed 6, against `openssh-server` installed in the sandbox. Nothing visible changed, so there was no visual pass.

Refs #1725

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvMABuGGKd9weFcWVxL
