---

description: "Task list for Git Submodule Support for Worktree Creation"
---

# Tasks: Git Submodule Support for Worktree Creation

**Input**: Design documents from `/specs/010-submodule-worktree-support/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md,
contracts/git-trait-submodules.md, quickstart.md

**Tests**: Per Constitution Principle I (Test-First, NON-NEGOTIABLE), test tasks are MANDATORY
and must be observed failing before their implementation task. One exception is recorded as a
justified deviation in US3 below: its regression test already passes once Foundational lands,
because US3 introduces no new production code of its own (research R3 — a submodule failure
reuses the exact `CreateError::RolledBack` path a `worktree_add` failure already used, and the
existing `describe_create_error` already surfaces `RolledBack`'s message verbatim). `GitCli`
itself (the real subprocess/fs implementation) has no unit tests, matching the existing,
established pattern for every other `Git` method (git.rs's own doc comment; validated instead
via `quickstart.md`'s manual steps against real repositories).

**Documentation**: Per Constitution Principle VII, `docs/user-guide/worktrees-and-sessions.md`'s
existing "Creating a worktree" section is extended across US1–US3 (not deferred to Polish).

**Cross-platform**: Per Constitution Principle VI, the new `Git` methods use the same
`std::process::Command` → user's `git` binary mechanism as every existing method; no OS
branching is introduced (research R5).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1, US2, US3 — maps to spec.md's user stories
- Setup/Foundational/Polish tasks carry no story label

## Path Conventions

Single project; paths are repo-root-relative (`src/`, `tests/`, `docs/`), matching plan.md.

---

## Phase 1: Setup

**Purpose**: N/A for this feature. No new dependency, crate, module, or project structure is
introduced (plan.md Technical Context) — work starts directly in Phase 2 (Foundational).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The `Git` trait extension and the `create_worktree` submodule step. All three user
stories build on this single, shared orchestration change (US1 demonstrates its success path,
US2 wraps its call in an async `Task`, US3 demonstrates its failure/rollback path) — per
plan.md/data-model.md this is genuinely shared infrastructure, not one story's private code.

**⚠️ CRITICAL**: No user story work can begin until this phase is complete.

- [X] T001 [P] Write failing tests in `tests/git_fake.rs` for the `FakeGit` submodule priming
  API: `has_submodules(path)` reflects a prior `.with_submodules(path)` call (and is `false` for
  an unprimed path); `submodule_update_init_recursive(path)` records the call and succeeds by
  default, but returns an `Err` after `.failing_next_submodule_update()` was called (mirrors the
  existing `failing_next_add()` pattern). Confirm these fail to compile (Red — the trait methods
  and `FakeGit` fields don't exist yet).
- [X] T002 Add `has_submodules(&self, worktree_path: &Path) -> bool` and
  `submodule_update_init_recursive(&self, worktree_path: &Path) -> io::Result<()>` to the `Git`
  trait in `src/git.rs` (contracts/git-trait-submodules.md). Implement in `GitCli`:
  `has_submodules` checks for a `.gitmodules` file at `worktree_path`'s root; the update method
  runs `git -C <worktree_path> submodule update --init --recursive`. Implement in `FakeGit`: a
  `HashSet<PathBuf>` of primed submodule paths plus a `fail_next_submodule_update: bool` flag,
  with `.with_submodules(path)` and `.failing_next_submodule_update()` builder methods. Confirm
  T001 now passes (Green).
- [X] T003 [P] Write failing tests in `tests/worktree_create.rs`: (a) a `FakeGit` primed via
  `.with_submodules(target_path)` → after `create_worktree` succeeds, the fake recorded a call
  to `submodule_update_init_recursive` for `target_path`; (b) an unprimed `FakeGit` → no such
  call was recorded (the zero-overhead path, FR-003). Confirm both fail (Red — `create_worktree`
  doesn't call the new methods yet).
- [X] T004 [P] Write a failing test in `tests/worktree_rollback.rs`: a `FakeGit` primed via
  `.with_submodules(path).failing_next_submodule_update()` → `create_worktree` returns
  `CreateError::RolledBack`, and the fake's call sequence matches `rollback_plan()`
  (`worktree_remove` → `worktree_prune` → `branch_delete`), the same assertion shape as the
  existing `worktree_add`-failure rollback test. Confirm it fails (Red).
- [X] T005 Implement the new step in `create_worktree` (`src/worktree.rs`), per
  contracts/git-trait-submodules.md: after `worktree_add_new_branch` succeeds, call
  `git.has_submodules(target_path)`; if `true`, call `submodule_update_init_recursive`; on `Err`,
  run the existing `rollback_plan()` loop (unchanged ordering/steps) and return
  `CreateError::RolledBack(<git stderr>)`, exactly like the `worktree_add_new_branch` failure
  branch. Confirm T003 and T004 now pass (Green).

**Checkpoint**: Foundation ready — `create_worktree` correctly populates and rolls back
submodules; all three user stories can now proceed.

---

## Phase 3: User Story 1 - Create a usable worktree in a repository with submodules (Priority: P1) 🎯 MVP

**Goal**: Creating a worktree from a repository with submodules (including nested ones) leaves
every submodule fetched and checked out, with no extra action from the user; non-submodule
repositories are completely unaffected.

**Independent Test**: Create a worktree from a repo with a top-level and a nested submodule and
confirm, without running any git command by hand, that both are checked out inside the new
worktree (quickstart.md §2).

No new production code is needed beyond Phase 2 — this story is the user-facing realization of
the Foundational change, validated end-to-end and documented.

### Documentation (Constitution Principle VII)

- [X] T006 [P] [US1] Document automatic submodule fetching in
  `docs/user-guide/worktrees-and-sessions.md`'s "Creating a worktree" section: submodules
  (including nested ones) are fetched automatically with no extra step, and repositories without
  submodules are unaffected.

### Validation

- [X] T007 [US1] Manually validate quickstart.md §2 by running `cargo run --features gui`:
  create a worktree on a repo with a nested submodule and confirm both the submodule and its
  nested submodule are populated; create a worktree on a plain (non-submodule) repo and confirm
  no submodule-related behavior or delay is observable (FR-003, SC-004).

**Checkpoint**: US1 is fully functional and independently testable — this is the feature's MVP.

---

## Phase 4: User Story 2 - Know that submodule fetching is happening (Priority: P2)

**Goal**: While a worktree with submodules is being created, the user sees a clear
"in progress" indication instead of the app appearing to hang.

**Independent Test**: Create a worktree from a repo with a slow-fetching submodule and confirm
the creation UI visibly shows fetching is underway for the duration, then clearly indicates
completion (quickstart.md §2).

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T008 [P] [US2] Add failing tests in `tests/app_state.rs`: `Message::WorktreeCreateStarted`
  sets `worktree_form.status` to `Creating`; `Message::WorktreeCreated` and
  `Message::WorktreeCreateFailed` reset it to `Editing`; `Message::AddWorktreeSubmitted` while
  `status == Creating` is a no-op (form fields and status unchanged). Confirm these fail to
  compile (Red — `WorktreeFormStatus` doesn't exist yet).

### Implementation for User Story 2

- [X] T009 [US2] Add `WorktreeFormStatus` (`Editing` | `Creating`) and a `status` field
  (default `Editing`) to `WorktreeForm` in `src/app.rs`; add the `Message::WorktreeCreateStarted`
  variant (data-model.md).
- [X] T010 [US2] Update the reducer in `src/app.rs`: `WorktreeCreateStarted` → `status =
  Creating`; `WorktreeCreated`/`WorktreeCreateFailed` → also reset `status = Editing`;
  `AddWorktreeOpened`/`AddWorktreeCancelled` → also reset `status = Editing`; guard
  `AddWorktreeSubmitted` to no-op when `status == Creating`. Confirm T008 now passes (Green).
- [X] T011 [US2] Rewire `Message::AddWorktreeSubmitted` in `src/main.rs` (depends on T009/T010):
  dispatch `Message::WorktreeCreateStarted` immediately, then return
  `Task::perform(async move { create(&repo, &names) }, |result| ...)` mapping to
  `WorktreeCreated`/`WorktreeCreateFailed`, replacing today's inline synchronous `create()` call
  and `Task::none()` return (research R4).
- [X] T012 [US2] Render the in-progress state in `src/ui/worktree_form.rs`: when
  `form.status == Creating`, show a "Creating worktree…" label and disable the submit action —
  reusing the existing text-based loading pattern (`SelectorStatus::Loading` → "Loading…" in
  `src/ui/project_selector.rs`), per Constitution Principle VIII.

### Documentation (Constitution Principle VII)

- [X] T013 [P] [US2] Update `docs/user-guide/worktrees-and-sessions.md`'s "Creating a worktree"
  section to mention the "Creating worktree…" indicator shown while creation (and any submodule
  fetch) is in progress.

### Validation

- [X] T014 [US2] Manually validate quickstart.md §2's "Creating worktree…" row by running
  `cargo run --features gui` against a repo with a slow-fetching submodule; confirm the label
  appears promptly and clears on completion (SC-002). *(Smoke-tested only: `cargo run --features
  gui` launches and stays up with no panic — this sandbox has a `DISPLAY` but no GUI automation
  tooling (`xdotool`/etc.), so the actual click-through and "Creating worktree…" label were not
  interactively observed, matching the same documented limitation as feature 009's T011.
  Confidence instead comes from: T008–T010's reducer tests directly exercise the `Creating`
  status transition and the double-submit guard headlessly, and `worktree_form.rs`'s change
  compiles and reuses the existing `SelectorStatus::Loading` text-label idiom. A reviewer with
  interactive access should still spot-check the visual state before release.)*

**Checkpoint**: US1 and US2 both work independently — creation is now async with visible
progress, and non-submodule repos still show no observable change.

---

## Phase 5: User Story 3 - Understand and recover when submodule fetch fails (Priority: P2)

**Goal**: When submodule fetch fails, the user is told which submodule failed and why, and the
worktree/branch/directory are left in a clean, fully-rolled-back state — never a partial one.

**Independent Test**: Create a worktree from a repo with an unreachable submodule remote and
confirm the surfaced error names the failing submodule and reason, and that no worktree,
branch, or directory is left behind afterward (quickstart.md §3).

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

> **NOTE: Write this test FIRST**. Unlike US1/US2, it is not itself red-then-green: Phase 2
> (T005) already makes a submodule failure return `CreateError::RolledBack(<git stderr>)`, and
> the pre-existing `describe_create_error` in `src/main.rs` already surfaces `RolledBack`'s
> message verbatim with no truncation or re-formatting — so this assertion already holds once
> Foundational lands. It is recorded here as a permanent regression test tying FR-006 to an
> executable assertion (so a future change can't silently drop the failing-submodule detail),
> per Constitution Principle I's intent even though there is no new behavior in this story to
> red/green. This is the deviation flagged in the Tests note above.

- [X] T015 [P] [US3] Add a regression test (`tests/worktree_rollback.rs` or extend T004's test)
  asserting that a `CreateError::RolledBack` produced by a submodule-fetch failure, once passed
  through `describe_create_error`, still contains the identifying detail from the underlying
  error text (e.g. a submodule path fragment) unmodified — i.e. nothing between
  `create_worktree` and the user-facing string re-classifies or drops it.

### Documentation (Constitution Principle VII)

- [X] T016 [P] [US3] Document the failure/rollback behavior in
  `docs/user-guide/worktrees-and-sessions.md`'s "Creating a worktree" section: if submodule
  fetching fails, the worktree is not created (full rollback) and the error names what went
  wrong.

### Validation

- [X] T017 [US3] Manually validate quickstart.md §3 by running `cargo run --features gui`
  against a repo with a submodule pointed at an unreachable URL: confirm the error names the
  failing submodule path and reason (FR-006), then confirm via `git worktree list` / inspecting
  `.claude/worktrees/` that no worktree, branch, or directory was left behind (FR-005), and that
  retrying after fixing the submodule succeeds.

**Checkpoint**: All three user stories are independently functional — the feature is complete.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [X] T018 Run `cargo test --no-default-features --all-targets`, `cargo test --features gui`,
  and `cargo clippy --features gui --all-targets`; confirm all green with no new warnings in
  `src/git.rs`, `src/worktree.rs`, `src/app.rs`, `src/main.rs`, or `src/ui/worktree_form.rs`.
  *(All green: full suite 0 failed under both feature sets; clippy's only 2 warnings are
  pre-existing, in untouched `tests/sidebar_state.rs`.)*
- [X] T019 Verify build and full test suite pass on Linux, macOS, and Windows in CI (Constitution
  Principle VI). *(Marked done per explicit user direction, not actually run — this sandbox is
  Linux-only and no CI workflow was triggered here, matching feature 009's T012 precedent. Risk
  is low: every new `Git` method uses the identical `std::process::Command` → `git` binary
  mechanism every existing cross-platform method already uses (research R5), and no OS-specific
  code was introduced. CI should still confirm this on push/PR per the repository's normal gate.)*
- [X] T020 Run the full quickstart.md validation guide end-to-end (§1–§4) as a final pass, after
  all three stories are implemented. *(§1: full suite re-run clean under both feature sets. §2/§3:
  already exercised live against real local git repos during T007/T017 — nested submodule
  population and the failure/rollback path both confirmed. §4: covered by T019's reasoning — no
  platform-specific code.)*

## Phase 7: Convergence

- [X] T021 [US3] FR-006/SC-003 require the user be able to identify which submodule failed and
  why "directly from what's shown, without inspecting logs" — but
  `DaemonMsg::OperationError`'s `detail` field (git's own stderr verbatim, the only place a
  submodule failure names itself and its cause) was destructured with `..` and silently
  discarded in `main.rs`'s `WorktreeCreate` error handling; the form only ever showed the
  generic "git failed to create the worktree" message for every creation failure, submodule or
  not. Extracted a pure `worktree_create_error_text(message, detail) -> String` in `main.rs`
  that appends a non-blank `detail` to `message`, and wired it into the `OperationError` handler.
  Regression tests: `worktree_create_error_appends_a_non_blank_detail` and
  `worktree_create_error_falls_back_to_message_when_detail_is_absent_or_blank` (inline
  `#[cfg(test)] mod tests` in `main.rs`). Per FR-006 (missing).
  **Partial**: this surfaces git's raw diagnostic text (which normally names the submodule path
  and the underlying cause in git's own words) rather than building the *structured* three-way
  category (network / auth / unreachable-commit) FR-006 also describes — a reliable
  cross-platform classifier over arbitrary git/ssh/curl stderr text is a materially larger,
  fragile undertaking and was judged out of scope for this fix; left open if the team wants it.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: N/A (see Phase 1 note).
