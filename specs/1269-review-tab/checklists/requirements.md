# Specification Quality Checklist: Review as a per-worktree tab that keeps its state

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-24
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [ ] No [NEEDS CLARIFICATION] markers remain
- [ ] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [ ] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- "Requirements are testable and unambiguous" and "All acceptance scenarios are defined" stay
  unticked until the clarify unit answers the two markers: FR-003 and FR-012 depend on them.

- Two `[NEEDS CLARIFICATION]` markers remain, both in User Story 2, left for the clarify unit:
  (1) what a second press of the Changes button does while the Review tab is shown (scenario 4);
  (2) whether **Review changes** on another location makes it the shown location, and where a
  sessionless worktree's review is drawn (scenario 5; also referenced by the edge case "Location's
  last session closed").
- FR-016 names testability without a window or renderer because the issue's acceptance asks for the
  logic to be unit-tested in the render-free core; it names no module or type.
- The issue text names `State.open` and `Msg::Closed | SessionSelected`; the spec states their
  observable effect (FR-005, FR-007) instead.
