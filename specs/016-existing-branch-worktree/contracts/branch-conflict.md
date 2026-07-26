# Contract: branch-conflict pre-flight, resolution, and rollback

Owns the rule that a branch-name collision is a **decision**, not an error (FR-001), and the rule
that recovery never destroys a branch the operation did not create (FR-008).

---

## 1. Pre-flight classification

```
preflight(git, repo, target_path, branch, target_exists) -> io::Result<BranchSituation>
```

**Pure over the `Git` boundary. Performs no mutation.** Called twice per creation: once to raise
the prompt, once inside `create_worktree` to re-verify (§4).

### Inputs consumed

| Source | Used for |
|---|---|
| `worktree_list_porcelain` → `parse_worktrees()` (**raw records**) | directory clash *and* checked-out detection |
| `branch_exists(repo, branch)` | local branch presence |
| `list_branch_refs` (only when no local branch) | remote-only detection |
| `target_exists` (caller's `fs` fact) | directory clash |

> Use the raw `WorktreeRecord`s, **not** `reconcile()`. `reconcile()` filters to
> `.claude/worktrees/` children and so discards the repository's own main checkout — the exact
> record FR-021's second case needs (research R1).

### Classification order — first match wins

| # | Condition | Result |
|---|---|---|
| 1 | `target_exists`, or a record's `path` equals `target_path` | `DirectoryTaken { dir }` |
| 2 | a record's `branch` equals `branch` | `Blocked { branch, reason }` — `reason` is `CheckedOutInProjectRoot` when that record is the repo root, else `CheckedOutAt { path }` |
| 3 | `branch_exists` is true | `LocalAvailable { branch }` |
| 4 | `refs/remotes/<r>/<branch>` exists for one or more `r` | `RemoteOnly { branch, remotes }` — **all** matching remotes, sorted |
| 5 | otherwise | `Free` |

Rule 1 precedes everything because no branch choice can resolve a directory clash (FR-022).
Rule 2 precedes 3 because a checked-out branch is neither reusable nor overwritable (FR-021).
Rule 3 precedes 4 — local wins over remote (FR-019).
When rule 4 matches on more than one remote, the situation carries **every** matching remote
(sorted), and the prompt offers one "Continue from `<remote>`" action per remote. The app must
never choose a remote on the user's behalf (spec Edge Cases) — picking the alphabetically first
would silently decide where a later `push` goes.

---

## 2. Which actions each situation offers

| Situation | Offered | Not offered |
|---|---|---|
| `Free` | proceed silently (`NewBranch`) | — no prompt at all (FR-025) |
| `LocalAvailable` | Reuse, Overwrite, Cancel (FR-002) | — |
| `RemoteOnly` | Continue from each of `remotes`, Start fresh at HEAD, Cancel (FR-016/018) | Overwrite — there is no local branch to destroy |
| `Blocked` | Dismiss only (FR-021) | Reuse, Overwrite |
| `DirectoryTaken` | Dismiss only (FR-022) | every branch action |

The "Start fresh at HEAD" answer for `RemoteOnly` resolves to `CreateMode::NewBranch` and **must**
be presented alongside the statement that the resulting branch will diverge from the remote branch
of the same name (FR-018).

---

## 3. Resolution state machine (`WorktreeForm::resolution`)

```
                 preflight != Free
   Idle ─────────────────────────────► Choosing(situation)
    ▲                                    │        │
    │  cancel (inputs preserved, FR-007)  │        │ overwrite
    └────────────────────────────────────┘        ▼
    ▲                                     ConfirmingOverwrite(situation)
    │  back  ────────────────────────────────────┘
    │
    └──── submit(CreateMode) ──── from Choosing (reuse / track / fresh)
                              └── from ConfirmingOverwrite (confirm)
```

**Invariants**

1. `CreateMode::Overwrite` is reachable **only** via `ConfirmingOverwrite` (FR-005, SC-004).
2. Cancel from `Choosing` restores `Idle` with `type_`, `ticket`, `name`, `source`, and
   `selected_branch` untouched (FR-007, SC-007).
3. Back from `ConfirmingOverwrite` returns to `Choosing`, not `Idle` (US2 AS3).
4. `status == Creating` ⇒ `resolution == Idle`. No prompt during an in-flight create.
5. Reaching `Choosing`/`ConfirmingOverwrite` mutates nothing on disk. Every state in this diagram
   except the terminal submit is side-effect free.

---

## 4. Re-verification before acting (FR-009)

`create_worktree` re-runs `preflight()` as its first step and compares the fresh situation against
the `CreateMode` it was handed:

| `CreateMode` | Compatible situation |
|---|---|
| `NewBranch` | `Free`, or `RemoteOnly` (the deliberate "start fresh" answer) |
| `ReuseLocal` | `LocalAvailable` with the same branch |
| `Overwrite` | `LocalAvailable` with the same branch |
| `TrackRemote { remote }` | `RemoteOnly` with the same branch, where `remote` is among its `remotes` |

Incompatible ⇒ return `CreateError::SituationChanged` **before any mutation**. This is not
atomicity (nothing short of a repo lock would be) — it is a clean abort, with git's own refusal as
the backstop (`git-trait-branches.md`, research R2/R11).

---

## 5. Rollback by mode

`rollback_plan(mode) -> Vec<CleanupStep>` — same steps, same order as today, with one conditional:

```
WorktreeRemove → WorktreePrune → [BranchDelete] → RemoveDir
                                  ^ omitted when !mode.creates_branch()
```

| Mode | `BranchDelete`? | Why |
|---|---|---|
| `NewBranch` | yes | unchanged; the attempt created the branch |
| `ReuseLocal` | **no** | the branch predates the attempt (FR-008, SC-003) |
| `Overwrite` | yes | the branch present at failure is the *new* one; the old tip is already gone |
| `TrackRemote` | yes | the attempt created the local branch; the remote ref is untouched by `branch -D` |

`CleanupStep::RemoveDir` still runs for every mode (the binary's `fs` half).

---

## 6. Error surface

| `CreateError` | User-facing message shape |
|---|---|
| `DuplicateDir` | unchanged from today |
| `BranchInUse { branch, reason }` | names the branch **and** the holder — a worktree directory, or the project's own checkout (SC-006) |
| `SituationChanged` | states that the branch changed since the prompt and nothing was done |
| `RolledBack(msg)` | unchanged; for `Overwrite` the caller additionally states that the previous branch contents were already discarded (US2 AS4) |

`CreateError::DuplicateBranch` is **removed**. Its former call site now returns
`BranchSituation::LocalAvailable`. Removing rather than deprecating it is deliberate: the compiler
then finds every site that treated the collision as terminal.

---

## 7. Test obligations

**`tests/branch_conflict.rs`** (new)

1. Each of the five situations is produced by the matching `FakeGit` fixture.
2. Precedence: directory clash beats a checked-out branch beats a local branch beats a remote one.
3. `Blocked` distinguishes the project-root holder from a `.claude/worktrees/` holder and carries
   the holder's path.
4. Local + remote of the same name ⇒ `LocalAvailable` (FR-019).
5. `preflight` mutates nothing: branch list and worktree list are byte-identical before/after, for
   every situation (SC-007).
6. Every (`CreateMode`, situation) pair resolves to compatible/incompatible per §4's table; every
   incompatible pair yields `SituationChanged` with **no** mutation.

**`tests/worktree_create.rs`** (extended)

7. `ReuseLocal` binds the existing branch and leaves its tip unmoved.
8. `Overwrite` recreates the branch at HEAD and registers the worktree.
9. `TrackRemote` creates the local branch, records the upstream, and leaves the remote ref alone.
10. `Free` + `NewBranch` behaves byte-identically to today (SC-008).

**`tests/worktree_rollback.rs`** (extended)

11. **Reuse rollback preserves the branch**: with `failing_next_add` primed, after a `ReuseLocal`
    attempt the branch still exists. This is SC-003's regression guard — the single most important
    test in the feature.
12. `NewBranch`, `Overwrite`, `TrackRemote` rollbacks still delete the branch they created.
13. `rollback_plan(mode)` returns the documented step sequence for all four modes.

**`tests/app_state.rs`** (extended)

14. The §3 state machine: every transition, plus invariants 1–4.
15. Cancel from either prompt leaves all form inputs untouched.