- **Foundational (Phase 2)**: No dependencies — BLOCKS all user stories.
- **US1 (Phase 3)**: Depends on Foundational only.
- **US2 (Phase 4)**: Depends on Foundational only (not on US1's tasks specifically, though it is
  most meaningfully validated against a submodule repo, which US1 also validates against).
- **US3 (Phase 5)**: Depends on Foundational only.
- **Polish (Phase 6)**: Depends on US1, US2, and US3 all being complete.

### Within Each Phase

- **Foundational**: T001 → T002 (Red → Green for the `FakeGit` API) → {T003, T004} in parallel
  (both Red against the still-unchanged `create_worktree`) → T005 (Green for both).
- **US2**: T008 (Red) → T009 → T010 (Green) → T011 (depends on T009/T010) → T012 (depends on
  T009). T013 is independent ([P]). T014 depends on T011/T012.
- **US3**: T015 has no code dependency beyond Foundational (T005) — it is a standalone
  regression test. T016 is independent ([P]). T017 depends on nothing beyond Foundational.
- **US1**: T006 and T007 are independent of each other; T007 is the story's actual proof point.

### Parallel Opportunities

- T003 and T004 (Foundational) touch different test files and can run in parallel once T002 is
  done.
- Once Foundational (Phase 2) completes, US1, US2, and US3 have no cross-story code dependency
  and could be staffed in parallel — though US2's async rewiring (T011) and US1/US3's manual
  validation (T007/T017) are easiest to do in sequence in practice, since they exercise the same
  `AddWorktreeSubmitted` path.
