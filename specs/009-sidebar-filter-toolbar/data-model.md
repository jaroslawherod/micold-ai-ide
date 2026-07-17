# Phase 1 Data Model: Sidebar Filter Toolbar Button

No persisted data changes. This feature adds one transient UI-state field and one message;
everything else (filters, matching, tags) is feature 008's existing model, untouched.

## Entities

### `State.sidebar_filter_open: bool` (new field on `micold_ai_ide::app::State`)

- Whether the filter panel is currently shown. Defaults to `false` (hidden by default, FR-002).
- Not persisted (no `#[serde]` involvement) — matches the existing, equally-transient
  `help_menu_open` and `project_switcher_open` fields. Resets to `false` on every app launch.
- Mutually exclusive with `help_menu_open` and `project_switcher_open`: toggling any one of
  the three closes the other two, mirroring the existing `HelpMenuToggled` /
  `ProjectSwitcherToggled` reducer behavior (`src/app.rs:479-487`).

### `Message::SidebarFilterMenuToggled` (new variant on `micold_ai_ide::app::Message`)

- Emitted by the sidebar header's filter trigger button, and by the new Escape-dismiss path.
- Reducer: flips `sidebar_filter_open`, and closes `help_menu_open`/`project_switcher_open`
  when opening (symmetric with the existing two toggle messages).

### `Icon::Filter` (new variant on `micold_ai_ide::icons::Icon`)

- Maps to Material Symbols Outlined `filter_list`, codepoint `U+E152` (research R5/R7).
- No new `IconSurface` role: rendered with the existing `primary` (active) /
  `on_surface_variant` (inactive) tints already used by sibling sidebar-header buttons
  (research R4).

## Unchanged (reused as-is)

- `TagFilter` (`src/app.rs:144-152`), `State.sidebar_filters: BTreeSet<TagFilter>`
  (`src/app.rs:463`), `Message::SidebarFilterToggled` / `SidebarFiltersCleared`
  (`src/app.rs:293,295`), `State::matches_filters()` (`src/app.rs:155-166`),
  `State::filtered_worktree_tree()` (`src/app.rs:1075-1083`),
  `State::available_tag_filters()` (`src/app.rs:1085-1112`) — all untouched. This feature only
  changes *where* the UI built from these is displayed.
- `filter_bar()` / `filter_chip()` (`src/ui/sidebar.rs:214-271`) — reused verbatim as the
  `FilterOverlay`'s panel content; no signature or rendering change.

## State Transitions

```text
sidebar_filter_open: false (default)
  --SidebarFilterMenuToggled--> true   (help_menu_open := false, project_switcher_open := false)
  --SidebarFilterMenuToggled--> false  (from true)
  --HelpMenuToggled (opening)--> false (mutual exclusion, symmetric with existing pattern)
  --ProjectSwitcherToggled (opening)--> false (mutual exclusion, symmetric with existing pattern)
  --Escape (while true, no modal Overlay open)--> false (research R3)
```

No transition here alters `sidebar_filters` (the active filter set) — closing the panel by any
path (toggle, outside click, Escape) is purely a visibility change (FR-007/FR-008).
