---
name: autopilot-supervisor
description: Use when the user hands over several GitHub issues, or the label `autopilot:queue`, to run as autopilot sessions in parallel under a cap — "supervise these issues", "start the queue", "run #12 #14 #15 with the supervisor". Starts one supervised `/speckit-autopilot` session per issue, only while the machine has room.
argument-hint: "#<n> #<n> ... | autopilot:queue"
user-invocable: true
---

# Autopilot supervisor

Run at the project root. Decisions come from `scripts/autopilot/supervisor.sh` (pure, reads the
ledger `.autopilot/supervisor.md`); you do the effects: `gh`, `create_worktree`, `create_session`.
Keep your context small: read the ledger only through `supervisor.sh ledger table`, and never read
a session's transcript.

## Build the queue

1. Issues named in the arguments, or `gh issue list --label autopilot:queue --state open --limit 1000 --json number -q '.[].number'`; sort ascending.
2. `scripts/autopilot/supervisor.sh ledger init --owner <this session id> --cap <N> --source "<label autopilot:queue | issues ...>" <issues...>` (default cap 3). It refuses when a ledger exists: that is a restart, see *Restart*.

## Start an issue

For each issue `<n>` that `supervisor.sh next` prints, one at a time:

1. **Resource gate.** `scripts/autopilot/supervisor.sh gate` (it counts the `starting` and `running` rows of the ledger, so every start is written to the ledger before the next gate). `FAIL <check>`: start nothing more, report which check failed, retry at the next advance. `PASS`: go on.
2. `gh issue view <n> --json state,title -q .state`. A closed or missing issue, or one that already has a worktree or a ledger of its own: `supervisor.sh ledger set <n> State skipped` and `ledger set <n> Note "<reason>"`; next issue.
3. Write first: `ledger set <n> State starting`, `ledger set <n> Started <epoch seconds>`, `ledger set <n> Tries <tries + 1>`.
4. `create_worktree` with `github_issue: <n>`; `ledger set <n> Worktree <path>`.
5. `create_session` in that worktree with the prompt `/speckit-autopilot #<n> supervised`; `ledger set <n> Session <id>` and `ledger set <n> State running`.
6. A failed create: leave the state, write the reason with `ledger set <n> Note "<reason>"`, and go on. Then `supervisor.sh retry <n>` (the `Tries` cell counts the start; it prints `queued`, or `failed` at 3 tries).

## Advance loop

1. `supervisor.sh next` lists the issues to start. Nothing printed: say there is nothing to do.
2. Start each as above, gate first, one at a time.
3. `next` prints nothing: report with `ledger table` and end the turn. Never poll `next`: start the watcher (*Watcher*) and end the turn.

## Restart

A ledger already exists: continue from it, never rebuild the queue.

1. **Owner.** `list_sessions` (metadata only), then `supervisor.sh check-owner --live-sessions "<ids>" --me <this session id>`. `OWN`: go on. `FREE`: `supervisor.sh ledger owner <this session id>`, go on. `HELD <owner>` (exit 1): another supervisor owns the ledger; say which session and stop.
2. **Recover half-started rows first.** A `starting` row with an empty Worktree or Session cell (the supervisor ended mid-start): look for a worktree and a session of the issue (`list_sessions`, `git worktree list`). Found: fill the cells, State `running`. None: `supervisor.sh retry <n>` (back to `queued`, or `failed` after 3 tries).
3. **Map each row that has both Worktree and Session** and is `starting` or `running`. For `<n>` with Worktree `<dir>`: `scripts/autopilot/run-status.sh --json <dir> > <scratch>/j.json`, then `supervisor.sh map <scratch>/j.json --started <Started> --ledger .autopilot/supervisor.md` and `ledger set <n> State <result>`. Exit 2 (worktree gone) or 3 (several ledgers): `supervisor.sh map --exit <code> --started <Started>` gives `lost`; put the exit code in Note. `done` frees the slot; `running` keeps it and is never started twice.
4. **Liveness.** A `running` row with a Session id that is not in the `list_sessions` ids while its state is not `done`: `ledger set <n> State lost`, and report it. Rows with an empty Session were handled in step 2. Never restart a `lost` issue yourself.
5. Then the advance loop.

## Report

For each run the mapping left `running` whose run-status JSON state is `asking`, `stuck` or `blocked`, and for each `lost` row (a `dead-no-ledger` run past Grace is reported through its `lost` row), tell the person the issue, the state and why, from the JSON `escalation` (its `banner` and `questions` for `asking` and `blocked`) or the reason. A `dead-no-ledger` run within Grace counts as `running`: no alarm. Never answer a question, unblock or restart such a run yourself; the person decides. Finish with `supervisor.sh ledger table`.

## Watcher

After an advance that leaves issues `starting` or `running`, start `scripts/autopilot/supervisor-watch.sh` with `run_in_background` and end the turn. It exits on the first state change (`CHANGED <issue> <old> <new>`) or after 10 minutes (`CEILING`; `AUTOPILOT_WATCH_CEILING` sets it), and its exit wakes you: run *Restart* steps 2 to 4 and the advance loop again. Never poll with `sleep`.
