# TDD cycle log: 1272 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001 | wording constants moved to `review::source_menu`; client constants re-exported | structural, behaviour-preserving | client `features_changes`, `features_pr_changes` green unchanged |
| T002, T005 | `rows()`: one test per FR-005 state (Absent, Loading, Unavailable, Empty, List, stale List), input order, row invariants over every input, unlisted number marks nothing, chips and Default-entry note | bodies stubbed with `todo!()`: `cargo test -p micold-core --lib review::source_menu` 16 FAILED (0 passed) | 16 passed |
| T003, T006 | `stops()` order and enablement, heading/note/stale no stops, stale-independent, `keeps_open` only for `Kind(_)` | same run (stubs) | same run |
| T004, T007 | `label()` for every data-model §6 row (incl. Default entry), `Label::full()`, `tooltip()` with and without the base line | same run (stubs) | same run |
| T008 | client `PrSection` is `pub use …::PrRows as PrSection` | structural, behaviour-preserving | `features_pr_changes` green unmodified (SC-004) |

## M2

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T009, T012–T014 | control opens on the shown source's stop, closes, dismisses without a source change, walks the stops (saturating, disabled stop reachable), a disabled stop or no highlight takes nothing, picks equal `SourceChosen`, `SourceChosen` closes the list, `PullRequestsRead` keeps it open and re-seats the highlight, label follows previous/next and fallback, last pick wins, `Opened` resets | written before `SourceMenu`, `source_menu_input`, `source_menu_stops` and the four messages existed (compile red); mutation check: without the close in `SourceChosen(Some)` and the stop-keeping re-seat, `choosing_a_source_directly_closes_an_open_list` and `a_refresh_keeps_the_list_open_and_re_seats_the_highlight` FAILED | `features_pr_changes source_menu` 10 passed |
| T010, T015 | chip picks flip the kind like the toggles and keep the list open; Default entry's Committed changes nothing; a chip while a pull request is shown returns to the working tree (remembered PR forgotten, pane reset), flips, reads the list | compile red as above; mutation check: without `show_working_tree` in the toggles, `a_chip_while_a_pull_request_is_shown_returns_to_the_working_tree` and `…_reads_the_tree_at_the_working_tree` FAILED | `features_changes source_menu` 7 passed |
| T011 | Files pick equals `FilesPaneToggled` and closes the list (on and off); `FilesPaneToggled` closes the list; Files stays on across picks with tree and browsed file re-read at the new source and Back cleared; chip with a PR shown and Files on re-reads the tree at the working tree; `Files · …` labels | compile red as above | same run |
