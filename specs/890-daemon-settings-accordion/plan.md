# Implementation Plan: Daemon settings in titled sections, every daemon kind, one look

**Branch**: `claude/project-thread-rwlog8` | **Date**: 2026-10-18 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/890-daemon-settings-accordion/spec.md`

## Summary

Feature 804 M4 shipped the shared `AccordionSection` and put the local container's settings in five
sections. This feature (a) gives the component an optional one-line summary and a required marker
for its closed state, (b) splits every daemon kind's form into FR-002's sections with one shared
`Section` model whose rules (layout per kind, which field each section holds, default and forced
open state, summaries, marker) live in a new render-free module, and (c) unifies the forms' look:
one field gap and one section gap, a new shared `FieldGroup` for checkbox groups and multi-line
editors, explanations as supporting text or a section's first line, and the Kubernetes contexts in a
select above the *Context* field. No stored format changes. Three milestones, one PR each (FR-016).

## Technical Context

**Language/Version**: Rust (workspace edition), iced 0.14 (existing).

**Primary Dependencies**: none new. `micold-client` (`features::daemons`, `features::daemons_kube`,
`ui::settings::{daemons, container_fields}`, `ui::material`), `micold-core` (`required`, `tokens`).

**Storage**: none touched. `settings.json` daemon registry unchanged (SC-004); open sections are
view state, never persisted.

**Testing**: client integration tests in `crates/micold-client/tests/` (render-free reducer and
section rules; geometry via `tests/support/layout`), component unit tests in
`ui/material/*.rs`, the `layout_snapshot` gates in `tests/gates/`, showcase completeness, and the
quickstart Part B visual pass. Gate: `scripts/gate.sh` / `mise run gate`.

**Target Platform**: Linux, macOS, Windows (Principle VI): no platform branch added.

**Project Type**: desktop app (Rust workspace, `micold-core` + `micold-client`).

**Performance Goals**: none beyond today's; summaries are a few string joins per view.

**Constraints**: field values, validation and save unchanged (FR-005); keyboard and focus behaviour
of the section header unchanged (FR-013); Kubernetes form fits 800 logical px with default states
(SC-002).

**Scale/Scope**: ~10 files; one new render-free module (~250 lines with docs), one new material
component (~120 lines), view reshuffle of `ui/settings/daemons.rs` and `container_fields.rs`.

## Constitution Check

*Re-checked after Phase 1 design: unchanged.*

| Principle | Status | Note |
|---|---|---|
| I. Test-First | PASS | Section layout, field map, open state, summaries and marker are decision logic in render-free `features/daemon_sections.rs` and `features/daemons.rs`, test-first from `tests/`; component changes test-first in their unit tests; view glue covered by geometry tests and quickstart §B. See Test strategy. |
| II. Multi-Session | PASS | No session state touched; opening, closing or editing sections changes nothing until Save, as today (spec Edge Cases). |
| III. Worktree Integration | PASS | No worktree or VCS behaviour touched. |
| IV. Local-First Storage | PASS | Nothing stored or sent; no remote service. Kubernetes context listing is the existing local kubeconfig read. |
| V. Rust + iced | PASS | iced only; one `Section` enum makes "same concept, same name" a type fact (R1). |
| VI. Cross-Platform Parity | PASS | One code path for every platform; kinds not offered on a platform are already excluded by `runtime_choices`. |
| VII. Documentation | PASS | M2 and M3 update `docs/user-guide/sandboxed-daemon.md`, `settings.md`, `kubernetes-daemons.md`, `remote-daemons.md` (FR-017). M1 touches only `ui/material/**`, which `.gitattributes` marks not user-facing. |
| VIII. Reusable UI Components | PASS | `AccordionSection` extended with builder methods (`.summary`, `.summary_maybe`, `.required`); new `FieldGroup` is a builder-into-`Element` material component listed in the showcase; context chooser reuses `Select` (no new component, FR-009). |

## Design

Decisions and rejected alternatives: [research.md](research.md). Types and the field-to-section map:
[data-model.md](data-model.md). Exact layout, texts, spacing and summaries:
[contracts/ui.md](contracts/ui.md).

### Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-002, FR-010 | M2: `ui/settings/daemons.rs` `form` builds identity fields, then `Section::of(runtime)` sections, then actions; `kube_form`, SSH and host fields move into section bodies; `group(...)` headings removed. Contract §Form layout, §Section bodies |
| FR-003 | M2: `Section` enum titles; `Limits` → `Resources` (data-model §Section) |
| FR-004 | M2: `State::section_open` (default = first of kind, forced by `holding_error` and `Missing` required fields); `RuntimeChosen` clears flips (data-model §Section open state, R3) |
| FR-005 | M2: `SectionToggled` touches only `flipped_sections`; save path untouched |
| FR-006 | M3: `FIELD_GAP`, `SECTION_GAP` in `ui/settings/mod.rs`, one `section_body` builder (R8) |
| FR-007 | M3: explanation moves (R9; contract §Section bodies) |
| FR-008 | M3: `FieldGroup` (R7) for AI CLIs, shared/allowed credentials, node selector, tolerations |
| FR-009 | M3: `Select<ContextEntry>` above *Context*; `impl Display for ContextEntry` (R6) |
| FR-011, FR-012, FR-013 | M1: `AccordionSection::summary/summary_maybe/required` (R4, R5) |
| FR-014 | M3: `daemon_sections::summary`, `marked`; `AllowedKind::short_label` (R11; contract §Section summaries, §Required marker) |
| FR-015 | M1: `showcase/sections/surfaces.rs::accordion_section` poses, `showcase/catalogue.rs` |
| FR-016 | Milestones below |
| FR-017 | M2 (section names), M3 (summaries, context select) in the four user-guide pages |
| SC-001 | M2 geometry test: every control of each kind inside a section; no `group` heading |
| SC-002 | M2 gate `tests/gates/daemon_form_fits.rs` |
| SC-003 | M3 geometry test across the four kinds |
| SC-004 | existing round-trip tests stay green, no stored type touched |
| SC-005 | M1 component tests (focus stops, keyboard toggle with summary/marker) + quickstart §B.2 |
| SC-006 | quickstart Part B |

### Milestones (one PR each, FR-016)

| ID | Deliverable | Scope | Tier |
|---|---|---|---|
| M1 | `AccordionSection` summary and required marker; showcase poses | `ui/material/accordion_section.rs`, `showcase/sections/surfaces.rs`, `showcase/catalogue.rs` | component |
| M2 | Every kind split into FR-002 sections; open-state rules; user-guide section names | new `features/daemon_sections.rs`, `features/daemons.rs` (`Section` moved, `section_open`, `RuntimeChosen`), `ui/settings/daemons.rs`, `ui/settings/container_fields.rs`, tests, gate, 4 user-guide pages | feature |
| M3 | Unified look and summaries: `FieldGroup`, gaps, explanation moves, context select, summaries and markers wired in the forms; user guide | new `ui/material/field_group.rs`, `ui/material/mod.rs`, `features/daemon_sections.rs` (`summary`, `marked`), `features/daemons_kube.rs` (`short_label`), `kube/live.rs` (`Display`), `ui/settings/{mod,daemons,container_fields}.rs`, showcase, tests, user guide | feature |

### Test strategy

| Requirement | Layer | Test |
|---|---|---|
| FR-011–FR-013 | component unit (`accordion_section.rs` tests, `support` `Mounted`/`size_of`/`bounds_at`) | summary drawn closed only and ellipsized at a narrow width; marker closed only; header height unchanged without them; keyboard/pointer toggle and closed focus stops unchanged with both set; light and dark |
| FR-015 | client integration | `tests/showcase_completeness.rs` (the catalogue lists the three poses) |
| FR-002, FR-003 | client integration, render-free | `tests/daemon_sections.rs`: `Section::of` per kind, titles; field map complete for every `error_of` name and required `FieldId` |
| FR-004, FR-005 | client integration, render-free reducer | `tests/daemons_feature.rs`: default open per kind; `RuntimeChosen` resets; refused save and left-empty required field open their section in each kind; toggling leaves draft and registry equal |
| FR-001, FR-010, SC-001 | geometry (`tests/daemons_section_geometry.rs`, `support::layout` painted text) | per kind: section titles painted in order, only Connection's body painted, identity fields above and actions below, no group heading text painted |
| SC-002 | geometry gate (`tests/gates/daemon_form_fits.rs`) | Kubernetes add form, default states, canonical width: bottom of *Cancel* ≤ 800 |
| FR-006, SC-003 | geometry | every section open, each kind: all field gaps equal one value, all section gaps equal one value, same values across kinds |
| FR-007, FR-008 | geometry + component unit | group label above and left-aligned with its children and with the section's text fields; `FieldGroup` unit tests; no free-floating note between two fields (contract order) |
| FR-009 | geometry + reducer | select above the *Context* field when contexts are found, absent otherwise; choosing a context fills *Context* (existing `ContextChosen` reducer test) |
| FR-014 | client integration, render-free | `tests/daemon_sections.rs`: one test per contract row incl. fallbacks and `marked` |
| FR-017 | docs | `scripts/check-user-guide-updated.sh` in CI; reviewer reads the section names against the contract |
| SC-004 | client integration | existing save/edit tests unchanged and green |
| SC-005, SC-006, themes | quickstart Part B | visual pass, both themes, narrow width |

## Project Structure

### Documentation (this feature)

```text
specs/890-daemon-settings-accordion/
├── plan.md              # this file
├── research.md          # Phase 0
├── data-model.md        # Phase 1
├── quickstart.md        # Phase 1
├── contracts/ui.md      # Phase 1: layout, texts, spacing, summaries
├── autopilot.md         # run ledger
└── tasks.md             # Phase 2 (/speckit-tasks)
```

### Source Code (repository root)

```text
crates/micold-client/src/
├── features/
│   ├── daemon_sections.rs      # NEW (M2, M3): Section, Section::of, field map, summary, marked
│   ├── daemons.rs              # Section moved out and re-exported; section_open; RuntimeChosen
│   ├── daemons_kube.rs         # AllowedKind::short_label (M3)
│   └── mod.rs                  # registers daemon_sections
├── kube/live.rs                # impl Display for ContextEntry (M3)
├── ui/material/
│   ├── accordion_section.rs    # .summary, .summary_maybe, .required (M1)
│   ├── field_group.rs          # NEW FieldGroup (M3)
│   └── mod.rs                  # exports FieldGroup (M3)
├── ui/settings/
│   ├── mod.rs                  # FIELD_GAP, SECTION_GAP (M3)
│   ├── daemons.rs              # form per kind in sections (M2), look and summaries (M3)
│   └── container_fields.rs     # container section bodies per contract (M2, M3)
└── showcase/
    ├── catalogue.rs            # poses (M1), FieldGroup entry (M3)
    └── sections/surfaces.rs    # accordion_section poses (M1), field_group (M3)
crates/micold-client/tests/
├── daemon_sections.rs          # NEW
├── daemons_feature.rs
├── daemons_section_geometry.rs
├── showcase_completeness.rs
└── gates/daemon_form_fits.rs   # NEW, registered beside container_section_fits.rs
docs/user-guide/{sandboxed-daemon,settings,kubernetes-daemons,remote-daemons}.md
```

**Structure Decision**: existing two-crate layout. All new logic lives in `micold-client`'s
render-free `features/` (the form's draft types live there, not in `micold-core`); view changes stay
in `ui/settings/`; shared widgets in `ui/material/`.

## Complexity Tracking

No constitution violations.
