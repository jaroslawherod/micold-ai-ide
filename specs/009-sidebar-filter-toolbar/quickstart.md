# Quickstart: Sidebar Filter Toolbar Button

## Prerequisites

- Rust stable toolchain via `mise` (already set up for this repo).
- A project open with at least a couple of tagged worktrees (conventional-type branch names,
  e.g. `feature/...`, `fix/...`, and/or one linked to a Jira-style issue key) so
  `available_tag_filters()` is non-empty.

## Headless validation (logic, no GUI)

```sh
cargo test --no-default-features   # pure core: reducer, on_escape, icon regression locks
cargo test sidebar_state            # new open/close/mutual-exclusion/Escape coverage
cargo test icons                    # Icon::Filter codepoint + Icon::ALL count
cargo test icons_font               # regenerated font still resolves every Icon::ALL glyph
```

Expected: all pass, with `tests/sidebar_state.rs` covering (see `data-model.md`'s state
transitions):

1. `sidebar_filter_open` starts `false`.
2. `SidebarFilterMenuToggled` flips it, and closes `help_menu_open`/`project_switcher_open`.
3. Opening `help_menu_open` or `project_switcher_open` closes `sidebar_filter_open`.
4. `on_escape(state)` returns `Some(SidebarFilterMenuToggled)` when `sidebar_filter_open` is
   `true`, regardless of `state.overlay`.
5. Toggling the panel open/closed never changes `state.sidebar_filters`.

## Manual GUI validation (`cargo run`)

1. `cargo run` with a project that has ≥1 tagged worktree.
2. **Default state**: sidebar shows the worktree list with no filter chips visible. Confirm
   the sidebar header shows a filter icon button (three descending-length lines,
   `filter_list`) at the **left** edge, ahead of the "Worktrees" title.
3. **Open the accordion**: click the filter button. The chip row (by type / has-issue /
   untyped) expands open below the header, pushing the worktree list down — not a floating
   panel over it.
4. **Active-state indicator**: with the accordion open, click a chip to activate a filter, then
   close it (click the button again). Confirm the filter button's icon is now tinted (active
   color) even though the accordion is collapsed, and the worktree list is filtered.
5. **Dismissal paths**: reopen the accordion, then verify each of:
   - Pressing Escape closes it.
   - Clicking the filter button again closes it.
   (There is no outside-click dismissal — the accordion is inline content, not a floating
   panel; research R7.) In every case, the previously-activated filter stays applied (list
   stays filtered).
6. **Clear filters**: reopen the accordion, click "Clear filters". Confirm the worktree list
   shows all worktrees again and the filter button's tint reverts to inactive — the accordion
   may stay open.
7. **Empty-tag project**: open (or switch to) a project with no tagged worktrees. Confirm the
   filter button is still present; opening the accordion shows a "No tags to filter yet."
   message rather than an empty or broken panel.
8. **Live update**: with the accordion open and at least one filter available, add a new
   worktree with a different conventional type in another terminal/session. Confirm the chip
   set updates to include the new type without needing to close and reopen the accordion.

## Visual/asset check

- Confirm the filter icon renders as three descending-length horizontal lines (not a blank
  "tofu" box) in both light and dark themes, at the sidebar header's left edge.
- `assets/fonts/MaterialSymbolsOutlined.ttf` file size has grown from ~4 KB to the low
  single-digit-MB range (full glyph coverage) — sanity-check this isn't accidentally still the
  narrow subset (`ls -la assets/fonts/MaterialSymbolsOutlined.ttf`).
