# Phase 1 Data Model: Reuse or Overwrite an Existing Branch When Creating a Worktree

Types are grouped by the module that owns them. Everything here is transient — **no new persisted
state** (Constitution Principle IV). Enums are closed so unreachable combinations are
unrepresentable (Principle V).

> **Bugfix**: 2026-08-14 — [BUG-002](./bugs/BUG-002.md) Updated from bugfix patch. The "no new
> persisted state" claim above holds for everything written before this line; inclusion (FR-030) is
> the one exception, and it is a per-project list of paths in the existing project-state file. See
> *Included worktrees* at the end of the `src/worktree.rs` section.

---

## `src/naming.rs`

### `dir_name_from_branch(branch: &str) -> String` (new, pure)

The inverse of `derive()`'s branch→directory mapping (research R10).

| Rule | Detail |
|---|---|
| Split | on `/` |
| Per segment | `slugify()` (existing) |
| Empty segments | dropped |
| Join | `-` |

**Validation rules**: output is `[a-z0-9-]`, never starts/ends with `-`, never a Windows reserved
device name (inherited from `slugify`). An all-punctuation branch name yields `""` — callers treat
empty as "cannot derive a directory" and reject the candidate.

**Examples**: `feat/abc-123-login` → `feat-abc-123-login`; `main` → `main`;
`feature/JIRA-9/Fix Thing` → `feature-jira-9-fix-thing`.

---

## `src/worktree.rs`

### `BranchOrigin` (new)

Where a candidate branch lives.

| Variant | Fields | Meaning |
|---|---|---|
| `Local` | — | `refs/heads/<name>` exists. |
| `Remote` | `remote: String` | Only `refs/remotes/<remote>/<name>` exists. |

A name present both locally and remotely collapses to `Local` (FR-019).

### `BranchCandidate` (new)

One row of the picker, and the unit the listing parser produces.

| Field | Type | Notes |
|---|---|---|
| `name` | `String` | Short branch name (no `refs/…` prefix). |
| `origin` | `BranchOrigin` | Local, or the named remote. |
| `blocked_by` | `Option<BlockReason>` | `Some` ⇒ shown but not creatable (FR-012). |

**Ordering** (deterministic, for stable rendering and assertions): available before blocked; then
`Local` before `Remote`; then by `remote` name; then by branch `name`. All comparisons ASCII.

### `BlockReason` (new)

Why a branch cannot back a new worktree.

| Variant | Fields | Message shape |
|---|---|---|
| `CheckedOutAt` | `path: PathBuf`, `owner: WorktreeOwner` | A worktree **this app manages** — directly under `.claude/worktrees/`, so the sidebar can show it. Named by folder name. `owner` is `Agent` for an assistant-created worktree, which the sidebar hides by default: the message then also says how to reveal it (FR-021b). |
| `CheckedOutOutsideApp` | `path: PathBuf` | A worktree git knows about that this app does **not** manage — anywhere outside `.claude/worktrees/`. Named by **full path**, and said to be outside the app (FR-021a). |
| `CheckedOutInProjectRoot` | — | The repository's own main checkout (FR-021, second case). |

All three are derived from the same `worktree list --porcelain` records (research R1, R1a). The
split exists so each holder is described in terms the user can act on: the project-root case in the
user's language rather than as a repo path; the unmanaged case by a location they can navigate to,
rather than a folder name that reads exactly like a sidebar row and is not one (BUG-001).

**Which variant applies** is the same test `reconcile()` uses to decide what the sidebar shows —
`record.path.parent() == Some(repo/.claude/worktrees)` — so "named as one of your worktrees" and
"appears in your worktree list" cannot disagree. `owner` is derived from the folder and branch
names by the same rule as `Worktree::owner()` (feature 014, FR-005), shared rather than duplicated.

> `path` is always **absolute**. It is the one thing that makes an unmanaged holder findable, and a
> repo-relative rendering would be useless for the holder that sits outside the repository
> altogether — the sibling-directory case in BUG-001.

### `BranchSituation` (new)

The classified result of pre-flight for one derived/selected branch name. **The only producer of
`CreateMode`.**

| Variant | Fields | Offered actions |
|---|---|---|
| `Free` | — | Create as today — no prompt (FR-025). |
| `LocalAvailable` | `branch: String` | Reuse, Overwrite, Cancel (FR-002). |
| `RemoteOnly` | `branch: String`, `remotes: Vec<String>` | Continue from each remote, Start fresh at HEAD, Cancel (FR-016/018). |
| `Blocked` | `branch: String`, `reason: BlockReason` | None — explain only (FR-021). |
| `DirectoryTaken` | `dir: PathBuf` | None — explain only; takes precedence over every branch case (FR-022). |

