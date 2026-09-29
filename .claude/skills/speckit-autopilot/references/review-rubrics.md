# Agent review

Every review runs in a **fresh subagent**, because the author's context reads what it meant to
write. Give it paths, not a summary. It never edits.

## Dispatching a reviewer

Use `Agent` (`subagent_type: general-purpose`). Round 1 of every review (including review B's first
pass): omit `model`. Round 2 and later: `model: "sonnet"`.

Prompt parts, in order:

1. **Role.** "You are reviewing <artifact> for feature <NNN>. You did not write it. Do not edit any
   file."
2. **Read.** Artifact paths and `.specify/memory/constitution.md`. For code, also the milestone
   section of `tasks.md` and `git diff origin/main...HEAD`.
3. **Rubric.** The matching section below, verbatim.
4. **Output contract:**
   ```
   VERDICT: CLEAN | CHANGES
   F1 [BLOCKER|MAJOR|MINOR] <file>:<line or section> — <what is wrong>
      Evidence: <quote or command output>
      Fix: <concrete change>
   ```
   Report BLOCKER and MAJOR only when an item can be pointed at. Taste is MINOR.

After it returns:

- Verify each finding. Decline a wrong one and record the reason in the ledger.
- Fix every BLOCKER and MAJOR that holds up. Fix a MINOR only if it takes a few minutes.
- Re-dispatch a **new** reviewer, never the old one.
- A third round with BLOCKER or MAJOR is an escalation (category 5).

## Bug rubric (Phase 0, after `speckit-bugfix-verify`)

- `bugs/BUG-<k>.md` has reproduction steps. The reviewer follows them on `origin/main` and they
  reproduce the failure. If not, BLOCKER.
- The root cause names a file and a mechanism, not a symptom. The reviewer confirms it in that code.
- Every requirement the patch adds to spec.md describes behaviour the spec already intended: a
  missed edge case, or a conflict resolved in favour of the stated user story. New behaviour is
  MAJOR and sends the flow to Phase 1.
- Reopened tasks carry `(reopened — BUG-<k>)`. Fix tasks are few. The first is a regression test.
- `speckit-bugfix-verify` reports the BUG as Patched, with no orphaned references.

## Spec rubric (Phase 1)

- Every user story has a priority, an independent test and Given/When/Then acceptance scenarios.
- Every functional requirement is testable and names an observable outcome, not an implementation.
- Success criteria are measurable and technology-agnostic.
- Edge cases cover empty input, failure, concurrency (Principle II: multi-session) and
  cross-platform differences (Principle VI).
- No `[NEEDS CLARIFICATION]` markers beyond the three the template allows. Each open marker becomes
  a Phase 2 question.
- Scope: nothing in the spec is really a different feature. Out-of-scope items are listed.
- `checklists/requirements.md` agrees with the spec. A ticked item is true.

## Plan rubric (Phase 3)

- The Constitution Check table covers all eight principles. Each justified violation is in
  Complexity Tracking.
- Every functional requirement maps to a plan section, contract or data-model entity.
- Tech choices name real crates and modules in this workspace. The reviewer greps to confirm paths
  and types exist or are marked new.
- Local-first storage (Principle IV): nothing assumes a remote service.
- The test strategy says which layer tests each requirement (core unit, client, geometry gates,
  quickstart §B visual pass).
- research.md records each decision with its rejected alternatives.

## Tasks and milestone rubric (Phase 3, after `speckit-analyze`)

- `speckit-analyze` reports no CRITICAL or HIGH findings.
- Every task has an ID, a file path, and a story label where relevant. `[P]` tasks touch disjoint
  files.
- Test tasks come before the implementation tasks they cover (Principle I).
- The `## Milestones` section follows `milestones.md`:
  - every task is in exactly one milestone
  - M1 includes the P1 story
  - no milestone is Setup or Foundational alone
  - every deliverable is observable and has a **Verify** step
  - dependencies point backwards only
  - every user-guide task is in the milestone that ships its behaviour
- Every item in `checklists/*.md` is truly satisfied (and ticked) or reported as a finding.

## Code review B: conformance (Phase 4, every milestone)

Review A is the `code-review` skill at `high` (correctness bugs). Review B is this subagent. It
covers everything A does not:

- **Deliverable.** Run the milestone's **Verify** step and report the output. A deliverable that
  cannot be observed is a BLOCKER.
- **Scope.** The diff implements exactly the milestone's tasks. Work from a later milestone is
  MAJOR. A task ticked but not implemented is a BLOCKER.
- **Regression (bug milestones).** Run the regression test against `origin/main`. It must fail for
  the reason the BUG record gives, then pass on the branch.
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
- **Ownership.** The diff touches no other feature's `specs/` directory and no files unrelated to
  the milestone.
