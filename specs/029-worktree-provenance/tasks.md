---
description: "Task list for feature 029 — worktree provenance"
---

# Tasks: Worktree Provenance

**Input**: Design documents from `specs/029-worktree-provenance/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY. Every story writes its failing tests before its implementation tasks.

**Documentation**: Per Constitution Principle VII, each story that changes what a user sees updates
`docs/user-guide/worktrees-and-sessions.md` in the same change. Three stories touch that one file,
so those tasks are deliberately **not** `[P]`.

**Cross-platform**: Per Constitution Principle VI, nothing here is platform-specific — a
`BTreeMap` in a JSON file and a path-parent comparison. CI covers all three.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependency on an incomplete task)
- **[Story]**: US1–US5 from [spec.md](./spec.md)

## Path Conventions

Three-crate Rust workspace: `crates/micold-core/` (render-free logic + protocol types),
`crates/micold-daemon/` (single writer of durable state), `crates/micold-client/` (iced GUI +
render-free feature slices). Tests live in each crate's `tests/`. Build and test with
`mise run test-core` (fast loop) and `mise run test` (the gate).

---

## Phase 1: Setup

**Purpose**: Establish a known-good baseline and bound the blast radius of the classifier removal.

- [X] T001 Record a green baseline with `mise run test` so any later failure is attributable to this feature, in the working notes for `specs/029-worktree-provenance/`
- [X] T002 [P] Inventory every call site of `Worktree::owner()`, `Worktree::is_agent_owned()` and `WorktreeOwner` across `crates/` (currently: `crates/micold-core/src/worktree.rs`, `crates/micold-core/tests/worktree_owner.rs`, `crates/micold-core/tests/branch_candidates.rs`, `crates/micold-core/tests/branch_conflict.rs`, `crates/micold-client/src/features/worktree.rs`) and list them in the Notes section at the bottom of this file — T019 removes those methods and every one must have a replacement

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The provenance record exists, persists, and reaches the client. **Nothing classifies on
it yet**, so this phase is behaviour-neutral and independently verifiable: the suite stays green and
the app looks exactly as it does today.

**⚠️ CRITICAL**: No user story can begin until this phase is complete.

### Tests (write first, ensure they FAIL)

- [X] T003 [P] Failing tests for `Workspace` provenance accessors — `is_user_created`, `record_user_created`, `forget_user_created`, idempotence of each, the project key removed when its set empties, and `forget()` dropping both `worktree_provenance` and `provenance_migrated` — in `crates/micold-core/tests/workspace.rs`
- [X] T004 [P] Failing tests for persistence — `created_worktrees` and `provenance_migrated` round-trip through `StoredProjectState`, both default cleanly when absent from an older file, and a save that changes nothing is byte-identical (the set is sorted) — in `crates/micold-core/tests/store_roundtrip.rs`
- [X] T005 [P] Failing test that a corrupt per-project state file puts the project in `unreadable_projects` rather than silently emptying its records — in `crates/micold-core/tests/store_fault_isolation.rs`
- [X] T006 [P] Failing test that `WorktreeSnapshot::user_created` survives both wire encodings and defaults to `false` on an older payload — in `crates/micold-core/tests/protocol_roundtrip.rs`

### Implementation

- [X] T007 Add `worktree_provenance: BTreeMap<PathBuf, BTreeSet<String>>`, `provenance_migrated: BTreeSet<PathBuf>` and `unreadable_projects: BTreeSet<PathBuf>` to `Workspace`, with the three accessors and the `forget()` removals, per [data-model.md](./data-model.md) §1 — in `crates/micold-core/src/workspace.rs`
- [X] T008 Persist the two durable fields as `#[serde(default)] created_worktrees: Vec<String>` and `#[serde(default)] provenance_migrated: bool` on `StoredProjectState`, in both `from_workspace` and the load path, with no schema-version bump — in `crates/micold-core/src/store.rs`
- [X] T009 Mark projects whose `load_project_state()` returns `ProjectStateLoad::Corrupt` in `unreadable_projects` instead of dropping their records silently (`store.rs:633`) — in `crates/micold-core/src/store.rs`
- [X] T010 [P] Add `user_created: bool` to `WorktreeSnapshot` per [contracts/provenance-store.md](./contracts/provenance-store.md) §5 — in `crates/micold-core/src/protocol/messages.rs`
- [X] T011 Fill `user_created` from `worktree_provenance` where `display_name` is already filled from `worktree_names`, in both the durable-only snapshot and the live overlay — in `crates/micold-daemon/src/catalog.rs` (`snapshot`) and `crates/micold-daemon/src/state.rs` (`snapshot_locked`)
- [X] T012 Rebuild `core.workspace.worktree_provenance[active]` from the snapshot's flags — insert when non-empty, remove the key when empty — exactly beside the existing display-name mirror at `catalog_sync.rs:202-212`, in `crates/micold-client/src/catalog_sync.rs`

