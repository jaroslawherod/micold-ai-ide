# Autopilot ledger — #482 worktree-changes-review

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #482 "Review a worktree's changes with inline comments and send them to the session" (scope verbatim from the issue); labels: enhancement, flow:feature
- **Kind**: feature
- **Effort**: default
- **Issue**: #482
- **Worktree branch**: claude/project-thread-v1va8z
- **Started**: 2026-10-06
- **Phase**: implement
- **Next step**: milestone M2 (T027–T040)

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #624 | whole branch (design + every milestone), opened by the orchestrator | draft | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T026 | full | Changes view with the changed-file list (MVP) | #624 | done |
| M2 | T027–T040 | full | Unified diff of the selected file | #624 | in progress |
| M3 | T041–T049 | full | Side-by-side layout, syntax colouring, kept layout | — | todo |
| M4 | T050–T063 | full | Comments on lines and ranges | — | todo |
| M5 | T064–T075 | full | Send comments to the running session | — | todo |
| M6 | T076–T080 | full | Send when no session is running | — | todo |
| M7 | T081–T089 | full | Live refresh and outdated comments | — | todo |
| M8 | T090–T097 | full | Clear, discard, and removal with the worktree | — | todo |

## Decisions

- Clarify round 1: US3 scenario 4 and FR-003 answered by the orchestrator as defaults (user away); recorded in spec.md Clarifications. No further ambiguities, no second round.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

