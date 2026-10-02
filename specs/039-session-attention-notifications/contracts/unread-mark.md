# Contract: unread mark and count

**Feature**: 039 | Code: `crates/micold-client/src/ui/material/unread_mark.rs` *(new)* and three
hosts that already exist.

## `UnreadMark`

```rust
UnreadMark::new(roles)          // the mark alone
    .count(2)                   // "● 2"
    .worded(true)               // "● 2 unread"
    .into()                     // Element<'_, Message>
```

| # | Rule | Requirement |
|---|---|---|
| U1 | The mark is a filled circle, 8dp, in the `primary` role; the number and word are in the host's label role and colour. | FR-018, FR-023 |
| U2 | `count(0)` renders nothing, and hosts do not call it with zero. | FR-021, FR-023 |
| U3 | The mark meets the non-text contrast gate (3:1 against its surface) in the light and the dark scheme; the text meets the text gate. | FR-018, FR-023 |
| U4 | Builder form, ending in `.into()`. No free function. | Principle VIII |

## Hosts

| Host | API added | Rendering | Requirement |
|---|---|---|---|
| `TreeItem` (`tree_view.rs`) | `.unread(bool)` | The mark in the trailing slot before any trailing action, and the label in the emphasised weight. The badge slot and the `ActivityBadge` in it are untouched. | FR-018, FR-032 |
| `MenuItem` (`menu.rs`) | field `trailing_mark: Option<usize>` | `● n unread` after `trailing_text`, separated by the menu's trailing gap. With no running count, the mark alone trails. | FR-021, FR-032 |
| `Button` (`button.rs`) | `.trailing_mark(n, tooltip)` | `● n` after the label, inside the button; the tooltip reads `{n} unread sessions in other projects` (`1 unread session in other projects` for one). | FR-023 |

| # | Rule | Requirement |
|---|---|---|
| U5 | The session row shows the mark when `session.unread` and the session is not the one this window has in view. | FR-016, FR-019 |
| U6 | A switcher row shows `unread_session_count(project, in_view)` when it is one or more, on the active project's row as on the others. | FR-021, FR-022 |
| U7 | The switcher's button shows `other_projects_unread(active)` when it is one or more, with its panel closed and open. | FR-023 |
| U8 | A row with the mark keeps its height, and its label truncates before the mark is pushed out. | FR-032 |

## Showcase (FR-030)

Three entries in `crates/micold-client/src/showcase/catalogue.rs`: a session row with the unread
mark beside one without, a switcher row with a running count and an unread count, and the
switcher's button with an unread count.
