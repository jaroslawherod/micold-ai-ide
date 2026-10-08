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

## The orchestrator

After you dispatch a unit, run `scripts/autopilot/hold.sh --long "$SCRATCHPAD/unit-<description>"`
with `run_in_background` and `timeout: 3300000`. When it reports `HOLD` and the unit has not
returned, run it again. Stop on `STOP`, when the unit returns, and while you wait on the human.

- **A notification with nothing new** (a subagent you no longer wait on, a review you have already
  read) needs no reply and no tool call: end the turn in one line. Each wake re-reads the whole
  context, and re-writes it when the last call was over an hour ago.
- **Before a long wait on the human** (`AskUserQuestion`, a manual step): run
  `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py`. On `OVER`, make sure the ledger holds
  the open escalation and the next step, and add to the banner: `Cheaper: /clear, then
  /speckit-autopilot resume. The ledger is current.` An answer hours later otherwise re-writes your
  whole context to the cache.
