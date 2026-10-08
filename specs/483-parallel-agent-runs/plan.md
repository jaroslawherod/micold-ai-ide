# Implementation Plan: Run One Prompt Across Several Agents and Pick the Best Result

**Branch**: `claude/project-thread-wysm57` | **Date**: 2026-10-08 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification from `specs/483-parallel-agent-runs/spec.md` (issue #483).

## Summary

A **Run in parallel** dialog takes one prompt, a name, a base branch and 2 to 8 runs each with a
provider, and the daemon creates one worktree per run from the base branch on a derived branch
(`feat/login-page-1..N`), starts one session of the run's provider in each and types the prompt as
its first input — each run on its own task, so one failure costs only that run. The group is daemon
state, persisted per project in `runs/<project id>.json` beside feature 482's reviews and pushed to
every window, so the sidebar shows the runs under one collapsible group row and a restart restores
them. A **Compare** view lists the runs with their status and their changed-file and line counts,
read with feature 482's `micold_core::review` git reader, and opens any run's Changes view.
**Pick this one** integrates the winner's branch into the base branch with git's own merge evaluated
without a working tree (`merge-tree --write-tree`, then a fast-forward or a `commit-tree` merge
commit, then a compare-and-swap `update-ref` — or `git merge` in the base's checkout when it has
one), refusing and changing nothing on conflicts or uncommitted changes, and then offers to remove
the losers through the existing worktree Delete.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`), MSRV 1.97

**Primary Dependencies**: no new crate. The user's `git` CLI, 2.38 or newer for
`merge-tree --write-tree` (R1); feature 482's `micold_core::review` for counts and
uncommitted detection (R7, R8) and its `notify`-based watcher shape for the 2 s refresh (R11);
feature 034's `ops::create_session_with_prompt` for the prompt (R4); iced 0.14 as ever.

**Storage**: new `runs/<project id>.json` beside `reviews/` and `projects/`, written by the daemon
through new `ProjectStore::{load_runs, save_runs, remove_runs}` (R2). No new setting.

**Testing**: `mise run test-core` (core unit tests + real-git integration tests in
`crates/micold-core/tests/`), `mise run gate` (daemon integration tests with the fake CLI, client
state tests, in-crate geometry gates), quickstart §B visual pass

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-daemon`,
`micold-client`)

**Performance Goals**: a group of 8 runs starts from one dialog with no frame over 100 ms; Compare's
counts refresh within 2 s of a change in a run's worktree (FR-010)

**Constraints**: offline, local only, nothing pushed (Principle IV); the prompt never logged; a
refused pick leaves the base branch, every run branch and every worktree byte-for-byte unchanged
(SC-006); 2 to 8 runs per group (R6)

