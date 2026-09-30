# GitHub issue

Read when the run starts from an issue, and again at the handoff.

- **Start**: the first unit runs `scripts/autopilot/issue.sh start <n> <branch>` and records
  **Issue** `#<n>` in the ledger. `ISSUE_TAKEN`: another flow's work; escalate as *blocked by work
  outside my flow*.
- **Every PR body** ends with `Refs #<n>`, never `Closes #<n>`: the first merge would close it.
- **Done**: `scripts/autopilot/issue.sh done <n> <pr>...` with every PR in the ledger and the record
  PR, only after `handoff-check.sh` passes. A run that stops unfinished leaves the issue open and
  labelled.
