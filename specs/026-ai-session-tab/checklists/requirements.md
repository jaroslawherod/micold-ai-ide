# Specification Quality Checklist: The AI Session as a Tab

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-16
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

Three decisions were settled with the reporter before writing rather than left as
[NEEDS CLARIFICATION] markers: strip visibility (always, superseding 012 FR-005), the AI tab's label
(the existing AI CLI icon), and its position (the strip's right-hand end). They are recorded in
Clarifications.

One ambiguity is recorded as an **assumption** rather than a marker, because a reasonable default
exists and the cost of being wrong is one requirement: "at the right side" is read as the right end
of the *strip*. The alternative reading — that the strip as a whole should be right-aligned in the
bar — is already true today, which is why the strip reading is the more likely intent. If it is
wrong, FR-002 is the only line that changes.

A **third** session on 2026-08-19 added FR-013–FR-015 on request: the tab and the strip become
shared components, the gallery poses both indicator orientations, and a tab's highlight becomes a
tab's state layer rather than a button's pill. FR-013 is not a preference — the gallery discovers
components structurally, so a tab assembled at the call site cannot be posed at all, which makes the
promotion the only route to FR-014. Two spec edits were needed to keep this checklist honest rather
than to tick it: code identifiers were taken back out of FR-013 and FR-015, and User Story 1 gained
an acceptance scenario for the highlight so FR-015 has criteria and not only a success measure.

FR-012 (a tab whose process is not running is visually distinct) was stated at P3 and **raised to
P2** by the second 2026-08-19 clarification session. It was scoped as consistency polish — "the AI tab
should not silently omit what the terminal tabs show" — and feature 012's BUG-005 then moved the
restart affordance off the tab into a menu, so no tab shows lifecycle and no cue points at the one
action the strip offers. It is still severable from Story 1, which is why it is not P1.
