# Tasks: Container daemon name defaults to the daemon name

**Input**: `specs/1116-container-name-default/` (spec.md, plan.md, research.md, data-model.md, contracts/ui.md, quickstart.md)

**Tests**: required (Constitution I); each test task precedes its code task and must fail first.

## Phase 1: Foundational (core function)

- [x] T001 [P] Tests for `default_container_name` in `crates/micold-core/tests/daemons_registry.rs`: `Build Box!` -> `Build-Box`, `_web.` -> `web`, `  web  ` -> `web`, `build box` -> `build-box`, `a--b` stays `a--b`, a 70-char name whose 63rd char is `-`/`.`/`_` loses the trailing run, `日本語` and `!!!` -> `None`, 100-char name truncates to 63, runs of replaced chars collapse, a typed run of `-` stays (FR-002, FR-003, FR-011)
- [x] T002 Add `default_container_name(&str) -> Option<String>` and `DEFAULT_CONTAINER_NAME_MAX` (63) to `crates/micold-core/src/daemons.rs`, per plan.md *Design* (FR-002, FR-003, FR-011)

## Phase 2: User Story 1 - Save without a container name (P1) 🎯 MVP

- [x] T003 [US1] Update the tests `a_container_daemon_requires_name_and_container_name_and_its_own_image_only_when_chosen` and `a_container_daemon_marks_name_and_container_name_together` in `crates/micold-client/tests/features_daemons.rs`: the container name is not required (FR-007)
- [x] T004 [US1] Add client tests in `crates/micold-client/tests/features_daemons.rs` driving `Msg::Saved`: empty container name stores the sanitized daemon name; whitespace-only container name defaults; typed `my-ctr` is kept (FR-001, FR-004)
- [x] T005 [US1] In `crates/micold-client/tests/features_daemons.rs`, unit test for the pub pure helper `container_name_hint(&Draft) -> String`: sanitized name when the field is empty, empty when the daemon name is empty or unusable (FR-007)
- [x] T006 [US1] In `crates/micold-client/src/features/daemons.rs`: add `Draft::resolved_container_name`, use it in `runtime_of`, drop `DaemonContainerName` from `required_fields`, add `container_name_hint` (FR-001, FR-003, FR-004, FR-007)
- [x] T007 [US1] In `crates/micold-client/src/ui/settings/daemons.rs`: label no longer required, placeholder from `container_name_hint` (FR-007); checked by quickstart §B step 1-2

## Phase 3: User Story 4 - Uniqueness enforced (P1)

- [x] T008 [US4] Tests in `crates/micold-client/tests/features_daemons.rs`: a defaulted-name clash (`devbox`) lands on `container_name`; a port-only clash lands on `port`; `a b` vs `a-b` refused; truncation collision refused; no daemon created; the registry `check` still refuses a duplicate resolved name (FR-005, FR-010, SC-004)
- [x] T009 [US4] In `form_error` (`crates/micold-client/src/features/daemons.rs`), compare the holder's container name with `draft.resolved_container_name()` (FR-010)

## Phase 4: User Stories 2 and 3 - Fixed at save, invalid names refused (P2)

- [x] T010 [US2] [US3] Tests in `crates/micold-client/tests/features_daemons.rs`: rename after save keeps the container name; editing shows the stored name as a value (rename and edit-open cases are regression guards, expected green at once; the `日本語` refusal and clearing the stored name while editing, which derives from the current daemon name, are red-first); `日本語` with an empty field refused with "Enter a container name." on `container_name`; empty daemon name keeps "A daemon needs a name."; host/ssh/kube drafts unchanged (FR-003, FR-006, FR-008, FR-009)

## Phase 5: Docs and gate

- [x] T011 Update `docs/user-guide/settings.md` and `docs/user-guide/sandboxed-daemon.md`: container name is optional, defaults to the sanitized daemon name, stored at save (FR-007; Constitution VII)
- [x] T012 Run `mise run gate`; confirm `container_section_fits` still passes

## Dependencies

T002 after T001. T006 after T002, T003-T005. T007 after T006. T009 after T006, T008. T010's red-first cases need T006 and T009. Tasks sharing a file run in order. T012 last.

## Implementation Strategy

Core function first (T001-T002), then the client wiring (T003-T010), docs, gate. The whole feature is one small diff, so it ships as one milestone.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Container name defaults to the daemon name 🎯 MVP

- **Tasks**: T001–T012
- **Deliverable**: Saving a container daemon with an empty container name stores the sanitized daemon name; the field is not required and shows the default as a placeholder; duplicates and unusable names are refused on the container name field.
- **Satisfies**: US1–US4 acceptance scenarios; FR-001–FR-011; SC-001–SC-004
- **Verify**: `mise run test-core`, `cargo test -p micold-client --test features_daemons`, quickstart §A and §B
- **Depends on**: —
- **Tier**: full

## Phase 6: TDD remediation

The TDD audit (`tdd/verification.md`, verdict FAIL) is not cleared by tasks alone: finding 1 (no recorded red; tests and source in one commit) cannot be repaired after the fact, so the feature stays FAIL until the maintainer accepts it or the behaviors are re-proven. The tasks below fix the repairable findings.

- [ ] T013 Finding 1: for B5, B8 and B11, prove red by running each test against the pre-feature source (`git stash`-free: check out `crates/micold-*/src` from `2ecf9e9ea` in a scratch worktree) and record the failing output in `specs/1116-container-name-default/tdd/cycle-log.md`; `scripts/build-lock.sh cargo test -p micold-client --test features_daemons default_container_name` fails there and passes on HEAD — **not done: the red phase cannot be recovered after the fact; accepted as TEST_AFTER**
- [ ] T014 [P] Finding 2: add a rendering test in `crates/micold-client/tests/` that opens the container form with name `Build Box!` and an empty container name and asserts the field shows placeholder `Build-Box` and has no required marker (`crates/micold-client/src/ui/settings/daemons.rs:411`); `scripts/build-lock.sh cargo test -p micold-client --test daemons_section_geometry` — **not done: no render-level harness reads a text_input placeholder; FR-007 is covered through `container_name_hint`**
- [ ] T015 [P] Finding 3: in `an_empty_daemon_name_keeps_its_own_message` (`crates/micold-client/tests/features_daemons.rs:1530`) assert the actual "A daemon needs a name." surfacing (the field's `Missing` requirement or `form_error` message, whichever the code produces) so changing the text fails it; `cargo test -p micold-client --test features_daemons an_empty_daemon_name` — **not done: an empty name is stopped by the required-field gate before `form_error` builds the message, so it cannot be asserted without a production change; a comment in the test says so**
- [x] T016 [P] Finding 4: split the Container case out of the loop at `crates/micold-client/tests/daemons_feature.rs:822` into `an_unresolvable_container_name_keeps_connection_open`; `cargo test -p micold-client --test daemons_feature a_required_field_left_empty`
- [x] T017 [P] Finding 5: make `the_registry_still_refuses_a_duplicate_resolved_name` (`crates/micold-client/tests/features_daemons.rs:1475`) save a defaulted-name daemon through `Msg::Saved` then `registry.add` the same resolved name, or delete it as redundant (done: deleted; explicit-name duplicates are covered by `crates/micold-core/tests/daemons_registry.rs:91`, the defaulted-name clash by the client save-clash test); `cargo test -p micold-client --test features_daemons`
- [x] T018 [P] Finding 6/7/8: mark the rename and typed-name tests as regression guards in their names or comments, comment the `70` and `7001` literals, and label the five asserts at `crates/micold-core/tests/daemons_registry.rs:203-211`; `mise run test-core`
