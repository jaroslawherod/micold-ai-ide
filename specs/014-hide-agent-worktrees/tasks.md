---

description: "Task list for Hide Agent Worktrees"
---

# Tasks: Hide Agent Worktrees

**Input**: Design documents from `/specs/014-hide-agent-worktrees/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md), [data-model.md](./data-model.md), [contracts/](./contracts/)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are MANDATORY. Every user story writes its failing tests FIRST (Red), then implements to green.

**Documentation**: Per Constitution Principle VII, the user-facing stories (US1, US4) carry their own user-guide task in the same change.

**Cross-platform**: Per Constitution Principle VI, everything here is name-string logic with no OS branching; CI must stay green on Linux, macOS, and Windows.

**Test command**: `mise run test` (`cargo test --no-default-features --all-targets`) — the render-free core, matching CI.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (US1, US2, US3, US4)
- Include exact file paths in descriptions

## Path Conventions

Single Rust crate at repository root: render-free core in `src/`, `gui`-only binary code in
`src/ui/`, integration tests in `tests/`, user guide in `docs/user-guide/`.

---

## Phase 1: Setup

**Purpose**: Establish a known-green starting point. There is no scaffolding to create — this
feature lands entirely inside an existing crate.

- [X] T001 Trust the repo config once in this worktree and record a green baseline by running `mise trust` then `mise run test` from the repository root (per `mise.toml`); note the pass count so later Red states are unambiguous

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The ownership type and method surface every user story depends on. Deliberately
*behavior-free*. T003's method bodies are **compile-only stubs**: Rust cannot express a Red test
against a method that does not exist, so the stub is what lets US1's T004 fail rather than
fail-to-build. The first real behavior arrives in T006, under an already-failing test
(Constitution Principle I, Red-Green-Refactor).

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T002 Add the `WorktreeOwner { User, Agent }` enum in `src/worktree.rs`, deriving `Debug, Clone, Copy, PartialEq, Eq` to match the neighbouring `WorktreeStatus`, with a doc comment stating it is derived from names only and never stored (data-model.md § Worktree ownership)
- [X] T003 Add `Worktree::owner()` and `Worktree::is_agent_owned()` in `src/worktree.rs` as compile-only stubs returning `WorktreeOwner::User` — a placeholder that exists so T004's test compiles and is observed failing, NOT a behavior decision — carrying the doc comment that records the `reconcile()` location precondition from contracts/agent-worktree-classification.md (depends on T002)

**Checkpoint**: The type exists and the crate compiles; every story can now build on it

---

## Phase 3: User Story 1 - The worktree list shows only my worktrees (Priority: P1) 🎯 MVP

**Goal**: Agent-owned worktrees stop appearing in the sidebar, with nothing on disk touched.

**Independent Test**: Open a project containing a mix of user-created and agent-owned worktrees and
confirm the sidebar lists exactly the user-created set.

**⚠️ Ships with US2**: this phase implements the *positive* half of the rule only, so between US1
and US2 the classifier deliberately over-matches (a worktree literally named `agent-foo` would be
hidden). US2 adds the guard. The spec's own priority rationale sanctions this ordering — "this
guard must exist, but it is only meaningful once P1 hides anything at all" — but the two stories
MUST land in the same release.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T004 [P] [US1] Create `tests/worktree_owner.rs` with the positive rows of the truth table in contracts/agent-worktree-classification.md — both identifiers matching, dir-only (branch `None`), and branch-only — asserting `owner()` returns `Agent`
- [X] T005 [P] [US1] Extend `tests/sidebar_tree.rs`: a project of 3 user worktrees + 3 agent worktrees yields only the 3 user rows from `worktree_tree()`; a project whose worktrees are all agent-owned yields zero worktree nodes; and `sidebar_entries()` contains no agent `dir_name` — pinning FR-004, since those rows are the only entry point to start-session, rename, and delete, a claim research R8 argues but nothing currently asserts

### Implementation for User Story 1

- [X] T006 [US1] Implement the positive classification rule in `Worktree::owner()` in `src/worktree.rs` — `dir_name` starting `agent-`, or `branch` starting `worktree-agent-` (length and hex constraints arrive in US2)
- [X] T007 [US1] Add the transient `show_agent_worktrees: bool` field (default `false`, documented as never persisted) and the `visible_worktrees()` iterator to `State` in `src/app.rs`, per contracts/agent-worktree-classification.md § State visible-set API
- [X] T008 [US1] Rebase `worktree_tree()` onto `visible_worktrees()` in `src/app.rs`, leaving `self.worktrees` holding every discovered worktree (depends on T007)
- [X] T009 [P] [US1] Add the "Agent worktrees" section to `docs/user-guide/worktrees-and-sessions.md` covering what they are, that the app hides them by default, and that their lifecycle belongs to the agent — using the word "agent", never "assistant" (FR-012, spec § Terminology)

**Checkpoint**: Agent worktrees are gone from the sidebar; `git worktree list` is unchanged

---

## Phase 4: User Story 2 - My own worktrees are never hidden by mistake (Priority: P2)

**Goal**: The hiding rule is precise enough that no user-created worktree can be caught by it.

**Independent Test**: Create worktrees whose names share the reserved prefix but not the
machine-generated identifier shape, and confirm every one of them remains listed.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation** — T010 and T011 fail
> against US1's loose rule, which is exactly the Red this story exists to fix.

- [X] T010 [P] [US2] Add the negative and boundary rows of the truth table to `tests/worktree_owner.rs`: `agent-foo`, `agent-face`, a non-hex tail after a long hex run, `feat-1234-agent-runner`, bare `agent-`, `worktree-agent-…` in the *directory* position, uppercase hex accepted, uppercase prefix rejected, and both sides of the 16/15-character boundary
- [X] T011 [P] [US2] Extend `tests/sidebar_tree.rs` with a corpus of user-created worktrees whose names share the reserved prefix, asserting all of them survive into `worktree_tree()`
- [X] T012 [P] [US2] Extend `tests/worktree_discovery.rs` to assert `reconcile()` still excludes worktrees outside the project's `.claude/worktrees/` root regardless of their names, so the classification rule is never consulted for them (US2 acceptance #4)

### Implementation for User Story 2

- [X] T013 [US2] Tighten `Worktree::owner()` in `src/worktree.rs` to require an agent id of **at least 16 characters, every one `is_ascii_hexdigit`**, with the prefix matched case-sensitively (FR-005/FR-006, research R2)

**Checkpoint**: The classifier matches the real generator and nothing else; US1 + US2 are releasable together

---

## Phase 5: User Story 3 - Everything derived from the list stays consistent (Priority: P3)

**Goal**: Counts, filter chips, the empty state, and action targets all agree with the visible list.

**Independent Test**: With agent worktrees present, exercise filtering and the empty state and
confirm no count, chip, or result set reflects a hidden worktree.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T014 [P] [US3] Extend `tests/app_state.rs`: `available_tag_filters()` offers no chip derived from a hidden worktree (in particular no phantom `Untyped`), and a rename override for a hidden worktree survives `set_worktrees()` because pruning still runs against the full set
- [X] T015 [P] [US3] Extend `tests/sidebar_tree.rs`: agent worktrees with `WorktreeStatus::Missing` and with `Invalid` (an orphan directory git does not know) are hidden rather than surfacing as broken entries (FR-007, US3 acceptance #3)
- [X] T016 [P] [US3] Extend `tests/session_lifecycle.rs`: a session recorded against a now-hidden worktree renders nowhere, is not pruned from the store, and is still returned by `sessions_in_worktree()` — asserting the absence of any dedicated handling (FR-011, research R8)

### Implementation for User Story 3

- [X] T017 [US3] Rebase `available_tag_filters()` onto `visible_worktrees()` in `src/app.rs` (research R7)
- [X] T018 [US3] Add a `has_visible_worktrees()` (or visible-count) accessor to `State` in `src/app.rs` so the sidebar's empty-state decision lives in the testable core rather than in the `gui`-only binary
- [X] T019 [US3] Replace the `state.worktrees.is_empty()` check at `src/ui/sidebar.rs:102` with the T018 accessor, so an agent-only project shows "No worktrees yet. Add one to get started." instead of the misleading "No worktrees match the filter." (depends on T018)

**Checkpoint**: Nothing downstream of the list disagrees with it

---

## Phase 6: User Story 4 - Reveal them on demand (Priority: P4)

**Goal**: A "Show agent worktrees" chip in the filter accordion brings the hidden rows back,
badged and fully actionable, for the current project in the current run.

**Independent Test**: With agent worktrees present, toggle the reveal control on and off and
confirm the list gains and loses exactly those entries.

### Tests for User Story 4 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T020 [P] [US4] Extend `tests/app_state.rs`: `show_agent_worktrees` defaults to `false`; `Message::ShowAgentWorktreesToggled` flips only that field and leaves `sidebar_filters`, `expanded`, and `overlay` untouched (FR-010d); two toggles restore the prior list; and a project switch resets it to `false` (FR-010e)
- [X] T021 [P] [US4] Extend `tests/sidebar_tree.rs`: with the flag on, `worktree_tree()` includes agent worktrees, each carrying `Tag::Agent`, in unchanged `dir_name` order; and active tag filters apply to revealed rows exactly as to user-created ones

### Implementation for User Story 4

- [X] T022 [US4] Add `Message::ShowAgentWorktreesToggled` and its reducer (sole mutation: flip the flag) in `src/app.rs`, per contracts/sidebar-reveal-control.md § Reducer
- [X] T023 [US4] Reset `show_agent_worktrees = false` in `restore_after_activation()` in `src/app.rs`, immediately alongside the existing `default_expanded = false` and for the same reason (FR-010e, invariant 2a) (depends on T022)
- [X] T024 [US4] Add the `Tag::Agent` variant in `src/naming.rs`, documented as label-only with deliberately no `TagFilter` counterpart (research R5), and add the arm it forces at **both** exhaustive match sites so the crate still compiles: `Tag::Agent => {}` in `available_tag_filters()` at `src/app.rs:1537` (T027 covers the second site, `tag_chip()` in `src/ui/sidebar.rs`). The empty arm is correct, not a stub — a revealed agent worktree carries no `Type` tag and therefore still sets `has_untyped`, the FR-010d-consistent behavior data-model.md § Interaction with existing tag logic already sanctions
- [X] T025 [US4] Append `Tag::Agent` in `State::worktree_tags()` in `src/app.rs` when `worktree.is_agent_owned()`, beside the existing `Tag::Status` append (depends on T024)
- [X] T026 [P] [US4] Create the shared `ToggleChip` builder in `src/ui/material/toggle_chip.rs` and export it from `src/ui/material/mod.rs`, matching the existing chip's padding, radius, text size, and active/inactive treatment exactly (contracts/sidebar-reveal-control.md § ToggleChip)
- [X] T027 [US4] Add the `Tag::Agent` arm to `tag_chip()` in `src/ui/sidebar.rs` — label `"agent"`, accent `roles.on_surface_variant` (depends on T024)
- [X] T028 [US4] Rewrite `filter_chip()` in `src/ui/sidebar.rs` to delegate to `ToggleChip`, deleting the duplicated button styling so one primitive serves both call sites (Principle VIII Component-reuse gate) (depends on T026)
- [X] T029 [US4] Render the reveal chip labelled "Show agent worktrees" as the first element of the filter accordion body in `src/ui/sidebar.rs` — **above** `filter_bar()` and outside its `available.is_empty()` early return, so it stays reachable in an agent-only project (FR-010c, research R4) (depends on T026, T022)
- [X] T030 [US4] Confirm `row_actions_cluster()` in `src/ui/sidebar.rs` is untouched by this feature, so a revealed row keeps start-session, rename, and delete with no agent-specific gating or extra confirmation (FR-013)
- [X] T031 [P] [US4] Extend the user-guide section in `docs/user-guide/worktrees-and-sessions.md` with how to reveal agent worktrees from the filter panel, that the choice resets on restart and on project switch, and that revealed rows are fully actionable (FR-012)

**Checkpoint**: All four stories functional; the escape hatch works and defaults safely

---

## Phase 7: Polish & Cross-Cutting Concerns

- [X] T032 [P] Sweep every string this feature adds — the chip label, the badge, and the user-guide section — confirming all say "agent" and none says "assistant" (spec § Terminology, contracts/sidebar-reveal-control.md § Terminology)
- [X] T033 Run the full automated gate `mise run test` from the repository root and confirm the pre-existing worktree suites (`worktree_model`, `worktree_discovery`, `sidebar_state`, `session_lifecycle`) needed no edits beyond those listed above — `Worktree` gained methods but no fields
- [ ] T034 Execute the manual procedure in [quickstart.md](./quickstart.md) Part 2 against a scratch repo, including the post-run `git worktree list` / branch diffs that prove nothing on disk changed (SC-005, FR-008)
- [ ] T035 Confirm CI is green on Linux, macOS, and Windows (Constitution Principle VI, Cross-platform gate)
- [X] T036 [P] Update `docs/user-guide/` navigation/index if the new section warrants an entry, and re-check the docs build in CI (Principle VII Documentation gate)

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: no dependencies
- **Foundational (Phase 2)**: depends on Setup — BLOCKS all user stories
- **US1 (Phase 3)**: depends on Foundational
- **US2 (Phase 4)**: depends on US1's `owner()` implementation (T006) — it tightens that same function
- **US3 (Phase 5)**: depends on US1's `visible_worktrees()` (T007); independent of US2 and US4
- **US4 (Phase 6)**: depends on US1's flag + accessor (T007); independent of US2 and US3
- **Polish (Phase 7)**: depends on all desired stories

### User Story Dependencies

This feature departs from the usual "all stories independent" shape in exactly one place, and it is
deliberate:

- **US1 (P1)**: independent once Foundational lands. The MVP.
- **US2 (P2)**: **sequentially depends on US1** — it edits the same function US1 writes, tightening
  a loose rule into a precise one. It is independently *testable* (its tests are its own truth-table
  rows) but not independently *deliverable*, and shipping US1 without it would over-hide.
- **US3 (P3)**: independent of US2 and US4. Only needs `visible_worktrees()`.
- **US4 (P4)**: independent of US2 and US3. Only needs the flag and the accessor.

### Within Each User Story

- Tests are written and observed failing before implementation (Principle I)
- Core (`src/worktree.rs`, `src/app.rs`) before UI (`src/ui/`)
- Shared primitives (`ToggleChip`) before their call sites
- User-guide docs ship in the same story (Principle VII)

### Parallel Opportunities

- T004 + T005 (US1 tests) — different files
- T010 + T011 + T012 (US2 tests) — three different files
- T014 + T015 + T016 (US3 tests) — three different files
- T020 + T021 (US4 tests) — different files
- T026 (`src/ui/material/`) runs alongside the `src/app.rs` work in its phase
- T009 and T031 (docs) run alongside their phase's code work
- **Not parallel**: T027, T028, T029, T030 all touch `src/ui/sidebar.rs`; T024 and T025 both touch
  `src/app.rs` and are ordered (T025 depends on T024); T017 and T018 both touch `src/app.rs`

---

## Parallel Example: User Story 3

```bash
# Launch all three US3 test tasks together — three separate files, no shared state:
Task: "T014 available_tag_filters + rename-override survival in tests/app_state.rs"
Task: "T015 Missing/Invalid agent worktrees stay hidden in tests/sidebar_tree.rs"
Task: "T016 session bound to a hidden worktree in tests/session_lifecycle.rs"