**Checkpoint**: `mise run test` green; the record round-trips from creation site to client with nothing reading it.

---

## Phase 3: User Story 1 — Assistant session worktrees stop polluting the list (Priority: P1) 🎯 MVP (part 1)

**Goal**: Classification stops reading names and starts reading the record. An unrecorded worktree
under the managed root is hidden, whatever it is called.

**Independent Test**: With the record set seeded directly in a test fixture, a project holding a mix
of recorded and unrecorded worktrees under ordinary names lists exactly the recorded ones, and
`agent-`-named worktrees are decided by their record rather than by their name.

> **Ship note**: this phase inverts the rule but nothing writes records yet, so on its own it hides
> everything. It is independently *testable* (fixtures seed the set) but **US1 + US2 together are the
> MVP** — the spec ranks them co-equal for exactly this reason.

### Tests for User Story 1 (MANDATORY) ⚠️

- [X] T013 [P] [US1] Failing tests for `classify_owner` / `ProvenanceView` covering every row of [contracts/worktree-classification.md](./contracts/worktree-classification.md) §5, plus §4 invariant 6 (two worktrees identical but for a reserved-convention name classify by their record) — in new file `crates/micold-core/tests/worktree_provenance.rs`
- [X] T014 [P] [US1] Failing tests for `matches_reserved_convention` carrying 014's fourteen-row truth table over unchanged, including the 16-vs-15 boundary pair and the case-sensitivity pair — in new file `crates/micold-core/tests/reserved_convention.rs`
- [X] T015 [P] [US1] Failing tests for `visible_worktrees()` under provenance — unrecorded hidden, recorded listed, out-of-root always listed — re-asserting 014's three invariants (superset ordering preserved, identity when revealed, non-destructive) — in new file `crates/micold-client/tests/worktree_visibility.rs`
- [X] T016 [P] [US1] Failing tests that `BlockReason::CheckedOutAt`'s `owner` is decided by provenance rather than by name, so the blocked-branch sentence still names a hidden holder correctly (016 FR-032) — in `crates/micold-core/tests/branch_conflict.rs` and `crates/micold-core/tests/branch_candidates.rs`

### Implementation for User Story 1

- [X] T017 [US1] Add `ProvenanceView<'a> { root, records, state_unreadable }` and `classify_owner(&Worktree, &ProvenanceView) -> WorktreeOwner` with the §2 normative rule — unreadable checked first, then direct-parent-is-root, then record presence — in `crates/micold-core/src/worktree.rs`
- [X] T018 [US1] Rename the private name rule to `pub fn matches_reserved_convention(dir_name, branch) -> bool`, documented as the FR-007a veto whose only caller is the migration, keeping `AGENT_DIR_PREFIX`, `AGENT_BRANCH_PREFIX` and `is_agent_id` — in `crates/micold-core/src/worktree.rs`
- [X] T019 [US1] Remove `Worktree::owner()` and `Worktree::is_agent_owned()` (removed, not deprecated — see contract §1) and delete `crates/micold-core/tests/worktree_owner.rs`, whose corpus now lives in T014's file — in `crates/micold-core/src/worktree.rs`
- [X] T020 [US1] Thread the project's provenance through the holder classification — `classify_holder` (`worktree.rs:639`), `preflight`, `branch_candidates` and `create_worktree`'s re-verification — beside the `included: &[PathBuf]` parameter already threaded through all four for the same reason, in `crates/micold-core/src/worktree.rs`
- [X] T021 [US1] Supply provenance at the daemon's `preflight`, `branch_candidates` and `create_worktree` call sites — in `crates/micold-daemon/src/server.rs`
- [X] T022 [US1] Add a private `State::provenance_view()` for the active project and switch `visible_worktrees()`'s predicate from `!w.is_agent_owned()` to `classify_owner(w, &self.provenance_view()) == WorktreeOwner::User` — in `crates/micold-client/src/features/worktree.rs`
- [X] T023 [US1] Switch `worktree_tags()`'s `Tag::Agent` injection to the same classification, leaving the tag, its label and its non-filterability untouched — in `crates/micold-client/src/features/worktree.rs`
- [X] T024 [US1] Update the client fixtures that construct an agent worktree *by name* so they withhold a record instead — in `crates/micold-client/tests/sidebar_tree.rs` and `crates/micold-client/tests/features_worktree.rs`
- [X] T025 [US1] Rewrite the "Agent worktrees" section for the inverted rule — the app hides worktrees it did not create, assistant session worktrees are therefore hidden too, how to reveal them — in `docs/user-guide/worktrees-and-sessions.md`
- [X] T026 [US1] Walk [quickstart.md](./quickstart.md) Part 2 steps 1–4 and 11 against a scratch repo

