# Research: Start a Session in the Project Root Directory

No `NEEDS CLARIFICATION` markers remain in the Technical Context (see plan.md) — this
feature extends an existing, well-understood domain model (`Session`, `Workspace`,
`store.rs`, the sidebar tree) rather than introducing new technology. This document
records the concrete design decisions needed to turn the spec into an unambiguous plan.

## R1: How is a session's location represented in the type system?

**Decision**: Replace `Session.worktree_dir: String` with a new closed enum:

```rust
pub enum SessionLocation {
    Worktree(String), // the hosting worktree's dir_name, same identity as today
    Default,          // the project root — no worktree
}
```

`Session.worktree_dir: String` becomes `Session.location: SessionLocation`.

**Rationale**: This codebase already uses closed enums specifically to make invalid or
ambiguous domain states unrepresentable (`WorktreeStatus`, `SessionLifecycle`,
`RestartDecision`, `CleanupStep` — all in `src/worktree.rs`/`src/session.rs`), per
Constitution Principle V. A `SessionLocation` variant reads unambiguously at every call
site (`main.rs` cwd resolution, `app.rs` sidebar-tree filtering, `store.rs`
persistence) and cannot be accidentally confused with a real worktree `dir_name` string.

**Alternatives considered**:
- `worktree_dir: Option<String>` (`None` = Default) — rejected as the primary in-memory
  representation: weaker signal at call sites (`if let Some(dir) = ...` reads as "maybe
  missing data", not "this session deliberately has no worktree"), and doesn't name the
  Default case the way the constitution's amended Principle III does. (It IS the right
  shape for the *on-disk* schema — see R4.)
- A `is_default: bool` flag alongside the existing `worktree_dir: String` — rejected: it
  allows the invalid combination `is_default: true` with a non-empty `worktree_dir`,
  exactly what Principle V's "make invalid states unrepresentable" rule exists to avoid.

## R2: How does a Default session resolve its working directory?

**Decision**: A `SessionLocation::Default` session's cwd is the project's own root path
— the same `repo`/`Project.path` value every session-start and session-reopen call site
in `main.rs` already has in scope. No new path derivation, discovery, or
directory-creation step is needed (the project root is guaranteed to already exist
whenever a project is open — spec Assumptions).

**Rationale**: Directly satisfies FR-003 with the minimum change: every one of the
**five** existing `repo.join(".claude/worktrees").join(&session.worktree_dir)` call
sites becomes a match on `session.location`, using `repo` unchanged for the `Default`
arm. All five, by function, MUST be updated — this list is authoritative (tasks.md T009
enumerates it identically, to prevent any one being missed):

1. `session_has_conversation` (`main.rs:316-318`) — the BUG-001 empty-session pruning
   check run on every load; missing this silently prunes a Default session's persisted
   record on every restart (breaks FR-009).
2. The `Message::SessionStartRequested` handler (`main.rs:530-531`) — new session start.
3. `sync_session_titles` (`main.rs:901-903`) — the title-sync poll; missing this means a
   Default session's label never updates from its placeholder (breaks FR-005 parity).
4. `session_cwd` (`main.rs:961`) — reopen/resume within the active project.
5. `session_cwd_any` (`main.rs:971-972`) — reopen/resume across any project, backing the
   background-restart crash-loop guard (feature 008, BS-6).

**Alternatives considered**: Materializing a synthetic "root worktree" directory entry
(e.g., a `.claude/worktrees/__default__` symlink or copy) so every session still
literally maps to a path under `.claude/worktrees/` — rejected: the amended Principle III
and FR-002/FR-006 are explicit that a Default session must not create, resemble, or be
presented as a worktree; a synthetic worktree-shaped directory would violate that and add
filesystem state with no behavioral benefit.

## R3: How is a Default session's on-disk record persisted, without breaking existing data?

**Decision**: Widen `StoredSession.worktree_dir` (`src/store.rs`) from `String` to
`Option<String>`. `Some(dir_name)` ⇒ `SessionLocation::Worktree(dir_name)`;
`None`/absent ⇒ `SessionLocation::Default`. See `contracts/storage-schema.md` for the
full persistence contract.

