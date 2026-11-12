# Contract: sidebar user groups

**C1 Layout invariants**: each item in exactly one list; Default never stored; group names unique (trimmed, case-insensitive) and non-blank; deleting a group keeps its items, appended to the end of `ungrouped` in their order; `reconcile` drops absent items, keeps a duplicated item in the first group, appends unknown live items to `ungrouped`.

**C2 Naming**: `create` and `rename` return `Blank` or `Duplicate`; the dialog shows the message and keeps the draft; state unchanged on error.

**C3 Drag**: press on a row arms; movement past 4 px activates; below that, release is a click. While active, `over` is `Some` only above a group header, a group's rows, an empty group, the ungrouped rows, or between group headers when the dragged item is a group; otherwise `None` and neither highlight nor marker draws. Release with `over = Some` commits; Escape, release with `None`, or the dragged item vanishing from the live list ends the drag with no change. Default and the chip bar/Overview rows never arm. Hidden (tag filtered) rows are not targets; their stored positions stay.

**C4 Drop commit**: dropping on a collapsed group sets it expanded; dropping on the item's own position changes nothing; sessions are untouched.

**C5 Entries**: `sidebar_entries` = Default, then per group in order `UserGroup(header)` and, if expanded, its member rows (run groups with their runs), then ungrouped rows. No header names a state (FR-013). A header counts as one navigation stop; reveal expands a collapsed group holding the target.

**C6 Menus**: worktree/run-group menu: "Move to group" entries (each group, "Ungrouped", "New group…"), "Move up", "Move down" (disabled at the ends); group menu: Rename, Delete, Move up, Move down. Opened by pointer and by the 803 keyboard chord.

**C7 Store**: `JsonFileStore::{load_layout, save_layout}`; missing, corrupt or older file gives an empty layout; write is atomic.

**C6a New group and collapse**: "New group" is an item of the chip bar's more menu and of a sidebar empty-area menu, with a chord in the shortcut sheet; it opens the naming dialog (C2) with no worktree needed. Collapse: Enter/Space on a focused header, or "Collapse"/"Expand" in the group menu.