**Checkpoint**: Provenance is the sole hiding signal; no name decides anything outside the veto.

---

## Phase 4: User Story 2 — My own worktrees are never hidden, whatever they are called (Priority: P1) 🎯 MVP (part 2)

**Goal**: Every worktree the app creates is recorded, immediately and durably; a storage failure
fails visible; and the one-time migration keeps the worktrees the user already worked in.

**Independent Test**: Create worktrees through the app under a range of names — including one
imitating the reserved machine convention — restart, and confirm every one is still listed. Then make
a project's records unreadable and confirm nothing is hidden.

### Tests for User Story 2 (MANDATORY) ⚠️

- [X] T027 [P] [US2] Failing daemon tests that a record is written for each of the four `CreateMode`s (`NewBranch`, `ReuseLocal`, `Overwrite`, `TrackRemote`) and **not** for a rolled-back create, and that the record is persisted before the catalog broadcast — in new file `crates/micold-daemon/tests/worktree_provenance_rpc.rs`
- [X] T028 [P] [US2] Failing client test that an app-created worktree is listed in the same frame, regardless of whether persistence succeeded (FR-010) — in `crates/micold-client/tests/worktree_visibility.rs`
- [X] T029 [P] [US2] Failing test that a project marked unreadable classifies every one of its worktrees `User`, with a record set both empty and non-empty (FR-011, SC-007) — in `crates/micold-core/tests/worktree_provenance.rs`
- [X] T030 [P] [US2] Failing tests for `durably_known_worktrees()` — a display-name override counts, a worktree-bound session counts, an **archived** session counts, a Default-location session contributes nothing — in new file `crates/micold-core/tests/provenance_migration.rs`
- [X] T031 [P] [US2] Failing tests for `plan_backfill()` — backfills on evidence, skips without it, applies the reserved-convention veto, skips out-of-root, never overwrites an existing record, returns `None` when already migrated, returns `None` and writes no marker when unreadable, and returns `Some(empty)` (which still marks) when it ran and found nothing — in `crates/micold-core/tests/provenance_migration.rs`
- [X] T032 [P] [US2] Failing daemon tests that the migration runs on a project's first `refresh_worktrees()` and never again, and that a session started afterwards in a revealed worktree does not promote it (FR-006c, FR-006d) — in `crates/micold-daemon/tests/worktree_provenance_rpc.rs`

### Implementation for User Story 2

- [X] T033 [US2] Add `Catalog::record_worktree_provenance(project, dir_name)` beside `set_worktree_display_name` — in `crates/micold-daemon/src/catalog.rs`
- [X] T034 [US2] Call it after `create_worktree` returns `Ok` and **before** `refresh_worktrees_and_broadcast`, so the first snapshot the client sees already carries `user_created: true` — in `crates/micold-daemon/src/server.rs` (`WorktreeCreate` arm, ~line 903)
- [X] T035 [US2] Record provenance optimistically in the client's `created()` reducer so the row is user-owned in the frame it appears (FR-010) — in `crates/micold-client/src/features/worktree.rs`
- [X] T036 [US2] Honour `unreadable_projects` in `State::provenance_view()`, setting `state_unreadable` for the active project — in `crates/micold-client/src/features/worktree.rs`
- [X] T037 [US2] Extract `durably_known_worktrees(overrides, sessions)` and rewrite `Catalog::snapshot()`'s inline expression (`catalog.rs:155-168`) to call it, keeping its own `!archived` filter at its own call site — in `crates/micold-core/src/worktree.rs` and `crates/micold-daemon/src/catalog.rs`
- [X] T038 [US2] Implement `plan_backfill()` as a pure function returning the records to write, per [contracts/provenance-store.md](./contracts/provenance-store.md) §4 — in `crates/micold-core/src/worktree.rs`
- [X] T039 [US2] Apply the returned plan and set `provenance_migrated` for the project in a **single** persist, so a crash cannot leave records without the marker or vice versa — in `crates/micold-daemon/src/catalog.rs`
- [X] T040 [US2] Invoke the migration from `DaemonState::refresh_worktrees()` immediately after discovery populates `Inner::worktrees`, skipping unreadable and already-migrated projects — in `crates/micold-daemon/src/state.rs` (~line 799)
- [X] T041 [US2] Document what happens at the upgrade and what to expect for a worktree made by hand outside the app — in `docs/user-guide/worktrees-and-sessions.md`
- [X] T042 [US2] Walk [quickstart.md](./quickstart.md) Part 3 steps 1–6, including the corrupt-store case

