# Rubric: review B, conformance (feature flow, every milestone)

Review A is the `code-review` skill at `high` (correctness bugs). Review B is this subagent. It
covers everything A does not:

- **Deliverable.** Run the milestone's **Verify** step and report the output. A deliverable that
  cannot be observed is a BLOCKER.
- **Scope.** The diff implements exactly the milestone's tasks. Work from a later milestone is
  MAJOR. A task ticked but not implemented is a BLOCKER.
- **Regression (a bugfix milestone, when B is run).** Run the regression test against `origin/main`. It must fail for
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
