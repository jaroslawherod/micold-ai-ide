---

description: "Task list for feature 483: run one prompt across several agents in parallel worktrees and pick the best result"
---

# Tasks: Run One Prompt Across Several Agents and Pick the Best Result

**Input**: Design documents from `specs/483-parallel-agent-runs/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/ (run-group-wire,
integration, parallel-surfaces), quickstart.md

**Tests**: Mandatory (Constitution I). Every test task comes before the implementation it covers and
must fail first for the right reason (TDD loop, cycle log in `tdd/cycle-log.md`). The only code
without a test of its own is the glue plan.md's Constitution Check row I names
(`ui/parallel_dialog.rs`, `ui/compare.rs`, the sidebar group row's rendering, and
`shell/runs.rs` / `shell/runs_watch.rs`, which only run `review::git`, `runs::summary` and
`review::watch` — all tested — on the blocking pool or a watcher thread), covered by quickstart §B.

**Documentation**: `docs/user-guide/parallel-runs.md` grows with each story, in the same milestone
as the behaviour it describes (Constitution VII, CI user-guide gate).

**Cross-platform**: names come from `naming::derive` (lowercase, valid on all three platforms, R5),
git runs through the existing `no_window`/`local_only` wrapper, the Compare watcher is notify's
recommended backend per OS (R11); quickstart §C covers macOS and Windows (Constitution VI).

**Wire**: every wire-visible change of this feature is made in **one** edit (T009), so
`PROTOCOL_VERSION` moves 35 → 36 once; a second bump later in the feature fails
`crates/micold-core/tests/schema_hash.rs`. Messages whose daemon side lands in a later milestone are
answered by a `Refused` placeholder until then (T016), as feature 482 did; no client sends them
before their milestone.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1–US4 from spec.md

---

## Phase 1: Setup (Shared Infrastructure)

- [X] T001 Create the empty `crates/micold-core/src/runs/` module tree (`mod.rs` declaring `naming`, `summary`, `integrate`, `store`; one empty file each) and `pub mod runs;` in `crates/micold-core/src/lib.rs`; create an empty `crates/micold-daemon/src/runs.rs` declared in `crates/micold-daemon/src/lib.rs` (plan D1, D3)

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: the domain value types, the persistence format and the feature's whole wire delta in
one edit. No user-visible behaviour yet.

- [X] T002 [P] Write failing unit tests in `crates/micold-core/src/runs/mod.rs`: `MIN_RUNS == 2`, `DEFAULT_RUNS == 2`, `MAX_RUNS == 8`; `RunGroup::new` accepts `MIN_RUNS..=MAX_RUNS` runs numbered exactly `1..=runs.len()` with no winner and refuses 1 run, 9 runs and any other numbering; `RunGroup::validate` (on load) accepts a group whose "`runs` is non-empty, holds at most `MAX_RUNS` runs, and its `number`s are distinct, strictly increasing and within `1..=MAX_RUNS`" — including a gap left by a removed run (`#1, #3`), a single remaining run, and a `winner` whose run was removed — and refuses an empty group, a repeated or decreasing number, a number or `winner` outside `1..=MAX_RUNS`; `RunStatus::can_become` allows exactly "`Creating → Starting | Failed{Worktree}`; `Starting → Prompted | PromptNotDelivered | Failed{Session}`; `Prompted → Picked`; `PromptNotDelivered → Picked`" and refuses every other pair, `Failed` and `Picked` being terminal; serde of `GroupId` is a bare UUID string as `SessionId` is
- [X] T003 Implement in `crates/micold-core/src/runs/mod.rs` `GroupId(pub Uuid)`, `RunGroup { id, name, naming: WorktreeNaming, prompt, base_branch, base_commit, created: SystemTime, runs: Vec<Run>, winner: Option<u8> }`, `Run { number: u8, provider: AiCli, names: DerivedNames, session: Option<SessionId>, status: RunStatus }`, `RunStatus { Creating, Starting, Prompted, PromptNotDelivered { reason }, Failed { step: RunStep, reason }, Picked }`, `RunStep { Worktree, Session }`, `RunSummary { files: u32, added: u32, removed: u32, uncommitted: bool }`, `PickRefusal { AlreadyPicked, RunUnavailable(String), Uncommitted { files: Vec<RelPath> }, Conflicts { files: Vec<RelPath> }, BaseBusy(String), BaseMoved, GitTooOld, Git(String) }`, `Integration { FastForward { base_tip }, MergeCommit { commit } }`, the constants, `RunGroup::new`, `RunGroup::validate` and `RunStatus::can_become` (data-model.md § Core) until T002 passes
- [X] T004 [P] Write failing unit tests in `crates/micold-core/src/runs/naming.rs` for `derive_group(&WorktreeNaming, count: u8) -> Result<Vec<DerivedNames>, NamingError>`: `feat` + `login page` × 3 gives branches `feat/login-page-1..3` and folders `feat-login-page-1..3` in order; a ticket keeps its boundary; an empty or unslugifiable name is the `NamingError` `naming::derive` gives; every derived name is lowercase, so two group names differing only in letter case derive identical names and a case-only clash on a case-insensitive file system is the same existing-name collision `ops::create_worktree` already refuses (Edge Name collisions); every derived branch passes `git check-ref-format`'s rules and every folder is a valid directory name on Linux, macOS and Windows (no reserved names or characters) (R5, Cross-platform)
- [X] T005 Implement `derive_group` in `crates/micold-core/src/runs/naming.rs` over the existing `crate::naming::derive`, appending `-<number>` to the name part (spec Assumptions) until T004 passes
- [X] T006 [P] Write failing unit tests in `crates/micold-core/src/runs/store.rs`: a `RunsFile { version: 1, groups }` holding a group with a run in every `RunStatus` round-trips through JSON in the shape of data-model.md § Persistence (`"dir_name"`, `"branch"` on the run, `"winner": null`); `JsonFileStore::runs_path` is `runs/<project id>.json` beside `reviews_path`; a missing file loads an empty list; an unparseable file is kept aside as `<name>.corrupt` and an empty list loads; unknown fields are ignored on read; `save_runs` then `load_runs` returns the same groups and `remove_runs` deletes the file
- [X] T007 Implement `RunsFile` (de)serialisation in `crates/micold-core/src/runs/store.rs`, and in `crates/micold-core/src/store.rs` `JsonFileStore::{runs_path, load_runs, save_runs, remove_runs}` (atomic temp + rename, exactly as `save_reviews`) plus `ProjectStore::{load_runs, save_runs, remove_runs}` with persist-nothing defaults as the review methods have, and the delegating impls wherever `load_reviews` is delegated, until T006 passes
- [X] T008 Write failing wire tests in `crates/micold-core/tests/protocol_roundtrip.rs`: round-trip `ClientMsg::RunGroupCreate { req, project, naming, prompt, base_branch, providers }`, `ClientMsg::RunGroupPick { req, project, group, run }`, `ClientMsg::RunGroupDismiss { req, project, group }`, `DaemonMsg::RunGroupsChanged { project, groups }` with a group in every status, `OperationResult::RunGroupCreated { group }` and `OperationResult::RunPicked { group, run, integration }` with each `Integration` (contracts/run-group-wire.md)
- [X] T009 Make the feature's whole wire delta in one edit: the three `ClientMsg` variants, `DaemonMsg::RunGroupsChanged` and the two `OperationResult` variants in `crates/micold-core/src/protocol/messages.rs`; `PROTOCOL_VERSION` 35 → 36 with its "Bumped 35 → 36" doc line in `crates/micold-core/src/protocol/version.rs`; the new pin in `crates/micold-core/tests/schema_hash.rs`; the exhaustive matches the new variants break in `crates/micold-daemon/src/server.rs` and the client (no-op arms for now). T008 passes

