# Specification Quality Checklist: Reopen on the session I was last using

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-08-11
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

**On the four questions the request asked the spec to settle**, all four are answered without a
clarification marker, because each has a defensible default drawn from behaviour the application
already has:

| Question | Answered by | Reasoning |
|---|---|---|
| Remembered session's record or worktree is gone | FR-005, FR-006, US3 | Fall back to no-memory behaviour and disturb nothing else. A memory pointing at something absent is worse than none. |
| Does restoring start a process? | FR-004, SC-005 | No. Restoring is display; starting is an explicit act. This is the rule feature 008's FR-001/FR-002 already keep on a switch. |
| Does it take keyboard focus? | FR-013 | No. Focus is a separate deliberate act — the same call BUG-001/focus-model already make for arriving somewhere. |
| The last-used session was closed | FR-005, US3 scenario 1 | Not restored. A closed session is not listed at all, so restoring one would display something the user cannot see in the panel. |

**Deliberately not marked as needing clarification**, though a reviewer might expect it: *when* the
memory is written (continuously vs at exit). Recorded as an assumption instead — "as the current
session changes" is the behaviour with no surprising failure mode, and the alternative only differs
after a force-kill, which the Edge Cases already cover.

**One thing the spec deliberately does not decide**: which *project* opens at launch. That is
existing behaviour and out of scope; this feature only decides which session is in front of the user
once a project has opened. Stated in Assumptions so planning does not widen into it.

**Prior art this leans on, and does not re-specify**:

- feature 008 FR-003a / [BUG-001](../../008-background-project-switching/bugs/BUG-001.md) — restoring
  a session whose process has stopped, within a run. FR-003 here is the same rule across restarts.
- feature 024 — the side panel reveals whatever session becomes current, so FR-012 is a
  consequence rather than new work.
