# Specification Quality Checklist: Notify When a Session Needs Attention, and Track Unread Sessions

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-02
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
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [ ] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- Three `[NEEDS CLARIFICATION]` markers are open and wait for the clarify phase: FR-008 (what
  happens with no window open), FR-023 (whether the closed switcher button shows unread) and FR-028
  (a switch per AI CLI). The three unticked items wait on them alone: those three requirements are
  not yet testable and have no acceptance scenario of their own.
- "Operating system", "desktop notification", "container" and the AI CLIs' names are the
  user-facing terms of the feature, not implementation details. FR-030 names the shared component
  library and the showcase because the constitution's Principle VIII makes reuse a requirement.
- The issue names Linux and macOS; the spec requires Windows too (Principle VI). See spec.md,
  Assumptions.