**Checkpoint**: core types, the runs file format and protocol v36 in place; nothing sends a run-group
message yet.

---

## Phase 3: User Story 1 — Start N runs from one dialog (Priority: P1) 🎯 MVP

**Goal**: one dialog creates N worktrees from the base branch on derived branches, starts one
session of each run's provider and types the prompt into each; a failing run fails alone.

**Independent Test**: start a group of 3 runs (two providers) and see 3 worktrees on
`feat/login-page-1..3` at the base commit, each with one session that received the prompt once;
repeat with one provider unavailable and see the others complete.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [X] T010 [P] [US1] Write failing daemon tests `crates/micold-daemon/tests/run_group_create.rs` (fake AI CLI installed as `crates/micold-daemon/tests/mcp_create_session.rs` does): each W1 refusal (project unknown or not a repository → `InvalidInput`; `providers.len()` outside `2..=8` → `InvalidInput` naming the range; a naming error → `InvalidInput` with the `NamingError` text; an empty or whitespace prompt → `InvalidInput`; a base branch that is not a local branch → `NotFound` naming it) creates no group, worktree or branch, writes no runs file and pushes no `RunGroupsChanged`; an accepted 3-run group answers `OperationOk(RunGroupCreated)`, pushes `RunGroupsChanged` with every run `Creating`, then ends with 3 worktrees on `feat/login-page-1..3` whose tip is the base branch's tip at create time, one session per run of its provider whose working directory is that run's own worktree, each run `Prompted`, and each fake CLI having received exactly one input — the prompt, once — and nothing typed into another run; with `feat/login-page-2` already existing, run 2 ends `Failed { step: Worktree, reason }` with `CreateError`'s message and no folder or branch of its own while runs 1 and 3 end `Prompted`; a run whose provider is unavailable ends `Failed { step: Session, .. }` alone; a fake CLI that never becomes ready ends `PromptNotDelivered { reason }` with `FirstPromptUndelivered::message`, keeping its worktree and session; the prompt text appears in no log line; `RunGroupPick` and `RunGroupDismiss` are answered `OperationError { kind: Refused }` (not yet served; T035 and T058 retire these assertions)
- [X] T011 [P] [US1] Write failing daemon tests `crates/micold-daemon/tests/run_group_persist.rs`: the runs file holds the group before the first `RunGroupsChanged` push and is written atomically (no partial file observable); a failing write (read-only runs directory) answers `OperationError`, creates no worktree and leaves the groups as they were (W5); two attached clients receive the same `RunGroupsChanged`; a client receives `RunGroupsChanged` once right after `Attached`; forgetting the project removes its runs file
- [ ] T012 [P] [US1] Write failing client state tests `crates/micold-client/tests/features_runs.rs` for the dialog: opening gives `DEFAULT_RUNS` rows each on the user's default AI CLI and the base branch defaulted to the project root's current branch; each field reducer (prompt, type, ticket, name, base branch, a run's provider); add and remove stop at 2 and 8; the derived names shown update with the name and count (FR-003); `validate()` reports, in this order, empty prompt, missing type, empty name, run count outside `MIN_RUNS..=MAX_RUNS`, a provider not offered by `features::session::State::offered_providers` for the project, an invalid derived name, and confirm is refused while it fails; a valid confirm closes the dialog and emits exactly one `Effect::Send(ClientMsg::RunGroupCreate { .. })` carrying the providers in run order; dismissing emits nothing; `RunGroupsChanged` replaces the active project's group list
- [ ] T013 [P] [US1] Write a failing test in `crates/micold-client/tests/features_sidebar.rs`: the project menu offers **Run in parallel** directly after **New worktree**, and choosing it yields the outcome that opens the dialog for that project (parallel-surfaces D)

### Implementation for User Story 1

- [X] T014 [US1] Implement `Runs` in `crates/micold-daemon/src/runs.rs`: `projects: HashMap<PathBuf, Vec<RunGroup>>` read from `load_runs` on first use; `create` (the W1 checks in contract order, `git rev-parse <base_branch>` for `base_commit`, `runs::naming::derive_group`, `RunGroup::new` with every run `Creating`), `set_run_status` (only transitions `RunStatus::can_become` allows), `forget_project`; every change persisted through the catalog before memory changes, then `RunGroupsChanged` broadcast (W5)
- [X] T015 [US1] Implement the per-run task `runs::spawn_run` in `crates/micold-daemon/src/runs.rs`: `Creating` → `ops::create_worktree` (failure → `Failed { step: Worktree, reason: CreateError's message }`) → `Starting` → `ops::create_session_with_prompt` with `FirstPrompt { require_bracketed: true, .. }` (session failure → `Failed { step: Session, .. }`; undelivered → `PromptNotDelivered { reason }`; else `Prompted`), recording the session id on the run; one task per run so no run waits on or rolls back another (R3, R4, FR-005); the prompt is never formatted into a log line or error
- [X] T016 [US1] Route the three messages in `crates/micold-daemon/src/server.rs`: `RunGroupCreate` → `Runs::create` answering `OperationOk(RunGroupCreated)` then spawning the run tasks; `RunGroupPick` and `RunGroupDismiss` → `OperationError { kind: Refused, message: "run groups cannot be picked from or dismissed in this build" }` placeholders that T058 and T035 replace. In `crates/micold-daemon/src/catalog.rs` add the runs-file write (temp + rename) and `remove_runs` in `forget_project` beside `remove_reviews`, and send `RunGroupsChanged` once per connection after `Attached`; the `Runs` handle in `crates/micold-daemon/src/state.rs`. T010 and T011 pass
- [ ] T017 [US1] Implement `crates/micold-client/src/features/runs.rs` (`State`, `Msg`, `Effect::Send`, `ParallelDialog` with `naming`, `prompt`, `base_branch`, `runs: Vec<AiCli>`, `error: Option<String>`, derived names, `validate()`), register it in `crates/micold-client/src/features/mod.rs` and wire `Message::Runs` and `RunGroupsChanged` in `crates/micold-client/src/app.rs` until T012 passes
- [ ] T018 [US1] Add **Run in parallel** to the project menu in `crates/micold-client/src/features/sidebar.rs` and its rendering in `crates/micold-client/src/ui/sidebar.rs`, opening the dialog and requesting `ClientMsg::BranchList` for the base branch select, until T013 passes
- [ ] T019 [US1] Compose the dialog in `crates/micold-client/src/ui/parallel_dialog.rs` (new) per parallel-surfaces D1–D5 from existing `ui/material` builders (`Modal` + `dialog::body`, `TextArea`, `Select`, `TextField`, `IconButton`, `Button`), declare it in `crates/micold-client/src/ui/mod.rs` and register it in `crates/micold-client/src/overlay/registry.rs`; the existing builder and call-site gates (`material_builder_api.rs`, `composite_call_sites.rs`, `overlay_registration.rs`) stay green
- [ ] T020 [P] [US1] Add a showcase pose of the Run in parallel dialog (3 runs, mixed providers, derived names shown; and one with a validation error) in `crates/micold-client/src/showcase/sections/surfaces.rs` and `crates/micold-client/src/showcase/catalogue.rs`; the showcase completeness and determinism tests pass
- [ ] T021 [P] [US1] Write `docs/user-guide/parallel-runs.md` (new): what Run in parallel does, the dialog's fields, the derived branch and folder names, what happens when one run fails or its prompt is not delivered; link it from `docs/SUMMARY.md`, `docs/README.md` and `docs/user-guide/worktrees-and-sessions.md` (FR-022 part)

---

## Phase 4: User Story 2 — See the runs as one group (Priority: P1)

**Goal**: the runs sit under one collapsible group row that survives restarts, follows deletes and
can be dismissed.

**Independent Test**: start a group of 3 runs beside 2 unrelated worktrees, see one group row with
the 3 runs under it and the 2 others outside; restart and see the same.

### Part A — The group row (US2 s1, s2; ships with US1 in M1, so a failed run is visible)

- [ ] T022 [P] [US2] Write failing tests in `crates/micold-client/tests/features_sidebar.rs`: with a group of 3 runs and 2 unrelated worktrees, the tree holds one `SidebarEntry::Group` at the worktree level with the 3 runs as children in run order, each run row carrying its number and provider, and the 2 unrelated worktrees outside it at their usual positions; runs of a group never also appear ungrouped (G3); a run with no worktree (`Failed { step: Worktree, .. }`) is still a child row with its number, provider, a `failed` tag and its reason, and no worktree menu (G1a); a collapsed group hides its runs and still reports its run count and failed count (G1, G2); `row_heights` and `scroll_target` account for the group row; a run row keeps the ordinary worktree menu
- [ ] T023 [US2] Add `SidebarEntry::Group(GroupNode)` with its runs as children (worktree-backed runs wrap their usual worktree node; a run with no worktree is a leaf), its counts and expansion, and its place in `row_heights` and `scroll_target` in `crates/micold-client/src/features/sidebar.rs`, fed from `features::runs` state, until T022 passes
- [ ] T024 [US2] Render the group row (`TreeItem::expandable`, the group's name, `3 runs` and `1 failed` `Tag`s), the run rows' `#<n>` and provider, and the no-worktree run row with its reason as tooltip in `crates/micold-client/src/ui/sidebar.rs` (parallel-surfaces G1, G1a, G2, G3)
- [ ] T025 [P] [US2] Add showcase poses of the group row (expanded with 3 runs, one failed with no worktree; collapsed with `3 runs` and `1 failed`) in `crates/micold-client/src/showcase/sections/surfaces.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T026 [P] [US2] Add to `docs/user-guide/parallel-runs.md` the group row: its runs, counts and collapsing, and how a failed run is shown (FR-022 part)
- [ ] T027 [US2] Run the visual pass for quickstart B1–B6 (`visual-pass` skill) and record the results under quickstart.md § Results § B

### Part B — Restart, delete and dismiss (US2 s3–s6)

- [ ] T028 [P] [US2] Write failing unit tests in `crates/micold-core/src/runs/mod.rs` for `RunGroup::interrupted_on_load`: a persisted `Creating` run becomes `Failed { step: Worktree, reason: "interrupted" }`, a `Starting` run becomes `Failed { step: Session, reason: "interrupted" }`, every other status is unchanged, and it returns which runs it changed
- [ ] T029 [P] [US2] Extend `crates/micold-daemon/tests/run_group_persist.rs` with failing tests: after a daemon restart the groups, their runs and their order are as they were, and each run's `session` still names its restored session (FR-008, FR-021, US2 s3); a runs file holding a `Creating` run whose worktree directory exists with app-created provenance and no session loads with that run `Failed { step: Worktree, reason: "interrupted" }` and its folder and branch removed, while the finished runs are intact (R10, US2 s5); the same with a worktree that hosts a session leaves it and says so in the reason; an interrupted `Starting` run is `Failed { step: Session, .. }` and keeps its worktree and branch; `RunGroupDismiss` removes only the group (worktrees, branches and sessions untouched), pushes `RunGroupsChanged` and answers `OperationOk(Ack)`, and an unknown group is `NotFound` (W4); a `WorktreeDelete` of run 2 of 3 leaves runs `#1, #3` with their numbers, deleting the last run removes the group, and deleting the winner's run of a picked group (fixture file) keeps `winner` set (W5, US2 s4, data-model § RunGroup)
- [ ] T030 [P] [US2] Extend `crates/micold-client/tests/features_runs.rs` and `crates/micold-client/tests/features_sidebar.rs` with failing tests: the group row's menu is **Dismiss group** (Compare joins it in US3, G4); **Dismiss group** first asks for confirmation naming that the worktrees, branches and sessions stay; confirming emits exactly one `Effect::Send(ClientMsg::RunGroupDismiss { .. })`; cancelling emits nothing (G5); a group whose list shrank to `#1, #3` shows those two numbers and a dropped group is not shown
- [ ] T031 [US2] Implement `RunGroup::interrupted_on_load` in `crates/micold-core/src/runs/mod.rs` until T028 passes
- [ ] T032 [US2] On the first read of a project's runs file in `crates/micold-daemon/src/runs.rs`, apply `interrupted_on_load` and reconcile each interrupted `Creating` run on disk per research R10 (remove the worktree through `worktree::remove_worktree` + directory removal + branch delete only when it is app-created provenance and hosts no session; otherwise leave it and say so in the reason); a `Starting` run's worktree is kept; persist before anything is pushed
- [ ] T033 [US2] Implement `Runs::forget_worktree(project, dir_name)` in `crates/micold-daemon/src/runs.rs` (the run leaves its group keeping the others' numbers; an emptied group is dropped; `winner` is left as is; persisted and broadcast), `forget_run_worktree` in `crates/micold-daemon/src/state.rs` beside `forget_review_worktree`, and its call in `ops::delete_worktree` in `crates/micold-daemon/src/ops.rs` next to `forget_review_worktree`
- [ ] T034 [US2] Implement `Runs::dismiss` in `crates/micold-daemon/src/runs.rs`
- [ ] T035 [US2] Replace the `RunGroupDismiss` placeholder in `crates/micold-daemon/src/server.rs` with `Runs::dismiss`, retiring its T010 assertion. T029 passes
- [ ] T036 [US2] Implement the group row's menu (**Dismiss group**) in `crates/micold-client/src/features/sidebar.rs` and the dismiss confirmation in `crates/micold-client/src/features/runs.rs` until T030 passes
- [ ] T037 [US2] Render the group row's menu in `crates/micold-client/src/ui/sidebar.rs` and register the dismiss confirmation in `crates/micold-client/src/overlay/registry.rs` (parallel-surfaces G4, G5)
- [ ] T038 [P] [US2] Add to `docs/user-guide/parallel-runs.md` what a restart keeps, interrupted runs, deleting a run, and Dismiss group (FR-022 part)
- [ ] T039 [US2] Run the visual pass for quickstart B7, B21, B22 and record the results under quickstart.md § Results § B

**Checkpoint**: groups are visible, persistent and dismissable.

---

## Phase 5: User Story 3 — Compare the runs (Priority: P2)

**Goal**: a Compare view lists every run with status and change counts, refreshes live, and opens
any run's Changes view.

**Independent Test**: make different committed and uncommitted changes in 3 runs of a group based on
the default branch; Compare's counts equal each run's Changes view totals with both toggles on, and
Open diff opens that view.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T040 [P] [US3] Write failing unit tests in `crates/micold-core/src/runs/summary.rs`: `totals(&ChangeList)` over committed only, uncommitted only and both; a binary row counts as a file with no lines; an empty list is `0` files `+0` `−0`; `has_uncommitted` is true for an untracked-not-ignored, a staged and an unstaged row and false for committed-only rows
- [ ] T041 [P] [US3] Write failing real-git tests `crates/micold-core/tests/runs_summary.rs`: counts for a run against a non-default base branch exclude the base branch's own commits; the same run's counts equal `review::change_list` totals with both toggles on when the base is the default branch (FR-010, SC-004)
- [ ] T042 [P] [US3] Extend `crates/micold-client/tests/features_runs.rs` with failing Compare tests: opening emits one `Effect::ReadSummary { seq, run }` per run with a worktree and `update` itself reads nothing (the read is the shell's, off the UI thread); an answer with a stale `seq` is dropped and refreshes are coalesced; `Msg::RunChanged { run }` re-reads only that run; status text is Creating/Starting/Prompt not delivered/Failed/Picked from the run status and Working/Waiting for input from the session's `activity` (C2); a failed run shows its reason and no **Open diff** when it has no worktree (C3); **Open diff** yields `Outcome::ChangesRequested` for that run's worktree (C4); a run with uncommitted changes carries the `uncommitted` tag (C5); closing Compare ends its reads; and, in `crates/micold-client/tests/features_sidebar.rs`, the group row's menu now holds **Compare** and **Dismiss group** (G4)

### Implementation for User Story 3

- [ ] T043 [US3] Implement `totals` and `has_uncommitted` in `crates/micold-core/src/runs/summary.rs` over feature 482's `review::changes::ChangedFile` until T040 and T041 pass
- [ ] T044 [US3] Implement Compare state in `crates/micold-client/src/features/runs.rs` (`CompareView`: open group id, per-run `Load<RunSummary>`, read sequence numbers dropping stale answers as `features/changes.rs` does) and the **Compare** item of the group menu in `crates/micold-client/src/features/sidebar.rs` until T042 passes
- [ ] T045 [US3] Implement `crates/micold-client/src/shell/runs.rs` (new, glue): `Effect::ReadSummary` in `spawn_blocking` through `review::git` + `runs::summary`, declared in `crates/micold-client/src/shell/mod.rs`
- [ ] T046 [US3] Implement `crates/micold-client/src/shell/runs_watch.rs` (new, glue): one `Subscription` per run root while Compare is open, reusing `review::watch::{relevant_paths, Debouncer}` and `notify::RecommendedWatcher`, emitting `Msg::RunChanged { run }`; register it in `crates/micold-client/src/shell/subscriptions.rs` only while Compare is open (R11, FR-010)
- [ ] T047 [US3] Compose Compare in `crates/micold-client/src/ui/compare.rs` (new) per parallel-surfaces C1–C5 from existing components, shown by `crates/micold-client/src/ui/mod.rs` in the place of the Changes view while open
- [ ] T048 [P] [US3] Add a showcase pose of Compare (a working, a waiting, a failed and an uncommitted run) in `crates/micold-client/src/showcase/sections/review.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T049 [P] [US3] Add to `docs/user-guide/parallel-runs.md` the Compare view: its columns and statuses, which base it compares with (the group's base branch, committed and uncommitted together; equal to the Changes view's totals when that is the default branch, R7), live refresh, and Open diff (FR-022 part)
- [ ] T050 [US3] Run the visual pass for quickstart B8–B11 and record the results under quickstart.md § Results § B

**Checkpoint**: the user can compare the runs and read each diff.

---

## Phase 6: User Story 4 — Pick the winner and clean up (Priority: P2)

**Goal**: Pick this one integrates the winner into the base branch or refuses and changes nothing;
then a cleanup offer removes the chosen losers, never one with uncommitted changes without a second
confirmation.

**Independent Test**: run 2 committed, run 3 uncommitted; pick run 2, accept cleanup: the base holds
run 2's commits, run 1 is removed, run 3 is kept unless separately confirmed.

### Part A — Pick this one (US4 s1, s2, s6–s9)

- [ ] T051 [P] [US4] Write failing unit tests in `crates/micold-core/src/runs/integrate.rs`: `parse_merge_tree_conflicts` on captured `git merge-tree --write-tree` output (a content conflict, a modify/delete conflict, paths with spaces and UTF-8) returns each conflicted path once; `plan(base_tip, run_tip, is_ancestor, checkout)` gives the fast-forward when the base tip is an ancestor of the run tip, a merge commit otherwise, and the in-checkout merge when `checkout` is set; the stderr classifier gives `GitTooOld` when stderr names `--write-tree` and `Git(stderr)` otherwise; the merge-commit message is exactly `Merge run <n> of <group name> into <base>` (I2–I5)
- [ ] T052 [P] [US4] Write failing real-git tests `crates/micold-core/tests/runs_integrate.rs` (temp repos): a fast-forward moves the base and no run branch; a diverged base gets a merge commit with parents `[base_tip, run_tip]` and the run branch unmoved; a conflicting pick leaves every ref, the index and every file byte-identical and names the files; `update_ref_cas` fails and changes nothing when the base moved; a base checked out in a worktree merges in that checkout; a dirty checkout whose edit the merge would overwrite refuses with git's message and is left as it was (no `MERGE_HEAD`); a run with no changes integrates with the base unchanged
- [ ] T053 [P] [US4] Write failing daemon tests `crates/micold-daemon/tests/run_group_pick.rs`: W3's refusals in contract order (unknown group → `NotFound`; an existing winner → `Refused` "this group already has a winner"; a run `Creating` or `Starting` → `Busy` naming it; a `Failed` run → `Refused` with its reason; uncommitted changes in the run → `Refused` listing the files; conflicts → `Refused` listing the files; the base moved → `Refused`, `BaseMoved`), each leaving refs, files and the runs file unchanged; an accepted pick answers `OperationOk(RunPicked { integration })`, writes `winner` and the run's `Picked` only after the ref moved, and pushes `RunGroupsChanged`; two concurrent picks of the same group integrate exactly one and the other is refused with the winner reason (FR-018); retire T010's `RunGroupPick` placeholder assertion
- [ ] T054 [P] [US4] Extend `crates/micold-client/tests/features_runs.rs` with failing pick tests: **Pick this one** is absent for a `Failed` run and disabled on every row while any run is `Creating` or `Starting`, with the reason (C6); after a pick no row offers it and the winner reads Picked; picking a run whose session is working first asks a confirmation saying so, and cancelling sends nothing (C7); confirming sends exactly one `RunGroupPick`; each refusal's text (uncommitted files, conflicting files, git's message, the winner reason) is reported and the view is unchanged (C8)
- [ ] T055 [US4] Implement the pure half of contracts/integration.md in `crates/micold-core/src/runs/integrate.rs` (`parse_merge_tree_conflicts`, `plan`, the stderr classifier, the message builder) until T051 passes
- [ ] T056 [US4] Add `merge_tree_write_tree`, `commit_tree_merge`, `update_ref_cas`, `merge_in_checkout` and `merge_abort` to the `Git` trait in `crates/micold-core/src/git.rs`, each a thin `run_git` call through `no_window`/`local_only`, with the `FakeGit` arms; reuse `branch_tip` and `is_ancestor`; until T052 passes
- [ ] T057 [US4] Implement `ops::pick_run` in `crates/micold-daemon/src/ops.rs` under the project's worktree gate (W3 order; I1 observed once; uncommitted check via `review::git` + `runs::summary::has_uncommitted`; merge abort on a left-over `MERGE_HEAD`), `Runs::pick` recording the winner after the ref moved in `crates/micold-daemon/src/runs.rs`, and `base_branch_is_checked_out` from the discovery cache in `crates/micold-daemon/src/state.rs`
- [ ] T058 [US4] Replace the `RunGroupPick` placeholder in `crates/micold-daemon/src/server.rs` with `ops::pick_run`. T053 passes
- [ ] T059 [US4] Implement the pick gate, the still-working confirmation and refusal reporting in `crates/micold-client/src/features/runs.rs` until T054 passes; render **Pick this one**, its tooltip, the confirmation and the refusal snackbar in `crates/micold-client/src/ui/compare.rs` (C6–C8), registering the confirmation in `crates/micold-client/src/overlay/registry.rs`
- [ ] T060 [P] [US4] Add to `docs/user-guide/parallel-runs.md` Pick this one: fast-forward or merge commit, the commit message, when a pick is unavailable or refused and that a refusal changes nothing, the still-working confirmation, one winner per group, that a base branch checked out somewhere (for example the project root) is merged in that checkout so sessions there see the new files, git 2.38 or newer (FR-022 part)
- [ ] T061 [US4] Run the visual pass for quickstart B12–B16 and B20 and record the results under quickstart.md § Results § B

### Part B — The cleanup offer (US4 s3–s5)

- [ ] T062 [P] [US4] Extend `crates/micold-client/tests/features_runs.rs` with failing cleanup tests: a successful `RunPicked` opens the offer once with a heading naming what happened ("Run 2 was merged into main" / "… fast-forwarded main", K1), one row per loser with its sessions count and **Delete the branch too** on (K2), and one fresh `Effect::ReadSummary` per loser; a loser whose read says uncommitted, is still pending, or failed is tagged, starts unselected and counts as uncommitted (K3); `CleanupOffer::removable()` never contains such a loser unless it is both selected and confirmed (FR-015, SC-005); confirming re-reads every selected loser first and sends nothing until those answers arrive; a loser that turned uncommitted since the offer opened, or any selected uncommitted loser, triggers a second confirmation naming it, and declining it removes the others only; then exactly one `ClientMsg::WorktreeDelete { stop_sessions: true, delete_branch }` per removable loser and nothing else (K4, K5); dismissing the offer emits nothing and it does not reopen (K6)
- [ ] T063 [US4] Implement `CleanupOffer` (per loser `selected: bool`, `uncommitted: Clean | Uncommitted | Unknown` with `Unknown` treated as `Uncommitted`, `delete_branch: bool`, and `confirming: Option<u8>` for the second confirmation), its fresh reads on open and on confirm, and `CleanupOffer::removable()` in `crates/micold-client/src/features/runs.rs` until T062 passes
- [ ] T064 [US4] Compose the cleanup offer and its second confirmation as `Modal`s (`Checkbox`, `Tag`, `Button`) in `crates/micold-client/src/ui/compare.rs` and register both in `crates/micold-client/src/overlay/registry.rs` (parallel-surfaces K1–K6)
- [ ] T065 [P] [US4] Add a showcase pose of the cleanup offer (two losers, one with uncommitted changes) in `crates/micold-client/src/showcase/sections/surfaces.rs` and `crates/micold-client/src/showcase/catalogue.rs`
- [ ] T066 [P] [US4] Add to `docs/user-guide/parallel-runs.md` the cleanup offer: what removal deletes, keeping a branch, losers with uncommitted changes and the second confirmation, declining (FR-022 part)
- [ ] T067 [US4] Run the visual pass for quickstart B17–B19 and record the results under quickstart.md § Results § B

**Checkpoint**: the whole loop of the issue works: start, group, compare, pick, clean up.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T068 Run quickstart B23 (showcase, both schemes) and record it under quickstart.md § Results § B
- [ ] T069 Run or record quickstart §C (macOS and Windows: B1, B3, B8, B9, B13, B16, B22, names and merge message byte-for-byte) under quickstart.md § Results § C, or record it as not run with what CI covers instead
- [ ] T070 Re-read `docs/user-guide/parallel-runs.md` end to end against the shipped behaviour and spec FR-022

---

## Dependencies & Execution Order

### Phase dependencies

- Setup → Foundational → US1 → US2 Part A → US2 Part B → US3 → US4 Part A → US4 Part B → Polish.
- US2 Part A ships with US1 because the group row is where a failed run and its reason are seen
  (FR-005, US1 s3). US2 Part B needs Part A. US3 needs US2 (Compare opens from the group row). US4
  Part A needs US3 (Pick this one lives in Compare); Part B needs Part A (the offer follows a pick).

### Within each part

Tests first and failing (Constitution I), then core, then daemon, then client state, then shell and
glue; showcase and user guide in the same milestone; the visual pass last.

### Parallel opportunities

Tasks marked [P] touch disjoint files: in Foundational T002, T004, T006 together; in US1 T010–T013
together and T020–T021 beside T019; in US2 Part B T028–T030 together; in US3 T040–T042 together; in
US4 Part A T051–T054 together.

## Parallel Example: User Story 1

```text
Task: "T010 run_group_create.rs daemon tests"
Task: "T011 run_group_persist.rs daemon tests"
Task: "T012 features_runs.rs dialog tests"
Task: "T013 features_sidebar.rs project menu test"
```

## Implementation Strategy

MVP first: Setup, Foundational, US1 and the group row give one dialog that starts N prompted runs,
shown together with any failure and its reason. Each later milestone adds an observable step:
restart, delete and dismiss; Compare; Pick this one; the cleanup offer. Polish changes no code; the
close unit does it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).
T068–T070 (Polish) change no code and are left to the close unit (milestones rule 4).

### M1a — The service starts N prompted runs as one group

- **Tasks**: T001–T011, T014–T016 (split from M1 by the orchestrator: core, protocol and daemon)
- **Deliverable**: `RunGroupCreate` creates N worktrees on `feat/<name>-1..N` from the base branch, each with a session of its provider that received the prompt once; the group is persisted and pushed to every window as `RunGroupsChanged`, and a failing run fails alone with its reason
- **Satisfies**: US1 acceptance scenarios 1–4 (service side); FR-003–FR-006, FR-008 (persisted and pushed to every window), FR-021; SC-002; protocol v36
- **Verify**: `cargo test -p micold-core --all-targets runs`, `cargo test -p micold-core --test protocol_roundtrip --test schema_hash --test worktree_create`, `cargo test -p micold-daemon --test run_group_create --test run_group_persist`
- **Depends on**: —
- **Tier**: full

### M1b — Run in parallel dialog and the group row

- **Tasks**: T012, T013, T017–T027
- **Deliverable**: project menu → **Run in parallel** opens the dialog that sends `RunGroupCreate`; the runs appear under one collapsible group row with run and failed counts, and a failing run's reason is shown on its row
- **Satisfies**: US1 acceptance scenarios 1–5 (client side), US2 acceptance scenarios 1–2; FR-001, FR-002, FR-007, FR-022 (dialog and group-row parts); SC-001
- **Verify**: `cargo test -p micold-client --test features_runs --test features_sidebar`; quickstart B1–B6
- **Depends on**: M1a
- **Tier**: full

### M2 — Groups survive restarts, follow deletes and can be dismissed

- **Tasks**: T028–T039
- **Deliverable**: after a restart the groups and runs are as they were (interrupted runs marked, half-created ones cleaned); deleting a run's worktree removes it from its group and an emptied group disappears; **Dismiss group** removes only the grouping
- **Satisfies**: US2 acceptance scenarios 3–6; FR-007, FR-008, FR-019, FR-020, FR-022 (lifecycle part); SC-003
- **Verify**: `cargo test -p micold-core --lib runs`, `cargo test -p micold-daemon --test run_group_persist --test run_group_create`, `cargo test -p micold-client --test features_sidebar --test features_runs`; quickstart B7, B21, B22
- **Depends on**: M1
- **Tier**: full

### M3 — Compare the runs

- **Tasks**: T040–T050
- **Deliverable**: group row → **Compare** lists each run with provider, status and `<files> files +a −r` (equal to its Changes view totals for a default-branch base), refreshing within 2 s, and **Open diff** opens the run's Changes view
- **Satisfies**: US3 acceptance scenarios 1–5; FR-009, FR-010, FR-011, FR-022 (Compare part); SC-004
- **Verify**: `cargo test -p micold-core --all-targets runs`, `cargo test -p micold-core --test runs_summary`, `cargo test -p micold-client --test features_runs --test features_sidebar`; quickstart B8–B11
- **Depends on**: M2
- **Tier**: full

### M4 — Pick this one

- **Tasks**: T051–T061
- **Deliverable**: **Pick this one** in Compare fast-forwards the base branch or writes a merge commit on it, refuses and changes nothing on conflicts, uncommitted changes in the winner, a busy base checkout or an existing winner, and records the winner
- **Satisfies**: US4 acceptance scenarios 1, 2, 6, 7, 8, 9; FR-012, FR-013, FR-017, FR-018, FR-022 (pick part); SC-006
- **Verify**: `cargo test -p micold-core --lib runs::integrate`, `cargo test -p micold-core --test runs_integrate`, `cargo test -p micold-daemon --test run_group_pick --test run_group_create`, `cargo test -p micold-client --test features_runs`; quickstart B12–B16, B20
- **Depends on**: M3
- **Tier**: full

### M5 — Clean up the losers

- **Tasks**: T062–T067
- **Deliverable**: after a pick a cleanup offer removes the selected losers (worktree, sessions, optionally branch) through the existing Delete; a loser with uncommitted (or not yet known) changes starts unselected and is removed only after a second confirmation naming it
- **Satisfies**: US4 acceptance scenarios 3, 4, 5; FR-014, FR-015, FR-016, FR-022 (cleanup part); SC-005
- **Verify**: `cargo test -p micold-client --test features_runs`; quickstart B17–B19
- **Depends on**: M4
- **Tier**: full
