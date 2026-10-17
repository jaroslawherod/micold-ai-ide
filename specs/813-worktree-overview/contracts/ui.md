# Contract: Overview view, components and keys

**Feature**: 813 | Research: [R1–R3, R11–R13](../research.md) | Data: [data-model.md](../data-model.md)

## U1 — View placement

`crates/micold-client/src/ui/mod.rs` `view_with`: Settings → Overview (`overview.open`) → Changes →
Compare → terminal → `shell::view`. `main_content_key` → `u64::MAX - 3` while open (crossfade).
`app.rs` `terminal_focused` and `view_facts.main_area_taken` treat the Overview like Changes.
Sidebar and top bar stay. Sidebar `SessionMsg::Selected` closes it (FR-023).

## U2 — Top bar button

`ui/toolbar.rs`: after the project switcher, `Button::text("Overview", r).leading(Icon::Overview)
.selected(open).tooltip("Overview (Ctrl+Shift+O)")` (`Cmd+Shift+O` on macOS, from one
`keymap::overview_chord_label()`), `.mark(..)` when `attention(cards)` is `Some`: an `UnreadMark`
tinted `error` (NeedsAction) or `warning` (NeedsAttention), with the state's icon and the count,
accessible label / tooltip line *N need action* / *N need attention*. Disabled while Settings is
open (R2). Shown whether or not the Overview is open (FR-012).

## U3 — Shared components (`ui/material/`, builder API, showcase)

| Component | API | Showcase |
|---|---|---|
| `OverviewCard` | `new(&CardModel, roles).on_press(msg)` → `Element`; focusable; Enter/Space press | five states × (with / without progress, unknown counts, Default, orphan) |
| `CardGrid` | `new(cards).min_width(280.0).spacing(..)` | 1, 3, 30 cards; narrow |
| `ProgressBar` | `new(ratio, roles).width(..)` (determinate) | 0/10, 6/10, 10/10 |
| `Button::selected(bool)`, `Button::mark(UnreadMark)` | on the existing `Button` | toggled; tinted mark |
| `UnreadMark::icon(Icon)` | on the existing `UnreadMark` | with icon |

`OverviewCard` layout, top to bottom: type icon + display name (ellipsized, full text in tooltip)
and state icon + word; branch (when any); summary line; current activity line (Story 5, when any);
progress: phase word + ratio text + `ProgressBar` (only with a ratio) or *No task progress*;
footer: *N sessions, M running* and *X open · Y merged* (or *—* with tooltip; hidden without a
branch). The state reason is the state word's tooltip. Idle uses the de-emphasised surface.

## U4 — Geometry gates (in-crate tests)

- Card height is the same for the five states with the same content; text never overlaps at
  280 px; names ellipsize.
- `CardGrid` columns = `max(1, floor((w + gap) / (min + gap)))`; at 279 px one column; no card
  overlaps another; 30 cards scroll.
- Contrast (`composition_contrast.rs`): each state's text and icon role ≥ 4.5:1 (text) / 3:1
  (icon) on its card fill, both schemes; Idle text ≥ 4.5:1.

## U5 — Keys

| Key | Where | Effect |
|---|---|---|
| Ctrl+Shift+O (Cmd+Shift+O) | terminal focused | `KeyOutput::ToggleOverview`, event captured, 0 bytes to PTY |
| same | not focused, Settings closed | `overview::Msg::Toggled` |
| same | Settings open | nothing (R2) |
| Tab / Shift+Tab | Overview open | focus next / previous card (`Message::FocusMoved`) |
| Arrows | Overview open, no text field focused | Right/Down next, Left/Up previous |
| Enter / Space | card focused | activate (R12) |
| Escape | Overview open, no overlay | close (after registry and Settings in `on_escape`) |

`keymap.rs` tests: the chord on both platforms' modifiers; `O` without Shift, with Alt, or plain
Ctrl+O is not the chord (Ctrl+O still sends `0x0f`).
