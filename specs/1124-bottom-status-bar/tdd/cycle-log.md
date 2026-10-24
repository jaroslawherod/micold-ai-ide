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
