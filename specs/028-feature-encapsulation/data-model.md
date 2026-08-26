# Phase 1 Data Model: Feature Encapsulation

**Feature**: 028-feature-encapsulation | **Plan**: [plan.md](./plan.md) | **Research**: [research.md](./research.md)

This feature adds no domain data. What it defines is **where existing data lives and who is allowed
to name it**, so this document is a placement map rather than a schema: the target shape of the two
vocabularies (§1), the assignment of all 119 root message variants (§2), and the classification of
all 44 root state fields (§3).

---

## 1. Entities and their target shape

| Entity | Today | After |
|---|---|---|
| **Feature message vocabulary** | 1 (`worktree_form::Msg`, 22 variants) | **10** — one `Msg` per feature module |
| **Feature reducer** | 1 (`worktree_form::update`) | **10** entry points, in one of the three shapes below |
| **Root message vocabulary** (`app::Message`) | 119 variants | **15** — 10 feature wrappers + 5 cross-cutting |
| **Root application state** (`app::State`) | 44 flat public fields | **10 feature structs** + the declared shared members |
| **Feature-owned state struct** | 1 (`worktree_form::WorktreeForm`, reached via `state.worktree_form`) | **10**, one per feature |
| **Outcome vocabulary** (`features::Outcome`) | 12 variants | unchanged in shape; extended only where a conversion needs it |
| **Ownership map** (`OWNERS`, 51 entries) | hand-maintained `const` in a test | **derived from the feature structs**; entries remain only for the shared members |
| **Component-owned state** | 13 `Widget` impls, all presentational | unchanged — see [research.md](./research.md) §R4 |

### 1.1 The three accepted reducer-entry shapes (FR-002, FR-005)

```text
A. pure       features/<n>.rs :: pub fn update(&mut State, Msg) -> Vec<Outcome>
B. effectful  shell/<n>.rs    :: pub fn update(&mut App, Msg) -> Task<Message>
C. neither    a module that declares no `Msg` at all (derivation only)
```

A feature MUST expose A or B (or both — `worktree_form` does today: 18 arms in A, 4 in B) whenever
it declares a `Msg`. Shape C is the FR-005 no-ceremony case: it is legal and needs no allowlist
entry, because a module with no vocabulary has nothing to route.

### 1.2 Feature-owned state struct

```text
features/<n>.rs
    pub struct State { ...the fields OWNERS assigns to <n>... }   // feature-owned
app.rs
    pub struct State {
        pub <n>: <n>::State,      // one line per feature, 10 lines
        pub workspace: Workspace, // declared shared members
        ...
    }
```

**Invariant S3 (FR-009).** A feature struct is never assigned whole. No
`state.<n> = <n>::State::default()` and no `..Default::default()` over one. Fields that are reset
together are reset by a named operation on the feature module, because a wholesale replacement
silently changes what survives — the case `State::set_worktrees` makes concrete
([research.md](./research.md) §R5).

---

## 2. Message attribution — all 119 variants

Owner resolved from the calls each `State::update` / `update_inner` arm makes; see
[quickstart.md](./quickstart.md) §A.3 to reproduce. `worktree_form`'s single root arm already wraps
its own 22-variant vocabulary and needs no conversion.

#### `session` — 37 variants → `session::Msg`

