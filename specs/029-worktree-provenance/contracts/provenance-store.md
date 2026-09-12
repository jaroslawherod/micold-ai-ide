# Contract: The Provenance Record — Storage, Writes, and the One-Time Migration

Governs where the record lives, who may write it, and the FR-006 backfill. Normative statement of
FR-001, FR-002, FR-003, FR-006–FR-006d, FR-009, FR-010 and FR-011 (research R1, R2, R3, R5, R8).

## 1. Storage

```rust
// micold-core/src/workspace.rs
pub worktree_provenance: BTreeMap<PathBuf, BTreeSet<String>>,  // project → dir_names (durable)
pub provenance_migrated: BTreeSet<PathBuf>,                    // projects backfilled (durable)
pub unreadable_projects: BTreeSet<PathBuf>,                    // this run only (derived)
```

```rust
// micold-core/src/store.rs — StoredProjectState
#[serde(default)] created_worktrees: Vec<String>,   // sorted
#[serde(default)] provenance_migrated: bool,
```

### Invariants

1. **Same home as the display-name overrides.** Both fields live in the *per-project* state file,
   never in `projects.json`. There is no legacy-catalog seed to fall back to, unlike
   `worktree_display_names` — a project with no field has simply not migrated yet, which is
   the FR-006 case and not an error.
2. **Backward and forward compatible.** Both are `#[serde(default)]`; no schema version bump. An
   older build ignores them; a newer build reading an older file sees an unmigrated project.
3. **Deterministic on save.** `created_worktrees` is written sorted (it is a `BTreeSet`), so a save
   that changes nothing produces a byte-identical file.
4. **Disposed of with the project.** `Workspace::forget()` removes the `worktree_provenance` and
   `provenance_migrated` entries alongside `worktree_names` and `included_worktrees`;
   `remove_project_state()` deletes the file (FR-009).
5. **`unreadable_projects` is never persisted and never sent as durable state.** It is
   re-derived on every load from `ProjectStateLoad::Corrupt` (`store.rs:633`).

## 2. Who writes a record

Exactly three writers. Any fourth is a defect.

| Writer | Trigger | Requirement |
|---|---|---|
| Creation | the daemon's `WorktreeCreate` handler, after `create_worktree` returns `Ok` | FR-001 |
| Migration | `plan_backfill()` applied once per project (§4) | FR-006 |
| Claim | the daemon's `WorktreeClaim` handler | FR-021 |

And exactly two removers: `WorktreeDelete` (after git removal succeeds) and `forget_project`
(FR-009).

### Invariants

1. **Every creation route.** FR-001 says *every* route by which the app creates a worktree. There is
   exactly one — `create_worktree()` in `micold-core`, reached from the daemon's `WorktreeCreate`
   arm (`server.rs:903`), for all four `CreateMode`s (`NewBranch`, `ReuseLocal`, `Overwrite`,
   `TrackRemote`). The record is written once, after the `Ok`, covering all four. A test asserts a
   record for each mode.
2. **Not on a rolled-back create.** `create_worktree` rolls back on failure and returns `Err`; no
   record is written for a worktree that does not exist.
3. **Written before the broadcast.** The record is persisted and then `broadcast_catalog()` runs, so
   the snapshot the client receives already carries `user_created: true`. A row that appears and then
   turns into an `agent` row a tick later is the visible form of getting this order wrong.
4. **Immediately visible regardless of persistence (FR-010).** The client's optimistic
   `Outcome::WorktreeCreated` path already adds the row locally; it records provenance locally in the
   same step. A persistence failure in the daemon surfaces as the existing error notification and
   leaves the row listed for the rest of the run.
5. **Idempotent.** Recording a `dir_name` already present is a no-op, not a duplicate.
6. **Nothing on disk (FR-003).** No writer performs a filesystem or git operation to establish
   provenance. The git work in the create path is the creation itself, which would happen anyway.

## 3. The evidence rule

```rust
// micold-core — extracted from the expression `Catalog::snapshot()` already computes
/// The worktrees a project durably knows about: those with a display-name override, plus those
/// with a session bound to them — archived or not (FR-006).
pub fn durably_known_worktrees(
    overrides: Option<&BTreeMap<String, String>>,
    sessions: &[Session],
) -> BTreeSet<String>;
```

