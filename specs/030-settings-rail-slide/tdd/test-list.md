---
feature: 030-settings-rail-slide
loop: outside-in
profile: .specify/memory/tdd-profile.md
spec_criteria: 11 # US1 scenarios 1–7, US2 scenarios 1–4
planned_at: a9f54e77
updated_at: bcfcb15d
suite_baseline: red # local only: rechecked on b27ffe62, only the daemon `pi` test `exclusivity` fails on this host; CI on main is green; see cycle-log.md
---

# Test List: The settings rail slides when it collapses and expands

Traces use `US1.n` / `US2.n` for acceptance scenario *n* of a user story, and `FR-`/`SC-` ids from
[spec.md](../spec.md). Behavior names match [quickstart.md](../quickstart.md) §A.2; where a test
passes on `origin/main` before its implementation exists, the `kind` column says `example (mutant)`
and the red is taken against the named mutant (quickstart §A.2 *Fails on*), per the playbook's
deliberate-mutant step.

**Acceptance runner.** The profile's acceptance runner is the sandbox's real-runtime suite, which
has no GUI. The highest level this repository can drive the rail at is the settings view mounted
headless in `crates/micold-client/tests/settings_rail_motion.rs` (built through `view_of`, laid out
and updated with the real `iced` widget tree, frames fed with chosen instants). That is an
integration test of the composed view, not an end-to-end test through a window; what it cannot see
(the clip, the tint's colour, frame pacing) is quickstart §B's recorded visual pass.

`ui::material` is `pub(crate)`, so `tests/` reaches the rail only through `view_of`. A6 (which needs a
`NavigationDrawer`, absent from Settings) and U18 (a rail Settings does not have) are therefore
mounted in `section_list.rs`'s test module on `test_support::renderer()`: the same components,
composed in-crate rather than through the settings view.

## Outer loop: acceptance behaviors

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| A1  | Pressing Collapse in an expanded rail yields at least one frame with a width strictly between 80 and 288, then settles at 80 | US1.1, FR-001, SC-001 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::collapsing_the_rail_slides_it_to_its_icons` |
| A2  | Pressing the expand icon in a collapsed rail yields at least one frame strictly between 80 and 288, then settles at 288 | US1.2, FR-001, SC-001 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::expanding_the_rail_slides_it_back_to_its_labels` |
| A3  | A settled rail, in either state, has exactly `origin/main`'s visible rectangles | US1.3, FR-006, SC-005 | approval | DONE | `crates/micold-client/tests/layout_snapshot.rs` + quickstart §A.3 set comparison |
| A4  | A second press mid-slide turns the width round from the width reached, with no step larger than an uninterrupted slide's over the same interval | US1.4, FR-004, SC-004 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_second_press_reverses_from_where_it_is` |
| A5  | Settings opened with the rail collapsed (or expanded) lays the rail out at 80 (or 288) on its first frame | US1.5, FR-005, SC-006 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::the_rail_is_mounted_at_its_width_rather_than_animating_to_it` |
| A6  | Driven by the same flip and frame instants, a mounted `SectionList` and a `NavigationDrawer` (its node width) are at the same fraction of their width change at every sample, that fraction is not linear at 25% of `MEDIUM_4`, and both settle within `MEDIUM_4` + 2 × the longest gap (in-crate, `section_list.rs`) | US1.6, FR-003, SC-002 | example | DONE | `crates/micold-client/src/ui/material/section_list.rs::tests::the_rail_and_the_sidebar_move_alike` |
| A7  | On every frame of both slides every row with an icon, and the collapse control, keeps its rest height, and each icon's per-frame step stays within the SC-007 bound and heads for the target | US1.7, FR-013, FR-014, SC-007 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::rows_keep_their_height_and_icons_their_line` |
| A8  | With the collapse control focused by keyboard and activated, focus is on that control on every frame of both slides and after they settle | US2.1, FR-007, SC-003 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::the_keyboard_stays_on_the_collapse_control_and_reaches_nothing_hidden` |
| A9  | On every frame of both slides the number of keyboard-reachable controls equals the count at rest, and the section's focusables (by line, height and place on the line, scrolled-out ones included) are the rest sequence in the rest order | US2.2, FR-008, SC-003 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::the_keyboard_stays_on_the_collapse_control_and_reaches_nothing_hidden` |
| A10 | Mid-slide, a cursor inside a drawn form's bounds but outside its row's bounds hovers nothing and a press there publishes no rail message | US2.3, FR-009 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_pointer_past_a_row_reaches_nothing` |
| A11 | Selecting a section mid-slide shows it, moves only the changed rows' icons by `12 · f`, and the slide still settles | US2.4, FR-014 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::selecting_mid_slide_moves_only_the_changed_icons` |

A8 and A9 are held by one test, `the_keyboard_stays_on_the_collapse_control_and_reaches_nothing_hidden`,
because both are read on the same frames of the same slide; a regression in either fails it with its
own message.

## Inner loop: unit behaviors

### `crates/micold-client/src/ui/material/navigation_drawer.rs`

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U1  | Driven at instants 0, +64 ms and +84 ms, a drawer's track is within 0.01 of `EMPHASIZED` at a linear twin's value (0.25 ± 0.001) and not within 0.25 ± 0.05 | FR-003 | example | DONE | `navigation_drawer.rs::tests::the_sidebar_slides_on_the_emphasized_curve` |
| U23 | A drawer (rail 32, handle 6) lays out exactly its rail's width opening at progress 0 and 0.05 (panel 300), `300 · p + 6` closing at 0.125 and 0.5, and a 10 px panel's own width plus the handle open at 1, with the handle and the panel's right edge both at the revealed width. Added in M1 review round 1 (D20); re-cut in round 3, when its closing cases below the floor began showing the rail (U24) | FR-003 (invariant: the swap moves nothing) | example | DONE | `navigation_drawer.rs::tests::a_sliding_drawer_is_never_narrower_than_its_rail` |
| U24 | Closing, a drawer (panel 300, rail 32, handle 6) has its rail on screen at 0.05 and 0.085 and its panel at 0.1; opening at 0.05 its panel; with a 0 px rail and no handle, its panel at 0.01 and its rail at `CLOSED`. Each read from the rail's laid-out x and the state `draw` and `update` read. Added in M1 review round 3 (D21) | FR-003 (invariant: no still panel before the swap) | example | DONE | `navigation_drawer.rs::tests::a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider` |

### `crates/micold-client/src/ui/material/section_list.rs` — derived values

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U2  | `fraction` maps 64 → 0, 168 → 0.5 and 272 → 1 | FR-013, FR-014 | example | DONE | `section_list.rs::tests::a_rows_fraction_runs_from_its_icons_only_width_to_its_labelled_width` |
| U3  | `fraction` clamps below 64 to 0 and above 272 to 1 | FR-013 | example | DONE | `section_list.rs::tests::a_rows_fraction_is_clamped_outside_its_rest_widths` |
| U4  | `form` is `IconsOnly` at `f = 0.001` and not `IconsOnly` just above it | FR-006, FR-013 | example | DONE | `section_list.rs::tests::a_row_draws_its_icons_only_form_at_the_collapsed_threshold_and_no_later` |
| U5  | `form` is `Labelled` at `f = 0.999` for a badged row, and `Marked` just below it | FR-006, FR-015 | example | DONE | `section_list.rs::tests::a_badged_row_draws_its_labelled_form_only_from_the_expanded_threshold` |
| U6  | `form` of an unbadged row strictly between the thresholds is `Labelled` | FR-013 | example | DONE | `section_list.rs::tests::an_unbadged_row_draws_its_labelled_form_between_the_thresholds` |
| U7  | `offset` puts the drawn icon at `x_icons + (x_labelled − x_icons) · f` whichever form is drawn, at `f` 0, 0.5 and 1 | FR-014 | example | DONE | `section_list.rs::tests::the_drawn_icon_is_on_its_line_whichever_form_is_drawn` |
| U8  | For the current row, the line moves by `12 · f` against another row at the same `f` | FR-014 | example | DONE | `section_list.rs::tests::the_current_rows_line_is_twelve_times_the_fraction_right_of_another_rows` |
| U9  | `first_leaf_x` finds a button's glyph in its labelled form and one level deeper in its icons-only form, relative to the node passed | FR-014 | example | DONE | `section_list.rs::tests::the_first_leaf_is_the_glyph_in_both_forms_relative_to_the_node` |

### `crates/micold-client/src/ui/material/section_list.rs` + `keyboard_focus.rs` — focus handoff

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U10 | `TakeFocus` over two enabled controls, the second focused unshown, yields `(1, false)` and leaves both unfocused | FR-007 | example | DONE | `section_list.rs::tests::taking_focus_reports_the_focused_controls_index_and_whether_it_was_shown_and_clears_it` |
| U11 | `GiveFocus { index: 1, visible: false }` focuses the second control with its indicator unshown and clears the first | FR-006, FR-007 | example | DONE | `section_list.rs::tests::giving_focus_focuses_the_nth_control_as_shown_as_it_was_and_clears_the_rest` |
| U12 | The focus operations skip a non-`Focus` state (a ripple) offered through `custom` | FR-007 | example | DONE | `section_list.rs::tests::the_focus_operations_skip_state_that_is_not_focus` |
| U13 | A disabled `TakesTheKeyboard` offers nothing through `custom`, so indices count enabled controls only | FR-008 | example | DONE | `section_list.rs::tests::a_disabled_control_is_not_counted` |

### `crates/micold-client/src/ui/material/section_list.rs` — `Rail` and `RowSlide`, through `tests/settings_rail_motion.rs` (U18 in-crate)

| id  | behavior | traces | kind | state | test |
| --- | --- | --- | --- | --- | --- |
| U14 | On a slide with at least one frame strictly between 80 and 288, on every frame the section region starts at the rail's right edge with its rest content offset | FR-002 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::the_section_follows_the_rail_edge` |
| U15 | The application's collapsed flag changes on the press, before any frame; mid-slide, section selection, Save and Cancel (Settings' only exits, so its closing) each take effect on the frame they happen (section shown changes; `settings_draft` becomes `None`) | FR-010 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::the_state_flips_on_the_press` |
| U16 | Once a slide has settled, a further frame requests no redraw | FR-011 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_settled_rail_asks_for_nothing` |
| U17 | On every frame of both slides the badged row draws its `marked` slot (tinted icon) or a form whose chip node intersects the rail; red from the mutant *`form` returning `Labelled` for a badged row* once `RowSlide` exists | FR-015 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_badged_row_keeps_its_mark` |
| U18 | On a bare rail with a badged no-icon destination (mounted in `section_list.rs`'s test module), its chip is inside its row and the row inside the rail's padding on every frame, the row is never empty, and the column is never taller than its taller rest state | FR-013, FR-015 | example (mutant) | DONE | `section_list.rs::tests::a_row_with_no_icon_keeps_its_chip` |
| U19 | After a pointer press on Collapse and the settled slide, the control is focused and its region's pixels equal the same state with focus cleared | FR-006, FR-007 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_click_on_collapse_shows_no_focus_ring` |
| U20 | A keyboard-focused badged row keeps focus in its drawn form when its badge clears and returns mid-slide | FR-007, FR-015 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_badge_appearing_mid_slide_keeps_focus` |
| U21 | A row pressed just above `f = 0.001` and released after its form parks is not left pressed: on expanding, a bare release over it publishes nothing | FR-009 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_press_held_as_its_form_parks_is_released` |
| U22 | A ripple started just before a form parks still requests frames after the slide settles, and once it has finished the rail requests no frame | FR-009, FR-011 | example (mutant) | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_ripple_in_a_parked_form_settles` |
| U25 | After a pointer click on Collapse, no frame of the collapse draws anything past the rail on the control's line (pixels equal a flag-collapsed twin's) | FR-009, FR-013 | example | DONE | `crates/micold-client/tests/settings_rail_motion.rs::a_ripple_is_cut_off_at_the_rail` |

## Invariants and edge cases still to place

None left unplaced. Spec edge cases map as follows:

- *Repeated presses* — A4 (each press retargets from the value reached).
- *Leaving Settings mid-slide* — A5: the rail's track is widget state, so reopening mounts a new one
  at the flag's value.
- *Window resized mid-slide* — structural: `Rail::layout` computes `80 + 208 · p` from its own track
  only (research R1); the section's `Fill` absorbs the rest. No test beyond A1/A2's widths.
- *Theme or section changed mid-slide* — A11 for section; theme is a view rebuild like it.
- *Badges* — U17; *a destination with no icon* — U18; *focus while clipped* — U19 and §B.2.
- *No failure path*, *refresh rate*, *at rest* — `Progress`'s bounded step (A6's gaps) and U16.

## Out of scope

- The sidebar's collapsed strip itself: unchanged, contract §6.
- Animating the selection marker's move and any reduced-motion preference: spec Assumptions.
- Frame pacing, the look of the clip and the tint's colour: not observable by layout; quickstart §B. Which form a badged row draws is observable, and is U17.
- *Empty or minimal rails* and *several windows*: no code path depends on row count or window
  count; the rail's state is widget state (research R1). No test.
- The user guide's wording (FR-012): documentation, checked in the milestone's Review A.
- `tests/gates/rail_icons_align.rs`: an existing gate kept green (T028), not a new behavior.

## Verification commands

Copied verbatim from `.specify/memory/tdd-profile.md` at planning time:

- Single test: `scripts/build-lock.sh cargo test --test {file} {name} -- --exact`
- File: `scripts/build-lock.sh cargo test --test {file}`
- Full suite: `scripts/build-lock.sh cargo test --workspace`
- Fast subset: `scripts/build-lock.sh cargo test -p micold-core --all-targets`
- Coverage: none (profile `coverage: null`)
- Mutation: none (profile `mutation: null`); mutants are applied by hand

For this feature, unit behaviors in a source module run as
`scripts/build-lock.sh cargo test -p micold-client --lib section_list` (or `navigation_drawer`), and
the motion tests as `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion {name} -- --exact`.
