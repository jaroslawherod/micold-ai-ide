# Specification Quality Checklist: Worktree Provenance

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

- **Clarified 2026-08-31** — 5 questions asked and answered; all three original markers resolved, plus
  two consequences they surfaced:
  1. **FR-006** — migration grandfathers selectively, from evidence the app already holds (a stored
     displayed-label override, or a persisted session bound to the worktree).
  2. **FR-007** — 014's naming rule is deleted as a hiding signal and survives only as a veto on that
     migration (FR-007a), with a real record outranking it (FR-007b).
  3. **Out of Scope** — the "session live here" badge is deferred to its own feature.
  4. **FR-020…FR-024** — a claim action on a revealed row writes the same record, making any
     misclassification recoverable in one action.
  5. **FR-015a** — 014's user-visible strings (`agent` chip, "Show agent worktrees") are unchanged.
- One contradiction introduced by the migration answer was found and fixed during validation: FR-011
  previously covered "never written" as a fail-visible case, which would have negated FR-006. It is
  now scoped to a *failed read* only, and forbids the backfill from running on one.
- **Second clarify pass, 2026-08-31** — 2 further questions, both closing gaps the first pass created:
  6. **FR-006/006c/006d** — the evidence backfill is a one-time, per-project migration recorded as
     done, not a standing rule. Continuous evaluation would have let 014's "start a session in a
     revealed agent worktree" permanently un-hide that worktree, which the FR-007a naming veto cannot
     catch for an ordinary-named session worktree.
  7. **FR-025/025a/025b** — the reveal control shows how many worktrees are currently hidden, so the
     worktrees the migration takes away are accounted for on screen. No notification is added; FR-015a
     gained a carve-out saying the label text is unchanged and the count sits beside it.
- Ready for `/speckit-plan`.
