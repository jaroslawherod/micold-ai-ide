# Handoff: the last message

First dispatch the record unit with the last PR and its merge SHA. It closes the ledger and opens
the record PR; wait on it and merge it as any other. It is not listed in the ledger, so pass it to
the checks.

Then run `scripts/autopilot/handoff-check.sh <ledger> <record-pr>`. It checks the tree is clean,
every commit is on `origin/main` (by patch, since rebase-merge rewrites SHAs), and every PR reads
`MERGED`. If it prints `NOT DONE`, report exactly what remains. Otherwise send this with a `PushNotification`:

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
