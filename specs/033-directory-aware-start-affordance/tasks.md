---
description: "Task list for feature 033 — the start affordance answers for its own directory"
---

# Tasks: The start affordance answers for its own directory

**Input**: Design documents from `/specs/033-directory-aware-start-affordance/`

**Prerequisites**: [plan.md](./plan.md), [spec.md](./spec.md), [research.md](./research.md),
[data-model.md](./data-model.md), [contracts/availability-asks.md](./contracts/availability-asks.md),
[quickstart.md](./quickstart.md)

**Tests**: Per Constitution Principle I (Test-First Development, NON-NEGOTIABLE), test tasks are
MANDATORY and must be observed failing first. Behavior ids (`[A#]`, `[U#]`) refer to
[tdd/test-list.md](./tdd/test-list.md); `/speckit.tdd.run` ticks a task from them. Every phase writes its failing tests before its implementation. No GUI-only exception is
used. The per-row decision is render-free, so the view only passes a directory.

**Documentation**: Per Constitution Principle VII, each user-facing story carries its own user-guide
task in the milestone that ships it.

**Cross-platform**: Per Constitution Principle VI, no `cfg` arm is added (plan, Technical Context).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies on incomplete tasks)
- **[Story]**: Which user story the task belongs to (US1–US3)
- Every description carries an exact file path

## Path Conventions

All production changes are in `crates/micold-client/`. `micold-core` and `micold-daemon` are read,
not changed. Build and test through `mise run <task>` (CLAUDE.md). Shell event tests live in
`crates/micold-client/src/main_tests.rs` (the binary `micold-ai-ide`'s `tests` module; name each new
test with `availability` so the milestone Verify filter selects it), which has an outbox harness to extend
(`connected_with_outbox`, `availability_asked_for`).

---

## Phase 1: Setup (Shared Infrastructure)

No setup tasks: no new crate, dependency, module tree or configuration (plan, Structure Decision).

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: The per-directory answer store and the directory-taking readers every story reads
through. It replaces `session::State::available_providers` and `App::cli_availability_asked`.

### Tests (write first, confirm they fail)

- [X] T001 [P] [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] [U9] [U10] Write `crates/micold-client/tests/directory_availability.rs`, pinning `AvailabilityAnswers` (data-model.md) as a pure type:
  - `asked(req, key)` then `answered(req, ..)` files the answer under `key` and returns `true`.
  - A late answer to an older `req` for the **same** directory is dropped once a newer `req` was asked (FR-009).
  - Answers for two **different** directories are both kept, in either arrival order (FR-009).
  - An answer whose `req` was never asked is dropped (a pre-reconnect reply).
  - A `Home` answer never changes any `Dir` answer, and a `Dir` answer never changes `home()` (FR-002).
  - `for_dir(d)` returns `d`'s answer when held and the home answer otherwise (FR-005).
  - `retain(wanted)` drops `Dir` answers, `latest` and `in_flight` entries outside `wanted`, and keeps `Home` (FR-003, FR-012).
  - `unasked(wanted)` lists wanted directories with no held answer and no request in flight, each once (FR-006).
  - `clear()` drops everything, `home` included (FR-011).
  - `env_include_changed(s)` returns `true` and records `s` only when `s` differs from `asked_under` (R6).
- [ ] T002 [P] [U11] [U12] [U13] [U14] [U15] Add reader tests to `crates/micold-client/tests/features_session.rs`: with home = `[ClaudeCode]`, `P` = `[ClaudeCode, Pi]` and `Q` = `[ClaudeCode]` held in one state:
  - `start_affordance_offers_a_choice(P)` is true and `(Q)` is false (US1-1, US1-2, FR-001).
  - With default `Pi`, `start_intent(Primary, P)` is `Start(Pi)` (US1-5), and `start_intent(Primary, Q)` is `OfferChoice { unavailable_default: Some(Pi), .. }` (FR-010).
  - A directory with nothing held reads the home answer (FR-005).
  - With nothing held at all, the result is `NothingAvailable`.
  - `offered_providers(Some(P))` lists `Pi`, and `offered_providers(None)` reads home only (FR-008).
