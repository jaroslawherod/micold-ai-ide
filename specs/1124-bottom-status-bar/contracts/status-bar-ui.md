# Contract: status bar component and window placement

**Feature**: [spec.md](../spec.md) | [research R1–R4, R10](../research.md) |
[data model](../data-model.md)

## Component (`ui/material/status_bar.rs`, NEW)

```rust
StatusBar::new(roles)
    .item(StatusItem)            // leading group, in order
    .item_element(Element, GiveWay)  // an existing component as an item (plan usage)
    .hint(StatusItem)            // trailing group, right-aligned
    .into()                      // -> Element<'a, M>

StatusItem::new(text)
    .dot(ItemTone)               // coloured dot before the text
    .tone(ItemTone)              // text colour (default Neutral)
    .busy(bool)                  // BusyIndicator ring before the text; frames only while true
    .tooltip(String)             // full detail on hover; also the full text when shortened
    .give_way(GiveWay)           // default Keep
```

- B1: Height exactly `anatomy::status_bar::HEIGHT` (28 dp), width `Fill`. Surface
  `surface_container`, 1 dp `outline_variant` divider on its top edge.
- B2: Items are vertically centred, separated by `spacing::MD`; the leading group starts and the
  trailing group ends at `spacing::MD` from the window edges.
- B3: No item is a button: no `on_press`, no focus, no hover state other than its tooltip (FR-014).
- B4: When the natural width exceeds the bar: `Drop` items vanish in rank order, then `Shorten`
  items are ellipsized in rank order down to dot + one character + "…". No text is painted past
  its box at any width ≥ `MIN_WINDOW_SIZE.width` (640 dp) (FR-012).
- B5: A dropped item has no layout node area and no tooltip hit area.
- B6: Showcase entry in `showcase/sections/` covering steady, busy, warning and a too-narrow
  instance.

## Window placement (`ui/mod.rs::view_with`)

| Row (top to bottom) | Before | After |
|---|---|---|
| `toolbar::view` | always | always (plan usage removed, see below) |
| `connection_banner` | non-`Connected` | the four action states only (`Disconnected` → zero size) |
| `sandbox_banner` | fallback / Stale / Failed | unchanged |
| bring-up row (`sandbox_status::indicator`) | Probing / Acquiring / Starting | **removed** |
| `body` | always | always |
| `status_bar::view` | — | **always** (except install-location screen, which returns earlier) |

- P1: The bar is a child of the `base` element, so every overlay surface (menus, dialogs, quick
  switcher, notifications) draws above it (FR-013), and `KeyboardElsewhere` keeps the terminal's
  focus (FR-013).
- P2: In the steady state the vertical gap between the toolbar's bottom and the body's top is 0 in
  every body branch (SC-001); over a bring-up it does not change (SC-003).

## App bar (`ui/toolbar.rs`)

- A1: `toolbar::view` no longer adds `UsageIndicator`; `usage_indicator` moves to
  `ui/status_bar.rs` with its test (FR-011).

## Test hooks

- `ui::status_bar::model(&State, &ConnectionStatus, &Sandbox, now, offset) -> StatusModel` — public
  to the crate's tests (re-exported from `ui`), render-free.
- `tests/support/layout.rs::StateUnderTest::sandbox(Sandbox)` — NEW builder; covered states that do
  not call it keep `Sandbox::default()` (off).
