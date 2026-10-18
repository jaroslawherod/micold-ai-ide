# Quickstart: Daemon settings in titled sections

How to check feature 890 works. Expected layouts and texts: [contracts/ui.md](contracts/ui.md).

## Part A — automated

Run in the worktree (CI's order): `scripts/gate.sh` (or `mise run gate`). The feature's tests:

| Milestone | Test file | What it proves |
|---|---|---|
| M1 | `crates/micold-client/src/ui/material/accordion_section.rs` (unit tests) | summary shown closed only, ellipsized in a narrow width, absent while open; marker closed only; a section without either has today's header height; toggling and focus stops unchanged with both set; both themes |
| M1 | `crates/micold-client/tests/showcase_completeness.rs` | the showcase poses the section closed with a summary, open, closed with the marker |
| M2 | `crates/micold-client/tests/daemon_sections.rs` (new) | `Section::of` per kind (FR-002); every `FormError::field` and required `FieldId` maps to its section; titles (FR-003) |
| M2 | `crates/micold-client/tests/daemons_feature.rs` | first section open on add, edit and kind change; toggle touches neither draft nor registry; a refused save and a left-empty required field each open their section, in every kind (FR-004, FR-005) |
| M2 | `crates/micold-client/tests/daemons_section_geometry.rs` | each kind paints its section titles in order with only `Connection`'s body painted; no `group` heading painted; no overflow |
| M2 | `crates/micold-client/tests/gates/daemon_form_fits.rs` (new) | the Kubernetes form with default states fits 800 logical px tall at the canonical width (SC-002) |
| M3 | `crates/micold-client/tests/daemon_sections.rs` | every summary row and fallback of the contract, the marker per section (FR-014) |
| M3 | `crates/micold-client/tests/daemons_section_geometry.rs` | one field gap and one section gap across the four kinds with every section open (SC-003); group labels and checkboxes share the text fields' left edge (FR-008); the context select sits above the *Context* field (FR-009) |
| M3 | `crates/micold-client/src/ui/material/field_group.rs` (unit tests), `tests/showcase_completeness.rs` | `FieldGroup` label/note/children order and left edge, both themes; listed in the showcase |

Stored data (SC-004): the existing save/edit round-trip tests in `tests/daemons_feature.rs`,
`tests/features_daemons.rs` and `tests/features_daemons_kube.rs` stay green unchanged.

## Part B — visual pass (needs eyes at a display; `visual-pass` skill)

Prerequisites: a build of the client and of the component showcase.

1. **Showcase (M1)**: open the showcase, find `AccordionSection`. Expect: closed with a summary
   (muted one-line text under the title), open (no summary), closed with `*` after the title; the
   live entry toggles by click and by Space/Enter with focus. Narrow the window: the summary ends in
   `…`, the chevron stays. Repeat in dark theme.
2. **Sections per kind (M2)**: Settings › Daemons › Add daemon. Set *Runs in* to Container, Host,
   Remote over SSH, Kubernetes in turn. Expect the contract's section list for each, `Connection`
   open, the rest closed; Name and Runs in above, Save/Cancel below. Tab through: focus visits each
   header and only the open section's fields.
3. **Errors open sections (M2)**: Kubernetes, open Connection, clear Context, Tab away, close
   Connection. Expect it reopened with the required error. Type `x` in Resources › CPU limit, close
   Resources, Save. Expect Resources open with the refusal.
4. **One look (M3)**: for each kind open every section. Expect: equal spacing between fields and
   between sections; every checkbox group and editor with a label above it; explanations under
   their field or as a section's first line; no stray line between two fields. Kubernetes with
   contexts found: a select above *Context*, no column of buttons.
5. **Summaries and marker (M3)**: close every section. Expect each header's summary as in the
   contract's examples; type in a field and see the summary of its section change once closed.
   Add form, Kubernetes, nothing typed: `Connection *`.
6. Repeat 4–5 in dark theme and at the narrowest Settings width: nothing overflows, summaries end
   in `…`.

Record the pass under `specs/890-daemon-settings-accordion/visual-pass/` with the screenshots.
