# Implementation Plan: Container daemon name defaults to the daemon name

**Branch**: `feat/1116_container-daemon-default-the-container-name-to-the` | **Date**: 2026-10-23 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1116-container-name-default/spec.md`

## Summary

A pure function `default_container_name(&str) -> Option<String>` in `micold_core::daemons` applies
FR-002. The client's `runtime_of` resolves the container name once (typed, trimmed, else the default)
and `form_error` compares the holder against that resolved name, so a duplicate of a defaulted name
lands on the container name field (FR-010). The form drops the container name from `required_fields`
(FR-007) and shows the default as the field's placeholder. The registry's `check` is unchanged: it
already refuses an empty name and a duplicate, and it runs on the resolved value (FR-005, SC-004).
User guide updated (FR-007 wording).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14 (existing `TextField` placeholder); no new crate

**Storage**: the daemon registry in the local settings file; `ContainerSettings.container_name` already stores a resolved string, so no schema change, no migration

**Testing**: `cargo test -p micold-core` (`tests/daemons_registry.rs`, new unit tests for the function), `cargo test -p micold-client` (`tests/features_daemons.rs`); `mise run gate`

**Target Platform**: Linux, macOS, Windows desktop (the function is pure ASCII string handling, no OS call)

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`, `micold-daemon`)

**Performance Goals**: N/A (runs per keystroke on a short string)

**Constraints**: offline; deterministic; the same output on every OS

**Scale/Scope**: one new core function, ~4 client edits, 2 doc paragraphs

## Constitution Check

| Principle | Status |
|---|---|
| I. Test-First | PASS. Function tests, form-validation tests and the required-fields test change before the code (see Test strategy). |
| II. Multi-Session | PASS. No new state. Two windows saving the same sanitized name: the registry check runs at commit on the shared registry, one wins (spec edge case). |
| III. Worktree Integration | PASS. Not touched. |
| IV. Local-First (NON-NEGOTIABLE) | PASS. Pure local string function; the stored value is the existing settings field. |
| V. Rust + iced | PASS. `Option<String>` makes "no usable default" explicit; the registry still rejects an empty name, so an invalid default is unrepresentable in storage. |
| VI. Cross-Platform | PASS. One ASCII-only function, no `cfg`; tests run on all three in CI. |
| VII. Documentation | PASS. `docs/user-guide/settings.md` and `sandboxed-daemon.md` updated in the same change. |
| VIII. Reusable UI | PASS. Existing `TextField` placeholder reused; no new widget. |

No violations; Complexity Tracking is empty.

## Project Structure

### Documentation (this feature)

```text
specs/1116-container-name-default/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/ui.md
└── tasks.md             # /speckit-tasks, not this command
```

### Source Code

```text
crates/micold-core/src/daemons.rs                 # new: default_container_name (FR-002, FR-003, FR-011)
crates/micold-core/tests/daemons_registry.rs      # new tests for it
crates/micold-client/src/features/daemons.rs      # Draft::required_fields, runtime_of, form_error; new Draft::resolved_container_name
crates/micold-client/src/ui/settings/daemons.rs   # placeholder, label no longer required
crates/micold-client/tests/features_daemons.rs    # updated and new tests
docs/user-guide/settings.md, docs/user-guide/sandboxed-daemon.md
```

**Structure Decision**: logic in `micold-core` (render-free, `mise run test-core`), wiring in `micold-client`.

## Design

**Requirement map**

| FR | Where |
|---|---|
| FR-001, FR-004 | `Draft::resolved_container_name` (client): trimmed typed value if non-empty, else `default_container_name(&draft.name)`; `runtime_of` stores it |
| FR-002, FR-011 | `default_container_name` (core) |
| FR-003 | `resolved_container_name` returns `None` when both are empty; `runtime_of` then returns `invalid_field("container_name")` ("Enter a container name.") |
| FR-005 | unchanged `DaemonRegistry::check` on the resolved `ContainerSettings` |
| FR-006, FR-008 | stored string; `begin_edit` already seeds the draft with the stored name; nothing recomputes it for an existing entry |
| FR-007 | remove the `DaemonContainerName` entry from `required_fields`; placeholder in `ui/settings/daemons.rs` |
| FR-009 | only the `DraftRuntime::Container` arm changes |
| FR-010 | `form_error` `ContainerInUse` compares the holder's name with `draft.resolved_container_name()` (`Some(c) if Some(c) == resolved.as_deref()`) instead of `draft.container_name`; a port-only clash still lands on `port` |

**`default_container_name(name: &str) -> Option<String>`**: trim; map each char outside
`[A-Za-z0-9_.-]` to `-` and collapse a run of replaced chars into one `-` (a run of valid `-` stays as typed);
truncate to 63 (ASCII only by then, so byte truncation is safe); `trim_start_matches` non-alphanumerics;
`trim_end_matches` `-_.`; `None` if empty. Constant `DEFAULT_CONTAINER_NAME_MAX: usize = 63`.

**Edge**: while editing, a user who clears the stored name gets the default of the current daemon name (a fresh derivation); this is the only way a rename can change the name and it is explicit user action.

**Spec note handled**: `form_error`'s `same_name` test (daemons.rs ~965) compared with the raw draft field; with an empty draft it would pick `port`. Fixed by the FR-010 row above and covered by a test.

## Test strategy

| Requirement | Layer |
|---|---|
| FR-002, FR-003, FR-011 (examples `Build Box!`, `_web.`, `  web  `, `日本語`, `!!!`, 100-char, collapse) | core unit, `micold-core` |
| FR-001, FR-004, FR-005, FR-006, FR-010, edge cases (whitespace container name, empty daemon name, `a b` vs `a-b`, collision after truncation) | client, `tests/features_daemons.rs` (drives `Msg::Saved`) |
| FR-007 not required | client: updated `a_container_daemon_requires_name_and_container_name...` and `...marks_name_and_container_name_together` tests |
| FR-007 placeholder text | client: a pure helper `container_name_hint(&Draft) -> String` is unit-tested; view only passes it |
| FR-008, FR-009 | client: edit-open shows stored value; host/ssh/kube drafts unchanged |
| Geometry gates | existing `container_section_fits` must still pass (no layout change: placeholder only) |
| Visual | quickstart §B: placeholder looks like a hint, not a value |