**Checkpoint**: The MVP. Assistant session worktrees are gone from the list; nothing the user made through the app, or had already worked in, has disappeared.

---

## Phase 5: User Story 3 — Everything 014 established still works (Priority: P2)

**Goal**: The wider hidden set inherits 014's behaviour intact, and the reveal control says how many
worktrees it is withholding.

**Independent Test**: Run 014's own acceptance scenarios for hiding, reveal, the chip, row actions
and the project-switch reset against provenance fixtures; then confirm the count matches the number
of rows the control reveals.

### Tests for User Story 3 (MANDATORY) ⚠️

- [X] T043 [P] [US3] Failing tests for `hidden_worktree_count()` — zero while the control is on, zero when nothing is hidden, and equal to the number of rows switching it on adds (FR-025/025a/025b) — in `crates/micold-client/tests/worktree_visibility.rs`
- [X] T044 [P] [US3] Failing tests that `ToggleChip::count(0)` renders identically to a chip with no count, and that an un-counted chip is unchanged by this feature — in new file `crates/micold-client/tests/toggle_chip_count.rs`
- [X] T045 [P] [US3] Re-assert 014's suite against provenance fixtures — `show_agent_worktrees` defaults false, the toggle's sole mutation, the project-switch reset, the `agent` chip on a revealed row, full row actions, tag filters applying to revealed rows — in `crates/micold-client/tests/app_state.rs` and `crates/micold-client/tests/sidebar_tree.rs`
- [X] T046 [P] [US3] Failing tests that the project root ("Default") is never classified (FR-017) and that an all-hidden project shows "No worktrees yet" rather than the filter empty-state — in `crates/micold-client/tests/worktree_visibility.rs`

### Implementation for User Story 3

- [X] T047 [US3] Add `State::hidden_worktree_count()` defined as `worktrees.len() - visible_worktrees().count()`, so FR-025b holds by construction rather than by two filters agreeing — in `crates/micold-client/src/features/worktree.rs`
- [X] T048 [US3] Add the chainable `.count(usize)` builder step rendering a trailing `· N`, nothing at zero, themed from the same `Roles`, leaving un-counted chips pixel-identical — in `crates/micold-client/src/ui/material/toggle_chip.rs`
- [X] T049 [US3] Pass `state.hidden_worktree_count()` into `reveal_chip()`, keeping the label string `"Show agent worktrees"` unchanged (FR-015a) and its placement above `filter_bar()`'s early return unchanged (014 FR-010c) — in `crates/micold-client/src/ui/sidebar.rs`
- [X] T050 [US3] Confirm `row_actions_cluster()` is untouched by the whole feature diff so far — a change there is the signal the implementation drifted into special-casing agent rows, which 014 FR-013 and 029 FR-015 both forbid — in `crates/micold-client/src/ui/sidebar.rs`
- [X] T051 [US3] Document what the count beside the reveal control means — in `docs/user-guide/worktrees-and-sessions.md`
- [X] T052 [US3] Walk [quickstart.md](./quickstart.md) Part 2 steps 3–5 and 8

**Checkpoint**: SC-003 holds — every 014 scenario still passes — and SC-010 is reachable from the sidebar alone.

---

## Phase 6: User Story 4 — A deleted worktree's name can be reused safely (Priority: P3)

**Goal**: A record dies with its worktree, so a later worktree reusing the directory name inherits
nothing.

**Independent Test**: Create a worktree in the app, delete it in the app, recreate a directory of the
same name from a terminal, and confirm it is hidden.

### Tests for User Story 4 (MANDATORY) ⚠️

- [X] T053 [P] [US4] Failing daemon tests that a successful delete removes the record, that a failed (git-refused) delete leaves it alone, and that a later same-named worktree classifies `Agent` — in `crates/micold-daemon/tests/worktree_provenance_rpc.rs`
- [X] T054 [P] [US4] Failing test that forgetting a project discards its provenance and its migration marker with its other per-project records — in `crates/micold-client/tests/forget_project.rs`
- [X] T055 [P] [US4] Failing test that `set_worktrees()`'s pruning does **not** drop the provenance record of a hidden worktree, exactly as 014 required of its rename override — in `crates/micold-client/tests/worktree_visibility.rs`