- [X] T003 [U16] [U17] [U18] [U19] [U20] [U21] Add `wanted_availability_dirs` tests to `crates/micold-client/tests/directory_availability.rs`:
  - The result is the active root plus `location_dir(&SessionLocation::Worktree(dir_name))` for each visible worktree with `can_start_session()`.
  - An agent worktree hidden by the reveal control is excluded, and included once `sidebar.show_agent_worktrees` is on.
  - A `WorktreeStatus::Missing` worktree is excluded.
  - An included worktree (outside `.claude/worktrees`) is keyed by `location_dir`, not `Worktree::path` (research R3).
  - The set is empty with no active project.
  - `location_dir` (U16): `Default` → the active root; `Worktree(d)` → `root/.claude/worktrees/d`; no active project → `None`.

### Implementation

- [ ] T004 [U1] [U2] [U3] [U4] [U5] [U6] [U7] [U8] [U9] [U10] Implement `AvailabilityKey { Home, Dir(PathBuf) }`, `EnvIncludeSettings` and `AvailabilityAnswers` in `crates/micold-client/src/features/session.rs`:
  - The fields are `home`, `dirs`, `latest`, `in_flight` and `asked_under`. The operations are `asked`, `answered`, `for_dir`, `home`, `retain`, `unasked`, `clear` and `env_include_changed`, exactly as data-model.md specifies.
  - Replace `session::State::available_providers` with `availability: AvailabilityAnswers`, and move its doc comment's R11 note ("never persisted") onto the new field.
- [ ] T005 [U11] [U12] [U13] [U14] [U15] [U16] [U17] [U18] [U19] [U20] [U21] Change the readers in `crates/micold-client/src/features/session.rs` to take the answer to read, per data-model.md "Readers":
  - `available_in(Option<&Path>)` replaces `known_available()`, and `default_ai_cli_is_available`, `offered_providers`, `start_affordance_offers_a_choice(&Path)` and `start_intent(target, &Path)` take the directory.
  - Add `app::State::location_dir(&SessionLocation) -> Option<PathBuf>` in `crates/micold-client/src/app.rs`, through `SessionLocation::cwd`.
  - Add `features::session::wanted_availability_dirs(&app::State) -> BTreeSet<PathBuf>` over `visible_worktrees()`.
- [ ] T040 Move every existing consumer onto the new API **with today's behaviour**, so the crate builds and the US1 tests can go red for the right reason:
  - `crates/micold-client/src/ui/sidebar.rs` and `ui/mod.rs` read the home answer (`available_in(None)` and the readers with the home key).
  - `crates/micold-client/src/shell/daemon_sync.rs`: `ask_cli_availability` records `asked(req, AvailabilityKey::Home)` for every existing call, and the `DaemonMsg::AiCliAvailability` arm files through `answered`.
  - Delete `App::cli_availability_asked` from `crates/micold-client/src/main.rs`, `shell/startup.rs` and the `App` literals in `src/main_tests.rs`.
  - T013, T016 and T017 then switch these sites to per-directory keys.
- [ ] T006 Update `crates/micold-client/tests/support/state_scan.rs`: `MUTATORS` gains `asked`, `answered` and `env_include_changed`; `READERS` gains `for_dir`, `home`, `unasked` and `available_in`; `READERS` loses `known_available` (data-model.md "Source-scan vocabulary"). Run `feature_write_isolation.rs` and `root_state_is_shared.rs` green.
- [ ] T007 Migrate the existing tests that seed `state.session.available_providers` to seed `AvailabilityAnswers` (home, or a directory where the test is about a row): `crates/micold-client/tests/unavailable_default_says_so.rs`, `session_start_press.rs`, `provider_choice_surfaces.rs`, `missing_cli_is_reported_where_it_is_chosen.rs`, `a_field_note_shares_its_fields_column.rs` and `features_session.rs`. In `crates/micold-client/src/main_tests.rs`, migrate `connecting_asks_which_clis_the_service_can_run` to read the home answer, and rewrite `an_answer_to_an_earlier_question_does_not_replace_a_later_one`: its premise (a later directory ask discards the home answer) is reversed by FR-002/FR-009, so it becomes a home-plus-directory "both kept" case (U2/U3 own the same-directory rule). In `cli_availability_comes_from_the_service.rs`, replace the vacuity spelling `available_providers = Some(` with the new write (`.answered(`).

**Checkpoint**: `mise run test-core` is unchanged, the workspace builds, and `cargo test -p micold-client` passes with T001–T003 green. Behaviour is still today's: every consumer reads home (T040).

---

