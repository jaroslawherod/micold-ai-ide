# Data Model: Start a Session in the Project Root Directory

Extends the model from `specs/005-worktree-session-terminal/data-model.md`
(`Project 1 — * Worktree`, `Worktree 1 — * Session`). This feature adds a second,
sanctioned way for a `Session` to belong to a `Project`, bypassing `Worktree` entirely.

## `SessionLocation` (new)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionLocation {
    /// Hosted by a worktree, identified by its dir_name (identity, unchanged from today).
    Worktree(String),
    /// Hosted directly by the project's own root directory — no worktree (constitution
    /// v1.3.0, Principle III exception). Presented to users as "Default".
    Default,
}
```

- Replaces `Session.worktree_dir: String` (research.md R1).
- No lifecycle/state machine of its own — it is a fixed tag chosen at session-start time
  and never changes for the life of a `Session` (a session cannot move between locations).

## `Session` (modified)

| Field | Type | Change |
|---|---|---|
| `id` | `SessionId` | unchanged |
| `location` | `SessionLocation` | **was** `worktree_dir: String` |
| `label` | `SessionLabel` | unchanged |
| `lifecycle` | `SessionLifecycle` | unchanged (transient, not persisted) |

Constructors (`src/session.rs`):
- `Session::start_new(location: SessionLocation) -> Self` — was `start_new(worktree_dir: impl Into<String>)`. Callers that used to pass a worktree `dir_name` now pass `SessionLocation::Worktree(dir_name)`; the new Default call site passes `SessionLocation::Default`.
- `Session::restored(id, location: SessionLocation, label) -> Self` — same shape change.

Every existing equality/filter site that compared `s.worktree_dir == dir_name` (e.g.
`app.rs::worktree_tree`, `Workspace::running_session_count`'s per-worktree callers)
becomes a match/comparison against `SessionLocation::Worktree(dir_name)`; sites that need
"is this a Default session" become `matches!(s.location, SessionLocation::Default)`.

## `Project` (unchanged, reused)

`Project.path` — already the project's canonical root path — is now also the value
resolved as a `Default` session's working directory (research.md R2). No new field.

## `Workspace` (unchanged shape, updated semantics)

`Workspace.sessions: BTreeMap<PathBuf, Vec<Session>>` is unchanged: Default sessions live
in the same per-project `Vec<Session>` as worktree sessions, distinguished only by
`location`. `find_session`, `find_session_mut`, `running_session_count` are unaffected in
shape; call sites that filter by worktree `dir_name` gain a parallel filter for
`SessionLocation::Default` where the sidebar needs it (see `SidebarEntry` below).

## `StoredSession` (persistence, `src/store.rs`)

| Field | Type | Change |
|---|---|---|
| `id` | `Uuid` | unchanged |
| `worktree_dir` | `Option<String>` | **was** `String`. `Some(dir)` ⇒ `SessionLocation::Worktree(dir)`; `None` ⇒ `SessionLocation::Default`. |
| `title` | `Option<String>` | unchanged |

Full persistence contract, including backward-compatibility reasoning: `contracts/storage-schema.md`.

## `SidebarEntry` (new, presentation-layer grouping)

The sidebar currently builds one `WorktreeNode` per discovered `Worktree`
(`app.rs::worktree_tree`). This feature adds a second, non-worktree row that is not
sourced from git discovery at all:

```rust
pub enum SidebarEntry {
    Worktree(WorktreeNode),  // unchanged shape
    Default(DefaultNode),    // new
}

pub struct DefaultNode {
    /// Always the literal "Default" — not user-renamable (no rename action exists for it).
    pub display_name: &'static str,
    /// Sessions with SessionLocation::Default for the active project.
    pub sessions: Vec<Session>,
    /// Expansion state, mirroring WorktreeNode.expanded (shares the same expand/collapse UI).
    pub expanded: bool,
}
```

- Always present (once a project is open) and always rendered, regardless of the active
  sidebar tag filters (research.md R4) — `filtered_worktree_tree()`'s equivalent for this
  feature filters only the `Worktree(..)` entries, passing `Default(..)` through
  unconditionally.
- Has no `Tag`s (no type/issue/status) and no worktree-only actions (rename, delete,
  copy-name) — those remain exclusively on `WorktreeNode`/`Worktree` rows.
- Its "start a session" action dispatches the same generalized start message (below) with
  `SessionLocation::Default`, reusing the existing per-row `IconButton`/`Icon::AddSession`
  affordance (research.md R5).

## `Message::SessionStartRequested` (modified, `src/app.rs`)

```rust
SessionStartRequested { location: SessionLocation }  // was { worktree_dir: String }
```

One generalized message for both cases, dispatched from either a worktree row (
`SessionLocation::Worktree(dir_name)`, unchanged trigger) or the Default row
(`SessionLocation::Default`, new trigger) — avoids a parallel `DefaultSessionStartRequested`
message duplicating the same handling logic in `main.rs`.

## Validation rules (from spec Functional Requirements)

- FR-002: starting a `Default` session MUST NOT call any `Git` worktree-mutation method
  (`create_worktree`, `remove_worktree`, or their underlying `git worktree ...` calls) —
  enforced by construction: the Default start path never touches `src/worktree.rs`.
- FR-003: a `Default` session's resolved cwd MUST equal the active project's root path
  exactly (no join/suffix) — testable by asserting the resolved `PathBuf` equals `repo`.
- FR-004/SC-005: two sessions with `SessionLocation::Default` for the same project MUST
  both be independently startable, listed, and closable — same invariant already proven
  for two sessions sharing one `Worktree(dir_name)`.
- FR-008: existing `Worktree`-path tests (`tests/worktree_create.rs`,
  `tests/worktree_delete.rs`, `tests/worktree_rollback.rs`, `tests/worktree_discovery.rs`)
  MUST continue to pass unmodified except for the mechanical `worktree_dir` → `location`
  rename at call sites that construct a `Session`.

## State transitions

None new. `SessionLifecycle` (`Idle` / `Starting` / `Running` / `Restarting` / `Failed`)
is entirely orthogonal to `SessionLocation` and is unaffected by this feature.
