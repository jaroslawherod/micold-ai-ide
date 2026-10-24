# TDD cycle log: 1126 (M1)

The feature has no `tdd/test-list.md`: tasks.md's test tasks are the list, and each row names the
task pair it covers. Where a test was written beside its implementation, the red is a recorded run
against a stub or with the implementation left out (said in the row), never a reconstructed one.
Append only.

## M1 (US1: the context header above the terminal)

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T001, T002 | `context_header`, `session_strip`, `session_area` modules; the active-session branch draws through `session_area::view` | structural | client builds; `layout_snapshot` green with the fixture unchanged before the header (T002 drew the pane alone, so no wrapper renumbered the tree) |
| T006, T007, T012, T013 | H1–H4 `HeaderText::of`; `StatusPhrase::of_status`/`of_state` agree with `of` | stubs (`of` returning empty tooltips and no chips, `of_state` `None`); `cargo test -p micold-core --lib -- context_header a_status_reading`: 5 FAILED, e.g. `the_title_is_the_cards_and_its_tooltip_names_the_location left: "" right: "CI red on main: daemon tests\n1117_ci-red-on-main"`; `the_branch_chip_is_short_with_the_full_branch_on_hover left: None right: Some(Chip { text: "fix/1117", .. })`; `the_pull_request_chip_names_number_state_and_phrase left: None right: Some("PR #1118 · Open · checks failing")`; `a_status_reading_and_a_listed_pull_request_read_alike left: None right: Some(MergeConflict)` | `cargo test -p micold-core --lib -- context_header pr_list`: 15 passed |
| T005 | `HeaderFollowed` sets, keeps or clears `changes.header.entry` | stub `header_followed` returning `Effect::None` untouched; `cargo test -p micold-client --lib -- changes::tests`: `the_header_follows_the_shown_location` `a new entry is followed left: None right: Some(("/repo", Worktree("feat-1_x")))` | see below |
| T008, T014 | `location_card` is the Overview's `CardModel` for that place | stub `None`; same run: `a_locations_card_is_the_overviews_card_for_that_place` `Default: the header's model is the card's left: None right: Some(CardModel { location: Default, .. title: "Default" ..})` | see below |
| T003 | `StripTab`, predicates and strip arithmetic moved to `features::session_strip` | behaviour-preserving move; the moved tests ran unchanged: same run, `session_strip::tests` 9 passed, `terminal::tests` passed | — |
| T004 | `cdk::Keyed` row and column | promotion; the moved tests run over both axes | see below |
| T010 (title read), T015 | test written beside the hook; a followed worktree's issue title is read once, the Default entry and a repeat ask nothing (R3) | the shell's follow hook left out (`on_header_followed` not called): `cargo test -p micold-client --bin micold-ai-ide -- the_header_reads_its_worktrees_issue_title_once`: `the title's remotes are asked for once left: 0 right: 1` | hook restored; `-- issue_source` all passed |
| T016 | `Tag::max_width` (content-sized under a ceiling, ellipsis above), `Tag::leading` | builders as no-ops: `cargo test -p micold-client --lib -- material::tag`: `a_long_label_is_cut_at_the_ceiling_with_an_ellipsis` `Size { width: 369.4209, .. }` (not cut at 120); `a_leading_element_widens_the_chip_by_itself_and_a_gap` `48.71 vs 48.71`; the short-label case passes on the no-op by design (its answer is the natural size) | 3 passed |
| T009, T017 (`pr_chip_input`) | the chip reads the sidebar's status only while on, for the branch, stale by the sidebar's rule | stub `None`: `cargo test -p micold-client --lib -- session_area`: `the_pull_request_chip_reads_what_the_sidebar_reads left: None right: Some(PrChipInput { number: 1118, .. })` | 2 passed |
| T010 (US1 sc. 5) | a state and pull request change shows in the sidebar row and the header of the same state; the follow step points the header at the shown location | same stub: `the_header_shows_a_change_on_the_refresh_the_sidebar_shows_it` "the header names the sidebar's pull request: None" | same run green |
| T005 (follow step), T010 (H7) | the root's follow step points the header at the shown location; same location: nothing; worktree gone: `None` | follow step removed from `State::update`: `cargo test -p micold-client --test features_changes -- the_header_follows`: `the shown location is followed left: None right: Some(("/projects/p", Worktree("feat-a")))` | restored; `features_changes` 110 passed |
| T011 | H10: title and chips inside the session area, no overlap, 640×480 (sidebar 300) and 1280×800, light and dark, plus a long-title-with-PR variant | header left out of `session_area::view` (empty slot): `cargo test -p micold-client --test layout_snapshot -- session_area_fits`: both FAILED, `the header draws its title` | header drawn: 2 passed |
| T018 | anchor `session.header`; session anchors one level down; fixture regenerated | structural; fixture diff limited to the six session states (header added above the panes, panes one level down) | `layout_snapshot` 107 passed |
| T016 (showcase) | the atoms section poses a glyph-led chip and one cut under a ceiling | structural | `showcase_completeness` (gate) |
| T019 | user guide *The header above the terminal* | docs | — |
| T020 | visual pass B1 (header), B8, B9 (header), B12 | — | `visual-pass.md` |

## M1 review A round 1 fix (F1)

- Red: `the_header_reads_its_title_once_titles_become_readable` with the "became readable" trigger
  left out: `cargo test -p micold-client --bin micold-ai-ide -- the_header_reads_its`: `the shown
  worktree's title is asked for once left: 0 right: 1`.
- Green: the shell asks again when titles become readable while a worktree is shown;
  `-- issue_source`: 63 passed.
