# Autopilot ledger — #1530 daemon-config-file (Issue 9)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1530` (The daemon's credential helper chooses the token from the configuration); labels: `effort:high`, `flow:feature`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1529 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-008, FR-010, FR-016, FR-030, FR-031, FR-035, `contracts/daemon-config.md` sections `[git]` and `[[github.tokens]]`); the tasks unit only cuts milestones for the "Issue 9" group (T046–T054, T092–T094).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1530
- **Worktree branch**: feat/1530_the-daemon-s-credential-helper-chooses-the-token
- **Started**: 2026-11-04
- **Phase**: tasks (milestones cut, in review)
- **Next step**: design PR open (see PR table); orchestrator waits on CI and merges; then M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1686 | Design (milestones for Issue 9) | OPEN | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T046–T052, T093, T054 | full | helper chooses token from config (most specific, ties first), reach cached by fingerprint, per-kind session git env with fallback/clear, per-host rewrite | — | pending |
| M2 | T092, T094, T053 | full | git setup from daemon.kind on every start path, GIT_CONFIG_GLOBAL, gpgsign stripped, grant messages retired + protocol bump | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1530? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-008/010/016/030/031/035. Skipped as for #1524/#1525/#1526/#1529/#1534. Blockers #1524 and #1529 are merged (#1529 via PR #1684). | agent-resolved | specs/daemon-config-file/autopilot-1529.md |
| D2 | tasks | Split Issue 9? | Yes: 11 tasks, two stories (rule 3). M1 = helper + env (T046–T052, T093) with T054 guide (rule 4); M2 = kind gating, git config file, retirement T053 (wire change reviewed alone). Both full: security check, sandbox boundary, wire protocol. | agent-resolved | milestones.md |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks | 1 | fd03d24b9bf4c7aac5fd8b2e77207878cc688dbc:259bcb30e95d60c215c8bba19b45fac7123e8c07 | CHANGES: F1 MAJOR (no guide task in M2) fixed by writing docs into T094; F2 MINOR git_credential_grant.rs added to T053; F3 MINOR noted: matching.rs and git_credential_config.rs are created by T046/T047, no `TokenChoice` exists yet. Fixes are task-text only, no new round |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

None yet.
