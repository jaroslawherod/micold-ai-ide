# TDD cycle log: 803 M1

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T003 → T004 | `micold_core::navigation::step`: neighbours, wrap, one entry, empty, no/absent current, SC-001 ⌊N/2⌋ bound | 6 of 7 `navigation::tests` failed against the stub (unit 2) | `cargo test -p micold-core --lib navigation`: 7 passed |
| T005 → T006, T007 | location chords by physical key and logical fallback, near misses, reserved chords, labels, `route_key` | the keymap tests were written against stubs in unit 2; in unit 3's first run only the docs test stayed red (`terminal-panes.md lists Ctrl+Shift+,`) | `--test keymap`: 36 passed after T016 |
| T008 → T013 | `location_order` (Default first, drawn order, filter, collapsed/expanded groups, current location kept) and `update` (U1 gate, Next/Previous, wrap, stop, `nav_stop`) | 7 of 9 `--test navigation` failed on assertions against the stub (`left: None`, `left: []`) | `--test navigation`: 9 passed |
| T010 → T014 | `apply_navigation`: `Session` → `Selected` follow-up + reveal armed; `Stop` → `nav_stop`, terminal released, `keyboard_row`, no session started; `None` no change; `Selected` closes the Overview | 3 of 4 `--test navigation_apply` failed on assertions against the stub | `--test navigation_apply`: 4 passed |
| T011 → T015 | focused pane publishes `Message::Navigate` for Ctrl+Shift+. on the physical `.` key, captures it, writes 0 PTY bytes; `to_keymap_physical`; `app_chord` answers the Overview and location chords | 3 tests failed against a `to_keymap_physical` stub returning `Other` and the Overview-only `app_chord` | lib tests green (see gate) |
| T009 / T012 | `row_index`, reveal scroll targets `keyboard_row`, stop clearings (U5) | **Deviation: test-after.** T012 was implemented in unit 2 before its T009 tests ran. Red shown by mutation in unit 3: with `reveal_row` ignoring `keyboard_row` (and `keyboard_row_left` not clearing `nav_stop`), `the_reveal_scroll_reaches_the_keyboard_row_when_one_is_set` failed; restored | `--test sidebar_state`: 25 passed; `current_session_writers` green |
| T016 | user guide rows for the location chords | `the_user_guide_lists_every_navigation_chord_in_both_forms` red until the docs landed | `--test keymap` docs test passes |

# TDD cycle log: 803 M2

| Task | Behaviour | Red evidence | Green |
|---|---|---|---|
| T017 → T019 | `TakesTheKeyboard::shown`: ring on a flag turning true over a mounted wrapper, on a fresh wrapper, none when disabled or Tab-skipped; `TreeItem::keyboard_focus` keeps the row's layout | **Deviation:** tests were written before the code in the same step, but the red run was not captured (the tests did not compile without `shown` and `keyboard_focus`) | `keyboard_focus` and `tree_view` lib tests pass |
| T018 → T020 | `finish_navigation`: terminal without focus → released, `keyboard_row` = the session; with focus → unchanged; after a Stop the next press stays in the sidebar | did not compile before `finish_navigation` existed | `--test navigation_apply` (9 tests) passes |
| T021, T022 | sidebar wraps every row; showcase lists the focused row | `showcase_completeness` lists the new posed variant | `--test showcase_completeness` passes |
