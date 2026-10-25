# Implementation Plan: Container name minimum length

**Branch**: `feat/1422_container-name-enforce-a-minimum-length-docker` | **Date**: 2026-10-25 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1422-container-name-min-length/spec.md`

## Summary

A core constant `CONTAINER_NAME_MIN = 2` and a pure predicate `container_name_long_enough(&str) -> bool` in `micold_core::daemons`. The client's `runtime_of` (Container arm) refuses a resolved name below the minimum with a new message on the `container_name` field, and `container_name_hint` hides a default below it. No schema change; load paths are untouched, so legacy one-character daemons load as before and are refused only on Save (FR-006). User guide updated.

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: none new; existing `TextField` placeholder

**Storage**: unchanged; `ContainerSettings.container_name` stays a plain string, no migration

**Testing**: `mise run test-core` (new unit tests in `crates/micold-core/tests/daemons_registry.rs`), `cargo test -p micold-client` (`tests/features_daemons.rs`); `mise run gate`

**Target Platform**: Linux, macOS, Windows (pure `chars().count()`, no OS call)

**Project Type**: desktop application (workspace: `micold-core`, `micold-client`, `micold-daemon`)

**Performance Goals**: N/A

**Constraints**: offline; deterministic; same on every OS

**Scale/Scope**: one constant, one predicate, ~3 client edits, 2 doc sentences

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | PASS. Predicate, form-validation and hint tests are written red first (Test strategy). |
| II. Multi-Session | PASS. Pure rule on the name being saved, no shared state; registry uniqueness at commit unchanged. |
| III. Worktree Integration | PASS. Not touched. |
| IV. Local-First (NON-NEGOTIABLE) | PASS. Local string rule; nothing remote. |
| V. Rust + iced | PASS. Constant and predicate in core; no new widget. |
| VI. Cross-Platform | PASS. One rule, no `cfg`; CI runs all three. |
| VII. Documentation | PASS. `docs/user-guide/sandboxed-daemon.md` and `settings.md` updated in the same change. |
| VIII. Reusable UI | PASS. Existing `TextField` and `FormError` reused; no layout change. |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/1422-container-name-min-length/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/ui.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code

```text
crates/micold-core/src/daemons.rs               # new: CONTAINER_NAME_MIN, container_name_long_enough
crates/micold-core/tests/daemons_registry.rs    # new predicate tests
crates/micold-client/src/features/daemons.rs    # runtime_of Container arm, container_name_hint
crates/micold-client/tests/features_daemons.rs  # new tests (no existing test saves a one-character name)
docs/user-guide/sandboxed-daemon.md (paragraph at line 78, the container-name default), docs/user-guide/settings.md (line 514, the Add daemon form)
```

**Structure Decision**: rule in `micold-core` (render-free), wiring in `micold-client`. The registry's `check` is deliberately not changed: it also guards paths that must keep accepting legacy stored names (FR-006), and the form is the only way a container name is entered.

## Design

**Requirement map**

| FR | Where |
|---|---|
| FR-001, FR-004 | `runtime_of` Container arm: after `resolved_container_name()` yields `Some(name)`, if `!container_name_long_enough(&name)` return `FormError { field: "container_name", message: <min-length text built from CONTAINER_NAME_MIN> }` directly (`invalid_field` keeps its key-to-message map and is not edited). Covers typed (trimmed) and defaulted alike; no daemon is built, so nothing is created or changed |
| FR-002, FR-008 | message "A container name needs at least 2 characters." built from `CONTAINER_NAME_MIN`; distinct from "Enter a container name." (still returned when `resolved_container_name()` is `None`) |
| FR-003, FR-005 | `container_name_hint`: `default_container_name(&draft.name).filter(container_name_long_enough)`; placeholder shows exactly when Save would accept the default (uniqueness aside, as in 1116) |
| FR-006 | no load-path change; `begin_edit` seeds the stored `a`; Save runs `runtime_of` and refuses |
| FR-007 | predicate is true for every name of 2 to 63 characters; only the Container arm changes |

**Predicate**: `name.trim().chars().count() >= CONTAINER_NAME_MIN`. `default_container_name` is unchanged (it still returns `Some("a")`), so a one-character default reaches the minimum check and yields the min-length message rather than "Enter a container name." (Scenario 1.1, Edge Cases).

**Order of checks**: resolution (`None` gives "Enter a container name.") then minimum, both before `container_port` and before registry uniqueness, so a too-short name never shows a duplicate message (spec edge case). `form_error`'s `ContainerInUse` branch is untouched.

## Test strategy

| Requirement | Layer |
|---|---|
| Predicate: 0, 1, 2, 63 chars, multibyte counts as characters, surrounding whitespace | core unit, `micold-core` |
| FR-001/002/004, US1 scenarios 1-4, US3 (`x`, ` x `, `xy`), duplicate-vs-short ordering, empty-name messages unchanged | client, `tests/features_daemons.rs` (drives `Msg::Saved`) |
| FR-003/005, US2 | client: `container_name_hint` unit tests (`a` gives empty, `ab` gives `ab`) |
| FR-006, US4 | client: registry with stored `a` loads; edit-open then save refused. Core: registry round-trip keeps `a` |
| FR-007 | client: existing 1116 tests stay green; host/ssh/kube drafts unaffected |
| Geometry gates | existing `container_section_fits` must pass (message line already exists for field errors) |
| Visual | quickstart §B: message shows beneath the container name field |
