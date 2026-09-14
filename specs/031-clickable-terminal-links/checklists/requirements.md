# Specification Quality Checklist: Clickable links in the terminal

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-14
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

- FR-006 names `FORCE_HYPERLINK` and `TERM_PROGRAM` on purpose: they are what a user types into the session environment-include script to opt in (clarify decision), so they are user-facing configuration, not implementation detail. Tasks review round 2 (2026-09-14) raised this against the two content-quality items; they stay ticked on that basis.
- Clarify (2026-09-14) resolved the three markers by user decision: FR-004 is Ctrl/Cmd+click, FR-006
  advertises nothing (opt-in documented, inherited identity variables stripped), FR-011 allows no
  application-specific schemes. A second clarify scan found no further critical ambiguities.
- Review round 4 (2026-09-14) made FR-006's identity rule checkable (same identity variables in every
  session, sandboxed included), defined what hover shows for sandboxed `file` links (FR-008, SC-006),
  defined a declared link's extent (FR-007, US2 scenario 4), and added edge cases for encoded and
  Windows `file` paths, all-executable mounts and pending opens, plus Linux package types (FR-013).
- Review round 3 (2026-09-14) found that AI CLIs which break their own lines deliver long addresses
  in pieces, so every marked link now shows its address on hover (FR-008) and SC-002 checks the
  first-row piece; it also scoped FR-014 focus and SC-003 timing around the sandbox confirmation,
  pinned the activation moment (FR-004), added the sandbox `localhost` edge case, and widened the
  runnable lists (FR-013).
- Review round 2 (2026-09-14) accepted this machine's and the sandbox's hostname in `file` links,
  resolved the remote-host contradiction, pinned the menu to the link captured when it opened,
  scoped SC-002 to covered addresses, defined runnable files per platform (FR-013, SC-007), added a
  confirmation for sandboxed `file` links (FR-018a), and fixed the FR-016 cross-reference.
- Review round 1 (2026-09-14) corrected the sandbox file-link edge case against the real mount table,
  added the runnable-file and remote-`file` rules, resolved plain-text scheme coverage, made the
  mouse-reporting rule gesture-independent, advertised hyperlink support (FR-006) and made SC-005
  numeric.
- Scheme names (`http`, `mailto`, `file`) and "declared hyperlink" appear in the spec because they are
  the user-visible kinds of link, not implementation choices.
