# UI contract: Overview full-screen mode

Extends `specs/813-worktree-overview/contracts/ui.md` and `specs/1024-overview-larger-cards/contracts/ui.md`.

## Component API (Principle VIII)

| Component | Builder (new parts in bold) | Showcase poses |
|---|---|---|
| `NavigationDrawer` | `new(panel, rail).open(b).handle(h)`**`.retracted(b).instant(b)`** → `Element` | + "retracted", + "retracting" |
| `CardGrid` | `new(cards, roles).min_width(..).spacing(..).max_columns(..)`**`.zoom(Zoom { full_screen, freed }).on_reflow(Fn(AbsoluteOffset) -> M).scroll_id(id)`** → `Element` | + "zoom" (toggle in the showcase state) |
| `Icon` | **`FullScreen` (U+E5D0), `FullScreenExit` (U+E5D1)** | icon gallery |

## Messages

- `overview::Msg::FullScreenToggled { settings_open: bool }`: header button and chord.
- `overview::Msg::Reflowed(AbsoluteOffset)`: the grid asks the root to scroll (R6); root returns
  `operation::scroll_to(GRID_SCROLL_ID, offset)`.
- `settings::Msg::ReduceMotionToggled(bool)`: Appearance page; `appearance::SETTINGS` gains
  `("reduce_motion", "ReduceMotionToggled")`.

## Keys

- Ctrl+Shift+F (Cmd+Shift+F on macOS): `keymap::is_overview_full_screen_chord`, matched in
  `ui/mod.rs::app_chord` only; does nothing while the Overview is closed.
- Escape: unchanged route (`escape_message` → `overview::Msg::Closed`), which also leaves the mode.

## Overview header (U1 addition)

A row above the grid; at its end `Button::outlined("Full screen" | "Exit full screen")` with leading
`Icon::FullScreen | Icon::FullScreenExit`, in a `Tooltip` reading "Full screen (Ctrl+Shift+F)" /
"Exit full screen (Ctrl+Shift+F)" (Cmd on macOS). Tab reaches it (`TakesTheKeyboard` in `Button`).

## Geometry gates (in-crate tests)

Drawer (`navigation_drawer.rs`):
- retracted and settled: width 0, panel, rail and handle all parked, from expanded and from
  collapsed;
- retracting: width = full × eased presence, monotone, never negative, content translated left;
- reversal mid-way continues from the current width (no jump to either end);
- built retracted: starts at width 0 (no zoom on rebuild);
- `.instant(true)`: one frame from either end to the other, also from mid-way.

Grid (`card_grid.rs`):
- at `f = 0` and `f = 1` rectangles equal the rest grid for the end widths;
- for 1150 → 1450, 1300 → 1650, 1464 → 1770 px and back, sampled at 0.05 steps: no two drawn pieces
  intersect, every width within the end widths, every card drawn (one or two pieces, total width
  `w`), every piece inside `0..W`;
- reversal at `f = 0.5`: the next frame's rectangles are within one frame's motion of the previous;
- anchor: the first visible card's top stays at its viewport-relative y after `on_reflow` is applied.
