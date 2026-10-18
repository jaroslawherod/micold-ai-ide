# Data model: Overview full-screen mode

## Full-screen mode (per window, not persisted)

`crates/micold-client/src/features/overview.rs`, `State`:

| Field / fn | Type | Rule |
|---|---|---|
| `full_screen` (new) | `bool` | set by `Msg::FullScreenToggled` only while `open`, Settings closed, no `activating`, no `card_menu` (its panel hangs at fixed window pixels; review A M1); cleared when the Overview opens (`Msg::Toggled` from closed) |
| `full_screen()` (new) | `fn(&self) -> bool` | `open && full_screen`; the only reader (R1) |

Transitions: closed → open: `full_screen = false`. Open, toggle: `full_screen = !full_screen`.
Any exit (`Closed`, `CardActivated`, `open = false` in `app.rs`): `full_screen()` is false at once.
Toggle with Settings open, while closed, or during an activation delay: no change.

Nothing about the sidebar is stored: `state.sidebar.hidden` and `state.sidebar.width` are read, never
written, by this feature (R2).

## Reduce motion setting (persisted)

| Where | Field | Rule |
|---|---|---|
| `micold_core::settings::Settings` | `reduce_motion: bool` (new) | `#[serde(default)]` → `false`; old files load unchanged |
| `features/settings.rs` `AppearanceDraft` | `reduce_motion: bool` (new) | seeded by `SettingsDraft::from_settings`; `Msg::ReduceMotionToggled(bool)` edits it |
| `ValidSettings` | `reduce_motion: bool` (new) | copied from the draft by validation |
| `SettingsState` | `reduce_motion: bool` (new) | applied by `saved` next to `theme_pref`; Cancel/Esc drop the draft |
| `shell/persist.rs` | writes `settings.reduce_motion` | same path as `theme` |

## Zoom transition (widget tree state, not app state)

| Owner | State | Rule |
|---|---|---|
| `NavigationDrawer` | `presence: Progress` (new), 1 shown / 0 retracted | `new(target)` at build; eases on `SLIDE` (400 ms) `EMPHASIZED`; retarget reverses from the current value; `.instant(true)` snaps with `restart_at`; at ≤ `CLOSED` all children parked |
| `CardGrid` | `ZoomState` (new): `full_screen_seen: bool`, `ends: Option<(f32, f32)>` (start, end widths), `started: Instant` | flip of `Zoom::full_screen` → `ends = (W, W ± freed)`; flip during a zoom keeps `ends` (reversal); zoom ends when `W` within 0.5 px of an end, `W` outside `ends`, or `SLIDE + 100 ms` elapsed |

Derived per layout during a zoom (R5): `f = clamp((W − start)/(end − start), 0, 1)`;
`w = lerp(w_rest(start), w_rest(end), f)`; card `i` at ribbon offset `i·(w + gap)` on rows of length
`W + gap`; a card crossing a row end is drawn in two clipped pieces. Invariants: no two cards'
drawn rectangles intersect; `min(w_start, w_end) ≤ w ≤ max(w_start, w_end)`; at `f ∈ {0, 1}` the
rectangles equal feature 1024's rest grid.
