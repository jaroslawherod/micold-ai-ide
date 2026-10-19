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

## Cycle 2: M2, U18–U26 (T018–T026)

- tests: `card_grid.rs` `mod tests` (zoom_fraction, ribbon end/overlap/range gates, laid-out `Rig` tests).
- red 1 (T021, compile): `cargo test -p micold-client --lib --no-run card_grid` -> `E0422/E0425: cannot
  find ... Zoom / ribbon / zoom_fraction / rest_width / Piece / ZoomState`, `E0599: no method named zoom
  found for CardGrid`, `E0432: unresolved import card_grid::Zoom` (27 errors).
- red 2 (behavioural, by mutation after the fact, review B F1): `Grid::layout` made to ignore the zoom
  (`state.advance(..)` result replaced by `None`), `cargo test -p micold-client --lib card_grid` ->
  `FAILED. 18 passed; 2 failed`: `a_flip_starts_a_zoom_between_the_two_end_widths` and
  `a_card_count_change_mid_zoom_keeps_the_zoom` (pieces equal the rest layout instead of the ribbon).
  The other zoom tests compare against `ribbon(..)` or the rest layout and pass either way, so they
  guard the geometry (U18–U20, U22) rather than the flip; the implementation was written before this
  run, which is the gap this entry records. Mutation reverted.
- green: `cargo test -p micold-client --lib card_grid` -> 20 passed; `--test showcase_completeness` -> 42
  passed (its CardGrid assertion `!interactive` became `interactive == (component == "CardGrid")`).

## Cycle 3: M3, U27–U28 (T027–T031)

- tests: `card_grid.rs` `the_first_visible_card_stays_in_view_across_a_zoom` (30 cards, 400 px view at
  card 20, 4 → 5 columns in 50 px steps, a user scroll mid-zoom); `features_overview.rs`
  `reflowed_changes_no_state`.
- red (T028, compile): the implementation was written before the tests this cycle (the gap review B
  flagged in M2 again), so the red is by mutation: `shift = moved.unwrap_or(0.0) * 0.0` (the grid never
  asks to scroll), `cargo test -p micold-client --lib the_first_visible` -> `FAILED`:
  `step 1 (1350 px): card 20 sits -188 from the view's top, began at 0`. Mutation reverted.
- green: `cargo test -p micold-client --lib card_grid` -> 21 passed; `--test features_overview` -> 8 passed.
- note: `Msg::Reflowed` carries an `f32` y (`features_are_render_free` bars iced types); `Msg` could no longer derive `Eq` (a float); `impl Eq for Msg {}` is
  written by hand (finite offsets) because `Message` and the overlay registry need it.
