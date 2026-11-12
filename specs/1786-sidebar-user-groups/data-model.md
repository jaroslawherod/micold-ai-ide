# Data model: Sidebar user groups

## SidebarLayout (per project; `micold-core::sidebar_layout`)
| Field | Type | Notes |
|---|---|---|
| `version` | u32 | 1; an unknown or missing version loads as empty |
| `groups` | Vec<UserGroup> | order = display order (FR-019) |
| `ungrouped` | Vec<LayoutItem> | saved order of ungrouped items; unknown rows append in stable order |

## UserGroup
| Field | Type | Rules |
|---|---|---|
| `id` | GroupKey (u64, monotonic per layout) | never reused |
| `name` | String | trimmed, non-empty, unique case-insensitively (FR-004) |
| `collapsed` | bool | default false (FR-011) |
| `items` | Vec<LayoutItem> | ordered members |

## LayoutItem
`Worktree(dir_name: String)` | `Run(GroupId)`. Default is never an item (FR-015). An item appears in exactly one of `groups[*].items` or `ungrouped` (FR-005); every operation moves, never copies.

## Operations (pure, return `Result<(), LayoutError>`)
`create(name) -> GroupKey`, `rename(id, name)`, `delete(id)` (items go to the front of `ungrouped`... see contract C1: appended at the group's former items order), `move_item(item, DropTarget)`, `move_group(id, to_index)`, `step_item(item, ±1)`, `step_group(id, ±1)`, `set_collapsed(id, bool)`, `reconcile(live: &[LayoutItem])`.

`LayoutError`: `Blank`, `Duplicate`, `UnknownGroup`, `UnknownItem`, `IsDefault`.

## DropTarget
`Group { id, index }` (into a group's items at `index`) | `Ungrouped { index }` | `GroupSlot { index }` (header reorder). `None` is the invalid place.

## Transient (client, not stored)
`DragState { armed: Option<(Item, origin)>, active: bool, over: Option<DropTarget> }`; naming draft `{ target: New | Rename(id) | NewAndMove(item), text, error }`.

## Persistence
`layouts/<project id>.json`; write atomic; unreadable file renamed `.corrupt` and an empty layout loads (FR-018). State transitions: none beyond the operations above.
