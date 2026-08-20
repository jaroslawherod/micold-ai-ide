# Specification Quality Checklist: Choose which AI CLI a session runs on

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-14
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

- The two CLIs are named throughout (Claude Code, GitHub Copilot CLI) and a Copilot version is
  cited in the Assumptions. This is the subject of the feature, not implementation leakage — the
  spec never names a language, framework, file format, or storage engine.
- Nineteen clarifications are recorded across three sessions: two during authoring (2026-08-14),
  twelve on 2026-08-16 after the plan was written, and five on 2026-08-18 after the fourth analysis
  pass. All are in the spec's Clarifications section, so no markers remain. The 2026-08-18 five
  settle concurrent external attachment, the badge's scope, discovery timing, the label text, and the
  large-history claim (now SC-009).
- FR-018/SC-005 (the activity badge) **is no longer a droppable slice.** It was, on the assumption
  that the signal would have to be inferred from a database. Research R5 disproved that — Copilot
  writes a structured per-turn event log — and the 2026-08-16 clarifications withdrew the escape
  hatch, tightened SC-005 to one second, and committed to a cross-platform watch facility so FR-019
  can forbid polling outright. Dropping the badge from here would be a deliberate spec change.
- FR-021 anticipates a change to the existing provider seam's shape. **Its original rationale was
  wrong** and has been corrected in the spec: Copilot *does* organise its conversation storage by
  working directory (research R3 found a per-directory index of session ids), just in a different
  shape from Claude Code's. The requirement stands on the corrected ground — the seam must assume
  neither CLI's layout — and the replacement mechanism is settled in `contracts/ai-cli-provider.md`.
- **FR-014/FR-015 are net-new behaviour, not a generalisation of existing discovery.** Recorded here
  because three analysis passes assumed otherwise. Nothing in any `src/` calls
  `discover_transcript_session_ids`, `transcript_dir` or `is_archived`; the only exercise of them is
  `micold-core/tests/session_reconciliation.rs`, which its own module doc describes as a *mirror* of
  a client function that has since been deleted. The requirements themselves are unaffected and
  stand as written — what changed is the plan's and the task list's estimate of the work, and the
  fact that FR-014 needs a gate against the real entry point before it can be called covered.