**Precedence** (checked in this order, first match wins): `DirectoryTaken` → `Blocked` →
`LocalAvailable` → `RemoteOnly` → `Free`. Directory first because no branch choice can resolve a
directory clash; blocked before available because a checked-out branch is neither reusable nor
overwritable.

### `CreateMode` (new)

The user's resolved decision, handed back into creation.

| Variant | Fields | Git command (research R2–R4) | Compatible situation |
|---|---|---|---|
| `NewBranch` | — | `worktree add -b <branch> <path> HEAD` | `Free`, `RemoteOnly` (the "start fresh" answer) |
| `ReuseLocal` | — | `worktree add <path> <branch>` | `LocalAvailable` |
| `Overwrite` | — | `worktree add -B <branch> <path> HEAD` | `LocalAvailable` |
| `TrackRemote` | `remote: String` | `worktree add --track -b <branch> <path> <remote>/<branch>` | `RemoteOnly` whose `remotes` contains `remote` |

`Default` is `NewBranch`, so every existing call site keeps today's behavior.

### `WorktreeForm::mode_for(situation, preferred_remote) -> Option<CreateMode>`

`preferred_remote` is the remote named by the picked branch row. For `RemoteOnly` it returns
`None` when the name is on several remotes and no preference is given — the prompt opens so the
user picks, rather than the app choosing (spec Edge Cases).

### `CreateMode::creates_branch(self) -> bool` (new)

`true` for every mode except `ReuseLocal`. Sole input to the rollback change below.

### `rollback_plan(mode: CreateMode) -> Vec<CleanupStep>` (changed signature)

Same steps in the same order as today —
`WorktreeRemove → WorktreePrune → BranchDelete → RemoveDir` — with `BranchDelete` **omitted when
`!mode.creates_branch()`** (research R5, FR-008). No other behavior changes.

### `preflight(...) -> BranchSituation` (new, pure over `Git`)

```
preflight(git, repo, target_path, branch, target_exists) -> io::Result<BranchSituation>
```

Reads `worktree_list_porcelain` (for directory clash *and* checked-out detection, from the raw
records — not `reconcile`, research R1) and `branch_exists`, plus `list_branch_refs` only when the
local branch is absent. Performs **no mutation**.

### `CreateError` (extended)

| Variant | Status | Meaning |
|---|---|---|
| `DuplicateDir` | kept | Directory clash (FR-022). |
| `DuplicateBranch` | **removed** | Superseded — this situation is now a decision, not an error (FR-001). Its former call site returns `BranchSituation::LocalAvailable`. |
| `BranchInUse { branch, reason }` | new | The blocked case, carrying enough to name the holder (FR-021). |
| `SituationChanged` | new | Re-verification found a different situation than the user answered for (FR-009, research R11). |
| `RolledBack(String)` | kept | Unchanged. |

### `create_worktree(...)` (changed signature)

Gains a `mode: CreateMode` parameter. Its first action is a fresh `preflight()`; incompatibility
with `mode` returns `SituationChanged` **before any mutation**. Then it dispatches to the mode's
git command and runs the existing submodule step unchanged. On failure it runs
`rollback_plan(mode)`.

### `CreateStage::label(mode)` (changed signature)

Stage set unchanged; `CreatingWorktree`'s text becomes mode-dependent (research R12, FR-024).

### Included worktrees (new — BUG-002, FR-027–FR-032)

The **one** persisted addition in this feature (research R13): a per-project set of absolute paths
the app also shows.

