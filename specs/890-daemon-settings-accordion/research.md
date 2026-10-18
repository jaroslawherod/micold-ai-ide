# Research: Daemon settings in titled sections, every daemon kind, one look

Feature 890. Each decision is taken against the code as it stands on `origin/main` (paths are
workspace-relative). No `NEEDS CLARIFICATION` remained after the spec's clarify sessions; the
decisions below settle the design questions the spec leaves to planning.

## R1. One `Section` enum for every kind, listed per kind

- **Decision**: widen `features::daemons::Section` (today: Container, Credentials, Network, Limits,
  Sessions, container only) to the union of every kind's sections — `Connection`, `Container`,
  `Image`, `Credentials`, `Network`, `Resources`, `Placement`, `Sessions` — and add
  `Section::of(DraftRuntime) -> &'static [Section]` returning FR-002's list in order. `Limits` is
  renamed `Resources` (FR-003). The enum and its rules move to a new render-free module
  `crates/micold-client/src/features/daemon_sections.rs`, re-exported from `features::daemons` so
  existing `daemons::Section` imports keep compiling.
- **Rationale**: FR-003 says one concept has one name across kinds; one enum makes that a type fact
  (Principle V) rather than four string tables. `features/daemons.rs` is already 2,161 lines; the
  section rules (layout, which field each holds, open state inputs, summaries, marker) are one
  cohesive unit and testable on their own from `tests/` (Principle I: they carry decision logic, so
  they cannot live in `ui/`).
- **Alternatives rejected**: one enum per kind (`ContainerSection`, `KubeSection`, …) — four copies
  of "Credentials" and "Resources", the drift FR-003 forbids; string section ids — no exhaustiveness
  check when a kind gains a section.

## R2. Which field a section holds: by `FieldId` for required errors, by error name per kind

