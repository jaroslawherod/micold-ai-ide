# Autopilot ledger — 035-report-missing-include-script

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: GitHub issue #435, "An environment-include script path that does not exist is never
  reported", via `specs/011-env-include-script/bugs/BUG-006.md`. Phase 0 routed it here
  (`specs/011-env-include-script/bugs/BUG-006.autopilot.md`, decision D2): the fix is behaviour
  feature 011 never intended.
- **Kind**: feature (from bug BUG-006)
- **Worktree branch**: fix/github-issues
- **Started**: 2026-09-29
- **Phase**: 1-spec
- **Next step**: the orchestrator waits for PR 1's `ci complete` and merges it; then Phase 2 (clarify FR-004, FR-007, FR-008).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Deliverable | PR | Status |
|---|---|---|---|---|

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | 1-spec | Does the spec settle the issue's three open points (BUG-006 ledger D3–D5)? | No. Each is an open `[NEEDS CLARIFICATION]` marker in spec.md for Phase 2: FR-004 (D3, when Settings checks the path and whether save is blocked), FR-007 (D4, reporting beyond Settings), FR-008 (D5, recovery). | agent-resolved | orchestrator instruction; SKILL.md Phase 1 |
| D2 | 1-spec | Spec review round 1 | CHANGES: 5 MAJOR, 5 MINOR, all verified against the code and fixed. `~` is taken literally as resolution does (`env_include.rs:421`, no expansion); relative paths get their own "relative, not checked" indication, because resolution's directory depends on the active session (`shell/env_include.rs` `default_resolution_cwd`); "readable file" defined as a regular readable file, with both notes shown for a directory while on; FR-014 added for "file exists now but the cached attempt failed" (no re-source on show); FR-008 re-posed so "report only" is a valid answer; FR-006/SC-004 given a 2 s bound and a hung-check edge case. | agent-resolved | reviewer subagent round 1 |
| D3 | 1-spec | Spec review round 2 (sonnet) | CLEAN. Four MINORs, all applied: which state a fast check error gets; relative path with the feature on shows both notes; "readable" as the OS reports it for the current user; rewrapped a long line. | agent-resolved | reviewer subagent round 2 |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Open escalation

None.

## Follow-ups not done

- The daemon only logs (`tracing::warn!`, `crates/micold-daemon/src/state.rs:573`) when it fails
  to resolve the script for a session's own directory, and tells no client. That is a defect
  against 011 as written (FR-013 with FR-020), filed as #454. It is outside #435
  and outside this spec. See `specs/011-env-include-script/bugs/BUG-006.md`, "Observations".
