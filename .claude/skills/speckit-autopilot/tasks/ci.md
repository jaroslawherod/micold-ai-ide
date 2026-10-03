# Task: unblock a PR that CI or the merge refused (orchestrator)

When: `wait-merge.sh` printed `RED` outside this flow's code, `CHECKLESS` or `MERGE-FAILED`.

## Red outside this flow's code

To tell whose code is red, never read a long log yourself: a `"haiku"` helper subagent reads the
CI log, gate log or report and returns only its failures.

Rerun once: `gh run rerun <run-id> --failed`. Passes: a flake, carry on. Fails again: check `main`
(`gh run list --branch main --status completed --limit 3`). Either way, do not fix it. Escalate as
*blocked by work outside my flow*, with the evidence.

## A PR with no checks

`ci complete` appears only once every other job finishes. Right after a push, `gh pr checks
--watch` prints `no required checks reported` and exits 0: that is not green. No check at all
after about 5 minutes means the PR is checkless. Diagnose in this order:

1. `gh pr view <n> --json state,mergeable`
   - `state` is `MERGED`: done.
   - `mergeable` is `CONFLICTING`: no workflow can fire. Have the unit that opened the PR run
     `git fetch origin && git rebase origin/main`, resolve conflicts, re-run the gate, and
     `git push --force-with-lease`.
2. `gh run list --branch <branch> --limit 5`: runs with conclusion `action_required` await
   approval. Approve **only runs on this flow's PR**:
   `gh api -X POST repos/{owner}/{repo}/actions/runs/<id>/approve`.
3. Otherwise an Actions outage dropped the event: `gh pr close <n> && gh pr reopen <n>`.

## Merge problems

Trust `gh pr view <n> --json state -q .state`, not the exit status of `gh pr merge`.

| Problem | Fix |
|---|---|
| `This branch can't be rebased` while the PR reads `MERGEABLE` | Commits already on `main`. Run `git rebase origin/main` (git skips them), `git push --force-with-lease`, wait for green. |
| Branch has a merge commit from `main` | Squash the green head: `gh api -X PUT repos/{owner}/{repo}/pulls/<n>/merge -f merge_method=squash -f sha=<head>`. |
| "base branch policy prohibits the merge" | Usually a missing `workflow` token scope on a PR touching `.github/workflows`. Escalate (category 4) with the user's command: `gh auth refresh -s workflow`. |

Next: run `wait-merge.sh` again ([merge.md](merge.md)).
