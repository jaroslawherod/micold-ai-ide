# Specification Quality Checklist: macOS Package

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

**Status: all items pass.** Ready for `/speckit-plan`.

## Notes

### Validation pass 1 (2026-08-27)

**Failing**: "No [NEEDS CLARIFICATION] markers remain" — 2 markers, both scope-level decisions with
no safe default, so both were put to the user rather than guessed:

1. macOS trust level — gated FR-012..FR-015 (as then numbered), US2, SC-004.
2. Logout survival via an opt-in background registration, or keep pointing at the container
   placement — gated US6 and its two lifecycle requirements.

Every other item passed on the first pass.

### Validation pass 2 (2026-08-27) — after clarification

Both answers were folded into the spec, not merely recorded:

- **Ad-hoc signed.** US2 was rewritten around a first launch that *is* blocked and a documented
  gesture that clears it, rather than one that may or may not be. The trust requirements became
  FR-012..FR-018: an ad-hoc signature (FR-012), a predicted outcome (FR-013), a Finder-only gesture
  (FR-014), automated signature verification at release time (FR-015), no account or secret required
  (FR-016), and a release process shaped so Developer ID + notarization is a later credential drop
  rather than a redesign (FR-017). Key Entities lost "Signing identity" and "Notarization ticket" and
  gained "Ad-hoc signature", "Quarantine flag", and "Download-page trust notice". SC-004 and SC-010
  were tightened accordingly, and two edge cases were added for the per-copy nature of the block
  (clearing it on the mounted download instead of the installed copy; meeting it again after an
  update).
- **Keep pointing at the container.** US6 became a documentation-and-boundary story rather than a
  feature story. FR-024 now forbids registering any background component at install; FR-025 requires
  the documentation to state the logout boundary and name the container placement as the answer.
  FR-030 was reworded from "bring the existing statements into agreement" to "check them and leave
  them in agreement", since they already say the right thing. SC-011 and SC-012 were added to make
  both halves measurable.

Requirements were renumbered once, in one pass, when the trust block grew from four entries to
seven; FR-001..FR-011 are unchanged and the tail then ran to FR-034.

### Validation pass 3 (2026-08-28) — after `/speckit-clarify`

Five clarifications were folded in. All 16 items passed before this pass and all 16 pass after it;
nothing regressed and nothing was newly unchecked. What changed, and why each item still passes:

- **Supported macOS floor** (Q1). FR-004 became a moving floor — the two most recent releases —
  rather than a pinned version. Still testable: the declared value must equal the second-newest
  release at the time the release is produced, and the operating system, not the user, enforces it.
  The floor is what collapses the first-launch block into a single documented gesture (FR-014), so
  "requirements are testable and unambiguous" is stronger after this, not weaker.
- **Release failure handling** (Q2). FR-011 now holds the whole release as an unpublished draft until
  every platform's artifact is attached. This closed a real gap under "scope is clearly bounded": the
  spec previously did not say what a partial release does, and publishing is immutable.
- **Per-PR packaging verification** (Q3). FR-036 requires the bundle to be assembled *and shown to
  start* on every change able to affect it; FR-037 confines the second architecture, the delivery
  container, and the signature check to the release, tied back to FR-011's draft-hold. SC-013 makes
  it measurable. This is what moved "all functional requirements have clear acceptance criteria" from
  passing-on-intent to passing-on-evidence.
- **File-access posture** (Q4). FR-027 became least-privilege and per-location with an explanation
  attached to each request; FR-029 was added for the optional one-time broad grant, stated as
  optional and never a prerequisite. FR-030 now requires the documentation to cover it. No
  implementation detail leaked: no requirement names a permission API, an entitlement, or a
  system-settings pane.
- **Running from the download** (Q5). The Edge Cases section carried an explicit either/or — work
  from the mounted location, or refuse. FR-019 resolves it as refuse-with-an-instruction, and the
  edge case was rewritten to match. This was the last unresolved disjunction in the spec, so
  "requirements are testable and unambiguous" now holds with no exception.

Requirements were renumbered twice more, each time in one pass: once when the permissions block grew
by one entry, once when FR-019 was inserted. FR-001..FR-018 are unchanged and the tail now runs to
FR-037; success criteria run to SC-013. No [NEEDS CLARIFICATION] markers remain.

### Validation pass 4 (2026-08-31) — after `/speckit-analyze`

One requirement was rewritten. All 16 items passed before and pass after; the rewrite strengthens
"requirements are testable and unambiguous" rather than trading it away.

- **FR-004's floor clause.** As written after pass 3, FR-004 required the declared value to *equal*
  the second-newest macOS release at the time the release is produced. That is a MUST nothing in this
  project can check: no signal available here knows when Apple has shipped a major release, so the
  clause could sit silently false for a year with every gate green. `/speckit-analyze` flagged it as
  an unenforced MUST (finding I1). It now says so plainly and places the obligation on how the value
  is maintained instead — recorded in exactly one place, stated identically in the documentation with
  an automated check enforcing the agreement, and re-checked against Apple's current releases as a
  named step in the developer documentation for producing a release. Three checkable requirements
  replace one uncheckable one. **This supersedes the first bullet of pass 3**, which recorded the
  older clause as testable; the decision it came from (Q1: the two most recent releases, macOS 15
  today) is unchanged, and no clarification was reopened.

Nothing else in the spec changed: FR-001..FR-003 and FR-005..FR-037 are untouched, success criteria
still run to SC-013, and no [NEEDS CLARIFICATION] markers were introduced.

The other findings from that analysis were fixed outside the spec — in `tasks.md` (test-before-
implementation ordering inside the foundational phase, a missing permissions-documentation task for
FR-029, the Principle VIII gate for the new screen, and a concrete mechanism for the release-page
trust notice) and in `quickstart.md` (§A's gate list brought back into agreement with `tasks.md`).
They are recorded here only so this file explains why the spec moved and the checklist did not.

### Deliberate wording choices

Recorded so a reviewer does not read them as leaked implementation:

- Named platform concepts (Apple silicon, Intel, notarization, Developer ID, Launchpad, Spotlight,
  the Dock, the Finder) are the operating system's user-facing vocabulary, not the project's
  technology choices. A macOS packaging spec that avoided them would be unreadable and untestable.
- No requirement names a bundle layout, a file format, a signing tool, a build flag, or a workflow
  step. FR-008 says "the platform's conventional delivery container", not the format; FR-001 says
  "self-contained application", not the directory structure. Those belong in the plan.
- "Ad-hoc signature" in FR-012 is the user-visible trust *state* the release commits to, not a tool
  invocation — it is what the operating system reports and what FR-015 verifies.