- **Decision**: two lookups in `daemon_sections.rs`:
  - `Section::holding_error(runtime, field: &str) -> Option<Section>` for `FormError::field`
    (names such as `port` and `image` exist in two kinds, so the runtime disambiguates);
  - `Section::holding(runtime, FieldId) -> Option<Section>` for required fields
    (`Draft::required_fields()` is keyed by `FieldId`).
  The mapping table is in [data-model.md](data-model.md#field-to-section-map). `name` and `runtime`
  map to no section (they sit above the sections, FR-001).
- **Rationale**: the two error sources already use these two keys (`FormError::field` is a
  `&'static str`; `RequiredMarks` is keyed by `FieldId`). Translating one into the other would need
  `field_name` (`features/daemons.rs:1147`) to grow into a full bijection, which it is not (`port`
  is two fields).
- **Alternatives rejected**: changing `FormError::field` to a `FieldId` — touches every refusal in
  `form_error`, `invalid_field` and the Kubernetes validators for no user-visible gain, outside this
  feature's scope.

## R3. Open state: flips from the kind's default, cleared when the kind changes

- **Decision**: keep 804's `State::flipped_sections: BTreeSet<Section>` (the sections the user
  flipped from the default). The default is "open iff the section is the first of
  `Section::of(draft.runtime)`" (Connection for every kind). `Msg::RuntimeChosen` clears the set
  (FR-004: a kind change shows the new kind's default). `State::section_open(section)` keeps its
  signature and adds two forcing reasons: the refused save's field (`holding_error`) and any field of
  the section whose `RequiredMarks` state is `FieldRequirement::Missing` (FR-004, clarification D3).
- **Rationale**: the flip set already gives "toggling changes no value" (FR-005) and "forgotten
  when the form closes" (`begin_add`, `begin_edit`, `discard_draft` clear it). A forced-open section
  is not written into the set, so it falls back to the user's choice once the error is gone, as 804
  does.
- **Alternatives rejected**: an explicit `open: BTreeSet<Section>` seeded per kind — needs seeding
  in three places and on every kind change, with the same result; persisting open state — out of
  scope (spec Assumptions).

## R4. The summary: under the title, one line, ellipsized

- **Decision**: `AccordionSection::summary(text)` (and `summary_maybe(Option<String>)`, mirroring
  `Button::on_press_maybe`) draws, while closed only, an `Ellipsized::at_role(text,
  TypeRole::Caption, roles.on_surface_variant)` line under the title, inside the header button. The
  header becomes `row![column![title, summary], Space::Fill, chevron]`.
- **Rationale**: `ui/material/ellipsized.rs` is the library's one-line truncating label (FR-011's
  "ellipsized"); putting the line inside the header button keeps the whole header one press target
  and one focus stop (FR-013). Under the title rather than beside it: at the narrowest Settings
  width (`tests/gates/container_section_fits.rs`'s `NARROW`) a beside layout leaves the summary a
  few characters.
- **Alternatives rejected**: `Text` with `Wrapping::None` — clips mid-glyph without an ellipsis;
  summary as a separate row outside the button — a second, inert line that is not part of the press
  target and reads as body content.

## R5. The required marker: the field marker appended to the title

- **Decision**: `AccordionSection::required(bool)`; while closed and `true`, the title is drawn as
  `format!("{title}{}", micold_core::required::MARKER)` in the title's own role and tint — exactly
  how `form_field.rs:308-313` marks a field label (FR-012: "same style as the form fields' required
  marker").
- **Rationale**: one marker glyph and one style for the whole form; the form's `RequiredLegend`
  already explains `*`.
- **Alternatives rejected**: an error-tinted dot or icon badge — a second notation for the same
  fact; showing the marker while open too — the open section shows the field's own marker.

## R6. The Kubernetes context chooser: a `Select` over the found contexts

- **Decision**: replace the column of per-context buttons (`ui/settings/daemons.rs:529-540`) with
  `Select::new(list, selected, |c: ContextEntry| ContextChosen(c.name), roles)` labelled
  "Context from your kubeconfig", placed above the *Context* text field and shown only for
  `ContextList::Found` with at least one entry (FR-009). `ContextEntry` (`crate::kube::live`) gains
  `impl Display` (its name) so the select can list it. The *Context* field and its
  `ContextChosen(v)` input stay.
- **Rationale**: same arrangement as `HostPicker` over the SSH *Host* field (FR-009); `Select` is
  the shared primitive (Principle VIII), so no new component is added (FR-009's last sentence).
- **Alternatives rejected**: generalising `HostPicker` into a string picker — renames a component
  four call sites and tests use, for one wording difference; a combined select-and-type field —
  excluded by FR-009.

## R7. Group labels: a new shared `FieldGroup` component

- **Decision**: add `ui/material/field_group.rs`: `FieldGroup::new(label, roles)
  .note(text)` (optional, the group-wide explanation, FR-008) `.push(child)` / `.extend(children)`
  `.into()`. Label in `TypeRole::Label` muted (the style `ui/settings::group` uses today), note in
  `TypeRole::Caption` muted directly under it, children in one column at the label's left edge with
  the field gap (R8). Used for: AI CLIs (container and Kubernetes), shared credentials, allowed
  credentials, node selector editor, tolerations editor. Listed in the component showcase
  (`showcase/catalogue.rs`, `tests/showcase_completeness.rs`).
- **Rationale**: FR-008 asks for one shared style across five groups in two forms; Principle VIII
  asks for a builder-style shared component rather than a settings-local helper.
- **Alternatives rejected**: keep `note("AI tools in this daemon")` lines — the caption style is
  the one FR-007 gives explanations, so a label and an explanation look alike (the inconsistency the
  user asked to fix); a free `field_group(label, note, children)` function — fails the
  component-reuse gate's builder rule.

## R8. One field gap, one section gap

- **Decision**: two constants in `ui/settings/mod.rs`: `FIELD_GAP = spacing::SM` (between
  consecutive controls in a section body, and between a `FieldGroup`'s children) and
  `SECTION_GAP = spacing::SM` (between consecutive sections, and between the identity fields and the
  first section). Every daemon section body is built by one function (`section_body`) that applies
  `FIELD_GAP`; the form column applies `SECTION_GAP`. Header-to-body stays the component's
  `spacing::XS`.
- **Rationale**: SC-003 asks for one value of each; both are today's `SM`, so the change is in
  *making* them single (Kubernetes and SSH push fields straight into the form column today) rather
  than in moving pixels. The section header button's own padding already separates sections
  visually.
- **Alternatives rejected**: `MD` between sections — measured against the 800 px budget of SC-002,
  four extra 8 px gaps buy nothing a header does not already show.

## R9. Explanations: supporting text first, section line second

- **Decision** (FR-007): a field's explanation is its `.supporting(...)`; a section-wide one is the
  first child of the section body (`note`); a group-wide one is the `FieldGroup` note. Concretely
  the moves are: the Network "Names still resolve…" note becomes the Network select's supporting
  text; `AI_CLI_NOTE` moves from after the checkboxes to the AI CLI group's note; "Contexts in your
  kubeconfig" becomes the select's label; "Node selector, one key=value per line" and "Tolerations,
  one per line" become group labels "Node selector" / "Tolerations" with notes "One key=value per
  line" / "One per line"; "Nothing reaches the pod unless it is ticked here." becomes the allowed
  credentials group note. Cautions stay under their control (unshared sign-in under its share,
  no-outbound warning under the network select, node-selector/tolerations errors under their editor,
  context-list failure under the *Context* field); the sharing summary caution stays last in the
  shared-credentials group (it concerns the group as a whole).
- **Alternatives rejected**: removing the explanatory lines — they carry FR texts from 804/806/686.

## R10. Milestones as FR-016 orders them

- **Decision**: M1 component (summary, marker, showcase) — library only, not a user-facing path in
  `.gitattributes` (`ui/material/** -micold-user-facing`), so no user-guide change; M2 every kind's
  sections + the `Limits` → `Resources` rename + the user guide's section names (FR-017); M3
  `FieldGroup`, gaps, explanation moves, the context select, summaries and markers in the forms, and
  the user guide's description of summaries and the context select.
- **Rationale**: each milestone is independently shippable: M2 uses the M1 component without a
  summary (FR-013: looks as today), M3 only adds to M2's sections.
- **Alternatives rejected**: summaries with M1 — M1 would have to touch the daemon form before M2
  reorganises it, then M2 would redo them.

## R11. Summary texts are computed in render-free code

- **Decision**: `daemon_sections::summary(&Draft, Section) -> Option<String>` and
  `daemon_sections::marked(&Draft, Section) -> bool`, tested from `tests/daemon_sections.rs`. The
  view passes them straight to the builder. Exact texts: [contracts/ui.md](contracts/ui.md#section-summaries).
- **Rationale**: Principle I's GUI exception covers glue only; the summary is a decision per value.
- **Alternatives rejected**: building summary strings in `ui/settings/daemons.rs` — untestable from
  `tests/`.
