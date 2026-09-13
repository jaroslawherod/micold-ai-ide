# Specification Quality Checklist: Run a session on the Pi coding agent

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-09-03
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

**Iteration 1 (2026-09-03, `/speckit-specify`)** — one item failed: a `[NEEDS CLARIFICATION]` marker
stood on FR-012, the activity badge for Pi sessions. It was not a detail a default settles. Pi's only
*reported* busy/idle signal reaches code loaded into Pi as an extension, and Pi's own documentation
states extensions run with the user's full system permissions, so whether this application supplies
one is a security decision belonging to the user — and both alternatives (inferring from Pi's
conversation file, or shipping Pi with no badge) were viable, since feature 026 already established
that a badge may degrade rather than block a provider.

**Iteration 2 (2026-09-03, `/speckit-specify`)** — resolved; all 16 items passed. The answer was to
supply the code, scoped to this application's own launches, with the cost bounded rather than
accepted: FR-012a–FR-012d.

**Iteration 3 (2026-09-03, `/speckit-clarify`)** — five further ambiguities found and closed; all 16
items still pass. No item regressed. What changed:

- **Domain & data model, identity (was Partial).** The spec had punted the correspondence between a
  session and Pi's own conversation record to the design phase. FR-005a now fixes the conversation in
  Pi's own store, addressed by the application's session identity, and FR-005b settles the fallback
  if Pi will not accept an application-chosen location — so the research question changes the mechanism
  and not the behaviour. FR-015 and FR-016 were rewritten against that single store, and FR-016
  gained an explicit prohibition on destroying history that is now shared with the user's own `pi`.
- **Security & privacy (was Partial).** FR-012c required disclosure without recourse. FR-012e adds
  the setting that declines the activity component, on by default, degrading through the path
  FR-012d already specified; FR-012f forbids presenting a session running without it as faulty.
  SC-005a measures it.
- **Edge cases, conflict resolution (was Missing, and opened by the store decision).** FR-006a–d
  split a known collision from an inferred one: two of the application's own sessions on one
  conversation is a refusal, anything else is a warning the user can proceed past, an unanswerable
  check is silence, and none of it may scale with a project's history. SC-005b measures it.
- **Integration, versioning (was Partial).** FR-003a settles that availability stays the presence
  check every provider gets — no version floor, no probe — with the failure reports naming the
  installed version instead. SC-006a holds the cost.
- **Terminology (was Partial).** The spec had pinned the label register (`pi`) and never the menu
  register. FR-001a fixes "Pi Coding Agent" for menus and sentences and forbids the two crossing;
  SC-004a checks it.

On terminology generally: the spec names `pi`, `claude` and `copilot`, the sidebar, and the sandbox
image. These are the product's domain vocabulary and its user-visible surfaces, not implementation
details — the same register features 026 and 027 use. Pi's `turn_start` / `agent_settled` events and
its `--session` flag are named only inside Clarifications records, where the rejected alternatives
cannot be understood without them; no requirement depends on those names. No language, framework,
file format, module, or storage path appears in a requirement.

**Iteration 4 (2026-09-12, `/speckit-clarify`, second pass)** — five further ambiguities found and
closed; all 16 items still pass. No item regressed. What changed:

- **Integration and external dependencies (was Partial).** The activity component had to exist
  wherever a session runs, and nothing said where it came from under sandboxed placement. FR-012a
  now has it travel with the application's own session service, and FR-017a keeps an image's
  obligation at `pi` and its runtime — so no image can run Pi sessions correctly while silently
  losing the badge. SC-007a measures the parity.
- **Domain and data model, identity (was Partial, and newly contradictory).** The store decision had
  the application "naming the conversation after its own session id" while FR-011 shows the name Pi
  recorded — together, a UUID on every row this application started. The identity is now carried by
  *where* the conversation is stored and never by Pi's name field, which stays the user's (FR-005a,
  FR-005b, FR-011). SC-004b checks it. The earlier Clarifications record was corrected in place
  rather than left to contradict the requirement.
- **Interaction and UX flow (was Partial).** FR-012e said "a setting" without saying where it
  applies. It is now application-wide, one switch beside the default-CLI setting, not duplicated per
  project or per session — a consent control found in three places is one nobody can answer with
  confidence. SC-005a carries the measurement.
- **Non-functional, performance and scale (was Partial).** FR-015's bound counted conversations and
  said nothing about their size, while Pi writes a conversation as one record that grows for its
  whole life. FR-011 now bounds a label read to a prefix, FR-015 says the cost may not grow with
  conversation length, and SC-006b makes opening a busy project cost what opening a new one does.
- **Edge cases (was Partial).** Forking is an ordinary Pi workflow and the spec was silent on it.
  FR-015 now lists a fork as an ordinary independent session, with parentage never read — the
  alternatives both required reading the session tree this feature declines to surface. Out of Scope
  says the same in the negative: the relationship is not depicted, the conversation is still listed.

Also corrected in this pass: Key Entities described activity as something that "may be derived from"
Pi's conversation record, which FR-012 forbids outright. It now says so.

One research question remains for `/speckit-plan`, and it is flagged in the spec rather than hidden:
whether Pi accepts an application-chosen location for a conversation that does not yet exist. FR-005b
settles the behaviour either way, so it is a mechanism question, not an open requirement.

All items pass. Ready for `/speckit-plan`.
