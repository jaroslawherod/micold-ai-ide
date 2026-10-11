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

## M2 (2026-10-11)

| Red commit | Green commit | Behaviours (tasks) |
|---|---|---|
| `0ce588f9` | `6df5e7a7` | User status line precedence local → project → user dir, `CLAUDE_CONFIG_DIR`, command-only, padding/refresh carried, bad files fall through, only `statusLine` read; `RelayConfig` serde; `claude_settings_dir` (T018 → T022) |
| `bb7cb07d` | `40d08310` | Relay posts exactly `rate_limits` with the bearer token; user command gets the whole input and `MICOLD_STATUS_RELAY=1`, output copied unchanged; exit status passed through; a silent receiver costs ≤ 500 ms; refused post, missing/malformed relay file silent; over 1 MiB no post; nested runs nothing; the binary's `status-line` is silent and starts no daemon (T019 → T023, T024) |
| `d49280e7` | `a18374e3` | Settings file names the relay with quoted `/` paths; padding/refresh from the user's line; relay file route, token, command, owner-only; off: no block, no relay file, hooks identical; `forget` deletes the relay file; the switch rewrites every prepared session (T020, T021 → T025, T026) |

The binary test was red for the right reason: before the `main.rs` dispatch, `status-line` started
a daemon, which ran until its idle stop.

Green after M2: `plan_usage_user_status_line` 11, `status_relay` 11, `hooks_receiver` 12,
`plan_usage_setting` 5, `plan_usage_route` 15.

## M3 (2026-10-11)

| Red commit | Green commit | Behaviours (tasks) |
|---|---|---|
| `23abe2ba` | `8b311565` | The window keeps the newest pushed reading, `None` and a disconnect clear it, switch off clears and ignores late pushes; the view is the headline label and every window in the details in the given offset; a passed window drops out and none left hides it; the tick runs only while a reading is held, every 30 s; `Icon::PlanUsage` is U+E1AF (T028–T030 → T032–T034) |

Written with the green commit (no separate red run): the `UsageIndicator` size tests (40 px high,
width follows the label, the tooltip adds none), the toolbar test (the indicator only for a current
reading) and the routing test in `main_tests.rs` (Welcome, push, disconnect) (T031, T035, T036).
T031 named `tests/gates/bar_controls_hold_their_size.rs`, which gates the terminal's bottom bar, not
the app bar; the app-bar checks are the component and toolbar unit tests instead.
