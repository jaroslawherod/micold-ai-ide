---

description: "Task list for required-field markers in forms (937)"
---

# Tasks: Required-field markers in forms

**Input**: Design documents from `specs/937-required-field-markers/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/required-field.md, quickstart.md

**Tests**: Per Constitution Principle I, test tasks are mandatory and come first in each phase:
write them, see them fail for the right reason, then implement.

**Documentation**: Per Principle VII, each story's user-guide task ships in that story's milestone.

**Cross-platform**: Per Principle VI, the marker is an ASCII `*` in the label font; no platform
branch anywhere in this feature.

Paths: `core/` = `crates/micold-core/src/`, `client/` = `crates/micold-client/src/`,
`ctests/` = `crates/micold-client/tests/`. Names (`FieldRequirement`, `Requirement`,
`RequiredMarks`, `LEGEND`, `MESSAGE`, `MARKER`) are those of [data-model.md](data-model.md); C1–C6
are the clauses of [contracts/required-field.md](contracts/required-field.md); R1–R9 are entries of
[research.md](research.md).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: US1, US2, US3 from spec.md

---

## Phase 1: Setup

None: no new crate, dependency or tool (plan *Technical Context*).

---

## Phase 2: Foundational (blocking prerequisites)

**Purpose**: the render-free requirement type and the label marker in the shared components, which
every story uses.

- [ ] T001 [P] Write failing unit tests in `core/required.rs` (module `tests`): `Requirement::text(true, "  ")` is empty (trimmed) and `Requirement::text(true, " a ")` is not; `Requirement::choice(true, None::<&u8>)` is empty; on `RequiredMarks::default()` a required empty field reads `Required`, never `Missing` (FR-006 "never on a fresh form"); after `leave(k)` only `k` reads `Missing`; after `attempt()` every required empty field reads `Missing`; a required non-empty field reads `Required` with the marks unchanged (FR-007); a not-required field reads `Optional` even after `attempt()` and `leave(k)` (edge case *stops being required*); `legend(..)` is true exactly when one requirement has `required: true`; `LEGEND == "* required"`, `MESSAGE == "This field is required"`, `MARKER == "*"`
- [ ] T002 Implement `FieldRequirement { Optional, Required, Missing }`, `Requirement { required, empty }` with `text`/`choice`, `RequiredMarks<K: Ord + Copy> { left: BTreeSet<K>, attempted: bool }` with `leave`, `attempt`, `state`, `legend`, and the three constants in `core/required.rs`; add `pub mod required;` to `core/lib.rs` (depends on T001)
- [ ] T003 [P] Write failing anatomy tests (C1) in `client/ui/material/form_field_anatomy.rs`, `client/ui/material/text_field_anatomy.rs` and `client/ui/material/select_anatomy.rs`: `.required(FieldRequirement::Required)` renders the label text `"Name*"` both resting (empty) and floating (filled), same role and tint as without it; `Optional` (the default) leaves the label unchanged; a field with no label gets no marker; `TextField`, `Select` and `Typeahead` all accept `.required(..)`
- [ ] T004 Add `required: FieldRequirement` (default `Optional`) to `FieldText` and a `.required(FieldRequirement)` builder to the `label_supporting` arm of `field_text_builders!` in `client/ui/material/form_field.rs`; `FormField` appends `required::MARKER` to a present label for `Required` and `Missing` (the `Missing` error drawing is T031) (depends on T002, T003)

**Checkpoint**: core module and label marker exist; no form uses them yet.

---

## Phase 3: User Story 1 — See which fields must be filled before submitting (P1) 🎯 MVP

**Goal**: every field a form refuses empty carries `*` in its label; no optional field does.

**Independent Test**: open each FR-009 form without submitting; every required field's label ends
in `*`, no optional field's does (quickstart §B 2–5, markers only).

### Tests for User Story 1, dialogs (write first, see them fail)

- [ ] T005 [P] [US1] Write failing tests in `ctests/features_worktree_form.rs`: `WorktreeForm::required_fields()` lists, in display order, `(FieldId::AddWorktreeType, _)` and `(FieldId::AddWorktreeName, _)` for the new-branch and issue sources and `(FieldId::AddWorktreeBranch, _)` for the existing-branch source, each `required: true`; Ticket, daemon, AI CLI and first prompt are not listed; for a table of form states, a listed field has `required && empty` exactly when `preview()` refuses it for emptiness: `NamingError::NoType` (Type), `EmptyNameAfterSlug` with the name blank after trim (Name), `EmptyNameAfterSlug` with no branch picked (Branch); a non-blank name that slugs to nothing (e.g. `!!!`) is not `empty`, and `preview()` still refuses it with `EmptyNameAfterSlug`, its own non-emptiness error (FR-002, FR-010, R5)
- [ ] T006 [P] [US1] Write failing tests in `ctests/features_runs.rs`: `ParallelDialog::required_fields()` lists `RunPrompt`, `RunType`, `RunName`, `RunBaseBranch` in display order, all `required: true`, and each is `empty` exactly when `ParallelDialog::validate` reports `Invalid::EmptyPrompt`, `NoType`, `EmptyName`, `NoBaseBranch` respectively; Ticket and per-run Provider are not listed (R5)
- [ ] T007 [P] [US1] Write failing tests in `ctests/features_project.rs` and `ctests/features_worktree.rs`: `RenameDraft::required_fields()` is `[(RenameProjectName, required)]` and `WorktreeRenameDraft::required_fields()` is `[(RenameWorktreeName, required)]`, `empty` exactly when `micold_core::project::RenameError::{Empty, Whitespace}` is returned for the draft's value (R4, R5)

### Implementation for User Story 1, dialogs

- [ ] T008 [US1] Add `FieldId::AddWorktreeType`, `AddWorktreeBranch`, `RunType`, `RunBaseBranch` in `client/features/window.rs` (data-model *FieldId changes*); find every exhaustive `match` on `FieldId` with LSP `findReferences` and extend it
- [ ] T009 [P] [US1] Add `pub required: RequiredMarks<FieldId>` (default on open) and `required_fields(&self) -> Vec<(FieldId, Requirement)>` to `WorktreeForm` in `client/features/worktree_form.rs`, mirroring `preview()`'s trim rule (depends on T002, T005, T008)
- [ ] T010 [P] [US1] Same for `ParallelDialog` in `client/features/runs.rs`, mirroring `ParallelDialog::validate` (depends on T002, T006, T008)
- [ ] T011 [P] [US1] Same for `RenameDraft` in `client/features/project.rs` and `WorktreeRenameDraft` in `client/features/worktree.rs`, mirroring `RenameError` (depends on T002, T007)
- [ ] T012 [US1] Wire the views: in `client/ui/worktree_form.rs` and `client/ui/parallel_dialog.rs` pass `.required(draft.required.state(id, req))` to each field `required_fields()` lists (Type and Base branch are `Select`s, Branch is the `Typeahead`); in `client/ui/rename_dialog.rs` carry the `FieldRequirement` in `RenameSpec` (filled by `client/ui/rename.rs` and `client/ui/worktree_rename.rs`) and pass it to the name field (depends on T004, T009–T011)
- [ ] T013 [P] [US1] Create the source-scan gate `ctests/required_call_sites.rs`, modelled on `ctests/field_focus_call_sites.rs`: fail when a `.rs` file under `client/ui/` outside `client/ui/material/` has a `.label(` string argument ending in `*` (C5, marker part; the legend and message parts are T024 and T035)
- [ ] T014 [P] [US1] Add a *Required field* entry to `client/showcase/sections/controls.rs`: a `TextField` and a `Select`, each `Required` resting (empty) and `Required` filled (C6, marker part); list the entry in `ctests/showcase_completeness.rs`
- [ ] T015 [P] [US1] Document `.required(FieldRequirement)` (C1) in `docs/development/component-library.md`; add a short *Required fields* note ("fields marked `*` must be filled") to `docs/user-guide/worktrees-and-sessions.md` (add-worktree form), `docs/user-guide/parallel-runs.md` and `docs/user-guide/project-selection.md` (rename)

**Checkpoint (M1)**: the add-worktree form, run-in-parallel dialog and both rename dialogs mark their required fields.

### Tests for User Story 1, settings and daemons (write first, see them fail)

- [ ] T016 [P] [US1] Write failing tests in `ctests/features_settings.rs`: `SettingsDraft::required_fields(section)` returns, for `Terminal`, `SettingsScrollback`; for `Environment`, `SettingsEnvIncludeTimeout`, `SettingsLongTaskThreshold`, and `SettingsPlanUsageWarn` only while that field is shown; for `GitHub`, one `IssueMappingLabel(i)` per mapping entry; for `Jira`, `SettingsJiraSite`, `SettingsJiraKey`, `SettingsJiraEmail` required only while Jira is enabled and any of the four fields holds input or a connection is stored, and `SettingsJiraToken` under the same condition and only when no token is stored; for `Appearance`, nothing; for each section, a listed field is `required && empty` exactly when `SettingsDraft::validate` (`scrollback()`, `timeout()`, `long_task_threshold()`, `WarnPercent::parse`, `MappingErrorKind::Blank`, `jira_action()`/`JiraConnection::parse`, `ApiToken::parse`) refuses it for emptiness (R5)
- [ ] T017 [P] [US1] Write failing tests in `ctests/features_daemons.rs`: the daemon `Draft::required_fields()` lists `DaemonName` always; `DaemonSshHost` only while the runtime is SSH, and switching back to local drops it (US1 scenario 3); `DaemonContainerName` and `DaemonImage` for a container daemon, `DaemonImagePath` only for the imported-file image source; Port, SSH User and Key file never; each `empty` exactly when `runtime_of`/`form_error` refuses it (`RegistryError::BlankName`, `SshTarget::new` empty host, `RegistryError::InvalidField("container_name")`, `ImageRefError::Empty`, `ImageSourceProblem::MissingArchivePath`); the repository step's `FieldId::DaemonRepository(id)` is required, `empty` exactly when `RemotePathError::Empty`, and two daemons' ids are distinct fields
- [ ] T018 [US1] Change `FieldId::DaemonRepository` to `DaemonRepository(DaemonId)` in `client/features/window.rs` and update every use (LSP `findReferences`), in `client/features/daemons.rs` and `client/ui/settings/daemons.rs` among others
- [ ] T019 [P] [US1] Add `pub required: RequiredMarks<FieldId>` and `required_fields(&self, section: SettingsSection) -> Vec<(FieldId, Requirement)>` to `SettingsDraft` in `client/features/settings.rs`; removing or moving a GitHub mapping entry clears every `IssueMappingLabel(_)` mark (data-model) (depends on T002, T016)
- [ ] T020 [P] [US1] Add `pub required: RequiredMarks<FieldId>` and `required_fields(&self)` to the daemon `Draft`, and `repository_marks: RequiredMarks<FieldId>` to the daemons `State`, in `client/features/daemons.rs` (depends on T002, T017, T018)
- [ ] T021 [US1] Wire the views `client/ui/settings/terminal.rs`, `environment.rs`, `github.rs`, `jira.rs`, `daemons.rs` (form and `repository_step`) and `container_fields.rs`: pass `.required(..)` on each listed field (depends on T004, T019, T020)
- [ ] T022 [P] [US1] Add the *Required fields* note to `docs/user-guide/settings.md` (sections and the daemon form, the SSH Host appearing with the SSH choice)

**Checkpoint (M2)**: every FR-009 form marks its required fields (SC-001, SC-002).

---

## Phase 4: User Story 2 — Understand what the marker means (P2)

**Goal**: each form with a required field shows `* required` once, above its first field.

**Independent Test**: each form with a required field shows the legend exactly once at the top;
Settings › Appearance and Jira with nothing typed show none (quickstart §B 2–5, legend).

### Tests for User Story 2 (write first, see them fail)

- [ ] T023 [P] [US2] Write a failing anatomy test in `client/ui/material/required_legend.rs`: `RequiredLegend::new(roles)` renders the text `required::LEGEND` in `TypeRole::Caption` with the muted role (C3, R6)
- [ ] T024 [P] [US2] Extend `ctests/required_call_sites.rs`: fail when a file under `client/ui/` outside `material/` contains the `"* required"` literal, and when one of the inventory view files (`ui/worktree_form.rs`, `ui/parallel_dialog.rs`, `ui/rename_dialog.rs`, `ui/settings/{terminal,environment,github,jira,daemons}.rs`) does not call `RequiredLegend` (C5, SC-003, SC-006)

### Implementation for User Story 2

- [ ] T025 [US2] Implement the `RequiredLegend` builder in `client/ui/material/required_legend.rs` and export it from `client/ui/material/mod.rs` (Principle VIII) (depends on T023)
- [ ] T026 [US2] Push `RequiredLegend` as the first child of the fields column, only when `RequiredMarks::legend(required_fields())` is true, in `client/ui/worktree_form.rs`, `client/ui/parallel_dialog.rs` and `client/ui/rename_dialog.rs`; at the top of the section content in `client/ui/settings/{terminal,environment,github,jira}.rs`; once at the top of the daemon form (its `container_fields.rs` part adds none) and of the repository step in `client/ui/settings/daemons.rs` (FR-004: one per form) (depends on T025, T012, T021)
- [ ] T027 [US2] Re-record the layout snapshots the legend row changes (`ctests/layout_snapshot.rs`, `ctests/gates/`), checking that the only change is the one caption row plus `spacing::SM` per form (R8)
- [ ] T028 [P] [US2] Add the legend to the showcase *Required field* entry in `client/showcase/sections/controls.rs`; document `RequiredLegend` (C3) in `docs/development/component-library.md`; mention the `* required` line in the *Required fields* notes of `docs/user-guide/{worktrees-and-sessions,parallel-runs,project-selection,settings}.md`

**Checkpoint (M3)**: SC-003 holds.

---

## Phase 5: User Story 3 — An empty required field says what is wrong (P2)

**Goal**: an empty required field shows the error state with "This field is required" on a submit
attempt and on blur, never on a fresh form; Create and Start runs are pressable.

**Independent Test**: in each form leave a required field empty, press its submit button, then (M6)
focus and leave a field; the field shows the error with the message, no separate line elsewhere
(quickstart §B 2–5, errors).

### Tests for User Story 3, submit attempt in dialogs (write first, see them fail)

- [ ] T029 [P] [US3] Write failing anatomy tests in `client/ui/material/form_field_anatomy.rs`: `.required(Missing)` draws the field invalid (`FilledFieldState::error`, label and indicator in the error role) with `required::MESSAGE` in the supporting slot replacing the supporting text; `.required(Missing).error(Some(e))` shows `e`, not the message (FR-010); `Required` draws no error (C1, FR-005); going from `Missing` to `Required` changes only the error state and the supporting text, so the field's widget state (focus, cursor, value) is untouched (FR-007)
- [ ] T030 [P] [US3] Write failing tests: in `ctests/features_worktree_form.rs`, `submitted` on a form with empty Type and Name sets `required.attempted`, both read `Missing`, nothing is created, and the form's own error line no longer reports `NoType` or a blank name; then typing a name makes Name read `Required` with the typed text and cursor unchanged (FR-007); `can_press()` is true while editing and not prompting with an empty name (FR-012, R7); `submitted` with a non-blank name that slugs to nothing (e.g. `!!!`) creates nothing and shows today's `EmptyNameAfterSlug` message as the Name field's own error, not the required message (FR-010, edge case *Submit failure that is not about emptiness*). In `ctests/features_runs.rs`, `confirmed` with all four fields empty puts all four in `Missing` and starts no run; `revalidate` keeps only `RunCount` and `ProviderNotOffered` as form lines; `BadName` is the Name field's error. In `ctests/features_project.rs` and `ctests/features_worktree.rs`, `rename_confirmed` with an empty name calls `leave` on the name field (it reads `Missing`) and renames nothing, with no separate error line

### Implementation for User Story 3, submit attempt in dialogs

- [ ] T031 [US3] Draw `Missing` in `client/ui/material/form_field.rs`: invalid state and `MESSAGE` in the supporting slot unless the field's own `.error(Some(..))` is set (depends on T004, T029)
- [ ] T032 [US3] In `client/features/worktree_form.rs` call `required.attempt()` in `submitted`, rename `can_submit()` to `can_press()` without its `preview().is_ok()` clause, and stop showing `NoType`/blank name as a form line, while a non-emptiness `preview()` refusal (a name that slugs to nothing) still blocks the submit and shows as the Name field's own error; use `can_press()` in `default_actions` in `client/ui/worktree_form.rs` (depends on T030, T031)
- [ ] T033 [P] [US3] In `client/features/runs.rs` call `required.attempt()` in `confirmed`, make `revalidate` keep only `RunCount` and `ProviderNotOffered`, and put `BadName` on the Name field; make Start runs always pressable in `client/ui/parallel_dialog.rs` `actions` (depends on T030, T031)
- [ ] T034 [P] [US3] In `client/features/project.rs` and `client/features/worktree.rs` call `required.leave(<name field>)` in `rename_confirmed`; remove the free-standing error `Text` under the field in `client/ui/rename_dialog.rs` (R4) (depends on T030, T031)
- [ ] T035 [P] [US3] Add `Missing` `TextField` and `Select` states to the showcase *Required field* entry in `client/showcase/sections/controls.rs` (C6, FR-011); extend `ctests/required_call_sites.rs` to fail on the `"This field is required"` literal outside `material/` (C5); add to the user-guide notes in `docs/user-guide/{worktrees-and-sessions,parallel-runs,project-selection}.md` that pressing Create, Start runs or Rename with a required field empty marks it "This field is required" and submits nothing

**Checkpoint (M4)**: US3 scenario 1 and 2 in the dialogs; Create and Start runs enabled (FR-012).

### Tests for User Story 3, submit attempt in settings and daemons (write first, see them fail)

- [ ] T036 [P] [US3] Write failing tests in `ctests/features_settings.rs`: `saved` with a required field empty calls `required.attempt()`, the field reads `Missing`, nothing is saved, and `SettingsDraft::validate` no longer returns a `FieldError` for an empty value (e.g. "Enter a whole number…" on empty Scrollback, `MappingErrorKind::Blank`); a non-empty invalid value keeps its `FieldError` (FR-010)
- [ ] T037 [P] [US3] Write failing tests in `ctests/features_daemons.rs`: the `Saved` arm with an empty name sets `required.attempted` and `DaemonName` reads `Missing` with no "A daemon needs a name." error; an empty SSH Host reads `Missing`, and switching the runtime back to local makes it `Optional` (edge case *stops being required*); a Port that is not a number keeps its own error (US3 scenario 4); `add_repository` with an empty path calls `repository_marks.leave(DaemonRepository(id))` for that daemon only

### Implementation for User Story 3, submit attempt in settings and daemons

- [ ] T038 [P] [US3] In `client/features/settings.rs` call `required.attempt()` in `saved` and drop the empty-value `FieldError`s (depends on T036, T031)
- [ ] T039 [P] [US3] In `client/features/daemons.rs` call `required.attempt()` in the `Saved` arm, drop the empty-value errors, and `leave(DaemonRepository(id))` in `add_repository` (depends on T037, T031)
- [ ] T040 [US3] In `client/ui/settings/{terminal,environment,github,jira,daemons,container_fields}.rs` stop passing emptiness errors to `.error(..)` where T038/T039 removed them, so the field's `Missing` state shows the message (depends on T038, T039)
- [ ] T041 [P] [US3] Add to the *Required fields* note in `docs/user-guide/settings.md` that Save with a required field empty marks it "This field is required" and saves nothing

**Checkpoint (M5)**: SC-004 holds on submit attempt.

### Tests for User Story 3, error on blur (write first, see them fail)

- [ ] T042 [P] [US3] Write failing tests for `Select::on_blur` (C2, R2) in `client/ui/material/select_anatomy.rs`: published once when the select goes from engaged (`focused || open`) to not engaged without a pick in between; not published on a pick; not published by a select that was never engaged
- [ ] T043 [P] [US3] Write failing tests: in `ctests/features_window.rs`, `field_focus_changed(id, false)` returns `Outcome::FieldBlurred(id)`, also when its guard leaves `focused_field` unchanged (R3); in a new `ctests/features_required.rs`, `features::required::blurred(state, FieldId::AddWorktreeName)` calls `leave` on the open add-worktree form only, a blur of a field whose draft is not open changes nothing, a blur in one window's form leaves another window's form unmarked (Principle II), and a fresh form shows no `Missing` (US3 scenario 3); in `ctests/features_worktree_form.rs`, `FormMsg::BranchDismissed` calls `leave(AddWorktreeBranch)`; extend `ctests/required_call_sites.rs` to fail when a required `Select` in an inventory view (Type, Base branch) is built without `track_focus` (C4, covers T045 and T047)

### Implementation for User Story 3, error on blur

- [ ] T044 [US3] Implement `Select::on_blur(M)` in `client/ui/material/select.rs` with an `engaged` bit in `SelectState`, published from `Widget::update` (depends on T042)
- [ ] T045 [US3] Implement `TrackFocus for Select<'_, T, Message>` in `client/ui/focus.rs` (C4) (depends on T044)
- [ ] T046 [US3] Add `Outcome::FieldBlurred(FieldId)` (`client/features/mod.rs`, returned by `field_focus_changed` in `client/features/window.rs`), the new `client/features/required.rs` with `blurred(state, id)` matching `id` to its owning draft, its application in `client/app.rs`, and `leave(AddWorktreeBranch)` on `BranchDismissed` in `client/features/worktree_form.rs` (depends on T043)
- [ ] T047 [US3] Give the required `Select`s their `FieldId` through `TrackFocus` in `client/ui/worktree_form.rs` (Type), `client/ui/parallel_dialog.rs` (Type, Base branch) and the settings and daemon views that list a select in `required_fields()`, so every required field reports its blur (depends on T045, T046)
- [ ] T048 [P] [US3] Document `Select::on_blur` (C2) and `TrackFocus for Select` (C4) in `docs/development/component-library.md`; add to the user-guide notes in `docs/user-guide/{worktrees-and-sessions,parallel-runs,project-selection,settings}.md` that leaving a required field empty also marks it

**Checkpoint (M6)**: FR-006 complete; all stories done.

---

## Phase 6: Polish & cross-cutting

Changes no code: left to the close unit (milestones rule 4).

- [ ] T049 Run quickstart Part B with the `visual-pass` skill (showcase, add worktree, run in parallel, rename, settings, two windows; light, dark, greyscale) and record it in `specs/937-required-field-markers/visual-pass.md`
- [ ] T050 Run quickstart Part A (`scripts/gate.sh`) on the final tree and tick the spec's checklist items the pass confirms

---

## Dependencies & Execution Order

- **Foundational (T001–T004)** blocks everything.
- **US1 dialogs (T005–T015)** after Foundational; **US1 settings (T016–T022)** after T002, T004.
- **US2 (T023–T028)** after both US1 parts (T026 wires the views those parts wired).
- **US3 dialogs (T029–T035)** after US1 dialogs; **US3 settings (T036–T041)** after T031 and US1
  settings; **US3 blur (T042–T048)** after both US3 submit parts.
- **Polish (T049–T050)** last.
- Within a part: tests first and failing; then core/feature modules; then views; docs alongside.

## Parallel Examples

```text
# Foundational
T001 (core tests)  ‖  T003 (anatomy tests)
# US1 dialogs
T005 ‖ T006 ‖ T007, then T009 ‖ T010 ‖ T011, with T013 ‖ T014 ‖ T015 at any point
# US3 blur
T042 ‖ T043, then T044 → T045 and T046, then T047
```

## Implementation Strategy

MVP is M1: core, marker and the dialogs. Each later milestone adds one observable increment: the
settings forms marked (M2), the legend (M3), the required error on submit in dialogs (M4) and in
settings (M5), then on blur (M6). Between M4 and M6 the error appears on a submit attempt only,
which is a subset of FR-006's final behaviour, not a contradiction of it.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

US1 and US3 are split (milestones rule 3): each has more than 10 tasks and touches six drafts and
nine views. US1 splits along its scenarios (1, 4: dialogs; 2, 3: settings and daemons); US3 along
its scenarios (1, 2, 4 on submit, dialogs then settings; 3 on blur).

### M1 — Required fields marked in the dialogs 🎯 MVP

- **Tasks**: T001–T015
- **Deliverable**: the add-worktree form, the run-in-parallel dialog and both rename dialogs show `*` after the label of every field they refuse empty, and on no other field.
- **Satisfies**: US1 acceptance scenarios 1, 4; FR-001, FR-002, FR-003, FR-008 (marker), FR-009 (dialogs), FR-011 (resting and filled)
- **Verify**: `scripts/gate.sh` (`required::tests`, `features_worktree_form`, `features_runs`, `features_project`, `features_worktree`, the anatomy tests, `required_call_sites`, `showcase_completeness`); quickstart §B 1–4, markers only
- **Depends on**: —
- **Tier**: full

### M2 — Required fields marked in Settings and the daemon form

- **Tasks**: T016–T022
- **Deliverable**: Settings › Terminal, Environment, GitHub, Jira and Daemons (form, container fields, repository step) mark their required fields, and the SSH Host's marker follows the SSH choice.
- **Satisfies**: US1 acceptance scenarios 2, 3; FR-002, FR-009 (settings); SC-001, SC-002
- **Verify**: `scripts/gate.sh` (`features_settings`, `features_daemons`); quickstart §B 5, markers only
- **Depends on**: M1
- **Tier**: full

### M3 — Marker legend

- **Tasks**: T023–T028
- **Deliverable**: every form with a required field shows `* required` once above its first field; Settings › Appearance shows none.
- **Satisfies**: US2 acceptance scenarios 1–2; FR-004, FR-008 (legend), FR-011 (legend); SC-003, SC-006 (legend)
- **Verify**: `scripts/gate.sh` (`required_legend` anatomy test, `required_call_sites`, `layout_snapshot`); quickstart §B 1–5, legend
- **Depends on**: M1, M2
- **Tier**: full

### M4 — Required error on submit in the dialogs

- **Tasks**: T029–T035
- **Deliverable**: pressing Create, Start runs (both now always enabled) or Rename with a required field empty shows "This field is required" on each such field and submits nothing; typing clears it.
- **Satisfies**: US3 acceptance scenarios 1, 2, 4 (dialogs); FR-005, FR-006 (submit), FR-007, FR-008 (error), FR-010, FR-011 (error), FR-012; SC-005, SC-006 (error)
- **Verify**: `scripts/gate.sh` (`form_field_anatomy`, `features_worktree_form`, `features_runs`, `features_project`, `features_worktree`, `required_call_sites`); quickstart §B 2–4, errors on submit
- **Depends on**: M1, M2, M3
- **Tier**: full

### M5 — Required error on save in Settings and the daemon form

- **Tasks**: T036–T041
- **Deliverable**: saving a settings section or a daemon with a required field empty shows "This field is required" on the field instead of the old emptiness error; a non-empty invalid value keeps its own error.
- **Satisfies**: US3 acceptance scenarios 1, 4 (settings); FR-005, FR-006 (submit), FR-010; SC-004
- **Verify**: `scripts/gate.sh` (`features_settings`, `features_daemons`); quickstart §B 5, errors on save
- **Depends on**: M2, M4
- **Tier**: full

### M6 — Required error on leaving a field

- **Tasks**: T042–T048
- **Deliverable**: focusing a required field and leaving it empty (text field, select or branch picker) shows its required error; untouched fields and fresh forms show none, and one window's form never marks another's.
- **Satisfies**: US3 acceptance scenario 3; FR-006 (blur); edge case *Several sessions and windows*
- **Verify**: `scripts/gate.sh` (`select_anatomy`, `features_window`, `features_required`, `features_worktree_form`); quickstart §B 2, 6
- **Depends on**: M4, M5
- **Tier**: full

Polish (T049–T050) changes no code and is left to the close unit.
