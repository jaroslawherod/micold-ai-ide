---

description: "Task list for feature 021 — Feature-Module MVU Architecture"
---

# Tasks: Feature-Module MVU Architecture

**Input**: Design documents from `/specs/021-mvu-slice-architecture/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/, quickstart.md — all present

**Tests**: MANDATORY per Constitution Principle I. This feature has an unusual test posture worth
stating once: the **existing 71-file suite is the specification** (spec assumption "Test suite is the
behavior specification"), and FR-027 freezes its assertions. So for *extraction* tasks the Red state
already exists — any behavior drift turns the suite red. New **invariants** (the three guard tests)
follow ordinary Red-Green-Refactor: the guard is written and observed failing first.

**Documentation**: Not user-facing, so Principle VII is satisfied by architectural documentation
(`docs/development/architecture.md`), written per-story rather than deferred to polish.

**Cross-platform**: Principle VI. Only one platform branch is touched (the OS-theme probe), and
porting it improves parity. SC-006 requires all three platforms green.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: US1–US4 from spec.md
- Every task names exact file paths

## Path Conventions

Three-crate Rust workspace. `crates/micold-core/` (render-free domain + ports),
`crates/micold-client/` (iced GUI + app state), `crates/micold-daemon/` (**out of scope**, Q1).

## Story-to-tier map, and why the order differs from priority

US1 and US2 are **both P1**. The spec breaks the tie explicitly: US2 "is the outcome the other three
depend on — without per-feature boundaries there is nothing for the overlay registry to register
into". So Tier 1 (US2's first half) goes first, and it is the MVP.

US2 is delivered across **two** phases, because the spec assigns it both feature modules (Tier 1)
and per-feature reducer modules (Tier 3). This is not a numbering accident.

| Phase | Tier | Story | research.md §6 steps |
|---|---|---|---|
| 3 | Tier 1 | US2 (part 1) 🎯 MVP | 1–7 |
| 4 | Tier 2 | US1 | 8–11 |
| 5 | Shell split | US3 | 12–16 |
| 6 | Tier 3 | US2 (part 2) + US4 | 17–20 |

**Every task is its own commit** — SC-009 is verified from git history, not just the endpoint.
A task that needs a later task to compile is a planning error, not an acceptable intermediate.

---

## Phase 1: Setup

**Purpose**: Scaffolding and a baseline to measure against

- [X] T001 Create `crates/micold-client/src/features/mod.rs` with an empty module tree and declare `mod features;` in `crates/micold-client/src/lib.rs`
- [X] T002 [P] Record the pre-change baseline in `specs/021-mvu-slice-architecture/baseline.md`: per-file line counts from `find crates -name '*.rs' -exec wc -l {} + | sort -rn | head -10`, `State` field count, `Message` variant count, and the current commit SHA
- [X] T003 [P] Create `docs/development/architecture.md` with section headings only — tier structure, where a feature lives, adding a floating surface, adding a capability, the read/write asymmetry

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The safety nets every later phase relies on

**⚠️ CRITICAL**: No extraction may begin until T004 exists — it is what makes FR-027 enforceable
rather than aspirational

- [X] T004 Add an assertion-freeze check to `scripts/check-assertions-frozen.sh` that fails when `git diff <base>...HEAD -- crates/*/tests/` removes or alters a line matching `assert`, allowing pure relocation (the identical assertion re-added elsewhere in the same diff), per FR-027
- [X] T005 [P] Wire T004 into CI in `.github/workflows/ci.yml` as a non-blocking advisory job first, so its false-positive rate is known before it gates merges
- [ ] T006 [P] Confirm the baseline suite is green on Linux, macOS and Windows via CI before any change lands

**Checkpoint**: Behavior drift and assertion tampering are both detectable. Extraction can start.

---

## Phase 3: User Story 2 (part 1) — Feature modules (Priority: P1) 🎯 MVP

**Goal**: Every custom type in the monolithic state file moves into a module named for its feature,
together with the helper functions over it. Tier 1 of research.md §6, steps 1–7.

**Independent Test**: Pick any feature; write a test constructing only that feature's types,
exercising only its own operations, with no reference to any unrelated feature's types. It must
compile and pass without the application shell (SC-004).

**Method for every extraction task below**: move the type *with* its helpers (FR-001), `pub use` it
back from `app.rs` in the same commit so no call site changes, and keep the whole suite green. Never
split a feature across parallel state/update/view files (FR-001a).

### Tests for User Story 2 — write first, observe failing ⚠️

Each test constructs only its own feature's types, so each fails to compile until its module exists.
All are separate files, so all are parallelizable.

- [X] T007 [P] [US2] Isolation test for the worktree-creation form in `crates/micold-client/tests/features_worktree_form.rs`
- [ ] T008 [P] [US2] Isolation test for sidebar types in `crates/micold-client/tests/features_sidebar.rs`
- [ ] T009 [P] [US2] Isolation test for project/workspace types in `crates/micold-client/tests/features_project.rs`
- [ ] T010 [P] [US2] Isolation test for settings types in `crates/micold-client/tests/features_settings.rs`
- [ ] T011 [P] [US2] Isolation test for worktree types in `crates/micold-client/tests/features_worktree.rs`
- [ ] T012 [P] [US2] Isolation test for notification types in `crates/micold-client/tests/features_notifications.rs`
- [ ] T013 [P] [US2] Isolation test for session types in `crates/micold-client/tests/features_session.rs`
- [ ] T014 [P] [US2] Isolation test for daemon-connection types in `crates/micold-client/tests/features_connection.rs`
- [X] T014a [P] [US2] Render-free guard in `crates/micold-client/tests/features_are_render_free.rs` asserting no module under `crates/micold-client/src/features/` names the rendering framework in code — comments excepted, matching the existing convention in `app.rs` (FR-006). Follows the mechanism of `crates/micold-client/tests/{material_boundary,cdk_no_appearance}.rs`. **This is a regression lock, not a migration**: the property holds today and the guard exists to keep it holding across eight extractions. Q2's decision to site feature modules in the client rests entirely on it

### Implementation for User Story 2 (part 1)

Sequential — every task edits `app.rs`, so none of these are parallelizable against each other.

- [X] T015 [US2] Move `WorktreeForm`, `WorktreeFormStatus`, `BranchSource`, `ResolutionState` and their impls from `crates/micold-client/src/app.rs:86–326` to `crates/micold-client/src/features/worktree_form.rs` (~240 lines)
- [ ] T016 [US2] Move `SidebarEntry`, `DefaultNode`, `WorktreeNode`, `TagFilter`, `matches_filters`, `worktree_location_label` from `crates/micold-client/src/app.rs:372–456` to `crates/micold-client/src/features/sidebar.rs` (~85 lines)
- [ ] T017 [US2] Move `ProjectMenu`, `clamp_menu_anchor`, `SwitcherEntry`, `RenameDraft`, `SelectKind` from `crates/micold-client/src/app.rs:327–371, 457–497` to `crates/micold-client/src/features/project.rs` (~85 lines)
- [ ] T018 [US2] Move `SettingsDraft` from `crates/micold-client/src/app.rs:469–484` to `crates/micold-client/src/features/settings.rs` (~16 lines)
- [ ] T019 [US2] Move `WorktreeRenameDraft` and the worktree helpers `worktree_tree`, `filtered_worktree_tree`, `visible_worktrees`, `has_visible_worktrees`, `worktree_tags`, `worktree_display_name`, `available_tag_filters` from `crates/micold-client/src/app.rs:498–510, 2156–2245, 2276+` to `crates/micold-client/src/features/worktree.rs` (~105 lines)
- [ ] T020 [US2] Move `NoticeLevel` and `Notification` from `crates/micold-client/src/app.rs:923–944` to `crates/micold-client/src/features/notifications.rs`, reconciling against the existing `micold_core::notify` queue rather than duplicating it (~22 lines)
- [ ] T021 [US2] Move the session helpers `sessions_in_worktree`, `active_sessions`, `switch_active`, `record_foreground`, `restore_after_activation`, `restore_foreground`, `arm_notice`, `note_background_restart`, `session_mut` from `crates/micold-client/src/app.rs:2014–2155` to `crates/micold-client/src/features/session.rs` (~142 lines)
- [ ] T022 [US2] Extract the daemon-connection types and the `connection_status` projection from `crates/micold-client/src/app.rs` and `crates/micold-client/src/main.rs:2106` to `crates/micold-client/src/features/connection.rs` — **this step is absent from research.md §6's Tier 1 table, which lists seven steps for eight features; the gap was found during task generation** (FR-001, as amended)
- [ ] T023 [US2] Remove the transitional `pub use` re-exports from `crates/micold-client/src/app.rs` and update every call site to import from `crate::features::*`
- [ ] T024 [US2] Write the "where a feature lives" and "tier structure" sections of `docs/development/architecture.md`, listing all nine modules
- [ ] T025 [US2] Verify SC-010 by review — name the single module for each feature in FR-001 — and record the intermediate `app.rs` line count against T002's baseline

**Checkpoint**: `app.rs` should be roughly 1,700 lines (types out, both reducers still in). Every
feature answers "where does it live?" with one module. Full suite green on all three platforms.

---

## Phase 4: User Story 1 — Overlay registry (Priority: P1)

**Goal**: Adding a floating surface costs its own module plus at most one registration line, and
zero edits to any central match statement. Tier 2 of research.md §6, steps 8–11.

**Independent Test**: Add a throwaway overlay end-to-end and count changed files with
`git diff --stat`. It must be its own module plus ≤1 registration line, with zero central match
edits (SC-001). Then revert.

**⚠️ Highest-risk phase in the feature.** The exit-animation snapshot (FR-011) renders a *copy* of a
surface whose live state has been cleared, and `ClosingOverlay` exists solely to serve it. Steps
land as four separate commits so a bisect finds one of them, not a monolithic overlay rewrite.

### Tests for User Story 1 — write first, observe failing ⚠️

- [ ] T026 [P] [US1] Registration guard in `crates/micold-client/tests/overlay_registration.rs` — a surface that exists but is not registered MUST fail the build or this test, never be discovered by hand at runtime (FR-010, contract R2)
- [ ] T027 [P] [US1] Dismissal-ordering test in `crates/micold-client/tests/overlay_dispatch_ordering.rs` covering contract obligations D1 (popover closes before modal on Escape), D2 (opening a modal closes popovers) and D3 (closing the filter panel leaves filters intact), asserted against the generic dispatch rather than the special-case match

### Implementation for User Story 1

- [ ] T028 [US1] Introduce the uniform `FloatingSurface` type and `StackBand`/`DismissalRules` in `crates/micold-client/src/overlay/mod.rs`, built on feature 017's existing `Layer`/`Surface`/`Trigger` vocabulary in `crates/micold-core/src/overlay.rs` — not a parallel one (FR-014)
- [ ] T029 [US1] Add the registry and its `register!` macro in `crates/micold-client/src/overlay/registry.rs`, with `Overlay`/`ClosingOverlay` still present and deriving into it so both representations coexist green
- [ ] T030 [US1] Implement the builder API for `FloatingSurface` terminating in `.into()` per Principle VIII and FR-030, and confirm `crates/micold-client/tests/material_builder_api.rs` still passes
- [ ] T031 [US1] Migrate the 7 ad-hoc popovers off their loose `State` fields onto the registry — `help_menu_open`, `project_switcher_open`, `sidebar_filter_open`, `worktree_menu_open`, `project_menu_open`, `terminal_context_menu`, `session_menu_open` in `crates/micold-client/src/app.rs` (FR-007)
- [ ] T032 [US1] Migrate the 9 real `Overlay` variants onto the registry, preserving each surface's dismissal rules (the 10th variant, `None`, becomes "nothing open" rather than a surface)
- [ ] T033 [US1] Collapse central match site 1 of 6 — the `Overlay` enum at `crates/micold-client/src/app.rs:55` — onto generic dispatch
- [ ] T034 [US1] Collapse sites 2 and 3 — `on_escape` at `crates/micold-client/src/app.rs:2322` and its keyboard-subscription mirror at `crates/micold-client/src/ui/mod.rs:519` — preserving the popover-before-modal priority currently hand-written at `ui/mod.rs:554`
- [ ] T035 [US1] Collapse sites 4 and 5 — the view match at `crates/micold-client/src/ui/mod.rs:337` and `capture_overlay` at `crates/micold-client/src/main.rs:727`
- [ ] T036 [US1] Collapse site 6 — delete the `ClosingOverlay` enum at `crates/micold-client/src/app.rs:2387` and its impl, moving snapshot behavior onto `FloatingSurface::snapshot` (FR-011, contract A1–A3)
- [ ] T037 [US1] Delete the `Overlay` enum and confirm `crates/micold-client/tests/{one_overlay_implementation,overlay_dismissal_delta,overlay_stacking,overlay_transition_identity}.rs` all pass **unmodified**
- [ ] T038 [US1] Write the "adding a floating surface" section of `docs/development/architecture.md`
- [ ] T039 [US1] Add the **permanent** SC-001 guard in `crates/micold-client/tests/surface_registration_cost.rs`, failing if any registered surface is reachable from anywhere beyond its own module and the single registration point (SC-001, SC-002a — clarified 2026-08-07: a permanent guard replaces the one-time file count, which proves the property only on the day it is taken)
- [ ] T040 [US1] Perform quickstart.md procedure M2 (six manual overlay behaviors) and record the result

**Checkpoint**: 19 enum variants and 7 loose fields gone. Six central match statements reduced to
zero. Every pre-existing overlay test passes unmodified.

---

## Phase 5: User Story 3 — Capabilities and shell split (Priority: P2)

**Goal**: Every I/O concern is a narrow declared capability; the binary is the single place real
implementations are chosen; the shell divides by external system. research.md §6, steps 12–16.

**Independent Test**: For each capability, run the behavior depending on it against a fake and
assert the outcome, with no real filesystem, repository, clipboard or OS query involved (SC-005).

**Scoping note**: FR-017 already largely holds — `app.rs` constructs no concrete implementation, and
all nine construction sites are already inside the shell. The real work is FR-018 (single assembly
point); T048's guard is a regression lock on an existing property, not a migration.

### Tests for User Story 3 — write first, observe failing ⚠️

- [ ] T041 [P] [US3] Guard in `crates/micold-client/tests/no_concrete_implementations.rs` asserting non-shell code names no concrete implementation — `GitCli`, `JsonFileStore`, `JsonFileSettingsStore`, `StdFolderScanner` — and that they are constructed in exactly one place (FR-017, FR-018)
- [ ] T042 [P] [US3] Fake-coverage test in `crates/micold-client/tests/service_capability_fakes.rs` asserting every declared capability has a fake and at least one test exercising real behavior through it (FR-019, SC-005), **and** that each capability is narrow enough that no consumer must implement an operation it does not exercise — the spec's own narrowness test, applied per capability (FR-016). A capability failing the narrowness check MUST be split rather than the check relaxed
- [ ] T043 [P] [US3] Behavior test for env-include resolution through its fake in `crates/micold-core/tests/env_include.rs`
- [ ] T044 [P] [US3] Behavior test for the OS theme probe through its fake in `crates/micold-core/tests/os_theme.rs`
- [ ] T045 [P] [US3] Test asserting a feature emits `Outcome::ClipboardWrite` with zero real clipboard access in `crates/micold-client/tests/clipboard_request.rs` (FR-015a, contract C2)

### Implementation for User Story 3

- [ ] T046 [US3] Declare `EnvIncludeResolver` in `crates/micold-core/src/env_include.rs` with a real implementation moved from `crates/micold-client/src/main.rs:397–450` and a fake
- [ ] T047 [US3] Declare `OsThemeProbe` in `crates/micold-core/src/os_theme.rs` wrapping the `dark_light` call at `crates/micold-client/src/main.rs:2678` with a fake — this is the codebase's only direct OS branch, so this also serves Principle VI
- [ ] T048 [US3] Add fakes for any of the seven existing ports lacking one — `ProjectStore`, `SettingsStore`, `FolderScanner`, `TerminalBackend`, `TerminalHandle`, `AiCliProvider` — in `crates/micold-core/src/` beside each capability, as ordinary public items matching `FakeGit` at `crates/micold-core/src/git.rs:467`. **Not** behind a `cfg` feature and **not** in a separate crate (FR-019, clarified 2026-08-07)
- [ ] T049 [US3] Create the `Capabilities` struct in `crates/micold-client/src/shell/capabilities.rs`, assembled once at boot, replacing all nine inline construction sites in `crates/micold-client/src/main.rs` (523, 532, 649, 1295, 1310, 1330, 1924, 2604, 2709) — four of which are inside `update_inner` (FR-018)
- [ ] T050 [US3] Split `crates/micold-client/src/main.rs` startup into `crates/micold-client/src/shell/startup.rs` — `boot`, `window_settings`, `main` — with inline `#[cfg(test)]` tests relocated alongside (FR-019a, FR-027's relocation clause)
- [ ] T051 [US3] Split persistence into `crates/micold-client/src/shell/persist.rs` — `persist`, `persist_settings`, `prune_empty_sessions` — with its tests
- [ ] T052 [US3] Split daemon synchronisation into `crates/micold-client/src/shell/daemon_sync.rs` — `send_op`, `switch_daemon_attachment`, `reconcile_catalog`, `PendingOp` — with its tests
- [ ] T053 [US3] Split subscriptions into `crates/micold-client/src/shell/subscriptions.rs` — `subscription`, `cursor_move_events`, `window_focus_events`, `os_theme_poll` — with its tests
- [ ] T054 [US3] Split the remaining two systems into `crates/micold-client/src/shell/env_include.rs` and `crates/micold-client/src/shell/os_theme.rs` with their tests
- [ ] T055 [US3] Move `update_inner`'s effectful arms from `crates/micold-client/src/main.rs:775–2028` to the shell module addressing each arm's external system
- [ ] T056 [US3] Route clipboard through `Outcome::ClipboardWrite` interpreted by the shell, replacing the three direct `iced::clipboard` calls at `crates/micold-client/src/main.rs:1840, 1847, 1856` (FR-015a)
- [ ] T057 [US3] Write the "adding a capability" section of `docs/development/architecture.md`

**Checkpoint — SC-004b**: Tiers 1 and 2 and the shell split are all merged with **zero Tier 3 work**.
Demonstrating green here is the criterion. Do not start Phase 6 before recording it.

- [ ] T058 [US3] Record the SC-004b demonstration: full suite green on all three platforms with no part of Tier 3 merged, noted in `specs/021-mvu-slice-architecture/baseline.md`

---

## Phase 6: User Story 2 (part 2) + User Story 4 — Reducer split and outcomes (Priority: P1/P2)

**Goal**: The reducer becomes per-feature modules over the shared state, and cross-feature effects
become explicit returned outcomes. research.md §6, steps 17–20.

**Independent Test (US2)**: The root state, messages and reducer contain composition and routing
only — no feature's decision logic (FR-002).
**Independent Test (US4)**: Run the worktree-delete path in isolation; assert it returns outcomes
describing the session and overlay consequences while touching only worktree data. The existing
`worktree_delete.rs` must pass unchanged.

**Applies to both reducers.** Per FR-004a as amended, a feature's pure arms go to its reducer module
and its effectful arms to the shell module for their external system. `app.rs::update` is 778 lines;
`main.rs::update_inner` was 1,253 before Phase 5 reduced it.

### Tests for Users Stories 2 and 4 — write first, observe failing ⚠️

- [ ] T059 [P] [US4] Cross-feature write guard in `crates/micold-client/tests/feature_write_isolation.rs` asserting no feature reducer mutates another feature's data, and **naming the offending path** in its failure message (FR-020, FR-024a, SC-007, contract O6)
- [ ] T060 [P] [US4] Termination test in `crates/micold-client/tests/outcome_termination.rs` asserting outcome interpretation terminates under a cycle and does not depend on composition order (FR-024, contract O4/O5)
- [ ] T061 [P] [US2] Root-routing test in `crates/micold-client/tests/root_is_routing_only.rs` asserting the root reducer contains no feature decision logic (FR-002)

### Implementation

- [ ] T062 [US2] Split `State::update` at `crates/micold-client/src/app.rs:1165–1942` into per-feature reducer modules, one per feature module from Phase 3, each operating on the shared state (FR-004a)
- [ ] T063 [US2] Reduce the root reducer in `crates/micold-client/src/app.rs` to routing only, dispatching to the per-feature reducer modules (FR-002)
- [ ] T064 [US2] Promote the worktree-creation form to a nested unit in `crates/micold-client/src/features/worktree_form.rs` with its own message type absorbing the 22 root variants — 18 `AddWorktree*` and 4 `WorktreeCreate*` — routed through one wrapping variant (FR-003; the sole nested unit per research.md §5)
- [ ] T065 [US4] Introduce the `Outcome` enum in `crates/micold-client/src/features/mod.rs` with the four known variants — `SessionsClosed`, `OverlayDismissed`, `ClipboardWrite`, `NotificationRaised` — and the root's draining interpreter with a fixed iteration bound (FR-021, FR-022, FR-024)
- [ ] T066 [US4] Convert the worktree-delete path to mutate only worktree data and return `SessionsClosed` + `OverlayDismissed`, and confirm `crates/micold-client/tests/worktree_delete.rs` passes **unmodified** (FR-023)
- [ ] T067 [US4] **Discovery**: run T059's guard against the full codebase, enumerate every cross-feature write it names, and record the list in `specs/021-mvu-slice-architecture/cross-feature-writes.md` with a proposed outcome variant for each. This task is complete when the list exists — it converts nothing
- [ ] T067a [US4] **Conversion**: for each write enumerated by T067, convert it to return an outcome, one commit per write. Expand this into one concrete task per entry once T067's list exists — its size is unknowable until then, which is why it is not estimated here (FR-020, FR-021)
- [ ] T068 [US2] Write the "read/write asymmetry" section of `docs/development/architecture.md`, including why guard tests hold the line rather than the type system (plan.md Complexity Tracking)

**Checkpoint**: Zero direct cross-feature reducer writes. Root is routing only. 22 variants left the
root message enum.

---

## Phase 7: Polish & Cross-Cutting Concerns

- [ ] T069 Measure SC-003: run `find crates -name '*.rs' -exec wc -l {} + | sort -rn | head -10` and confirm neither `app.rs` nor `main.rs` is near the top and neither holds more than one feature. **FR-005 governs; the 500-line figure is indicative** (clarified 2026-08-07). Record the count as a progress signal — do NOT split a single-feature module to cross a threshold
- [ ] T070 [P] Add the **permanent** SC-002 guard in `crates/micold-client/tests/feature_registration_cost.rs`, failing if adding a feature would require edits beyond its own module and one registration point (SC-002, SC-002a)
- [ ] T071 [P] Verify SC-004 — each of the **eight** feature modules has an isolation test (T007–T014). The overlay registry is a ninth module but not a feature module, and is covered by its own guards instead
- [ ] T072 Perform quickstart.md procedure M1 (persisted state written by the pre-change build loads and behaves identically) and record the result (SC-008, FR-026)
- [ ] T073 Run the FR-027 check across the whole feature branch: `git diff main...HEAD -- crates/*/tests/ | grep -E '^-.*assert'` must show nothing but pure relocations
- [ ] T074 [P] Promote T004's assertion-freeze job from advisory to blocking in `.github/workflows/ci.yml` now its false-positive rate is known
- [ ] T075 [P] Review `docs/development/architecture.md` end to end for coherence and update `docs/README.md` navigation
- [ ] T076 Confirm SC-009 from history: every task above is its own commit, and each commit builds, runs and passes
- [ ] T077 Verify the full suite green on Linux, macOS and Windows (SC-006, Principle VI)
- [ ] T078 Re-measure the baseline table in `specs/021-mvu-slice-architecture/spec.md` against the final tree and record the delta — the figures have moved four times during this feature's life

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 1 (Setup)**: No dependencies
- **Phase 2 (Foundational)**: Depends on Phase 1. **Blocks everything** — T004 is what makes FR-027 enforceable
- **Phase 3 (US2 part 1, Tier 1)**: Depends on Phase 2. The MVP
- **Phase 4 (US1, Tier 2)**: Depends on Phase 3 — the registry needs feature modules to register into (spec, US2's "Why this priority")
- **Phase 5 (US3, shell)**: Depends on Phase 2 only. **Orthogonal to Tiers 1–3** (FR-019a) — can run parallel to Phases 3–4 with a second developer
- **Phase 6 (US2 part 2 + US4, Tier 3)**: Depends on Phases 3, 4 and 5. Last, because §5's nesting evidence is only trustworthy once boundaries are visible
- **Phase 7 (Polish)**: Depends on all above

### Critical constraint

**Phase 5 must complete and be recorded green (T058) before Phase 6 begins.** SC-004b requires
demonstrating Tiers 1, 2 and the shell split with zero Tier 3 merged. Starting Phase 6 early makes
that criterion unverifiable — it is the one ordering rule that cannot be relaxed for convenience.

### Within each phase

- Tests written and observed failing before implementation (Principle I)
- Extraction tasks are sequential where they touch the same file (all of Phase 3 edits `app.rs`)
- Architectural docs ship inside their own phase, not deferred (Principle VII)

### Parallel Opportunities

- T002, T003 in Setup
- T005, T006 in Foundational
- All eight isolation tests T007–T014 plus the render-free guard T014a (separate files)
- All five capability tests T041–T045 (separate files)
- T059–T061 (separate files)
- **Phase 5 against Phases 3–4** — the shell split is orthogonal by FR-019a, the single largest parallelization win here
- T070, T071, T074, T075 in Polish

**Not parallelizable**: T015–T023 all edit `app.rs`. T028–T037 form a deliberate serial chain so a
bisect lands on one overlay change.

---

## Parallel Example: Phase 3 tests

```bash
# All eight isolation tests are separate files with no shared state.
# Each fails to compile until its module exists — that is the Red state.
# T014a joins them: a regression lock that passes from the start and must keep passing.
Task: "Isolation test for the worktree-creation form in crates/micold-client/tests/features_worktree_form.rs"
Task: "Isolation test for sidebar types in crates/micold-client/tests/features_sidebar.rs"
Task: "Isolation test for project/workspace types in crates/micold-client/tests/features_project.rs"
Task: "Isolation test for settings types in crates/micold-client/tests/features_settings.rs"
Task: "Isolation test for worktree types in crates/micold-client/tests/features_worktree.rs"
Task: "Isolation test for notification types in crates/micold-client/tests/features_notifications.rs"
Task: "Isolation test for session types in crates/micold-client/tests/features_session.rs"
Task: "Isolation test for daemon-connection types in crates/micold-client/tests/features_connection.rs"
```

---

## Implementation Strategy

### MVP (Phase 3 only)

1. Phase 1 Setup → Phase 2 Foundational
2. Phase 3 — Tier 1 feature modules
3. **STOP and VALIDATE**: every feature answers "where does it live?" with one module (SC-010);
   `app.rs` roughly 1,700 lines; suite green on three platforms
4. This is a complete, shippable improvement. Tiers 2 and 3 need not follow immediately.

### Incremental Delivery

Each phase is independently shippable (FR-004c), which is unusual and worth exploiting:

1. Setup + Foundational → safety nets in place
2. Phase 3 → feature modules → **ship** (MVP)
3. Phase 4 → overlay registry → **ship** (removes the largest source of "I forgot a site" bugs)
4. Phase 5 → capabilities and shell split → **ship** (SC-004b checkpoint)
5. Phase 6 → reducer split and outcomes → **ship**
6. Phase 7 → verification and measurement

### Parallel Team Strategy

Two developers, after Phase 2:

- Developer A: Phase 3 → Phase 4 (Tiers 1 and 2, both touching `app.rs`)
- Developer B: Phase 5 (shell split, touching `main.rs`)

The split is clean because FR-019a makes the shell orthogonal to the tiers, and the two developers
touch different files. They converge for Phase 6, which needs both.

---

## Notes

- **Every task is its own commit.** SC-009 is verified from `git log`, not from the endpoint
- Verify tests fail before implementing — for extraction, the existing suite already provides Red
- The existing 71-file suite is frozen (FR-027). Additions and relocations are fine; rewrites and
  deletions are defects
- Re-measure rather than trusting any figure here — the baseline has moved four times in ten days
- **T022 covers a gap**: research.md §6's Tier 1 table lists seven steps for eight features. The
  daemon-connection extraction was missing and is added here