**Scale/Scope**: one new core module (5 files), one daemon module, protocol v36, one client feature +
shell watcher + three surfaces, no new shared component, one user-guide page

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | How |
|---|---|---|
| I. Test-First | PASS | Every decision is in tested code: name derivation, group invariants and status transitions, the interrupted-run rule, the merge-tree conflict parser, the integration plan (ff vs merge commit vs refusal), summary arithmetic and the dialog and cleanup validation rules in core and client unit tests; git writes against real temp repos; create/pick/dismiss, one-pick-per-group and isolation in daemon integration tests; sidebar tree and Compare reducers in client state tests; row and dialog layout in geometry gates. Only `ui/parallel_dialog.rs`, `ui/compare.rs`, the sidebar group row's rendering and `shell/runs*.rs`'s I/O wrappers are glue (quickstart §B); every decision they would make is a tested pure function they call (`runs::naming::derive_group`, `ParallelDialog::validate`, `CleanupOffer::removable`, `runs::summary::totals`, `review::watch`). Red first per the TDD loop. |
| II. Multi-Session | PASS | Each run's session is an ordinary session of its own worktree (FR-021), independently addressable, persisted and restorable; the group record holds only each run's session id. The prompt is typed into that run's session alone, through the path 482 W7 hardened; nothing of one run reaches another except the repository history after a pick (Edge Session isolation). |
| III. Worktree Integration | PASS | Every run's worktree is created and removed by the app through `ops::create_worktree` / `ops::delete_worktree` — same validation, naming, provenance and rollback as the New worktree dialog; the user runs no git step. The pick writes only a branch ref or merges in an existing checkout, creates no worktree of its own and never leaves one behind (R1, I6). No session runs anywhere but a worktree or the project root. |
| IV. Local-First | PASS | Local git only: no fetch, no push, no remote in the pick (I6). The group lives in a local JSON file; the prompt is typed into a local AI CLI session and never logged (W2). Fully functional offline. |
| V. Rust + iced | PASS | iced only; no new GUI crate. Types make invalid states unrepresentable: `RunStatus` with terminal `Failed`/`Picked` and a closed transition set, `RunStep`, `PickRefusal` (every variant a no-change outcome), `Integration`, `GroupId`, the run count bounded by validated constructors. |
| VI. Cross-Platform | PASS | Names derived through `naming::derive`, already valid as a git ref and a directory name on all three platforms (R5); git invoked through the existing `no_window` wrapper (Windows); the Compare watcher is `notify`'s recommended watcher per OS behind one subscription (R11); loser removal reuses the delete that already handles Windows' locked directories; CI on all three; quickstart §C. |
| VII. Documentation | PASS | New `docs/user-guide/parallel-runs.md` (the dialog, the group in the sidebar, Compare, Pick this one and the cleanup offer — FR-022), linked from `docs/SUMMARY.md`, `docs/README.md` and `worktrees-and-sessions.md`; it states which base Compare compares with (R7) and the git 2.38 requirement. Same PR as the UI. |
| VIII. Components | PASS | No new shared primitive: the dialog, group row and Compare rows are composed from existing `ui/material` builders (`Modal` + `dialog::body`, `TextArea`, `TextField`, `Select`, `Checkbox`, `Button`, `IconButton`, `Tag`, `TreeView`/`TreeItem`, `Snackbar`) — research R12 records why a bespoke run row is not wanted. Showcase poses for the three surfaces keep them under the existing showcase gates. |

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Core `runs` module (render-free) — R1, R5, R6, R7, R8, R10

`crates/micold-core/src/runs/` (new; `pub mod runs;` in `lib.rs`):

- `mod.rs` — `GroupId`, `RunGroup`, `Run`, `RunStatus`, `RunStep`, `RunSummary`, the constants
  `MIN_RUNS`/`DEFAULT_RUNS`/`MAX_RUNS`, and the invariants of [data-model.md](./data-model.md)
  (`RunGroup::validate`, `RunStatus::can_become`, `RunGroup::interrupted_on_load`).
- `naming.rs` — `derive_group(&WorktreeNaming, count: u8) -> Result<Vec<DerivedNames>, NamingError>`
  over the existing `naming::derive` (R5).
- `summary.rs` — `totals(&ChangeList) -> RunSummary` and `has_uncommitted(&ChangeList) -> bool` over
  feature 482's `review::changes::ChangedFile` (R7, R8).
- `integrate.rs` — the pure half of [contracts/integration.md](./contracts/integration.md):
  `parse_merge_tree_conflicts`, `plan(base_tip, run_tip, is_ancestor, checkout) -> IntegrationPlan`,
  `PickRefusal`, `Integration`, and the merge-commit message builder.
- `store.rs` — `RunsFile` (de)serialisation and `JsonFileStore::runs_path`, beside
  `reviews_path` in `crates/micold-core/src/store.rs`; the `ProjectStore` trait gains
  `load_runs` / `save_runs` / `remove_runs` with persist-nothing defaults, exactly as 482's review
  methods.

The I/O half lands in `crates/micold-core/src/git.rs`: the `Git` trait gains
`merge_tree_write_tree`, `commit_tree_merge`, `update_ref_cas` and `merge_in_checkout` (and
`merge_abort`), each a thin `run_git` call through `no_window`/`local_only`, with the `FakeGit`
arms the existing tests' fake needs. Branch tips and ancestry reuse feature 040's existing
`Git::branch_tip` (local `refs/heads/` only) and `Git::is_ancestor` (commit ids): they already
answer what the pick needs, so no new methods for them.

