# Contract: `Scrollable`'s addressable viewport

Feature 024. Two additions to the shared `Scrollable`
(`crates/micold-client/src/ui/material/scrollable.rs`). Both are chainable builder methods on the
existing struct, terminating in the existing `impl From<Scrollable> for Element` — Principle VIII's
required shape, not free functions.

## §1 `id`

```rust
Scrollable::new(list, roles)
    .height(Length::Fill)
    .id(sidebar::SCROLL_ID.clone())
    .on_scroll_offset(Message::SidebarScrolled)
    .into()
```

**§1.1** `id(impl Into<iced::widget::Id>)` forwards to the rendering stack's
`scrollable::id(...)`, making the viewport addressable by
`iced::widget::operation::scroll_to(id, AbsoluteOffset)`.

**§1.2** Unset by default. A `Scrollable` without an id behaves exactly as before, so the folder
browser's call site is unaffected.

**§1.3** The id lives on the scrollable itself, never on a wrapper around it. Scroll operations reach
widgets through `operate` traversal, and a wrapper that does not forward `operate` silently swallows
them for its whole subtree — the trap `Ripple` documents at `ui/material/ripple.rs:248-256`. Any new
wrapper introduced between this viewport and its rows must forward `operate` for the same reason.

## §2 `on_viewport_resize`

```rust
Scrollable::new(list, roles)
    .on_viewport_resize(|size| Message::SidebarViewportResized(size.height))
```

**§2.1** `on_viewport_resize(impl Fn(Size) -> M)` reports the **viewport's** laid-out size — the
scrolling window, not the content inside it.

**§2.2** It fires on first layout and on every subsequent size change. This is the reason it exists
rather than reusing `on_scroll`'s `Viewport`: `on_scroll` fires only when something scrolls, and the
case that matters is the first draw after a project switch, where nothing has. *(research R6)*

**§2.3** Implemented with iced 0.14's `Sensor` (`on_show` + `on_resize`). The `Sensor` wraps the
scrollable, so §1.3 applies to it — it must forward `operate`, or `scroll_to` stops at it. iced's own
`Sensor` does; a future replacement must be checked.

**§2.4** Unset by default, and the two subscriptions are independent: setting
`on_viewport_resize` does not disturb `on_scroll` / `on_scroll_offset`, whose existing
"offset form wins" rule (`scrollable.rs:100-109`) is unchanged.

**§2.5** A consumer that never sets it pays nothing — no `Sensor` is inserted into the tree.

## §3 Why both live here and not in the sidebar

`Scrollable` is the shared wrapper for every scroll region in the application
(`ui/material/scrollable.rs:1-13`). A sidebar-local scrollable with an id would be the forked one-off
the component-reuse gate exists to reject, and the next scroll region that needs to be addressed
would fork a second. Both additions are generic: an id and a viewport measurement are properties any
scroll region can want.

## §4 What is not added

- No "scroll child into view" helper. iced 0.14 has no such operation and the geometry that would
  implement one belongs to the list that knows its row heights, not to a generic viewport
  *(research R6)*.
- No content-size reporting. Nothing needs it: the list's own metrics give content height.
- No animated scrolling. `scroll_to` is a single jump, matching the instant expansion this codebase
  already has *(research R8)*.
