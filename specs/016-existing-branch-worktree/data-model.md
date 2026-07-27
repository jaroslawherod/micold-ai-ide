# Phase 1 Data Model: Reuse or Overwrite an Existing Branch When Creating a Worktree

Types are grouped by the module that owns them. Everything here is transient — **no new persisted
state** (Constitution Principle IV). Enums are closed so unreachable combinations are
unrepresentable (Principle V).

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
| `CheckedOutAt` | `path: PathBuf` | Names the worktree directory holding it. |
| `CheckedOutInProjectRoot` | — | The repository's own main checkout (FR-021, second case). |

Both are derived from the same `worktree list --porcelain` records (research R1); the split exists
so the UI can phrase the project-root case in the user's language rather than showing them the
repo path as if it were a worktree.

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

---

## Entity mapping to the spec

| Spec entity | Implemented as |
|---|---|
| Existing branch conflict | `BranchSituation` (+ `BlockReason` for the "where" half) |
| Branch candidate | `BranchCandidate` (+ `BranchOrigin`, `blocked_by`) |
| Conflict resolution choice | `CreateMode`, gated by `ResolutionState` |
