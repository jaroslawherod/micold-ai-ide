# Handoff: the last message

When the run's last PR (the close PR, or a bug's fix PR) has merged, run
`scripts/autopilot/handoff-check.sh <ledger>`. It checks the tree is clean,
every commit is on `origin/main` (by patch, since rebase-merge rewrites SHAs), and every PR reads
`MERGED`. If it prints `NOT DONE`, report exactly what remains. Except for a quick run (its ledger
is not committed), post the token report on the last PR without reading it:

```bash
mise run autopilot-tokens > "$SCRATCHPAD/tokens.md" && gh pr comment <last-pr> --body-file "$SCRATCHPAD/tokens.md"
```

 A ledger with an **Issue**: close
it per [issue.md](issue.md). Otherwise send this with a `PushNotification`:

```
✅ WORK COMPLETE — <NNN-feature>
Delivered (all merged to main):
  M1 #<pr> — <deliverable>        (a bug: BUG-<k> #<pr> — <what now works>)
  M2 #<pr> — <deliverable>
Decisions: <n> made by you, <m> resolved by me from repo evidence — see <ledger path>
Follow-ups not done: <none | list>
This worktree has no uncommitted work, no unpushed commits and no open PRs.
👉 You can remove this worktree in micold IDE now — that also cleans up its branch.
```
