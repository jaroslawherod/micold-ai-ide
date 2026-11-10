# Task: wait for CI and merge (orchestrator)

When: a unit returned `DONE` with a PR. Every PR merges on green before the next unit starts.

Run `scripts/autopilot/wait-merge.sh <n>` detached, and wait on its last line (it runs as long as
CI does):

```bash
log="$SCRATCHPAD/pr-<n>.log"
# The wrapper writes its own pid: `setsid` may fork, so `$!` is not the process to watch.
AUTOPILOT_LOG_DIR="$SCRATCHPAD" setsid nohup \
  bash -c 'echo $$ >"$0.pid"; scripts/autopilot/wait-merge.sh <n>; echo "WAIT_EXIT=$?"' "$log" \
  >"$log" 2>&1 &
# then, with run_in_background (stops too if the script was killed without a result):
until grep -q '^WAIT_EXIT=' "$log" || ! kill -0 "$(cat "$log.pid")" 2>/dev/null; do sleep 30; done
tail -6 "$log"
```

The script waits for the `ci complete` check, the only required one, runs the verdict guard, and
merges with `gh pr merge <n> --rebase --match-head-commit <head>` (`wait-merge.sh <n> --squash`
squash-merges instead): never `--delete-branch`, never `--admin`. A direct `gh pr merge` is
denied by `gate-hook.sh`; `wait-merge.sh` is the only merge path.

## A verdict belongs to one head

A review verdict counts for the commit it covered. Every round is recorded with
`scripts/autopilot/record-verdict.sh <ledger> <review> <round> <scope> <snapshot> <head> <verdict>`:
`scope` is a PR number, a milestone ID or `design`, `head` is `git rev-parse HEAD` with the review's
fixes committed (before the ledger commit), `verdict` starts with `CLEAN`, `MINOR-ONLY`, `CHANGES`
or `pending`. The *Review rounds* table has the columns `Review | Round | Scope | Head | Snapshot | Verdict`.

Before merging, `scripts/autopilot/verdict-guard.sh <n>` (run by `wait-merge.sh`) reads the PR's
head fresh and finds the ledger of the PR's head branch. It allows the merge only when the latest
round of every review whose scope names the PR is `CLEAN` or `MINOR-ONLY` for that head, or for a
head that differs only in ledger files (`specs/*/autopilot*.md`, bugs and quick ledgers). A PR whose
branch no ledger names is not guarded (`UNGUARDED`). Anything else is refused: a head changed since
the review, no row, `CHANGES` or `pending`, an unreadable PR head or ledger, or a ledger with no
`Head` column (a run started before this rule re-reviews once). `gh` failing is a refusal.

### Carrying a verdict over a clean replay

A rebase or an amended message leaves the code unchanged, so an **approved** verdict carries from
`H` to the new head `P` when, ledger files excluded, neither `merge-base(X,base)..X` range has a
merge commit or is empty, the ordered `git patch-id --stable` lists are equal and the
`git diff --name-status -M` path sets are equal. Anything else, a conflict resolution included,
does not carry; `CHANGES` and `pending` never do. The guard posts the PR comment
`verdict carried <H> -> <P>` once per pair and prints `CARRIED <n> <H> <P>` (the last carried `H`);
the branch is untouched. `H` is the original SHA in the ledger, so chains do not drift.

| Last line | Do |
|---|---|
| `MERGED <n> <sha>` | Run `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py`; on `OVER`, tell the user in one line that `/clear` then `/speckit-autopilot resume` would restart you small, and carry on. Dispatch the next unit with the PR and SHA; it records them in the ledger. Never edit the ledger yourself between units: `branch-start.sh` refuses a dirty tree. After the run's last PR: [handoff.md](handoff.md). |
| `CARRIED <n> <H> <P>` | The merge proceeds on its own (`wait-merge.sh` merges next); nothing to do. |
| `RED <n> <run> <log>` | In this flow's code: continue the unit that opened the PR ([dispatch.md](dispatch.md), *Continuing a unit*) with the log path and `tasks/red-ci.md` to read, at most 3 attempts. Outside it: [ci.md](ci.md). |
| `CHECKLESS <n> <reason>` | [ci.md](ci.md), *A PR with no checks*, then run the script again. |
| `VERDICT-REFUSED <n> reviewed <H\|none> current <P> re-review: <why>` | Do not merge. Continue the unit that opened the PR ([dispatch.md](dispatch.md), *Continuing a unit*): review the current head (the fix diff since `H`), fix, record the new round with `record-verdict.sh`, push, and run the script again. |
| `MERGE-FAILED <n> <message>` | [ci.md](ci.md), *Merge problems*, then run the script again. |
| `CLOSED <n>` | Someone closed the PR. Escalate (category 1); never reopen it unasked. |
| `TIMEOUT <n> <what>` or no result line | Check `gh auth status` and the PR by hand, then run the script again. A timeout means 6 h passed: CI is stuck or queued, so say so in one line and re-arm once; do not re-arm a third time without asking. |
