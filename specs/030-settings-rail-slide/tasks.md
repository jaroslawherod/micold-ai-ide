---

description: "Task list for feature 030 — the settings rail slides when it collapses and expands"
---

# Tasks: The settings rail slides when it collapses and expands

**Input**: Design documents from `/specs/030-settings-rail-slide/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/rail-slide.md](./contracts/rail-slide.md),
[quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), every implementation task is
preceded by a task that writes a test and **observes it fail**. Where quickstart §A.2 names a mutant
rather than `origin/main` or the WIP as what a test fails on, the test task says so: the test is
checked against that mutant by hand once, per the TDD playbook's deliberate-mutant step. Tasks with
no test task of their own are setup, verification, documentation, structural changes (no behaviour
of their own), or the draw-time clip, which is the plan's one GUI exception, backed by quickstart §B.

**Behavior markers**: `[A#]` and `[U#]` name the behaviors in [tdd/test-list.md](./tdd/test-list.md)
a task serves. `/speckit.tdd.run` ticks a marked task once every behavior it names is `DONE`;
`/speckit.implement` covers the unmarked ones.

**Reuse**: WIP commit `14d9c0e4` (ledger D6, research R11) supplies `settings_view.rs`'s wrapper
removal, `support/layout.rs`'s `pub fn view_of`, the `Rail` shell and its `Progress` wiring, the test
harness in `tests/settings_rail_motion.rs` and the user-guide paragraph. Its two-rendering form choice
(`Rail::shows_labels`, `on_screen`, `hand_focus`, `Focused`, `FocusNth`, the `LABELLED`/`ICONIC`
indices) is **not** reused; research R1–R5 replace it. Its 027 spec, plan, task and bug edits are not
reused (D5). Read a WIP file with `git show 14d9c0e4:<path>`.

**Documentation**: Per Principle VII, `docs/user-guide/settings.md` gains the collapse control and
its slide (FR-012) in the milestone that ships the slide, and `section_list.rs`'s module docs say the
component owns its slide.

**Cross-platform**: Per Principle VI, nothing here branches on the host OS. The slide is measured in
elapsed time and layout units.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: The user story the task serves (US1, US2)
- Every task names the exact file it touches

## Path Conventions

Rust workspace. Paths are repository-relative. `section_list.rs` is
`crates/micold-client/src/ui/material/section_list.rs`; the motion tests are
`crates/micold-client/tests/settings_rail_motion.rs`.

## Phase order, and why it is not the template's

The sidebar's curve (US1 acceptance scenario 6, FR-003's sidebar half) needs nothing the rail's
foundation builds, and ships first as milestone M1. So Phase 2 is that half of US1, and the rail's
foundation follows as Phase 3. Task ids stay in execution order, except T045 and T046, added to M1 in review.

---

## Phase 1: Setup

**Purpose**: Know the ground is green, and have the WIP at hand.

- [X] T001 Run `mise run gate` in a detached worktree at the current `origin/main` (`git worktree add --detach <scratchpad dir> origin/main`, removed with `git worktree remove` afterwards) and record the result. The baseline on `a9f54e77` failed locally in two daemon `pi` tests only (`exclusivity` `a_second_open_of_a_held_pi_conversation_starts_nothing`, `pi_launch_wiring` `a_pi_session_carries_the_component_only_while_the_switch_is_on`; `tdd/cycle-log.md`) while CI on `main` is green. If those two, and only those, fail again, record it in `specs/030-settings-rail-slide/autopilot.md` under *Follow-ups not done* and continue: CI is the merge gate and they are not this feature's code. Escalate only if CI on `main` fails too, or another test fails; nothing here is fixed by this feature
- [X] T002 Write the WIP's reusable files to the session scratchpad with `git show 14d9c0e4:<path>` for `crates/micold-client/src/ui/material/section_list.rs`, `crates/micold-client/src/ui/settings_view.rs`, `crates/micold-client/tests/settings_rail_motion.rs`, `crates/micold-client/tests/support/layout.rs` and `docs/user-guide/settings.md`, so later tasks copy named pieces rather than cherry-picking the commit (research R11)

---

## Phase 2: User Story 1, part A — the sidebar moves on its curve (Priority: P1)

**Goal**: The worktree sidebar's panel hides and shows on the *sidebar slide* row's `emphasized`
curve instead of at a constant speed (FR-003, D7).

**Independent test**: Drive a `NavigationDrawer`'s track from closed to open with frame instants;
at 25% of `MEDIUM_4` in linear time its progress is the emphasized curve's value there, not 0.25.

