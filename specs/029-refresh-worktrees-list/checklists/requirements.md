# Specification Quality Checklist: Refresh the Worktree List on Demand

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-31
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

- Items marked incomplete require spec updates before `/speckit-clarify` or `/speckit-plan`.

### Validation record (iteration 1)

Two items failed on the first pass and were fixed before this checklist was marked complete:

- *No implementation details* — the Edge Cases and Assumptions sections named "the session
  service" as the thing the client talks to, and FR-004 described the refreshed listing being
  "applied through the same path the application already uses". Both named internal structure. The
  edge case is now stated as "the application is disconnected", the assumption as "reachable
  wherever the listing is read today", and FR-004 as an observable equivalence: a user must not be
  able to tell from the result which trigger produced the listing.
- *Success criteria are technology-agnostic* — passed on re-read; SC-002's "50 worktrees / 2
  seconds" is a user-facing volume and latency bound, not an internal metric.

### Deliberate scope exclusions (recorded so planning does not re-open them)

- Automatic, periodic, or filesystem-triggered refresh is out of scope (FR-012). The user asked for
  an on-demand control.
- No keyboard shortcut, no new notification or dialog surface, no new persisted state.
- The collapsed sidebar strip gains no control; refreshing requires a visible sidebar.

### Zero clarification markers — the three judgement calls made instead

1. **Where the control goes and how it looks**: left of "Add a worktree", compact icon button with
   a neutral tint. Recorded in Assumptions; the alternative orderings change no requirement.
2. **What "refresh" re-reads**: the same project-level pass the application already runs when a
   project is opened, including whatever else that pass picks up, rather than a narrower
   worktree-only variant invented for this control. Recorded in Assumptions.
3. **What happens on failure**: the previous listing stays on screen and the control returns to
   idle (FR-007, FR-008), rather than emptying the sidebar. This is the safer default and matches
   how the list behaves when a listing does not arrive today.