### Implementation for User Story 4

- [X] T056 [US4] Add `Catalog::forget_worktree_provenance(project, dir_name)` beside `forget_worktree_name` and call it from the `WorktreeDelete` handler only after git removal succeeds — in `crates/micold-daemon/src/catalog.rs` and `crates/micold-daemon/src/server.rs` (~line 1077)
- [X] T057 [US4] Confirm `forget_project` disposes of provenance and the marker via `Workspace::forget()` plus `remove_project_state()` — in `crates/micold-daemon/src/catalog.rs`
- [X] T058 [US4] Confirm the client's pruning path reasons about existence, not visibility, and leaves both the rename override and the provenance record of a hidden worktree alone — in `crates/micold-client/src/app.rs`
- [X] T059 [US4] Walk the create → delete → recreate-outside-the-app sequence from [quickstart.md](./quickstart.md) Part 2 step 10 and confirm the new worktree is hidden

**Checkpoint**: The record set holds no lies; a reused name is classified on its own merits.

---

## Phase 7: User Story 5 — Claim a worktree the app did not create (Priority: P3)

**Goal**: One action on a revealed row makes any worktree the user's own, permanently.

**Independent Test**: Create a worktree outside the app under the managed root, confirm it is hidden,
claim it from a revealed row, switch reveal off, restart, and confirm it is still listed.

### Tests for User Story 5 (MANDATORY) ⚠️

- [X] T060 [P] [US5] Failing test that `ClientMsg::WorktreeClaim` round-trips both wire encodings — in `crates/micold-core/tests/protocol_roundtrip.rs`
- [X] T061 [P] [US5] Failing daemon tests that a claim acks, persists and broadcasts; is honoured for a reserved-convention name (FR-023); is idempotent; and reports `IoFailed` when persistence fails — in new file `crates/micold-daemon/tests/worktree_claim.rs`
- [X] T062 [P] [US5] Failing daemon test that a claim performs no git or filesystem operation — the worktree directory and its branch are unchanged before and after (FR-003, US5 scenario 4) — in `crates/micold-daemon/tests/worktree_claim.rs`
- [X] T063 [P] [US5] Failing client tests for the reducer — the row is user-owned in the same frame, loses the `agent` chip, remains listed once reveal is switched off, and the reducer mutates only `worktree_provenance` and `menu_open` — in new file `crates/micold-client/tests/worktree_claim.rs`
- [X] T064 [P] [US5] Failing test that no un-claim exists: no `WorktreeUnclaim` variant, no menu entry, and no path removes a record except delete and forget (FR-024) — in `crates/micold-client/tests/worktree_claim.rs`

### Implementation for User Story 5

- [X] T065 [US5] Add `ClientMsg::WorktreeClaim { req, project, dir_name }` per [contracts/claim-and-reveal.md](./contracts/claim-and-reveal.md) §1 — in `crates/micold-core/src/protocol/messages.rs`
- [X] T066 [US5] Add `Catalog::claim_worktree(project, dir_name)` writing the same record T033 writes — in `crates/micold-daemon/src/catalog.rs`
- [X] T067 [US5] Add the `WorktreeClaim` arm modelled on `WorktreeRename` (`server.rs:1250`) — persist, broadcast, `Ack`, with `IoFailed` as its only error case — in `crates/micold-daemon/src/server.rs`
- [X] T068 [US5] Add `Msg::ClaimRequested(String)` and its reducer — optimistic record, close the row menu, touch nothing else — in `crates/micold-client/src/features/worktree.rs`
- [X] T069 [US5] Match `Msg::ClaimRequested` a second time in the effect half to send `WorktreeClaim`, following the split `RenameConfirmed` establishes — in `crates/micold-client/src/main.rs`
- [X] T070 [US5] Add the `Claim as mine` entry to the worktree row's right-click menu, offered only when the row classifies `Agent`, leaving `row_actions_cluster()` untouched — in `crates/micold-client/src/ui/sidebar.rs`
- [X] T071 [US5] Document claiming — how to reach a hand-made worktree and keep it listed — in `docs/user-guide/worktrees-and-sessions.md`
- [X] T072 [US5] Walk [quickstart.md](./quickstart.md) Part 2 steps 5, 7 and 9

**Checkpoint**: Every misclassification, including anything the migration got wrong, is recoverable in one in-app action.

---

## Phase 8: Polish & Cross-Cutting Concerns

