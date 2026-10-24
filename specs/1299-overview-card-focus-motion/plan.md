# Implementation Plan: Overview Card Focus and Expansion Motion

**Branch**: `claude/project-thread-vugbvf` | **Date**: 2026-10-24 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/1299-overview-card-focus-motion/spec.md`

## Summary

Ease an Overview card's focus indication and its expanded shadow over `duration::SHORT_3` (150 ms)
on the `STANDARD` curve, driven by `cdk::motion::Progress` in widget tree state, the way the
feature 971 ripple runs. The focus fade is an opt-in on the shared `TakesTheKeyboard` wrapper; the
expanded card's shadow (its *lift*) moves from the card's container style to the `CardGrid`, which
keeps one eased lift per card location and draws a settling card above its neighbours until its
shadow is gone. *Reduce motion* reaches both through an `instant` builder flag and makes every
transition end on the first frame. **FR-006 decision: the growth is not animated**; the height
changes in one step as today (research R1), and that is what is posted on #1299. No new crate, no
storage or wire change.

## Technical Context

**Language/Version**: Rust, stable (via `mise`)

**Primary Dependencies**: iced 0.14 (workspace); `cdk::motion::Progress`
(`crates/micold-client/src/ui/cdk/motion.rs`); `micold_core::tokens::motion::{duration::SHORT_3,
STANDARD}`; no new crate

**Storage**: N/A. Transitions are widget tree state; *Reduce motion* is the existing saved setting
(`features::settings`, feature 970)

**Testing**: `cargo test` through `scripts/gate.sh` (cloud; `mise run gate` locally): client-crate
unit tests in `ui/material/keyboard_focus.rs`, `ui/material/card_lift.rs` (new),
`ui/material/card_grid.rs`, `ui/material/overview_card.rs`, `ui/material/style.rs`; the existing
`tests/idle_requests_no_frames.rs` and `tests/motion_tokens.rs` stay green unchanged; quickstart §B
visual pass

**Target Platform**: Linux, macOS, Windows desktop (iced)

**Project Type**: desktop-app (Rust workspace; change is in `micold-client` only)

**Performance Goals**: frames only while a card transition runs; none at rest (FR-009, SC-003). No
extra layout pass: every transition here is draw-only

**Constraints**: 150 ms ± one frame (SC-001); end states pixel-identical to today (FR-010); no other
card moves (SC-005)

**Scale/Scope**: one shared wrapper option, one new small module, two widgets, one view call, one
showcase pose, two doc pages

## Constitution Check

*Re-checked after Phase 1 design: unchanged.*

- [x] **I. Test-First**: every design item below has its failing test in the Test strategy; tasks
  put each test before its code. The decisions (fade target, lift bookkeeping, eased shadow, draw
  order) live in in-crate unit-tested code; `ui/overview.rs` gains one builder call (glue).
- [x] **II. Multi-Session**: transitions are per widget instance (a card's `Focus` state, a grid's
  `Lifts`), keyed by the card's `SessionLocation`, never global; nothing persists; a session's state
  change only recolours the card mid-transition (spec Edge Cases).
- [x] **III. Worktree Integration**: N/A, no file or VCS operation.
- [x] **IV. Local-First**: no storage, no network.
- [x] **V. Rust + iced**: iced only. A lift exists only while it is off rest (`Lifts` drops an entry
  at 0), so "a resting card with a lift" is not stored; the fade is `Option<Duration>`, so "eased
  with no duration" is unrepresentable.
- [x] **VI. Cross-Platform**: platform-neutral widget code, wall-clock timed; no `cfg`; the OS
  reduced-motion preference is not read.
- [x] **VII. Documentation**: `docs/user-guide/settings.md` (*Reduce motion*) and
  `docs/user-guide/overview.md` in the same change (FR-013).
- [x] **VIII. Reusable UI Foundation**: the fade is a builder option on the shared
  `TakesTheKeyboard` (default off, so every other control is unchanged); the eased shadow is a
  shared `style` function; `CardGrid::instant` and `OverviewCard::instant` are chainable builders
  ending in `.into()`. No card-local widget.

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/1299-overview-card-focus-motion/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/ui-contract.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code (repository root)

```text
crates/micold-client/src/ui/
├── material/keyboard_focus.rs   # Focus.fade, TakesTheKeyboard::fade, faded draw (D1)
├── material/overview_card.rs    # instant(), CARD_MOTION, shadow left to the grid (D2)
├── material/card_lift.rs        # NEW: Lifts, one eased lift per card location (D3)
├── material/card_grid.rs        # ZoomState.lifts, instant(), draw order and shadow quad (D3, D4)
├── material/style.rs            # eased_shadow(roles, level, strength) (D3)
├── material/mod.rs              # mod card_lift
└── overview.rs                  # CardGrid::instant(state.settings.reduce_motion) (D4)
crates/micold-client/src/showcase/sections/surfaces.rs   # live expand/collapse pose (D5)
docs/user-guide/settings.md, docs/user-guide/overview.md  # (D6)
```

**Structure Decision**: edits in existing `micold-client` modules plus one small module beside the
grid; nothing in `micold-core` (timing and drawing, not a rule). `card_grid.rs` is already 3,237
lines, so the lift bookkeeping goes in `card_lift.rs` rather than growing it.

## Design

### D1 Focus fade on the shared wrapper (FR-001, FR-002, FR-004, FR-005, FR-007, FR-009)

`keyboard_focus.rs`:

- `Focus` gains `fade: Progress`, on the `STANDARD` curve. Its `Default` is written out so a fresh
  control starts at `0.0`; for a wrapper with `on_focus_change`, `state()` for a control created
  already focused and shown starts at `1.0` (a control built focused must not animate into
  existence, `Progress::new` docs). Overview cards wire only `on_shown_change`
  (`overview_card.rs`), so `state()` returns `Focus::default()` for them: focus that traversal
  gives back after the Overview reopens fades in, as FR-001 allows.
- `TakesTheKeyboard::fade(self, over: Option<Duration>) -> Self`, default `None`: `None` is today's
  behaviour, so every other control (and the card's links, FR-011) is unchanged.
- `update`, after `shown` is worked out at its end: target = `shown` as `0.0`/`1.0`. With `Some(over)`,
  `fade.on_frame(event, target, over, shell)` (steps only on `RedrawRequested`, asks for frames only
  while moving). With `None`, `fade.restart_at(target)`. A traversal changes focus inside
  `operate`, outside any event; the `RedrawRequested` that follows retargets the track, as it
  already publishes `on_shown_change` today.
- **A pointer press on the control itself** that hides a shown focus (`visible = false` in the press
  branch) calls `fade.restart_at(0.0)`: the press ripples, and the ripple is the card's only state
  layer (feature 971 D1, FR-011), so the hidden focus does not fade under it. Focus *lost* to a press
  elsewhere fades out (FR-002).
- `draw`: strength = `target` when `fade` is `None`, else `fade.value()`. The state-layer opacity is
  today's top-up (`Focus::indicator`'s body, now computed whether or not the focus is shown) times
  the strength, and the ring's colour alpha is the strength; nothing is drawn at strength 0. At
  strength 1 both are exactly today's (FR-010). One opacity for ring and layer (spec Assumptions).
- Reversal (FR-004) comes from `Progress::on_frame` retargeting inside the call when the target
  changes (`from = value`, `t = 0`; the retarget helper itself is private): a reversed fade turns
  back from where it is and takes at most the full 150 ms.

### D2 Card wiring (FR-001, FR-002, FR-010, FR-011)

`overview_card.rs`:

- `pub(crate) const CARD_MOTION: Duration = Duration::from_millis(duration::SHORT_3)`: one name for
  the fade and the lift, so they cannot drift (`tests/motion_tokens.rs` accepts a named token).
- `OverviewCard::instant(self, instant: bool) -> Self` (pub, default `false`): the activatable card's
  `TakesTheKeyboard` gets `.fade((!instant).then_some(CARD_MOTION))`.
- `pub(crate) fn shadow_by_owner(self) -> Self`: the fill container draws no shadow; the grid draws it
  (D3). A card used on its own keeps casting the menu shadow when expanded, so feature 1121's
  `an_expanded_card_casts_the_menu_shadow` stays as it is.
- Nothing else in the card's face changes: issue #1298 is redesigning that face in parallel, and the
  edit here is confined to the shadow's `if` and the focusable chain.

### D3 The grid's lift (FR-003, FR-004, FR-005, FR-009, FR-010, SC-005)

`card_lift.rs` (new, `pub(super)`):

- `Lifts`: `Vec<(SessionLocation, Progress)>`, one entry per card that is lifted or settling,
  eased on `STANDARD` over `CARD_MOTION`.
- `seeded(expanded: Option<&SessionLocation>) -> Lifts`: the expanded card at `1.0`, for a grid
  whose state is created with a card already expanded (reopened Overview, the showcase's static
  poses): nothing animates into existence.
- `step(&mut self, event, expanded, present, instant, shell)`: adds an entry at `0.0` for a newly
  expanded location; drops entries whose location is no longer in `present` (a removed card drops
  its transition); steps each entry toward `1.0` if it is the expanded one else `0.0` with
  `on_frame`, or `restart_at(target)` when `instant`; drops an entry resting at `0.0`.
- `strength(at, expanded, instant) -> f32`: the target when `instant`, else the entry's value, else
  `0.0` (expanded but not yet stepped: the fade starts from none).
- `settling() -> impl Iterator<Item = &SessionLocation>`: entries above `0.0` that are not expanded.

`card_grid.rs`:

- `ZoomState` gains `lifts: Lifts`, seeded in `Grid::state()`.
- `Grid` gains `roles` and `instant`; `CardGrid::instant(bool)` (pub, default `false`) sets it, and
  `into_grid` builds each card with `.instant(instant).shadow_by_owner()` (the twin is never drawn).
- `update`: `lifts.step(...)` on every event, beside `observe_rest`, with the expanded card's
  location and the cards' locations from `rests`. While a zoom runs (`state.ends.is_some()`) the
  lifts are stepped with `instant` (a zoom draws cards in pieces; no settling card is drawn over
  them).
- `draw`, in three passes: (1) every piece as today, skipping the expanded card and every settling
  card; (2) each settling card whole at its slot layout, in a layer over the viewport, after its
  shadow quad; (3) the expanded card as today, after its shadow quad. The shadow quad is
  `fill_quad` at the card's drawn bounds, radius `overview_card::RADIUS` (made `pub(crate)`), transparent fill, shadow
  `style::eased_shadow(roles, elevation::MENU, strength)`; the opaque card face covers its inside.
  A settling card is drawn over its neighbours only while its shadow fades, so its shadow is never
  cut by the cards after it; its layout, events, focus and hit-testing stay at its slot, so nothing
  but drawing order changes for 150 ms (SC-005, FR-011).
- At most one card is expanded (feature 1121); any number may settle at once (rapid hover across
  cards, US2-3), each from where it is (FR-005).

`style.rs`: `eased_shadow(r, level, strength) -> Shadow` scales the level's shadow colour alpha,
`offset.y` and `blur_radius` by `strength.clamp(0, 1)` (CSS `box-shadow` from `none` interpolates
each, research R4). `eased_shadow(r, l, 1.0) == elevation_shadow(r, l)` and `0.0` gives
`Shadow::default()` (FR-010).

### D4 Reduce motion (FR-007, FR-008, FR-012)

`ui/overview.rs` `view`: `.instant(state.settings.reduce_motion)` on the `CardGrid`, the same flag
`drawer_flags` gives the drawer (feature 970). `state.settings` is the saved setting, so the
one-form rule holds (FR-008). With `instant`, `draw` reads every strength from its target and
`update` snaps the tracks: the first frame after the change shows the end state (FR-007), and a
transition running when the setting is saved ends on the next frame (FR-008). The ripple is not
touched (spec Assumptions).

### D5 Showcase (FR-013)

`surfaces.rs`, Overview card section: one pose of a few activatable cards (`on_press(NoOp)`, so Tab
shows the focus fade) whose first card is expanded while `showcase.shown(index)` and toggled by an
"Expand / Collapse" button sending `Message::Reversed(index)`, the mechanism the zoom pose beside it
already uses. Showcase grids keep the default (eased). The existing static expanded poses are
seeded lifted (D3), so they open as today.

### D6 Documentation (FR-013, Principle VII)

`docs/user-guide/settings.md` *Reduce motion*: it also makes the Overview cards' focus outline and
the expanded card's shadow change at once instead of easing over 150 ms; "other animations are
unchanged" stays true of the rest. `docs/user-guide/overview.md`: the focus and expansion easing in
one line beside the existing *Reduce motion* mention.

### D7 FR-006: growth is not animated

The height of an expanded card changes in one step as today; only the shadow and the focus
indication ease. Why (research R1): the twin already keeps every other card's slot, so no
neighbour would move; but the growth itself cannot be eased without reflowing. Easing the laid-out
height means a full grid layout per frame (every card and the twin re-measured) and an expanded
body whose inner scrollable overflows on every frame of the growth: its scrollbar shows for the
transition, its scroll edges are published each frame, and Page Up / Page Down go to the card body
while the edges say there is more (feature 1121 K4), which changes keyboard behaviour mid-transition
(FR-011). A draw-only reveal (full layout, clipped drawing) avoids that, but cuts the card's
rounded bottom corners and the bottom edge of its focus ring while it grows. Neither has a
collapse: the application rebuilds the card collapsed in the same view that drops the expansion, so
there is no expanded content left to shrink. Text for the issue comment is in research R1.

### FR → design map

FR-001 D1, D2 · FR-002 D1, D2 · FR-003 D3 · FR-004 D1, D3 (`on_frame` retargets) · FR-005 D1 (per-card
tree state), D3 (per-location entries) · FR-006 D7 · FR-007 D4 · FR-008 D4 · FR-009 D1, D3
(`Progress` asks only while moving; `Lifts` drops resting entries) · FR-010 D1, D2, D3
(`eased_shadow` at 1) · FR-011 D1 (`fade` default off; press snaps), D2, D3 (layout and events
unchanged) · FR-012 Test strategy · FR-013 D5, D6.

## Test strategy

| Requirement | Layer | Test |
|---|---|---|
| FR-001, FR-002, SC-001 | client unit, `keyboard_focus.rs` (`Mounted`, frame ticks 16 ms apart from `origin`) | Tab onto a faded control: strength 0 < s < 1 at the tick nearest 75 ms, 1 by 150 ms + one frame; focus moved away: back to 0 the same way |
| FR-004 | same | reverse at ~75 ms: the next frame's strength is within one step of the last, and it reaches its end by 150 ms after the reversal |
| FR-005 | same, two faded controls | Tab from one to the other: one falling, one rising on the same frames |
| FR-009, SC-003 | same + `Lifts` unit | after the end, a `RedrawRequested` leaves `shell.redraw_request()` at `RedrawRequest::Wait`; `tests/idle_requests_no_frames.rs` unchanged and green |
| FR-011 (press, ring) | same | a press on a shown-focused control: strength 0 on the next frame; a control without `fade` draws exactly as before (existing ring tests unchanged) |
| FR-003, FR-005, SC-005 | client unit, `card_lift.rs` and `card_grid.rs` | expanded location eases 0 → 1 and back, ticks 16 ms apart: 0 < lift < 1 at the tick nearest 75 ms, 1 (or 0 on the way back) by 150 ms + one frame (SC-001), both directions; two locations settle independently; seeded grid starts at 1; removed location dropped; slot rectangles of every card equal across all frames of a transition |
| FR-003 draw order | client unit, `card_grid.rs` (rasterised `drawn`) | a settling card's shadow shows below it over the card beneath (not cut), and a settling card is drawn after its neighbours |
| FR-010 | client unit, `style.rs` + `card_grid.rs` | `eased_shadow(.., 1.0) == elevation_shadow`, `0.0` is default; grid's expanded card at rest paints the same pixels below it as feature 1121's card-drawn shadow |
| FR-007, FR-012, SC-002 | client unit, `card_grid.rs` (grid `.instant(true)`) | **the Reduce motion test**: on the first redraw after gaining and after losing the keyboard focus the ring is at full / none, and after expanding and collapsing the shadow is full / none, with no request for a next frame |
| FR-008 | same | grid mid-fade rebuilt `.instant(true)`: next frame at the end state |
| FR-011, SC-004 | existing ripple (971), context menu (972), link focus (1122, 1123), keyboard navigation (813, 1121) tests | pass unmodified |
| FR-006 | none (decision, D7) | posted on #1299 |
| FR-013, SC-001 by eye | quickstart §B visual pass (`visual-pass` skill) | showcase pose and the client Overview, Reduce motion off and on |

## Complexity Tracking

None.
