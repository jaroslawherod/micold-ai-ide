# Data Model: Overview Card Context Menu

**Feature**: 972 | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

All state is per-window memory in `crates/micold-client/src/features/overview.rs`; nothing is
persisted or sent (Principle IV).

## `MenuOrigin` (new)

```text
enum MenuOrigin { Pointer, Keyboard }
```

Which gesture opened the menu. `Keyboard` arms focus return (FR-011) and starts the highlight at
the first entry (FR-009).

## `CardMenu` (new)

| Field | Type | Meaning |
|---|---|---|
| `location` | `SessionLocation` | The target location. Identity, not a grid index, so the mark follows reorders (Edge: cards reorder). |
| `anchor` | `(u16, u16)` | Panel top-left in window pixels: the press point, or inside the focused card's bounds. Clamped at render time (`clamp_menu_anchor`). |
| `origin` | `MenuOrigin` | See above. |
| `highlighted` | `Option<usize>` | Keyboard-highlighted entry; `Some(0)` from the keyboard, `None` from the pointer until Up/Down. |

## `overview::State` (extended)

| Field | Type | Change |
|---|---|---|
| `card_menu` | `Option<CardMenu>` | new; at most one (FR-014) |
| `focus_return` | `Option<SessionLocation>` | new; armed by a keyboard-opened menu closing or running an entry, drained by `main.rs` (R9) |

## `overview::Msg` (extended)

| Variant | Effect in `update` |
|---|---|
| `CardMenuOpened { location, anchor, origin }` | Replace `card_menu` (another card's menu closes, US2.4); `highlighted = Some(0)` iff `Keyboard`; outcome `SurfaceOpened(OverviewCardMenu::ID)`. |
| `CardMenuRequested(SessionLocation)` | No state change; `main.rs` resolves the focused bounds and sends `CardMenuOpened { origin: Keyboard }`. |
| `CardMenuDismissed` | `card_menu = None`; `focus_return = Some(target)` iff origin `Keyboard`. Escape, outside click, displacement, scroll and empty-space right-click all arrive as this. |
| `CardMenuMoved { forward, items }` | `highlighted` steps by one, wrapping over `items` (the entry count, supplied by the listener's caller); `None` starts at first (forward) or last (back). |
| `CardMenuActivated` | No state change; `main.rs` resolves the highlighted entry (`ui::overview::card_menu_entry`) and sends it as `CardMenuEntryChosen`. |
| `CardMenuEntryChosen(Box<Message>)` | `card_menu = None`; `focus_return` as for dismissal; `main.rs` then dispatches the inner message through `update_inner`. |
| `GridScrolled` | Root calls `dismiss_on_scroll_beneath()`. |

## Invariants

- `card_menu.is_some()` ⇒ `open` and the target is among `ui::overview::cards(state)` after every
  outermost update (`settle_card_menu`, R8).
- `!open` ⇒ `card_menu.is_none() && focus_return.is_none()`.
- At most one context menu is open: the registry displacement in R2 is the rule.
- `highlighted`, when `Some(i)`, is `< ` the entry count it was last stepped over; a stale index
  beyond the current list resolves to no entry (`card_menu_entry` returns `None`).

## Surface

`OverviewCardMenu` — `SurfaceId::new("overview_card_menu")`, layer `ContextMenu`, cancelled by
`Message::Overview(Msg::CardMenuDismissed)`, open iff `state.overview.card_menu.is_some()`.
