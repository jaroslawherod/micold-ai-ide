# Task: unblock a PR that CI or the merge refused (orchestrator)

When: `wait-merge.sh` printed `RED` outside this flow's code, `CHECKLESS` or `MERGE-FAILED`.

## Red outside this flow's code

To tell whose code is red, never read a long log yourself: a `"haiku"` helper subagent reads the
CI log, gate log or report and returns only its failures.

Every state has a dwell limit: `scripts/autopilot/pr-state.sh <pr>` says whether the PR is past
it and the action ([waiting.md](../rules/waiting.md), *PR states and dwell limits*). Run it when
`wait-merge.sh` returns `HOLD`/`TIMEOUT`, and before any rerun, close or reopen below.

A tooling-only red is not rerun or escalated: see *Tooling-only diffs* below.

Rerun once: `gh run rerun <run-id> --failed`. Passes: a flake, carry on. Fails again: check `main`
(`gh run list --branch main --status completed --limit 3`). Either way, do not fix it. Escalate as
*blocked by work outside my flow*, with the evidence.

## Tooling-only diffs: a green local gate replaces green CI

When the PR's diff touches only autopilot tooling (`.claude/skills/**`, `.claude/agents/**`,
`scripts/autopilot/**`, the autopilot suites `scripts/tests/autopilot*`, `run-status*`,
`lib/autopilot*`, `docs/**`, `specs/**`), the merge condition is a green local gate for the head
(`mise run gate`, recorded by the gate), not a green `ci complete`. `scripts/autopilot/diff-class.sh
<n>` classifies the diff (`TOOLING` or `PRODUCT`; an unreadable or 100+ file list is `PRODUCT`).
`wait-merge.sh` applies the rule: on a **red** `ci complete` it merges with `--admin` only when the diff
is `TOOLING` and the head's tree is in the local gate stamp (`autopilot-gate-ok`). It prints
`SKIPPED-CI <n> <head> <checks>` before `MERGED`, writes `pr-<n>-skipped-checks.txt` and comments. **Record every skipped check** in the ledger (*Decisions*) and, for an epic, in
its checklist. No recorded gate for the head: run the gate first. Any change under `crates/`, `.github/`
or other product code keeps the full CI requirement (`RED`, above). Branch protection is never bypassed
any other way.

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
| Branch has a merge commit from `main` | Squash the green head: `scripts/autopilot/wait-merge.sh <n> --squash` (it runs the verdict guard and pins the merge to the head it checked). |
| "base branch policy prohibits the merge" | Usually a missing `workflow` token scope on a PR touching `.github/workflows`. Escalate (category 4) with the user's command: `gh auth refresh -s workflow`. |

Next: run `wait-merge.sh` again ([merge.md](merge.md)).
