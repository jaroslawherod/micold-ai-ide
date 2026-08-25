# Specification Quality Checklist: Feature Encapsulation — Own Your Messages, Own Your State

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-25
**Last validated**: 2026-08-25 (iteration 2)
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

### Validation findings (iteration 1)

**Stakeholder framing — adapted, following feature 021's precedent rather than inventing one.**
This is an internal restructuring with no end-user-visible change, so the stakeholder *is* the
maintainer. The spec says so in its Context section and frames every story and success criterion as
cost-of-change rather than product behavior. Feature 021's checklist resolved the identical tension
the identical way; doing it differently here would make the two specs inconsistent about what a
non-user-facing feature is allowed to look like.

**Where implementation names appear, and why the criterion still holds.** Identifiers, file paths
and framework names appear in exactly two places, both evidentiary:

1. The **baseline table and Context section**, which cite measured facts (`app.rs:42–502`,
   119 variants, 18 `Widget` impls). A baseline that does not say where it was measured is not a
   baseline — 021's checklist made this point after discovering its own description cited paths
   that no longer existed.
2. The **framework-constraint paragraph**, which exists to record that an alternative was
   considered and rejected on evidence, not only forbidden by Principle V. Suppressing the
   framework's name there would hide the reasoning the paragraph exists to preserve.

Every **FR** and every **SC** is written in framework-agnostic terms — "message vocabulary",
"reducer entry point", "the component that owns it", "the shared component library", "the
environment that runs without a window". Verified mechanically over the Requirements and Success
Criteria sections rather than by eye; the scan found two leaks and both were corrected: SC-007 said
"root state struct" and FR-012 said "per-feature widgets".

**Baseline measured, not inherited.** Every figure in the baseline table was measured against the
working tree at `b43c11c` on 2026-08-25, not copied from feature 021's completion table. Three of
021's closing figures have since moved and the spec uses the current values: the message enum is
**119** variants (021 recorded 120), the state struct has **44** public fields (021 recorded 45),
and `State::update` is **300** lines (021's checkpoint recorded 834, before Phase 6 removed the
worktree-form arms). Reporting 021's numbers as though they were still true would have been the
easy error, and it would have overstated the problem by a factor of nearly three on the reducer
row.

**The central premise was verified against the code, not taken from the user's report.** The claim
that 021 nested exactly one of eleven features was checked by locating every message enum and every
reducer entry point under `src/features/`: one of each, both in `worktree_form.rs`. The claim that
even that feature did not gain local state was checked by reading its imports
(`use crate::app::{Message, State}`). Both are cited in the spec with line numbers so a reviewer
can repeat the check rather than trust it.

**No clarification markers were needed.** Three decisions could have become markers and were
instead settled as documented Assumptions, because each has a defensible default and because
leaving them open would reproduce the exact failure this feature exists to correct:

- *Which state qualifies to move* — settled mechanically (one writer, no outside reader) rather
  than per-feature judgment. 021's optional judgment reached one feature in eleven.
- *Whether a feature's view counts as part of the feature* — settled yes; settled no would move
  nothing, since views live beside features by existing convention.
- *Whether the session/terminal cluster is in scope* — settled as in-scope for Story 1 and
  case-by-case for Story 2, because it is simultaneously the largest share of the root vocabulary
  and the state most likely to have multiple readers.

### Validation findings (iteration 2)

**SC-002 and SC-003 state a rule rather than a target number, and that was a deliberate choice
rather than an oversight.** 021's own completion analysis found that "a number in a table asserts
it on the day it was taken", and two of its rows moved the wrong way for correct reasons. Both
criteria therefore state the rule as the target and report the count as evidence — which is also
what makes them checkable by a guard rather than by a person with a calculator.

**User Story 3 is P1 despite being listed third, and the ordering is deliberate.** It reads like
housekeeping and is in fact the load-bearing story: without it this feature is feature 021 again.
The priority is stated with that reasoning attached rather than left to look like an error.

**One item deserves a caveat for the planning phase, recorded here rather than silently passed.**
SC-001 ("changes one feature module and its view, and no other file") is measured by making a
change and counting. That is stronger than an assertion but weaker than a guard, because it is
measured once. `tests/feature_registration_cost.rs` already guards the analogous claim for *adding
a feature*; whether the same machinery can guard *changing* one is a planning question, not a
specification one, and is flagged for `/speckit-plan` rather than answered here.

### Outcome

All items pass. Ready for `/speckit-clarify` (optional — no open questions remain) or
`/speckit-plan`.
