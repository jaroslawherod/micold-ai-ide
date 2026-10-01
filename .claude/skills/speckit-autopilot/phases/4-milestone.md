# Phase 4: milestone unit

For milestone K:

0. **Brief.** `scripts/autopilot/brief.py milestone <feature-dir> MK` prints the milestone's block,
   its tasks, the requirements it satisfies and its stories' scenarios. Work from that. Do not read
   spec.md or tasks.md whole; pull anything else with `brief.py section <file> <heading>` or
   `brief.py items <file> FR-012 T031`. **For a bug** there is no milestone block: read the BUG
   record, then `brief.py items tasks.md <fix task IDs>` and the patched requirements by ID.
1. `speckit-implement` with `Milestone MK only: tasks T0xx–T0yy. Do not start any other task. Stop
   when these are done.` Afterwards confirm every task in the range is ticked and none outside it,
   then check your context (unit.md, *Hand over at 150k*). Its
   mandatory `tdd.run` hook drives red → green → refactor. For a bug, the first task is a regression
   test that fails on `origin/main` for the reported reason. If it asks "proceed anyway?", never
   answer "yes": close the checklist item as in Phase 3 (confirm, or fix the spec or plan), and
   escalate only when it needs a user decision.
2. **Gate, with review A in its shadow.** Start `mise run gate` detached, as in
   [../references/pr-and-merge.md](../references/pr-and-merge.md) §2 (if `cfg(target_os)` code
   changed, also `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`).
   While it builds, run review **A**: the `code-review` skill at `high` on `origin/main...HEAD` (a
   scoped round on the fix diff when A has run before). A builds nothing, so it does not wait on
   the build lock. Then wait for the gate with `hold.sh`.
   - **A found a real BLOCKER or MAJOR, or the gate is red:** fix, and repeat step 2. The gate
     must pass on the tree as it now is.
   - Otherwise go to step 3.
3. **Green gate: review B and the visual pass, together.** Dispatch **B** in the background: a
   fresh subagent checks the diff against the milestone's deliverable, its acceptance scenarios and
   the constitution (conformance rubric); a scoped round on the fix diff when B has run before. Its
   **Verify** reuses the gate's build. If anything visible changed since the last visual pass, run
   the `visual-pass` skill meanwhile. It is a forked Sonnet subagent that sees only its arguments:
   pass the worktree path, the quickstart section or change to check, and what counts as a pass.
   - **B or the visual pass found something real:** fix, and go back to step 2, then 3.
   - Otherwise go to step 4.

   Verify each finding against the code first; decline one that contradicts the spec, and record
   why in the ledger. Only rounds after a review's own BLOCKER or MAJOR count toward its limit
   ([../references/review-rubrics.md](../references/review-rubrics.md) *After it returns*). Red
   gates for one cause: after the third failed fix, escalate (category 5). A milestone on a cheaper
   model (**Tier** `light` or `docs`) instead writes *Handover* after the second failed fix and
   returns `FAILED`, so the orchestrator retries it on the session model.
4. Check the ticks from step 1, update the ledger, commit, push, and open the PR. Title
   `feat(NNN): <deliverable>`, or `fix(NNN): … (BUG-<k>)` for a bug.

**Continued with a red CI log:** run `systematic-debugging`, fix, re-run the gate, push, and return
`DONE` again. After the third failed attempt, escalate (category 5).
