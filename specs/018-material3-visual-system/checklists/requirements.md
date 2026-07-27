# Specification Quality Checklist: Material 3 Visual System

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-07-26
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [ ] Written for non-technical stakeholders
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

- **15/16 pass.** Re-validated 2026-07-27 after the clarification session.
- **Regression — "Written for non-technical stakeholders"** now fails, and this is a real cost
  rather than a wording nit. The spec grew from 44 to 87 requirements, and roughly twenty of them
  (the feature 017 series) specify component architecture: a behavior/appearance layer split, widget
  wrapping, state ownership, overlay consolidation. A non-technical reader cannot evaluate those.
  They are legitimate spec content here — Constitution Principle VIII makes component architecture a
  governance concern, not an implementation detail — but they are not stakeholder-readable.
  - **Mitigation, already applied**: the 2026-07-27 clarification split the work in two. Feature A
    carries the architectural requirements; Feature B carries the user-visible visual system and
    reads for a general audience. Executing that split resolves this item without deleting content.
  - Accepted as-is until the split is executed. Not a blocker for planning, which has already run.
- The other 15 items pass, including the ones most at risk from the growth: no
  `[NEEDS CLARIFICATION]` markers remain, all 19 success criteria are measurable, and scope is now
  *more* clearly bounded than before thanks to the explicit Feature A / Feature B boundary.
- Two earlier clarifications were superseded on 2026-07-27 and are annotated in place rather than
  rewritten, so the decision history stays legible: the snackbar is now the *first* of two
  sanctioned behavior exceptions, and the two row densities are now two points on a four-step
  density scale (heights unchanged).
- Validated on 2026-07-26, one iteration.
- Four decisions (D1–D4) were flagged by explicit request in the feature description, which named
  them as decisions to resolve rather than assume. This deliberately exceeded the usual
  three-marker guideline. All four were put to the user and resolved in the same pass; they now
  appear in the spec's **Resolved Decisions** section with rationale, and each is carried by a
  functional requirement (FR-005a, FR-008a, FR-025) and by `contracts/design-tokens.md`.
  - D1 → bake Material 3 tonal ramps as core data; roles are palette+tone pairs.
  - D2 → Roboto, two static instances (weight 400 and 500).
  - D3 → one seed, both schemes derived from the same ramps.
  - D4 → small app bar retained; medium/large variants not adopted.
- Terms like "surface tone", "elevation level" and "type role" are Material 3 design vocabulary,
  not implementation details; they name the user-visible visual concepts the feature delivers.
  Likewise the concrete dp values in the contract are the *specification* of the visual result, not
  a description of how it is coded.
- The contract at `contracts/design-tokens.md` supersedes
  `specs/003-material-design-layout/contracts/design-tokens.md` in full, and records the migration
  from that contract's five raw type sizes and four radii (§2.5, §3).
