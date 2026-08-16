---

description: "Task list for feature 016 — Reuse or Overwrite an Existing Branch When Creating a Worktree"
---

# Tasks: Reuse or Overwrite an Existing Branch When Creating a Worktree

**Input**: Design documents from `/specs/016-existing-branch-worktree/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are MANDATORY. Every story writes failing tests BEFORE implementation (Red-Green-Refactor). The only exception used here is Principle I's documented GUI-wiring exception for `src/ui/` and `src/main.rs` glue, validated by [quickstart.md](./quickstart.md).

**Documentation**: Per Constitution Principle VII, each user-facing story carries its own `docs/user-guide/worktrees-and-sessions.md` update in the same change (FR-026).

**Cross-platform**: Per Constitution Principle VI, all logic stays platform-agnostic behind the `Git` trait; CI covers Linux, macOS, Windows.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1–US5)
- Include exact file paths in descriptions

## Path Conventions

Single Rust crate: render-free core in `src/` (`--no-default-features`), `gui`-only binary in `src/main.rs` + `src/ui/`, tests in `tests/`.

---

## Phase 1: Setup

**Purpose**: Confirm a green baseline before changing shared creation code. No new dependencies are introduced by this feature.

- [X] T001 Establish a green baseline: run `mise run test`, `cargo test --features gui`, and `cargo clippy --features gui --all-targets` from the repository root and record that all pass before any change

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The shared two-phase creation mechanism. Every user story is a surface on top of this — none can begin until it exists.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

**Why so much lands here**: this feature is one shared mechanism (`preflight` → `BranchSituation` → `CreateMode` → dispatch) with five thin per-story surfaces. `BranchSituation` and `CreateMode` are closed enums (Principle V); splitting their variants across story phases would mean re-editing the same `match` arms five times.

### Tests (write first, confirm RED)

- [X] T002 [P] Failing tests for `CreateMode::creates_branch()` and the `rollback_plan(mode)` step sequence for all four modes in `tests/worktree_rollback.rs` (contracts/branch-conflict.md §5)
- [X] T003 [P] Failing tests for `parse_branch_refs()` — line-shape mapping, `refs/remotes/*/HEAD` dropped, multi-segment remote branch names split on the first component, local+remote duplicates collapsing to `Local` — in `tests/git_fake.rs` (contracts/git-trait-branches.md)
- [X] T004 [P] Failing tests for `preflight()` classification, the five-rule precedence order, and the no-mutation guarantee, in new file `tests/branch_conflict.rs` (contracts/branch-conflict.md §1, §7 items 1–5)
- [X] T005 [P] Failing tests for `create_worktree(mode)` re-verification: every (`CreateMode`, `BranchSituation`) pair resolves compatible/incompatible per contracts/branch-conflict.md §4, incompatible pairs return `SituationChanged` with no mutation, and `Free` + `NewBranch` stays byte-identical to today (SC-008), in `tests/worktree_create.rs`
- [X] T006 [P] Failing tests for the `ResolutionState` machine — every transition plus invariants 1–4, and cancel preserving all form inputs — in `tests/app_state.rs` (contracts/branch-conflict.md §3)

### Implementation

- [X] T007 Add `BranchOrigin`, `BranchCandidate`, `BlockReason`, `BranchSituation`, and `CreateMode` (with `creates_branch()`, `Default = NewBranch`) to `src/worktree.rs` per data-model.md
- [X] T008 Change `rollback_plan()` to `rollback_plan(mode: CreateMode) -> Vec<CleanupStep>`, omitting `CleanupStep::BranchDelete` when `!mode.creates_branch()`, in `src/worktree.rs` (FR-008)
- [X] T009 Add `list_branch_refs` to the `Git` trait, implement on `GitCli` (`for-each-ref --format=%(refname) refs/heads refs/remotes`) and on `FakeGit` (plus a `with_remote_branch(repo, remote, name)` builder), in `src/git.rs`
- [X] T010 Implement the pure `parse_branch_refs()` in `src/worktree.rs` per contracts/git-trait-branches.md parser rules
- [X] T011 Implement `preflight(git, repo, target_path, branch, target_exists) -> io::Result<BranchSituation>` in `src/worktree.rs`, reading the **raw** `parse_worktrees()` records rather than `reconcile()` (research R1 — `reconcile()` filters out the project's own checkout, which FR-021 needs)
- [X] T012 Update `CreateError` in `src/worktree.rs`: remove `DuplicateBranch`, add `BranchInUse { branch, reason }` and `SituationChanged`, and fix every resulting compile error (removal is deliberate — it makes the compiler find each site that treated the collision as terminal)
- [X] T013 Change `create_worktree()` in `src/worktree.rs` to take `mode: CreateMode`, re-run `preflight()` as its first action and abort with `SituationChanged` on incompatibility before any mutation, dispatch `NewBranch` to today's `worktree_add_new_branch`, and unwind through `rollback_plan(mode)`
- [X] T014 Make `CreateStage::label()` mode-dependent for the `CreatingWorktree` stage in `src/worktree.rs` (research R12, FR-024); stage set is unchanged
- [X] T015 Add `BranchSource`, `ResolutionState`, and the `source` / `candidates` / `selected_branch` / `resolution` fields to `WorktreeForm` in `src/app.rs` per data-model.md
- [X] T016 Add the new `Message` variants and their reducer arms implementing the `ResolutionState` transitions in `src/app.rs` (contracts/branch-conflict.md §3)
- [X] T017 Thread `CreateMode` through the background `create()` job and update `describe_create_error()` for the new `CreateError` variants in `src/main.rs`

**Checkpoint**: Pre-flight classifies every situation, rollback is mode-aware, and conflict-free creation is provably unchanged. User stories can now begin.

---

## Phase 3: User Story 1 - Continue work on a branch that already exists (Priority: P1) 🎯 MVP

**Goal**: A branch-name collision becomes a prompt offering **Reuse**, and reusing checks the existing branch out into a new worktree with its history intact.

**Independent Test**: Create a branch with a distinctive commit outside the app, create a worktree deriving that name, choose Reuse, and confirm the worktree is on that branch with the commit present.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> Write these FIRST and confirm they FAIL before implementing.

- [X] T018 [P] [US1] Failing test: `CreateMode::ReuseLocal` binds the existing branch and leaves its tip unmoved, in `tests/worktree_create.rs`
- [X] T019 [P] [US1] Failing test — **the feature's most important guard**: with `failing_next_add` primed, a `ReuseLocal` attempt rolls back without deleting the pre-existing branch, in `tests/worktree_rollback.rs` (FR-008, SC-003)
- [X] T020 [P] [US1] Failing tests for `worktree_add_existing_branch` on `FakeGit`: branch set unchanged, error for an unknown branch and for a branch already bound to another worktree, `failing_next_add` honored once — in `tests/git_fake.rs`
- [X] T021 [P] [US1] Failing test: a `LocalAvailable` situation drives the form to `Choosing` rather than returning an error, and the reuse answer submits `CreateMode::ReuseLocal`, in `tests/branch_conflict.rs`

### Implementation for User Story 1

- [X] T022 [US1] Add `worktree_add_existing_branch` (`git worktree add <path> <branch>`) to the `Git` trait, `GitCli`, and `FakeGit` in `src/git.rs`
- [X] T023 [US1] Dispatch `CreateMode::ReuseLocal` to `worktree_add_existing_branch` in `create_worktree()` in `src/worktree.rs`
- [X] T024 [US1] Render the conflict panel (branch name, plain-language reuse/overwrite explanation, Reuse and Cancel actions) inside the existing `Modal` in `src/ui/worktree_form.rs`
- [X] T025 [US1] Add the `main.rs` arm that submits a create with the resolved `CreateMode` from the panel, reusing the existing background-create plumbing, in `src/main.rs`
- [X] T026 [US1] Document the existing-branch prompt and what Reuse does in `docs/user-guide/worktrees-and-sessions.md` (FR-026)

**Checkpoint**: US1 is independently shippable — quickstart Scenarios 1 and 3 pass. This is the MVP.

---

## Phase 4: User Story 2 - Pick an existing branch instead of guessing its name (Priority: P2)

**Goal**: The create form gains an existing-branch source with a candidate list, so continuing prior work no longer depends on retyping inputs that happen to derive the right branch name.

**Independent Test**: Create several branches outside the app, open the form, switch to Existing branch, select one, and confirm a worktree is created on exactly that branch without typing a name.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [X] T027 [P] [US2] Failing tests for `naming::dir_name_from_branch()` — multi-segment branches, uppercase, punctuation, Windows reserved device names, and the empty-result case — in `tests/naming.rs`
- [X] T028 [P] [US2] Failing tests for candidate listing, `blocked_by` annotation for both `BlockReason` variants, the four-key ordering, row labels, and the two distinct empty-list explanations, in new file `tests/branch_candidates.rs` (contracts/branch-picker.md §6 items 1–5)
- [X] T029 [P] [US2] Failing tests for form behavior: switching source clears `selected_branch` and preserves new-branch inputs, `preview()` derives from the candidate under `Existing`, blocked selection disables submit, and the §5 skip rule never reaches `Overwrite` — in `tests/app_state.rs`

### Implementation for User Story 2

- [X] T030 [P] [US2] Implement the pure `dir_name_from_branch(branch) -> String` (split on `/`, `slugify` each segment, drop empties, join with `-`) in `src/naming.rs`
- [X] T031 [US2] Implement `branch_candidates()` in `src/worktree.rs`: annotate parsed candidates with `blocked_by` from the same worktree records `preflight()` uses (one pass, no second git call) and apply the four-key ordering
- [X] T032 [US2] Implement `Display for BranchCandidate` producing the four row-label shapes in `src/worktree.rs` (contracts/branch-picker.md §2)
- [X] T033 [US2] Make `WorktreeForm::preview()` honor `source`, deriving the directory from `selected_branch` under `Existing`, in `src/app.rs` (FR-014)
- [X] T034 [US2] Add reducer arms for source switching, candidate listing, and candidate selection — including the submit skip rule of contracts/branch-picker.md §5 — in `src/app.rs`
- [X] T035 [US2] Render the `ToggleChip` source switch, the candidate `Select`, the remote-staleness note, and the empty-list explanations in `src/ui/worktree_form.rs` (reuse existing shared primitives — Principle VIII, no new widget)
- [X] T036 [US2] Show the blocked explanation and disable Create when a blocked candidate is selected, in `src/ui/worktree_form.rs` (research R8 — `pick_list` has no per-item disabling, so block at the point of action)
- [X] T037 [US2] Populate the candidate list when the source switches to Existing, via `list_branch_refs`, in `src/main.rs`
- [X] T038 [US2] Document the existing-branch picker, its row labels, and the staleness note in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: US1 and US2 both work independently — quickstart Scenario 5 passes.

---

## Phase 5: User Story 3 - Start over on a name that is already taken (Priority: P3)

**Goal**: The prompt's **Overwrite** answer discards a stale branch and recreates it at HEAD, behind a second explicit destructive confirmation.

**Independent Test**: Create a branch with a distinctive commit, create a worktree deriving that name, choose Overwrite and confirm, and verify the worktree starts at HEAD with the distinctive commit no longer reachable from that branch.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [X] T039 [P] [US3] Failing test: `CreateMode::Overwrite` recreates the branch at HEAD and registers the worktree, in `tests/worktree_create.rs`
- [X] T040 [P] [US3] Failing tests: `worktree_add_reset_branch` creates-or-keeps the branch and leaves it present on primed failure (`tests/git_fake.rs`), and an `Overwrite` rollback deletes the branch it created (`tests/worktree_rollback.rs`)
- [X] T041 [P] [US3] Failing test: `CreateMode::Overwrite` is reachable **only** via `ConfirmingOverwrite` (invariant 1), and backing out returns to `Choosing` rather than `Idle` (invariant 3), in `tests/app_state.rs`

### Implementation for User Story 3

- [X] T042 [US3] Add `worktree_add_reset_branch` (`git worktree add -B <branch> <path> HEAD`) to the `Git` trait, `GitCli`, and `FakeGit` in `src/git.rs`
- [X] T043 [US3] Dispatch `CreateMode::Overwrite` to `worktree_add_reset_branch` in `create_worktree()` in `src/worktree.rs`
- [X] T044 [US3] Render the destructive-overwrite warning (naming the branch, stating its commits will be discarded) with Confirm and Back actions in `src/ui/worktree_form.rs` (FR-005)
- [X] T045 [US3] Document Overwrite and state plainly that it is not undoable — git's reflog is not an undo feature this app offers — in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: quickstart Scenario 2 passes; US1–US3 independently functional.

---

## Phase 6: User Story 4 - Continue work pushed from somewhere else (Priority: P4)

**Goal**: A branch that exists only on a remote is recognized and can be continued as a local tracking branch — without contacting the remote.

**Independent Test**: Make a branch available on a remote with no local counterpart, create a worktree for that name, choose Continue from remote, and verify the local branch sits at the remote tip and tracks it.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

- [X] T046 [P] [US4] Failing tests: `worktree_add_tracking_branch` requires the remote ref, creates the local branch, records the upstream, never mutates the remote ref, and honors `failing_next_add` — in `tests/git_fake.rs`, plus the `TrackRemote` creation path in `tests/worktree_create.rs`
- [X] T047 [P] [US4] Failing tests: `RemoteOnly` classification, the same name on multiple remotes, and local-beats-remote precedence (FR-019), in `tests/branch_conflict.rs`
- [X] T048 [P] [US4] Failing test: the "start fresh at HEAD" answer to a `RemoteOnly` situation submits `CreateMode::NewBranch`, in `tests/app_state.rs`

### Implementation for User Story 4

- [X] T049 [US4] Add `worktree_add_tracking_branch` (`git worktree add --track -b <branch> <path> <remote>/<branch>`, remote named explicitly — never DWIM/`--guess-remote`) to the `Git` trait, `GitCli`, and `FakeGit` in `src/git.rs`
- [X] T050 [US4] Dispatch `CreateMode::TrackRemote { remote }` to `worktree_add_tracking_branch` in `create_worktree()` in `src/worktree.rs`
- [X] T051 [US4] Render the remote-branch answers — Continue from `<remote>`, and Start fresh at HEAD carrying the divergence warning (FR-018) — in `src/ui/worktree_form.rs`
- [X] T052 [US4] Document continuing from a remote branch and the last-fetch caveat (nothing is downloaded) in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: quickstart Scenario 4 passes, including its offline check with the remote moved away.

---

## Phase 7: User Story 5 - Understand when an existing branch cannot be used (Priority: P5)

**Goal**: A branch already checked out elsewhere produces a clear explanation naming its holder, instead of a raw git failure — and offers no reuse or overwrite.

**Independent Test**: Bind a branch to one worktree, attempt to create a second worktree deriving the same name, and confirm the app names the holding worktree and offers no branch actions.

### Tests for User Story 5 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T053 [P] [US5] Failing tests: `Blocked` distinguishes `CheckedOutInProjectRoot` from `CheckedOutAt { path }` and carries the holder's path, in `tests/branch_conflict.rs` — **reopened by BUG-001**: two variants is one too few; superseded by T064
- [X] T054 [P] [US5] Failing test: a `Blocked` or `DirectoryTaken` situation offers no actionable resolution and returns to `Idle` on dismiss, leaving inputs intact, in `tests/app_state.rs`

### Implementation for User Story 5

- [X] T055 [US5] Produce the `BranchInUse` user-facing message naming the branch and its holder — a worktree directory, or the project's own checkout — in `describe_create_error()` in `src/main.rs`
- [X] T056 [US5] Render the blocked/directory-taken explanation panel with dismiss only (no reuse, no overwrite) in `src/ui/worktree_form.rs`
- [X] T057 [US5] Document when neither reuse nor overwrite is available, and what to do instead, in `docs/user-guide/worktrees-and-sessions.md`

**Checkpoint**: quickstart Scenario 6 passes; all five stories independently functional.

---

## Phase 8: Polish & Cross-Cutting Concerns

> Per-story user-guide docs ship inside their own story (Principle VII). This phase is cross-cutting review only, not deferred documentation.

- [X] T058 [P] Cross-cutting documentation review — reconcile the five story additions into one coherent section and update `docs/README.md` navigation if needed
- [X] T059 [P] Confirm no `CreateError::DuplicateBranch` references or other dead code remain: `grep -rn "DuplicateBranch" src/ tests/ docs/` returns nothing
- [X] T060 Run `cargo clippy --features gui --all-targets` clean from the repository root
- [X] T061 Run `mise run test` and `cargo test --features gui` and confirm the full suite is green
- [ ] T062 Verify build and tests pass on Linux, macOS, and Windows via the matrix in `.github/workflows/ci.yml` (Constitution Principle VI)
- [ ] T063 Run all seven [quickstart.md](./quickstart.md) scenarios manually, including Scenario 4's offline check (the GUI-wiring validation Principle I's exception requires)

---

## Phase 9: BUG-001 — a holder that is named but not listed

**Goal**: A branch held by a worktree the sidebar cannot show is still blocked, but the explanation
identifies the holder in terms the user can act on (FR-021, FR-021a, FR-021b, SC-006).

**Independent Test**: In a repository carrying a worktree outside `.claude/worktrees/`, select the
branch that worktree holds and confirm the message says the holder is outside the app and gives its
full path — not a bare folder name.

### Tests for BUG-001 (MANDATORY — Constitution Principle I) ⚠️

- [X] T064 [P] Failing tests: `preflight()` classifies all four holder shapes in `contracts/branch-conflict.md` §1 — repo root, a `.claude/worktrees/` child, an agent-owned `.claude/worktrees/` child, and a record anywhere else — into the right `BlockReason`, carrying the holder's absolute path, in `tests/branch_conflict.rs` (supersedes T053)
- [X] T065 [P] Failing tests: `branch_candidates()` annotates `blocked_by` with the same four shapes, and the `Display` row labels match `contracts/branch-picker.md` §2, in `tests/branch_candidates.rs`
- [X] T066 [P] Failing test: the classification split uses `reconcile()`'s parent test, so a holder described as one of the app's worktrees is one `discover()` would return — a nested path under `.claude/worktrees/a/b` is `CheckedOutOutsideApp`, not `CheckedOutAt`, in `tests/branch_conflict.rs`

### Implementation for BUG-001

- [X] T067 Add `CheckedOutOutsideApp { path }` and `owner: WorktreeOwner` on `CheckedOutAt`; classify by the worktrees-root parent test in `checked_out_branches()`, in `micold-core/src/worktree.rs`
- [X] T068 Extract the name-derived owner rule shared by `Worktree::owner()` and the new classification into one function, so the two cannot drift (feature 014, FR-005/FR-009), in `micold-core/src/worktree.rs`
- [X] T069 [P] Word the three holders in `block_sentence()` — folder name, folder name plus the reveal hint, and "outside this app" with the full path — in `micold-client/src/ui/worktree_form.rs`
- [X] T070 [P] Word the same three in the daemon's `describe_create_error()`, in `micold-daemon/src/server.rs`
- [X] T071 [P] Row labels for the two new shapes in `Display for BranchCandidate`, in `micold-core/src/worktree.rs`
- [X] T072 Document the unmanaged and hidden holders in `docs/user-guide/worktrees-and-sessions.md` (Principle VII)

---

## Phase 10: BUG-002 — a refusal that closes the form, and a holder that cannot be included

**Bugfix**: 2026-08-14 — [BUG-002](./bugs/BUG-002.md) Updated from bugfix patch. Twenty tasks added
(T073–T092); no earlier task reopened — every one of them is complete against the requirement it was
written for, and both halves of this bug are requirements that did not exist.

**Goal (A)**: reaching for a branch the app refuses costs the user nothing — the press is consumed
where it lands and the form survives (FR-034, FR-035, SC-009).

**Goal (B)**: a branch held by a worktree the app does not manage stops being a dead end — the
worktree can be included, exactly where it is (FR-027–FR-033, SC-010, SC-011).

**Independent Test (A)**: with the branch list open, press an in-use row that renders below the
dialog's lower edge; the form is still there with every input intact.
**Independent Test (B)**: `git worktree add ../elsewhere feat/x` outside the app, select `feat/x`,
include the worktree it names, and confirm it appears in the list, hosts a session, survives a
restart, and is unchanged on disk.

### Tests for BUG-002 (MANDATORY — Constitution Principle I) ⚠️

- [X] T073 [P] Failing test: an unavailable row consumes its press — a left press inside a disabled row publishes no message **and reports the event captured**, so nothing behind it can see it, in `crates/micold-client/src/ui/material/picker_press.rs` (new, in-crate: `material` is `pub(crate)` and the type-ahead cannot be built from `tests/`, the same reason `picker_parity` gives)
- [X] T074 [P] Failing test: with the branch list open and a disabled row laid out **past the dialog's own bounds**, a press on that row publishes no `AddWorktreeCancelled` (FR-034/FR-035, SC-009) — the position is the whole test, and it is *measured* rather than assumed, in `crates/micold-client/tests/add_worktree_form_survives_a_refusal.rs` (new)
- [X] T075 [P] Failing tests: `reconcile()` keeps a record whose path is in the included set and marks it `included`, still drops one that is neither, and reports an included path git no longer registers as missing rather than inventing it (FR-029/FR-031), in `crates/micold-core/tests/worktree_discovery.rs`
- [X] T076 [P] Failing tests: holder classification follows the same widened predicate — an included holder is `CheckedOutAt`, an unincluded one `CheckedOutOutsideApp`, the project root unaffected (FR-032), in `crates/micold-core/tests/branch_conflict.rs`
- [X] T077 [P] Failing tests: `StoredProjectState` round-trips `included_worktrees`; a file written before the field loads empty; unknown fields are still ignored (FR-030), in `crates/micold-core/tests/store_roundtrip.rs`
- [X] T078 [P] Failing tests: include and exclude are idempotent in both directions, and including a path the repository does not report as a worktree is rejected rather than recorded (contract `branch-rpc.md` §3a, FR-028), in `crates/micold-daemon/tests/mutation_semantics.rs`

### Implementation for BUG-002 — Goal A (the press)

- [X] T079 Make an unavailable row consume its press — still unpressable, no longer transparent (research R14, FR-035). The picker's rows are `row_element()` in `crates/micold-client/src/ui/material/picker.rs`, **not** `material::menu` as BUG-002 first recorded; `menu::item_column()` carries the same shape for message-less menu items and is fixed with it, so the two kinds of inert row cannot diverge
- [X] T080 Belt-and-braces: `Menu::update()` captures a left press within its own bounds even when its content declines it, in `crates/micold-client/src/ui/cdk/picker.rs`
- [X] T081 [P] Record in the module doc of `crates/micold-client/tests/branch_search_state.rs` that it covers the reducer only, and point at T073/T074 for the press — the seam that let a stated requirement (021 FR-012a) ship violated

### Implementation for BUG-002 — Goal B (inclusion)

- [X] T082 Add `included_worktrees: Vec<PathBuf>` to `StoredProjectState` — `#[serde(default, skip_serializing_if = "Vec::is_empty")]`, no `schema_version` bump, per `data-model.md` — in `crates/micold-core/src/store.rs`
- [X] T083 Widen `reconcile()` with the included set and add `Worktree::included`; thread it through `discover()` so every consumer inherits it, in `crates/micold-core/src/worktree.rs` (blocks T084)
- [X] T084 Apply that same predicate in `checked_out_branches()`, so FR-032 holds by construction rather than by two rules kept in step (BUG-001's invariant), in `crates/micold-core/src/worktree.rs`
- [X] T085 `WorktreeInclude` / `WorktreeExclude` messages and their results, in `crates/micold-core/src/protocol/messages.rs` — **this file is inside the `SCHEMA_HASH` guard**, so the hash changes (`branch-rpc.md` §1). No test edit was needed: `build.rs` bakes the hash and `tests/schema_hash.rs` recomputes it from the same source, so the guard updated itself — a mismatched client/daemon pair now fails the handshake, which is the intent
- [X] T086 Daemon handlers: validate the path against the repository's own records, write the project's included set, and answer with the worktree `discover()` now returns, in `crates/micold-daemon/src/server.rs`
- [X] T087 [P] Offer "Include that worktree" in the blocked explanation — for `CheckedOutOutsideApp` and no other holder (FR-033) — and wire `WorktreeIncludeRequested`, in `crates/micold-client/src/ui/worktree_form.rs` and `crates/micold-client/src/app.rs`
- [X] T088 [P] Show an included worktree's location in its sidebar row, and name that location in its delete confirmation before anything is removed (FR-029/FR-033), in `crates/micold-client/src/features/worktree.rs` and `crates/micold-client/src/ui/`
- [X] T089 [P] "Stop showing" from the worktree's own context menu, leaving it untouched on disk (FR-030) — offered only on the rows inclusion produced, in `crates/micold-client/src/ui/mod.rs` and `crates/micold-client/src/app.rs`
- [X] T090 Document inclusion in `docs/user-guide/worktrees-and-sessions.md`: what it shows, what it does **not** touch, and how to undo it (Principle VII, FR-026)

### Validation for BUG-002

- [X] T091 Run the repo's `visual-pass` skill over both flows: a press on an in-use branch row rendered past the dialog's edge (SC-009), and include → start a session → restart → stop showing (SC-010, SC-011) — **passed** 2026-08-14 on Xvfb + lavapipe; evidence and the two things it could not answer are in [bugs/evidence/BUG-002-visual-pass.md](./bugs/evidence/BUG-002-visual-pass.md)
- [X] T092 `mise run test` and `cargo clippy --workspace --all-targets -- -D warnings` green from the repository root (the `gui` feature no longer exists — the client is its own crate since the workspace split, so the whole workspace is the gate)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (T001)**: no dependencies
- **Foundational (T002–T017)**: depends on Setup — **BLOCKS all user stories**
- **US1 (T018–T026)**: depends on Foundational
- **US2 (T027–T038)**: depends on Foundational; shares the reuse creation path with US1 but is independently testable
- **US3 (T039–T045)**: depends on Foundational
- **US4 (T046–T052)**: depends on Foundational
- **US5 (T053–T057)**: depends on Foundational
- **Polish (T058–T063)**: depends on every story you intend to ship
- **BUG-001 (T064–T072)**: depends on US5
- **BUG-002 Goal A (T073–T074, T079–T081)**: depends on US2 only — the press fix is independent of everything else in this phase and ships on its own
- **BUG-002 Goal B (T075–T078, T082–T090)**: depends on US5 and on BUG-001's holder taxonomy. T082 blocks T083; T083 blocks T084 and T086; T085 blocks T086; T086 blocks T087–T089
- **BUG-002 validation (T091–T092)**: depends on whichever of the two goals is being shipped

### Within Foundational

- Tests T002–T006 are all [P] (five different files) and must be RED before implementation
- T007 (types) blocks T008, T011, T013, T015
- T009 (trait method) blocks T010, T011
- T012 (`CreateError`) blocks T013
- T015 (form fields) blocks T016; T016 blocks T017

### Within each user story

- Tests first, confirmed failing (Principle I)
- `Git` trait method → `create_worktree` dispatch → UI → docs
- The story's user-guide task ships in the same change; the story is not done without it

### ⚠️ Shared-file serialization across stories

`src/ui/worktree_form.rs` is touched by T024, T035, T036, T044, T051, T056; `src/worktree.rs` by T023, T031, T032, T043, T050; `src/git.rs` by T022, T042, T049. **Stories are logically independent but not file-independent** — if two stories are worked in parallel, these tasks must be serialized or the edits will collide. Story phases are safest run in priority order.

### Parallel Opportunities

- All five Foundational test tasks (T002–T006) run in parallel
- Within each story, all test tasks marked [P] run in parallel
- T030 (`src/naming.rs`) is genuinely isolated and parallel with any other US2 task
- T058 and T059 run in parallel

---

## Parallel Example: Foundational tests

```bash
# All five RED-phase test files are independent — write them together:
Task: "CreateMode/rollback_plan tests in tests/worktree_rollback.rs"
Task: "parse_branch_refs tests in tests/git_fake.rs"
Task: "preflight classification tests in tests/branch_conflict.rs"
Task: "create_worktree re-verification tests in tests/worktree_create.rs"
Task: "ResolutionState machine tests in tests/app_state.rs"
```

## Parallel Example: User Story 1 tests

```bash
Task: "ReuseLocal binds existing branch, tip unmoved — tests/worktree_create.rs"
Task: "Reuse rollback preserves the branch — tests/worktree_rollback.rs"
Task: "worktree_add_existing_branch FakeGit semantics — tests/git_fake.rs"
Task: "LocalAvailable raises Choosing, not an error — tests/branch_conflict.rs"
```

---

## Implementation Strategy

### MVP First (User Story 1 only)

1. T001 — baseline green
2. T002–T017 — Foundational (the bulk of the mechanism; blocks everything)
3. T018–T026 — US1
4. **STOP and VALIDATE**: quickstart Scenarios 1, 3, and 7 (7 proves no regression for conflict-free names)
5. Shippable: continuing work started outside the app is now possible — the request's core ask

### Incremental Delivery

1. Setup + Foundational → mechanism ready, conflict-free creation provably unchanged
2. + US1 → reuse works (**MVP**)
3. + US2 → branches become discoverable rather than guessed
4. + US3 → overwrite closes the requested choice
5. + US4 → remote-only branches join the continuation story
6. + US5 → blocked cases stop being confusing failures
7. Polish

### Risk Notes

- **T019 is the highest-stakes test in the feature.** Today's rollback deletes the branch unconditionally; under reuse that destroys the user's commits after a failure they didn't cause. If only one test is reviewed carefully, review that one.
- **T011 has a known trap**: reach for `reconcile()` and the project's own checkout silently disappears from the records, so FR-021's second case never fires. Use the raw `parse_worktrees()` output (research R1).
- **T012 will cascade.** Removing `DuplicateBranch` is intentional — expect and follow the compile errors rather than adding a compatibility variant.

---

## Notes

- [P] = different files, no dependencies on incomplete tasks
- [Story] label maps each task to its user story for traceability
- Verify tests fail before implementing (Red-Green-Refactor, Principle I)
- Commit after each task or logical group
- Stop at any checkpoint to validate a story independently
