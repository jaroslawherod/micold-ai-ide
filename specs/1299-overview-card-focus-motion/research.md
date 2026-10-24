# Research: Overview Card Focus and Expansion Motion

## R1 FR-006: is the growth of an expanded card animated? No.

- **Decision**: the expanded card's height changes in one step, as today; the focus indication and
  the shadow ease (FR-006, second branch).
- **Rationale**: the grid already lays out every row from the expanded card's collapsed twin
  (feature 1121, contract U2), so easing the expanded card's height would move no other card. But
  the growth cannot be eased without reflowing:
  1. *Eased layout height.* The expanded card's height comes out of the grid's layout pass
     (`card_grid.rs`, `card_heights[index]`), so each frame of a growth is a full grid layout:
     every card and the twin measured again. The expanded body is an inner `Scrollable`
     (`overview_card.rs#expanded_body`) that overflows on every frame but the last. Its themed
     scrollbar is drawn whenever content overflows (`style::scrollbar`), so it would show for the
     whole growth. Its scroll edges are published on every viewport change (`on_scroll_metrics`), and
     Page Up / Page Down go to the card body while the edges say there is more (feature 1121 K4,
     `page_target`). Keyboard behaviour would change mid-transition (FR-011), and the
     application would get a message per frame.
  2. *Draw-only reveal.* Laying out at the full height and clipping the drawing to an eased
     rectangle avoids all of that. But a renderer layer clips to a plain rectangle, so the card's
     rounded bottom corners and the bottom edge of its focus ring (drawn inside the card's bounds,
     `keyboard_focus::ring_bounds`) are cut off until the growth ends.
  3. *Collapse.* In both cases the application rebuilds the card collapsed in the same view that
     drops the expansion (`ui/overview.rs`, `.expanded(expanded)`), so there is no expanded content
     left to shrink. A shrink would draw a collapsed card stretched with an empty bottom. The only
     alternative is a "collapsing" phase in feature 1121's expansion state machine
     (`micold_core::overview_expansion`), which changes when the rest, focus and Escape rules
     apply.
- **Rejected**: (1) and (2) for the reasons above; (3) a collapsing phase in the expansion state:
  it touches feature 1121's keyboard and rest rules, which FR-011 keeps unchanged, for a 150 ms
  effect.
