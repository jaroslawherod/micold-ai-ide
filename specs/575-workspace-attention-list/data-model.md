# Data Model: Attention Indicator on the Sidebar's Worktree Rows

**Feature**: 575 | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

Nothing is stored or sent that 039 does not already store or send (FR-006). Every entity below is
derived on display from 039's per-session `unread` flag.

## Existing, unchanged

| Entity | Where | Fields used |
|---|---|---|
| `Session` | `crates/micold-core/src/session.rs` | `id`, `unread` (039, persisted by the session service), `archived` (closed, 005 FR-015a), `location` |
| `Workspace::sessions` | `crates/micold-core/src/workspace.rs` | every session of every known project, closed ones included |
| `SidebarEntry::{Default, Worktree}` | `crates/micold-client/src/features/sidebar.rs` | `DefaultNode::sessions`, `WorktreeNode::sessions`: the location's non-closed sessions, whether expanded or not |
| `in_view` | `features::attention::in_view(&State)` | the session this window last reported in view (039 FR-019) |

## Counted session (new rule, no new data)

`micold_core::attention::counts_as_unread(session, in_view) -> bool`

| Condition | Source |
|---|---|
| `session.unread` | 039 FR-016 |
| `!session.archived` | spec FR-002, FR-010; 039 US1 scenario 9 |
| `in_view != Some(session.id)` | 039 FR-019, spec edge case "The session in view" |

Used by `Workspace::unread_session_count` (switcher rows; through it `other_projects_unread`, the
switcher's button) and by `SidebarEntry::unread_count`.

## Location attention count (derived)

`SidebarEntry::unread_count(&self, in_view: Option<SessionId>) -> usize`: the number of the
entry's `sessions` for which `counts_as_unread` holds. Never stored; recomputed on each render.

Invariants:

- Independent of `expanded` (FR-003).
- Zero for a location whose only unread sessions are closed (spec edge case).
- For a project whose worktrees are all listed: the sum over `sidebar_entries()` equals the
  project's `SwitcherEntry::unread_count` (FR-010).

## Row unread state (rendering, `TreeItem`)

`crates/micold-client/src/ui/material/tree_view.rs`, replacing `unread: bool`:

```text
RowUnread
  Read                 nothing drawn (default)
  Unread               039 session row: bare mark, label emphasised
  Count(NonZeroUsize)  575 location row: mark + number, label role unchanged
```

Transitions: set by the builder only. `.unread(true)` → `Unread`, `.unread(false)` → `Read`,
`.unread_count(0)` → `Read`, `.unread_count(n > 0)` → `Count(n)`. The last builder call wins.

## Tooltip line (derived)

`features::sidebar::unread_tooltip_line(n) -> Option<String>`: `None` for 0, `"1 unread session"`,
`"{n} unread sessions"`. `features::sidebar::with_unread_line(tooltip, n) -> String` appends it
on a new line, returning `tooltip` unchanged at 0.