- `Message::TabStripScrolled`
- `Message::TabStripViewportResized`
- `Message::SessionStartRequested`
- `Message::SessionStarted`
- `Message::SessionSelected`
- `Message::SessionCloseRequested`
- `Message::SessionRunning`
- `Message::SessionTitleUpdated`
- `Message::SessionMenuToggled`
- `Message::SessionMenuDismissed`
- `Message::SessionRemoveRequested`
- `Message::SessionRemoveConfirmed`
- `Message::SessionRemoveCancelled`
- `Message::TerminalRestartRequested`
- `Message::ShellInstanceOpenRequested`
- `Message::ShellInstanceSelected`
- `Message::ShellInstanceCloseRequested`
- `Message::ShellInstanceRestartRequested`
- `Message::StripTabMenuRequested`
- `Message::ShellInstanceMenuClosed`
- `Message::TerminalAiCliSelected`
- `Message::ShellInstanceRunning`
- `Message::ShellInstanceExited`
- `Message::TerminalTick`
- `Message::TerminalFocused`
- `Message::TerminalFocusReleased`
- `Message::TerminalBytes`
- `Message::TerminalSelectStart`
- `Message::TerminalSelectUpdate`
- `Message::TerminalSelectCleared`
- `Message::TerminalScrolled`
- `Message::TerminalScrolledTo`
- `Message::TerminalResized`
- `Message::TerminalCopyRequested`
- `Message::TerminalPasteRequested`
- `Message::TerminalContextMenuOpened`
- `Message::TerminalContextMenuClosed`

#### `project` — 19 variants → `project::Msg`

- `Message::ProjectSelectorOpened`
- `Message::SelectorNavigatedInto`
- `Message::SelectorNavigatedUp`
- `Message::SelectorListingReady`
- `Message::SelectorListingFailed`
- `Message::FolderChosen`
- `Message::ProjectSelectorClosed`
- `Message::KnownProjectReopened`
- `Message::RenameStarted`
- `Message::RenameTextChanged`
- `Message::RenameConfirmed`
- `Message::RenameCancelled`
- `Message::ProjectForgetRequested`
- `Message::ProjectForgetConfirmed`
- `Message::ProjectForgetCancelled`
- `Message::ProjectMenuToggled`
- `Message::ProjectMenuDismissed`
- `Message::ProjectOpenRefused`
- `Message::ProjectSwitcherToggled`

#### `worktree` — 18 variants → `worktree::Msg`

- `Message::WorktreesLoaded`
- `Message::WorktreeMenuToggled`
- `Message::WorktreeMenuDismissed`
- `Message::WorktreeDeleteRequested`
- `Message::WorktreeIncludeRequested`
- `Message::WorktreeIncluded`
- `Message::WorktreeExcludeRequested`
- `Message::WorktreeExcluded`
- `Message::WorktreeDeleteConfirmed`
- `Message::WorktreeDeleteCancelled`
- `Message::WorktreeDeleteKeepBranchToggled`
- `Message::WorktreeRenameStarted`
- `Message::WorktreeRenameTextChanged`
- `Message::WorktreeRenameConfirmed`
- `Message::WorktreeRenameCancelled`
- `Message::WorktreeHovered`
- `Message::WorktreeUnhovered`
- `Message::TextCopyRequested`

#### `connection` — 12 variants → `connection::Msg`

- `Message::DaemonConnected`
- `Message::DaemonEvent`
- `Message::DaemonGridFrame`
- `Message::DaemonDisconnected`
- `Message::DaemonConnectFailed`
- `Message::ConnectionTakeoverRequested`
- `Message::DaemonVersionMismatch`
- `Message::DaemonBuildMismatch`
- `Message::ConnectionRestartServiceRequested`
- `Message::DiagnosticsRequested`
- `Message::LogoutSurvivalRequested`
- `Message::LogoutSurvivalOutcome`

#### `settings` — 10 variants → `settings::Msg`

- `Message::ThemePreferenceChanged`
- `Message::ThemeModeCycled`
- `Message::SystemThemeChanged`
- `Message::SettingsOpened`
- `Message::SettingsScrollbackChanged`
- `Message::SettingsEnvIncludeEnabledToggled`
- `Message::SettingsEnvIncludePathChanged`
- `Message::SettingsEnvIncludeTimeoutChanged`
- `Message::SettingsSaved`
- `Message::SettingsCancelled`

#### `sidebar` — 10 variants → `sidebar::Msg`

