# Phase 4: milestone unit

For milestone K:

1. `speckit-implement` with `Milestone MK only: tasks T0xx–T0yy. Do not start any other task. Stop
   when these are done.` Afterwards confirm every task in the range is ticked and none outside it. Its
   mandatory `tdd.run` hook drives red → green → refactor. For a bug, the first task is a regression
   test that fails on `origin/main` for the reported reason. If it asks "proceed anyway?", never
   answer "yes": close the checklist item as in Phase 3 (confirm, or fix the spec or plan), and
   escalate only when it needs a user decision.
2. `mise run gate`, detached as in [../references/pr-and-merge.md](../references/pr-and-merge.md)
   §2. If `cfg(target_os)` code changed, also
   `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`. If
   anything visible changed, run the `visual-pass` skill. It is a forked Sonnet subagent that sees
   only its arguments: pass the worktree path, the quickstart section or change to check, and what
   counts as a pass.
3. **Code review. Run both, in parallel:**
   - **A**: the `code-review` skill at `high` on `origin/main...HEAD`.
   - **B**: a fresh subagent checks the diff against the milestone's deliverable, its acceptance
     scenarios and the constitution (conformance rubric).

   Verify each finding against the code first. Fix real ones and go back to step 2. Decline a
   finding that contradicts the spec, and record why in the ledger.
4. Tick the milestone's tasks, update the ledger, commit, push, and open the PR. Title
   `feat(NNN): <deliverable>`, or `fix(NNN): … (BUG-<k>)` for a bug.

**Continued with a red CI log:** run `systematic-debugging`, fix, re-run the gate, push, and return
`DONE` again. After the third failed attempt, escalate (category 5).
