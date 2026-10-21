# TDD cycle log: 1123 (M1)

The feature has no `tdd/test-list.md` (as 1121): tasks.md's test tasks are the list, and each row
names the task pair it covers. Append only. Baseline: `main` at 9607e14b (design PR #1230 merged),
green in CI.

## M1 (US1, US2: rows, counts from the list)

**Written after the fact (review B M1 r1 F1).** M1 landed as one commit (efff51eb) and its unit
kept no red output, so the red column below does not quote a failing run for T001–T018. What can
still be shown is structural: each test task names symbols that do not exist at the baseline, so
those tests could not compile there (`git grep` at 9607e14b: no `pub mod pr_list`, no
`PrMark::word`, no `CardPrRow`; `COUNT_PAGE` still present). Whether each behavioural assertion
failed on its own before its implementation is **not evidenced**; the close unit's tdd-verify should
treat these rows as green-only. Rows from review fixes onward carry real red output.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001, T002 | K1 `BranchPrList::new` dedupes by number and orders open/draft then merged/closed, newest first; K2 `StatusPhrase::of` precedence and words; `counts()` from the list, *10+* on a full page; `rows(expanded)` 2 collapsed + more, all expanded; `ListedPr`'s `Debug` omits the url | not captured; `micold_core::pr_list` absent at baseline (compile red) | `pr_list` unit tests, `cargo test -p micold-core --all-targets` green in the M1 gate at efff51eb |
| T003, T004, T006 | Q1/Q2 query has `m<i>` with scalar fields, no `oc`/`mc`, no branch name; parser builds lists from `o ∪ r ∪ m`, drops forks, `o`/`r` wins over `m`; fixtures re-recorded | not captured; the Q1 assertion (no `oc<i>`) contradicts the baseline query, which still had `COUNT_PAGE` | `pull_request_query`, `pull_request_parse`, `pull_request_counts`, `pull_request_source` green in the M1 gate |
| T005, T006 | K3 the list's first entry is `select_pull_request`'s pick, exhaustively over up to three own pull requests | not captured; needs `BranchPrList` (compile red) | `pull_request_select` green in the M1 gate |
| T007, T008 | `State.lists` replaced by an `Ok` reading, cleared by `Unavailable` and release, kept through rate limit and passing failure | not captured; `State.lists` absent at baseline (`counts`) | `features_pr_status` 30 passed (review B M1 r1 Verify) |
| T010, T011, T013, T014 | U1 rows drawn collapsed and expanded ("#N", pill word, phrase; *checks failing* in `error`; "+N more"); `PrMark::word`; solid pill contrast ≥ 4.5:1 in every card state, both schemes (D11) | not captured; `CardPrRow`, `.pull_requests`, `PrMark::word` absent at baseline (compile red) | `--lib overview_card pull_request_indicator composition_contrast` 90 passed (review B M1 r1 Verify) |
| T012, T015 | card rows from `State.lists` by `rows(expanded)`: sc. 1, collapsed two of four with "+2 more", expanded all, no list no row | not captured; needs `State.lists` (compile red) | `--test overview` 74 passed (review B M1 r1 Verify) |
| T016, T017 | showcase poses; covered state `overview-pr-rows` | not captured | `showcase_completeness` 43, `layout_snapshot` 84 passed (review B M1 r1 Verify) |
| T009, T018 | user guide | docs | — |
| visual r1 observation (US1 sc. 4, FR-007) | the dash keeps its reason as the tooltip on the expanded card, as on the collapsed one | `cargo test -p micold-client --lib -- the_dash_carries`: `the_dash_carries_its_reason_collapsed_and_expanded` `assertion left == right failed: expanded true left: []` (collapsed passed) | `counts_line` shared by both bodies wraps the dash in its `Tooltip`; `--lib overview_card` 77 passed |
| review A M1 r1 F1, F3, F4 | refactors with no new behaviour: `features::changes::pr_mark` reused, `StatusPhrase::label` takes the first three words from `CardReason`, `PrMark::on_role` merged into the one impl | guards: `each_phrase_has_its_words`, the U1 pill tests | green in the full gate (see ledger) |