- [X] T003 [US1] [U1] Write a failing unit test `the_sidebar_slides_on_the_emphasized_curve` in `crates/micold-client/src/ui/material/navigation_drawer.rs`'s test module: build a drawer closed, open it, and drive its `Track`'s `Progress` with `on_frame` at instants 0, +64 ms and +84 ms. `Progress` steps one `FRAME` (16 ms) on a track's first frame and caps later gaps at `MAX_STEP` (64 ms) (`crates/micold-client/src/ui/cdk/motion.rs`), so those instants are 25% of `MEDIUM_4` in linear time. Drive a linear twin `Progress::new(..)` with the same instants beside it. Assert the twin reads 0.25 ± 0.001, and the drawer's value is **not** within `0.25 ± 0.05` and is within 0.01 of 0.607, the value of `cubic-bezier(0.2, 0, 0, 1)` at x = 0.25 (contract §2 *Curve*; `cdk::motion::ease` is private, so the reference is a named constant in the test, with a comment saying it was solved for x = 0.25 by bisection). Observe it fail on the linear track
- [X] T004 [US1] [U1] Build the drawer's track as `Progress::new(..).easing(EMPHASIZED.x1, EMPHASIZED.y1, EMPHASIZED.x2, EMPHASIZED.y2)` in `NavigationDrawer::state` in `crates/micold-client/src/ui/material/navigation_drawer.rs` (`Progress::new` at line 113), importing `micold_core::tokens::motion::EMPHASIZED`, to make T003 pass. The strip swap in `crates/micold-client/src/ui/mod.rs` is untouched (plan *The sidebar curve*)
- [X] T045 [US1] [U23] Added in M1 review round 1 (D20): `emphasized` lingers where `full · p` plus the handle is narrower than the 32 px rail, so the main pane crept left of the rail's edge for ~150 ms and jumped back at the swap (linear passed through in ~33 ms). Write a failing unit test `a_sliding_drawer_is_never_narrower_than_its_rail` in `navigation_drawer.rs`'s test module (panel 300 wide, rail 32, `resize_handle::WIDTH` handle; closing at progress 0.05, 0.02 and `2 · CLOSED`, and opening at 0, the node is exactly 32 wide; closing at 0.5 it is 156; the handle sits at the panel's right edge throughout), then floor the revealed width in `NavigationDrawer::layout` at the rail's width less the handle's, clamped to the panel's
- [X] T046 [US1] [U24] Added in M1 review round 3 (D21): with the floor, a closing sidebar's width stopped at 32 px ~150 ms before the rail swapped in at `CLOSED`, a still sliver of panel. Write a failing unit test `a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider` in `navigation_drawer.rs`'s test module (panel 300; rail 32 with a handle: closing at 0.05 and 0.085 the rail is on screen, closing at 0.1 and opening at 0.05 it is not; rail 0 without a handle: closing at 0.01 it is not, at `CLOSED` it is; each read from the rail's laid-out x and from the drawer state's recorded decision), then swap in `NavigationDrawer::layout` once a closing panel's `full · p` is no wider than the floor, record the decision in `Track`, and have `update`, `draw`, `mouse_interaction` and `overlay` read it. U23's closing cases below the floor now show the rail, so U23 is re-cut to opening cases, a case above the floor, and a panel narrower than the floor

**Checkpoint**: `mise run gate` green. Hiding and showing the worktree sidebar eases in and out.
This is milestone M1.

---

## Phase 3: Foundational — the rail's building blocks (blocks Phases 4 and 5)

**Purpose**: The pure decisions, the focus operations and the test harness that both the slide
(US1) and its keyboard and pointer guarantees (US2) are built from. No story is delivered here.

**⚠️ CRITICAL**: Complete this phase before Phase 4.

- [X] T005 [U2] [U3] [U4] [U5] [U6] Write failing unit tests in `section_list.rs`'s test module for `fraction(w)` and `form(f, has_badge)` (data-model *Derived values*): `fraction` maps 64 → 0, 272 → 1, 168 → 0.5, and clamps 40 → 0 and 300 → 1 (`clamp((w − 64) / 208, 0, 1)`); `form` returns `IconsOnly` at `f = 0.0` and `f = 0.001`, `Labelled` at `f = 0.999` and `1.0`, at `f = 0.5` returns `Marked` when badged and `Labelled` when not, and just inside each threshold `form(0.002, false)` and `form(0.998, false)` are `Labelled` while `form(0.002, true)` and `form(0.998, true)` are `Marked` (`IconsOnly if f ≤ 0.001; Labelled if f ≥ 0.999; else Marked if badged, else Labelled`). Declare only the signatures needed to compile; observe the assertions fail
- [X] T006 [U2] [U3] [U4] [U5] [U6] Add `enum RowForm { Labelled, Marked, IconsOnly }`, the thresholds as named constants beside `RAIL_WIDTH`, and `fraction` and `form` in `section_list.rs` to make T005 pass
- [X] T007 [U7] [U8] [U9] Write failing unit tests in `section_list.rs` for `offset(f, x_labelled, x_icons, x_drawn) = x_icons + (x_labelled − x_icons) · f − x_drawn` (research R4): with `x_labelled` 20 and `x_icons` 33, the drawn x (`x_drawn + offset`) equals `33 − 13·f` for `x_drawn` 20 and 33 at `f` 0, 0.5 and 1; with `x_labelled` 32 (current row) the drawn x moves by `12·f` against 20 at the same `f` (FR-014's selection exception); and for the first-leaf lookup, a laid-out `Button` form's first leaf in depth-first order is its glyph both at `button/0/0/0` (labelled) and one level deeper (icons-only's centring container), read relative to the form's node. Declare only the stubs needed to compile (`fn offset(..) -> f32 { 0.0 }`, `fn first_leaf_x(..) -> Option<f32> { None }`); observe the assertions fail
- [X] T008 [U7] [U8] [U9] Add `offset` and the first-leaf x lookup (`first_leaf_x(&layout::Node) -> Option<f32>`, relative to the node passed) in `section_list.rs` to make T007 pass
- [X] T009 [U10] [U11] [U12] [U13] Write failing unit tests in `crates/micold-client/src/ui/material/section_list.rs`'s test module for the focus handoff operations (research R5): running `TakeFocus` over a subtree of two enabled `TakesTheKeyboard` controls, the second focused with `visible = false`, yields `Some((1, false))` and leaves both unfocused; `GiveFocus { index: 1, visible: false }` over a fresh pair focuses the second with `visible` false and clears the first; an operation that also receives a `RippleState` through `custom` skips it without panicking; and a disabled `TakesTheKeyboard` offers nothing through `custom`, so indices count enabled controls only (`custom` offered under the same `enabled` condition as `focusable`). Declare only what is needed to compile: `TakeFocus` and `GiveFocus { index, visible }` as `Operation` implementations that do nothing, and crate-private access to `Focus`'s `focused` and `visible` flags in `keyboard_focus.rs` (the tests set and read them); observe the assertions fail
- [X] T010 [U10] [U11] [U12] [U13] In `crates/micold-client/src/ui/material/keyboard_focus.rs`, make `TakesTheKeyboard::operate` also call `operation.custom(None, layout.bounds(), state)` with its `Focus` inside the existing `if self.enabled` branch, (the flags' access came with T009's stubs). In `section_list.rs`, add crate-private `TakeFocus` and `GiveFocus { index, visible }` operations that traverse and `downcast_mut::<Focus>()` what `custom` offers, skipping other types. Makes T009 pass
- [X] T011 [P] Make `parked` `pub(super)` in `crates/micold-client/src/ui/material/navigation_drawer.rs` so `RowSlide` parks forms the way the drawer does. No test task: visibility change only (structural)
- [X] T012 [P] Make `view_of` `pub` in `crates/micold-client/tests/support/layout.rs` (WIP hunk). No test task: visibility change only
- [X] T013 Create `crates/micold-client/tests/settings_rail_motion.rs` from the WIP's harness only — its module docs (retitled to 030 and FR-001–FR-015), the `Focusables` and focus-at-bounds operations, and `Surface` with `new`, `node`, `page_x`, `toggle`, `run`, `focusables` and `toggle_and_trace` — without the WIP's four tests, which T014 and T031 rewrite. Extend `Surface::frame` to iced_winit's loop (research R9): it takes `at: Instant` and `cursor: Option<Point>`, runs `update(RedrawRequested(at))` with that cursor, relays out, and repeats update-then-relayout with the same instant while the shell reports the layout invalidated, at most three times; and expose whether the last frame requested a redraw. `frame` and `run` return the messages published, and `Surface::press(at: Point)` presses and releases the left button there over one frame and feeds every published message through `micold_client::app::State::update` (`crates/micold-client/src/app.rs`) before the next `view_of`, and `Surface::send(message)` feeds a message the same way (T017's `SettingsMsg::Opened`), so T017 presses Save and Cancel as a user does rather than setting state. `tests/` reaches the rail only through `view_of`, because `ui::material` is `pub(crate)` (`crates/micold-client/src/ui/mod.rs`): tests that mount a bare `SectionList` or a `NavigationDrawer` (T019, T024) live in `section_list.rs`'s test module instead, on `test_support::renderer()`. No test task: this is test infrastructure; every helper is exercised by the tests that follow

**Checkpoint**: `scripts/build-lock.sh cargo test -p micold-client --lib section_list` green.

---

## Phase 4: User Story 1, part B — watch the rail slide in and out (Priority: P1) 🎯 MVP

**Goal**: The settings rail slides between 288 and 80 on the sidebar's duration and curve; every
row keeps its height and its icon's line, badges stay marked, and the rest states are exactly
today's.

**Independent test**: Open Settings, press Collapse, pump frames and record the rail's width until
it settles; again for expand. Both traces pass strictly between 80 and 288, the section starts at
the rail's edge on every frame, and every row's icon stays on its line (quickstart §A.2).

### Tests for the rail's width (write first, observe fail)

- [X] T014 [US1] [A1] [A2] [A5] Write failing tests in `settings_rail_motion.rs`: `collapsing_the_rail_slides_it_to_its_icons` and `expanding_the_rail_slides_it_back_to_its_labels` (a trace from `toggle_and_trace` has a width strictly between 80 and 288 each way and settles at the other, FR-001, SC-001) and `the_rail_is_mounted_at_its_width_rather_than_animating_to_it` (first frame at 288 expanded and 80 collapsed, FR-005, SC-006), adapted from the WIP. Observe the first two fail on `origin/main`'s snap; the mount test passes there, so once T020 lands, check it against the mutant *the track created at 0 rather than at the target* (A5), observe it fail, restore, and record the red in `tdd/cycle-log.md`
- [X] T015 [US1] [U14] Write test `the_section_follows_the_rail_edge` in `settings_rail_motion.rs`: toggle, and require at least one frame whose rail width is strictly between 80 and 288 (so the test cannot pass on a snap); on every frame of both slides the section region's x equals the rail's right edge and the section content's offset from that edge equals its rest offset (FR-002). It fails on `origin/main` only through the intermediate-frame requirement, so once T020 lands, also check it against the mutant *`Rail::layout` returning a node at the destination width while laying its child out at the animated width* (the section jumps to the destination edge), observe it fail there, restore, and record the red in `tdd/cycle-log.md`
- [X] T016 [US1] [A4] Write failing test `a_second_press_reverses_from_where_it_is` in `settings_rail_motion.rs`: toggle, pump to mid-slide and require the width reached to be strictly between 80 and 288, toggle again; the next frame's width starts from the width reached, and no frame-to-frame step of the reversal exceeds the largest step an uninterrupted slide makes over the same interval (FR-004, SC-004). Observe it fail on `origin/main` at the mid-slide precondition (the snap never reaches an intermediate width)
- [X] T017 [US1] [U15] Write test `the_state_flips_on_the_press` in `settings_rail_motion.rs`: after the press, before any frame, the application state's `settings_rail_collapsed` has changed; and mid-slide, pressing another section's row publishes `SectionShown` on that frame and the section shown changes at once. Then each of Settings' two exits is taken mid-slide, each after its own reopen (`SettingsMsg::Opened` through `app::State::update`) and a toggle pumped to an intermediate width: pressing **Save** publishes `SettingsMsg::Saved` on that frame, and pressing **Cancel** publishes `SettingsMsg::Cancelled`; after each, `state.settings.settings_draft` is `None` with no frame advanced in between (FR-010). These are FR-010's closing too: Settings has no close button and Escape does not dismiss it (it is not a registered surface, `overlay/registry.rs`), and only `saved` and `cancelled` clear the draft (`features/settings.rs`). Persisting the save is the shell's, which the reducer does not run, so the draft closing is the observable. Once T020 lands, check it against the mutants *the flag held until the slide settles* and *`Rail::update` withholding events from its child while its track animates* (selection delayed), observe each fail, restore, and record the reds in `tdd/cycle-log.md`. Save and Cancel sit outside the rail and pass both mutants; they are held as regression coverage of FR-010's list
- [X] T018 [US1] [U16] Write test `a_settled_rail_asks_for_nothing` in `settings_rail_motion.rs`: after a slide settles (frames past `MEDIUM_4` + 2 × gap), a further frame requests no redraw (FR-011). It passes on `origin/main`, which never animates, so once T020 lands, check it against the mutant *`Rail::update` requesting a frame unconditionally*, observe it fail there, restore, and record the red in `tdd/cycle-log.md`
- [X] T019 [P] [US1] [A6] Write failing test `the_rail_and_the_sidebar_move_alike` in `section_list.rs`'s test module (in-crate because `ui::material` is `pub(crate)`; Settings has no drawer to compare against). Add there a small test-only mount: an `Element` with its `Tree`, laid out at a fixed size with `test_support::renderer()`, rebuilt through `Tree::diff` when its builder's flag changes, and advanced by `update(RedrawRequested(at))` followed by a relayout. Mount a `NavigationDrawer` (open, its rail child a zero-width `Space`, so its node width is the panel's slide down to 0) and a `SectionList` (expanded) with the rail's sections, flip both before the first `update` at instant 0, and drive both with the same frame instants (gaps ≤ 64 ms, beginning 0, +64 ms, +84 ms as in T003, so the third sample is 25% of `MEDIUM_4` in linear time); at every sample the drawer node's width (mounted with no handle and a fixed-width panel; its panel child keeps its full width and is translated) and the rail's node width have completed the same fraction of their change (±0.01), the fraction at that third sample is not `0.25 ± 0.05`, and both settle within `MEDIUM_4` + 2 × the longest gap (FR-003, SC-002). Observe it fail (no rail slide yet)

### Implementation for the rail's width

- [X] T020 [US1] [A1] [A2] [A4] [A5] [A6] [U16] Add `Rail` to `section_list.rs` from the WIP shell: tree state `Slide { progress }` created at the target of the first `collapsed` seen; constants `SLIDE = Duration::from_millis(duration::MEDIUM_4)` and `SLIDE_CURVE = motion::EMPHASIZED`; `layout` lays its one child out at `80 + 208 · progress.value()` and returns a node of that width; `update` calls `on_layout_frame` toward `collapsed ? 0.0 : 1.0` and then forwards to the child; `size()` is `Fixed(width_of(collapsed))`; `operate`, `mouse_interaction`, `draw` and `overlay` forward to the one child (no overlay constructed). Convert `SectionList` into `Rail` around a single rendering built as today's `From<SectionList>` builds the rail's column. Keep `animated_layout_relayouts.rs` green: only `Rail::layout` reads the track. Makes T014, T016, T018 and T019 pass
- [X] T021 [US1] [U14] [U15] Remove the pinned-open `NavigationDrawer` wrapper around the rail in `crates/micold-client/src/ui/settings_view.rs` and its comment, taking the WIP hunk with its comment reworded to cite 030 FR-001 instead of 027 FR-026f (research R11, D3: the wrapper forwards operations to its parked child). T015 and T017 must be green after it

### Tests for the rows (write first, observe fail)

- [X] T022 [US1] [A7] Write failing test `rows_keep_their_height_and_icons_their_line` in `settings_rail_motion.rs`: on every frame of both slides, every destination row with an icon and the collapse control have their rest height, and between consecutive frames each icon's x moves by at most `|x_labelled − x_icons| · |Δf| + 0.5` and never away from the target's rest x (FR-013, FR-014, SC-007). Observe it fail on T020's single-rendering rail, whose rows are laid out at the intermediate widths (labels wrap, heights change, the icon swaps position at one width); also run it once against the WIP's two-rendering rail (`git show 14d9c0e4`, 13dp step at the swap) and record that failure. *Done in M2:* the WIP run was not repeated, because the WIP's harness predates T013's `Surface` and the test does not compile against it; the red came from T020's rail (cycle-log cycle 10)
- [X] T023 [P] [US1] [U17] Write failing test `a_badged_row_keeps_its_mark` in `settings_rail_motion.rs`: on every frame of both slides, the badged row either draws its tinted form or draws a badge chip whose rectangle intersects the rail's bounds (FR-015). Both are read from layout: the tinted form is the row's `marked` child slot (slot 1 of `RowSlide`, tinted by construction, U4) being the one not parked, and the chip is the drawn form's badge node. Its first red cannot come from earlier code: T020's single rendering keeps the chip inside the rail, and the WIP has no `RowSlide` slots to read, so a failure there would be the tree's shape rather than FR-015. So once T025 lands, take the red from the mutant *`form` returning `Labelled` for a badged row mid-slide* (the chip is cut off at the rail's edge while the untinted form is drawn), observe it fail, restore, and record it in `tdd/cycle-log.md`
- [X] T024 [P] [US1] [U18] Write test `a_row_with_no_icon_keeps_its_chip` in `section_list.rs`'s test module, on T019's test-only mount, with a bare `SectionList` whose sections include a badged destination that has no icon, laid out in a fixed-height column: on every frame of both slides its chip is inside the row's bounds and the row is never drawn empty, and the rail's column is never taller than at the taller of its two rest states (FR-013 exemption, FR-015, spec Edge Cases). Its assertions hold on T020's rail, so once T025 lands, check it against the mutant *a no-icon row laid out at 272 like the others*, observe it fail there, restore, and record the red in `tdd/cycle-log.md`

### Implementation for the rows

- [X] T025 [US1] [A7] [U17] [U18] Add `RowSlide` to `section_list.rs` with `Forms::Sliding { labelled, marked, icons_only }` — three `Element`s of the same widget type, in this fixed child order — and `Forms::Single(element)`. For each destination with an icon and for the collapse control, build `labelled` and `icons_only` exactly as today's rendering builds that row for each state (FR-006), and `marked` as `labelled` with its icon tinted with the badge fill when badged, or, when not, a copy of `labelled` that is never drawn and is given a zero-size parked node (research R3). A destination with no icon is `Single`, laid out at the width given (research R3a). `layout` lays out `labelled` and `marked` at 272 and `icons_only` at 64, reads `x_*` with `first_leaf_x` before parking, picks the drawn form with `fraction` and `form`, offsets it by `offset` and parks the other two with `parked`. The row's node has the drawn form's height and the width given. `operate`, `update`, `mouse_interaction` and `overlay` reach the drawn form only. Makes T022, T023 and T024 pass
- [X] T026 [US1] Clip `RowSlide::draw` with `renderer.with_layer(bounds, ..)` when the drawn form is wider than the row, drawing it directly otherwise (research R6). No test task: draw-only, the plan's GUI exception; quickstart §B.2 records it
- [X] T027 [US1] [A3] Regenerate `crates/micold-client/tests/fixtures/layout_snapshot.txt` through `tests/layout_snapshot.rs`'s regenerate path, move the `settings.rail` anchor for both settings states in `crates/micold-client/tests/support/covered_states.rs` to the rail's padded container in the new tree (read the path from the regenerated fixture; rows sit two levels below it, research R10), and run quickstart §A.3's awk set comparison against `origin/main`, pasting its output into the PR body. A difference is a rest-state regression to fix in T025, not a fixture to accept (SC-005)
- [X] T028 [US1] Confirm `crates/micold-client/tests/gates/rail_icons_align.rs` passes, and check it against the mutant *`icons_only` not the last child slot* (swap `marked` and `icons_only` in T025's order), observe it fail there, and restore (research R10, FR-006). *Done in M2:* the mutant survived the unchanged gate (parked glyphs at −8.5e37 share one f32 "axis"), so the gate gained an inside-the-rail clause; cycle-log cycle 10. Then run the US1 outer loop (A1–A7 in `tdd/test-list.md`); US1 is not complete until all seven are green
- [X] T029 [US1] Update `docs/user-guide/settings.md`: the WIP paragraph, amended so it names **Collapse** at the bottom of the rail when expanded and the expand icon it is drawn as when collapsed, and says both slide the rail and that a section with something to report keeps marking its row (FR-012)
- [X] T030 [US1] Update the module docs at the top of `section_list.rs` to say the component owns its slide: `Rail` owns time and width, each row owns its forms, and no caller opts in or out (plan Constitution VII, VIII)

**Checkpoint**: US1 works on its own: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion --test layout_snapshot` green, quickstart §A.3 prints `SC-005: visible rects unchanged`.

---

## Phase 5: User Story 2 — keep the keyboard and pointer working while it moves (Priority: P2)

**Goal**: Focus stays on the control the user activated through every form change, Tab reaches
only drawn controls, pointer input reaches only what is drawn under it, and no press, ripple or
focus ring is left behind in a parked form.

**Independent test**: Focus Collapse by keyboard and activate it; on every frame until it settles,
focus is on that control and the reachable count equals rest's. Move and press the pointer across
the rail mid-slide; nothing outside a row's bounds is hovered or pressed (quickstart §A.2).

### Tests for US2 (write first, observe fail)

- [X] T031 [US2] [A8] [A9] Write failing test `the_keyboard_stays_on_the_collapse_control_and_reaches_nothing_hidden` in `settings_rail_motion.rs`, adapted from the WIP: focus the collapse control by keyboard, toggle, and on every frame of both slides (including each row's form change) focus is on the control, the number of focusables equals the count at rest, and the section's focusables, in traversal order and identified by their bounds relative to the section's left edge, are the same sequence as at rest, including those scrolled out of view (FR-007, FR-008, SC-003). Observe it fail on T025 (focus lost when the control's form parks). Also check it against the mutant *`operate` reaching parked forms* (count doubles) and restore
- [X] T032 [US2] [U19] Write failing test `a_click_on_collapse_shows_no_focus_ring` in `settings_rail_motion.rs`: press and release the pointer on **Collapse**, let the slide settle, then assert the control is focused and that a screenshot of the control's region from `support::layout::renderer()` equals, pixel for pixel, the same settled state with focus cleared (the renderer is tiny-skia and yields pixels, not quads, as `ripple_layers.rs` compares them) (FR-006, FR-007). Observe it fail on T025 (the press focuses the labelled form, which parks, so the control is no longer focused); once T038 lands, also check it against the mutant *`GiveFocus` calling `Focusable::focus()`* (indicator shown, as the WIP's `FocusNth` did), observe it fail there, and restore
- [X] T033 [US2] [U20] Write failing test `a_badge_appearing_mid_slide_keeps_focus` in `settings_rail_motion.rs`: focus the badged row's control by keyboard, start a slide, and mid-slide rebuild the view with the badge cleared, then with it back; focus stays on that row's control in the drawn form on every frame (FR-007, FR-015). Observe it fail on T025 (focus lost at the form change); once T038 lands, also check it against the mutant *an empty placeholder of another widget type in the `marked` slot*, observe it fail, restore, and record both reds in `tdd/cycle-log.md`
- [X] T034 [US2] [A10] Write failing test `a_pointer_past_a_row_reaches_nothing` in `settings_rail_motion.rs`: mid-collapse, with the cursor inside the drawn labelled form's bounds but outside its row's bounds, a frame hovers nothing in the rail and a press there publishes no rail message (FR-009). Observe it fail against T025 without the cursor mask
- [X] T035 [US2] [U21] Write failing test `a_press_held_as_its_form_parks_is_released` in `settings_rail_motion.rs`: press a destination row at `f` just above 0.001, pump frames until its form parks, release, expand and let it settle, then send a bare left-button release over the row with no press before it, and assert no message is published (FR-009). A form whose pressed flag stuck publishes `SectionShown` on that release, which is how a control left drawn pressed shows through the harness. Observe it fail on T025, where parked forms receive no events
- [X] T036 [US2] [U22] Write failing test `a_ripple_in_a_parked_form_settles` in `settings_rail_motion.rs`: press a row just before its form parks; a frame after the slide has settled but before the ripple's duration ends still requests a redraw (the parked ripple is advancing), and once the ripple's duration and the slide have both passed, a frame requests no redraw (FR-009, FR-011). The harness sends every frame whether or not one was requested, so the first assertion is what separates a ripple that advances from one that does not. Observe it fail on T025, where parked forms receive no events (the ripple is frozen, and nothing requests a frame); once T040 lands, also check it against both mutants — *parked forms' redraw requests dropped* and *forwarded forever* — observe each fail, restore, and record the reds in `tdd/cycle-log.md`
- [X] T037 [US2] [A11] Write failing test `selecting_mid_slide_moves_only_the_changed_icons` in `settings_rail_motion.rs`: mid-slide select another section; the rows whose selection changed move their icon at once by `12 · f` (±0.5), unchanged rows only within the SC-007 bound, and the slide still settles (FR-014 exception, US2 scenario 4). Research R4 expects this to pass once T025 reads `x_labelled` on every layout; if it passes on first run, check it against the mutant *`RowSlide` caching `x_labelled` from its first layout* (the changed row's icon stays on its old line), observe it fail there, and restore

### Implementation for US2

- [X] T038 [US2] [A8] [A9] [U19] [U20] In `RowSlide::layout` in `section_list.rs`, after choosing the drawn form, run `TakeFocus` over each parked form's tree with a `Layout` built from that form's node and, when it yields `(index, visible)`, run `GiveFocus { index, visible }` over the drawn form's tree; do nothing when no parked form holds focus, so the step is idempotent (research R5, data-model step 2). Makes T031, T032 and T033 pass
- [X] T039 [US2] [A10] In `RowSlide::update` and `RowSlide::mouse_interaction` in `section_list.rs`, pass the cursor to the drawn form as unavailable when it lies outside the row's own bounds (research R5). Makes T034 pass
- [X] T040 [US2] [U21] [U22] In `RowSlide::update` in `section_list.rs`, forward to each parked form only `RedrawRequested`, left-button release, finger lifted, finger lost and cursor left, always with the cursor unavailable, against a local `Shell` whose messages and event capture are dropped and whose redraw requests and layout or widget invalidation are copied to the outer shell (research R5, contract §5 *Pointer*). Makes T035 and T036 pass
- [X] T041 [US2] [A11] Make T037 pass in `section_list.rs` if it does not already (the selection exception follows from `x_labelled` changing on rebuild, research R4); if a change is needed it goes in `RowSlide::layout`, never in the test. Then run the US2 outer loop (A8–A11 in `tdd/test-list.md`); US2 is not complete until all four are green

**Checkpoint**: US1 and US2 both work: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion` green.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T042 Run `mise run gate` (fmt, clippy with `-D warnings`, `cargo test --workspace`, `scripts/tests/*.test.sh` with `CHANGE_TITLE` set) and confirm every gate named in quickstart §A.2 is green, including `animated_layout_relayouts.rs`, `one_overlay_implementation.rs`, `motion_tokens.rs` and `cdk_no_appearance.rs`
- [X] T043 Correct quickstart §A.2's test names and *Fails on* column in `specs/030-settings-rail-slide/quickstart.md` to match `tdd/test-list.md`, `tdd/cycle-log.md` and the tests as written
- [X] T044 Run quickstart §B (B.1–B.4) with the repository's `visual-pass` skill on a private Xvfb display and pin dir, and write `specs/030-settings-rail-slide/visual-pass.md` with cropped images under `specs/030-settings-rail-slide/images/`: date, Xvfb + lavapipe, the binaries' pin check, which steps were exercised and which were not and why. A step not run is recorded as not run, never as a pass

---

## Dependencies & Execution Order

### Phase order

1. **Setup (T001–T002)**: no dependencies.
2. **US1 part A, the sidebar curve (T003–T004, T045–T046)**: after Setup; T045 after T004, T046 after T045. Independent of everything after it.
3. **Foundational (T005–T013)**: after Setup. Blocks Phases 4 and 5.
4. **US1 part B, the rail slide (T014–T030)**: after Foundational. T019 needs T004.
5. **US2 (T031–T041)**: after T025, since its tests exercise `RowSlide`'s forms.
6. **Polish (T042–T044)**: after everything.

### Within phases

- Every test task comes before the implementation task that names it: T003 → T004; T045 (its test, then its floor); T046 (its test, then its swap); T005 → T006;
  T007 → T008; T009 → T010; T014–T019 → T020–T021; T022–T024 → T025; T031–T037 → T038–T041.
- `section_list.rs` is touched by T005–T010, T019, T020, T024–T026, T030, T038–T041, so those run
  in sequence. `settings_rail_motion.rs` is touched by T013, T014–T018, T022, T023 and T031–T037,
  also in sequence. `[P]` marks only tasks whose files are disjoint from the other `[P]` tasks beside
  them.
- T027 (fixture) after T025 and T026; T028 after T027.

### Parallel opportunities

- Phase 3: T011 (`navigation_drawer.rs`) and T012 (`tests/support/layout.rs`).
- Phase 4: T019 (`section_list.rs`) beside T014–T018 (`settings_rail_motion.rs`); T023
  (`settings_rail_motion.rs`) beside T024 (`section_list.rs`).
- Phase 5: none; every test task is in `settings_rail_motion.rs`.

```text
# Phase 3, together:
T011 parked pub(super) in navigation_drawer.rs
T012 view_of pub in tests/support/layout.rs
# Phase 4, together:
T019 the rail and the sidebar move alike (section_list.rs) · T015 the section follows the rail edge (settings_rail_motion.rs)
```

## Implementation Strategy

1. **M1** — Setup, then the sidebar curve. Small, user-visible, merges on its own.
2. **M2** — Foundational, the rail slide (US1 part B) and its keyboard and pointer guarantees
   (US2), with the user guide and the visual pass. The rail's slide is not shipped without US2: a
   slide whose rows park forms without the focus handoff, reach filtering, cursor mask and parked
   events would strand focus and double Tab reach on `main`, which is worse than the snap it
   replaces (milestones rule 6).

Where this departs from the autopilot's milestone rules, and why (ledger D15):

- **Rule 1 (M1 = Setup + Foundational + P1).** M1 is Setup plus the sidebar half of US1 (scenario 6).
  It needs none of the Foundational phase, is observable on its own (the sidebar eases) and keeps the
  rail's PR to the rail. The rest of P1 is M2, which carries the 🎯 MVP mark.
- **Rule 2 (each further story its own milestone).** US2 ships in M2 with US1 part B, by rule 6
  above: US1 part B without US2 is a keyboard regression on `main`.
- **Rule 3 (split above ~15 tasks).** M2 has 40 tasks, and no split along an acceptance scenario
  leaves both halves with a deliverable: the Foundational tasks (T005–T013) deliver nothing
  observable (rule 1), a rail slide without its forms (T022–T025) reflows every label mid-slide
  (FR-013), and forms without US2 strand focus (rule 6). Its diff will also exceed 800 changed lines
  (the WIP's smaller design was about 900 outside the fixture); rule 3 keeps a story whole when no
  such split exists, and M2 is kept whole on that ground.
- **Rule 4 (Polish last).** T042–T044 verify M2's own behaviour (its gate, its test names, its
  visual pass) and have nothing to verify before it, so they sit in M2 rather than a milestone of
  their own. The user guide (T029) is in M2, which ships the behaviour.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — The sidebar eases

- **Tasks**: T001–T004, T045–T046
- **Deliverable**: Hiding and showing the worktree sidebar's panel moves on the `emphasized` curve
  over `medium_4` instead of at a constant speed, never laid out narrower than its rail, and hands
  over to the rail as soon as it is no wider.
- **Satisfies**: US1 acceptance scenario 6 (sidebar half); FR-003 (sidebar half)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib navigation_drawer`
  (`the_sidebar_slides_on_the_emphasized_curve`, `a_sliding_drawer_is_never_narrower_than_its_rail`,
  `a_closing_drawer_swaps_to_its_rail_once_it_is_no_wider`);
  `mise run gate`
- **Depends on**: —

### M2 — The settings rail slides 🎯 MVP

- **Tasks**: T005–T044
- **Deliverable**: In Settings, **Collapse** and the expand icon slide the rail between its labelled
  and icon widths on the sidebar's duration and curve, with icons on their line, badges marked, focus
  kept and pointer input confined to what is drawn; at rest it is exactly today's rail.
- **Satisfies**: US1 acceptance scenarios 1–7; US2 acceptance scenarios 1–4; FR-001–FR-015;
  SC-001–SC-007
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --lib section_list`;
  `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion --test layout_snapshot`;
  quickstart §A.3; `specs/030-settings-rail-slide/visual-pass.md`
- **Depends on**: M1 (T019 compares the rail with the eased drawer)

---

## Phase 7: Convergence

- [X] T047 Correct two stale doc comments in `crates/micold-client/src/ui/material/section_list.rs`: the module docs' "The component holds no state of its own" (lines 5–7) now sits above `Rail`'s widget-tree `Slide` state (lines 889–893), so say it holds no *application* state (the selection and the collapsed flag are the caller's; only the slide lives in the widget tree); and `RowSlide::is_unused`'s "neither laid out nor given events" (lines 564–565) and the "so it is not laid out either" comment (line 624) contradict the focus step that lays that form out at `ROW_WIDTH` on every layout (lines 638–645), so say it gets a zero-size node and is laid out only to take focus back. Comments only, no behaviour change per plan Constitution VII / T030 and ledger *Declined review findings* M2 code r1 (B) F2 (partial)

---

## Phase 8: TDD remediation

From `tdd/verification.md` (verdict `PASS_WITH_GAPS`; no blocking findings — none of these are
required before the feature can be considered done, but each is a real gap worth closing).

- [X] T048 [Finding 1, MED] Tighten `rows_keep_their_height_and_icons_their_line` in `crates/micold-client/tests/settings_rail_motion.rs:689-694` so its per-frame icon-step check compares against the value `offset()` itself predicts for that frame (within the existing `HALF_PIXEL` tolerance), not only an upper bound, so a regression that moves an icon too slowly (under-scaled or stalled) fails it. Verify: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion rows_keep_their_height_and_icons_their_line -- --exact`, then apply the mutant *halve the `offset` formula's coefficient* and confirm it now fails (it currently would not). *Done in close:* that mutant also moves the icons at rest, which the test reads as its endpoints, so it is `layout_snapshot`'s; the lagging mutant `fraction.powf(1.2)` passes the old test and fails the new check (cycle-log cycle 13)
- [X] T049 [Finding 2, MED] Split `the_state_flips_on_the_press` in `crates/micold-client/tests/settings_rail_motion.rs:479-540` into separate tests per FR-010 sub-claim (flip-on-press; mid-slide section change; mid-slide Save/Cancel closing), so a CI summary line names which sub-claim broke. Verify: `scripts/build-lock.sh cargo test -p micold-client --test settings_rail_motion -- state_flips` (or the new names) all green
- [X] T050 [Finding 3, LOW] No action required now — flagged as a repository-wide convention (`layout.rs`/`rail_icons_align.rs` already locate nodes by tree-index path); revisit only if a future structural change in `ui::view` produces a wave of path-based failures in `settings_rail_motion.rs`
- [X] T051 [Finding 4, LOW] No action required — an isolated restatement of one multiplication in `crates/micold-client/src/ui/material/navigation_drawer.rs:545`'s test table, not a pattern; leave as is
- [X] T052 [Finding 5, LOW] No action required — A3's approval-style verification (quickstart §A.3's rectangle-set comparison) is the rubric's expected shape for a snapshot/approval test; record any future re-run of that comparison in `tdd/cycle-log.md` so it stops being self-reported only
