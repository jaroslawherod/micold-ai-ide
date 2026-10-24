# TDD cycle log — 1124 bottom status bar

## M1

The StatusBar and model tests (T003, T007) were written in the same step as their implementation
(T004, T004b, T012), so their first red was not captured then. After review B M1 F1 each behaviour
was shown red against a mutated build (reverted with `git checkout`): the give-way
pass made the identity, every tone `on_surface_variant`, the leading group flush to the edge, the bar
4 dp taller, no busy ring, the connection dot always positive, the fallback check removed, warning
words neutral with no notice, bring-up not busy, and the host daemon named instead of "local".

| Behaviour | Test | Red | Green |
|---|---|---|---|
| `anatomy::status_bar::HEIGHT` = 28, `ALL` holds 48 (T001/T002) | `micold-core/tests/tokens_anatomy.rs` | compile error `cannot find status_bar in anatomy` before T002 | passes after T002 |
| B1 height and width | `the_bar_is_the_anatomy_height_and_fills_the_width` | FAILED (bar 4 dp taller) | ok |
| B2 MD insets and gaps | `items_sit_md_from_the_edges_and_md_apart` | FAILED (`assertion left == right`, leading group at x 0) | ok |
| B4 drop/shorten order, floor | `drops_go_by_rank_before_any_shortening`, `shortening_goes_by_rank_down_to_the_floor`, `of_two_hints_the_rightmost_drops_first`, `a_too_narrow_bar_drops_hints_then_shortens_to_the_floor` | all FAILED (no give-way) | ok |
| B5 a dropped item draws nothing | `a_dropped_item_draws_nothing` | FAILED | ok |
| B3 no item publishes | `a_press_on_any_item_publishes_nothing` | passes by construction (no item has a press handler); a guard | ok |
| Tones map to roles | `tones_map_to_their_roles` | FAILED | ok |
| Busy ring asks for frames only while busy (B, T010) | `only_a_busy_bar_asks_for_frames`, `idle_requests_no_frames::the_status_bar_asks_for_frames_only_while_the_sandbox_comes_up` | both FAILED (no ring) | ok |
| Connection item table (T007) | `disconnected_is_an_error_dot_…`, `read_only_states_are_a_warning_…`, `mismatches_stay_connected_…` | FAILED (dot always positive) | ok |
| Location: host "local", other runtime named (T007, T011) | `a_host_daemon_is_local_and_another_runtime_is_named` | FAILED (host named) | ok |
| Sandbox item table (T007) | `bring_up_stages_are_busy_…`, `standing_conditions_are_warning_words_…`, `a_taken_fallback_wins_over_the_state` | FAILED | ok |
| Location for no project / unbound / default key, steady sandbox words, retry tooltip, empty CLI/usage/hints | 6 model tests | not reached by these mutations (they pass on the mutated build) | ok |
| Contrast of the bar's words and dots (T006) | `the_status_bars_text_and_dots_are_legible_on_its_fill` | a guard on existing roles; passes on first run, as T006 allows | ok |
| No focus stop from the bar (T010) | `terminal_focus::the_status_bar_adds_no_focusable_widget` | a guard: the bar holds no focusable widget by construction | ok |
| The bar is mounted under every body branch (T013) | `features_window::the_install_location_screen_draws_no_status_bar` (premise half) | with the mount line removed from `ui::view_with`: "an ordinary window draws the status bar" FAILED | ok |
| The bar fits at 640 dp (T009) | `status_bar_fits::at_the_smallest_window_the_bars_texts_stay_in_their_boxes` | FAILED on the mutated build (`the bar is 28 dp high … height: 32.0`) | ok |
| Every body 28 dp shorter, the bar at y 772 (T014) | `tests/layout_snapshot.rs` | the fixture differed in all 59 existing states before re-recording; `composer_fits`, `tooltip_clears_its_row`, `notification_kind_rows_sit_under_the_switch` failed against the window's old bottom edge | re-recorded; those gates measure against the body's bottom edge |

T012 also factored the connection banner's words into `ui::connection_words` / `connection_action`,
so the status bar's tooltip and the banner read one source (FR-006; review B M1 F2).

### M3 (T021–T025)

