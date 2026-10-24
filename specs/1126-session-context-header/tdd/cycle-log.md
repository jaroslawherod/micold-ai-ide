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

## M1 review B round 1 and full gate notes

- T004 (`cdk::Keyed`) green: the full gate's client lib run, `ui::cdk::keyed::tests` passed (both
  axes).
- T003 also moved `natural_strip_width`, `leading_slack` and `tab_pitch` into
  `features::session_strip`: `strip_overflow` needs them; behaviour unchanged, their tests moved
  with them.
- T016 showcase: red was the gate's `showcase_completeness` (`44 passed; 2 failed`: `cdk::Keyed` has
  no gallery entry or exemption); green after the `Keyed` exemption in `showcase/catalogue.rs`:
  46 passed.
- Full gate red: `feature_registration_cost` failed 2 (`session_strip` had no
  `tests/features_session_strip.rs`; `session` ↔ `session_strip` not in
  `ALLOWED_CROSS_FEATURE_NAMES`). Green after adding the isolation test file and the two allowlist
  lines: `--test feature_registration_cost --test features_session_strip` 7 + 3 passed.
- Rebase onto origin/main 763a59dc (#943 M9, #1120 BUG-1298): `composer_fits::a_long_queue_and_a_long_draft_leave_the_transcript_and_send_in_view`
  failed (`Rectangle { y: 176.6, height: 99.4 }` against the room under the transcript's top).
  #943 M9 took more of the pane, and the header's fixed height came out of the transcript. The
  gate now subtracts the header's measured height (`session_area_fits::header_height`) from main's
  own bound. Green: `layout_snapshot` 107 passed, with the fixture regenerated on the new base and
  only the six session states differing from main.
- Same rebase: `terminal_size_reporting::the_terminal_area_reports_its_size_before_any_output_arrives`
  reported `129×30` (`rows > 30` failed). The header takes rows from the pane. The bound is now
  `cols > 100 && rows >= 30`, because the columns alone tell the measurement from the daemon's
  100×30 seed.
- Rebase onto origin/main afef53af (#1124 M1 status bar, #1299, #1260): three conflicts.
  `ui/terminal.rs` tests: kept main's two FR-007 font tests and M1's move of `failed`/`state_showing`
  into `features::session_strip::tests`. `composer_fits::a_long_queue_and_a_long_draft_…`: main
  replaced the absolute floor with a bound relative to an idle composer in the same pane, which
  already holds under any fixed chrome, so main's bound is kept with M1's `header > 0` check.
  Fixture regenerated with `UPDATE_LAYOUT_SNAPSHOT=1`. Two more gates went red under both chromes
  together: `a_long_draft_grows_the_composer_…` (`height: 247.4` under the `WINDOW.height / 3`
  floor; now less the header's measured height) and `an_interrupted_turn_closes_with_its_notice_…`
  (the 800px window's transcript ended before `You interrupted this turn`; it now paints at twice
  the shared window's height, since it checks words, not geometry). Green: `layout_snapshot` 108 passed.
- Same rebase: `terminal_size_reporting::the_terminal_area_reports_its_size_before_any_output_arrives`
  reported `129×28` (`rows >= 30` failed): 1124's status bar takes two more rows. The bound is now
  `cols > 100 && rows > 1`: the columns tell the measurement from the 100×30 seed, the rows only
  from a degenerate grid; `grid_size_fits_cells_and_floors` still pins the exact numbers.

## M2 (US2 + US4: the tab strip replaces the bar)

Reds were taken against stubs (bodies returning empty or zero) in one run where several tests share a
build; each row quotes its own failure.

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T021, T032 | S1–S4 `StripModel::of`: members and order (agent-started after its origin, folded or not), exactly one marked, stopped per member, `restart` of the shown marked process, status words, `location` `None` when it no longer resolves | stub `of` returning `None`, offsets empty: `cargo test -p micold-client --lib -- session_strip cdk::keyed material::tab`: 12 FAILED, e.g. `the_strip_holds_each_session_then_the_shown_ones_shells_and_plus` panicked at `session_strip.rs:540` (`expect("a session is shown")`), `an_agent_started_session_follows_its_origin_folded_or_not` at `:598` | `StripModel::of` over `location_node(..).unfolded_sessions()`; same run: 31 passed |
| T021 (labels) | a session tab reads `<command> · <name>`, role and status on hover | same stub run: `a_session_tab_names_its_cli_and_session_and_says_its_role_on_hover` panicked at `:731` | same |
| T022 | S5 press, close and menu per member; a member's menu offers *Restart* iff it wears the stopped mark (test sits in `session_strip.rs`'s tests: `ui/mod.rs` has no test module, and its `strip_tab_menu_labels` is `cfg(test)`-public to the crate) | same stub run: `each_member_presses_what_its_bar_control_pressed` at `:756`, `a_members_menu_offers_restart_exactly_when_it_wears_the_stopped_mark` at `:812` | same |
| T023 | S6 `member_offsets`, the three `strip_overflow` rows, `scroll_into_view` either edge | same stub run: `member_offsets_add_fixed_widths_and_gaps` at `:871`, `the_overflow_rows_of_the_s6_table` at `:886`, `a_member_beyond_either_edge_scrolls_just_into_view` at `:905` | same |
| T030 | `SESSION_LABEL_WIDTH`, `SESSION_WIDTH` derived | constants `0.0`: same run, `the_session_tab_width_is_derived_not_literal` `left: 0.0 right: 120.0` | 3 passed in `material::tab` |
| T031 | `new_terminal_chord_label()` per platform | stub `""`; written beside `overview_chord_label`'s test in `tests/keymap.rs` | `--test keymap`: 48 passed |
| T024 | keyed slot follows its key when a member is added before the pressed child | test run against a positional `Row::with_children`: `cargo test -p micold-client --lib -- cdk::keyed`: `a_member_added_before_the_pressed_child_keeps_its_state_with_its_key` `left: [0, 7, 0] right: [0, 0, 7]` | against `Keyed::row`: 3 passed |
| T026 | `tests/terminal_tabs.rs` retargeted to `session_area::member`/`strip`; S5 messages from a `State`; `tests/tab_reveal.rs` gains another session's tab (`Selected`) | the old file against the new view: `--test terminal_tabs` 5 FAILED (`fn tab_strip_row` gone); the new reveal row against `selected` unarmed: `every_strip_control_that_moves_the_mark_arms_the_reveal` "Selected (another session's tab) changes which tab is marked, so it must ask for that tab to be scrolled into view" | `selected` arms the reveal; `--test tab_reveal --test terminal_tabs`: 1 + 7 passed |
| T025 | `tests/session_area_stability.rs` replaces `terminal_bar_stability.rs`: no focus read, no child under an `if` on session shape, no release-focus control, release writers, restart through `restart_message`, `command_caption` label | the old file against the new view: `--test terminal_bar_stability` 2 FAILED (`the_bars_restart_control_asks_which_process_it_is_restarting`, `the_pinned_ai_tab_names_the_sessions_cli`); the new gates pass on arrival (source gates over the new builders) | 6 passed |
| T033, T034 | `session_area::strip` (S7) and `view` header → strip → pane; the bar, `tab_strip_row`, `pinned_ai_tab`, `right_aligned_tabs`, `leading_slack`, `natural_strip_width`, `strip_tab_count`, `session_provider`, `stopped_mark` and their tests removed | structural; the bar's gates went red as listed in T025–T027 | see T027–T029 |
| T027 | `strip_controls_hold_their_size`, `strip_controls_anchor_the_trailing_edge` (replacing `bar_controls_hold_their_size`, `tabs_anchor_the_trailing_edge`), `tab_children_fit` re-anchored on `session.strip.tabs`, `containment`'s strip exemption on `session-area-twelve-tabs`'s keyed row with its member count from `STRIP_MEMBERS` | the old gates against the new view: `--test layout_snapshot` 12 FAILED (`bar_controls_hold_their_size::*`, `tabs_anchor_the_trailing_edge::*`, `tab_children_fit::*`, `containment::*`); mutant (New session and splits swapped, strip drawn under the panes): `strip_controls_*` 3 FAILED, e.g. "strip child 0/0/0/1/1/1/0/0 laid out 0.0dp wide" | `--test layout_snapshot`: 106 passed |
| T028 | `session_area_fits`: 1, 6, 12 tabs, both windows and themes, the strip's four children inside the area and window with no overlap; header, strip, pane area in order, nothing under the panes; two panes (`status-bar-split-panes`): each once above the pane area | same mutant: `the_header_and_the_strip_fit_in_every_session_state_at_both_windows` "the strip's four children … left: 2" | same run |
| T029, T036 | covered states `session-area-one-tab`, `-six-tabs`, `-twelve-tabs`; anchors `session.strip`, `.tabs`, `.new_session`, `.split`; fixture regenerated | structural; fixture diff limited to states that show a session (the strip inserted between the header and the panes, the pane column gone) | `the_layout_matches_the_committed_fixture` |
| T039 | guard: a strip press (each member, close, restart, splits) leaves the keyboard with the terminal; New session opens its confirmation, which takes it | guard, passes on arrival (the presses are the bar's own messages); written after T033, not before T034 as the task asked | `--test terminal_focus -- a_strip_press`: 1 passed |
| T040 | a press in flight follows its member while a shell opens before it, its process stops, or a header chip appears (full-window press, rebuild, release) | against a positional `Row` in `session_area::strip`: `press_in_flight::a_press_survives_a_shell_instance_opening_before_it` `left: [] right: [Session(Selected(..))]` (the other two hold positionally by design) | against `Keyed::row`: 3 passed |
| T035 | showcase poses the session tab: shown, other, stopped | structural | `showcase_completeness` |
| T037 | user guide: *The tab strip above the terminal*; the sessions guide's bar passages corrected | docs | — |
| M2 review A F1 | New session's confirmation (`Started`) arms the strip's reveal: the new helper's tab is appended and marked | `tab_reveal` gains a `Started` row, run against `started` without `arm_tab_reveal`: `every_strip_control_that_moves_the_mark_arms_the_reveal` "Started (New session confirmed: …) changes which tab is marked, so it must ask for that tab to be scrolled into view" | `started` arms the reveal; `--test tab_reveal` passes |
| M2 review A F2 | nothing under the terminal, held inside `terminal::pane` (the geometry gate's pane area fills either way) | the pin over `origin/main`'s `terminal::pane` (M1, bar still there) would fail: its body builds `row![` (the bar), `split_buttons`, `Surface::new(bar, SurfaceKind::Toolbar, …)` | `session_area_stability::the_pane_area_draws_no_bar_under_the_terminal` passes |

## M3 (US3: the Changes button with counts)

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T043, T047 | H5 label rows (`−` U+2212; `no changes` for zero files; `Changes` alone while unknown); H6 `ChangeSummary::of` over text, binary and renamed rows, sums saturating | stubs (`of` returning zero, `changes_label` empty): `cargo test -p micold-core --lib -- context_header`: 2 FAILED, `the_changes_label_names_files_and_lines_or_says_there_are_none` at `context_header.rs:370`, `the_summary_counts_rows_and_sums_their_lines` at `:405` | 6 passed |
| T044, T048 | `HeaderCounts` transitions: follow → `Loading` + `ReadCounts`; current answer → `Ready`/`Failed`, queued `again` reads once more; old seq dropped; `Changed` rereads or queues; worktree gone → `entry: None`, `Idle`; `reading()` `Unknown` unless `Ready` | stubs (`header_counts_read`, `request_counts` returning `Effect::None`): `cargo test -p micold-client --lib -- features::changes::tests`: 4 FAILED (`following_a_new_entry_reads_its_counts` at `changes.rs:3876`, `a_change_rereads_the_counts_or_queues_a_read`, `a_removed_worktree_clears_the_header`, `the_current_counts_answer_is_kept_and_an_old_one_dropped` at `:3859`); `the_counts_are_unknown_unless_a_read_finished` passed on arrival (`reading()` written with the type) | 6 passed; M1's `the_header_follows_the_shown_location` now expects the read a new entry starts |
| T045, T049 | H7 through `State::update`: one `ReadCounts` per newly shown location, none for the same location or with nothing shown, an earlier location's answer dropped, `Changed` with the view on A and the header on B reads both | first run against the implementation: `the_header_reads_the_counts_of_the_location_it_follows` "both lists are read, got None" — the test answered through `State::update`, where `Message::Changes` is the shell's, so the view's list and the header's read were both still under way; the test now answers through `update_changes` as `shell::changes::update` does | `--test features_changes`: 112 passed |
| T050 | the header's location watched while shown and the Overview closed, not twice while the view is open on it (`State::header_watch_entry`); the subscription guarded by it | written with the code (no red taken); source guard `the_header_watch_runs_only_while_a_location_is_shown` beside the view's | `--test idle_subscriptions`: 10 passed; `the_header_watches_its_location_unless_the_view_does` passed |
| T046, T051 | H8 press rows: closed → `ReviewChangesRequested(shown)`; open here → `Closed`, selected; open elsewhere → `ReviewChangesRequested(shown)` | written before T051 but first run after it (no red taken) | `--lib -- session_area`: 3 passed |
| T052, T053 | the Changes button is laid out at the width it asks for at both windows and themes; the header stands over the open Changes view (header, then view); `session-area-changes-open`, anchor `session.header.changes` | the gate against H9's first layout (`row![column![title, chips], button]`): `a_long_title_and_a_pull_request_chip_still_fit` "the Changes button is laid out 212.9873 wide, it asks for 331.07813 (H10)", then, with smaller counts, "the chip 1 … width: 0.0" — at 640×480 the button beside both lines starved the chips | the button moved onto the title line (`column![row![title, button], chips]`, contract H9 updated): `session_area_fits` 3 passed; fixture regenerated, only states showing a session changed (header row gains the button; `session-area-changes-open` new) |
| T054 | user guide *Opening the Changes view* | docs | — |