## Phase 3: User Story 1 - A CLI one project provides can be started in that project (Priority: P1) 🎯 MVP

**Goal**: Each row's chevron and primary press read that row's directory answer, asked eagerly when
the project opens. Settings and reconnects no longer overwrite it.

**Independent Test**: quickstart.md §B steps 1–5. Automated: the `main_tests.rs` cases below.

### Tests for User Story 1 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T008 [US1] [U22] [U23] [A4] In `crates/micold-client/src/main_tests.rs`, test connect (C1-A1). Add a helper beside `connected_with_outbox` in `main_tests.rs`, `connect_with_catalog_keeping_outbox(app, snapshot)`, which connects with the given catalog and does **not** drain what the connect sent (the existing helper hard-codes an empty `/repo/demo` catalog and drains). With active project `/repo/demo`, one valid user worktree, one hidden agent worktree and one `Missing` worktree:
  - After `Welcome`, the outbox holds exactly one `AiCliAvailabilityRequest { cwd: None }` and one for each of `/repo/demo` and `/repo/demo/.claude/worktrees/<dir>`. That is D = 2 directory requests (FR-006, FR-007, SC-004).
  - A second `Welcome` (reconnect) clears held answers and asks the same set again (FR-011, US1-4).
- [ ] T009 [US1] [A1] [A2] [A5] [U23] In `crates/micold-client/src/main_tests.rs`, test filing:
  - Answers for the root and the worktree arrive in reverse order. Each row's `start_affordance_offers_a_choice` reflects its own answer (FR-001, FR-009).
  - An answer to a request sent before a reconnect is dropped.
  - With `Q` answering `[ClaudeCode]`, `Q`'s rows offer no choice and the primary press yields `Start(ClaudeCode)` (acceptance A2).
  - With default `Pi` held only in `P`'s answer, the primary press on `P`'s row yields `Start(Pi)` (acceptance A5).
- [ ] T010 [US1] [A3] [U24] [U25] In `crates/micold-client/src/main_tests.rs`, test Settings (C1-A2, US1-3, FR-002, FR-008). With `P`'s rows holding `[ClaudeCode, Pi]`, `Settings(Opened)` asks `cwd: None` only, and its answer `[ClaudeCode]` leaves `P`'s rows offering the choice. `StartMenuOpened` on a worktree row asks for that row's `location_dir` (U25).
- [ ] T011 [US1] [U26] [U27] In `crates/micold-client/src/main_tests.rs`, test project open and the catalog push (C1-A4, C1-A5, FR-004, FR-006):
  - Opening a project asks once per wanted directory, and opening it again asks nothing new.
  - A `CatalogChanged` carrying one new worktree asks for exactly that directory.
- [ ] T012 [P] [US1] [A1] [U35] [U36] In `crates/micold-client/tests/provider_choice_surfaces.rs`, test rendered surfaces (FR-008, US1-1):
  - The Settings default select lists the home answer.
  - The start list opened on `P`'s row lists `P`'s answer, `Pi` included, while home lacks it.
  - The missing-default notice in `missing_cli_is_reported_where_it_is_chosen.rs` reads the list's directory.

### Implementation for User Story 1

- [ ] T013 [US1] [U23] In `crates/micold-client/src/shell/daemon_sync.rs`, change `ask_cli_availability` to take the `AvailabilityKey` it asks for (T040 hard-wired `Home`), sending `cwd: None` for `Home` and `Some(dir)` for `Dir(dir)`, so an answer is filed under the key its request named.
- [ ] T014 [US1] [U22] [U23] [A4] In `crates/micold-client/src/shell/daemon_sync.rs`, add `sync_cli_availability(app)` and wire `on_connected`. The sync asks `unasked(wanted_availability_dirs(&app.core))` in this milestone; the pruning `retain` lands in T026. `on_connected` does `clear()`, sets `asked_under` from `Welcome`'s env-include settings, asks `Home` (the existing call), then syncs (research R5).
- [ ] T015 [US1] [U26] [U27] In `crates/micold-client/src/shell/daemon_sync.rs`, call `sync_cli_availability` in the `DaemonMsg::CatalogChanged` arm after `reconcile_catalog(.., true)`. In `crates/micold-client/src/shell/workspace.rs`, call it in `open_verified_project` and `on_known_project_reopened` after their `set_worktrees` (C1-A4, C1-A5).
- [ ] T016 [US1] [U24] [U25] In the `SessionMsg::StartMenuOpened` arm of `crates/micold-client/src/main.rs`, ask with `AvailabilityKey::Dir(dir)` where `dir = location_dir(&location)`; when that is `None` (no active project, so no row and no list), ask nothing (C1-A3). Rewrite the arm's comment, which still speaks of "the availability set" and two named events. In `crates/micold-client/src/shell/persist.rs`, keep `Settings(Opened)` asking `AvailabilityKey::Home` (C1-A2).
- [ ] T017 [US1] [A1] [A2] [A5] [U35] [U36] Make each consumer read its own key:
  - `crates/micold-client/src/ui/sidebar.rs`: each worktree row and the Default row pass their `location_dir` to `start_press` and `start_affordance_offers_a_choice`. Update the doc comment on `start_press` about "the set this decision reads".
  - `crates/micold-client/src/ui/mod.rs`: the Settings view takes `availability.home()`, and `session_start_menu_items` reads the menu location's directory.
  - `crates/micold-client/src/features/session.rs`: the `start_menu_toggled` notice reads the location's directory.
