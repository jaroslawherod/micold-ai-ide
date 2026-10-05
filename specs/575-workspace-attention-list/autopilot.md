# Autopilot ledger — #575 workspace-attention-list

Kept by the `speckit-autopilot` skill. Records what this flow owns and how far it got. `resume`
finds this file by its **Worktree branch** line. Keep it true.

- **Input**: Implement GitHub issue #575 "Show sessions that need attention at workspace level" (show unread sessions at workspace level: per-project marks with counts in the switcher, one workspace-wide list naming project, worktree and session, selecting an entry opens it like a notification click, live and restart-proof, reusing the existing attention state). labels: enhancement
- **Scope change (user, 2026-10-05)**: after spec and clarify, the user wrote verbatim: "the clue was to add indicator of attention at sidebar with the list of worktrees". The workspace attention list in the switcher's panel (old FR-001..FR-015, D2, D3) is dropped. New scope: each sidebar worktree row and the Default row with unread sessions (039) shows the unread mark with their count. Spec rewritten in place (directory name kept, title renamed).
- **Kind**: feature
- **Effort**: default
- **Issue**: #575
- **Worktree branch**: claude/project-thread-8dnq8h
- **Started**: 2026-10-05
- **Phase**: implement M1 (in progress, handed over)
- **Next step**: fresh milestone unit continues M1 from *Handover* (stacked on draft PR #602, branch claude/project-thread-8dnq8h; skip branch-start.sh).
- **Milestone notes**: US1 is 13 tasks plus US2's one; US2 adds no code (R7), so it rides in M1. US1 scenario 5 (closed sessions on the switcher, FR-010) is split out as M2 along that acceptance scenario. Polish T020–T021 changes no code: left to the close unit (quickstart B6, B9, B11, §C).
- **Plan/spec fixes in tasks unit**: FR-005 reworded after speckit-analyze F5: the assistive-technology clause is now conditional on the toolkit exposing an accessible label (plan Known limitation).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #602 | design + M1 stacked on the same branch | draft, open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | `● n` on each sidebar worktree and Default row with unread sessions, collapsed or expanded, live | #602 | in progress |
| M2 | T015–T019 | light | The switcher's counts skip closed sessions; location rows add up to the switcher count | — | pending |

## Decisions

| # | Phase | Question | Answer | By | Evidence |
|---|---|---|---|---|---|
| D1 | spec | What of #575 does 039 already ship? | Switcher row counts (FR-021/022), button total (FR-023), live clearing (FR-019), restart persistence (FR-008a), reveal path (FR-011–014). Spec covers only the workspace-wide list and its navigation. | agent-resolved | specs/039-session-attention-notifications/spec.md; crates/micold-client/src/ui/toolbar.rs:78; crates/micold-client/src/app.rs:503 |
| D2 | clarify | **Superseded (D4).** FR-004: order of attention-list entries? | Grouped by project in switcher order, then worktree and session in sidebar order; never by time of becoming unread. | agent-resolved | specs/039-session-attention-notifications/data-model.md (attention_seq is per-session, no time kept); spec.md FR-012, FR-014 |
| D3 | clarify | **Superseded (D4).** FR-001: where does the attention list open from? | (a) Section at the top of the switcher's panel, above the project rows. | orchestrator default, pending user answer on decision card | crates/micold-client/src/ui/toolbar.rs:57-94; 039 FR-023 |
| D4 | spec | What does #575 ask for? (scope change) | An attention indicator (unread mark + count) on each sidebar worktree row and the Default row holding unread sessions; the switcher attention list is dropped. | user: "the clue was to add indicator of attention at sidebar with the list of worktrees" | spec.md Clarifications |
| D5 | spec | Does the indicator show on an expanded location row? | Yes, expanded or collapsed; session rows keep their own marks. | orchestrator default | spec.md FR-003 |
| D6 | spec | Do closed (archived) unread sessions count? | No, on location rows nor in the switcher counts/button total, so a project's location rows add up to its switcher count (FR-010). | agent-resolved | 039 spec US1 scenario 9; crates/micold-core/src/workspace.rs:334 (`unread_session_count` filters on `unread` only); crates/micold-core/src/session.rs:339 `archived` |
| D7 | clarify | Which hidden worktrees does "the sidebar's filter" cover (edge case, FR-010)? | Both tag filters (008 FR-025) and hidden agent-owned worktrees (014); hidden rows show no indicator but still count on the switcher; the 024 re-admitted row shows its indicator. Wording only. | agent-resolved | crates/micold-client/src/features/sidebar.rs `filtered_worktree_tree`, `visible_worktrees`; crates/micold-core/src/workspace.rs:334 |

## Review rounds

| Review | Round | Snapshot | Verdict |
|---|---|---|---|
| Spec 575 | 1 | 30231c55b37effe9790da6ea31243981031aa938:937ae9f7da42e25571db7b988153f707ba931e3e | CLEAN: 3 MINOR (all fixed, prose only) — superseded by the rewrite |
| Spec 575 (rewrite) | 1 | 84afa026eb373957d3319604344384c2c53d8ee4:6075557947bfb5e1361568a3fa8ad0408ce9d424 | CLEAN: 3 MINOR (all fixed, prose only; run via `claude -p --agent autopilot-reviewer`) |
| Plan 575 | 1 | a544cbbec276b6e538a862790af2b637ccd7a155:d7bf4f9b57b064be396b814b2bc37f390bf39002 | CLEAN: 2 MINOR (both fixed, prose only; run via `claude -p --agent autopilot-reviewer`) |
| Tasks 575 | 1 | 546c4d5d35bd1de8132ad3d6d78f2341f17c4afd:c0987c41765f310a8299693bbe13c42d8aea98f0 | CLEAN: 1 MINOR (wrong task ref in T002, fixed, prose only; run via `claude -p --agent autopilot-reviewer`) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|

## Handover

Milestone unit M1 handed over at 150k context (2026-10-05). Commit `test(575): M1 red tests and stubs` holds:

- **Done (red, tests written, stubs only, nothing ticked in tasks.md)**: T001 tests (`attention.rs` `mod counted_session_tests`, stub `counts_as_unread` returns false: 2 of 5 fail, decisive red `attention::counted_session_tests::an_unread_session_out_of_view_counts ... FAILED`); T003 + T004 tests in new `crates/micold-client/tests/sidebar_attention.rs` (stubs in `features/sidebar.rs`: `SidebarEntry::unread_count` -> 0, `unread_tooltip_line` -> None, `with_unread_line` -> unchanged); T005 tests appended to `tree_view.rs` `mod tests` (stub `.unread_count` no-op; `TreeItem::label_role` extracted from the render code, behaviour-preserving; `pub(crate) fn count_tint(r, row_tint)` stub returns `row_tint`, real must return `r.on_surface`; render must call `.tint(count_tint(r, item.tint))` so T010 measures it). The client tests (sidebar_attention, tree_view lib) were NOT yet compiled/run: first step is `scripts/build-lock.sh cargo test -p micold-client --test sidebar_attention` and `--lib tree_view`, fix compile errors, record the red lines in `specs/575-workspace-attention-list/tdd/cycle-log.md`.
- **TDD hook**: no `tdd/test-list.md` exists (design did not run tdd.plan); follow 582's precedent: the test-first task pairs serve as the list, cycle-log.md records each red (create it; T001 red above).
- **Next**: T002, T006, T007, T008 (replace `pub unread: bool` with private `RowUnread`; grep `.unread\b|TreeItem {` across crates), T009, T010, T011, T012, T013 (visual-pass via `claude -p`), T014; then verify.md (review A high, review B sonnet, full gate), push to #602, PR-body section to the scratchpad `pr-M1.md`.
- **Baseline full suite (this container, run as root)**: 6 pre-existing failures unrelated to 575, all permission tests root bypasses: core `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (2), daemon `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path...`, `settings_service_write_refuses_failed_read`. Note them in the PR body, do not fix. `ui_glyph_literals` flags any literal U+25CF in `src/` (also in comments): never write the dot glyph in code or comments.
- **Environment**: no mise (run gate commands raw), gh unauthenticated (no gh calls, no PR), reviewers via `claude -p --agent autopilot-reviewer`.

## Open escalation

None. (D3's decision card is moot: superseded by D4.)

## Follow-ups not done

None.
