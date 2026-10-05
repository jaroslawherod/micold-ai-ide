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
- **Phase**: done
- **Next step**: handoff
- **Milestone notes**: US1 is 13 tasks plus US2's one; US2 adds no code (R7), so it rides in M1. US1 scenario 5 (closed sessions on the switcher, FR-010) is split out as M2 along that acceptance scenario. Polish T020–T021 changes no code: left to the close unit (quickstart B6, B9, B11, §C).
- **Plan/spec fixes in tasks unit**: FR-005 reworded after speckit-analyze F5: the assistive-technology clause is now conditional on the toolkit exposing an accessible label (plan Known limitation).

## Pull requests

| PR | Purpose | Status | Merge SHA |
|---|---|---|---|
| #602 | design + M1 + M2 + close stacked on the same branch (the close PR) | draft, open | — |

## Milestones

| ID | Tasks | Tier | Deliverable | PR | Status |
|---|---|---|---|---|---|
| M1 | T001–T014 | full | `● n` on each sidebar worktree and Default row with unread sessions, collapsed or expanded, live | #602 | pushed, CI pending |
| M2 | T015–T019 | light | The switcher's counts skip closed sessions; location rows add up to the switcher count | #602 | pushed, CI pending |

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
| Review A M1 (code-review high) | 1 | fd51b4f49d22a526379a017d433c16857cf5036b:4f6fb12c21364a11c8f758f2b13c8a7e48a53675 | 7 unlabelled findings, none BLOCKER/MAJOR on inspection: 2 fixed (row_unread delegates to counts_as_unread; guide names the re-admitted row), 5 declined (see below). Done. |
| Review B M1 (conformance, sonnet) | 1 | d1dac354ff9c1bab07c0cb224fbe372e7907f6b7:d8ad1e5b28906713af20e9bc9ee6708630c1324f | CHANGES on tooling only: F1 BLOCKER "could not run Verify" (its non-interactive session was refused cargo), F2 MINOR. No code finding; declined (below). Verify covered by the full gate's `cargo test --workspace`. Done. |
| Review A M2 (conformance + correctness, `claude -p --agent autopilot-reviewer`; covers A and B for the light tier) | 1 | 28140445b367d8d49868b2ed1bb543d45793f25e:b126d484e6128e1d7b9ee939a3d9e93f536e8fd6 | CLEAN: 1 MINOR (doc line over 100 columns, fixed, prose only) |
| Close 575 (tests + docs, `claude -p --agent autopilot-reviewer`) | 1 | f1e765fc1fa097a555b24e5953576f5bd0ea48ad:1bbe8f4b4252003c794eaeb3d89043d0d2ca0507 | CLEAN: 2 MINOR (guide line rewrapped, fixed, prose only; spec Status wording declined, below) |

## Declined review findings

| Milestone | Review | Finding | Why declined |
|---|---|---|---|
| M1 | A | `!archived` in `counts_as_unread` is dead for sidebar callers | By design (plan R1/D1): the same predicate serves the switcher counts in M2 (T018), where archived sessions are present. |
| M1 | A | `count_tint` ignores its `row_tint` parameter | Contract A6/R4: the parameter states what the count is independent of; the test pins that an error tint is not used. |
| M1 | A | count not matched to the selected-row label colour | Location rows are never selected (only session rows are); the contrast gate covers the selected fill at 4.5:1. |
| M1 | A | `let _ = mark(&counted)` in a geometry test | MINOR; `mark` asserts the mark exists. |
| M1 | A | counting each entry every render | MINOR; O(sessions), same order as building the rows. |
| M1 | B | F1 BLOCKER: Verify not run | Not a finding about the code: the reviewer's `claude -p` session was refused every cargo command. Verify's commands are all in the full gate's `cargo test --workspace`, run by this unit. |
| M1 | B | F2 MINOR: `row_unread` now also skips archived sessions | No visible effect: sidebar rows already filter `!s.archived` (`sidebar.rs:656,704,737`), so an archived session never renders as a row. |
| Close | review | F1 MINOR: spec Status says "shipped in PR #602" while #602 is open | `tasks/close.md` step 3 prescribes `Closed <date> — shipped in PRs #…`, written in the close PR itself; it is true once #602 merges, and nothing else merges this spec. |

## Handover

None.

## Environment notes (close)

- Close unit 1 handed over at the context cap; close unit 2 applied S3, S5 and S9 by hand (test-only, `sidebar_attention.rs` 23 → 26 tests, all pass; core `--lib workspace` 8 pass), wrote `tdd/verification.md` (as found FAIL on T010 test-after; PASS_WITH_GAPS after Phase 7 T022-T024), added the M2 and E2 rows to `cycle-log.md`. S6 and S7 (LOW) stay open in verification.md. The unreviewed `e.py` was not run.
- Full gate (raw commands, `CARGO_INCREMENTAL=0`, `--no-fail-fast`) on tree `f1e765fc`: fmt, clippy core and workspace, and all 60 `scripts/tests` suites pass. `cargo test --workspace` fails only the six root-only baseline tests (see M1). The tree the gate saw was stamped by hand, as in M1, because the root-only failures keep the raw gate from exiting 0. After the gate, only the guide rewrap and the ledger changed.

## Environment notes (M1)

- Full gate (raw commands, `CARGO_INCREMENTAL=0`, debuginfo off for disk) green on code tree `b364a9f8` (HEAD acc37dfa = d8ad1e5b's code): fmt, clippy core and workspace, scripts/tests all pass; `cargo test --workspace` fails only the six root-only baseline tests. One run of daemon `history_service_restart::a_stop_then_a_start_over_a_connection...` hung over 15 min and was killed; rerun alone: 13 passed (flake, daemon untouched by 575). An earlier gate (gate3) died of ENOSPC and left two empty untracked files at the repo root (removed); its stamp was recorded against HEAD's tree because the gate snapshot included them.
- T013: B10 passed both schemes (`visual/evidence.md`); B1-B5, B7, B8 not run. Needed `apt-get install libxkbcommon-x11-0 mesa-vulkan-drivers` for the showcase on Xvfb.
- Baseline full suite in this container (run as root): 6 pre-existing failures unrelated to 575, permission tests a root user bypasses: core `settings_refuses_save_over_failed_read`, `settings_write_is_logged`, `worktree_leftovers` (2), daemon `mutation_semantics::worktree_delete_blocked_by_an_unremovable_path...`, `settings_service_write_refuses_failed_read`. Noted in the PR body, not fixed.
- No mise (gate commands run raw), gh unauthenticated (no gh calls), reviewers via `claude -p --agent autopilot-reviewer`.
- `ui_glyph_literals` flags any literal U+25CF in `src/`, comments included.

## Environment notes (M2)

- Red: core `the_unread_count_leaves_out_a_closed_session`, `the_other_projects_total_leaves_out_closed_sessions` and `sidebar_attention::the_location_rows_add_up_to_the_switchers_count` failed before T018; green after. Full gate (raw commands, no-fail-fast) on tree bcf3ff4d: fmt, clippy core and workspace, scripts/tests pass; `cargo test --workspace` fails only the six root-only baseline tests listed above.

## Open escalation

None. (D3's decision card is moot: superseded by D4.)

## Follow-ups not done

None.
