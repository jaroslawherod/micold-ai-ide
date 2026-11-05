# Autopilot ledger — #1530 daemon-config-file (Issue 9)

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: `/speckit-autopilot #1530` (The daemon's credential helper chooses the token from the configuration); labels: `effort:high`, `flow:feature`. Spec, Clarify, Plan skipped by the same precedent as #1524, #1525, #1526, #1529 and #1534: design on main (PR #1509, `specs/daemon-config-file/`: FR-008, FR-010, FR-016, FR-030, FR-031, FR-035, `contracts/daemon-config.md` sections `[git]` and `[[github.tokens]]`); the tasks unit only cuts milestones for the "Issue 9" group (T046–T054, T092–T094).
- **Kind**: feature
- **Effort**: high
- **Issue**: #1530
- **Worktree branch**: feat/1530_the-daemon-s-credential-helper-chooses-the-token
- **Started**: 2026-11-04
- **Phase**: M2 gate and PR
- **Next step**: merge M2 PR, then the Close unit (last milestone)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1686 | Design (milestones for Issue 9) | MERGED | 826681cfb63664b82f65026abfd61abb190956c5 |
| #1700 | M1 | MERGED | 3ffb138fbe2f1fbacfb8cbad179ad4a99202e95f |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T046–T052, T093, T054 | full | helper chooses token from config (most specific, ties first), reach cached by fingerprint, per-kind session git env with fallback/clear, per-host rewrite | #1700 | merged |
| M2 | T092, T094, T053 | full | git setup from daemon.kind on every start path, GIT_CONFIG_GLOBAL, gpgsign stripped, grant messages retired + protocol bump | PR pending | in review/gate |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | Write a new spec.md for #1530? | No: the design of the whole feature is on main and covers this issue; a second spec would duplicate FR-008/010/016/030/031/035. Skipped as for #1524/#1525/#1526/#1529/#1534. Blockers #1524 and #1529 are merged (#1529 via PR #1684). | agent-resolved | specs/daemon-config-file/autopilot-1529.md |
| D2 | tasks | Split Issue 9? | Yes: 11 tasks, two stories (rule 3). M1 = helper + env (T046–T052, T093) with T054 guide (rule 4); M2 = kind gating, git config file, retirement T053 (wire change reviewed alone). Both full: security check, sandbox boundary, wire protocol. | agent-resolved | milestones.md |

## Review rounds

- M1: review A (code-review high) 10 findings, 4 fixed (host reset now per forge, fallback on unreadable config, absolute config path, token file leading blank line); review B (sonnet) CLEAN, 3 MINOR (F1 fixed). Snapshot 503b13b8:826681cfb.

- M2: review A (code-review high) CLEAN, 3 MINOR (F2 fixed, F1 noted in Follow-ups); review B (sonnet) F1 MAJOR (no cycle-log entry) fixed, F2/F3 MINOR noted. Rebased onto origin/main afterwards; full gate re-run on the rebased tree. A Close unit follows this last milestone.

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Tasks | 1 | fd03d24b9bf4c7aac5fd8b2e77207878cc688dbc:259bcb30e95d60c215c8bba19b45fac7123e8c07 | CHANGES: F1 MAJOR (no guide task in M2) fixed by writing docs into T094; F2 MINOR git_credential_grant.rs added to T053; F3 MINOR noted: matching.rs and git_credential_config.rs are created by T046/T047, no `TokenChoice` exists yet. Fixes are task-text only, no new round |

## Declined review findings

- M1 review A: reach listing capped at 30 pages, URL-form GHE host gets no `/api/v3`, fallback capture uses the daemon's cwd and exact keys, `run_fallback` has no timeout and forwards only `get`, `UreqTransport` duplicates the client's, legacy `GIT_CONFIG_KEY_n` leftovers: accepted for M1 (MINOR in effect; listed in Follow-ups). Review B MINORs F2 (cache lock), F3 (capture cwd): same.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None.

## Follow-ups not done

- M1: share `UreqTransport` via core; page cap and timeout for reach/fallback; forward `store`/`erase` to fallback helpers; capture fallback helpers per session cwd and URL-matching keys; `network.ca_file` for the daemon's HTTPS.

None yet.

- M2 follow-up: replacing `CredentialShare::GitConfig` and the kube `gitconfig` Secret mount (T094 text) was not done: the sanitized copy stays the delivery mechanism and container and pod daemons name it as `[git] config_file`. Also `Msg::CredentialStatuses` in the client feature is now without a producer (review A F1, MINOR).
- Review A (high): CLEAN, 3 MINOR (F2 fixed: stale module doc and unused `RENEW_MARGIN_SECS` removed). Review B (sonnet): F1 MAJOR (no cycle-log entry) fixed; F2/F3 MINOR noted.
