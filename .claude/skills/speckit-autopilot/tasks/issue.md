# Task: keep the run's GitHub issue (orchestrator)

When: at entry, once the flow and level are chosen; on a flow switch; at the handoff. **Every run
has an issue.** Its number is the run's ID: the feature flow's spec directory is
`specs/<issue>-<slug>`, a bugfix record is `BUG-<issue>`, and the bug and chore ledgers carry it
in their name.

- **Started from an issue** (`#<n>`, an issue URL):
  `scripts/autopilot/issue.sh start <n> <branch> <flow> [high|low]`. It labels the issue
  `in-progress`, `flow:<flow>` and `effort:<level>`, removes an older `flow:*` or `effort:*` that
  differs, assigns it and comments the branch. Pass a level only when one was named, labelled or
  answered. `ISSUE_TAKEN`: another flow's work; escalate as *blocked by work outside my flow*.
- **Started from text:** first `gh issue list --state open --search "<key words>" --json
  number,title,labels`. An open issue for this very work: read its labels, choose again if they
  change the flow, and `start` it. Else write the user's prompt verbatim to
  `$SCRATCHPAD/issue-body.md`, then `🤖 Opened by autopilot for this run.`, and run
  `scripts/autopilot/issue.sh new <branch> "<title>" "$SCRATCHPAD/issue-body.md" <flow> [high|low]`.
  It prints `ISSUE_NEW #<n>`, and labels the issue `bug` (bug, bugfix) or `enhancement` (feature)
  beside its flow. The title is the task in one line, in the user's words, with no `feat:` prefix.
- **Record** `#<n>` as the ledger's **Issue**: pass it to the first unit, with the labels read.
- **Switch:** `scripts/autopilot/issue.sh flow <n> <new-flow> [high|low]`
  ([../flows/switch.md](../flows/switch.md)).
- **Every PR of the run** ends its title with `(#<n>)` and its body with `Refs #<n>`, never
  `Closes #<n>`: the first merge would close it. The unit checks both ([pr.md](pr.md)).
- **Done:** `scripts/autopilot/issue.sh done <n> <pr>...` with every PR in the ledger, only after
  `handoff-check.sh` passes. A run that stops unfinished leaves the issue open and labelled.

Next: [dispatch.md](dispatch.md) at entry; the WORK COMPLETE message at the handoff.
