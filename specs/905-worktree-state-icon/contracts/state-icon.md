# Contract: `StateIcon` and the row's state slot

**Feature**: 905 | Research: [R6–R8](../research.md) | Data model: [LocationState](../data-model.md)

## S1 — Component (`crates/micold-client/src/ui/material/state_icon.rs`, Principle VIII)

```text
StateIcon::new(state: LocationState, roles: Roles)   // required inputs
    .size(px: f32)                                   // default TypeRole::SidebarLabel's size
impl From<StateIcon> for Element<'_, M>
```

Exported from `ui/material/mod.rs`. Draws the glyph of the data model's table in a
`Length::Fixed(size)` box centred on both axes; `Unknown` draws an empty box of the same size.

## S2 — Host slot (`TreeItem`, `crates/micold-client/src/ui/material/tree_view.rs`)

- `TreeItem::state_icon(self, StateIcon) -> Self`.
- Pushed after the twisty slot and before the leading icon, at `label_role.size()`.
- `tag_indent` adds `label_role.size() + icon_gap` when set, so the tag line still starts under
  the label.
- A row without a state icon is laid out exactly as before.

## S3 — Sidebar wiring (`crates/micold-client/src/ui/sidebar.rs`, glue)

- `build_items` worktree rows (top level and run rows inside a group):
  `.state_icon(StateIcon::new(node.state(), r))`; tooltip
  `with_unread_line(with_state_line(worktree_tooltip(..), node.state()), unread)`.
- `build_default_item`: the same, with `DEFAULT_LOCATION_LABEL`; the house icon and everything
  else unchanged (FR-002).
- Group rows and session sub-rows: unchanged.

## S4 — Render-free helpers (`crates/micold-client/src/features/sidebar.rs`)

`WorktreeNode::state()`, `DefaultNode::state()` (over `sessions`, via `location_state`) and
`with_state_line(tooltip: String, state: LocationState) -> String` (`"{tooltip}\n{line}"`).

## S5 — Guarantees

| # | Guarantee | FR |
|---|---|---|
| A1 | Every worktree row and the Default row carry a state slot, selected or not, expanded or not. | 001, 002 |
| A2 | `Unknown` keeps the slot's width and draws nothing. | 005, 010 |
| A3 | The label's x and the row's height are the same for all four states, with and without tags. | 010, SC-005 |
| A4 | At the narrowest sidebar the name ellipsizes and the slot keeps its width. | 010 |
| A5 | The three glyphs are distinct codepoints, shared with no other `Icon`. | 007, 008 |
| A6 | Each glyph is ≥ 3:1 against the row's rest, hover and selected fills in both schemes. | 007 |
| A7 | The tooltip has the state's line on its own, before the unread line. | 009 |
| A8 | No state change sends a message other than the re-render (no notification, sound, dialog, selection). | 011 |

## S6 — Showcase

A catalogue entry `material/state_icon.rs` / `StateIcon` with variants `Running`, `Blocked`,
`Idle`, posed `Running`, `Blocked`, `Idle`, `Unknown` and one worktree row with tags carrying it
(FR-012); `tests/showcase_completeness.rs` holds the variant list.
