---
name: autopilot-supervisor
description: Use when the user hands over several GitHub issues, or the label `autopilot:queue`, to run as autopilot sessions in parallel under a cap — "supervise these issues", "start the queue", "run #12 #14 #15 with the supervisor". Starts one supervised `/speckit-autopilot` session per issue, only while the machine has room.
argument-hint: "#<n> #<n> ... | autopilot:queue"
user-invocable: true
---

# Autopilot supervisor

Run at the project root. Decisions come from `scripts/autopilot/supervisor.sh` (pure, reads the
ledger `.autopilot/supervisor.md`); you do the effects: `gh`, `create_worktree`, `create_session`.
Keep your context small: read the ledger only through `supervisor.sh ledger table`, and never read a session's transcript.

## Build the queue

1. Issues named in the arguments, or `gh issue list --label autopilot:queue --state open --limit 1000 --json number -q '.[].number'`; sort ascending.
2. `scripts/autopilot/supervisor.sh ledger init --owner <this session id> --cap <N> --source "<label autopilot:queue | issues ...>" <issues...>` (default cap 3), then run *Touches* for every issue. It refuses when a ledger exists: that is a restart, see *Restart*.

## Touches

Before the first `next`, and for each issue whose `Touches` cell is empty (FR-004):

1. `gh issue view <n> --json body,labels -q '.body' > <scratch>/body-<n>.md`, and the label names with `-q '[.labels[].name] | join(" ")'`.
2. `scripts/autopilot/supervisor.sh touches <scratch>/body-<n>.md --labels "<names>"` prints the paths (declared `Touches:` line first, else inferred from `flow:*` labels and `specs/<dir>` paths) and `declared`, `inferred` or `none` on stderr. Store them: `ledger set <n> Touches <paths>`.
3. `none` (prints `-`): `ledger set <n> Note "no touched files declared or inferable: treated as disjoint"`.

`supervisor.sh next` then holds a queued issue whose paths overlap a `starting` or `running` row or an earlier queued issue, whatever the cap; `supervisor.sh overlap <a> <b>` answers `yes` or `no` for two lists. Issues with the same `flow:*` label and no `Touches:` line or `specs/` path share that flow file, so they run one at a time: declare `Touches:` to avoid it. A held issue stays `queued` and is released once the earlier one is `done`, `failed`, `lost` or `skipped`.

## Ownership

A run is yours by its worktree's `parent`, never by its name or by what you remember. Worktree parents: `specs/1857-worktree-parent`.