- [X] T073 [P] Add a superseded-by note at the top of `specs/014-hide-agent-worktrees/contracts/agent-worktree-classification.md` pointing at [contracts/worktree-classification.md](./contracts/worktree-classification.md), so a later reader does not implement the deleted rule
- [X] T074 [P] Cross-cutting docs review — the "Agent worktrees" section reads as one coherent story after three stories edited it, and the user-guide index still points at it — in `docs/user-guide/`
- [X] T075 Confirm SC-005 by inspecting the whole diff — no `fs` or `Git` call added on any provenance path (establish, read, migrate, claim, hide) — concentrating on `crates/micold-daemon/src/server.rs`, `crates/micold-daemon/src/catalog.rs` and `crates/micold-core/src/worktree.rs`
- [X] T076 Confirm SC-006 — opening a project and rendering its worktree list is no slower — by checking that the classification added to `crates/micold-client/src/features/worktree.rs` stays one `BTreeSet` lookup per worktree per refresh, with no allocation or path canonicalisation per row
- [X] T077 Run the local gate in full: `mise run test`, `cargo clippy --workspace --all-targets`, **and `cargo fmt --check`** — the local gate is easy to pass while CI stops at formatting, which gates every other job
- [X] T078 Confirm CI is green on Linux, macOS and Windows per `.github/workflows/ci.yml` (Principle VI) — this change affects what is built, so no documentation-only exemption applies
- [X] T079 Run [quickstart.md](./quickstart.md) end to end — Parts 1, 2 and 3 — recording Part 2 step 6 and Part 3 step 6, the two inversions of 014's behaviour a regression would silently undo

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies.
- **Foundational (Phase 2)**: depends on Setup. **Blocks every user story** — nothing can classify on
  a record that does not exist or persist.
- **US1 (Phase 3)** and **US2 (Phase 4)**: both depend only on Foundational. They are the two halves
  of the MVP and are *ordered* in practice (see below) even though each is independently testable.
- **US3 (Phase 5)**: depends on US1 — the count is a count of the hidden set.
- **US4 (Phase 6)**: depends on US2 — a record must be written before removing it means anything.
- **US5 (Phase 7)**: depends on US1 (a revealed row to act on) and on Foundational's record. It does
  **not** depend on US2, US3 or US4.
- **Polish (Phase 8)**: depends on every story that is being shipped.

### The one ordering that is not optional

**US1 must not ship without US2.** US1 inverts the rule; US2 supplies the records the inverted rule
reads. Landing US1 alone hides every worktree in every project. The spec ranks them co-equal P1 for
this reason, and Phase 4's checkpoint — not Phase 3's — is the first shippable state.

### Within each story

- Tests are written and failing before implementation (Principle I).
- Core (`micold-core`) before daemon before client, since the client's classification calls the core
  function and the daemon fills the flag the client mirrors.
- The user-guide task ships with its own story (Principle VII); the three doc tasks (T025, T041,
  T051, T071) all edit one file and are therefore sequential.

### Parallel Opportunities

