# TDD cycle log — 1124 bottom status bar

## M1

| Behaviour | Test | Red | Green |
|---|---|---|---|
| `anatomy::status_bar::HEIGHT` = 28, `ALL` holds 48 (T001/T002) | `micold-core/tests/tokens_anatomy.rs` | compile error `cannot find status_bar in anatomy` before T002 | passes after T002 |
| The bar is mounted under every body branch (T013) | `tests/features_window.rs::the_install_location_screen_draws_no_status_bar` (premise half) | with the mount line removed from `ui::view_with`: "an ordinary window draws the status bar" fails, the painted text holds no bar item | passes with the mount |
| StatusBar layout B1–B5, tones, idle frames (T003) and the model's wording tables (T007) | `ui/material/status_bar.rs`, `ui/status_bar.rs` unit tests | written in the same step as T004/T004b/T012, red not recorded separately | 23 pass |
| Every body is 28 dp shorter, the bar at y = 772 (T014) | `tests/layout_snapshot.rs` | the fixture differed in all 59 existing states before re-recording; three gates (`composer_fits`, `tooltip_clears_its_row`, `notification_kind_rows_sit_under_the_switch`) failed against the window's old bottom edge | re-recorded; the gates measure against the body's bottom edge |