| Where | Field | Type | Notes |
|---|---|---|---|
| `StoredProjectState` (`src/store.rs`) | `included_worktrees` | `Vec<PathBuf>` | `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, no `schema_version` bump — a file written before the field loads as empty, which is exactly today's behaviour, and an older build ignores it as an unknown field. The same forward-compatibility rule `last_session` follows. |

Paths are **absolute and canonical**, the same normalization projects themselves use, so a path
recorded once matches the record git reports for it.

**Nothing else is persisted.** Branch, status, and existence are derived at read time from the same
`worktree list --porcelain` records everything else here reads — an included worktree that has been
deleted or unregistered surfaces through the existing `WorktreeStatus`, never as a stale row
(FR-031).

#### `reconcile()` (changed signature)

Gains the included set, and widens its parent test by exactly that set:

```
reconcile(records, worktrees_root, included: &[PathBuf], on_disk_dir_names, exists)
```

| Record | Kept? |
|---|---|
| parent is `worktrees_root` | yes — unchanged |
| `path` is in `included` | **yes — new** |
| anything else | no — unchanged |

One test, widened rather than forked, which is what keeps FR-032 true for free: `checked_out_branches()`
applies the same predicate, so an included holder classifies as `CheckedOutAt` and an excluded one as
`CheckedOutOutsideApp`, with no second rule to drift (BUG-001's invariant, extended).

#### `Worktree` (extended)

| New field | Type | Notes |
|---|---|---|
| `included` | `bool` | `true` for a worktree kept by the included set rather than by its parent directory. The list shows its location for these (FR-029), and deletion names that location before removing anything (FR-033). |

#### `BlockReason::CheckedOutOutsideApp` — offered action

Unchanged in shape. What changes is that this is now the **only** `BlockReason` that carries an
action: the explanation offers "Include that worktree" (FR-027). `CheckedOutInProjectRoot` and
`CheckedOutAt` offer nothing new — the first is the project itself, the second is already included
(FR-033).

---

## `src/app.rs`

### `BranchSource` (new)

Which half of the form is active.

| Variant | Meaning |
|---|---|
| `New` (default) | Type + ticket + name inputs — today's form. |
| `Existing` | Candidate picker (User Story 2). |

Switching to `New` clears the selected candidate (FR-015).

### `ResolutionState` (new) — the conflict state machine (research R9)

| State | Fields | Reachable from | On cancel |
|---|---|---|---|
| `Idle` | — | initial | — |
| `Choosing` | `situation: BranchSituation` | `Idle` after a pre-flight that is not `Free` | → `Idle`, form inputs intact (FR-007) |
| `ConfirmingOverwrite` | `situation: BranchSituation` | `Choosing` | → `Choosing` (FR-005, US2 AS3) |

Transitions: `Choosing --reuse/track/fresh--> (submit with CreateMode)`;
`Choosing --overwrite--> ConfirmingOverwrite`; `ConfirmingOverwrite --confirm--> (submit with
CreateMode::Overwrite)`. A `Blocked` or `DirectoryTaken` situation renders an explanation with no
actionable choice and returns to `Idle` on dismiss (FR-021, US5 AS3).

### `WorktreeForm` (extended)

| New field | Type | Notes |
|---|---|---|
| `source` | `BranchSource` | Default `New`. |
| `candidates` | `Vec<BranchCandidate>` | Populated when `source` becomes `Existing`; empty until then. |
| `selected_branch` | `Option<BranchCandidate>` | The picked candidate. |
| `resolution` | `ResolutionState` | Default `Idle`. |

`WorktreeForm::preview()` gains a `source` branch: under `Existing` it derives the directory from
`selected_branch` via `dir_name_from_branch()` (FR-014) instead of from type/ticket/name.

**Invariant**: `status == Creating` ⇒ `resolution == Idle`. A create in flight and an unanswered
prompt cannot coexist.

### New `Message` variants

`AddWorktreeSourceChanged(BranchSource)`, `AddWorktreeBranchSelected(BranchCandidate)`,
`AddWorktreeBranchesListed(Vec<BranchCandidate>)`, `AddWorktreeConflictDetected(BranchSituation)`,
`AddWorktreeResolutionChosen(CreateMode)`, `AddWorktreeOverwriteRequested`,
`AddWorktreeOverwriteConfirmed`, `AddWorktreeResolutionCancelled`.

**Added by BUG-002**: `WorktreeIncludeRequested(PathBuf)` (from the blocked explanation),
`WorktreeIncluded(Worktree)`, `WorktreeExcluded(PathBuf)`. Inclusion does not close the form and does
not resolve the situation — the branch is still checked out where it was, so the block stands; what
changes is that the holder is now somewhere the user can go.

---

## Entity mapping to the spec

| Spec entity | Implemented as |
|---|---|
| Existing branch conflict | `BranchSituation` (+ `BlockReason` for the "where" half) |
| Branch candidate | `BranchCandidate` (+ `BranchOrigin`, `blocked_by`) |
| Conflict resolution choice | `CreateMode`, gated by `ResolutionState` |
| Included worktree (BUG-002) | `StoredProjectState::included_worktrees` + `Worktree::included` |
