# Tasks: Container name minimum length

**Input**: `specs/1422-container-name-min-length/` (spec.md, plan.md, research.md, data-model.md, contracts/ui.md, quickstart.md)

**Tests**: mandatory (Constitution I): each test task is written red first and precedes its implementation task.

Format: `[ID] [P?] [Story] Description`. Paths are repo-relative.

## Phase 1: Foundational (core rule)

- [x] T001 [US1] Write failing predicate tests in `crates/micold-core/tests/daemons_registry.rs`: `container_name_long_enough` is false for 0 and 1 characters, true for 2 and 63, counts multibyte characters not bytes, ignores surrounding whitespace (`" x "` false, `" xy "` true).
- [x] T002 [US1] Add `CONTAINER_NAME_MIN = 2` and `container_name_long_enough(&str) -> bool` (`name.trim().chars().count() >= CONTAINER_NAME_MIN`) to `crates/micold-core/src/daemons.rs`. `default_container_name` is unchanged.
- [x] T003 [US4] Add a core test in `crates/micold-core/tests/daemons_registry.rs` that a registry with a stored one-character container name `a` round-trips (load then save) unchanged (FR-006).

## Phase 2: User Story 1 + 3 — refuse a too-short name at Save (P1) 🎯 MVP

**Goal**: a resolved container name of 1 character, defaulted or typed, is refused at Save with the min-length message.

- [x] T004 [US1] Write failing client tests in `crates/micold-client/tests/features_daemons.rs` (drive `Msg::Saved`): daemon name `a` with empty container name is refused with "A container name needs at least 2 characters." on field `container_name` and no daemon is added (scenarios 1.1-1.4); daemon `build` with container name `x` and ` x ` refused (US3); `xy` accepted; empty container name with empty/unusable daemon name still gives "Enter a container name." / "A daemon needs a name."; a too-short name never shows the duplicate message when `a` already exists; host/ssh/kube drafts unaffected (FR-007).
- [x] T005 [US1] In `crates/micold-client/src/features/daemons.rs` `runtime_of` Container arm: after `resolved_container_name()` yields `Some(name)`, return `FormError { field: "container_name", message }` when `!container_name_long_enough(&name)`, the message built from `CONTAINER_NAME_MIN`, before the port and uniqueness checks. Do not edit `invalid_field`.
- [x] T006 [US1] Update `docs/user-guide/sandboxed-daemon.md` (container-name default paragraph, ~line 78) and `docs/user-guide/settings.md` (Add daemon form, ~line 514): the container name needs at least 2 characters, and a one-character daemon name needs a container name typed, and the field shows no placeholder in that case.

## Phase 3: User Story 2 — placeholder matches Save (P2)

- [x] T007 [US2] Write failing `container_name_hint` tests in `crates/micold-client/tests/features_daemons.rs`: daemon name `a` gives an empty hint, `ab` gives `ab`.
- [x] T008 [US2] In `crates/micold-client/src/features/daemons.rs`, `container_name_hint`: `default_container_name(&draft.name).filter(|n| container_name_long_enough(n))` (FR-003, FR-005).

## Phase 4: User Story 4 — legacy daemons (P1)

- [x] T009 [US4] Add a client test in `crates/micold-client/tests/features_daemons.rs`: a registry with container name `a` loads and lists unchanged; `begin_edit` then save is refused with the min-length message (FR-006). No load-path code change expected.

## Dependencies

T002 after T001; T005 after T004 and T002; T008 after T007 and T002; T006 after T005 and T008; T009 after T005. Existing `container_section_fits` geometry gate must stay green.

## Implementation Strategy

One small change in two crates (nine tasks): deliberately one milestone, a deliberate deviation from milestones.md rule 2, since each story alone is two or three tasks and no half is observable apart.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Refuse container names under 2 characters 🎯 MVP

- **Tasks**: T001–T009
- **Deliverable**: Saving a container daemon whose resolved container name is one character is refused with "A container name needs at least 2 characters."; the placeholder is hidden for such a default; stored one-character daemons still load.
- **Satisfies**: US1 acceptance scenarios 1–4; US2; US3; US4; FR-001–FR-008
- **Verify**: `mise run test-core`; `cargo test -p micold-client --test features_daemons`; quickstart Part A
- **Depends on**: —
- **Tier**: full

## Phase 5: TDD remediation

Verdict is PASS_WITH_GAPS; nothing blocks, but close these.

- [x] T010 [US1] Finding 2: add a test in `crates/micold-client/tests/features_daemons.rs` that daemon name `a!` with an empty container name is refused with the min-length message; prove with `cargo test -p micold-client --test features_daemons`.
- [x] T011 [US1] Findings 3-4: add a Save test for daemon name `ab` (stores `ab`) and a refuse-then-type-`ab` sequence in one form; same command.
- [x] T012 [US1] Finding 4: assert a Kubernetes (and host) draft is unaffected by the minimum; same command.
