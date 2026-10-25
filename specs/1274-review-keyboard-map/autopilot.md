# Autopilot ledger — #1274 review-keyboard-map

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1274 "Review UI: keyboard map for reviewing": Blocked by: #1273, #1271, #1127. j/k file, n/p hunk, ]/[ comment, v reviewed, g next unreviewed, c comment, Ctrl+Shift+Enter send, ? shortcut sheet; the jump-to field finds files of the open review and comments. Part of the review UI redesign: `docs/development/review-redesign.md`. Mockup: https://claude.ai/artifact/MviqD4uqBNaZtsCKH7yuTu (Review tab redesign). Acceptance also: pure logic in micold-core with unit tests; docs, layout snapshot and visual pass (1280x800, both themes) updated. labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #1274
- **Worktree branch**: claude/project-thread-m7gq7k
- **Started**: 2026-10-24
- **Phase**: milestones
- **Next step**: M1 PR #1404 open, waiting on CI and merge. M2/M3 wait for #1269, #1270, #1271, #1273; M4 for #1127. D4 still provisional.

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1366 | design (spec, plan, tasks) | merged | 76c1335e705aec8ebf02d278273bcc578d91112f |
| #1404 | M1 core key map | open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T011, T022–T023, T028–T032, T048–T051 | full | core REVIEW_KEYS + resolve, navigate rules, header_rows, jump candidates + detail ranking, unit tests (no blocker) | #1404 | PR open, CI pending |
| M2 | T012–T021, T024–T027 | full | j/k/n/p/v/g in the Review tab, hunk mark, docs — **blocked by #1269, #1270, #1271, #1273** | — | blocked |
| M3 | T033–T047 | full | ]/[, c, send chord, shortcut sheet, full docs — after M2 (same blockers) | — | blocked |
| M4 | T052–T057 | full | jump-to review entries — **blocked by #1127**, after M3 | — | blocked |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | `g` single key or `g f` prefix (redesign table) for jump-to? | `g` = next unreviewed (issue wording); jump-to keeps Ctrl/Cmd+Shift+P | agent-resolved | issue #1274 body; spec.md#assumptions |
| D2 | spec | Do next/prev wrap? | j/k, ]/[, g wrap (803 step rule); n/p continue into adjacent file and wrap | agent-resolved | crates/micold-core/src/navigation.rs; spec.md FR-003, FR-004 |
| D3 | clarify 1 | Send chord with unsaved composer text: save+send, or saved drafts only? | Saved drafts only; composer stays open with its text and the keyboard | agent-resolved | spec.md#FR-010 ("exactly" Send review); crates/micold-client/src/features/changes.rs Msg::SendPressed; review-redesign.md proposal 8 |
| D4 | clarify 1 | FR-001: single-letter keys on by default or behind a Settings switch (redesign open question 5)? | On by default, no setting | provisional — recommended option, awaiting user | review-redesign.md open question 5; spec.md#terms, FR-002, SC-002; specs/803-keyboard-navigation/spec.md#assumptions. If the user picks a switch (on or off by default), FR-001 gains a Settings requirement and clarify re-runs |
| D5 | tasks | Plan's M2 (US1–US5 client, ~29 tasks) as one milestone? | Split by story into M2 (US1, US2) and M3 (US3–US5); jump-to becomes M4. Plan, quickstart, contracts, research updated to match | agent-resolved | milestones.md rules 2–3; tasks.md#milestones |
| D6 | tasks | M1 is core-only (22 tasks, no user-visible deliverable)? | Kept whole: every client task needs the blockers (rule 6: core and tests ship, UI waits); splitting core gains nothing observable | agent-resolved | plan.md#milestones; research R1 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Review spec | 1 | 2f7c3dc307d7f818f4c2995e0ffd0af13213bfa9:3a333f7f1ddf23ad4d26f471cfd59f155bed3ff2 | CLEAN: 3 MINOR, all fixed (FR-002 scope, FR-004 p/n with no current hunk, checklist note) |
| Review plan | 1 | 209eba49689ebf46a4457c2a5dd880637479b3d2:5ca86ffe05655dc3708ecbaa3676d58bdfc4f439 | CLEAN: 3 MINOR, all fixed (R7 why not `.key_binding`; timing bound 50 ms; pending edge keyed to its path) |
| Review tasks | 1 | 7aa12f7058288de6231b9f39a7ed2953013b393a:dbe223deca1c5b2b34bb6d6d28a4e81151b49cd2 | CLEAN: 2 MINOR, fixed, prose only (T019/T020 test coverage note; T018 wording) |
| Review A M1 (code-review high) | 1 | a103038eb262768a10b41d932d46c9ca30051309:624a8eef02ba1008484c115d155023c9f0081051 | CLEAN: 8 findings, no BLOCKER/MAJOR; 1 fixed (`p` from a stale hunk past a shrunken diff now takes the last hunk, test added), 7 declined |
| Review B M1 (conformance, sonnet) | 1 | 570a8d0e8d03a8b98d1152198e00110d247a8c22:624a8eef02ba1008484c115d155023c9f0081051 | CLEAN: Verify (`cargo test -p micold-core --all-targets`) green, core lib 1707 passed; 1 MINOR (commit the review-A fix) — done in the M1 commit |

## Declined review findings

- Review A M1: declined (1) AltGr heuristic lets Ctrl+Alt+key act — contract K3 / research R7 say Ctrl+Alt without logo counts as none, on purpose (AltGr layouts); (2) comment `name` is whole text — contract J1 says text with lines joined by spaces; (3) jump-to uses the stored `range.start()`, not the located line — jump-to lists comments of unloaded files, whose located line is unknown; revisit in M4 when it is wired; (4) `header_rows` builds all rows — research R9's choice, once per `n`/`p`, MINOR; (5) `Press.primary` derivable from `ctrl`/`logo` — contract K5's shape, taste; (6) placeholder `has_hunks`/`in_diff` in `review_candidates` — `comment_order` reads neither, MINOR; (7) client `resolve` returns `None` for review keys — M4 (T052–T057) wires it.
- speckit-analyze (tasks): 0 CRITICAL/HIGH. Fixed C1 (SC-005 rank test in T050), C2 (SC-003 reducer timing in T017/T033/T041), C3 (SC-001 walk in T024/T038). Declined I1 (SC-004 counts FR-001/FR-010 keys; the sheet listing the save-comment chord too is a superset, not a conflict), U2 (T058 left to the close unit per milestones.md rule 4), S1 (M1 size: see D6), I2/C4/I3/U1/S2/P1 (LOW, no change).

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

None.

## Open escalation

None. (D4 is provisional; the coordinator's decision card to the user is still open.)

## Follow-ups not done

- Plan review ran via headless `claude -p --agent autopilot-reviewer`: Bash is denied there (workspace untrusted), so the reviewer used Read/Grep/Glob; a first run that only had denied Bash calls returned a vacuous CLEAN and was rerun. Symbols it could not confirm (`UnifiedIndex`, `SideIndex`, `jump_offset`, `settings::DiffLayout`, `is_primary_shift`, `Msg::SendPressed`, `window.focused_field`, `overlay::registry::topmost`) were confirmed by the plan unit with grep.
- Coordinator requirement from #1276 folded into the plan: public `micold_core::review::keys::REVIEW_KEYS` in M1; client binds only from it.

- Spec review ran via headless `claude -p --agent autopilot-reviewer` (read-only tools): no Agent tool in this unit.

- Blockers #1269/#1270/#1271/#1273/#1127 not merged at spec time; spec states the dependency (spec.md#dependencies).
