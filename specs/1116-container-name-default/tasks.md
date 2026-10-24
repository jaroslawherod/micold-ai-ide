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
