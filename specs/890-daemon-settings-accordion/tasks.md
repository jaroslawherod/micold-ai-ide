---

description: "Task list for feature 890: daemon settings in titled sections, every daemon kind, one look"
---

# Tasks: Daemon settings in titled sections, every daemon kind, one look

**Input**: Design documents from `specs/890-daemon-settings-accordion/`

**Prerequisites**: [plan.md](plan.md), [spec.md](spec.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/ui.md](contracts/ui.md), [quickstart.md](quickstart.md)

**Tests**: Constitution Principle I (Test-First) — every test task is written first and seen to fail
for the right reason before the implementation task it covers.

**Documentation**: Principle VII — M2 and M3 change user-facing forms, so each carries its
user-guide task (FR-017). M1 touches only `ui/material/**` and the showcase, which
`.gitattributes` marks not user-facing (research R10).

**Cross-platform**: Principle VI — no platform branch is added; the same code runs on Linux, macOS
and Windows. The kinds a platform offers are already decided by `runtime_choices`.

**Order**: phases follow FR-016's delivery order (research R10), not story priority: the shared
component first (US3's component part), then the sections (US1, P1), then the unified look and the
form summaries (US2, plus US3's form part). M2 uses M1's component without summaries, which FR-013
keeps looking as today, so each phase ships on its own.

**Paths**: `C` = `crates/micold-client`. Run one test with
`cargo test -p micold-client --test <file> <name>`; unit tests in `src/` with
`cargo test -p micold-client --lib <module>`.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on an unfinished task)
- **[Story]**: the user story the task serves (US1, US2, US3)

## Phase 1: Setup and Foundational

No setup or foundational work: the workspace, `AccordionSection`, `Select`, `Ellipsized`,
`RequiredMarks` and the geometry support (`tests/support/layout`) all exist. Every phase below
starts from `main`.

---

## Phase 2: User Story 3 (component part) — the shared section shows a summary and a required marker (M1)

**Goal**: `AccordionSection` takes an optional one-line summary and a required marker, both drawn
only while closed; a section without them looks exactly as today (FR-011 to FR-013, FR-015).

**Independent Test**: the component's unit tests and the showcase's `AccordionSection` entry
(quickstart Part A rows M1, Part B step 1).

### Tests for the component (write first, see them fail)

- [X] T001 [US3] Add failing unit tests to the `#[cfg(test)]` module of `C/src/ui/material/accordion_section.rs`, using the existing `support` helpers (`Mounted`, `size_of`, `bounds_at`): (a) closed with `.summary("Docker · Basic image")` paints the summary under the title in `TypeRole::Caption` and `roles.on_surface_variant`; (b) open with a summary paints no summary; (c) at a header width narrower than the summary the painted summary ends in `…` and the chevron stays inside the header bounds; (d) closed with `.required(true)` paints the title followed by `micold_core::required::MARKER` in the title's role and tint, open paints the title alone; (e) without `.summary`/`.required` the header height equals today's (pin the measured height before the change); (f) with both set, Space and Enter on the focused header and a pointer press still emit the toggle message once, and a closed section still offers no focus stop inside its body; (g) (a) and (d) hold in the light and the dark theme (FR-011–FR-013, SC-005).
- [X] T002 [P] [US3] Add a failing assertion to `C/tests/showcase_completeness.rs` that the `AccordionSection` catalogue entry poses `closed, with a summary`, `open`, `closed, a required field empty` and the live toggle (FR-015; contract §Shared component).

### Implementation

- [X] T003 [US3] In `C/src/ui/material/accordion_section.rs` add the builder inputs `summary: Option<String>` (default `None`) and `required: bool` (default `false`) with `.summary(impl Into<String>)`, `.summary_maybe(Option<String>)` (mirrors `Button::on_press_maybe`) and `.required(bool)`. While closed: the header becomes `row![column![title, summary], Space::Fill, chevron]` with the summary drawn by `Ellipsized::at_role(text, TypeRole::Caption, roles.on_surface_variant)` inside the same header `Button`; the title text becomes `format!("{title}{}", micold_core::required::MARKER)` when `required`, in the title's own role and tint (as `form_field.rs` marks a field label). While open, or with neither input: the header is built exactly as today. Doc comments state FR-011–FR-013 (research R4, R5). Makes T001 pass.
- [X] T004 [US3] In `C/src/showcase/sections/surfaces.rs::accordion_section` add the poses `closed, with a summary` (summary `Docker · Basic image`), `open`, and `closed, a required field empty` (`.required(true)`), keeping the live toggle; update the entry's captions in `C/src/showcase/catalogue.rs` so T002 passes (FR-015).
- [X] T005 [US3] Visual pass of quickstart Part B step 1 (showcase, both themes, narrow window) with the `visual-pass` skill; record it under `specs/890-daemon-settings-accordion/visual-pass/m1/`.

**Checkpoint**: M1 deliverable observable in the component showcase.

---

## Phase 3: User Story 1 — every daemon kind's form is split into well-named sections (Priority: P1) (M2)

**Goal**: under the form's title, *Name* and *Runs in* stay outside any section; every other setting
of the chosen kind sits in exactly one of FR-002's sections, in order; Connection opens first; an
error opens its section; toggling changes no value (FR-001–FR-005, FR-010, FR-017).

**Independent Test**: quickstart Part A rows M2 and Part B steps 2–3.

### Tests for User Story 1 (write first, see them fail)

- [x] T006 [P] [US1] Create `C/tests/features_daemon_sections.rs` with failing render-free tests: `Section::of` returns, in order, Container `[Connection, Container, Credentials, Network, Resources, Sessions]`, Kubernetes `[Connection, Image, Credentials, Resources, Placement]`, SSH `[Connection]`, Host `[Connection, Sessions]` (FR-002); `Section::title` gives `Connection`, `Container`, `Image`, `Credentials`, `Network`, `Resources`, `Placement and storage`, `Sessions` (FR-003, no `Limits`); `Section::holding_error(runtime, field)` and `Section::holding(runtime, id)` return the section of every row of data-model.md §Field-to-section map — every `FormError::field` name the form reads with `error_of` (`container_name`, `port`, `image`, `image_path`, `cpus`, `memory`, `pids`, `storage`, `context`, `namespace`, `ssh_key_file`, `pull_file`, `cpu_request`, `cpu_limit`, `memory_request`, `memory_limit`, `node_selector`, `tolerations`, `volume_size`, `storage_class`, `host`, `user`, `key_file`) and every `FieldId` the table lists — and `None` for `name`, `runtime` and `FieldId::DaemonName`; a section a kind lacks is never returned for that kind.
- [x] T007 [P] [US1] Add failing reducer tests to `C/tests/daemons_feature.rs`: for each `DraftRuntime`, after `begin_add` and after `begin_edit` of a stored daemon, `section_open` is true for `Connection` only (FR-004, US1 AS2); `Msg::RuntimeChosen` to another kind after toggling resets to that kind's default (spec Edge Cases); for each kind a refused save on a field in a closed non-first section opens that section (Container `cpus` → Resources, Kubernetes `cpu_limit` → Resources and `node_selector` → Placement, SSH `host` after closing Connection → Connection) and it falls back to the user's choice once the error is gone (US1 AS4); a required field left empty (`FieldRequirement::Missing`: Container `DaemonContainerName`, Kubernetes `DaemonKubeContext`, SSH `DaemonSshHost`) keeps Connection open after the user closed it (FR-004, clarification D3); `SectionToggled` leaves the draft and the registry equal to before (FR-005, US1 AS5). Replace the existing tests that assume `Section::Container` opens first and that name `Section::Limits`.
- [x] T008 [P] [US1] Update `C/tests/daemons_section_geometry.rs` with failing geometry tests (painted text via `tests/support/layout`): per kind the add form paints *Name* and *Runs in* above, the kind's section titles in FR-002 order, only `Connection`'s body, and *Save daemon*/*Cancel* below (FR-001, SC-001); no group heading is painted (FR-010): `Kubernetes` and `Allowed credentials` are not painted at all with every section closed, and `Placement and storage`, `Resources` and `Sessions` are each painted exactly once per kind, inside a section header's bounds; with Connection closed, Tab from *Runs in* reaches the next section header without stopping on a field of the closed Connection (SC-005); no text overflows. Rewrite `the_container_form_paints_five_titles_with_only_container_open` to the six container titles with Connection open, and the SSH and Kubernetes form tests to open the section holding the fields they assert.
- [x] T009 [P] [US1] Create the failing gate `C/tests/gates/daemon_form_fits.rs` and register it in `C/tests/layout_snapshot.rs` beside `container_section_fits` (`#[path = "gates/daemon_form_fits.rs"] mod daemon_form_fits;`): the Kubernetes add form with default section states, `ContextList` not yet loaded (no context buttons), no `FormError`, at the canonical Settings width, has the bottom of *Cancel* at or above 800 logical px (SC-002). Record the same form's height on `main` before T013 in the cycle log as the red evidence (today it overflows).

### Implementation for User Story 1

- [x] T010 [US1] Create `C/src/features/daemon_sections.rs` and register it in `C/src/features/mod.rs`: move `Section`, `Section::title` and `Section::holds` out of `C/src/features/daemons.rs`, rename `Limits` to `Resources`, add `Image` and `Placement`, drop `Section::ALL`, add `Section::of(DraftRuntime) -> &'static [Section]`, `Section::holding_error(DraftRuntime, &str) -> Option<Section>` and `Section::holding(DraftRuntime, FieldId) -> Option<Section>` answering from one table (data-model.md §Section, §Field-to-section map; research R1, R2); re-export as `features::daemons::Section`. Makes T006 pass.
- [x] T011 [US1] In `C/src/features/daemons.rs` change `State::section_open` to data-model.md §Section open state: default = first of `Section::of(draft.runtime)` XOR flipped, forced open by `holding_error(runtime, form_error.field)` and by any `(id, req)` of `draft.required_fields()` with `holding(runtime, id) == Some(section)` and `draft.required.state(id, req) == FieldRequirement::Missing`; `false` with no draft. Clear `flipped_sections` in `Msg::RuntimeChosen` (research R3). `SectionToggled` stays the only writer besides the clears (FR-005). Makes T007 pass.
- [x] T012 [US1] In `C/src/ui/settings/container_fields.rs` build the container's six sections from `Section::of(DraftRuntime::Container)` in contract §Section bodies' order: a new `Connection` body holding *Container name*, *Port* (placeholder `A free port is chosen if empty`) and the *Connect when the application starts* checkbox moved from `C/src/ui/settings/daemons.rs`; `Container`, `Credentials`, `Network`, `Resources` (was `Limits`) and `Sessions` bodies with their controls as today; each `AccordionSection` given `.open(state.section_open(s))` and `.on_toggle(SectionToggled(s))`.
- [x] T013 [US1] In `C/src/ui/settings/daemons.rs` make `form` build, per contract §Form layout: title and required legend; *Name* and *Runs in*; the `runtime` refusal caution under *Runs in*; the kind's sections from `Section::of(runtime)`; *Save daemon* and *Cancel*. Move the Kubernetes fields (`kube_form`) into `Connection` (context buttons as today, *Context*, the context-list failure caution under it, *Namespace*, connect at start), `Image` (*Image*, AI CLI checkboxes and their note), `Credentials` (allowed credentials with each file field under its checkbox), `Resources` (the four request/limit fields) and `Placement and storage` (node selector, tolerations, their error cautions under them, *Volume size*, *Storage class*); the SSH fields (`HostPicker`, *Host*, *User*, *Port*, *Key file*, connect at start) into `Connection`; the host's connect-at-start into `Connection` and its survival note into `Sessions`. Remove every `group(...)` heading from the daemon forms (FR-010). No field's validation or save path changes (FR-005). Makes T008 and T009 pass.
- [x] T014 [P] [US1] Update `docs/user-guide/sandboxed-daemon.md`, `docs/user-guide/settings.md`, `docs/user-guide/kubernetes-daemons.md` and `docs/user-guide/remote-daemons.md` to name each kind's sections as FR-002 does (Container: Connection, Container, Credentials, Network, Resources, Sessions; Kubernetes: Connection, Image, Credentials, Resources, Placement and storage; SSH: Connection; Host: Connection, Sessions), that Connection opens first and a section with an error opens itself, and replace every mention of the container's "Limits" section with "Resources" (FR-017).
- [x] T015 [US1] Visual pass of quickstart Part B steps 2–3 (both themes) with the `visual-pass` skill; record it under `specs/890-daemon-settings-accordion/visual-pass/m2/`.

**Checkpoint**: M2 deliverable — every kind's form shows FR-002's sections on `main`.

---

## Phase 4: User Story 2 — one look across all daemon forms (Priority: P2), and US3's form part (M3)

**Goal**: one field gap and one section gap; explanations as supporting text, a section's first line
or a group note; every checkbox group and multi-line editor under a shared `FieldGroup` label; the
Kubernetes contexts in a select above *Context*; each closed section showing its summary and the
required marker (FR-006–FR-009, FR-014, FR-017; US2 AS1–AS5, US3 AS1–AS3 in the forms).

**Independent Test**: quickstart Part A rows M3 and Part B steps 4–6.

### Tests for User Story 2 and the form summaries (write first, see them fail)

- [x] T016 [P] [US2] Create `C/src/ui/material/field_group.rs` with only its `#[cfg(test)]` module and a stub `FieldGroup`, registered in `C/src/ui/material/mod.rs`, with failing unit tests: label painted in `TypeRole::Label` muted, the note (when set) in `TypeRole::Caption` muted directly under it, then the children top to bottom `spacing::SM` apart (the same value as `ui/settings::FIELD_GAP`, which the material component must not import); label, note and children share one left edge; without a note the first child follows the label; both themes (data-model.md §FieldGroup, FR-008).
- [x] T017 [P] [US3] Add failing tests to `C/tests/features_daemon_sections.rs` for `daemon_sections::summary(&Draft, Section)`: one test per row of contract §Section summaries, each with its example and its fallback (`Any free port`, `No AI tools`, `Nothing shared`, `Runtime defaults`, `Default SSH image`, `Nothing allowed`, `No requests or limits`, `Default storage class`, `Manual start`/`Connects at launch`, the SSH `[<user>@]<host>[:<port>]` part present only with a host), parts joined with `" · "` (U+00B7), values trimmed, empty parts left out, `None` when every part is empty and the row has no fallback; and for `daemon_sections::marked(&Draft, Section)`: true exactly when a required field the section holds is empty (Container › Connection with no container name, Container › Container with `My own image` and neither reference nor file, Kubernetes › Connection with no context or namespace, SSH › Connection with no host), false otherwise (FR-014, US3 AS3). Add a test for `AllowedKind::short_label` giving `Git config`, `Git credentials`, `SSH key`, `<CLI display name> sign-in`, `Registry credentials`.
- [x] T018 [P] [US2] Add failing geometry tests to `C/tests/daemons_section_geometry.rs`: with every section open, for each of the four kinds every gap between consecutive controls in a section body equals one value and every gap between consecutive sections (and between *Runs in* and the first section) equals one value, the same values across kinds (FR-006, SC-003); each group label (`AI tools in this daemon`, `Shared with the container`, `AI CLIs installed in the pod`, `Allowed credentials`, `Node selector`, `Tolerations`) is painted above its first checkbox or editor and at the same left edge as the section's text fields (FR-008, US2 AS3); no explanation line is painted between two fields outside a group or a field's own supporting text (contract §Section bodies order, FR-007); with `ContextList::Found` holding contexts the `Context from your kubeconfig` select is painted above the *Context* field and no per-context button is painted, and with no contexts only the *Context* field shows (FR-009, US2 AS4); closed headers paint their summary and, for an empty required field, the title followed by `*` (FR-014).
- [x] T019 [P] [US2] Add a failing assertion to `C/tests/showcase_completeness.rs` that the catalogue lists `FieldGroup` (`material/field_group.rs`) with a pose with a note and one without.

### Implementation for User Story 2 and the form summaries

- [x] T020 [US2] In `C/src/ui/settings/mod.rs` add `pub(crate) const FIELD_GAP: f32 = spacing::SM;` and `pub(crate) const SECTION_GAP: f32 = spacing::SM;` with doc comments (FR-006; research R8; `FieldGroup` spaces its children with the same `spacing::SM` token itself, so `ui/material` never imports `ui/settings`), and a `section_body(note: Option<&str>, children)` builder that puts the optional section-wide note first (muted `Caption`) and spaces the controls `FIELD_GAP` apart.
- [x] T021 [US2] Implement `FieldGroup` in `C/src/ui/material/field_group.rs` — `FieldGroup::new(label, roles).note(text).push(child).extend(children).into()` (data-model.md §FieldGroup; research R7) — export it from `C/src/ui/material/mod.rs`, and add its showcase entry (with a note, without) in `C/src/showcase/sections/surfaces.rs` and `C/src/showcase/catalogue.rs`. Makes T016 and T019 pass.
- [x] T022 [P] [US2] Add `impl std::fmt::Display for ContextEntry` (writes `name`) in `C/src/kube/live.rs` (research R6).
- [x] T023 [P] [US3] Add `AllowedKind::short_label(self) -> String` in `C/src/features/daemons_kube.rs` (contract §Section summaries), then `summary` and `marked` in `C/src/features/daemon_sections.rs` per data-model.md §Section summary and marker and contract §Section summaries, with the `" · "` separator as one constant. Makes T017 pass.
- [x] T024 [US2] In `C/src/ui/settings/container_fields.rs` build every container section body with `section_body` and set the contract §Section bodies look: Container — the AI CLIs in `FieldGroup` `AI tools in this daemon` with note `AI_CLI_NOTE` (moved from after the checkboxes); Credentials — note `The container starts with none of your credentials. Share only what the agent needs.`, then `FieldGroup` `Shared with the container` with the unshared sign-in caution under `AI CLI sign-in` and the sharing-summary caution last; Network — `Names still resolve either way — the runtime's resolver answers from the host side. Blocked means connections fail, not that nothing leaves the container.` as the select's supporting text, the no-outbound caution under the select; Resources — note `What the sandbox may consume. A limit the runtime cannot enforce is shown, and says so, rather than being quietly dropped.` then the four fields; Sessions — checkbox then `survival_note` (FR-007, FR-008; research R9).
- [x] T025 [US2] In `C/src/ui/settings/daemons.rs` build the Kubernetes, SSH and host section bodies with `section_body` and space the form's sections `SECTION_GAP` apart (FR-006); replace the per-context buttons with `Select::new(contexts, selected, |c: ContextEntry| Msg::ContextChosen(c.name), roles)` labelled `Context from your kubeconfig`, placeholder `Choose a context`, shown only for `ContextList::Found` with at least one entry, above *Context* (FR-009); Image — *Image* with supporting `Empty uses the default SSH image`, then `FieldGroup` `AI CLIs installed in the pod` with note `Installed when the pod starts.`; Credentials — `FieldGroup` `Allowed credentials` with note `Nothing reaches the pod unless it is ticked here.`, each file field under its ticked checkbox; Placement and storage — `FieldGroup` `Node selector` (note `One key=value per line`) and `FieldGroup` `Tolerations` (note `One per line`), each error caution directly under its editor, then *Volume size*, *Storage class* (FR-007, FR-008; contract §Section bodies).
- [x] T026 [US3] In `C/src/ui/settings/container_fields.rs` and `C/src/ui/settings/daemons.rs` pass `.summary_maybe(daemon_sections::summary(draft, s))` and `.required(daemon_sections::marked(draft, s))` to every daemon form section (FR-014; contract §Section summaries, §Required marker). Makes T018 pass.
- [x] T027 [P] [US2] Update `docs/user-guide/sandboxed-daemon.md`, `docs/user-guide/settings.md`, `docs/user-guide/kubernetes-daemons.md` and `docs/user-guide/remote-daemons.md`: a closed section shows a one-line summary of its settings and `*` when a required field inside it is empty; Kubernetes contexts found in the kubeconfig are chosen from the `Context from your kubeconfig` select above *Context*, which can still be typed (FR-017).
- [x] T028 [US2] Visual pass of quickstart Part B steps 4–6 (every kind, both themes, narrowest Settings width) with the `visual-pass` skill; record it under `specs/890-daemon-settings-accordion/visual-pass/m3/` (SC-006).

**Checkpoint**: M3 deliverable — one look and section summaries in every daemon form on `main`.

---

## Phase 5: Polish & Cross-Cutting Concerns

Left to the close unit (no code change, so not a milestone):

- [ ] T029 Run quickstart.md Part A in full and confirm the SC-004 round-trip tests in `C/tests/daemons_feature.rs`, `C/tests/features_daemons.rs` and `C/tests/features_daemons_kube.rs` stayed green unchanged; record the result in `specs/890-daemon-settings-accordion/quickstart.md`.

---

## Dependencies & Execution Order

### Phase Dependencies

- **Phase 2 (M1)**: none.
- **Phase 3 (M2)**: after M1 merges (uses `AccordionSection` as M1 leaves it; no summary needed).
- **Phase 4 (M3)**: after M2 merges (adds look and summaries to M2's sections; `summary`/`marked`
  live in M2's `daemon_sections.rs`).
- **Phase 5**: after M3, in the close unit.

### Within each phase

- Tests first and failing: T001–T002 before T003–T004; T006–T009 before T010–T013; T016–T019
  before T020–T026.
- T010 before T011 (`section_open` uses `Section::of` and the lookups) and before T012–T013.
- T012 before T013 (`form` places the container sections T012 builds).
- T020 and T021 before T024–T025; T023 before T026; T022 before T025.
- The user-guide task and the visual pass close each milestone.

### Parallel opportunities

- M1: T002 beside T001.
- M2: T006, T007, T008, T009 (four files); T014 beside T010–T013.
- M3: T016, T017, T018, T019 (four files); T022 and T023 beside T020–T021; T027 beside T024–T026.

## Parallel Example: User Story 1

```bash
Task: "Create C/tests/features_daemon_sections.rs with failing Section::of / holding tests (T006)"
Task: "Add failing open-state reducer tests to C/tests/daemons_feature.rs (T007)"
Task: "Update C/tests/daemons_section_geometry.rs with failing per-kind section tests (T008)"
Task: "Create the failing gate C/tests/gates/daemon_form_fits.rs (T009)"
```

---

## Implementation Strategy

### Delivery order (FR-016)

1. **M1**: the component's summary and marker, posed in the showcase. Nothing in the app changes
   look: no caller passes either input yet.
2. **M2** (🎯 the P1 story): every kind's form in FR-002's sections, with the user guide.
3. **M3**: the unified look and the summaries in the forms, with the user guide.

Each milestone is one PR, merged to `main` before the next starts.

---

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot).

### M1 — Section summary and required marker in the shared component

- **Tasks**: T001–T005
- **Deliverable**: the component showcase's `AccordionSection` entry shows a closed section with a one-line muted summary (ellipsized when narrow), an open one without it, and a closed one with `*` after its title; sections without them look as today.
- **Satisfies**: US3 acceptance scenarios 1, 2, 4 (component); FR-011, FR-012, FR-013, FR-015; SC-005 (component)
- **Verify**: `cargo test -p micold-client --lib accordion_section` and `cargo test -p micold-client --test showcase_completeness`; quickstart Part B step 1
- **Depends on**: —
- **Tier**: full

### M2 — Every daemon kind's form split into named sections 🎯 MVP

- **Tasks**: T006–T015
- **Deliverable**: in Settings › Daemons, the add/edit form of every kind (Container, Kubernetes, SSH, Host) shows *Name* and *Runs in*, then FR-002's sections with Connection open, then Save/Cancel; an error or empty required field opens its section; the user guide names the sections.
- **Satisfies**: US1 acceptance scenarios 1–5; FR-001–FR-005, FR-010, FR-017 (section names); SC-001, SC-002, SC-004, SC-005
- **Verify**: `cargo test -p micold-client --test features_daemon_sections --test daemons_feature --test daemons_section_geometry --test layout_snapshot`; quickstart Part B steps 2–3
- **Depends on**: M1
- **Tier**: full

### M3 — One look and section summaries in every daemon form

- **Tasks**: T016–T028
- **Deliverable**: every daemon form has one field gap and one section gap, checkbox groups and editors under `FieldGroup` labels, explanations under their field or as a section's first line, Kubernetes contexts in a select above *Context*, and each closed section showing its summary and `*` when a required field inside is empty; `FieldGroup` is in the showcase; the user guide describes summaries and the context select.
- **Satisfies**: US2 acceptance scenarios 1–5; US3 acceptance scenario 3 and scenario 1 in the forms; FR-006–FR-009, FR-014, FR-017; SC-003, SC-006
- **Verify**: `cargo test -p micold-client --test features_daemon_sections --test daemons_section_geometry --test showcase_completeness` and `cargo test -p micold-client --lib field_group`; quickstart Part B steps 4–6
- **Depends on**: M2
- **Tier**: full

## Notes

- M2 (10 tasks) and M3 (13 tasks) are kept whole rather than split by milestones.md rule 3: the
  user asked for FR-016's three PRs, and splitting M3 along US2 vs the summaries would leave a
  milestone with no deliverable a user can tell apart (ledger).
- T029 is in no milestone: a Polish task that changes no code goes to the close unit and its PR
  (milestones.md rule 4).
- M1 is the shared component, not the P1 story: FR-016 orders the three PRs so, and M1 still has
  an observable deliverable (the showcase entry); ledger D6.
- Stored formats are untouched (SC-004): no task edits `micold-core::daemons` or the registry.