**Rationale**: `serde_json` deserializes an existing plain JSON string value directly
into `Some(String)` when the target type is `Option<String>`, so every already-persisted
session (`"worktree_dir": "feat-foo"`) loads unchanged with zero migration and no
`schema_version` bump — consistent with the "adding-optionality is forward-compatible"
policy already established in `specs/005-worktree-session-terminal/contracts/storage-schema.md`.
New Default sessions simply serialize `"worktree_dir": null`.

**Alternatives considered**: A separate `is_default: bool` field alongside the unchanged
`worktree_dir: String` (written as `""` for Default) — rejected: reintroduces the exact
"invalid combination is representable" problem from R1, now at the persistence boundary
(what does `is_default: true, worktree_dir: "feat-foo"` mean on a hand-edited or
corrupted file?). A magic sentinel string for `worktree_dir` (e.g. `"__default__"`) —
rejected: worktree `dir_name`s are derived by `naming.rs` and never need escaping today,
but a sentinel is stringly-typed and could theoretically collide; `Option<String>` is
equally simple and fully unambiguous.

## R4: Does the sidebar's tag-filter panel (feature 009) apply to the Default entry?

**Decision**: No. The Default entry is always shown in the sidebar regardless of the
active tag filters, and is never itself offered as a filterable/filter-target row.

**Rationale**: Tag filters (`type`, `issue`, `status`, `untyped`) are derived from a
worktree's branch-naming convention (`naming.rs`) and its git-discovered `WorktreeStatus`
— concepts that don't exist for the project root. Applying "untyped" to the Default entry
would misrepresent it as an atypical worktree, which the constitution amendment and
FR-006 explicitly say it is not.

**Alternatives considered**: Giving the Default entry a synthetic "untyped" tag so it
participates in existing filter logic uniformly — rejected as exactly the
misrepresentation FR-006 rules out.

## R5: How is the Default entry presented, reusing existing shared components (Principle VIII)?

**Decision**: Render the Default entry with the same `TreeView`/tree-item shape already
used for each worktree row (`src/ui/material/tree_view.rs`, consumed from
`src/ui/sidebar.rs`), with a distinct icon (one new closed `Icon` variant,
`src/icons.rs`) instead of the git/branch iconography used for worktrees. Its hover
location tooltip (FR-010) reuses the existing `Tooltip::new(content, label, roles)`
builder (`src/ui/material/mod.rs`), the same primitive already wrapping the sidebar
filter trigger — no new tooltip widget. Its "start a session" affordance reuses the
existing per-row `IconButton`/`Icon::AddSession` action already used on worktree rows.

**Rationale**: Principle VIII requires reuse over forking one-off widgets; every visual
element the Default entry needs (row shape, tooltip, icon button) already exists as a
shared primitive — the only genuinely new surface is the icon glyph itself, which is
already how new concepts are added to this codebase's closed `Icon` enum (most recently
`Icon::Filter` for feature 009).

**Alternatives considered**: A bespoke "pinned header row" component separate from
`TreeView` — rejected: it would duplicate row layout, hover, and tooltip logic that
`TreeView` rows already provide, for no behavioral difference the spec requires.

## R6: What is the location-tooltip content for a worktree row vs. the Default row (FR-010)?

**Decision**: For a worktree, the tooltip shows the worktree's directory path relative
to the project root, computed via `Path::strip_prefix(project_root)` — safe because
every worktree always lives at `.claude/worktrees/<dir_name>` directly under the project
root, so no general-purpose relative-path algorithm is needed. For the Default entry, the
tooltip states that it is the project's own root directory (exact copy is a UI-wording
detail for tasks.md, not a planning blocker — e.g. "Project root").

**Rationale**: Keeps the computation trivial and grounded in an invariant the codebase
already relies on (`worktree.rs`'s `reconcile` already assumes every worktree's parent is
`worktrees_root`), rather than pulling in a path-diffing dependency for a case that never
needs one.

**Alternatives considered**: A general-purpose relative-path diff (e.g. `pathdiff` crate)
— rejected as an unjustified new dependency (Principle: "every dependency MUST be vetted
... prefer minimal, well-maintained crates") for a relationship that is always a direct
child path.