- `Message::WorktreeExpansionToggled`
- `Message::DefaultExpansionToggled`
- `Message::SidebarFilterToggled`
- `Message::SidebarFiltersCleared`
- `Message::SidebarFilterMenuToggled`
- `Message::ShowAgentWorktreesToggled`
- `Message::SidebarScrolled`
- `Message::SidebarViewportResized`
- `Message::SidebarToggled`
- `Message::SidebarDragMoved`

#### `help` — 3 variants → `help::Msg`

- `Message::HelpMenuToggled`
- `Message::AboutOpened`
- `Message::AboutClosed`

#### `window` — 2 variants → `window::Msg`

- `Message::FieldFocusChanged`
- `Message::WindowResized`

#### `notifications` — 2 variants → `notifications::Msg`

- `Message::NotificationDismissed`
- `Message::NotificationsAdvanced`

#### `worktree_form` — 1 variant → `worktree_form::Msg (exists)`

- `Message::WorktreeForm`

#### `ROOT` — 5 variants → `— stays in `Message``

- `Message::ScrolledBeneathOverlay`
- `Message::EscapePressed`
- `Message::OverlayTransitionFinished`
- `Message::WindowFocusChanged`
- `Message::NoOp`


---

## 3. Root state fields, classified against FR-007

| Field | Owner | Class | Readers outside the feature |
|---|---|---|---|
| `about_open` | `help` | QUALIFIES | — |
| `active_session` | `session` | FEATURE | `app.rs`, `catalog_sync.rs`, `features/sidebar.rs`, `main.rs`, `shell/clipboard.rs`, `shell/daemon_sync.rs`, `shell/env_include.rs`, `ui/mod.rs`, `ui/sidebar.rs` |
| `default_expanded` | `sidebar` | QUALIFIES | — |
| `expanded` | `sidebar` | QUALIFIES | — |
| `focused_field` | `window` | COMPOSITION | `app.rs`, `ui/confirm_delete.rs`, `ui/rename.rs`, `ui/settings_form.rs`, `ui/worktree_form.rs`, `ui/worktree_rename.rs` |
| `forget_target` | `project` | SHELL | `shell/daemon_sync.rs` |
| `help_menu_open` | `help` | COMPOSITION | `ui/mod.rs` |
| `hovered_worktree` | `worktree` | COMPOSITION | `app.rs`, `ui/sidebar.rs` |
| `last_foreground_choice` | `session` | SHELL | `main.rs` |
| `notify` | `notifications` | COMPOSITION | `app.rs`, `shell/daemon_sync.rs`, `shell/persist.rs`, `shell/service_control.rs`, `shell/subscriptions.rs`, `ui/mod.rs` |
| `pending_reveal_scroll` | `sidebar` | SHELL | `main.rs` |
| `pending_tab_reveal` | `session` | SHELL | `main.rs` |
| `project_menu_open` | `project` | COMPOSITION | `ui/mod.rs` |
| `project_switcher_open` | `project` | COMPOSITION | `ui/mod.rs` |
| `rename_draft` | `project` | SHELL | `shell/daemon_sync.rs` |
| `restarted_while_inactive` | `session` | SHELL | `shell/daemon_sync.rs` |
| `reveal_suppressed_for` | `session` | FEATURE | `features/sidebar.rs` |
| `selector` | `project` | SHELL | `shell/workspace.rs` |
| `session_menu_open` | `session` | COMPOSITION | `ui/mod.rs` |
| `session_remove_target` | `session` | SHELL | `shell/daemon_sync.rs` |
| `settings_draft` | `settings` | SHELL | `main.rs`, `shell/persist.rs` |
| `shell_instance_menu` | `session` | COMPOSITION | `ui/mod.rs` |
| `show_agent_worktrees` | `sidebar` | FEATURE | `features/worktree.rs` |
| `sidebar_filter_open` | `sidebar` | QUALIFIES | — |
| `sidebar_filters` | `sidebar` | QUALIFIES | — |
| `sidebar_hidden` | `sidebar` | COMPOSITION | `ui/mod.rs` |
| `sidebar_scroll_offset` | `sidebar` | SHELL | `app.rs`, `main.rs` |
| `sidebar_viewport_height` | `sidebar` | SHELL | `main.rs` |
| `sidebar_width` | `sidebar` | ROOT | `app.rs` |
| `system_scheme` | `settings` | SHELL | `app.rs`, `shell/startup.rs` |
| `tab_strip_scroll_offset` | `session` | SHELL | `main.rs` |
| `tab_strip_viewport_width` | `session` | SHELL | `main.rs` |
| `terminal_context_menu` | `session` | SHELL | `main.rs` |
| `terminal_released` | `session` | ROOT | `app.rs` |
| `theme_pref` | `settings` | COMPOSITION | `app.rs`, `shell/persist.rs`, `shell/startup.rs`, `ui/toolbar.rs` |
| `window_size` | `window` | COMPOSITION | `ui/mod.rs` |
| `worktree_delete_keep_branch` | `worktree` | SHELL | `shell/daemon_sync.rs` |
| `worktree_delete_target` | `worktree` | SHELL | `app.rs`, `shell/daemon_sync.rs` |
| `worktree_error` | `worktree_form` | SHELL | `shell/daemon_sync.rs`, `shell/workspace.rs` |
| `worktree_form` | `worktree_form` | SHELL | `shell/daemon_sync.rs` |
| `worktree_menu_open` | `worktree` | COMPOSITION | `app.rs`, `ui/mod.rs` |
| `worktree_rename_draft` | `worktree` | SHELL | `shell/daemon_sync.rs` |
| `worktrees` | `worktree` | FEATURE | `app.rs`, `features/sidebar.rs`, `main.rs` |