- T003–T006 (four test files, four crates' worth of concerns) run in parallel.
- T013–T016 run in parallel; so do T027–T032, T043–T046, T053–T055 and T060–T064.
- T010 is parallel with T007–T009 (different file).
- **US5 can be built in parallel with US2, US3 and US4** once US1 lands — it shares only the record
  writer, and its files are otherwise disjoint.
- The doc tasks are the one place parallelism is deliberately refused: one file, four edits.

---

## Parallel Example: User Story 2

```bash
# All six failing tests first, in parallel — they touch four different files:
Task: "plan_backfill() cases in crates/micold-core/tests/provenance_migration.rs"
Task: "durably_known_worktrees() cases in crates/micold-core/tests/provenance_migration.rs"
Task: "unreadable project fails visible in crates/micold-core/tests/worktree_provenance.rs"
Task: "record per CreateMode in crates/micold-daemon/tests/worktree_provenance_rpc.rs"
Task: "migration runs once in crates/micold-daemon/tests/worktree_provenance_rpc.rs"
Task: "created worktree visible immediately in crates/micold-client/tests/worktree_visibility.rs"

# Then the core implementation, before anything daemon-side depends on it:
Task: "durably_known_worktrees() in crates/micold-core/src/worktree.rs"
Task: "plan_backfill() in crates/micold-core/src/worktree.rs"
```

---

## Implementation Strategy

### MVP (User Story 1 + User Story 2)

1. Phase 1: Setup.
2. Phase 2: Foundational — **blocks everything**.
3. Phase 3: US1 — the rule inverts.
4. Phase 4: US2 — records are written and the migration runs.
5. **STOP and VALIDATE**: quickstart Part 2 steps 1–9 and Part 3 in full. This is the first state
   that is safe to run against a real project.

### Incremental delivery after the MVP

1. US3 → 014's behaviour re-verified and the hidden count on screen → the feature is honest about
   what it is withholding.
2. US4 → the record set stops being able to accumulate lies.
3. US5 → every misclassification becomes recoverable in one action.

Each adds value without touching the previous one's surfaces: US3 is the sidebar chip, US4 is the
delete path, US5 is a new row action and a new message.

### Parallel team strategy

With Foundational done, US1 is the critical path and everything else queues behind it. Once US1
lands, one developer can take US2 (daemon-heavy: creation, migration) while another takes US5
(protocol + reducer + menu) and a third takes US3 (client-only: the count and the widget). US4 is
small and belongs with whoever holds US2, since both touch the daemon's catalog.

---

## Notes

- `[P]` means a different file and no dependency on an incomplete task.
- Verify each test fails before implementing it — a provenance test that passes against the old
  name-based rule is testing the wrong thing.
- Commit after each task or logical group.
- **T002's inventory belongs here**: record the `Worktree::owner()` / `is_agent_owned()` /
  `WorktreeOwner` call sites found, so T019's removal can be checked off against a list rather than
  against the compiler alone.
- The two tasks most likely to be skipped and most expensive to skip: **T050** (`row_actions_cluster()`
  untouched) and **T075** (no `fs`/`Git` call added). Both are stated as *confirmations* precisely
  because their failure mode is a diff that looks reasonable.

### T042 found an FR-011 defect, and `store.rs` changed after T077 was first marked done

Part 3 step 6's second leg — restart after the corrupt run, restoring nothing — showed
`Show agent worktrees · 6` over "No worktrees yet": every worktree in the project hidden, which is
the outcome FR-011 exists to prevent.

`load_project_state` renamed a corrupt per-project file aside to `<id>.json.bak`. Two processes read
the same store — `crates/micold-client/src/shell/startup.rs:129` and the daemon the client spawns —
so the second reader found a merely *missing* file, marked nothing unreadable, ran the one-time
FR-006 backfill against evidence that had just been discarded, and wrote `provenance_migrated: true`
over an empty record set.

Fixed test-first in `crates/micold-core/src/store.rs`: the corrupt file is left exactly as found, and
`save()` skips any project in `unreadable_projects` rather than writing back the empty shape the
failed read degraded to. Two new tests in `crates/micold-core/tests/store_fault_isolation.rs` went
red first. **T077 was therefore re-run in full after the fix** — `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`
(2719 passed, 0 failed) — and Part 3 step 6 re-walked against rebuilt binaries. The whole episode is
recorded in [visual-pass.md](./visual-pass.md).

### Where three tasks landed, against where they were planned

Three tasks named a file the work did not end up in. Recorded so a later reader checking the diff
against this list does not conclude they were skipped.

| Task | Planned file | Actual file | Why |
|---|---|---|---|
| T050 | `ui/sidebar.rs` | *(confirmation only)* | `row_actions_cluster()` is untouched: the whole-feature diff of `ui/sidebar.rs` is one hunk, the `.count(...)` step on `reveal_chip`. Confirmed by `git diff crates/micold-client/src/ui/sidebar.rs`, which mentions the cluster nowhere. |
| T070 | `ui/sidebar.rs` | `ui/mod.rs` | The worktree row's right-click menu is built in `ui/mod.rs`, not in the sidebar module the plan assumed. `Claim as mine` sits there, gated on the row classifying `Agent`. |
| T076 | `features/worktree.rs` | *(confirmation only)* | SC-006 holds by construction: `classify_owner` is one `BTreeSet<String>::contains(&str)` over a borrowed set, and `provenance_view()` borrows `workspace.user_created_worktrees(..)` rather than cloning. No allocation and no path canonicalisation per row; the only per-refresh cost is the `active_project()` lookup, once per call. |

### T063's row lookup: `dir_name`, never `display_name`

`worktree_claim.rs`'s `node_tags` helper first looked a row up by `WorktreeNode::display_name`, which
is the friendly prose *derived* from the directory name — `hand-made` renders as `Hand made`, so the
lookup found nothing and the test failed with `no row for hand-made` rather than with anything about
chips. The identity on a node is `worktree.dir_name`; a fixture that keys on the display name is
asserting against a string the app is free to re-word.

### Beyond T024: the fixtures that construct an *ordinary* worktree

T024 anticipated the fixtures that build an **agent** worktree by name and have to withhold a record
instead. The inversion also reaches the opposite case, which the task list did not name: any fixture
that fills `worktree.worktrees` and renders the sidebar now draws **no rows at all**, because an
unrecorded worktree inside the managed root is hidden (FR-004). Four places needed the record added:

| File | What it renders |
| --- | --- |
| `crates/micold-client/tests/support/covered_states.rs` | every covered state with a project open — `with_project()`, plus the six states that replace the whole `workspace` afterwards and so discard its records |
| `crates/micold-client/tests/gates/containment.rs` | the thirty-worktree sidebar whose overflow proves `SCROLL_CONTENT` is the list |
| `crates/micold-client/tests/gates/context_menu_anchor.rs` | the eight-row sidebar this gate right-presses |
| `crates/micold-client/tests/layout_text_overflow.rs` | the one session row the CLI-label gate measures |

The failure is worth recording because of how it presents: not as "provenance is wrong" but as
`no node at 0/0/0/1/0/0/0/2/0/0/8 to press`, `a sidebar holding thirty worktrees does not lay it
outside its viewport`, and a painted-text list ending in "No worktrees yet." Three gates reporting
their own subject matter, one cause. The fixtures now say what they always meant — these are
worktrees made through the app — rather than relying on a rule that read the name.

### T001 — the green baseline

This feature branched from `851af342` ("feat(027): reach the users the namespace correction could
not"), and that commit's CI run is green on every job that gates a merge — `fmt + clippy`, `build +
test` on ubuntu-latest, macos-latest and windows-latest, `docs check`, `assertion freeze`, and the
real-runtime sandbox suite on Linux. So any red in this branch is this feature's, which is the only
thing the baseline is for.

CI is used rather than a local `mise run test` deliberately: the local gate omits `cargo fmt
--check`, and it runs on one platform out of three. A baseline that a later failure is attributed
*against* should cover at least as much as the run that will find the failure.

### T002 — call-site inventory (as of `851af342`, before this feature)

Every use of `Worktree::owner()`, `Worktree::is_agent_owned()` or `WorktreeOwner` that existed when
this feature started, with what T019's removal replaced it by:

**`Worktree::owner()` / `Worktree::is_agent_owned()` — the two methods T019 removes**

| Call site | Replacement |
| --- | --- |
| `crates/micold-core/src/worktree.rs:146,152` (the definitions) | Removed; `classify_owner(&Worktree, &ProvenanceView)` is the only classifier (T017) |
| `crates/micold-core/src/worktree.rs:153` (`is_agent_owned` → `owner`) | Gone with both methods |
| `crates/micold-client/src/features/worktree.rs:117` (`worktree_tags`, `Tag::Agent`) | `classify_owner(worktree, provenance) == WorktreeOwner::Agent` (T023) |
| `crates/micold-client/src/features/worktree.rs:150` (`visible_worktrees`) | `classify_owner(w, &self.provenance_view()) == WorktreeOwner::User` (T022) |
| `crates/micold-core/tests/worktree_owner.rs` (18 call sites across 14 tests) | File deleted; its corpus is `tests/reserved_convention.rs` against `matches_reserved_convention` (T014/T019) |

**`WorktreeOwner` the enum — kept, and still reached from `BlockReason::CheckedOutAt`**

| Call site | Outcome |
| --- | --- |
| `crates/micold-core/src/worktree.rs:57` (the definition) | Unchanged — both variants keep their meaning |
| `crates/micold-core/src/worktree.rs:73` (private `classify_owner(dir_name, branch)`) | Renamed `matches_reserved_convention` returning `bool`, now the migration's veto only (T018) |
| `crates/micold-core/src/worktree.rs:346` (`BlockReason::CheckedOutAt { owner }`) | Unchanged; its `owner` is now filled from provenance (T020) |
| `crates/micold-core/src/worktree.rs:374,381,475` (`classify_holder`, `create_worktree`'s re-verification) | Provenance threaded in beside `included` (T020) |
| `crates/micold-core/tests/branch_candidates.rs:6,88,127` | Fixtures pass a `ProvenanceView`; expectations unchanged (T016) |
| `crates/micold-core/tests/branch_conflict.rs:10,18,150,181,363,386,704` | Same, via an `app_made()` fixture (T016) |

Verified after T019: `grep -rn -E 'is_agent_owned|\.owner\(\)' crates --include=*.rs` matches only
the explanatory comment left at `crates/micold-core/src/worktree.rs:350`.
