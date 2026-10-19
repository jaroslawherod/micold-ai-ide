# Implementation Plan: Settings accordions form a group: opening one closes the others

**Branch**: `feat/1091_settings-accordions-should-form-a-group-opening` | **Date**: 2026-10-19 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/1091-settings-accordion-group/spec.md`

## Summary

`AccordionSection` stays stateless; the view state a form keeps about it moves into a new shared,
render-free `AccordionGroup<K>` (`ui/material/accordion_group.rs`). A group holds which one section
is open (or "none", or "not chosen yet: the default") and answers `is_open(key, default)` and
`toggle(key, default)`; sections that are not given to a group stay as today (FR-001, FR-003). The
daemon form replaces its `flipped_sections: BTreeSet<Section>` with one `AccordionGroup<Section>`
(FR-004), keeping the 890 rule that a section holding a shown error is open and ignores presses
(FR-006). The showcase gets a grouped and an independent set (FR-008). No stored format changes. One
milestone, one PR.

## Technical Context

**Language/Version**: Rust (workspace edition), iced 0.14 (existing).

**Primary Dependencies**: none new. `micold-client` only: `ui::material` (new `AccordionGroup`,
small `AccordionSection::in_group`), `features::daemons` (`State`, `Msg::SectionToggled`),
`showcase::{state, catalogue, sections::surfaces}`. `micold-core` untouched.

**Storage**: none. Open sections are view state, never persisted (spec Assumptions).

**Testing**: unit tests in `ui/material/accordion_group.rs` and `accordion_section.rs`; reducer
tests in `crates/micold-client/tests/daemons_feature.rs`; `daemons_section_geometry.rs` (existing,
must stay green: its toggles now go through the group); `showcase/state.rs` tests and
`tests/showcase_completeness.rs`; quickstart Part B visual pass. Gate: `mise run gate`.

**Target Platform**: Linux, macOS, Windows: no platform branch.

**Project Type**: desktop app (Rust workspace, `micold-core` + `micold-client`).

**Performance Goals**: none beyond today's; a group is one `Option` compare per section per view.

**Constraints**: field values, draft and save untouched (FR-005); header press, focus and keyboard
behaviour of the section unchanged (FR-009); ungrouped sets unchanged (FR-003, SC-003).

**Scale/Scope**: ~6 files; one new module (~80 lines with docs and tests aside), a few lines in
`daemons.rs` and the showcase.

## Constitution Check

*Re-checked after Phase 1 design: unchanged.*

| Principle | Status | Note |
|---|---|---|
| I. Test-First | PASS | The group's rules (exclusivity, close-on-reopen, default) are decision logic in render-free `AccordionGroup`, test-first in its unit tests; the error-held rule and form reset in `features/daemons.rs`, test-first from `tests/daemons_feature.rs`; showcase state test-first in `tests/showcase_state.rs`. View glue (`in_group`, the showcase render, `container_fields.rs`) is covered by geometry tests and quickstart §B. |
| II. Multi-Session | PASS | View state only, in the daemons `State`, cleared when the form closes; no session state touched (spec Edge Cases). |
| III. Worktree Integration | PASS | No worktree or VCS behaviour touched. |
| IV. Local-First Storage | PASS | Nothing stored or sent. |
| V. Rust + iced | PASS | iced only; `AccordionGroup<K>`'s three states are an enum, so "none chosen yet" and "all closed" cannot be confused (R2). |
| VI. Cross-Platform Parity | PASS | One code path on every platform. |
| VII. Documentation | PASS | `docs/user-guide/sandboxed-daemon.md`, `kubernetes-daemons.md` and `remote-daemons.md`, which describe the form sections, each gain a sentence that one section is open at a time; `ui/material/**` is marked not user-facing in `.gitattributes`. |
| VIII. Reusable UI Components | PASS | The behaviour is a shared component capability (`AccordionGroup` + `AccordionSection::in_group`, builder style, `.into()` end of chain unchanged), not daemon code; the showcase lists it. |

Complexity Tracking: no violations.

## Design

Decisions and rejected alternatives: [research.md](research.md). Types and transitions:
[data-model.md](data-model.md). Component and form contract: [contracts/ui.md](contracts/ui.md).
Validation: [quickstart.md](quickstart.md).

### Requirement map

| Requirement | Where |
|---|---|
| FR-001, FR-003 | `ui/material/accordion_group.rs` (new `AccordionGroup<K>`); `AccordionSection::in_group` (opt-in; no call means no change). Contract §Component |
| FR-002 | `AccordionGroup::toggle`: pressing the open key closes the group; `State` skips a held section first (FR-006) |
| FR-004 | `features/daemons.rs`: `flipped_sections` replaced by `sections: AccordionGroup<Section>`, one per form; `container_fields.rs` and `ui/settings/daemons.rs` keep calling `state.section_open(section)` |
| FR-005 | `Msg::SectionToggled` touches only the group; save path untouched (existing test `sections_open_by_toggle_and_by_error_without_touching_the_draft` extended) |
| FR-006 | `State::section_open` = `group.is_open(s, first) \|\| held_open(s)`; `SectionToggled` ignores a held section. Data-model §Transitions |
| FR-007 | The group is reset (`reset()` to "not chosen") where `flipped_sections.clear()` is called today (form open x3, `RuntimeChosen` when the kind changes) |
| FR-008 | `showcase/state.rs` holds an `AccordionGroup<usize>` and a new `Message::GroupSectionToggled(usize)`; `sections::surfaces::accordion_section` renders both sets; `catalogue.rs` poses |
| FR-009 | Unchanged: `AccordionSection::from` is not touched except `in_group`; existing focus and summary tests stay |

### Test strategy

| Requirement | Layer |
|---|---|
| FR-001, FR-002, FR-003 | Client unit (render-free): `accordion_group.rs` tests (default, open, replace, close-open, reopen after close, two groups independent); `accordion_section.rs` test that `in_group` sets `open` and a section without it is unchanged |
| FR-004, FR-005, FR-006, FR-007 | Client reducer: `tests/daemons_feature.rs` (every kind via `Section::of`; error-held section stays open when another opens and closes when fixed; press on held section no-op; form open and runtime change reset; two forms in turn do not leak; draft unchanged after toggles) |
| FR-008 | `tests/showcase_state.rs` reducer test for `GroupSectionToggled`; `tests/showcase_completeness.rs` poses |
| FR-009, no layout jump | `tests/daemons_section_geometry.rs` (existing; its `open_form` helper, which opens every section after the first, is replaced by one measurement per section so each is still laid out open); component tests (existing) |
| Look of simultaneous close and open | Quickstart §B visual pass |

## Project Structure

```text
specs/1091-settings-accordion-group/   plan.md research.md data-model.md quickstart.md contracts/ui.md
crates/micold-client/src/ui/material/accordion_group.rs     (new)
crates/micold-client/src/ui/material/accordion_section.rs   (in_group)
crates/micold-client/src/ui/material/mod.rs                 (mod + pub use)
crates/micold-client/src/features/daemons.rs                (group in State)
crates/micold-client/src/showcase/{state.rs,catalogue.rs,sections/surfaces.rs}
crates/micold-client/tests/{daemons_feature.rs,daemons_section_geometry.rs,showcase_state.rs,showcase_completeness.rs}
docs/user-guide/{sandboxed-daemon,kubernetes-daemons,remote-daemons}.md
```

Structure decision: single milestone; the component and its first two consumers are one small
change that cannot usefully ship apart.
