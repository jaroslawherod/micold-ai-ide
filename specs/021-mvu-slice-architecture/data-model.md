# Phase 1 Data Model: Feature-Module MVU Architecture

**Feature**: 021 | **Date**: 2026-08-07 | **Plan**: [plan.md](./plan.md)

This feature adds no domain data. Every entity below is *organizational* — it describes how
existing state is grouped, not new state. The persisted format is unchanged (FR-026), so nothing
here has a serialized representation.

## Entities

### Feature module (Tier 1)

One module holding a feature's types together with every helper function operating on them.

| Property | Rule |
|---|---|
| Contents | A feature's types + all helpers over them. Never split state/update/view across files (FR-001a). |
| Naming | `features/<feature>.rs`, named for the feature or its principal type |
| Message vocabulary | None of its own, unless it is also a nested unit |
| Render dependency | None — must stay render-free and separately testable (FR-006) |
| Testability | At least one test constructing only this feature's types (FR-004, SC-004) |

**Instances** (9): `worktree`, `worktree_form`, `session`, `project`, `sidebar`, `settings`,
`notifications`, `connection`, plus the `overlay` registry module.

**Validation**: a maintainer asked "where does feature X live?" names exactly one module (SC-010).

### Feature reducer module (Tier 3)

The root reducer's arms for one feature, extracted into their own module over the shared state.

| Property | Rule |
|---|---|
| Operates on | The shared `State` — not a private sub-state |
| May mutate | Only its own feature's data (FR-020), enforced by guard test (FR-024a) |
| May read | Any feature's data, for display (FR-003a) |
| Returns | `Vec<Outcome>` where a cross-feature consequence exists; nothing otherwise (FR-021) |
| Effectful half | Its `Task`-returning arms live in the shell module for the system they address (research.md §2) |

**Relationship**: one per feature module. Does *not* imply a nested state or message type.

### Nested unit (Tier 3, conditional)

A feature module that additionally owns its own state, message vocabulary and reducer.

| Property | Rule |
|---|---|
| Permitted only when | Opened, edited and dismissed as a unit whose intermediate state no other feature reads (FR-003) |
| Recorded | Per-feature verdict with evidence (SC-004a) — research.md §5 |
| Count | **One**: `worktree_form`. Zero would also be valid (FR-004b). |

**Instance — `worktree_form`**: owns `WorktreeForm`, `WorktreeFormStatus`, `BranchSource`,
`ResolutionState` and a private message type absorbing 22 root variants (18 `AddWorktree*`, 4
`WorktreeCreate*`). The root routes to it through one wrapping variant.

### Feature outcome

An explicit value returned by a feature reducer module describing a consequence outside its own
data. The **only** sanctioned channel for cross-feature writes.

| Property | Rule |
|---|---|
| Produced by | A feature reducer module, returned — never applied in place |
| Interpreted by | The root reducer, and only the root (FR-022) |
| Termination | Interpretation must terminate and must not depend on module composition order (FR-024) |
| Scope | Required only where a cross-feature consequence exists; not blanket plumbing (FR-021) |
| Also carries | Shell-effect requests that cannot be plain ports, e.g. clipboard (research.md §7) |

**Known instances at plan time**:

| Outcome | Emitted by | Interpreted as |
|---|---|---|
| `SessionsClosed(Vec<SessionId>)` | worktree delete | Session feature closes them |
| `OverlayDismissed(SurfaceId)` | worktree delete | Overlay registry dismisses it |
| `ClipboardWrite(String)` | any feature | Shell issues `iced::clipboard::write` |
| `NotificationRaised(Notification)` | any feature | Notification queue push |

**Termination rule**: an outcome's interpretation may itself produce outcomes. The root drains a
work queue with a fixed iteration bound; exceeding it is a panic in debug and a logged no-op in
release, so a cycle fails loudly in tests rather than hanging the UI (FR-024).

### Floating surface (Tier 2)

The uniform representation of any transient surface over the base view — modal and popover alike.
Replaces both the 10-variant `Overlay` enum and its 9-variant `ClosingOverlay` twin.