- **Create.** Leave `parent` out of the worktree call: the daemon records the creating worktree as the parent (`default` for a supervisor at the project root). Never pass another worktree's name.
- **Record.** `ledger set <n> Parent <your worktree ref>` (`default` at the root, else your own worktree's directory name) beside `Worktree`, so a restart knows which runs are yours.
- **List.** `list_worktrees` with `parent: <your ref>`, saved to `<scratch>/worktrees.json`; `scripts/autopilot/runs-of-parent.sh <your ref> < <scratch>/worktrees.json` prints your runs. A run's `Worktree` cell that is not in that list is not yours: do not map, start, stop, send to or interrupt it.
- **Check before every act** on a run (`send_session_input`, `interrupt_session`, `stop_session`, `delete_worktree`): `runs-of-parent.sh <your ref> --check <worktree> --rows <scratch>/worktrees.json`. Exit 1 (another parent, no parent, or a removed parent): refuse, and say so in the ledger Note. Never act on a run of another parent or on one with none.
- **Removed parent.** A run whose `parent_state` is `removed` is an orphan: `runs-of-parent.sh <your ref> --orphans` lists them. Report each to the person and never adopt it, even if it looks abandoned.
- **Where the daemon helps.** A supervisor in its own worktree is limited by the daemon to its parent, siblings and children, and so is every worker it starts. A supervisor in `default` is not limited by the daemon, so this rule still applies to it in full: it sees every run and acts only on its own.

### Relay

A worker contacts only its parent (you) and its siblings: parent and siblings. Its questions and any message for another scope come to you, and you forward them up or down with your own `send_session_input` (FR-017). The supervisor never expects a worker to reach another supervisor's run,. Put this in the worker prompt: after `/speckit-autopilot #<n> supervised`, add the line `You can reach only your supervisor and your sibling runs. Send a question or a message for anyone else to your supervisor; it forwards it.`

## Start an issue

For each issue `<n>` that `supervisor.sh next` prints, one at a time:

1. **Resource gate.** `scripts/autopilot/supervisor.sh gate` (it counts the `starting` and `running` rows of the ledger, so every start is written to the ledger before the next gate). `FAIL <check>`: start nothing more, report which check failed, retry at the next advance. `PASS`: go on.
2. `gh issue view <n> --json state,title -q .state`. A closed or missing issue, or one that already has a worktree or a ledger of its own: `supervisor.sh ledger set <n> State skipped` and `ledger set <n> Note "<reason>"`; next issue.
3. Write first: `ledger set <n> State starting`, `ledger set <n> Started <epoch seconds>`, `ledger set <n> Tries <tries + 1>`.
4. `create_worktree` with `github_issue: <n>` and no `parent` (*Ownership*); `ledger set <n> Worktree <path>` and `ledger set <n> Parent <your ref>`.
5. `create_session` in that worktree with the prompt `/speckit-autopilot #<n> supervised` and the relay line of *Ownership*; `ledger set <n> Session <id>` and `ledger set <n> State running`.
6. A failed create: leave the state, write the reason with `ledger set <n> Note "<reason>"`, and go on. Then `supervisor.sh retry <n>` (the `Tries` cell counts the start; it prints `queued`, or `failed` at 3 tries).

## Advance loop

1. `supervisor.sh next` lists the issues to start. Nothing printed: say there is nothing to do.
2. Start each as above, gate first, one at a time.
3. `next` prints nothing: run *Report* (it ends with `supervisor.sh ledger table`) and end the turn. Never poll `next`: start the watcher (*Watcher*) and end the turn.

## Restart

A ledger already exists: continue from it, never rebuild the queue.

1. **Owner.** `list_sessions` (metadata only), then `supervisor.sh check-owner --live-sessions "<ids>" --me <this session id>`. `OWN`: go on. `FREE`: `supervisor.sh ledger owner <this session id>`, go on. `HELD <owner>` (exit 1): another supervisor owns the ledger; say which session and stop.
2. **Recover half-started rows first.** A `starting` row with an empty Worktree or Session cell (the supervisor ended mid-start): look for a worktree and a session of the issue (`list_sessions`, `git worktree list`). Found: fill the empty cells, State `running`. None: `supervisor.sh retry <n>` (back to `queued`, or `failed` after 3 tries).
3. **Map each row that has both Worktree and Session** and is `starting` or `running`. For `<n>` with Worktree `<dir>`: `scripts/autopilot/run-status.sh --json <dir> > <scratch>/<n>.json` (one file per issue, kept for *Report*), then `supervisor.sh map <scratch>/<n>.json --started <Started> --ledger .autopilot/supervisor.md` and `ledger set <n> State <result>`. Exit 2 (worktree gone) or 3 (several ledgers): `supervisor.sh map --exit <code> --started <Started>` gives `lost`; put the exit code in Note. `done` frees the slot; `running` keeps it and is never started twice.
4. **Liveness.** A `running` row with a Session id that is not in the `list_sessions` ids while its state is not `done`: `ledger set <n> State lost`, and report it. Rows with an empty Session were handled in step 2. Never restart a `lost` issue yourself.
5. **Re-ask.** `supervisor.sh route next --reask` prints every `asked` row. For each issue in it, use that issue's `<scratch>/<n>.json` from step 3 (never a transcript): a session that left `asking` unanswered gets `supervisor.sh route drop <issue>` and its question is not asked. Then run `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py`: `OVER`: make no `AskUserQuestion` call, print `Cheaper: /clear, then /autopilot-supervisor — it asks this again from the ledger.` and end the turn. Otherwise ask the remaining rows, without a banner or notification, in `AskUserQuestion` calls of at most 4 questions in ascending issue order, each prefixed `#<issue>`, the next call after each answer, until every printed row is answered; then *Route questions* steps 8 to 11. Do this before any new start.
6. Then the advance loop.

## Report

For each run the mapping left `running` whose run-status JSON state is `stuck` or `blocked`, and for each `lost` row (a `dead-no-ledger` run past Grace is reported through its `lost` row), tell the person the issue, the state and why, from that issue's `<scratch>/<n>.json` (the file Restart step 3 wrote; never a second `run-status` read), its JSON `escalation` (its `banner` and `questions` for `blocked`), or the reason, without reading the session. A `dead-no-ledger` run within Grace counts as `running`: no alarm. An `asking` run is not only reported: run *Route questions* for it. Never unblock or restart a `stuck`, `blocked` or `lost` run yourself; the person decides. Finish with `supervisor.sh ledger table`.

## Route questions

For every `running` row whose `<scratch>/<n>.json` state is `asking` (never a transcript), in this order:

1. **Collect.** From that file's JSON `escalation` (`category`, `banner`, `questions`): `scripts/autopilot/supervisor.sh route add <n> <session id> <scratch>/<n>.json`. It writes one `R<n>.Q<k>` row per question as `pending` and is a no-op for rows already there, so a restart never asks twice.
2. **Resolve.** For each `pending` row, judge its subject, not the category label, against `.claude/skills/speckit-autopilot/rules/escalation.md` *Never ask about these*. `supervisor.sh route gate "<category>"` prints `user-only` (category 2, 3 or 4: never answer it, whatever the evidence; go on to step 3) or `judge` (category 1, 5 or 6). On `judge`, a subject that matches a *Never ask* item overrides the label: look for the answer in the repo (`rules/escalation.md`, the flow's spec, plan and ledger, the failing command's own output) and, with evidence, `supervisor.sh route answer <key> supervisor "<answer> (evidence: <path or command>)"`; the row becomes `resolved` and stays out of the user batch. A subject that matches no *Never ask* item, or one with no evidence, is never guessed: the row stays `pending` and goes to the user batch. A `route gate` that exits 2 (a missing or unreadable category) goes to the user too.
3. **Batch.** `supervisor.sh route next` prints up to 4 `pending` questions, ascending issue then number. Nothing printed: run steps 9 to 11 for each issue whose `route table` rows are all `resolved` or `answered` with an empty Sent cell (a run the supervisor resolved entirely, or a restart), then stop.
4. **Banner.** For each question send the banner of `.claude/skills/speckit-autopilot/tasks/ask.md` with unit `supervisor`: first line prefixed `#<issue>`, a `(Recommended)` first option with its evidence, written so the person can answer it cold.
5. **Notify.** One `PushNotification` (load it with `ToolSearch("select:PushNotification")`), text `🛑 <issues> need a decision: <topic>`.
6. **Mark.** `supervisor.sh route state <key> asked` for each row of the batch, after the banners and the notification and before the question call (a restart never asks twice).
7. **Ask.** Run `AUTOPILOT_CONTEXT_CAP=80000 scripts/autopilot/context.py`. `OVER`: the batch is already in the ledger (`asked` rows, step 6), so make no `AskUserQuestion` call: after the banners, the notification and `route state <key> asked`, print `Cheaper: /clear, then /autopilot-supervisor — it asks this again from the ledger.` and end the turn (*Restart* re-asks it). Otherwise one `AskUserQuestion` of at most 4 questions in ascending issue order, each prefixed `#<issue>`. More than 4 questions: the rest go in the next batch after the first answer (back to step 3). A single question goes alone, at once.
8. **Record.** `supervisor.sh route answer <key> user "<answer>"` per answer.
9. **Send.** `supervisor.sh route compose <issue>`. Exit 0: its first line is `session <id>`; `send_session_input` the remaining lines as one message (they already begin with `resume`, or `resume --by supervisor` when the supervisor resolved every question) only to that session, never to another. Exit 1 prints `missing: <n> ...`: ask the person again for only the missing question, then compose again.
10. **Mark sent.** `supervisor.sh route sent <issue> <epoch seconds>`.
11. **Confirm.** After a `CHANGED` wake shows the run left `asking`, `supervisor.sh route drop <issue>` (the worker's *Decisions* holds the record). A run still `asking` after the send: `supervisor.sh route overdue --now <epoch seconds> --ceiling <Watcher ceiling seconds>` lists the issues whose Sent is older than the Watcher ceiling; report each whose Note does not already say it as not accepting input, then `supervisor.sh ledger set <n> Note "not accepting input"` so the next wake does not report it again; never re-sent.

A send that fails with `not_found` (the session is gone): `route drop <issue>` and report the session lost. Any other failure: `supervisor.sh ledger set <n> Note "<error>"`, leave the rows `answered`, report the issue, and never retry in a loop.

## Watcher

After an advance that leaves issues `starting` or `running`, start `scripts/autopilot/supervisor-watch.sh` with `run_in_background` and end the turn. It exits on the first state change (`CHANGED <issue> <old> <new>`) or after 10 minutes (`CEILING`; `AUTOPILOT_WATCH_CEILING` sets it), and its exit wakes you: run *Restart* steps 2 to 4, the advance loop and *Report* again. Never poll with `sleep`.
