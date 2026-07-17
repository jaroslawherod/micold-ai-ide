# Contract: UI State, Messages & Shared Components

**Modules**: `src/app.rs` (state + messages + reducers), `src/ui/mod.rs` (render + Esc),
`src/ui/sidebar.rs`, `src/ui/material/{tag,tree_view,menu}.rs`.

## State additions (`State`, `src/app.rs`)

```rust
pub sidebar_filters: BTreeSet<TagFilter>,     // transient; empty ⇒ show all
pub worktree_menu_open: Option<String>,       // dir_name whose context menu is open (≤1)
pub worktree_rename_draft: Option<WorktreeRenameDraft>,
```

`Overlay` gains: `RenameWorktree`, `ConfirmWorktreeDelete { dir_name: String }`.

## Messages (`Message`, `src/app.rs`)

```rust
// Filtering
SidebarFilterToggled(TagFilter),
SidebarFiltersCleared,
// Context menu (lightweight dropdown; NOT an Overlay)
WorktreeMenuToggled(String),   // dir_name
WorktreeMenuDismissed,
// Rename (mirrors project RenameStarted/TextChanged/Confirmed/Cancelled)
WorktreeRenameStarted(String), // dir_name; seeds draft from current display name
WorktreeRenameTextChanged(String),
WorktreeRenameConfirmed,
WorktreeRenameCancelled,
// Delete (see contracts/worktree-removal.md)
WorktreeDeleteRequested(String),
WorktreeDeleteConfirmed,
WorktreeDeleteCancelled,
```

## Reducer behavior (pure, `State::update`)

- `SidebarFilterToggled(f)` inserts/removes `f` in `sidebar_filters`; `SidebarFiltersCleared`
  empties it.
- `WorktreeMenuToggled(dir)` sets `worktree_menu_open = Some(dir)` (or `None` if same); opening
  a menu closes any other. `WorktreeMenuDismissed` → `None`.
- `WorktreeRenameStarted(dir)` → `Overlay::RenameWorktree` + draft `{ dir_name: dir, text:
  current_display_name, error: None }`, closes menu. `…TextChanged` updates text. `…Confirmed`
  validates + `Workspace::set_worktree_name`, closes overlay on success, keeps overlay + sets
  `error` on failure. `…Cancelled` closes overlay, discards draft.
- Delete messages: see contracts/worktree-removal.md.
- `on_escape` maps `RenameWorktree` → `WorktreeRenameCancelled` and `ConfirmWorktreeDelete` →
  `WorktreeDeleteCancelled` (extend the existing `on_escape` match + the GUI Esc subscription).

## `worktree_tree()` filter predicate (pure)

```rust
fn matches_filters(tags: &[Tag], filters: &BTreeSet<TagFilter>) -> bool {
    if filters.is_empty() { return true; }
    filters.iter().any(|f| match f {
        TagFilter::Type(t)  => tags.iter().any(|tag| matches!(tag, Tag::Type(x) if x == t)),
        TagFilter::HasIssue => tags.iter().any(|tag| matches!(tag, Tag::Issue(_))),
        TagFilter::Untyped  => !tags.iter().any(|tag| matches!(tag, Tag::Type(_))),
    })
}
```

Applied to each worktree before building its `TreeItem`; empty filtered result ⇒ the sidebar
body shows an empty-state message with a "clear filters" affordance (FR-027).

## Shared components (Principle VIII — builder API, `.into()`)

### NEW `Tag` chip — `src/ui/material/tag.rs`

```rust
pub struct Tag { /* label, fill, on_fill, size */ }
impl Tag {
    pub fn new(label: impl Into<String>, fill: Rgb, on_fill: Rgb) -> Self;
    pub fn size(self, px: u16) -> Self;      // defaults to tokens::sidebar::TAG
}
impl<'a, Message> From<Tag> for iced::Element<'a, Message> { /* pill container + text */ }
```

- Pure builder terminating in `.into()`; theming via the `Rgb` pair passed in (from
  `design-tokens.md`). No feature-local one-off.

### EXTEND `TreeItem` / `TreeView` — `src/ui/material/tree_view.rs`

- Support a two-line row: primary label line + an optional tag row (a `Row` of `Tag` chips).
  New builder method e.g. `.tags(Vec<Tag>)`.
- Remove the leading git-status icon slot for worktree rows (FR-010); the row no longer renders
  `Icon::Git`/`Icon::Unavailable` as the leading glyph. (The expand twisty stays.)
- Row label uses `tokens::sidebar::NAME`; tags use `tokens::sidebar::TAG`.

### EXTEND `MenuOverlay` — `src/ui/material/menu.rs`

- Add a builder `.anchor(...)` (row-anchored position) so the panel is no longer hard-wired to
  the toolbar's top-right (`TOP_OFFSET`/`align_x(Right)`). Default behavior (toolbar overflow
  menu) preserved when no anchor is set.
- Reused for the worktree context menu: items = Rename (`Icon::Rename`), Delete
  (`Icon::Unavailable` or similar) emitting `WorktreeRename Started` / `WorktreeDeleteRequested`.

## Right-click wiring (`src/ui/sidebar.rs`)

- Wrap each worktree row in `mouse_area(...).on_right_press(Message::WorktreeMenuToggled(dir))`.
- When `worktree_menu_open == Some(dir)`, render the anchored `MenuOverlay` for that row.
- Filter chip row rendered at the top of the sidebar body using `Tag`-style toggle chips bound
  to `SidebarFilterToggled` + a clear control bound to `SidebarFiltersCleared`.

## Tests

- `tests/sidebar_state.rs`: filter toggle/clear set transitions; menu open/close (`Option`
  single-open invariant); rename draft lifecycle incl. validation error path.
- `tests/sidebar_tree.rs`: `worktree_tree()` yields correct tags per row and the correct
  filtered subset for each `TagFilter` and for OR-combined sets; empty-filter shows all.
- `tests/app_state.rs`: `on_escape` arms for the two new overlays; rename confirmed updates
  display name via `Workspace`.
