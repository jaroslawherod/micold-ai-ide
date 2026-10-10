# Autopilot ledger — #490 claude-plan-usage

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 490 — implement GitHub issue #490 "Show Claude plan usage and next limit reset" (show current usage and the next limit reset for the signed-in Claude account in the status area or Settings; warn past a configurable threshold; show and log nothing alarming when data is unavailable; works offline; no credentials read beyond the chosen source's needs, none leave the machine except to the provider; undocumented endpoints not used). labels: enhancement, flow:feature (in-progress added at claim)
- **Kind**: feature
- **Effort**: default
- **Issue**: #490
- **Worktree branch**: claude/project-thread-u1pay5
- **Started**: 2026-10-10
- **Phase**: design PR
- **Next step**: merge the design PR, then milestone M1

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | `/status` POST → daemon keeps reading, Welcome + PlanUsageChanged broadcast | — | pending |
| M2 | T018–T027 | full | app-run Claude sessions post readings via `micold-daemon status-line`, user's status line kept | — | pending |
| M3 | T028–T038 | full | app bar shows `NN% · HH:MM` + tooltip; hidden without a current reading | — | pending |
| M4 | T039–T046 | light | warning look at threshold; Settings → Environment switch, threshold, statement | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| 1 | clarify | FR-012 usage source | Claude Code status-line `rate_limits` from app-run sessions via the existing `--settings` file, user's own status line chained (FR-020) | decided by user (jaro, 2026-10-10) | code.claude.com/docs/en/statusline; crates/micold-daemon/src/state.rs:872 |
| 2 | clarify | FR-001 default | On | decided by user (jaro, 2026-10-10) | spec FR-001 condition: no credential read, nothing sent |
| 3 | clarify | FR-009 staleness without polling | Each window current until its own reset passes; indicator re-picks the highest current window, hidden when none; details show reading time (FR-009 reworded in tasks unit after analyze I4) | agent | statusline docs `rate_limits` |
| 4 | clarify | FR-019 account identity | None in source; newest reading wins | agent | spec Out of Scope (no private storage) |
| 5 | clarify | FR-007 switch vs running sessions | New sessions always; running ones when Claude Code reloads the per-session settings file | agent | state.rs activity_launch_for |
| 6 | plan | Relay binary for the status line | The daemon executable with a `status-line` argument; user's statusLine resolved at session prepare time and chained | agent | research.md R3, R4 |
| 7 | tasks | Milestone cut | M1 (17 tasks) is Setup+Foundational+the daemon half of US1 s3: US1/US3 share one data path, so the P1 stories split by layer along US1 s3 (route / relay / indicator), each with an observable deliverable; M4 joins US2 and US4 because US2 s4 (threshold change applies) needs US4's field and US2 alone is 4 tasks; T047–T048 are doc-only, left to the close unit | agent | milestones.md rules 1–4; tasks.md Milestones |
| 8 | tasks | FR-009 when the shown window passes but another is current | Re-pick the highest current window; hide only when none remains (spec FR-009, US1 s4 reworded; was "hide") | agent | speckit-analyze I4; Claude Code drops each window at its own reset, the others stay accurate |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 9b2c2be0c4f91c4d56352d31de30d7e240d7d52b:149cda625a4e7942faafdff528924971fb1bd223 | CLEAN (3 MINOR, all fixed: FR-019 + US3 scenario 6 for account change, FR-007 single 5-minute interval, checklist testable item unticked) |
| Plan | 1 | 6faf43340f356aff088fee399abf65dceed3b86c:fddb3ebf8d1340bb82ea64e182257534998538e1 | CLEAN (3 MINOR, all fixed: claude_settings_dir marked new, sandbox placement risk, FR-003/FR-011 test lines) |
| Tasks | 1 | f491c06f7c981feadd01b9f3fe232bba92085878:ce1baee44edcfdf64c10b86807c39f96ce42d697 | CLEAN |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. Clarify round 1 (Q1 FR-012 source, Q2 FR-001 default) closed: the user (jaro) chose "Status line, on" (Q1 A, Q2 A) on 2026-10-10; Decisions 1-2.

## Follow-ups not done

- Plan usage for other AI CLIs (issue's open question): out of scope for this feature, a later request.
