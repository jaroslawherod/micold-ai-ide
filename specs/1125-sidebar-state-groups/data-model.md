# Data model: Sidebar grouped by state, with Overview as its first row (#1125)

Nothing is written to disk. Everything is derived per render from the window's state, except the
user's group choices, which `sidebar::State` holds in memory for the run (spec Key Entities).

## StateGroup (new, `micold_core::sidebar_groups`)

| Variant | Holds card states | Label | Starts open |
|---|---|---|---|
| `NeedsAction` | `NeedsAction`, `NeedsAttention` | "Needs action" | yes |
| `Working` | `Working` | "Working" | yes |
| `Idle` | `Idle` | "Idle" | yes |
| `Done` | `Done` | "Done" | no |

`StateGroup::ALL` is the display order above (FR-001). Derives `Ord` in that order, `Hash`, `Copy`.
`StateGroup::of(CardState) -> StateGroup` is total (contract G1).

## Group<T> (new, `micold_core::sidebar_groups`)

| Field | Type | Rule |
|---|---|---|
| `group` | `StateGroup` | |
| `items` | `Vec<T>` | the input items of this group, in input order (FR-003) |

`group_by_state(items: impl IntoIterator<Item = (CardState, T)>) -> Vec<Group<T>>`: one `Group` per
group with at least one item, in `StateGroup::ALL` order (FR-002: an empty group is not returned).
Count = `items.len()`; a run group is one item (FR-002).

## RunCard (new, `micold_core::sidebar_groups`)

What one run of a run group contributes to the group's state (Edge Cases, *Run groups*).

| Variant | Card state it counts as |
|---|---|
| `Worktree(CardState)` | that state |
| `NoWorktree { failed: true }` | `NeedsAttention` |
| `NoWorktree { failed: false }` | `Idle` |

`run_group_state(runs) -> CardState`: the minimum (worst) over the runs; `Idle` for no runs.

## SidebarGroup (new, client `features::sidebar`, render-free)

| Field | Type | Rule |
|---|---|---|
| `group` | `StateGroup` | |
| `open` | `bool` | `group_open(choice, group, holds_current)` (R4) |
| `entries` | `Vec<SidebarEntry>` | the group's `Default`, `Worktree` and `Group` entries, in `sidebar_entries()` order |

`State::sidebar_groups()` builds them: each entry of `sidebar_entries()` gets its state from
`features::overview::location_card_state` (a run group from `run_group_state` over its runs), then
`group_by_state`. `holds_current` is whether the group's entries hold the current session's
location (`current_session_location()`), a run group's runs included.

## ListRow (new, client `features::sidebar`, render-free)

One row of the drawn list, top to bottom.

| Variant | Carries | Drawn as |
|---|---|---|
| `Header(GroupHeaderRow)` | `group`, `count`, `open` | a heading `TreeItem` (R8), text `header_text(group, count)` |
| `Entry(SidebarEntry)` | an entry of an open group | as today, two lines (R6) |

`State::sidebar_list()`: for each `SidebarGroup`, its header, then its entries when `open`.
`row_heights`, `row_index`, `location_row` and `current_session_row` take `&[ListRow]`: a header is
one row of one-line height; every location row is two-line; session rows are one line as today.

## sidebar::State (changed, client)

| Field | Change | Notes |
|---|---|---|
| `group_choices: BTreeMap<PathBuf, BTreeMap<StateGroup, bool>>` | new | per project, per group: the user's last open (`true`) or closed (`false`) choice; absent = never chosen. Not pruned, not persisted (R4) |
| `more_menu_open: bool`, `more_menu_anchor: Option<(u16, u16)>` | new | the chip bar's more menu (R11) |
| `filter_open` | removed | the filter accordion is gone (FR-014) |

## worktree::State (changed, client)

| Field | Change |
|---|---|
| `hovered: Option<(String, usize)>` | was `Option<String>`: the hovered row's dir name and its drawn index at hover time (R14) |

## Messages (client)

| Message | Change | Effect |
|---|---|---|
| `SidebarMsg::GroupToggled(StateGroup)` | new | store `!open` for the active project's group (R4) |
| `SidebarMsg::MoreMenuToggled`, `SidebarMsg::MoreMenuAnchored((u16, u16))` | new | open/close the more menu; record its trigger's corner |
| `SidebarMsg::FilterMenuToggled` | removed | |
| `overview::Msg::Opened { settings_open }` | new | open the Overview; no-op when open, while Settings is open or an activation is pending (R9) |
| `WorktreeMsg::Hovered(String, usize)` | changed | carries the drawn index (R14) |

## Row text (derived, `micold_core::overview_text`)

| Row | First line | Second line (`sidebar_second_line`) |
|---|---|---|
| *Default* | "Default" | "project root · " + sessions line |
| worktree, title is not the branch | the card title (issue title, else branch, else name) | `branch_line(title, branch)` + " · " + sessions line |
| worktree, title is the branch, or no branch | the card title | sessions line only |

The row tooltip (`features::sidebar::worktree_tooltip`) gains a first line with the full title and a
`Branch` line with the full branch when the row shortened either (FR-012); the rest is unchanged.

## Overview row (derived, client view)

| Part | Source |
|---|---|
| label, icon | "Overview", `Icon::Overview` |
| badge | `features::overview::badge(&ui::overview::cards(state))` (R9, R10) |
| selected | `state.overview.open` |
| enabled | a project is active and Settings is closed |
| tooltip | `features::overview::overview_tooltip(badge)` |