- Tasks: US1 split in three (list / unified diff / layouts+colour), US2 in two (comments / send), US4 in two (refresh+outdated / tidy), along acceptance scenarios. M1 stays at 26 tasks because Setup + Foundational (core value types, protocol v30 in one edit — a second bump would fail schema_hash.rs) must ride with it. Polish T098–T100 changes no code: close unit.
- Gate (tasks unit): specs-only, `scripts/check-criteria-observables.sh` passed (spec has no criteria table). Design PR: none of its own — this run ships one PR for the whole branch (orchestrator opens it); PR body written to the session scratchpad pr-body-482.md.
- speckit-analyze (0 CRITICAL, 2 HIGH, 6 MEDIUM, 5 LOW): fixed in tasks.md (D1 watch::Debouncer + cap_spans as tested pure functions, D2 two-session test moved to T066, D3–D7, D12); spec.md edits (D8 no-bracketed-paste edge, D3 worktree removed outside the app, D10 FR-016 wording, D13 toggles reset on open) and plan.md Constitution row I (D1). D9, D11 left: wording only.
- M1 L1 deviation: the Committed/Uncommitted toggles are `ToggleChip`s (`.active().disabled()`), not `LabelledToggle` as contracts/changes-view.md L1 says — `LabelledToggle` has no on/off state to show. Same behaviour, the library's stateful control.
- M1 V1 deviation: the Default row has no menu of its own; it reuses the worktree `WorktreeMenu` with `dir_name == ""` (the wire's name for the project root), right-press added in `ui/sidebar.rs`, and `ui/mod.rs` offers only **Review changes** for `""`. Avoids a second context-menu surface.
- M1 part 5: the review reads go through the `Git` seam (`Git::review_base`/`change_list`, delegating to `GitCli`'s inherent methods; `FakeGit` answers no base / no files) and `Capabilities::shared_git()`, because `no_concrete_implementations` allows `GitCli` to be chosen only in `shell/capabilities.rs`. With no local git (daemon elsewhere) the view says the files cannot be read here.
- M1 part 5: `settings_sections` DEFERRED gains `("diff_layout", "482 T041")`; the root vocabulary guard moves to 15 feature wrappers (`Changes`); the layout snapshot's worktree-menu state grows by the **Review changes** row (regenerated deliberately).
- M1 part 4: the orchestrator waived the part-4 escalation limit for M1 (steady progress on a 26-task milestone) and continued.

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | c5e15968c86ce0c17544cde68a31ff74e2fb1e8b:107ab7ed3aa9b067a4dca5c02aca6e6709d4d158 | CHANGES: 2 MAJOR, 3 MINOR (all fixed) |
| Spec | 2 | c960571fb7bed1fa440f89fe3b9720fd30c8d897:c74e2d4a9e09412fc66dd4915e3c7e105b785552 | CLEAN: 2 MINOR (both fixed) |
| Plan | 1 | 76b23838a33fc28d5c9f6db389cc15f46acd39a1:3a5ac98c80a82e8c74fb02606f5bee99309eaeeb | CLEAN: 3 MINOR (all fixed, prose only) |
| Review A M1 (code-review high) | 1 | c30e1678e86420d958d5891c993fbec650892723:1ed086452e5a2581dc19a1453b11c73730c68936 | CLEAN: 2 MINOR (untracked files read whole to count lines; lossy UTF-8 of non-UTF-8 paths), not fixed |
| Review B M1 (conformance, sonnet) | 1 | 6f1dd0f1b8ccac63f75133430d4aa2f34f23e4c6:e3ab664fc93e6c9de5e7223181127788b4024731 | CHANGES: 1 MAJOR (cycle-log greens missing) fixed by running the tests and recording them; 2 MINOR (T009 test-after kept as recorded; Verify not runnable in reviewer sandbox, unit re-ran it: all pass) |
| Gate M1 | full | c30e1678e86420d958d5891c993fbec650892723:1ed086452e5a2581dc19a1453b11c73730c68936 | green except the 6 root-only permission tests (container runs as root; pass in CI) |
| Review A M2 (code-review high, scoped 24009c2d..HEAD) | 1 | 0141f0f550c385a1179f01e688868725e0526001:142cf0fa764870556e1cbadf46a52186aab5b9c6 | CHANGES: 5 MAJOR, 3 MINOR — F1–F3 fixed (literal pathspecs, symlink target, untracked line count); F4, F5 open; F6, F7 MINOR open; F8 declined |
| Tasks | 1 | bdc7eb7285b7355f190266e00dd32362649c017c:a284e9b2ea58bc2c731608a1600a913ad9b91dec | CLEAN: 3 MINOR (all fixed: checklist ticked, T067 outdated list deferred to M7, M1 Verify + SC-001/SC-005 mapped) |

## Declined review findings

- Review A M2 F8 (MINOR, `file_diff` reads both blobs into `SideLines` though nothing uses them yet): data-model `LoadedDiff` carries them for M4 quotes and M7 outdated checks; kept.

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

M2 unit 2 handed over at the 150k cap. **Done (committed):** T029, T030, T035–T039 (`ui/material/diff_view.rs` `DiffView` + pure `body`/`tint`, `DiffLayout` re-exported from `ui::material`; diff half of `features/changes.rs`: `diff`, `shown_large`, `diff_offset/viewport`, `Msg::{ShowLarge, DiffScrolled, DiffRead}`, `Effect::ReadDiff`, `diff_body`/`DiffBody`, `can_pick`; `shell/changes.rs` runs `ReadDiff` via `spawn_blocking`; `ui/changes.rs` diff pane with `StageProgress` "Loading diff…"; showcase `DiffView` entry with 3 poses; user guide sections). Full gate 1 (scratchpad gate1.log): fmt+clippy green; 3 client test failures since fixed (builder API, showcase completeness/isolation); else only the root-only permission tests. Review A M2 round 1 F1–F3 fixed with red/green in the cycle log.
**Next:** (1) fix review A F4 — `DiffView` builds `unified_rows` (O(N) Vec) on every view: index rows by hunk prefix sums (binary search per built row) instead; F5 — `list_read` with `Err` must clear `selected` and set `diff` Idle (add a red test in features_changes.rs first); F6 MINOR — `FileSelected` of the already-selected file returns early unless its diff `Failed`; F7 MINOR — `NUMBER_WIDTH` 52 → 64. (2) Review A round 2 on sonnet, scoped: `review-snapshot.sh diff 0141f0f5…:142cf0fa…`; prompt template `scratchpad/review-A-M2-r1.txt`. (3) Review B (conformance rubric, sonnet) + T040 visual pass (quickstart B1, B4, B5, B6, B15 diff part; evidence under specs/482-worktree-changes-review/visual-pass/). (4) Full gate (`scratchpad/gate.sh`), commit, push, write the M2 PR-body section to `scratchpad/pr-body-482-M2.md`, return `PR: #624`.

## Open escalation

None.

## Follow-ups not done

None yet.