- [ ] T018 [US1] Update the user guide:
  - `docs/user-guide/worktrees-and-sessions.md` ("Choosing which AI CLI a session runs"): each row offers the CLIs its own project or worktree provides, including what environment-include adds there, so the chevron can be present on one project's rows and absent on another's.
  - `docs/user-guide/settings.md` ("Default AI CLI"): the field answers for your home directory, and each row answers for its own directory.

- [ ] T030 [US1] [A1] Outer loop green: A1 (US1-1) passes via `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide availability` in `crates/micold-client/src/main_tests.rs`
- [ ] T031 [US1] [A2] Outer loop green: A2 (US1-2) passes in `crates/micold-client/src/main_tests.rs`
- [ ] T032 [US1] [A3] Outer loop green: A3 (US1-3) passes in `crates/micold-client/src/main_tests.rs`
- [ ] T033 [US1] [A4] Outer loop green: A4 (US1-4) passes in `crates/micold-client/src/main_tests.rs`
- [ ] T034 [US1] [A5] Outer loop green: A5 (US1-5) passes in `crates/micold-client/src/main_tests.rs`

**Checkpoint**: US1 is functional and testable on its own. quickstart §B 1–5 pass.

---

## Phase 4: User Story 2 - One row's answer never leaks into another's (Priority: P2)

**Goal**: Opening one row's start list never changes another row's affordance. The behaviour comes
from Phase 2's keyed filing and T016's keyed ask. This phase pins it with the story's own
acceptance tests, written before T013 (test-first; see Dependencies).

**Independent Test**: quickstart.md §B step 6.

### Tests for User Story 2 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T019 [US2] [A6] [A7] In `crates/micold-client/src/main_tests.rs`, test two worktrees, A holding `[ClaudeCode, Pi]` and B holding `[ClaudeCode]`:
  - Opening and closing A's start list (and its answer arriving) leaves B's row with no choice, and B's primary press yields `Start(ClaudeCode)` (US2-1).
  - Opening B's list leaves A's row offering the choice (US2-2).

### Implementation for User Story 2

No new code: T013 and T016 implement it. T019 fails on `main`, where the window-wide set is
overwritten.

- [ ] T035 [US2] [A6] Outer loop green: A6 (US2-1) passes in `crates/micold-client/src/main_tests.rs`
- [ ] T036 [US2] [A7] Outer loop green: A7 (US2-2) passes in `crates/micold-client/src/main_tests.rs`

**Checkpoint**: US1 and US2 both hold.

---

## Phase 5: User Story 3 - The answer follows the events that change it (Priority: P3)

**Goal**: Environment-include changes refresh every row. Rows that disappear drop their answers,
and rows that reappear are asked again. Nothing else asks, and nothing is scheduled.

**Independent Test**: quickstart.md §B steps 7–8. Automated: the tests below.

### Tests for User Story 3 (MANDATORY — Constitution Principle I) ⚠️

- [ ] T020 [US3] [A8] [U32] [U33] [U34] In `crates/micold-client/src/main_tests.rs`, test the env-include refresh (C1-A7, SC-005, US3-1):
  - **This window's save**: `apply_save` switches env-include off, then its `SettingsChanged` echo arrives. The echo re-asks `Home` and every wanted directory, and held answers stay readable until replaced.
  - **Another window's save**: a `SettingsChanged` with changed env-include also re-asks.
  - A `SettingsChanged` changing only `scrollback_lines` asks nothing.
