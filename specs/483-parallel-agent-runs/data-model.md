# Data Model: Parallel Agent Runs (#483)

Types live in a new render-free core module `micold_core::runs`. Everything here is serialisable and
local-only (Principle IV). Statuses are closed enums so an impossible run state is unrepresentable
(Principle V).

## Core (`crates/micold-core/src/runs/`)

### `GroupId`

`pub struct GroupId(pub Uuid)` — the group's identity on the wire and in the file, as `SessionId` is.
Stable across restarts; a dismissed group's id is never reused.

### `RunGroup`

| Field | Type | Notes |
|---|---|---|
| `id` | `GroupId` | identity |
| `name` | `String` | the user's free-text name (pre-slug), shown on the group row (FR-007) |
| `naming` | `WorktreeNaming` | type, optional ticket, name — what the run names were derived from (R5) |
| `prompt` | `String` | the one prompt sent to every run (FR-004); never logged |
| `base_branch` | `String` | the branch every run started from and the pick integrates into |
| `base_commit` | `String` | `base_branch`'s tip when the group started (Key Entities) |
| `created` | `SystemTime` | creation time, for ordering groups |
| `runs` | `Vec<Run>` | ordered by `number`, length 2..=`MAX_RUNS` |
| `winner` | `Option<u8>` | the picked run's `number`; `None` until a pick succeeds (FR-018) |

Invariants: `runs` is non-empty and its `number`s are `1..=runs.len()`, strictly increasing;
`winner`, when set, names an existing run; `base_branch` is a valid branch name.

### `Run`

| Field | Type | Notes |
|---|---|---|
| `number` | `u8` | 1-based position within the group (FR-003, FR-007) |
| `provider` | `AiCli` | `claude`, `copilot` or `pi` |
| `names` | `DerivedNames` | `dir_name` and `branch` (R5) |
| `session` | `Option<SessionId>` | set once the session record exists |
| `status` | `RunStatus` | below |

### `RunStatus`

```text
Creating                                  worktree being created
Starting                                  session starting
Prompted                                  prompt typed; live status comes from the session
PromptNotDelivered { reason: String }     worktree and session kept (US1 s4, FR-004)
Failed { step: RunStep, reason: String }  FR-005, FR-019
Picked                                    this run won the group (terminal)
```

`RunStep` is `Worktree | Session | Prompt`, so the failed step is reportable without parsing the
reason (FR-005).

Transitions (the only ones): `Creating → Starting | Failed{Worktree}`;
`Starting → Prompted | PromptNotDelivered | Failed{Session}`; `Prompted → Picked`;
`PromptNotDelivered → Picked`. `Failed` and `Picked` are terminal. On load, a persisted `Creating` or
`Starting` becomes `Failed { step, reason: "interrupted" }` (R10).

The statuses FR-009 lists that are *not* stored — working, waiting for input — are derived for
display from the run's session (`SessionSummary::activity`, Assumptions), never persisted.

### `RunSummary` (derived, never persisted)

`files: u32`, `added: u32`, `removed: u32`, `uncommitted: bool`, computed from feature 482's
`ChangeList` (R7, R8). Shown in Compare; recomputed on every refresh.

### Pick types

- `PickRefusal`: `AlreadyPicked | RunUnavailable(String) | Uncommitted { files: Vec<RelPath> } |
  Conflicts { files: Vec<RelPath> } | BaseBusy(String) | BaseMoved | GitTooOld | Git(String)`.
  Every variant changes nothing (FR-012, FR-013, SC-006).
- `Integration`: `FastForward { base_tip: String } | MergeCommit { commit: String }` — what a
  successful pick did, for the user guide's wording and the operation reply.

### Constants

`MIN_RUNS = 2`, `DEFAULT_RUNS = 2`, `MAX_RUNS = 8` (R6).

## Persistence (`runs/<project id>.json`)

Written by the daemon through `ProjectStore::{load_runs, save_runs, remove_runs}`, atomically
(temp + rename), exactly as `reviews/<project id>.json` is (R2). Shape:

```json
{ "version": 1, "groups": [ { "id": "…", "name": "login page", "naming": {…},
  "prompt": "…", "base_branch": "main", "base_commit": "…", "created": "…",
  "runs": [ { "number": 1, "provider": "claude", "dir_name": "feat-login-page-1",
              "branch": "feat/login-page-1", "session": "…", "status": {…} } ],
  "winner": null } ] }
```

A missing file is an empty list; an unparseable one is kept aside as `<name>.corrupt` and an empty
list loads (as `load_reviews` does). Unknown fields are ignored on read.

## Daemon state (`crates/micold-daemon/src/runs.rs`)

`Runs { projects: HashMap<PathBuf, Vec<RunGroup>> }`, read from the project's file on first use,
written before memory changes, then broadcast. Operations: `create` (validate, derive names, record,
spawn the per-run tasks), `set_run_status`, `pick` (under the project's worktree gate),
`dismiss`, `forget_worktree(project, dir_name)` (a deleted worktree's run leaves its group; an empty
group is dropped — FR-007, US2 s4), `forget_project`.

## Client state (`crates/micold-client/src/features/runs.rs`, render-free)

| Field | Purpose |
|---|---|
| `groups: Vec<RunGroup>` | last `RunGroupsChanged` for the active project |
| `dialog: Option<ParallelDialog>` | the Run in parallel form: `naming`, `prompt`, `base_branch`, `runs: Vec<AiCli>`, `error: Option<String>`, and the derived names it shows (FR-003) |
| `compare: Option<CompareView>` | open group id, per-run `Load<RunSummary>`, read sequence numbers (stale answers dropped, as `features/changes.rs` does) |
| `cleanup: Option<CleanupOffer>` | after a successful pick: per loser `selected: bool`, `uncommitted: bool`, `delete_branch: bool`, and `confirming: Option<u8>` for the second confirmation (FR-015) |

`ParallelDialog::validate()` is the pure rule behind FR-002 and Edge "Empty input": a prompt, a type
and a name are required, the run count is `MIN_RUNS..=MAX_RUNS`, every provider must be offered where
sessions run (`State::offered_providers`), and the derived names must all validate.

`CleanupOffer::removable()` is the pure rule behind FR-015: a loser with `uncommitted` is removable
only while it is both selected and confirmed; nothing else can put it in the delete set.