### D2 — Protocol v36 — contracts/run-group-wire.md

`crates/micold-core/src/protocol/messages.rs`: `ClientMsg::{RunGroupCreate, RunGroupPick,
RunGroupDismiss}`, `DaemonMsg::RunGroupsChanged`, `OperationResult::{RunGroupCreated, RunPicked}`;
`version.rs` 35 → 36. No settings change.

### D3 — Daemon — contracts/run-group-wire.md W1–W5, R2, R3, R9, R10

- `crates/micold-daemon/src/runs.rs` (new): `Runs` state (per project, the group list), load on
  first use with the interrupted-run reconcile (R10), `create`, `set_run_status`, `pick`,
  `dismiss`, `forget_worktree`, `forget_project`; writes through the catalog before memory changes
  and broadcasts `RunGroupsChanged`.
- Per-run tasks (`runs::spawn_run`) calling `ops::create_worktree` then
  `ops::create_session_with_prompt` with `FirstPrompt { require_bracketed: true }` (R3, R4), each
  recording its status transitions.
- `crates/micold-daemon/src/ops.rs`: `pick_run` (the gated integration of
  [integration.md](./contracts/integration.md)) beside the existing worktree operations, and
  `delete_worktree` gains the `runs::forget_worktree` call next to its `forget_review_worktree`.
- `crates/micold-daemon/src/server.rs`: route the three messages; `catalog.rs`: the runs-file write
  (temp + rename) and `RunGroupsChanged` once per connection after `Attached`.
- `crates/micold-daemon/src/state.rs`: `base_branch_is_checked_out` from the discovery cache, and
  `forget_run_worktree` as the thin state-side wrapper the review one already models.

### D4 — Client render-free state — data-model "Client"

`crates/micold-client/src/features/runs.rs` (new): `State`, `Msg`, `Effect` (`ReadSummary { seq,
run }`, `Send(ClientMsg)`), reducers for the dialog (open, every field, add and remove a run,
validate, confirm), the group list from `RunGroupsChanged`, Compare (open, per-run summary loads with
stale-seq dropping, refresh from the watcher, pick, the still-working confirmation) and the cleanup
offer (selection, the branch choice, the second confirmation, the per-loser `WorktreeDelete`).
Wired in `app.rs` as `Message::Runs`. `features/sidebar.rs` gains `SidebarEntry::Group(GroupNode)`,
its runs as children, and the group row in `row_heights`, `scroll_target` and the menus.

### D5 — Client shell — R11

`crates/micold-client/src/shell/runs.rs` (new) runs `Effect::ReadSummary` in `spawn_blocking` via
`review::git` + `runs::summary`. `shell/runs_watch.rs` (new): one `Subscription` per run root while
Compare is open, reusing `review::watch::{relevant_paths, Debouncer}` and
`notify::RecommendedWatcher`, emitting `Msg::RunChanged { run }`. Registered in
`shell/subscriptions.rs` only while Compare is open.

### D6 — Glue and docs

`crates/micold-client/src/ui/parallel_dialog.rs` and `ui/compare.rs` (new) compose the surfaces of
[contracts/parallel-surfaces.md](./contracts/parallel-surfaces.md) from existing components;
`ui/sidebar.rs` renders the group row and its menu; `ui/mod.rs` shows Compare in place of the
terminal pane when open; `overlay/registry.rs` registers the dialog, the cleanup offer and its second
confirmation. Showcase poses in `showcase/sections/` + `catalogue.rs`. User guide per row VII.

## Requirement map