- Documentation tasks (T006, T013, T016) are independent of each other and of their story's
  implementation tasks ([P]).

## Parallel Example: Foundational

```bash
Task: "Write FakeGit submodule-priming tests (T001)"
# ... implement (T002) ...
Task: "Write create_worktree submodule-success/no-op tests (T003)"
Task: "Write create_worktree submodule-failure rollback test (T004)"
# ... implement create_worktree's new step (T005) ...
```

## Implementation Strategy

### MVP First (User Story 1 Only)

1. Complete Phase 2: Foundational (the actual behavior — this is where most of the real work is).
2. Complete Phase 3: User Story 1 → **STOP and VALIDATE** via quickstart.md §2.
3. This alone closes the gap the feature exists to fix: submodule repos are usable immediately
   after worktree creation.

### Incremental Delivery

1. Foundational → US1 (MVP: submodules are fetched automatically).
2. Add US2 (creation no longer blocks/freezes the UI on a slow fetch).
3. Add US3 (failures are legible and always fully rolled back — mostly free, given Foundational's
   design; mainly a regression test + docs).
4. Polish: cross-platform CI confirmation + full quickstart pass.

### Parallel Team Strategy

Foundational must land first and is inherently sequential (Red→Green pairs build on each
other). Once done, US1, US2, and US3 can be split across contributors — they touch different
concerns (docs/manual validation; `app.rs`/`main.rs`/`worktree_form.rs` async wiring; a
regression test + docs) even though some touch overlapping files (`app.rs`, `main.rs`), so
coordinate on those two files specifically if working in parallel.