| Property | Rule |
|---|---|
| Knows | How to render itself, what dismisses it, which stacking band it occupies |
| Built on | Feature 017's existing `Layer`/`Surface`/`Trigger` vocabulary — not a parallel one (FR-014) |
| API | Chainable builder terminating in `.into()` (FR-030, Principle VIII) |
| Snapshot | Must support rendering a copy whose live state is cleared, so it can animate out — including reopening mid-animation (FR-011) |
| Dismissal | Must not alter state the dismissal does not own — closing the filter panel leaves filters intact (FR-013) |

**Migrated onto it** (16 surfaces): 9 real `Overlay` variants — the enum's 10th is `None`, the
absence of a surface, which the registry represents by having nothing open — and the 7 loose popover
fields — `help_menu_open`, `project_switcher_open`, `sidebar_filter_open`, `worktree_menu_open`,
`project_menu_open`, `terminal_context_menu`, `session_menu_open`.

### Overlay registry (Tier 2)

The single registration point through which a surface becomes known to the generic dispatch.

| Property | Rule |
|---|---|
| Adding a surface costs | Its own module + at most one registration line (FR-009, SC-001) |
| Central match statements to edit | **Zero**, down from six (SC-001) |
| Omitting registration | Caught at build time or by guard test, never at runtime (FR-010) |
| Preserves | Popover-checked-before-modal dismissal priority; opening a modal closes popovers (FR-012) |

### Service capability (port)

A narrow, single-purpose declaration of an I/O need, stated by the render-free core, satisfied by a
real implementation from the binary or a fake from a test.

| Property | Rule |
|---|---|
| Narrowness | A consumer needing one operation must not be forced to supply or fake unrelated ones (FR-016) |
| Declared in | `micold-core` |
| Supplied by | The binary, at a single assembly point (FR-018) |
| Fake | Every capability has one, and at least one test exercises real behavior through it (FR-019, SC-005) |

**Existing (7)**: `Git`, `ProjectStore`, `SettingsStore`, `FolderScanner`, `TerminalBackend`,
`TerminalHandle`, `AiCliProvider`.
**To declare (3)**: env-include resolution, OS theme probe, clipboard — the last as an outcome
rather than a called port (research.md §7).

### Composition shell

The thin layer owning composition, routing, outcome interpretation, and capability supply. Holds no
feature logic, and is divided by **external system**, never by feature (FR-019a).

| Module | External system it addresses |
|---|---|
| `shell/startup.rs` | Process launch, window creation, boot sequence |
| `shell/capabilities.rs` | The single assembly point for concrete implementations (FR-018) |
| `shell/persist.rs` | Local filesystem — project store, settings store |
| `shell/daemon_sync.rs` | The session daemon over its protocol |
| `shell/subscriptions.rs` | iced's event loop — keyboard, cursor, focus, timers |
| `shell/env_include.rs` | Environment-include script resolution |
| `shell/os_theme.rs` | Operating-system light/dark preference |

## Relationships

```text
                 ┌──────────────────────────┐
                 │   Composition shell      │  supplies capabilities,
                 │   (by external system)   │  interprets outcomes
                 └───────────┬──────────────┘
                             │ routes messages
                 ┌───────────▼──────────────┐
                 │   Root state + reducer   │  composition & routing ONLY (FR-002)
                 └───────────┬──────────────┘
             ┌───────────────┼───────────────┐
             │               │               │
   ┌─────────▼──────┐ ┌──────▼───────┐ ┌─────▼──────────┐
   │ Feature module │ │   Overlay    │ │  Nested unit   │
   │  + reducer     │ │   registry   │ │ (worktree_form)│
   │  module        │ │              │ │ own Message    │
   └─────────┬──────┘ └──────────────┘ └────────────────┘
             │ returns
   ┌─────────▼──────┐
   │ Feature outcome│ ──────► interpreted by root only (FR-022)
   └────────────────┘

   Writes: feature → own data only (FR-020, guard test FR-024a)
   Reads:  feature → any data, for display (FR-003a)
```

## State transitions

No new state machine is introduced. Two existing ones are relocated intact:

- **Overlay lifecycle** — `closed → opening → open → closing (snapshot) → closed`, with reopen
  permitted from `closing`. Moves from the `Overlay`/`ClosingOverlay` pair onto the single floating
  surface type, behavior-identical (FR-011).
- **Worktree creation** — `idle → collecting → resolving conflict → creating → created | failed`,
  currently spread across `WorktreeFormStatus` and `ResolutionState`. Moves into the nested unit
  unchanged.
