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

The script waits for the `ci complete` check, the only required one, and merges with
`gh pr merge <n> --rebase`: never `--delete-branch`, never `--admin`.

| Last line | Do |
|---|---|
| `MERGED <n> <sha>` | Run `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py`; on `OVER`, tell the user in one line that `/clear` then `/speckit-autopilot resume` would restart you small, and carry on. Dispatch the next unit with the PR and SHA; it records them in the ledger. Never edit the ledger yourself between units: `branch-start.sh` refuses a dirty tree. After the run's last PR: [handoff.md](handoff.md). |
| `RED <n> <run> <log>` | In this flow's code: continue the unit that opened the PR ([dispatch.md](dispatch.md), *Continuing a unit*) with the log path and `tasks/red-ci.md` to read, at most 3 attempts. Outside it: [ci.md](ci.md). |
| `CHECKLESS <n> <reason>` | [ci.md](ci.md), *A PR with no checks*, then run the script again. |
| `MERGE-FAILED <n> <message>` | [ci.md](ci.md), *Merge problems*, then run the script again. |
| `CLOSED <n>` | Someone closed the PR. Escalate (category 1); never reopen it unasked. |
| `TIMEOUT <n> <what>` or no result line | Check `gh auth status` and the PR by hand, then run the script again. A timeout means 6 h passed: CI is stuck or queued, so say so in one line and re-arm once; do not re-arm a third time without asking. |