| Behaviour | Test | Red | Result |
|---|---|---|---|
| A lost connection raises no banner (T021, T023) | `banner_is_not_a_snackbar::a_lost_connection_raises_no_banner` | FAILED (`connection_banner` had no `Disconnected` arm) | ok |
| Seven action conditions keep banner and button (T021) | `each_of_the_seven_action_conditions_keeps_its_banner_and_button` | a guard on unchanged code: passes on first run (the four action arms and `sandbox_banner` are untouched, R2/D3) | ok |
| Banner height: none connected, none lost, some for an action condition (T023) | `overview::the_connection_banner_has_height_only_for_an_action_condition` | the old `strip(true) > 0.0` assertion was inverted with the code change; the new `strip(disconnected) == 0.0` fails on the previous `connection_banner` | ok |
| Warning and lost-connection states fit (T022, T024) | `layout_snapshot`, `layout_text_overflow` | new covered states, recorded; `containment::CLIP_REVEALED` renumbered (the lost-connection state no longer shifts the body to child 2; the two-banner state shifts it to 3) | ok |

### M4 (T026–T032)

Tests and the implementation of this milestone landed in one commit; the red step below is stated
only where it was observed, and the rest are marked as written beside the change.

| Behaviour | Test | Red | Result |
|---|---|---|---|
| CLI item names the active session's CLI (T026, T030; US4 s1) | `status_bar::the_session_view_names_the_active_sessions_cli` | the placeholder `cli_usage_and_hints_are_empty_for_now` asserted `cli == None`; replaced with the change, not run red | ok |
| Every other body omits it (T026, T029; US4 s3) | `status_bar::no_other_body_shows_the_cli` | written beside the change (`session_view_visible` extracted first); first runs failed on test set-up (Changes opened through the wrong reducer, a one-run Compare group), not on the code | ok |
| Closing the Overview brings it back (US4 s5) | `status_bar::closing_the_overview_brings_the_cli_back` | written beside the change | ok |
| A background session leaves the model unchanged (T026) | `status_bar::the_cli_is_the_active_sessions_and_background_sessions_do_not_move_it` | a guard: `model` reads `session.active` only | ok |
| Hints read the platform's chords (T026, T030; US4 s4) | `status_bar::the_hints_name_the_overview_and_switcher_chords` | the placeholder asserted empty hints; replaced with the change | ok |
| Focus between panes of different CLIs changes the item (T027; US4 s2) | `main_tests::panes::moving_pane_focus_changes_the_status_bars_cli` | written beside the change (`follow_focus` already writes `session.active`, so it passes by construction) | ok |
| The CLI item and hints fit at 640 dp; hints go first (T028) | `status_bar_fits::*` | the 640 dp test fails if a hint is drawn there; checked by reading, not by a mutated build | ok |
| CLI item in session-view states, hints in every state (T031) | `layout_snapshot` | the fixture differed (additions only: 13 CLI items, hints in every state) before re-recording | re-recorded |

### M5 (T033–T038)

| Behaviour | Test | Red | Result |
|---|---|---|---|
| `model().usage` is `Some` only with a current reading and the switch on (T033, T035; US5 s1) | `status_bar::usage_is_present_only_for_a_current_reading_with_the_switch_on` | FAILED (`expect("a current reading")` on `None`: `model` set `usage: None`) | ok |
| The app bar draws no usage (T033, T035; US5 s2) | `toolbar::the_app_bar_draws_no_usage_indicator` | FAILED (the "37%" label was drawn in the app bar's top strip: `["Micold AI IDE", …, "37%", …, "Overview", …]`) | ok |
| Usage follows the CLI item (T034) | `status_bar_fits::with_room_the_usage_follows_the_cli_item` | passed before the change (the fixture has the usage drawn in the app bar, so its x was already after the CLI item's); a guard, not a red | ok |
| Usage drops with the hints, before any text shortens, at 640 dp (T034; FR-012) | `status_bar_fits::at_the_smallest_window_usage_drops_before_any_text_shortens` | FAILED once on a fixture clash (the sandbox's "Getting image… 42%" matched the usage label; usage reading changed to 37%); the bar's give-way order is the M1 pass, so it passes by construction | ok |
| Usage state fits (T034, T037) | `layout_snapshot`, `layout_text_overflow` (`status-bar-usage`) | new covered state, recorded; no existing state changed (none has a reading) | ok |

§B7 (640 dp): `status_bar_fits` now holds all of CLI item, key hints and plan usage against the
longest texts at `MIN_WINDOW_SIZE`: hints and usage are gone, nothing is shortened beyond its box.
