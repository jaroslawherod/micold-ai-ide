# UI contract: Overview Card Focus and Expansion Motion

## C1 `TakesTheKeyboard::fade(self, over: Option<Duration>) -> Self` (shared, extended)

- Default `None`: the indicator appears and disappears in one step, exactly as before.
- `Some(over)`: the ring and the focus state layer fade together, one strength from 0 to 1, over
  `over` on `STANDARD`, toward 1 while the focus is held and shown and toward 0 otherwise. A
  reversal continues from the current strength.
- A pointer press on the control that hides its shown focus ends the fade at 0 at once.
- At strength 1 the drawing is identical to the unfaded indicator. Frames are asked only while
  the strength moves.

## C2 `OverviewCard` (extended)

- `instant(self, instant: bool) -> Self`, default `false`: with `true` the card's focus does not fade.
- Activatable card: its focusable is built `.fade((!instant).then_some(CARD_MOTION))`, where
  `CARD_MOTION = Duration::from_millis(duration::SHORT_3)`.
- `pub(crate) shadow_by_owner(self) -> Self`: the expanded card's fill casts no shadow, because its
  owner draws it. A card on its own is unchanged: expanded, it casts the menu elevation's shadow.
- The links' indicators (features 1122, 1123) have no fade.

## C3 `CardGrid` (extended)

- `instant(self, instant: bool) -> Self`, default `false`: every card transition ends on the first
  frame; passed to each card as `OverviewCard::instant`.
- Each card is built `shadow_by_owner()`; the grid draws the shadow: a quad at the card's drawn
  bounds, radius 12, transparent fill, `style::eased_shadow(roles, elevation::MENU, strength)`.
- Strength per card location eases to 1 while that card is the expanded one and to 0 otherwise,
  over `CARD_MOTION` on `STANDARD`. A card expanded when the grid's state is created starts at 1.
- Draw order: resting cards, then each settling card (strength > 0, not expanded) whole at its
  slot, then the expanded card. Layout, events, focus and hit-testing are unchanged: only the order
  of drawing differs, and only while a shadow settles.
- The expanded card's height changes in one step (FR-006 decision, research R1).
- While a zoom (feature 970) runs, lifts end at once.

## C4 `style::eased_shadow(r: Roles, level: u8, strength: f32) -> Shadow` (new)

- The level's shadow with colour alpha, `offset.y` and `blur_radius` multiplied by
  `strength.clamp(0.0, 1.0)`; `1.0` equals `elevation_shadow(r, level)`, `0.0` equals
  `Shadow::default()`.

## C5 Overview view (`ui/overview.rs`)

- `CardGrid::instant(state.settings.reduce_motion)`: the saved *Reduce motion* setting.

## Motion assignment

The row this feature adds to the motion assignment of feature 018 contract §6.3, recorded here
(feature 018's own contract is not edited):

| Animation | Duration | Easing | Set |
|---|---|---|---|
| Overview card focus and elevation | `short_3` (150) | `standard` | standard |

Under *Reduce motion* this row is instant; the ripple rows are unchanged.