- **For the issue comment (#1299)**: "Growth is not animated. An expanded card's height still
  changes in one step; the focus outline and layer and the expanded card's shadow ease over 150 ms
  (standard easing). No other card would have moved, because the grid sizes rows from the collapsed
  twin. The card itself would have to reflow, though. Easing its laid-out height re-lays-out the grid
  every frame, and the inner scrollable overflows the whole time: its scrollbar shows, and Page Up /
  Page Down change target mid-growth. Clipping the drawing instead cuts the rounded corners and the
  focus ring's bottom edge. Neither has a collapse, because the card is rebuilt collapsed the moment
  the expansion ends. So the outline and shadow are enough, as the issue allows."

## R2 The focus fade lives in the shared `TakesTheKeyboard`, opt-in

- **Decision**: `Focus` (its tree state) gains a `Progress`; `TakesTheKeyboard::fade(Option<Duration>)`
  turns it on; the card passes `Some(CARD_MOTION)` unless instant.
- **Rationale**: the wrapper already owns the focus state and draws the ring and layer
  (`keyboard_focus.rs#draw`), so the fade needs no new coupling, and a per-instance track is the
  pattern `cdk::motion` prescribes (no registry, removed with the widget). Off by default, so the
  links' indicators and every other control stay instant (FR-011, Out of scope).
- **Rejected**: (a) a fade wrapper around the card that reads the focus from the child's tree:
  it couples two widgets' private state (the same reason feature 971 R2 rejected it); (b) turning
  the fade on for every control: it changes buttons, rows and links the issue does not name; (c) the
  application holding a per-card fade: a central animator is what feature 017 removed.

## R3 The expanded card's shadow is drawn by the grid, which owns the lift

- **Decision**: `CardGrid` keeps `Lifts` (one `Progress` per lifted or settling card, keyed by
  `SessionLocation`) and draws the shadow quad itself; the card in a grid draws none
  (`shadow_by_owner`).
- **Rationale**: a collapsing card goes back to normal draw order at once (`card_grid.rs#draw`
  skips only the expanded index), so a shadow fading in the card's own tree would be painted over
  by every card after it. Only the grid can keep a settling card above its neighbours while its shadow
  fades. Keying by location, as `observe_rest` does, keeps a lift on its card across a re-sort.
- **Rejected**: (a) the shadow `Progress` in the card's tree, with the card's container style:
  styles are fixed when the element is built, so it needs a drawing wrapper anyway, and the cut-off
  shadow above remains; (b) keying lifts by index: a re-sort mid-transition would move a shadow to
  another card.

## R4 What "easing the shadow" means

- **Decision**: interpolate the level's shadow from none: colour alpha, `offset.y` and
  `blur_radius` each times the eased strength (`style::eased_shadow`).
- **Rationale**: the design's `transition: box-shadow .15s` from `none` interpolates every
  component of the shadow from zero; scaling alpha alone would show a full-size faint shadow at once.
  At strength 1 the value equals `elevation_shadow` exactly (FR-010).
- **Rejected**: alpha only (above); a separate easing per component (no source asks for it).

## R5 Timing, curve and frames

- **Decision**: `duration::SHORT_3` with `motion::STANDARD` in both directions, through
  `Progress::on_frame` (wall-clock steps). One constant, `overview_card::CARD_MOTION`.
- **Rationale**: the issue fixes the duration and the curve; `on_frame` steps by elapsed time, so a
  slower frame rate does not stretch the 150 ms (spec Edge Cases, *Frames missed*). Gaps above
  `MAX_STEP` (64 ms) are the stated exception in `cdk/motion.rs`: a stalled frame runs long instead
  of jumping. That is the app-wide rule and is kept here.
- **Rejected**: `standard_decelerate`/`standard_accelerate` per direction (the issue names
  `STANDARD`); a per-direction duration (the design has one).
- **Motion assignment (contract §6.3)**: the new row, "Overview card focus and elevation |
  `short_3` (150) | `standard` | standard", is recorded in this feature's
  [contracts/ui-contract.md](contracts/ui-contract.md#motion-assignment). Feature 018's own table is
  another feature's contract and is not edited (ownership rule).

## R6 Reduce motion reaches the widgets as `instant`

- **Decision**: `CardGrid::instant(state.settings.reduce_motion)` in `ui/overview.rs`; the grid
  passes it to its cards. Instant draws from the target and snaps the tracks in `update`.
- **Rationale**: the navigation drawer takes the same flag the same way (`NavigationDrawer::instant`,
  `ui/mod.rs#drawer_flags`). Drawing from the target makes the first frame correct even before any
  `update` has stepped a track (FR-007), and a transition running when the setting is saved ends at
  once (FR-008).
- **Rejected**: a zero duration through `Progress` (it lands on the first *update*, not provably the
  first *draw*); reading the OS preference (spec, Out of scope).

## R7 A pointer press on the focused card snaps its focus off

- **Decision**: the press that hides a shown focus on the pressed control calls
  `fade.restart_at(0.0)`; focus lost to a press elsewhere fades.
- **Rationale**: the press starts a ripple, the card's only state layer while it runs (feature 971
  D1, feature 018 FR-024h). A focus layer fading under it would be two layers at once, and US4-1 says a
  press still draws no focus indication.
- **Rejected**: fading on every route including this one (two state layers under the ripple).

## R8 Edge cases

- **Built already focused or expanded** (reopened Overview, showcase poses): `Focus` built focused
  and shown starts at 1; `Lifts::seeded` starts the expanded card at 1; nothing animates into
  existence.
- **Card removed mid-transition**: its tree (fade) goes with it; `Lifts::step` drops a location no
  longer present. **Card added**: no entry, at rest.
- **Card state changes mid-fade**: the colours are read at each draw from the rebuilt element; the
  track continues (Principle II).
- **Zoom (feature 970)**: lifts step instantly while a zoom runs, so no settling card is drawn whole
  over cards drawn in pieces.
- **Overview closed mid-transition**: the grid's tree is dropped; no frame is asked afterwards.
- **Hover and focus on different cards / window blur**: the grid follows whichever card
  `expanded` names (feature 1121 decides it), so the left card settles while the focused one lifts.
