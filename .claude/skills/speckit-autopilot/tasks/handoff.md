# Task: hand off, the last message (orchestrator)

When: the run's last PR has merged (the close PR, the bugfix's fix PR, or the bug or chore PR).

1. `scripts/autopilot/handoff-check.sh <ledger>`. It checks the tree is clean, every commit is on
   `origin/main` (by patch, since rebase-merge rewrites SHAs), and every PR reads `MERGED`. If it
   prints `NOT DONE`, report exactly what remains, and stop here.
2. Post the token report on the last PR without reading it (not in the bug and chore flows: their
   ledger is not committed):

   ```bash
   mise run autopilot-tokens > "$SCRATCHPAD/tokens.md" && gh pr comment <last-pr> --body-file "$SCRATCHPAD/tokens.md"
   ```
3. Close the issue: [issue.md](issue.md), *Done*.
4. Send this, with a `PushNotification`:

```
✅ WORK COMPLETE — #<issue> <feature or task> (<flow>)
Delivered (all merged to main):
  M1 #<pr> — <deliverable>        (bugfix: BUG-<issue> #<pr> — <what now works>)
  M2 #<pr> — <deliverable>        (bug, chore: #<pr> — <what changed>)
Decisions: <n> made by you, <m> resolved by me from repo evidence — see <ledger path>
Follow-ups not done: <none | list>
This worktree has no uncommitted work, no unpushed commits and no open PRs.
👉 You can remove this worktree in micold IDE now — that also cleans up its branch.
```
