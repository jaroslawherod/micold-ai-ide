# Data model: sidebar stable order (#1718)

Removed: `StateGroup`, `Group<T>`, `SidebarGroup`, `GroupHeaderRow`, `ListRow::Header`,
`sidebar::State::group_choices`, `Msg::GroupToggled`.

| Entity | Shape | Rules |
|---|---|---|
| Stable order | `Vec<SidebarEntry>` from `State::sidebar_entries()` | Independent of card state; Default first, then worktrees in listing order; tag filter removes, never reorders |
| `ListRow` | `Entry(SidebarEntry)` only (or replaced by `SidebarEntry`) | One per location row; run group is one entry |
| Row indicator | `StateIcon(CardState, Roles)` | Five states, five glyphs; Idle for no session; run group uses `run_group_state` |
| `RunCard`, `run_group_state` | unchanged | Worst card state over a run group's runs |

No stored field changes (FR-008).
