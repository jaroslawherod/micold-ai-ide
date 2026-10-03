# Task: gate and review a milestone's diff (milestone unit)

When: [implement.md](implement.md) is done and every task in the range is ticked.

| Flow | Review A (`code-review` skill) | Review B | Visual pass |
|---|---|---|---|
| feature | `high` | yes, once | if anything visible changed |
| bugfix | `medium` | no | if anything visible changed |

1. **Scoped gate, with review A in its shadow.** Start the scoped gate detached ([gate.md](gate.md)).
   While it builds, run review **A** on `origin/main...HEAD` at your flow's level (a scoped round
   on the fix diff when A has run before). A builds nothing, so it does not wait on the build
   lock. Then wait for the gate ([../rules/waiting.md](../rules/waiting.md)).
   - **A found a real BLOCKER or MAJOR, or the gate is red:** fix, and repeat step 1.
2. **Review B and the visual pass, together.** B: dispatch it in the background per
   [review.md](review.md) with [../rubrics/conformance.md](../rubrics/conformance.md), once per
   milestone. Run it again only after fixing a BLOCKER or MAJOR of B's own; fixes for A or the
   visual pass do not re-run it. Visual pass: the `visual-pass` skill through an
   `autopilot-worker`, with the worktree path, the quickstart section or change to check, and
   what counts as a pass.
   - **B or the visual pass found something real:** fix, run the scoped gate, and go on.
3. **Full gate, once**, on the final tree ([gate.md](gate.md)). Red: fix, and run it again.
4. Update the ledger, commit, push and open the PR: [pr.md](pr.md). When this PR is the run's last
   (a bugfix), finish the ledger in it: **Phase** `done`, **Next step** `handoff`.

Findings and rounds: [review-rounds.md](review-rounds.md). Red gates for one cause: after the third
failed fix, escalate (category 5). A unit on a cheaper model (**Tier** `light` or `docs`) instead
writes *Handover* after the second failed fix and returns `FAILED`, so the orchestrator retries it
on the session model.

Hands on: return `DONE` with the PR number. Continued later with a red CI log:
[red-ci.md](red-ci.md).
