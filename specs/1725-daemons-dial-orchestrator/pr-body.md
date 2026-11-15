<!-- claude-projects-attribution: {"github_login":"jaroslawherod"} -->
_Requested by **jaro** · [project thread](https://claude.ai/code/project/chan_01RyHdfvmnDGyHasLedRFLeQ?thread=cmsg_01RyHdfvmnDGyHasLedRFLeQ2cgV2rwRgxSy8KUemnrkPr)_

## Milestone M3 of 5 — daemons dial the orchestrator (specs/1725-daemons-dial-orchestrator)

**Deliverable**: an orchestrator that refuses every attempt gets at most one attempt per capped delay, and `micold-daemon remote status` shows the link state and refusal reason.
**Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator_dial -- refused status`; `scripts/build-lock.sh cargo test -p micold-core status`
**Tasks**: T021–T027 · **Satisfies**: US5 scenarios 1–3; FR-007, FR-012; SC-004

What changed:
- `micold-core` `ssh/status.rs`: `RemoteStatus.link: Option<LinkStatusReport>` (`{ "state", "since", "detail" }`, `serde(default, skip_serializing_if)`). `STATUS_VERSION` is unchanged and an older file still parses. `state` is a word, not an enum, so an older reader that meets a newer state still parses the file and keeps the replace offer. `orchestrator/dial.rs`: `LinkStatus::word`.
- `orchestrator_link/mod.rs`: the status cell is now `LinkState` (status, the refusal's sentence, `since`, attempt count). A retry refused the same way keeps its `since` and logs nothing. A refused daemon whose orchestrator goes away logs `orchestrator link not linked` once. Refusals already waited `jittered(after_refusal())` from M1. `spawn_as` (`#[doc(hidden)]`) lets a test announce another protocol version.
- `runtime_status.rs` rewrites `status.json` when the session count or the link report changes. `server.rs` hands it the link's cell. `remote status` prints the JSON line (with `link`) on stdout and `orchestrator link: <state>: <detail>` on stderr (`remote::status_lines`).
- New tests:
  - `tests/orchestrator_dial/refused.rs`: on a paused 60 s clock, with each refusal way repeated twice, every gap between attempts is 15–18 s and there are at most 5 attempts, with one log line per change. Against the real orchestrator, an older daemon is listed `needs-update`, retries at the cap and logs once. A refused duplicate leaves the first link online. A lost link is re-dialled after 1–1.2 s once the backoff has grown. A refused link whose orchestrator goes away is logged once.
  - `tests/orchestrator_dial/status.rs`: `status.json` and `remote status` carry the state and the refusal sentence. Without a link there is no `link` object.
- Docs: `docs/user-guide/daemon-configuration.md` covers the retry delays, the refusal reasons and `remote status`. `docs/daemon.md` covers the cell, `status.json` and the log lines.
- Plan drift is recorded in `drift-M3.md`.

Evidence (`tdd/cycle-log.md`, cycles 8–10):
- T021 was red before the fix: 6 compile errors.
- The `status.rs` link test was red with the seams stubbed: `status.json is written: nothing within 20s`.
- The `refused.rs` back-off tests passed on their first run, because M1 already had the delays. They were checked with mutants:
  - refusal delay `after_failure`: `1s between attempts`;
  - logging every retry: `no line per retry`;
  - no reset on `Welcome`: `re-dialled 15s after the loss`.

## Agent review
- Review A (headless `autopilot-reviewer`, opus, high): CLEAN with 3 MINOR findings, all fixed in 11cf6ef6a:
  - a refused-to-absent change was not logged;
  - `spawn_as` was a public seam (now `#[doc(hidden)]`);
  - the SC-004 log check could not catch a line per retry.
- Review B (conformance, sonnet): run by the orchestrator.

## Local gate
`scripts/gate.sh` at 11cf6ef6a:
- passed: fmt, clippy (core, then workspace, `-D warnings`), nextest workspace (13198 passed, 23 skipped) and doctests;
- failed: one check in `scripts/tests/autopilot.test.sh` (`facts: caps the callers shown and flags truncation`: the review-facts caller scan found 0 callers in this container). This diff does not touch `scripts/` or `.claude/`, so CI is the gate for that check.

Not run: no `cfg(target_os)` arm, sandbox or client code changed, and nothing visible changed.

Refs #1725

🤖 Generated with [Claude Code](https://claude.com/claude-code)

https://claude.ai/code/session_01NCdYvNysMGX37erHLqkbpp
