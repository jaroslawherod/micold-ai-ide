# Cycle Log: The settings rail slides when it collapses and expands

Append only. Newest last. Every entry's `red` block is the evidence that the test
existed and failed before the implementation.

## Baseline

- suite: `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3073 passed, 2 failed, 6 ignored
- commit: `a9f54e77`
- recorded: cycle 0, before any change
- red, local only: `micold-daemon --test exclusivity`
  `one_conversation_one_session::a_second_open_of_a_held_pi_conversation_starts_nothing`
  (`exactly one `pi` for one conversation`, left 0, right 1) and `micold-daemon --test pi_launch_wiring`
  `a_pi_session_carries_the_component_only_while_the_switch_is_on` (`launch 0 never reached `pi``).
  Both launch `pi` on this host; CI on `main` is green (latest `CI` runs `success`). Neither touches
  code this feature changes, so every red this feature records is read from its own test, not the
  suite total. Task T001 rechecks them on fresh `main`.

## Recheck on fresh main (T001)

- suite: `mise run gate` in a detached worktree at `b27ffe62` -> fmt and both clippy passes clean;
  `cargo test --workspace` stopped at `micold-daemon --test exclusivity`
  (`a_second_open_of_a_held_pi_conversation_starts_nothing`, `exclusivity.rs:435`). Re-run with
  `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3074 passed, 1 failed, 6 ignored;
  all 13 `scripts/tests/*.test.sh` pass
- the one red is one of the two recorded at the baseline; `pi_launch_wiring` now passes. No other test
  fails, so the loop continues on it (D17)

## Cycle 1: U1 the sidebar slides on the emphasized curve

