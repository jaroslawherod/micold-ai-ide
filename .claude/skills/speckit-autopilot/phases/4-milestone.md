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
2. **Scoped gate, with review A in its shadow.** Start `scripts/autopilot/scoped-gate.sh` detached,
   as the gate in [../references/pr-and-merge.md](../references/pr-and-merge.md) §2: fmt, then
   clippy and tests of the crates this branch changed. While it builds, run review **A**: the
   `code-review` skill at `high` on `origin/main...HEAD` (a scoped round on the fix diff when A has
   run before). A builds nothing, so it does not wait on the build lock. Then wait for the gate
   with `hold.sh`.
   - **A found a real BLOCKER or MAJOR, or the gate is red:** fix, and repeat step 2.
   - Otherwise go to step 3.
3. **Review B and the visual pass, together.** Dispatch **B** once per milestone, in the
   background, on `"sonnet"`: a fresh subagent checks the diff against the milestone's deliverable,
   its acceptance scenarios and the constitution (conformance rubric). Run it again only after
   fixing a BLOCKER or MAJOR of B's own, as a scoped round on the fix diff; fixes for A or the
   visual pass do not re-run B. If anything visible changed since the last visual pass, run the
   `visual-pass` skill meanwhile, through an `autopilot-worker` (unit.md, *Wait with `hold.sh`*):
   pass the worktree path, the quickstart section or change to check, and what counts as a pass.
   - **B or the visual pass found something real:** fix, run the scoped gate, then go on with
     step 3 (B again only for its own findings).
   - Otherwise go to step 4.

   Verify each finding against the code first; decline one that contradicts the spec, and record
   why in the ledger. Only rounds after a review's own BLOCKER or MAJOR count toward its limit
   ([../references/review-rubrics.md](../references/review-rubrics.md) *After it returns*). Red
   gates for one cause: after the third failed fix, escalate (category 5). A milestone on a cheaper
   model (**Tier** `light` or `docs`) instead writes *Handover* after the second failed fix and
   returns `FAILED`, so the orchestrator retries it on the session model.
4. **Full gate, once.** `mise run gate` detached on the final tree (if `cfg(target_os)` code
   changed, also `scripts/build-lock.sh cargo check --workspace --target aarch64-apple-darwin`).
   Only this gate lets `git push` through. Red: fix, and run it again.
5. Check the ticks from step 1, update the ledger, commit, push, and open the PR. Title
   `feat(NNN): <deliverable>`, or `fix(NNN): … (BUG-<k>)` for a bug. A bug's PR is the run's last:
   finish the ledger in it, as [5-close.md](5-close.md) step 5 does.

**Continued with a red CI log:** run `systematic-debugging`, fix, re-run the gate, push, and return
`DONE` again. After the third failed attempt, escalate (category 5).