# Then implement sequentially — T017 and T018 share src/app.rs:
Task: "T017 rebase available_tag_filters() onto visible_worktrees()"
Task: "T018 add has_visible_worktrees() accessor"
```

---

## Implementation Strategy

### Minimum releasable scope: US1 + US2

Unlike the usual "US1 alone is the MVP" pattern, the honest minimum here is **both** P1 and P2.
US1 delivers the value (agent worktrees disappear); US2 stops that value costing a user their own
`agent-*` worktree. Ship them together.

1. Phase 1 → Phase 2 → Phase 3 (US1) → Phase 4 (US2)
2. **STOP and VALIDATE**: quickstart scenarios 1 and 10, plus the SC-005 disk diff
3. That is a complete, defensible release on its own

### Incremental delivery after that

4. Add US3 → the empty state and filter chips stop lying → validate quickstart scenario 8
5. Add US4 → the escape hatch → validate quickstart scenarios 2–7a
6. Phase 7 polish across the whole feature

### Notes

- Commit after each task or logical group; verify Red before writing the Green
- `Worktree` gains methods but no fields, so existing tests constructing `Worktree { .. }` literals
  should not need edits — if they do, the design drifted toward a stored `owner` field (research R1)
- A diff touching `row_actions_cluster()`, `parse_worktrees()`, `classify()`, `reconcile()`, or
  `main.rs::discover_worktrees()` is a signal the implementation left the design: this feature is
  presentation-only and adds no discovery or lifecycle behavior (research R9, FR-013)