- test: `crates/micold-client/src/ui/material/navigation_drawer.rs::tests::the_sidebar_slides_on_the_emphasized_curve` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::the_sidebar_slides_on_the_emphasized_curve -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:497:9: the sidebar moved linearly: 0.25 at a quarter of the slide` (1 failed)
- green: `NavigationDrawer::state` builds its `Progress` with `.easing(EMPHASIZED.x1, EMPHASIZED.y1, EMPHASIZED.x2, EMPHASIZED.y2)`
  (`navigation_drawer.rs:114`). Test -> 1 passed. Suite `cargo test --workspace --no-fail-fast`
  -> 3075 passed, 1 failed (the local-only `exclusivity` red above), 6 ignored; fmt and clippy clean
- refactor: none needed, one builder call on an existing constructor
- commit: uncommitted at the time of writing (`--no-commit`; the milestone commits after its reviews)
- notes: the single-test command is the profile's `--lib` form; the profile's `--test {file}` form
  names integration-test files and this test is in-crate (`ui::material` is `pub(crate)`)

## Correction to the two entries above (M1 review round 1)

- *Recheck on fresh main*: there are 14 `scripts/tests/*.test.sh` suites, not 13; all 14 passed. CI on
  `b27ffe62` (`main`) is `success`.
- *Cycle 1*: its commit is `bbc2414b`.

## Cycle 2: U23 a sliding drawer is never narrower than its rail

- test: `crates/micold-client/src/ui/material/navigation_drawer.rs::tests::a_sliding_drawer_is_never_narrower_than_its_rail` (new)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_sliding_drawer_is_never_narrower_than_its_rail -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:535:13: at progress 0.05 the drawer is 21 wide, narrower than its 31 rail` (1 failed)
- green: `NavigationDrawer::layout` computes the revealed width as
  `(full.width * progress).max(rail.size().width - handle_width).clamp(0.0, full.width)`. Module tests
  -> 8 passed. Suite `cargo test --workspace --no-fail-fast` -> 3076 passed, 1 failed (the local-only
  `exclusivity` red), 6 ignored; fmt and both clippy passes clean. Shell suites: `capture-harness`
  failed once under host memory pressure and passed on three re-runs; the other 13 passed
- refactor: none; `handle_width` moved above the width it now feeds
- commit: the M1 round-1 fixes commit on top of `bbc2414b`
- notes: added mid-milestone from M1 code review round 1 (ledger D20, task T045). The behaviour
  predates 030 on a linear track, where it lasted ~33 ms; `emphasized` stretched it to ~150 ms

## Correction and strengthening: U23 (M1 review round 2)

- correction: cycle 2's commit is `4af105ca`. The rail is 32 px wide, not 31: `collapsed_strip` is a
  `STRIP_WIDTH - 1` surface plus a 1 px `Divider` (`divider.rs:18`). The code reads `rail.size()`, so
  only the test's constant and the records were wrong
- test strengthened, no production change: `a_sliding_drawer_is_never_narrower_than_its_rail` now
  asserts the node's exact width (32 closing at 0.05, 0.02, `2 · CLOSED` and opening at 0; 156 closing
  at 0.5) and that the handle sits at the panel's right edge. It passes on `4af105ca`
- mutant: the floor without subtracting the handle (`.max(rail.size().width)`) ->
  `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_sliding_drawer_is_never_narrower_than_its_rail -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:551:13: assertion `left == right` failed: open false at progress 0.05: the drawer's width  left: 38.0 right: 32.0`; restored

## Cycle 3: U24 a closing drawer swaps to its rail once it is no wider

- test: `crates/micold-client/src/ui/material/navigation_drawer.rs::tests::a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider` (new),
  with a stub `rail_showing: bool` on `Track` set only in `state` so the test compiles
- red: `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:611:13: assertion `left == right` failed: open false at progress 0.05 beside a 32 rail: the rail on screen  left: false right: true` (1 failed)
- green: `NavigationDrawer::layout` shows the rail when `showing_rail(progress)` or, closing,
  `full.width * progress <= rail.size().width - handle_width`, and stores that in `Track::rail_showing`;
  `update`, `draw`, `mouse_interaction` and `overlay` read the stored decision instead of recomputing it
  from progress (`overlay` had its own inline copy). The width clamp became `.min(full.width).max(0.0)`,
  equal for every valid size and unable to panic. Module tests -> 9 passed. Suite: see the M1 round-3
  gate below
- refactor: U23 re-cut (next entry); no production refactor
- commit: the M1 round-3 fixes commit on top of `bcfcb15d`
- notes: added mid-milestone from M1 review round 3, A F1 (ledger D22, task T046)

## Re-cut and strengthening: U23 (M1 review round 3)

- why: after cycle 3, U23's closing cases below the floor (0.05, 0.02, `2 · CLOSED`) lay out the rail,
  where panel and handle are both parked and its handle-edge assertion held only by float absorption
  (`-f32::MAX / 4 + 300`). Their width, 32, is the rail's own node, and U24 now holds that the rail is
  what shows there
- test changed, no production change: cases are opening at 0 and 0.05 (panel 300, revealed 26),
  closing at 0.125 and 0.5 (37.5, 150) and open at 1 with a 10 px panel (10); each asserts the node is
  revealed + handle wide and that the handle's x and the panel's right edge both equal the revealed
  width. Passes after cycle 3 (9 passed)
- mutant: the floor without the panel's cap (`.min(full.width)` removed) ->
  `scripts/build-lock.sh cargo test -p micold-client --lib ui::material::navigation_drawer::tests::a_sliding_drawer_is_never_narrower_than_its_rail -- --exact`
  -> `panicked at crates/micold-client/src/ui/material/navigation_drawer.rs:561:13: assertion `left == right` failed: open true at progress 1: the drawer's width  left: 32.0 right: 16.0`; restored

## M1 round-3 gate

- `cargo fmt --all --check`, clippy (core, workspace, `-D warnings`) -> clean
- `scripts/build-lock.sh cargo test --workspace --no-fail-fast` -> 3077 passed, 1 failed (the local-only
  `exclusivity` red, D17), 6 ignored; all 14 `scripts/tests/*.test.sh` pass
- correction: *Correction and strengthening: U23 (M1 review round 2)* was committed as `bcfcb15d`

## Cycle 4: U2–U6 a row's fraction and form (T005, T006)

- tests (new, `crates/micold-client/src/ui/material/section_list.rs::tests`):
  `a_rows_fraction_runs_from_its_icons_only_width_to_its_labelled_width` (U2),
  `a_rows_fraction_is_clamped_outside_its_rest_widths` (U3),
  `a_row_draws_its_icons_only_form_at_the_collapsed_threshold_and_no_later` (U4),
  `a_badged_row_draws_its_labelled_form_only_from_the_expanded_threshold` (U5),
  `an_unbadged_row_draws_its_labelled_form_between_the_thresholds` (U6)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib section_list`, against stubs
  `fraction -> f32::NAN` and `form -> RowForm::Marked`
  -> `section_list.rs:578:9: left: NaN right: 0.0` (U2), `:585:9: left: NaN right: 0.0` (U3),
  `:592:13: left: Marked right: IconsOnly` (U4), `:600:9: left: Marked right: Labelled` (U5),
  `:609:9: left: Marked right: Labelled` (U6); 12 passed, 5 failed
- green: `ROW_WIDTH`, `ROW_WIDTH_COLLAPSED`, `ICONS_ONLY`, `FULL`, `RowForm`, `fraction` (clamped
  `(w − 64) / 208`) and `form` beside `RAIL_WIDTH`. Same command -> 17 passed; `cargo test -p
  micold-client --lib` -> 397 passed
- refactor: the tests' row widths restated as the literals 272 and 64 rather than derived from the
  rail constants the implementation also derives from, so the two cannot agree by construction
- commit: see the M2 milestone commit
- notes: five behaviors in one cycle, as T005/T006 group them: one pure function pair, each test
  red on its own assertion. Boundary mutants (`<` for `<=` at `ICONS_ONLY`, `>` for `>=` at `FULL`)
  are each caught by the `0.001` and `0.999` assertions. Per-cycle suite is the client crate's lib
  tests; the workspace suite runs at the milestone gate (T042)

## Cycle 5: U7–U9 the icon's line and the first-leaf lookup (T007, T008)

- tests (new, `section_list.rs::tests`): `the_drawn_icon_is_on_its_line_whichever_form_is_drawn` (U7),
  `the_current_rows_line_is_twelve_times_the_fraction_right_of_another_rows` (U8),
  `the_first_leaf_is_the_glyph_in_both_forms_relative_to_the_node` (U9; `collapse_control` laid out
  at 272 and at 64, its node moved to (100, 50))
- red: `scripts/build-lock.sh cargo test -p micold-client --lib section_list`, against stubs
  `offset -> 0.0` and `first_leaf_x -> None`
  -> `section_list.rs:661:17: left: 20.0 right: 33.0` (U7), `:675:13: left: 0.0 right: 6.0` (U8),
  `:698:13: left: None right: Some(12.0)` (U9); 17 passed, 3 failed. (A first run failed to compile
  on missing test imports; not counted as red)
- green: `offset` as research R4's formula; `first_leaf_x` follows first children down, summing
  their x. `cargo test -p micold-client --lib` -> 400 passed
- refactor: none needed
- commit: see the M2 milestone commit
- notes: U9's expected values are the anatomy, not the lookup: `PADDING_TEXT` (12) labelled, and
  contract §3's 33 less the rail's 8 icons-only. Mutants caught by construction: adding the node's
  own x (the moved node gives 112), and stopping one level down (icons-only's centring container)

## Cycle 6: U10–U13 the focus handoff operations (T009, T010)

- tests (new, `crates/micold-client/src/ui/material/section_list.rs::tests`, over a column of two
  `Button`s, so each control is a real `TakesTheKeyboard` around a `Ripple`):
  `taking_focus_reports_the_focused_controls_index_and_whether_it_was_shown_and_clears_it` (U10; cases
  `(1, false)` and `(0, true)`), `giving_focus_focuses_the_nth_control_as_shown_as_it_was_and_clears_the_rest`
  (U11; `(1, false)` and `(0, true)`, the other control focused beforehand),
  `the_focus_operations_skip_state_that_is_not_focus` (U12; a `cdk::ripple::Ripple` offered before the
  `Focus`), `a_disabled_control_is_not_counted` (U13)
- red: `scripts/build-lock.sh cargo test -p micold-client --lib section_list`, against no-op `TakeFocus`
  and `GiveFocus` (only `traverse`) and `Focus::held`/`Focus::hold`
  -> `section_list.rs:800:13: left: None right: Some((1, false))` (U10), `:825:17: left: (true, true)
  right: (false, false)` (U11), `:846:9: left: None right: Some((0, true))` (U12), `:863:9: left:
  (false, false) right: (true, true)` (U13); 20 passed, 4 failed. (A first run failed to compile on
  `column` being ambiguous in the test module; not counted as red)
- green: `TakesTheKeyboard::operate` offers its `Focus` through `operation.custom` inside `if self.enabled`;
  `TakeFocus::custom` records the first focused `Focus`'s index and `visible` and clears every focused
  one; `GiveFocus::custom` holds `(seen == index, seen == index && visible)`; both skip a failed
  downcast. `cargo test -p micold-client --lib` -> 404 passed
- refactor: none needed
- commit: see the M2 milestone commit
- notes: mutants caught by construction: offering `custom` outside `if self.enabled` makes U13's index 0
  the disabled button; counting a ripple's state shifts U12's index to 1; giving `visible` to every
  control fails U11's cleared control

## Cycle 7 (red only): A1, A2, A5, A4, U14, U15, U16 on the settings surface (T011–T018)

- structural first (no behavior): `navigation_drawer::parked` made `pub(super)` (T011), `support::layout::view_of`
  made `pub` (T012), and the `Surface` harness in `crates/micold-client/tests/settings_rail_motion.rs` (T013:
  `frame(at, cursor)` relays out while the shell invalidates layout, at most 3 times; `press`, `send`, `bounds`)
- tests (new, `crates/micold-client/tests/settings_rail_motion.rs`):
  `collapsing_the_rail_slides_it_to_its_icons` (A1), `expanding_the_rail_slides_it_back_to_its_labels` (A2),
  `the_rail_is_mounted_at_its_width_rather_than_animating_to_it` (A5), `the_section_follows_the_rail_edge`
  (U14), `a_second_press_reverses_from_where_it_is` (A4), `the_state_flips_on_the_press` (U15),
  `a_settled_rail_asks_for_nothing` (U16)
- red: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion`, on M1's tree (the rail
  still snaps inside the pinned-open drawer)
  -> `settings_rail_motion.rs:311:5: the rail reached 80 without passing through a width between 80 and 288;
  the section's x frame by frame: [80.0, 80.0, …]` (A1), the same at 288 (A2),
  `:395:9: no frame had the rail between its widths, so nothing was followed: [80.0, …]` (U14),
  `:424:10: precondition: the rail never reached a width between its two rest widths` (A4),
  `:452:5:` the same precondition (U15, after its on-the-press assertions had passed);
  A5 and U16 pass. 2 passed, 5 failed. (A first run failed to compile on a double mutable borrow of
  `surface` in U15; not counted as red. T014 alone was run first: 1 passed, 2 failed, the same A1/A2 lines)
- green: pending T020 (Rail) and T021 (wrapper removed)
- refactor: `Surface::page_x` now reads `bounds(PAGE)`
- commit: see the M2 milestone commit
- notes: A4's and U15's reds are their mid-slide preconditions, as tasks T016/T017 predict; U15's on-the-press
  half, A5 and U16 hold on the snap, so each takes its red from the mutants tasks T014, T015, T017 and T018
  name once T020 lands

## Cycle 8 (red only): A6 the rail and the sidebar move alike (T019)

- test (new, `crates/micold-client/src/ui/material/section_list.rs::tests`): `the_rail_and_the_sidebar_move_alike`,
  over a test-only `Mount` (an element with its tree at 1000×400 on `test_support::renderer()`, rebuilt through
  `Tree::diff` on a flip, advanced by `update(RedrawRequested(at))` then a relayout); a `NavigationDrawer` (300 px
  panel, zero-width rail, no handle) and a `SectionList` (badged icons, a toggle), both flipped closed before the
  frame at 0, sampled at 0, 64, 84, then every 64 ms to `MEDIUM_4` + 2 × 64
- red: `scripts/build-lock.sh cargo test -p micold-client --lib the_rail_and_the_sidebar_move_alike`
  -> `section_list.rs:1023:13: at 0 ms the sidebar had covered 0.017046304 of its slide and the rail 1 (widths
  294.8861 and 80)`; 0 passed, 1 failed
- green: pending T020 (Rail)
- refactor: pending
- commit: see the M2 milestone commit
- notes: the drawer is driven through `Widget::update`, answering M1 code r3 (B)'s declined NIT on U1

## Cycle 9: green for cycles 7 and 8, the rail slides (T020, T021)

- green (T020): `Rail` in `section_list.rs` wraps the one rendering `From<SectionList>` builds (its container now
  `Length::Fill`); tree state `Slide { progress }` created at the target, eased on `EMPHASIZED`; `layout` lays the child
  out at `80 + 208 · p` and returns a node of that width; `update` runs `on_layout_frame` toward the target, then
  forwards; `size`, `operate`, `mouse_interaction`, `draw` and `overlay` as T020 names.
  `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion` -> 6 passed, 1 failed:
  `settings_rail_motion.rs:389:13: frame 10: the section's content moved against the edge left: 23.999992 right: 24.0`
  (U14); `--lib section_list` -> 25 passed (A6 green)
- green (T021): the pinned-open `NavigationDrawer` around the rail removed from `settings_view.rs`; U14 unchanged
- test correction (U14, before any further implementation change): its content-offset check compared two `f32`
  differences exactly, and with the rail's edge at a fractional x, `content.x − page.x` rounds to 23.999992. It now
  allows `ROUNDING = 0.001` px, far below a pixel. Not a weakening: the edge assertion stays exact, and a moved
  section moves by whole fractions of a pixel per frame. Rechecked with the drawer wrapper restored: U14 passes
  there too, so its red on M1 was the intermediate-frame requirement alone, as T015 says.
  `settings_rail_motion` -> 7 passed
- mutant reds (each restored with `git checkout` from local WIP commit `51b78f3a`, then re-run):
  - A5, *track created at 0 rather than at the target*: `settings_rail_motion.rs:358:9: assertion left == right
    failed: collapsed: false left: (80.0, 83.54563)`
  - U14, *`Rail::layout` returning a node at the destination width*: `:399:9: no frame had the rail between its
    widths, so nothing was followed: [80.0, …]` (A1, A2, A4, U15 fail too)
  - U15, *`Rail::update` withholding events while its track animates*: `:510:5: a row pressed mid-slide published
    nothing for Terminal: []`
  - U15, *the flag not flipped on the press* (`rail_toggled` a no-op; the application holds no animation state, so
    "held until settled" can only show as not flipped on the press): `:487:5: the flag waited for the slide instead
    of flipping on the press`
  - U16, *`Rail::update` requesting a redraw unconditionally*: `:546:5: a frame after the slide settled asked for
    another`
- suite: `scripts/build-lock.sh cargo test -p micold-client` -> 1065 passed, 1 failed:
  `layout_snapshot::the_layout_matches_the_committed_fixture` (`settings-view-with-validation-error`, path
  `0/0/0/1/0/0/1` gone with the drawer). Expected: T027 regenerates the fixture and compares visible rects (SC-005)
- refactor: none needed
- commit: see the M2 milestone commit
- notes: the disk filled mid-run (0 bytes free); a restore `cp` truncated `section_list.rs` to 32 KiB. Restored from the
  scratchpad copy (`cmp` identical) after deleting `target-shared/debug/incremental` (30 G). No red above was taken
  on the full disk

## Cycle 10: A7, U17, U18 — rows keep their form (T022–T028)

- tests (written in the M2 WIP): `rows_keep_their_height_and_icons_their_line` (A7) and
  `a_badged_row_keeps_its_mark` (U17) in `settings_rail_motion.rs`; `a_row_with_no_icon_keeps_its_chip` (U18) in
  `section_list.rs::tests`
- red (A7), on T020's single-rendering rail (`section_list.rs` from `1b228001`):
  `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion` -> 7 passed, 2 failed:
  `settings_rail_motion.rs:678:17: frame 0 (from collapsed: false): row 0's icon stepped 103.22717 (from 32 to
  135.22717) at width 284.45435; the bound is 0.5170464`. U17 failed there on the tree's shape (`the badged row has
  no slot 1`), which T023 says is not its red. The WIP two-rendering run T022 names was not repeated: `14d9c0e4`'s
  harness predates T013's `Surface`, so the test does not compile against it; the single-rendering red above is
  the one A7 needs
- green (T025): `RowSlide` with `Forms::Sliding([labelled, marked, icons_only])` / `Forms::Single`; `settings_rail_motion`
  -> 9 passed; `--lib section_list` -> 26 passed
- mutant reds:
  - U17, *`form` returning `Labelled` for a badged row mid-slide*: `settings_rail_motion.rs:763:13: frame 3 (from
    collapsed: false): the badged row drew its untinted form with its chip Rectangle { x: 228.39133, … } outside the
    rail Rectangle { x: 0.0, y: 65.0, width: 207.9574, … }`
  - U18, *a no-icon row laid out at 272 like the others*: **survived** the test as first written, because the row's
    node takes the width of the form it lays out, so "the chip is inside its row" held trivially. Test strengthened
    (the row must also end inside the rail); the mutant then fails:
    `section_list.rs:1628:17: at 16 ms the row with no icon Rectangle { x: 8.0, y: 96.0, width: 272.0, height: 40.0 }
    reached past the rail's edge at 269.31317`; the real code passes
- T026: `RowSlide::draw` clips with `with_layer(bounds, ..)` when the drawn form is wider than the row
- T027: fixture regenerated; the `settings.rail` anchor path is unchanged (`0/0/0/1/0/0/0`: `Rail` takes the drawer's
  level and its padded container the drawer child's), so `covered_states.rs` needs no edit. §A.3 set comparison
  against `origin/main`: `SC-005: visible rects unchanged`
- T028: `rail_icons_align` passes, but the mutant *`icons_only` not the last child slot* **survived** it: with the
  forms reordered, every row's last slot is a parked node about −8.5e37 away, where f32 has no precision left, so
  every "centre" is the same number and the column held. Research R10's claim that the gate needs no change was
  wrong. The gate now also requires every glyph's centre inside the rail; the mutant then fails
  (`rail_icons_align.rs:118:5: collapsed, every row's glyph must be drawn inside the rail (0.0–80.0), but these are
  not: row 0 at -85070586659632214952926045871129231360.0, …`) and `layout_snapshot` -> 39 passed
- commit: see the M2 milestone commit

## Cycle 11: US2 — keyboard and pointer held while the rail moves (T031–T041)

- tests (new, `settings_rail_motion.rs`): `the_keyboard_stays_on_the_collapse_control_and_reaches_nothing_hidden` (A8, A9),
  `a_click_on_collapse_shows_no_focus_ring` (U19), `a_badge_appearing_mid_slide_keeps_focus` (U20),
  `a_pointer_past_a_row_reaches_nothing` (A10), `a_press_held_as_its_form_parks_is_released` (U21),
  `a_ripple_in_a_parked_form_settles` (U22), `selecting_mid_slide_moves_only_the_changed_icons` (A11)
- harness corrections before any implementation change: `rail_controls` cuts each control to the rail (a form wider
  than its row has its centre past the rail's edge, where nothing is pressed); A8/A9's section sequence is taken by
  left edge inside the section and compared by line and height (a control that fills the section, or is
  right-aligned, widens or moves with the width the rail gives up); `Surface::pixels` resets the renderer before
  drawing (two back-to-back screenshots differed in 312 of 10240 bytes without it)
- red, on T025 (`scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion`, 10 passed, 6 failed):
  - A8/A9: `frame 23 (from collapsed: false): focus left the collapse control left: None right: Some(Rectangle { x:
    7.9929028, y: 752.0, width: 64.0, height: 40.0 })`
  - U19: `the clicked collapse control lost the keyboard to the slide left: None`
  - U20: `mid-slide: the badged row's control lost the keyboard`
  - A10: `the pointer past the row's edge hovers something left: Pointer right: None`
  - U21: `a row left held down by the slide published [Settings(SectionShown(Terminal))] on a bare release`
  - U22: `96 ms after the press, the slide settled at 80 ms and the ripple has 404 ms left, but nothing asked for a frame`
  - A11 passed on first run, as research R4 expects
- green (T038 focus handoff in `RowSlide::layout`, T039 cursor masked to the row, T040 parked forms finish releases,
  lifts and ripples against a local shell, keeping redraw requests and invalidations): 16 passed. T041 needed no change
- mutant reds (each applied from a script, run, and restored with `git checkout`):
  - A8/A9, *`operate` reaching parked forms*: **survived** as first written, because the count at rest doubled too.
    Strengthened: no control the keyboard reaches may lie off screen, at rest or on any frame; the mutant then fails
    `(from collapsed: false) the keyboard reaches a control off screen at rest left: 6 right: 0`
  - U19, *`GiveFocus` calling `Focusable::focus()`*: `the clicked collapse control draws a focus indicator once the
    slide settles`
  - U20, *an empty placeholder of another widget type in the `marked` slot*: `frame 0 with the badge cleared: the
    badged row's control lost the keyboard`
  - U22, *parked forms' redraw requests dropped*: `96 ms after the press, … nothing asked for a frame`; *forwarded
    forever* (the outer shell asked for the next frame whenever a parked form was updated): `a frame after the ripple
    and the slide were both over asked for another`
  - A11, *`RowSlide` caching `x_labelled` from its first layout*: `row 0's icon moved 0 on the selection at fraction
    0.9829536; expected -11.795444`
- commit: see the M2 milestone commit

## Cycle 12: U25 — a ripple is cut off at the rail (M2 visual pass and review round 1)

- found by: T044's visual pass (B.2, B.4): after a pointer click on **Collapse**, frames 2–4 of the collapse drew the
  control's translucent press layer 100–200 px past the rail over the section. No Part A test covered drawing past
  the rail
- test (new, `settings_rail_motion.rs`): `a_ripple_is_cut_off_at_the_rail`: a surface collapsed by a click and one
  collapsed by the flag, frame for frame; on every frame with the rail between its widths, the pixels of the strip
  from the rail's edge to 288 on the control's line are equal
- red: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion a_ripple_is_cut` ->
  `settings_rail_motion.rs:1346:9: frame 4: the clicked control drew past the rail's edge at 183.99998 into Rectangle
  { x: 183.99998, y: 752.0, width: 104.000015, height: 40.0 }`
- cause: `RowSlide::draw` pushed its clip but handed the form the whole viewport; the ripple cuts each band to the
  viewport and pushes its own clip, which replaces the enclosing one rather than intersecting with it (`ripple.rs`)
- green: the clipped branch draws the form with the viewport cut to the row; `settings_rail_motion` -> 17 passed
- review round 1 (B) test tightening, each checked against its mutant:
  - A9 identifies the section's controls by line, height and place on the line, and keeps controls scrolled out of
    view (F3); the *`operate` reaching parked forms* mutant still fails (`left: 6 right: 0`), the real code passes
  - U18 compares the row with the rail's inner padding, not its outer edge (F4); the *no-icon row at 272* mutant fails
    `at 0 ms the row with no icon Rectangle { x: 8.0, y: 96.0, width: 272.0, height: 40.0 } reached past the rail's
    padding at 276.45435`, the real code passes
- commit: see the M2 milestone commit

## Cycle 13: close-phase test tightening (T048, T049, from `tdd/verification.md`)

- T048, A7: `rows_keep_their_height_and_icons_their_line` also asserts each icon is within `HALF_PIXEL` of
  `x_icons + (x_labelled − x_icons) · f` on every frame, not only that its step is bounded
  - mutant *icon lags its line* (`offset` uses `fraction.powf(1.2)`): the old test passes (`1 passed`); the new one
    fails `frame 2 (from collapsed: false): row 1's icon is at 23.579834, off its line at 23.060314 (fraction
    0.7645912)`; restored, `settings_rail_motion` -> 19 passed
  - the task's suggested mutant, *halve `offset`'s coefficient*, is no test of this check: it moves the icons at rest
    too, which the test reads as its endpoints (`layout_snapshot` is what holds the rest positions); *`fraction ·
    fraction`* is caught by the step bound already, before and after
- T049, U15: `the_state_flips_on_the_press` split into it, `a_section_chosen_mid_slide_is_shown_at_once` and
  `an_exit_mid_slide_closes_settings`, one FR-010 sub-claim each; assertions unchanged; green
- commit: see the close PR

## Cycle 14: U21, U26 — a press held across a form change is a click (T053, after close)

- test (changed): `a_press_held_as_its_form_parks_is_released` asserted, as a precondition, that the release after
  the form change published nothing. Renamed `a_press_held_across_a_form_change_is_a_click`; the release over the
  row now must publish `SectionShown` for that row
- red: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion a_press_held_across` ->
  `settings_rail_motion.rs:1206:5: a press on row 1 held while the rail finished collapsing was dropped on release: []`
- green: `RowSlide` gains tree state `Held(Option<usize>)`: the drawn form that captures a left-button or finger press
  stays drawn until release, lift or lost, then the layout is invalidated if the width asks for another form;
  `settings_rail_motion` and `layout_snapshot` green
- strengthening: U21 also asserts the labelled form is drawn while held and parked (icons-only drawn) after release;
  new U26 `a_press_let_go_off_its_row_still_changes_form`: released off the row, nothing is published and the layout
  is invalidated (the harness lays out every frame, so only the shell's flag shows the runtime would)
- mutants, each restored:
  - *never hold* (`held.0 = Some(drawn)` removed): `the pressed labelled form parked before its press was let go`
  - *never let go* (`take()` → `is_some()`): `once the press was let go, the collapsed row did not take its
    icons-only form`
  - *no relayout on let go*: U21 passes (harness relayouts); U26 fails `a release that changes the row's form left
    the layout valid, so the held form stays drawn`
- commit: see the fix PR