### Invariants

1. **One definition.** `Catalog::snapshot()` (`catalog.rs:155-168`) is rewritten to call this rather
   than compute it inline. Two expressions of "what the app durably knows about a worktree" that can
   drift is how a migration comes to hide something it should not have (research R2).
2. **Archived sessions count** (FR-006, "archived or not"). This function reads the unfiltered
   session list; `snapshot()` keeps its own `!s.archived` filter for its own purposes.
3. **Default-location sessions contribute nothing.** Only `SessionLocation::Worktree(dir)` yields a
   `dir_name`; a root session is not evidence about any worktree.

## 4. The migration

```rust
/// The records the FR-006 backfill would write for one project. Pure: returns them, writes nothing.
///
/// Returns `None` when the migration must not run at all — the project has already migrated
/// (FR-006c) or its state was unreadable this run (FR-011). `Some(empty)` is a legitimate result
/// meaning "ran, found no evidence", and still sets the marker.
pub fn plan_backfill(
    discovered: &[Worktree],
    root: &Path,
    evidence: &BTreeSet<String>,
    existing: &BTreeSet<String>,
    already_migrated: bool,
    state_unreadable: bool,
) -> Option<BTreeSet<String>>;
```

A worktree is backfilled iff **all** hold:

1. it is directly under `root` (FR-005 — nothing outside is classified, so nothing outside needs a
   record), **and**
2. its `dir_name` is in `evidence` (§3), **and**
3. `matches_reserved_convention(dir_name, branch)` is `false` — the FR-007a veto, **and**
4. it is not already in `existing` — FR-006b, the migration never overwrites.

### Invariants

1. **Read-only (FR-006a).** `plan_backfill` performs no I/O and asks the user nothing. It takes the
   already-discovered list; it does not trigger discovery.
2. **Idempotent (FR-006b).** Running it twice over the same inputs, applying the first result,
   yields an empty second result. A crash between the backfill and the marker write is therefore
   harmless.
3. **At most once per project (FR-006c).** `already_migrated` ⇒ `None`. The marker is persisted in
   the *same write* as the records, and set even when the result is empty — "ran and found nothing"
   and "never ran" are different states (data-model §3).
4. **Never runs on an unreadable project (FR-011).** `state_unreadable` ⇒ `None`, checked before
   anything else, and no marker is written. The project's one chance at a correct backfill survives
   to the next successful read.
5. **Evidence never applies afterwards (FR-006d).** Because the marker is persisted and checked
   first, a session started later in a revealed assistant worktree — which 014 permits — produces no
   record. Claiming is the only remaining route. A test starts a session in a revealed worktree of a
   migrated project, refreshes, and asserts the worktree is still `Agent`.
6. **The veto binds only here (FR-023).** `matches_reserved_convention` is consulted by this
   function and by nothing else. A claim on a reserved-convention name is honoured.

### Where it runs

`DaemonState::refresh_worktrees()` (`state.rs:799`), immediately after discovery populates
`Inner::worktrees` for the project — the single point where the discovered list, the durable records
and the marker are all in hand, already off the async runtime, and already reached by every path that
can reveal a worktree to a user (research R3). Applying the plan and persisting it is the daemon's
step; deciding it is core's.

## 5. Wire projection

```rust
// WorktreeSnapshot
/// Whether the app holds a provenance record for this worktree (FR-004).
pub user_created: bool,
```

### Invariants

1. **Filled from the durable answer.** `DaemonState::snapshot_locked()` (`state.rs:436`) reads
   `workspace.worktree_provenance[project]`, exactly where it already reads `worktree_names` for
   `display_name`.
2. **The client reconstitutes, it does not decide.** `catalog_sync.rs` rebuilds
   `core.workspace.worktree_provenance[active]` from the snapshot's flags for the active project,
   exactly as it rebuilds `worktree_names` from `display_name` (`catalog_sync.rs:202-212`) — insert
   when non-empty, remove the key when empty.
3. **Boot is classified correctly.** The client loads the store itself
   (`shell/startup.rs:129`), so the map is populated before the daemon connects; the first frame
   hides what it should rather than showing rows that vanish a tick later.
4. **The marker does not travel.** `provenance_migrated` is daemon-side only. The client never runs
   the migration and must not be able to.
