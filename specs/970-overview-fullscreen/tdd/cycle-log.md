# Cycle Log: Overview full-screen mode

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation.

## Baseline

- commit: `d7005808` (origin/main, #1066's merge). The full workspace suite runs in the gate.

## Cycle 1: M1, U1–U17 (T001–T017)

- tests: see `test-list.md` U1–U17. T001 committed alone as `0401e007`.
- red 1 (T007, compile): `CARGO_INCREMENTAL=0 cargo test -p micold-client --no-run`
  -> `error[E0425]: cannot find function full_screen_label / full_screen_icon / full_screen_tooltip`,
  `cannot find function overview_full_screen_chord_label in module crate::keymap`,
  `error[E0599]: no variant named FullScreenToggled found for enum features::overview::Msg`,
  `no variant ... named FullScreen / FullScreenExit found for enum icons::Icon`
- red 2 (T007, behavioural; stubs: `full_screen()` false, toggle ignored, chord false, label "",
  drawer ignores `retracted`, header text empty; icons are data and went in real):
  `cargo test -p micold-client --no-fail-fast -- navigation_drawer overview keymap icons app_chord full_screen`
  -> lib `FAILED. 55 passed; 6 failed` (`the_full_screen_chord_toggles_full_screen`, four drawer
  gates, `the_full_screen_toggle_names_its_action_and_chord`); `features_overview` `0 passed; 4
  failed`; `keymap` `4 passed; 2 failed`; `overview` `4 passed; 1 failed`
  (`settings_open_and_closed_keeps_full_screen`; U6, U16, U17 fail on the same entry assertion
  `state.overview.full_screen()` and were outside that filter). Green against the stubs, as
  expected: U9 (icons, data) and U14 (un-retraction is the drawer's existing open/close geometry).
- green: same filters plus `sidebar showcase the_gallery every_` -> exit 0. One existing test
  moved: `a_card_press_requests_its_activation` presses at y 120, below the new header row.
- refactor: the drawer's `layout` split into `layout_shown` (open/close) and the retraction on top;
  `draw` into `draw_children`, clipped to the drawer while it retracts.
