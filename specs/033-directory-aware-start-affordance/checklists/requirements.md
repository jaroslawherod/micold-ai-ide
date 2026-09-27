# Specification Quality Checklist: The start affordance answers for its own directory

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-27
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [ ] No [NEEDS CLARIFICATION] markers remain
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

- Three `[NEEDS CLARIFICATION]` markers are left open for Phase 2 by design (the autopilot flow does
  not ask in Phase 1): FR-001 (whether the affordance becomes directory-aware at all, or a cheaper
  product answer), FR-006 (when a row's first answer is asked for), FR-007 (a worktree's own
  directory or its project root).
- The spec names `PATH`, the environment-include script, the session service and the AI CLIs
  (`claude`, `pi`) because they are the product's domain — the user-visible settings and tools the
  answer is about — not an implementation choice. The *Background* section cites the code-level
  reproduction in prose only; the file-and-line trace lives in the ledger (D1).