| FR | Where | Research / contract |
|---|---|---|
| FR-001 | D4 dialog state, D6 `ui/parallel_dialog.rs`, sidebar menu | parallel-surfaces D1 |
| FR-002 | D4 `ParallelDialog::validate`, `offered_providers` | R6; run-group-wire W1, parallel-surfaces D |
| FR-003 | D1 `naming::derive_group`, D6 D3 | R5; D3 |
| FR-004 | D3 per-run tasks, `create_session_with_prompt` | R3, R4; W1, W2 |
| FR-005 | D3 one task per run, `set_run_status` | R3; W2 |
| FR-006 | `ops::create_worktree` rollback, D1 `RunStep::Worktree` | R3; W2 |
| FR-007 | D4 `SidebarEntry::Group`, D6 group row | R12; G1, G2 |
| FR-008 | D1 `store.rs`, D3 persistence + broadcast | R2; W5, G6 |
| FR-009 | D4 Compare state, D1 `summary.rs`, D6 `ui/compare.rs` | R7; C1–C3 |
| FR-010 | D1 `summary.rs` (one reader), D5 watcher | R7, R11; C5 |
| FR-011 | D4 Compare → `Outcome::ChangesRequested` | 482 V1; C4 |
| FR-012 | D1 `integrate.rs`, D3 `ops::pick_run` | R1, R8; I2–I5, W3 |
| FR-013 | D1 `PickRefusal`, D3 refusal order | R1; I2, I5, W3 |
| FR-014 | D4 `CleanupOffer`, D6 the modal | R13; K1, K2, K4 |
| FR-015 | D4 `CleanupOffer::removable` + second confirmation | R8, R13; K3, K5 |
| FR-016 | D4 per-loser `WorktreeDelete { stop_sessions: true }` | R13; K4, K6 |
| FR-017 | D1 run status rules, D3 W3 steps 3–4, D4 still-working confirmation | R9; C6, C7 |
| FR-018 | D3 `pick` under the project gate, `winner` | R9; W3 |
| FR-019 | D1 `interrupted_on_load`, D3 reconcile | R10; W5 |
| FR-020 | D3 `dismiss` | R2; W4, G5 |
| FR-021 | D3 reuse of `ops`, no run-specific session behaviour | R4; data-model |
| FR-022 | D6 user guide | — |

## Test strategy by layer

| Layer | Where | What it proves | FR / SC |
|---|---|---|---|
| Core unit | `runs/naming.rs` | `derive_group` numbers 1..N, keeps the ticket boundary, rejects an empty or unslugifiable name, two names differing only in case derive the same names, every derived branch passes `check-ref-format`'s rules and is a valid directory name | FR-003, Edge Name collisions, Cross-platform |
| Core unit | `runs/mod.rs` | `RunGroup::validate` (count bounds, numbering, winner names a run); every allowed and every forbidden `RunStatus` transition; `interrupted_on_load` marks `Creating`/`Starting` and nothing else | FR-002, FR-005, FR-019 |
| Core unit | `runs/summary.rs` | totals over committed, uncommitted and both; a binary row counts as a file with no lines; an empty list is `0/+0/−0`; `has_uncommitted` on untracked-not-ignored, staged and unstaged rows | FR-009, FR-010, Edge Run with no changes / binary |
| Core unit | `runs/integrate.rs` | `parse_merge_tree_conflicts` on git's real output (captured), including paths with spaces and UTF-8; `plan` gives FastForward when the base tip is an ancestor, MergeCommit otherwise, a refusal for each `PickRefusal` input, and the merge-commit message bytes | FR-012, FR-013, SC-006 |
| Core unit | `runs/store.rs` | JSON round-trip of a group with every status; missing file, corrupt file kept aside; unknown fields ignored | FR-008 |
| Core integration (real git) | `crates/micold-core/tests/runs_integrate.rs` (new) | temp repo: fast-forward moves the base and no run branch; a diverged base gets a merge commit with two parents; a conflicting pick leaves every ref and file byte-identical; the CAS update fails when the base moved; a checked-out base merges in its checkout; a dirty blocking checkout refuses and leaves it as it was; an old git is reported as such | FR-012, FR-013, SC-006 |
| Core integration (real git) | `crates/micold-core/tests/runs_summary.rs` (new) | counts for a run against a non-default base branch; the same run's counts equal `review::change_list` totals with both toggles on when the base is the default branch | FR-009, FR-010, SC-004 |
| Daemon integration | `crates/micold-daemon/tests/run_group_create.rs` (new, fake CLI as `mcp_create_session.rs`) | W1 refusals; N worktrees on derived branches from the base commit; one session per run of its provider, each receiving the prompt exactly once; a failing run leaves the others complete and no branch or folder of its own; an unavailable provider fails only its run; the prompt never appears in a log line | FR-004, FR-005, FR-006, SC-002 |
| Daemon integration | `crates/micold-daemon/tests/run_group_pick.rs` (new) | W3's refusal order; two concurrent picks integrate exactly one; a pick refuses while a run is still being created; the winner is recorded and pushed; a loser delete removes its run from the group and an emptied group disappears; dismiss keeps worktrees, branches and sessions; a restart marks interrupted runs and removes their half-created pair | FR-012, FR-015, FR-017–020, Isolation & lifecycle gate |
| Daemon integration | `crates/micold-daemon/tests/run_group_persist.rs` (new) | the runs file is written before the push and atomically; a failed write refuses the change and keeps the groups; forgetting a project removes the file; every window gets the same list | FR-008 |
| Client state | `crates/micold-client/tests/features_runs.rs` (new) | dialog validation rules one by one; add/remove bounded at 2 and 8; the derived names shown; Compare's stale summary answers dropped and coalesced; the pick gate while runs start; the still-working confirmation; `CleanupOffer::removable` (an uncommitted loser never enters the delete set without selection *and* confirmation); dismissing the offer removes nothing | FR-001–003, FR-009, FR-014–017, SC-005 |
| Client state | `crates/micold-client/tests/features_sidebar.rs` | the group entry holds its runs, a group with no runs disappears, runs of a group are not also listed ungrouped, collapsed counts, row heights and scroll targets with a group present | FR-007, US2 s1, s2, s4 |
| Geometry gates | `ui/material` call-site gates + `tests/gates` | the dialog's and Compare's anatomy, the group row's indentation and tag placement, no new bespoke widget (`material_builder_api.rs`, `composite_call_sites.rs`) | Principle VIII |
| Visual pass | quickstart §B (visual-pass skill) | B1–B23 | all US, SC-001, SC-004, SC-005 |
| Platforms | CI on three OSes; quickstart §C | names, the merge message bytes, the watcher, Windows deletes | Principle VI |
| Docs | `mise run gate` docs checks; user-guide gate | `docs/user-guide/parallel-runs.md` | FR-022 |

