# Specification Quality Checklist: Client-Managed Session Service Lifecycle

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-27
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

- Iteration 1: two [NEEDS CLARIFICATION] markers were open — FR-005 (fate of the Linux
  logout-survival opt-in, implemented today by a service-manager entry) and FR-006a (whether a live
  session holds the service up against the idle countdown).
- Iteration 2: both resolved by the user (Clarifications, session 2026-08-27) — connections alone
  drive the countdown, and the directly-hosted logout-survival opt-in is removed. FR-005/005a-c and
  FR-006a/006b/006c rewritten accordingly; User Story 2 and 3, the edge cases, the assumptions, and
  SC-009/SC-010 updated to match. All items pass.
- Carried forward for `/speckit-plan`: the clarified rule narrows the existing lifecycle invariant in
  `crates/micold-daemon/src/lifecycle.rs` ("never exit while any session is alive"), and retires the
  shipped US7/FR-038 logout-survival capability from feature 010. Both are deliberate, user-approved
  reversals of earlier decisions and should be recorded as such in the plan.
- Iteration 3 (post-plan, 2026-08-27): planning research R2 **falsified** FR-018/FR-019/FR-022 and
  User Story 4 scenario 5 by measurement — a container told to keep running is restarted even after
  a clean exit, so the idle stop and the sandbox's keep-it-running opt-in cannot both hold. The user
  approved the amendment ("the opt-in wins"). FR-018 now carries the exception, FR-019 is scoped to
  the opt-in being off, FR-022 states the mutual exclusion, FR-022a covers turning it back off, US4
  gains scenarios 5–6, SC-004/SC-008 are qualified, and a third Clarifications entry and two
  Assumptions record it. All items still pass.
