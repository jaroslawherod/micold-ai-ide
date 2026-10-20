# Autopilot ledger — #1150 diff-full-file-view

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Issue #1150 "Show the full file and browse the repo's code while reviewing a diff in the Changes view" (full text via gh issue view 1150); labels: none
- **Kind**: feature
- **Effort**: default
- **Issue**: #1150
- **Worktree branch**: feat/1150_show-the-full-file-while-reviewing-a-diff-in-the
- **Started**: 2026-10-20
- **Phase**: implement M1
- **Next step**: continue M1 per Handover

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #1151 | Design PR (spec, plan, tasks) | merged | df019910c4b525b25df8b08f3fd1073a11b28f72 |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T017 | full | Expand hidden lines in a diff (MVP) | | pending |
| M2 | T018–T022 | light | Whole-file toggle | | pending |
| M3 | T023–T031 | full | Pull request head version | | pending |
| M4 | T032–T035 | light | Pick revealed lines | | pending |
| M5 | T036–T046 | full | Browse repository files | | pending |
| M6 | T047–T053 | full | Browse at the PR head | | pending |
| M7 | T054–T062 | full | Follow references from the diff | | pending |

## Decisions

- M1 has 17 tasks (Setup + Foundational + US1): Foundational alone is no milestone, and no acceptance-scenario split exists below the gap marker. M5 has 11 tasks, kept whole: its tree, picker and back share one reducer state and no acceptance scenario splits them. Polish (T063-T064) changes no code: left to the close unit.
- analyze C1/C2: browsed-file pick pin moved to M5 (T038); M4 mirrors the existing line-pick handling.

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec | 1 | 61e32c1ce6afda347a1774dc7b2c2ebd349b7abd:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | round 1 CHANGES: 2 MAJOR (F1 reference undefined, F2 picking in untouched files); all fixed, 3 MINOR fixed |
| Spec | 2 | ebdf38fc38d5021afe13cc798bd796d24dabf88b:cf7c64d7d7ee21a78fe03d60d64165dd811a1c92 | CLEAN (1 MINOR fixed, prose only) |
| Plan | 1 | a062beaa985a9a6340ed2e978a69d8ea0fe6978c:6179267cd162fbbfa13ebaa171f85c55810ee0be | CHANGES: 3 MAJOR (PR trailing gap count, hunk-header conventions, test-layer map), 4 MINOR; all fixed |
| Tasks | 1 | 5266c05cc605449114f9bfbfc5ad2ee6617854f2:111e9d2846815d3132eb6a1358ea0f22e03e285a | CHANGES: 2 MAJOR (M5 deps, read_file tests placement), 3 MINOR; all fixed |
| Tasks | 2 | b4c0e78be90cc023ce70435de12ba29d12acbdfd:5c79030bf66c3416cdc958865266352c156ac6c0 | CLEAN (1 MINOR fixed) |
| Plan | 2 | 3213adb2c03edaa4431e840ab07b5a36c88de031:6179267cd162fbbfa13ebaa171f85c55810ee0be | CLEAN (3 MINOR fixed, prose only) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Unit M1 (T001-T017) hit the 150k context cap mid-implement; no PR yet, nothing pushed. Branch was reset to origin/main by branch-start.sh; WIP is in the working tree (committed as a WIP commit).

Done (uncommitted-then-WIP, code written, NOT all compiled/tested):
- `crates/micold-core/src/review/expand.rs` (T001-T007): Gap/GapKey/Position/Expansion(toggle, toggle_key, set_whole, is_open, reconcile)/Revealed/Shaped/ShapedRef/gaps()/shape() with unit tests. `mod expand` registered in review/mod.rs. `mise run test-core -- review::` was green (147 pass) except one typo in a diff.rs test, since fixed; expand.rs got ShapedRef + toggle_key afterwards and has not been re-run.
- `diff.rs` (T008-T009): `unified_rows`, `side_by_side_rows`, `UnifiedIndex::new`, `SideIndex::new` now take `impl Into<ShapedRef<'a>>` (a `&Shaped` or `ShapedRef::unshaped(hunks)`); new `UnifiedRow::Gap{gap,open}` / `SideRow::Gap{gap,open}` (open=true is the collapse row placed where a revealed run starts); tests updated and two new ones added. Needs re-run.
- `ui/material/diff_view.rs` (T011, T013): `body(diff, Option<&Shaped>, layout)`, `DiffView::shaped(&Shaped)`, `.on_gap(|GapKey| M)`, `gap_row`, `gap_label` ("N hidden lines"), geometry tests appended. NOT compiled yet (client crate still uses old `unified_rows(&diff.diff)` calls in features/changes.rs lines ~561-576, which break).

Next steps:
1. `cargo fmt`, run `mise run test-core` then fix compile in micold-client (features/changes.rs `row_of_line` uses unified_rows/side_by_side_rows on `&diff.diff`: use ShapedRef::unshaped(hunks) or the view's shaped).
2. T010/T012 reducer: add to `OpenView` `expansions: BTreeMap<(SourceKey, RelPath), Expansion>` and a cached `shaped: Shaped` for the selected worktree file; `SourceKey::{Worktree, Pr(u64)}`; `revealable(view)` (Changes source, Text diff, `LoadedDiff::new` Some, file kind not Added/Untracked/Deleted); `Msg::GapToggled(GapKey)` (uses `Expansion::toggle_key`); reconcile against `gaps(hunks, new.len())` on DiffRead/refresh; recompute shaped after every non-scroll message (plan: wrap `update`); `file_comments`, `diff_rows`, `row_of_line` must read `shaped.hunks` for Source::Changes. Decision: the existing `Msg::ShowLarge` gate already covers over-2 MB versions (git.rs gates `new` size before reading), so no separate `ShowLargeFile` Msg is added for M1; PR files have `new: None`, so they offer no markers in M1 (PR head read is M3). Write reducer tests first in `crates/micold-client/tests/features_changes.rs`.
3. T014: in `ui/changes.rs` (~line 515) pass `.shaped(&view.shaped)` and `.on_gap(|key| Message::Changes(Msg::GapToggled(key)))` to DiffView. T015: `highlight_file` in `ui/syntax.rs` (highlight over the full new side so revealed lines are coloured; spans for the whole new side may already exist in `LoadedDiff.spans.new`, check how spans are produced and extend to the full file). T016: gap row in `showcase/sections/review.rs`. T017: "Reading hidden lines" in `docs/user-guide/reviewing-changes.md`.
4. Tick T001-T017 in tasks.md, then flow per tasks/verify.md, review.md (Review A `high` in the shadow of the scoped gate, Review B once, visual pass), full `mise run gate`, PR (title `feat(1150): expand hidden lines in a diff (#1150)`, body ends `Refs #1150`).
Note: builds wait on other worktrees' build lock; use `setsid nohup ... & ` plus `scripts/autopilot/hold.sh`, foreground max 250 s.

## Open escalation

None.

## Follow-ups not done

None yet.
