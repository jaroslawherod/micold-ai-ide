# Autopilot ledger — #490 claude-plan-usage

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Spec 490 — implement GitHub issue #490 "Show Claude plan usage and next limit reset" (show current usage and the next limit reset for the signed-in Claude account in the status area or Settings; warn past a configurable threshold; show and log nothing alarming when data is unavailable; works offline; no credentials read beyond the chosen source's needs, none leave the machine except to the provider; undocumented endpoints not used). labels: enhancement, flow:feature (in-progress added at claim)
- **Kind**: feature
- **Effort**: default
- **Issue**: #490
- **Worktree branch**: claude/project-thread-u1pay5
- **Started**: 2026-10-10
- **Phase**: M3
- **Next step**: M3: implement T028–T038, verify, PR

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #694 | design (spec, plan, tasks) | merged | 984ef002 |
| #772 | M1 + gate runs permission tests as a normal user under root (jaro's pick) | merged | d9bd52e2 |
| #777 | M2 status-line relay | merged | a0dbee58 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | `/status` POST → daemon keeps reading, Welcome + PlanUsageChanged broadcast | #772 | merged |
| M2 | T018–T027 | full | app-run Claude sessions post readings via `micold-daemon status-line`, user's status line kept | #777 | merged |
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
| M1 review A (high) | 1 | f2c8b438 | 10 findings: 6 fixed (403 log set bounded to known sessions; idle-session repeats ignored, FR-019; non-numeric or out-of-range `resets_at` drops the window; key-order doc; `unix_secs` reuses `unix_now`; rejections reset only on a kept reading), 3 declined (below), 1 taste |
| M1 review B (conformance) | 1 | f2c8b438 | CHANGES: F1 MAJOR no `tdd/cycle-log.md` — fixed (backfilled with the red/green commits); F2 MINOR = review A's 403 set, fixed; F3 MINOR status/reason coupling, fixed |
| M2 review A (high) | 1 | ac3fec9c:6e6f9ab2 | 10 findings: 4 fixed (`forget` now runs at session removal; one lock over switch, prepare and forget, the receiver owns the switch; odd `padding`/`refreshInterval` keep the command; shared `claude_locations` helper), 6 declined (below) |
| M2 review B (conformance, sonnet) | 1 | ac3fec9c:6e6f9ab2 | CHANGES: F1 BLOCKER Windows shells not wrapped in `no_window` (console flash; `background_spawns_hide_console` red) — fixed |

## Declined review findings

- M1 review A: `Welcome` read before the window is registered can miss one push in between — the catalog has the same gap; the next changed reading corrects it. Declined (cost over value).
- M1 review A: `Settings`/wire carry `u8` rather than `WarnPercent` — taste; clamped at every write and read. Declined.
- M1 review A: `Welcome` settings and plan usage taken under two locks — transient, corrected by the next `SettingsChanged`/`PlanUsageChanged`. Declined.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M2 | A | `shell_quoted` does not escape `$`, backtick, `"` | The paths are the daemon executable and the app's own data dir; contract S1.2 fixes double quotes (Claude Code's Git Bash rule) |
| M2 | A | A higher-precedence `statusLine` that is not a usable command should win, not fall through | Spec/data-model *UserStatusLine* choose fall-through to the first usable command (T018 asserts it) |
| M2 | A | Switch off deletes the relay file while a running session still calls it, blanking the user's line until reload | Contract S1.3/FR-002: no relay file while off; Decision 5 accepts the reload lag |
| M2 | A | Feeder joined before `wait` can hang | Not reproducible: a child that never exits blocks `wait` either way; once it exits the write fails |
| M2 | A | Joining the POST adds up to 500 ms when the daemon is down | Contract S2.3 bounds it at 500 ms; the user's line is already written |
| M2 | A | Windows GUI-subsystem std handles may be invalid | Speculative; research R3 Windows note, quickstart §B4 checks it on Windows |
| M2 | A | Two switch toggles in the same instant can apply out of order | Far-fetched (two windows within ms); the next toggle corrects it |

## Handover

None.

## Open escalation

None. Clarify round 1 (Q1 FR-012 source, Q2 FR-001 default) closed: the user (jaro) chose "Status line, on" (Q1 A, Q2 A) on 2026-10-10; Decisions 1-2.

## Follow-ups not done

- Plan usage for other AI CLIs (issue's open question): out of scope for this feature, a later request.
