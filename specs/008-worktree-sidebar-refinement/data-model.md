# Phase 1 Data Model: Worktree Sidebar Refinement

Types are Rust, in the pure core (`src/`), unless noted. "Derived" = computed from existing
data, never persisted. "Persisted" = serialized into `projects.json`.

---

## New types

### `Tag` (derived) — `src/naming.rs`

```rust
pub enum Tag {
    Type(ConventionalType), // feat, fix, chore, docs, refactor, test, build, ci, perf, style
    Issue(String),          // Jira-style key, upper-cased, e.g. "ABC-123"
    Status(WorktreeStatus), // Missing | Invalid — Valid produces no Status tag
}
```

- **Source**: `parse_tags(dir_name)` for `Type`/`Issue`; `Status` injected from the worktree's
  `WorktreeStatus` at render time (only for `Missing`/`Invalid`).
- **Rules**: at most one `Type`; at most one `Issue` (FR-003, "at most one key" assumption);
  order = `Type`, then `Issue`, then `Status`.
- **Display label**: `Type` → the lowercase token (`as_str()`); `Issue` → the key verbatim;
  `Status` → "missing"/"invalid".
- **Color**: `Type` → its per-type role pair; `Issue` → the issue role pair; `Status` → the
  `error` role pair (see contracts/design-tokens.md).

### `TagFilter` (transient) — `src/app.rs`

```rust
pub enum TagFilter { Type(ConventionalType), HasIssue, Untyped }
```

- **Match**: `Type(t)` matches a worktree whose parsed tags contain `Type(t)`; `HasIssue`
  matches when tags contain any `Issue`; `Untyped` matches when tags contain no `Type`.
- **Combination**: empty set ⇒ all shown; otherwise a worktree shows if it matches ANY active
  filter (logical OR, FR-025).

### `WorktreeRenameDraft` (transient) — `src/app.rs`

```rust
pub struct WorktreeRenameDraft { pub dir_name: String, pub text: String, pub error: Option<RenameError> }
```

- Mirrors the existing `RenameDraft { path, text, error }` used for project rename.
- Reuses `project::validate_rename(&str) -> Result<String, RenameError>` and
  `enum RenameError { Empty, Whitespace }` (no new validation type).

---

## Extended existing types

### `Project` (persisted) — `src/project.rs`

Add:

```rust
pub worktree_names: BTreeMap<String, String>, // key: worktree dir_name → custom display name
```

- Identity of a worktree within a project is its `dir_name` (unique per repo).
- Absent key ⇒ no override ⇒ derive the friendly name via `naming::display_name(dir_name)`.
- Never contains the on-disk folder name as a value semantic; it is a pure display label.

### `StoredProject` (on-disk) — `src/store.rs`

Add (forward-compatible, no `SCHEMA_VERSION` bump):

```rust
#[serde(default)]
pub worktree_display_names: BTreeMap<String, String>,
```

- Mapped 1:1 with `Project::worktree_names` in `from_workspace`/`into_workspace`.
- Old files without the field load as an empty map (serde default); saving re-adds it.

### `Worktree` (derived, unchanged struct) — `src/worktree.rs`

No field changes. Its presentation is computed:
- `display_name` = override from `Project::worktree_names` if present, else
  `naming::display_name(dir_name)`.
- `tags` = `naming::parse_tags(dir_name)` (+ `Status` tag if `status != Valid`).

### `State` (transient) — `src/app.rs`

Add:

```rust
pub sidebar_filters: BTreeSet<TagFilter>,        // active tag filters (transient)
pub worktree_menu_open: Option<String>,          // dir_name of the worktree whose menu is open
pub worktree_rename_draft: Option<WorktreeRenameDraft>,
```

- `worktree_menu_open` as `Option<String>` makes "two menus open" unrepresentable (only one).
- Rename modal is represented by the new `Overlay::RenameWorktree` variant (below), paired with
  `worktree_rename_draft`.

### `Overlay` (transient) — `src/app.rs`

Add variant:

```rust
Overlay::RenameWorktree,
```

- Delete confirmation reuses the modal dialog pattern; represented via an
  `Overlay::ConfirmWorktreeDelete { dir_name }`-style variant (carries the target so the
  confirm text can name the directory, sessions, and branch — FR-019). Only one modal at a time
  (enum invariant preserved).

---

## Relationships & lifecycle

```
Project (1) ──< worktree_names: dir_name → custom name   (persisted override)
   │
   └── repo on disk ──< Worktree (N, derived from git; keyed by dir_name)
                           │  display_name = override ?? naming::display_name(dir_name)
                           │  tags         = naming::parse_tags(dir_name) [+ Status]
                           └──< Session (N; linked by session.worktree_dir == dir_name)
```

**Rename lifecycle** (FR-013..017): right-click → `WorktreeRenameStarted(dir_name)` opens
`Overlay::RenameWorktree` with a draft seeded from the current display name → `…TextChanged` →
`…Confirmed` validates via `validate_rename`, calls `Workspace::set_worktree_name(dir_name, name)`
(mutates `Project::worktree_names` only), boundary persists → `…Cancelled`/empty keeps prior
name. Tags are re-derived from `dir_name` and are unaffected (FR-016).

**Delete lifecycle** (FR-018..023, D9): right-click → `WorktreeDeleteRequested(dir_name)` opens
the confirm modal → on confirm: terminate the worktree's running sessions, git
`worktree_remove(force)` → `worktree_prune` → `branch_delete`, `fs::remove_dir_all`, reducer
drops sessions + clears `active_session` if matched, re-discover worktrees, persist → on cancel:
no change.

**Filter lifecycle** (FR-024..028): toggle chips mutate `sidebar_filters`; `worktree_tree()`
applies the OR predicate; clear empties the set. Add/rename/delete re-run through the same
predicate so the filtered list stays consistent.

---

## Validation rules

| Rule | Source | Enforced in |
|------|--------|-------------|
| Rename name non-empty, trimmed, not all-whitespace | FR-014, reuse | `validate_rename` |
| Override never mutates folder or branch | FR-014, FR-007 | `set_worktree_name` mutates only `worktree_names` |
| Override persists across restart | FR-015 | `StoredProject.worktree_display_names` round-trip |
| At most one Type tag, at most one Issue tag | FR-002, FR-003 | `parse_tags` |
| Non-conforming name ⇒ no Type tag | FR-008 | `parse_tags` returns no `Type`; matched by `Untyped` |
| Every type/issue tag color meets AA (light+dark) | FR-006, SC-007 | `tests/tokens.rs` `pairs()` array |
| Filter OR semantics; empty ⇒ all | FR-025 | filter predicate in `worktree_tree()` |
| Delete removes dir + sessions + branch, only after confirm | FR-018..020 | confirm modal + boundary orchestration (D9) |
| Delete terminates running sessions first | FR-020 (clarified) | boundary kills matching `terminals` before git |
| Delete leaves consistent state on partial failure | FR-023 | idempotent git steps; reducer drops records regardless |
