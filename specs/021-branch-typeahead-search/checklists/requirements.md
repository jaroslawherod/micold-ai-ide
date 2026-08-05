# Specification Quality Checklist: Branch Selector Type-Ahead Search

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-04
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

## Validation Notes

### Re-validation after `/speckit-analyze` (2026-08-04)

Eleven findings applied across all six artifacts. Still 16/16 — no item changed state — but two of the
findings were Principle I failures, and both were in the *plan*, not the spec, which is why the
checklist did not catch them:

- The keyboard rule was untested decision logic sitting in `src/ui/cdk/`, outside what the GUI-wiring
  exception covers. It moved to `micold-core` (FR-017a, FR-021), following `keymap.rs`'s precedent.
- User Story 1's checkpoint claimed a green suite it could not have had: `showcase_completeness.rs`
  fails for a library component with no catalogue entry, and the entry was scheduled two stories
  later.

Requirements added by the pass: FR-001b (what opens the list — previously nothing did), FR-012b
(unavailable rows distinguishable by more than the absence of emphasis, moved out of FR-011 where it
did not belong), FR-017a (the keyboard's exact rules), FR-021a (FR-019 held by a build check rather
than by review). A Terminology section now separates **emphasis** (matched text) from **highlight**
(the keyboard's row); the two words had been used for both.

Coverage gaps closed: FR-013, SC-001 and SC-006 had requirements but no verification.

### Re-validation after the post-plan `/speckit-clarify` (2026-08-04)

Three further clarifications, all surfaced by writing the design rather than by reading the spec
again. Still 16/16 — no item changed state. What changed underneath:

- **A contradiction the plan itself introduced, caught before implementation.** The component contract
  said a disabled row is not pickable, while the picker section said the refusal stays at Create as
  feature 016 left it. Both cannot be true. FR-012a resolves it: a blocked branch is listed with its
  reason and cannot be picked. This *supersedes shipped behaviour*, so it is recorded as such in the
  spec, in research R13a, and in the plan's risk table — not slipped in as a detail.
- **A regression avoided.** The plan had the search field holding query text only (correct — FR-014
  needs it), but that silently dropped the current `Select`'s habit of marking the current value when
  its list reopens (feature 013, FR-003). FR-014b puts the marking back, distinct from the keyboard
  highlight.
- **The last unmeasurable success criterion is now measured.** SC-003's "95% of cases" had no corpus
  and no method, so "success criteria are measurable" was passing on the strength of the other six.
  It is now a pinned corpus with committed query pairs and an assertion on the rate.

The remaining Outstanding items are unchanged and still low-impact: accessibility and localization are
unaddressed, consistent with the rest of the application.

### Re-validation after the first `/speckit-clarify` (2026-08-04)

Five clarifications plus one user directive were integrated; re-checked all 16 items against the
updated spec. Still 16/16 — no item changed state. What the session fixed:

- **Two items were passing on thin ice and are now solid.** "Success criteria are measurable" rested
  on SC-002's "no perceptible lag", and "Edge cases are identified" rested on a long-name edge case
  whose resolution was "the row must not appear unmarked" — neither was actually testable. SC-002 is
  now a 16 ms frame budget plus a no-dropped-frames clause, and FR-011d states exactly where the
  ellipsis goes.
- **FR-010 was self-contradictory** and would have failed "requirements are testable and
  unambiguous" under scrutiny: it demanded highlighting cover every matched character and no
  unmatched one, which no typo match can satisfy. Now split by match kind.
- **A correctness hole closed**: FR-006a's 3-character floor. Without it, FR-006 and FR-008
  contradicted each other for short queries — single-edit tolerance over a 2-character text matches
  nearly every branch.
- **"No implementation details" re-examined and still passing.** FR-011a–FR-011d name Material
  Design 3, design tokens, and the shared component library. Kept, on the grounds that a design
  language is product vocabulary rather than a technology choice, and that these entered by explicit
  user directive; no framework, widget, crate, or API appears.

### Initial review (2026-08-04)

All items passed on the first iteration. Points worth recording:

- **"Close to it" was resolved by informed guess, not left open.** The description's "or a close to
  it" admits several readings (subsequence, edit distance, phonetic, multi-error). FR-006 pins it to
  abbreviation-style subsequence plus single-character edit distance, and the Assumptions section
  records both the choice and why broader fuzziness was excluded (unexplainable results conflict with
  SC-005). No [NEEDS CLARIFICATION] marker was warranted — a reasonable default exists and the
  alternatives are narrower/wider variants of the same behavior rather than different features.
- **US3 is developer-facing by design.** "Written for non-technical stakeholders" is satisfied in the
  sense that matters here: the story states a reuse outcome and how to observe it, without naming a
  language, framework, or widget. The reusable-component requirement comes from the user's own
  request and from the project constitution's shared-primitive principle, so it belongs in the spec.
- **FR-021 is the closest call on implementation leakage.** It requires the matching and ranking
  behavior to be exercisable without rendering the interface. Kept because it is a testability
  constraint the constitution imposes on this feature and is phrased as an outcome ("exercisable
  without rendering") rather than a mechanism — it names no module, crate, or test harness.
- **Regression surface is spelled out rather than assumed.** FR-012 through FR-016 exist so that
  planning treats "the picker still does everything feature 016 made it do" as testable requirements
  rather than as an implicit hope.

## Notes

- Items marked incomplete require spec updates before `/speckit-clarify` or `/speckit-plan`
