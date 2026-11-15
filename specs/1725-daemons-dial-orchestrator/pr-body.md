<!-- claude-projects-attribution: {"github_login":"jaroslawherod"} -->
_Requested by **jaro** · [project thread](https://claude.ai/code/project/chan_01RyHdfvmnDGyHasLedRFLeQ?thread=cmsg_01RyHdfvmnDGyHasLedRFLeQ89eU7Xja8yPPBXmsk6YZbu)_

## Milestone M2 of 5 — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

**Deliverable**: stopping the orchestrator changes nothing for sessions or clients, the daemon is `online` again within 16 s of its return, and a stalled dialer no longer holds a link task or id reservation.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial -- outage stalled`; quickstart §A step 4
**Tasks**: T016–T020 · **Satisfies**: US4 scenarios 1–3; FR-008, FR-011; SC-003

What changed:
- `orchestrator/link.rs`: one `send_bounded(framed, item)` that is generic over the codec. It times out at `LIVENESS_DEADLINE` with `io::ErrorKind::TimedOut` and covers `refuse`, `Accepted`, `Welcome`, both `Refused` sends and the steady loop's ping and answer (FR-011).
- `orchestrator/mod.rs`: `Detached` is an orchestrator with no listener that serves the connections it is handed. The stalled-dialer tests need a pipe smaller than one frame, which a socket cannot give. The set-up `serve` and `Detached` share is now one `start`.
- New tests:
  - `tests/orchestrator_dial/outage.rs`: a 30 s outage on a paused clock leaves a real PTY session producing output and its viewing client untouched and logs the loss once; the daemon is `online` again within 16 s; a restart while linked re-links as the restored record.
  - `tests/orchestrator_dial/stalled.rs`: one stalled dialer for each of the four sends.
- `orchestrator_link/mod.rs` is unchanged: M1's task already met T019. Plan drift is recorded in `drift-M2.md`.
- Docs: `docs/daemon.md` and `docs/user-guide/daemon-configuration.md` describe the outage behaviour and the 16 s bound.

Evidence:
- The tests in `stalled.rs` failed before the fix: all 4 timed out with `the link task ends: nothing within 20s`.
- The tests in `outage.rs` passed on their first run, because M1 already had the behaviour. They were checked with three deliberate mutants, recorded in `tdd/cycle-log.md`, cycle 7:
  - logging every retry fails the log-once check (`left: 16`);
  - a 25 s retry delay fails the 16 s bound (`online 21.02s after the return`);
  - no redial after a loss fails scenarios 2 and 3.
- Quickstart §A step 4 was run by hand with the real binaries. After a 30 s stop the daemon was still running, logged `orchestrator link lost` once, and was listed `online` 0.6 s after the orchestrator came back.

## Agent review
- Review A (headless `autopilot-reviewer`, opus, high): CLEAN with 3 MINOR findings, all fixed:
  - a redundant `claim_failed`, since `Registry::closed` already releases the reservation;
  - a misleading `Detached` comment and duplicated set-up;
  - a thread-local log capture in the no-link test.
- Review B (conformance, sonnet): run by the orchestrator after this PR opens.

## Local gate
`scripts/gate.sh` (CI's order: fmt, clippy, nextest workspace 13187 passed, doctests, the root-only tests as a normal user) passed at b38d5f3d2. No `cfg(target_os)` arm, sandbox or client code changed; nothing visual.

Refs #1725

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvMUp5TdNVXzpBPtnAN
