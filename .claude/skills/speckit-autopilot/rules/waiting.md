# Waiting without losing the prompt cache

A subagent's prompt cache expires after 5 idle minutes; the next call then re-writes the whole
context, which costs as much as 12 calls. The orchestrator's lasts 60 minutes (20 calls).

## A unit

- A foreground `Bash` call may wait 250 s at most (`timeout` ≤ 250000); a hook blocks a longer
  one. Anything that can run longer (a gate, a test suite, a build): start it detached
  (`setsid nohup … >"$log" 2>&1 &`) and hold.
- While a detached gate or a background subagent runs, do not end your turn. Call
  `scripts/autopilot/hold.sh <log> [<regex>]` with Bash `timeout: 300000`, and again on each
  `HOLD`. It prints `DONE <line>` when the log matches (default: a line starting `<NAME>_EXIT=`).
- To hold for a subagent, name a file nothing writes; its result reaches you when a hold returns.
- On `STOP`, wait once in the background instead, and do not call `hold.sh` again.
- A forked skill that runs over 5 minutes (`visual-pass`, `speckit-tdd-verify`) blocks your turn:
  have an `autopilot-worker` run it and return its result, and hold.
- Run work that needs no build (a review, a subagent) while the gate builds, and wait for what
  runs together in one wait, not one after another.
- A job waiting on the build lock idles too: do not start it until the build is done.
- Never end your turn with a subagent or a gate still running, and plan a wait over 4 minutes as a
  `hold.sh` call before you start it: a unit that ended its turn waiting 21 minutes on a handback
  re-wrote its whole context.

## PR states and dwell limits

A PR sits in one state at a time. `scripts/autopilot/pr-state.sh <pr>` prints
`PR <n> STATE <state> DWELL <s> LIMIT <s> OK|STUCK ACTION <action>` and exits 1 when stuck. Dwell
counts from the later of the head commit and the latest finished check (an upper bound). Override a
limit with `AUTOPILOT_LIMIT_<STATE>` (`-` as `_`), in seconds.

| State | Max dwell | Owner | Action once past it |
|---|---|---|---|
| `DRAFT` | 30 min | unit that opened it | `gh pr ready`; still draft: escalate *blocked* |
| `CHECKLESS` | 5 min | orchestrator | [ci.md](../tasks/ci.md), *A PR with no checks* |
| `CI-RUNNING` | 60 min | orchestrator | `gh run cancel <run>` then `gh run rerun <run>` once; stuck again: escalate *blocked* |
| `CI-RED` | 30 min | unit that opened it | [red-ci.md](../tasks/red-ci.md); outside this flow's code: [ci.md](../tasks/ci.md) |
| `CONFLICTING` | 15 min | unit that opened it | `git fetch origin && git rebase origin/main`, gate, `git push --force-with-lease` |
| `ARMED` (auto-merge on, not merged) | 30 min | orchestrator | `wait-merge.sh`; still open: disarm and escalate *blocked* |
| `GREEN` (not merged) | 10 min | orchestrator | `wait-merge.sh` |

`MERGED` and `CLOSED` are terminal. A failed check wins over a running one (`CI-RED`), and mergeable `UNKNOWN` counts as not conflicting. A state's second action that fails is escalated as *blocked*
(category 4, with the `pr-state.sh` line as evidence); never loop on one action. Waiting for any
state stays a background `until` loop, `hold.sh` or Monitor, never polling; run `pr-state.sh` when
a hold returns `HOLD`, not on a timer.

## The orchestrator

After you dispatch a unit, run `scripts/autopilot/hold.sh --long "$SCRATCHPAD/unit-<description>"`
with `run_in_background` and `timeout: 3300000`. When it reports `HOLD` and the unit has not
returned, run it again. Stop on `STOP`, when the unit returns, and while you wait on the human.

- **A notification with nothing new** (a subagent you no longer wait on, a review you have already
  read) needs no reply and no tool call: end the turn in one line. Each wake re-reads the whole
  context, and re-writes it when the last call was over an hour ago.
- **Before a long wait on the human** (`AskUserQuestion`, a manual step): [../tasks/ask.md](../tasks/ask.md),
  *Restart small*. Over 80k, the question goes out as a banner and a notification and the turn
  ends, so the user can `/clear` and `/speckit-autopilot resume`; an answer hours later otherwise
  re-writes your whole context to the cache.
