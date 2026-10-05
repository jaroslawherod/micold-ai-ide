# Contract: attention indicator on a location row

**Feature**: 575 | Code: `crates/micold-client/src/ui/material/tree_view.rs` (host),
`crates/micold-client/src/features/sidebar.rs` and `crates/micold-core/src/attention.rs` (counts),
`crates/micold-client/src/ui/sidebar.rs` (wiring). Extends 039's
[`unread-mark.md`](../../039-session-attention-notifications/contracts/unread-mark.md).

## API

```rust
// core: the one rule for a counted session (R1)
micold_core::attention::counts_as_unread(&session, in_view) -> bool

// client, render-free (R2, R5)
entry.unread_count(in_view) -> usize                       // SidebarEntry
features::sidebar::unread_tooltip_line(n) -> Option<String>
features::sidebar::with_unread_line(tooltip, n) -> String

// host (R3): the counted variant on TreeItem, builder form (Principle VIII)
TreeItem::new(0, name, tint)
    .unread_count(n)            // "● n" at the trailing edge; 0 draws nothing
    .trailing_element(cluster)
```

## Rules

| # | Rule | Requirement |
|---|---|---|
| A1 | A location row draws `UnreadMark::new(r).count(n)` when `n = entry.unread_count(in_view) ≥ 1`, and nothing when `n = 0`. The view never calls `.count(0)` (U2). | FR-001, FR-002 |
| A2 | `n` counts the location's sessions with `counts_as_unread`: unread, not closed, not in view. | FR-002 |
| A3 | `n` and the indicator are the same with the row expanded or collapsed; session rows keep `.unread(row_unread(..))` as in 039 U5. | FR-003 |
| A4 | The indicator sits after the label and annotation and before the trailing element. The label is `Ellipsized`, so a narrow row shortens the name and the indicator keeps its full width; the row keeps its one- or two-line height. | FR-004, U8 |
| A5 | The indicator's bounds are the same with the row hovered and at rest, and do not intersect the row-action cluster's. | FR-004 |
| A6 | The number is drawn in the tree's label role (`TypeRole::SidebarName`) and the theme's text colour, also on a missing or invalid worktree; the location row's name keeps its role (no emphasis). | FR-004, R4 |
| A7 | The row tooltip ends with `1 unread session` / `n unread sessions` when `n ≥ 1`, and has no such line at 0. | FR-005 |
| A8 | `Workspace::unread_session_count` uses `counts_as_unread`, so the switcher's rows and button skip closed sessions; for a project with no hidden worktree, the location counts add up to its switcher count. | FR-010 |
| A9 | The mark meets 3:1 and the number 4.5:1 against the sidebar fill, the hovered row's fill and the selected pill, in the light and the dark scheme. | FR-013, U3 |
| A10 | `.unread(bool)` keeps its signature and rendering for session rows; a `TreeItem` is never both `Unread` and `Count`. | FR-011, Principle V |

## Showcase

The `UnreadMark` entry (`showcase/sections/atoms.rs::unread_mark`) gains three posed location
rows, rendered in both themes: a worktree row with `● 2` collapsed, the same row expanded with
its two unread session rows, and a worktree row with no indicator (FR-012).
