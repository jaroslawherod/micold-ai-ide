# Contract: what a navigation press does (UI)

**Feature**: 803 | [plan](../plan.md) | [data model](../data-model.md) |
[research R3–R8](../research.md)

## U1 — Gate (FR-012)

`navigation::update` returns `Step::None` while Settings is open (`settings_draft.is_some()`) or any
registered surface is open (`overlay::registry::topmost(state).is_some()`: dialogs, context menus
including the terminal's, the project switcher panel, the filter panel). Nothing changes.

## U2 — Resolving the step

| Level | Order (read at the press) | Current | Result |
|---|---|---|---|
| Location | `location_order()`: Default, worktrees after the tag filter, worktrees of expanded run groups, in drawn order | `nav_stop` if set and in the order, else `current_session_location()`; none → `step` with `None` | target has a main session → `Step::Session(main)`; else `Step::Stop(location)` |
| Session | current location's `listed_sessions()` | `session.active` | `Step::Session(id)`; no current session, or `nav_stop` is set → `Step::None` |
| Project | `switcher_entries()` where `available` | `workspace.active` | `Step::Project(path)` |

`step` (core) wraps both ends, keeps a one-entry list on its entry (→ `Step::None`, nothing
changes) and returns `None` for an empty list (→ `Step::None`). No active project: location and
session orders are empty.

## U3 — Applying the step (`main.rs`, `Message::Navigate`)

`terminal_had_focus = state.terminal_focused()` is read before anything changes.

| Step | Effects, in order |
|---|---|
| `Session(id)` | dispatch `Message::Session(SessionMsg::Selected(id))` (the click: closes Changes, Compare, Overview; daemon foreground; terminal focus); arm the reveal scroll; if `!terminal_had_focus`: release the terminal, `keyboard_row = Session(id)` |
| `Stop(location)` | `nav_stop = Some(location)`; release the terminal; `keyboard_row = Location(location)`; arm the reveal scroll (to `keyboard_row`). The next press starts with `terminal_had_focus = false` (the keyboard is on the stop's row), so a session it lands on gets the ring, not the terminal (FR-010) |
| `Project(path)` | dispatch `Message::Project(ProjectMsg::Reopened(path))`; if `!terminal_had_focus`: release the terminal, `keyboard_row = Session(restored current)` when one is restored |
| `None` | nothing |

Never: start, stop, close or archive a session; change a main session (FR-011, SC-005).

## U4 — Sidebar rendering

- Every row is built with `TreeItem::keyboard_focus(is_named, …)`, so the widget tree keeps its
  shape when `keyboard_row` moves (R7). The row whose `RowRef` equals `keyboard_row` is enabled,
  focused and shows the focus ring on the first frame after the press; every other row's wrapper is
  disabled (out of the Tab traversal) and draws nothing.
- Its `on_focus_change(false)` sends `SidebarMsg::KeyboardRowLeft`, which clears `keyboard_row`.
- The selected (highlighted) row stays the session the terminal shows (SC-004); on a `Stop` the
  highlight stays where it was and the ring marks the stop.
- A current helper session's location row opens by the existing reveal rule; the target row is
  scrolled into view only when out of view (`scroll_target`, minimal move).

## U5 — Clearing

| Event | `nav_stop` | `keyboard_row` |
|---|---|---|
| current session changes (any writer) | cleared | kept only if set in the same `Navigate` |
| project entered | cleared | cleared unless set by the same `Navigate` |
| a row press, the terminal focused by click, Tab (`KeyboardRowLeft`, pointer selection) | cleared | cleared (focus left the row) |
| the stop's location leaves the order (removed, filtered) | ignored at the next press | row gone; cleared |