- [ ] T021 [US3] [U28] [U29] [U30] [U31] In `crates/micold-client/src/main_tests.rs`, test row lifecycle (FR-003, FR-004, FR-012):
  - Switching the active project from `P` to `Q` drops `P`'s answers, and switching back asks again.
  - `ForgetConfirmed` drops the forgotten project's answers.
  - A `CatalogChanged` turning a worktree `Missing` drops its answer. One turning it valid again asks for it (deleted/recreated).
  - `ShowAgentWorktreesToggled` on asks for the revealed agent worktrees, and toggling it off drops them.
- [ ] T022 [US3] [A9] [A10] [U25] In `crates/micold-client/src/main_tests.rs`, test a held row's list open (US3-3). Opening the start list on a row whose answer is held still sends a request for that directory, and the new answer replaces the held one (A10). With every row answered, hover, scroll and a `view` call send no availability request (A9).
- [ ] T023 [P] [US3] [A9] [U37] Write the tripwire `crates/micold-client/tests/availability_is_asked_only_on_named_events.rs`, modelled on `tests/refresh_is_only_on_demand.rs` (research R10, SC-003, FR-006):
  - `MARKERS` are `ask_cli_availability`, `sync_cli_availability`, `refresh_cli_availability` and `ClientMsg::AiCliAvailabilityRequest`.
  - `ALLOWED` holds one entry per contract C1 site (C1-A1 to C1-A7, C1-A5b) with its reason, plus the function definitions.
  - The scan fails on an unlisted line or a stale entry, fails on any line under `ui/`, and skips `src/main_tests.rs` and `#[cfg(test)]` modules.

### Implementation for User Story 3

- [ ] T024 [US3] [A8] [U32] [U33] [U34] In the `DaemonMsg::SettingsChanged` arm of `crates/micold-client/src/shell/daemon_sync.rs`, build `EnvIncludeSettings` from the echo. When `availability.env_include_changed(..)`, call a new `refresh_cli_availability(app)`, which asks `Home` and every wanted directory without dropping held answers (research R6, C1-A7).
- [ ] T025 [US3] [U29] [U31] In `crates/micold-client/src/main.rs`, add shell arms:
  - `Message::Project(ProjectMsg::ForgetConfirmed)` runs `sync_cli_availability` after the existing handling (C1-A6).
  - A new `Message::Sidebar(SidebarMsg::ShowAgentWorktreesToggled)` arm applies the reducer through `app.core.update`, then syncs (C1-A5b).
- [ ] T026 [US3] [U28] [U29] [U30] [U31] In `crates/micold-client/src/shell/daemon_sync.rs`, make `sync_cli_availability` prune: `retain(&wanted)` before asking `unasked(&wanted)`. This makes switch, forget, `Missing` and hide drop answers (FR-003, FR-012).
- [ ] T027 [US3] Update `docs/user-guide/settings.md` ("The environment a session starts in"): saving a change to environment-include refreshes which CLIs every sidebar row offers, with no restart.

- [ ] T037 [US3] [A8] Outer loop green: A8 (US3-1) passes in `crates/micold-client/src/main_tests.rs`
- [ ] T038 [US3] [A9] Outer loop green: A9 (US3-2) passes in `crates/micold-client/src/main_tests.rs` and `crates/micold-client/tests/availability_is_asked_only_on_named_events.rs`
- [ ] T039 [US3] [A10] Outer loop green: A10 (US3-3) passes in `crates/micold-client/src/main_tests.rs`

**Checkpoint**: all three stories hold. The tripwire is green with exactly the C1 sites.

---

## Phase 6: Polish & Cross-Cutting Concerns

- [ ] T028 Run quickstart.md §B through the `visual-pass` skill on a private Xvfb display. Record screenshots for steps 1, 2, 6 and 7 and a pass/fail line per step in `specs/033-directory-aware-start-affordance/evidence/quickstart-b.md`.
- [ ] T029 Search `crates/micold-client/src/` for comments that still describe one window-wide availability set or "two named events" (`grep -rn "available_providers\|named events\|availability set" crates/micold-client/src`) and rewrite them against contract C1. The two historical notes naming the removed `Capabilities::available_providers()` (`shell/capabilities.rs`, `features/session.rs`) may stay, because they describe the pre-027 probe and not the window-wide set.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Foundational (Phase 2)** blocks every story: the readers change signature there.
- **US1 (Phase 3)** depends on Phase 2.
- **US2 (Phase 4)** depends on Phase 2. Its test T019 is written **with the US1 tests, before
  T013**, because T013 and T016 are what make it pass.
