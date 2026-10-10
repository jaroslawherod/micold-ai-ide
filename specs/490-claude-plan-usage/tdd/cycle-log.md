# TDD cycle log — #490 M1

Written test-first, in two red/green pairs. Each red commit failed to compile against `main`
because the types, fields and route it names did not exist yet; that is the missing behaviour, not
a typo.

| Red commit | Green commit | Behaviours (tasks) |
|---|---|---|
| `6dff92e2` | `5c923dec` | Model: names, percent label, threshold, current/headline/tie-break, label, details, `format_reset` in UTC and offsets (T003 → T007); `WarnPercent` clamp and parse, settings defaults, clamp on load, save/load, version unmoved (T004 → T008); status-line `extract` and `parse` rules (T005 → T009); wire round trips and the W4 example (T006 → T010) |
| `ac99d214` | `f2c8b438` | `/status` route answers 405/404/403/413/400/200; reading stored, stamped, pushed to every window; newest wins; ten sessions one push; 60 s dedupe; Welcome drops a passed reading; switch off drops (T011 → T014–T016); debug-once logging, never the body (T012 → T016); settings clamp, persist, push, off clears (T013 → T014–T015) |

Review A fixes (2026-10-10), each with its test first:

| Behaviour | Test (red before the fix) |
|---|---|
| A `resets_at` that is not a number or not a time drops the window | `plan_usage_status_line::parse_drops_a_window_whose_reset_is_not_a_time` (the string reset was kept as "no reset") |
| An idle session repeating its own reading does not undo another session's newer one | `plan_usage_route::an_idle_session_repeating_its_reading_does_not_undo_a_newer_one` (stored 420 after 620) |

Green after M1: `plan_usage` 17, `plan_usage_status_line` 8, `settings_plan_usage` 7,
`protocol_roundtrip` 24, `schema_hash` 22, `plan_usage_route` 15, `plan_usage_setting` 4.