## Project Structure

### Documentation (this feature)

```text
specs/483-parallel-agent-runs/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── run-group-wire.md
│   ├── integration.md
│   └── parallel-surfaces.md
└── tasks.md             # next: tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/
├── runs/{mod,naming,summary,integrate,store}.rs      # new
├── git.rs                                           # merge-tree, commit-tree, update-ref CAS, merge in checkout
├── store.rs                                         # runs_path, load/save/remove_runs
└── protocol/{messages,version}.rs                   # v36 messages
crates/micold-core/tests/{runs_integrate,runs_summary}.rs   # new
crates/micold-daemon/src/
├── runs.rs                                          # new
├── ops.rs                                           # pick_run; delete hook
├── state.rs, server.rs, catalog.rs
crates/micold-daemon/tests/{run_group_create,run_group_pick,run_group_persist}.rs  # new
crates/micold-client/src/
├── features/{runs.rs (new), sidebar.rs, mod.rs}
├── shell/{runs.rs, runs_watch.rs (new), subscriptions.rs}
├── ui/{parallel_dialog.rs, compare.rs}              # new glue
├── ui/{mod.rs, sidebar.rs}
├── overlay/registry.rs
├── showcase/sections/, showcase/catalogue.rs
└── app.rs
crates/micold-client/tests/features_runs.rs          # new
docs/user-guide/parallel-runs.md                     # new; SUMMARY.md, README.md, worktrees-and-sessions.md
```

**Structure Decision**: existing workspace layout. Logic in `micold-core::runs` (with the git calls
in `micold-core::git`), authority over groups and the pick in the daemon, view state in the client's
render-free `features/`, I/O in `shell/`, composition in `ui/` from existing `ui/material`
components.

## Complexity Tracking

None: no principle is violated. No dependency is added; the one new external requirement is git
2.38 for `merge-tree --write-tree`, justified in research R1 with the alternatives it replaces.