- **US3 (Phase 5)** depends on US1's `sync_cli_availability` (T014) and asker sites (T013–T016).
- **Polish (Phase 6)** depends on all stories.
- **Outer-loop gates** T030–T039 close each story: a story is complete only when its acceptance
  behaviors (A1–A10) are green. They are numbered after T029 because the id sequence was set before
  the test list (T040 likewise sits in Phase 2 after T005); they sit at the end of their story's phase and belong to that story's milestone.

### Within Each Phase

Tests first, confirmed failing for the stated reason, then implementation (Principle I). T004
before T005 (the readers read the new type), then T040 (consumers onto the new API, today's
behaviour). T006 and T007 follow T040 so that the guards and the migrated tests compile against the
new API. The US1 tests (T008–T012, T019) are written after T040, when they compile and fail for the
stated reason (a row reads home, not its own answer).

### Parallel Opportunities

- T002 (`features_session.rs`) is parallel with T001 and T003, which share
  `directory_availability.rs` and run in sequence.
- T012 (`provider_choice_surfaces.rs`) is parallel with T008–T011 (`main_tests.rs`).
- T023 (the tripwire file) is parallel with T020–T022.

## Parallel Example: User Story 3

```text
Task: "T023 tripwire in crates/micold-client/tests/availability_is_asked_only_on_named_events.rs"
Task: "T020–T022 event tests in crates/micold-client/src/main_tests.rs"
```

## Implementation Strategy

MVP = Phase 2 + US1 (+ US2's pinning test): the reported defect is fixed on every row. US3 then
makes the answer follow environment-include changes and row lifecycle, and pins the asking
discipline structurally. Polish runs the real-GUI pass.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Each row answers for its own directory 🎯 MVP

- **Tasks**: T001–T019, T030–T036, T040
- **Deliverable**: On `main`, a CLI that only one project's environment-include script provides is
  offered on that project's rows (chevron shown, list includes it) and on no other project's. Opening
  Settings, reconnecting or opening another row's list no longer replaces a row's answer.
- **Satisfies**: US1 acceptance scenarios 1–5; US2 acceptance scenarios 1–2; FR-001, FR-002,
  FR-005, FR-006, FR-007, FR-008, FR-009, FR-010, FR-011; FR-003's "at most one per directory, in
  memory" (its "only for rows that exist" pruning is M2); SC-001, SC-002, SC-004
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --tests` (the new `directory_availability.rs`, and the extended and migrated `features_session.rs`, `provider_choice_surfaces.rs`, `missing_cli_is_reported_where_it_is_chosen.rs` and T007 files)
  and `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide availability` (the T008–T011 and T019 cases, each named with `availability`); quickstart §B steps 1–6
- **Depends on**: —

### M2 — The answer follows the events that change it

- **Tasks**: T020–T027, T037–T039
- **Deliverable**: On `main`, saving an environment-include change (in this window or another)
  updates every row's offered CLIs with no restart. Switching, forgetting or hiding rows drops their
  answers, and revealing or recreating them asks again. A tripwire test fails if any code path other
  than the contract's named events asks.
- **Satisfies**: US3 acceptance scenarios 1–3; FR-003 (only for rows that exist), FR-004, FR-012;
  SC-003, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-client --test availability_is_asked_only_on_named_events`
  and `scripts/build-lock.sh cargo test -p micold-client --bin micold-ai-ide availability` (the T020–T022 cases); quickstart §B steps 7–8
- **Depends on**: M1

### M3 — Real-GUI validation

- **Tasks**: T028–T029
- **Deliverable**: `specs/033-directory-aware-start-affordance/evidence/quickstart-b.md` records
  quickstart §B passing end to end against a real service, and no source comment describes the
  window-wide set any more.
- **Satisfies**: SC-001, SC-002, SC-005 (observed end to end)
- **Verify**: read `evidence/quickstart-b.md`; `grep -rn "session.available_providers\|\.available_providers =\|cli_availability_asked" crates/micold-client/src` prints nothing
- **Depends on**: M1, M2
