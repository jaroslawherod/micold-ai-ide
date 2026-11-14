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

1. Issues named in the arguments, or `gh issue list --label autopilot:queue --state open --json number -q '.[].number'`; sort ascending.
2. `scripts/autopilot/supervisor.sh ledger init --owner <this session id> --cap <N> --source "<label autopilot:queue | issues ...>" <issues...>` (default cap 3). It refuses when a ledger exists: M2 covers a restart; for now stop and tell the person.

## Start an issue

For each issue `<n>` that `supervisor.sh next` prints, one at a time:

1. **Resource gate.** `scripts/autopilot/supervisor.sh gate --running <occupied + starts so far>`. `FAIL <check>`: start nothing more, report which check failed, retry at the next advance. `PASS`: go on.
2. `gh issue view <n> --json state,title -q .state`. A closed or missing issue, or one that already has a worktree or a ledger of its own: `supervisor.sh ledger set <n> State skipped` and `ledger set <n> Note "<reason>"`; next issue.
3. Write first: `ledger set <n> State starting`, `ledger set <n> Started <epoch seconds>`, `ledger set <n> Tries <tries + 1>`.
4. `create_worktree` with `github_issue: <n>`; `ledger set <n> Worktree <path>`.
5. `create_session` in that worktree with the prompt `/speckit-autopilot #<n> supervised`; `ledger set <n> Session <id>` and `ledger set <n> State running`.
6. A failed create: leave the state, write the reason with `ledger set <n> Note "<reason>"`, and go on. Counting tries and `failed` after 3 tries are enforced with the restart logic (M2, T014-T015).

## Advance loop

1. `supervisor.sh next` lists the issues to start. Nothing printed: say there is nothing to do.
2. Start each as above, gate first, one at a time.
3. Stop when no row occupies a slot (`starting` or `running`) and `next` prints nothing.

## Report

Finish with `supervisor.sh ledger table` and nothing longer.
