# Implementation Plan: Required-field markers in forms

**Branch**: `claude/project-thread-rkhzho` | **Date**: 2026-10-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/937-required-field-markers/spec.md`

## Summary

Every form field the form refuses empty carries `*` after its label; each form with such a field
shows `* required` above its first field; an empty required field shows the error state with "This
field is required" on blur and on a submit attempt, never on a fresh form. The decision (which
fields are required now, which the user has left, whether a submit was attempted) is render-free:
a generic `micold_core::required` module plus one `required_fields()` per draft, mirroring that
draft's existing validator. The drawing is in the material components: one `.required(..)` builder
on every field, `Select::on_blur`, and a `RequiredLegend` component. Create and Start runs become
enabled (FR-012, D2 default). Research: [research.md](research.md).

## Technical Context

**Language/Version**: Rust, stable toolchain (via `mise`)

**Primary Dependencies**: iced 0.14; no new crate

**Storage**: none; marks are runtime view state on open drafts (Principle IV)

**Testing**: `cargo test` via `mise run test-core` (core module) and `scripts/gate.sh` / `mise run
gate` (client feature tests in `crates/micold-client/tests/`, in-crate anatomy tests, new
`tests/required_call_sites.rs`). Test layer per requirement: [research.md R9](research.md)

**Target Platform**: Linux, macOS, Windows desktop

**Project Type**: desktop application (Cargo workspace: `micold-core`, `micold-client`,
`micold-daemon`)

**Performance Goals**: none new; `required_fields()` is a handful of comparisons per render

**Constraints**: the marker is a text character in the label font (FR-003, Principle VI); legend,
marker and message come only from the shared components (FR-008)

**Scale/Scope**: 1 core module, 4 component changes, 6 drafts (plus the daemons `State`), 9 view files (research R5), 1
showcase entry, 1 source-scan gate, 4 user-guide pages

## Constitution Check

- [x] **I. Test-First**: all decision logic (marks, requirement per field, blur routing, precedence)
  is in `micold-core` or render-free feature modules and is test-first; component drawing is
  covered by in-crate anatomy tests; only view wiring relies on the quickstart Part B. PASS
- [x] **II. Multi-Session**: marks live on each draft and die with it; a blur is routed only to the
  draft owning the `FieldId` (R3). PASS
- [x] **III. Worktree Integration**: no file or VCS operation changes. PASS
- [x] **IV. Local-First**: nothing persisted or sent. PASS
- [x] **V. Rust + iced**: `FieldRequirement` is one enum, so a `Missing` state without a
  marker is unrepresentable. PASS
- [x] **VI. Cross-Platform**: an ASCII `*` in the label font; no platform branch. PASS
- [x] **VII. Documentation**: user guide updated (D6). PASS
- [x] **VIII. Reusable UI**: the behaviour is added to `FormField`/`FieldText`, `Select`, and a new
  `RequiredLegend` builder; no form draws its own. PASS

Post-design re-check: unchanged, all PASS.

## Design

### D1 — Core: `micold_core::required` — R1, R6

`FieldRequirement`, `Requirement`, `RequiredMarks<K>`, `LEGEND`, `MESSAGE`, `MARKER` as in
[data-model.md](data-model.md). Unit tests in the module.

### D2 — Components — R1, R2, R6; contract C1–C4

`FieldText.required` + `.required(..)` in `field_text_builders!(label_supporting)`; `FormField`
appends the marker and, for `Missing` with no own error, draws invalid with `MESSAGE` in the
supporting slot. `Select::on_blur` (edge in `update` on `focused || open`). `RequiredLegend` in
`ui/material/required_legend.rs`, exported from `material/mod.rs`. `TrackFocus for Select` in
`ui/focus.rs`. Docs: `docs/development/component-library.md`.

### D3 — Blur routing — R3

New `FieldId` variants; `DaemonRepository(DaemonId)`; `Outcome::FieldBlurred`; new
`features/required.rs` with `blurred(state, id)`. Typeahead's `BranchDismissed` also calls
`leave(AddWorktreeBranch)`.

### D4 — Per-form requirement and submit — R5, R7

For each draft in data-model: `required` marks, `required_fields()` (tested against its validator),
`attempt()` in the submit handler. Emptiness errors stop being shown as form lines: the worktree
form's `NoType`/blank-name line, the parallel dialog's `EmptyPrompt`/`NoType`/`EmptyName`/
`NoBaseBranch` (its `revalidate` keeps only `RunCount`, `ProviderNotOffered`; `BadName` goes on the
Name field), the rename dialogs' line under the field, and settings/daemon `FieldError`s for empty
values. Create's `can_submit()` drops `preview().is_ok()`; Start runs is always pressable.

### D5 — Views (glue) — R5

Each inventory view file passes `.required(draft.required.state(id, req))` on its required fields
and pushes `RequiredLegend` first when `legend(..)` is true. Layout snapshots re-recorded for the
added legend row (R8).

### D6 — Showcase, gate, docs

Showcase entry (C6); `tests/required_call_sites.rs` (C5); user guide: a short "Required fields"
note in `docs/user-guide/worktrees-and-sessions.md` (add-worktree form), `parallel-runs.md`,
`settings.md` (sections and daemon form), and `project-selection.md` (rename).

## Requirement map

| Requirement | Design |
|---|---|
| FR-001, FR-003 | D2 marker in label text |
| FR-002 | D4 `required_fields()` tested against validators |
| FR-004 | D1 `legend`, D2 `RequiredLegend`, D5 |
| FR-005, FR-010 | D2 `Missing` drawing and precedence; D4 emptiness errors moved to fields |
| FR-006 | D1 marks, D3 blur routing, D4 `attempt()` |
| FR-007 | D1 `state` is pure over current value |
| FR-008, SC-006 | D2 components, D6 gate C5 |
| FR-009 | research R5 inventory, D5 |
| FR-011 | D6 showcase |
| FR-012 | D4 buttons |

## Pending decisions

D2 = A and D3 = A, confirmed by jaro 2026-10-17: Create and Start runs are enabled (R7/FR-012), and
errors show on blur and on a submit attempt.

## Project Structure

### Documentation (this feature)

```text
specs/937-required-field-markers/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/required-field.md
└── tasks.md            # tasks unit
```

### Source Code (repository root)

```text
crates/micold-core/src/required.rs                 # new (D1)
crates/micold-core/src/lib.rs                      # pub mod required
crates/micold-client/src/ui/material/form_field.rs # FieldText.required, marker, Missing (D2)
crates/micold-client/src/ui/material/select.rs     # on_blur (D2)
crates/micold-client/src/ui/material/required_legend.rs  # new (D2)
crates/micold-client/src/ui/material/{mod,text_field_anatomy,select_anatomy,form_field_anatomy}.rs
crates/micold-client/src/ui/focus.rs               # TrackFocus for Select
crates/micold-client/src/features/{window,mod,required}.rs   # FieldId, Outcome, routing (D3)
crates/micold-client/src/features/{worktree_form,runs,project,worktree,settings,daemons}.rs  # D4
crates/micold-client/src/app.rs                    # apply Outcome::FieldBlurred
crates/micold-client/src/ui/{worktree_form,parallel_dialog,rename_dialog,rename,worktree_rename}.rs  # D5
crates/micold-client/src/ui/settings/{terminal,environment,github,jira,daemons,container_fields}.rs
crates/micold-client/src/showcase/sections/controls.rs   # D6
crates/micold-client/tests/required_call_sites.rs  # new gate (D6)
crates/micold-client/tests/features_*.rs           # D4 tests, incl. new features_required.rs (D3)
crates/micold-client/tests/{layout_snapshot.rs,gates/,showcase_completeness.rs}  # R8, C6
docs/user-guide/{worktrees-and-sessions,parallel-runs,settings,project-selection}.md
docs/development/component-library.md
```

**Structure Decision**: existing workspace layout; decision logic in `micold-core` and the client's
render-free `features/` modules, drawing in `ui/material/`, wiring in `ui/`.

## Complexity Tracking

None.
