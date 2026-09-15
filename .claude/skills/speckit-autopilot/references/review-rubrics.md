# Agent review

In this flow the agent reviews everything, because no human does. The context that produced an
artifact is the worst one to judge it: it reads what it meant to write, not what it wrote. So
every review below runs in a **fresh subagent**. The subagent is given paths, not a summary, and it
never edits.

## Dispatching a reviewer

Use the `Agent` tool (`subagent_type: general-purpose`). Choose the model by round:

- **Round 1** of every review, including review B on a milestone's first pass: omit `model`, so
  the reviewer runs on the session's model. This is the review that has to find the problems.
- **Round 2 and later**, which check that earlier findings were fixed and nothing regressed:
  `model: "sonnet"`.

The prompt has these parts, in this order:

1. **Role.** "You are reviewing <artifact> for feature <NNN>. You did not write it. Do not edit any
   file."
2. **Read.** The artifact paths, the constitution (`.specify/memory/constitution.md`), and for
   code, the milestone section of `tasks.md` and the diff command `git diff origin/main...HEAD`.
3. **Rubric.** The matching section below, copied verbatim.
4. **Output contract:**
   ```
   VERDICT: CLEAN | CHANGES
   F1 [BLOCKER|MAJOR|MINOR] <file>:<line or section> — <what is wrong>
      Evidence: <quote or command output>
      Fix: <concrete change>
   ```
   Report BLOCKER and MAJOR only when an item can be pointed at. Taste is MINOR.

After the reviewer returns:

- Verify each finding yourself against the artifact or code. If a finding is wrong, decline it and
  record the reason in the ledger.
- Fix every BLOCKER and MAJOR that holds up. Fix a MINOR only when it takes a few minutes.
- Re-dispatch a **new** reviewer. Do not reuse the old one: it has seen the old version.
- A third round that still returns BLOCKER or MAJOR is an escalation (category 5).

## Bug rubric (Phase 0, after `speckit-bugfix-verify`)

- `bugs/BUG-<k>.md` gives reproduction steps that the reviewer follows on `origin/main`, and they
  reproduce the failure. If they do not, that is a BLOCKER.
- The root cause names a file and a mechanism, not a symptom. The reviewer confirms it by reading
  that code.
- Every requirement the patch adds to spec.md describes behaviour the spec already intended: a
  missed edge case, or a conflict resolved in favour of the stated user story. New behaviour is a
  MAJOR finding, and it sends the flow to Phase 1.
- Reopened tasks carry `(reopened — BUG-<k>)`, and the fix tasks are few. The first one is a
  regression test.
- `speckit-bugfix-verify` reports the BUG as Patched, with no orphaned references.

## Spec rubric (Phase 1)

- Every user story has a priority, an independent test and Given/When/Then acceptance scenarios.
- Every functional requirement is testable. Each one names an observable outcome, not an
  implementation.
- Success criteria are measurable and technology-agnostic.
- Edge cases cover empty input, failure, concurrency (Principle II: multi-session) and the
  cross-platform differences (Principle VI).
- There are no `[NEEDS CLARIFICATION]` markers beyond the three the template allows. Every open
  marker becomes a Phase 2 question.
- Scope: nothing in the spec is really a different feature. Out-of-scope items are listed.
- `checklists/requirements.md` agrees with the spec. A ticked item is true.

## Plan rubric (Phase 3)

- The Constitution Check table covers all eight principles, and each justified violation appears in
  Complexity Tracking.
- Every functional requirement maps to a plan section, contract or data-model entity.
- The tech choices name real crates and modules in this workspace. The reviewer greps to confirm
  that paths and types exist, or that they are marked as new.
- Local-first storage (Principle IV): nothing assumes a remote service.
- The test strategy says which layer tests each requirement (core unit, client, the geometry gates,
  quickstart §B visual pass).
- research.md records each decision with its rejected alternatives.

## Tasks and milestone rubric (Phase 3, after `speckit-analyze`)

- `speckit-analyze` reports no CRITICAL or HIGH findings.
- Every task has an ID, a file path, and a story label where relevant. `[P]` tasks touch disjoint
  files.
- Test tasks come before the implementation tasks they cover (Principle I).
- The `## Milestones` section follows `milestones.md`:
  - every task belongs to exactly one milestone
  - M1 includes the P1 story
  - no milestone is Setup or Foundational alone
  - every deliverable is observable and has a **Verify** step
  - dependencies point backwards only
  - every user-guide task sits in the milestone that ships its behaviour
- Every item in `checklists/*.md` is either truly satisfied (and ticked) or reported as a finding.

## Code review B: conformance (Phase 4, every milestone)

Review A is the `code-review` skill at `high`, which covers correctness bugs. Review B is this
subagent, and covers everything A is not asked about:

- **Deliverable.** Run the milestone's **Verify** step and report the output. If the deliverable
  cannot be observed, that is a BLOCKER.
- **Scope.** The diff implements exactly the milestone's tasks. Work belonging to a later milestone
  is MAJOR. A task ticked but not implemented is a BLOCKER.
- **Regression (bug milestones).** The reviewer runs the regression test against `origin/main` and
  sees it fail for the reason the BUG record gives. It must then pass on the branch.
- **Acceptance.** Each listed acceptance scenario has a test that fails without the change.
  `tdd/cycle-log.md` shows red before green for each behaviour.
- **Constitution:**
  - I: tests first.
  - II: multi-session safety.
  - IV: local-first.
  - VI: `cfg` arms for all three OSes.
  - VII: the user guide is updated when behaviour is user-facing.
  - VIII: UI composes shared components rather than styling widgets.
- **Leftovers.** No `todo!()`, `dbg!`, commented-out code, or unreachable half-wired UI.
- **Ownership.** The diff touches no other feature's `specs/` directory, and no files unrelated to
  the milestone.