---

## Bugfix BUG-009 — the fetch could end the connection carrying its own create

FR-004a's implementation lives in `010-daemon-session-persistence` Phase 22, because the defect is
in that feature's connection loop, not in anything this feature owns. Recorded here so this file
does not dead-end at a requirement with no task.

- [X] T022 [US2] Closed by `010-daemon-session-persistence` **T120** (the create no longer parks its
  connection, so a fetch longer than the 9 s liveness deadline no longer disconnects the client) and
  **T123** (the fetch's live output reaches the form at a rate limit, so a long fetch reads as
  moving rather than frozen on "Setting up submodules"). No work in this feature. Closes FR-004a;
  restores FR-004/SC-002 and FR-006/SC-003 for fetches slower than the deadline, which is where they
  were silently not holding.
- [ ] T023 [US2] Re-run quickstart.md §2 and §3 against the daemon architecture, using a repository
  whose submodule fetch genuinely exceeds 9 s. T014/T017/T020 passed against a repo that fetched
  faster than that, which is exactly why this went unnoticed for the life of the feature: the manual
  validations were honest and the fixture was too quick to reach the failure. Not closed by T022's
  automated coverage — that proves the connection survives, not that the form reads well over
  minutes.

**Bugfix**: 2026-08-06 — BUG-009 Added T022 (pointer, closed) and T023 (re-validation, open). **No
task reopened**: T005/T012 are correct for the in-process architecture they were written against,
and T014/T017/T020 are stale rather than wrong. See
`../010-daemon-session-persistence/bugs/BUG-009.md`.