Totals: **20 SHELL**, **12 COMPOSITION**, **5 QUALIFIES**, **4 FEATURE**, **2 ROOT**

### 3.1 What each class means for the migration

| Class | Disposition |
|---|---|
| **QUALIFIES** (5) | Meets FR-007 exactly. All five are blocked by `tests/logical_state_ownership.rs`, which FR-021 forbids relaxing — each becomes an allowlist entry naming its pinning test as the written reason (FR-016). See [research.md](./research.md) §R4. |
| **FEATURE** (4) | FR-008 applies: stays in root state, reason recorded in the shared-member declaration rather than inferred. |
| **COMPOSITION** (12) | Read by `ui/mod.rs` (the root composition) or by another feature's view. Not "its own view" under the spec's Edge Case, so FR-007 does not reach them. They move into the feature struct (Track 2A) and stay reachable. |
| **SHELL** (20) | Read by `main.rs` or `shell/*` to perform an effect. Move into the feature struct (Track 2A); the shell reads them through it. |
| **ROOT** (2) | Read only by `app.rs`. Move into the feature struct with the arms that read them. |

Every one of the 44 fields lands in exactly one feature struct under Track 2A. The classification
above decides only whether FR-007's *component* move applies on top of that — which, today, it does
for none of them.

### 3.2 Shared members that stay flat on the root

`workspace` (six members answering to three features, already keyed per-path in `OWNERS` and
governed by `Workspace`'s own core invariants — `CORE_MEDIATED` in
`tests/feature_write_isolation.rs`) is the one declared shared member. It is not folded into any
feature struct, and `Workspace::forget` continues to apply its invariant across all six.

---

## 4. State transitions

None are added, removed, or reordered. Every transition that exists today exists afterwards, reached
through the same sequence of writes:

- A root arm that called `features::<n>::op(state, ..)` calls `<n>::update(state, Msg::Op(..))`,
  which calls the same `op`.
- A shell arm that called `shell::<m>::on_x(app, ..)` matches `<n>::Msg::X` instead of
  `Message::X` and calls the same function.
- Outcomes are emitted, drained and interpreted exactly as today (`app::drain`, `app::interpret`,
  `OUTCOME_BUDGET`, FIFO ordering — contracts O4/O5 are untouched).

The one place where a transition could change silently is invariant **S3** above, which is why it is
stated as an invariant and checked rather than left to review.
