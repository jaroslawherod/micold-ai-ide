# Contract: Sidebar Reveal Control

Governs the "Show agent worktrees" chip, the shared `ToggleChip` primitive it is built on, and
the agent badge on revealed rows. Normative statement of FR-010/FR-010a–d and FR-010b
(research R4, R5, R6).

## `ToggleChip<M>` (NEW — src/ui/material/toggle_chip.rs)

Promoted from the sidebar's private `filter_chip()` so the reveal chip reuses it rather than
forking a second copy (Constitution Principle VIII, Component-reuse gate). Builder form, required
inputs in `new()`, chain terminating in `.into()`.

```rust
/// A pill-shaped on/off chip: filled in its accent when active, outlined when inactive.
/// Pressing it emits `on_press`. Builder form (Principle VIII):
/// `ToggleChip::new(label, on_press, roles).active(b).accent(fill, on).into()`.
pub struct ToggleChip<M> { /* label, message, roles, active, accent */ }

impl<M> ToggleChip<M> {
    /// A chip showing `label`, emitting `on_press` when pressed, themed by `roles`.
    /// Inactive by default, with the neutral `surface_variant`/`on_surface_variant` accent.
    pub fn new(label: impl Into<String>, on_press: M, roles: Roles) -> Self;

    /// Whether the chip reads as on (filled) or off (outlined).
    pub fn active(mut self, active: bool) -> Self;

    /// The `(fill, on_fill)` pair used while active — e.g. a worktree type's tag color.
    pub fn accent(mut self, fill: Rgb, on_fill: Rgb) -> Self;
}

impl<'a, M: Clone + 'a> From<ToggleChip<M>> for Element<'a, M>;
```

### Invariants

1. **Pixel-identical to today's filter chip** when driven with the same inputs: same padding
   (1px vertical, `spacing::SM` horizontal), `shape::FULL` radius, `sidebar::TAG` text size, 1px
   `outline` border when inactive and no border when active, `on_surface_variant` text when
   inactive. This feature must not restyle the existing filter chips.
2. **No visual state of its own**: `active` is supplied by the caller, never latched internally.
3. **Theme-aware**: every color comes from the supplied `Roles` (Principle VIII, Principle VI).

### Required call sites

| Call site | Accent | `active` |
|---|---|---|
| `sidebar::filter_chip()` (rewritten to delegate) | `r.type_tag(t)` / `r.issue_tag()` / neutral for `Untyped` | `state.sidebar_filters.contains(&filter)` |
| Reveal chip (new) | neutral default | `state.show_agent_worktrees` |

`filter_chip()` MUST be rewritten to delegate. Leaving both implementations alive is the
duplication the Component-reuse gate rejects.

## Reveal chip placement (src/ui/sidebar.rs)

The filter accordion's body becomes:

```text
column![
    reveal_chip(state, r),   // ← unconditional, always first
    filter_bar(state, r),    // ← existing tag chips, unchanged
]
```

### Invariants

1. **Unconditional (FR-010c)**: the reveal chip is rendered whether or not
   `available_tag_filters()` is empty. It MUST NOT be placed inside `filter_bar()`, which returns
   early with *"No tags to filter yet."* — precisely the state of a project whose only worktrees
   are agent-owned, i.e. the case where the control matters most (research R4).
2. **Label**: `"Show agent worktrees"`.
3. **Message**: presses emit `Message::ShowAgentWorktreesToggled` and nothing else.
4. **Filters untouched (FR-010d)**: the reveal chip does not read, clear, or reorder
   `sidebar_filters`; the "Clear filters" control does not reset `show_agent_worktrees`.
5. **No new overlay or animation**: it lives inside the existing accordion and inherits its
   expand/collapse progress.

## Reducer (src/app.rs)

```rust
/// Toggle whether agent-owned worktrees are included in the sidebar list (FR-010).
/// Transient — never persisted, so every app start begins hidden (FR-010a).
Message::ShowAgentWorktreesToggled => {
    self.show_agent_worktrees = !self.show_agent_worktrees;
}
```

### Invariants

1. **Sole mutation**: flips `show_agent_worktrees` and touches nothing else — not
   `sidebar_filters`, not `expanded`, not `overlay`, not `hovered_worktree` (FR-010d).
2. **Default off**: `State::default()` leaves it `false` (FR-010a). It MUST NOT appear in the
   persisted store schema.
2a. **Reset on project switch (FR-010e)**: `restore_after_activation()` (`src/app.rs:1369`) MUST
   set `show_agent_worktrees = false`, immediately alongside the existing `default_expanded = false`
   reset and for the same reason — view state switched on for one project must not silently carry
   into another. This is deliberately **unlike** `sidebar_filters`, which survives a switch.
   Switching back does not restore it: nothing is remembered per project.
3. **Idempotent pairs**: two toggles return the list to its exact prior contents.
4. **No re-discovery**: the reducer triggers no git call and no `Task` — the change is a pure view
   recomputation (FR-008).

## Agent badge (src/naming.rs, src/ui/sidebar.rs)

```rust
pub enum Tag {
    Type(ConventionalType),
    Issue(String),
    Status(WorktreeStatus),
    /// The worktree belongs to an AI assistant, not the user (FR-010b). Label only —
    /// deliberately has no `TagFilter` counterpart.
    Agent,
}
```

### Invariants

1. **Produced at the `State` layer**: appended by `State::worktree_tags()` when
   `worktree.is_agent_owned()`, alongside `Tag::Status`. NOT produced by `parse_tags(dir_name)`,
   which cannot see the branch.
2. **Not filterable**: `TagFilter` gains no variant, so `matches_filters()` is unchanged and the
   badge never appears as a filter chip (research R5).
3. **Rendered by the existing shared `Tag` primitive**: label `"agent"`, accent
   `roles.on_surface_variant`. No new widget, no icon-font change.
4. **Only visible when revealed**: hidden worktrees produce no row, so the badge is by construction
   only ever seen with the reveal control on.
5. **Distinguishable (FR-010b)**: a revealed row always carries the `agent` chip — it is not
   conditional on the worktree's health, name, or session count.

## Actions on revealed rows (FR-013)

A revealed row keeps the standard row-action cluster — start session, rename, delete. No action is
disabled, hidden, reordered, or given an extra confirmation because the worktree is agent-owned.

**`row_actions_cluster()` MUST be unchanged by this feature.** A diff that touches it is a signal
the implementation drifted toward special-casing agent rows, which FR-013 forbids. Deletion's only
guard is the app's existing confirm dialog (`Overlay::ConfirmWorktreeDelete`), which already covers
the branch-delete choice; no agent-specific warning is added.

## Terminology (spec Terminology section)

Every user-visible string introduced by this feature MUST use **"agent"**, never "assistant":

| Surface | Required text |
|---|---|
| Reveal control label | `Show agent worktrees` |
| Row badge | `agent` |
| User-guide section (FR-012) | "Agent worktrees" |

"Assistant-owned" is internal spec prose only and MUST NOT reach the UI or the docs.
