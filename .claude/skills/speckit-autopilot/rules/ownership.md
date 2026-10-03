# Ownership: only this flow's work

Other sessions work in this repo at the same time. A PreToolUse hook
([gate-hook.sh](../../../../scripts/autopilot/gate-hook.sh)) blocks the worst breaches: another
flow's PR, `--delete-branch`, `--admin`, removing the worktree, and pushing code no green gate saw.
A block is a rule you broke, not an obstacle: never work around it.

This flow owns:

- this worktree and its branch
- the feature directory (or BUG record) this flow created
- the PRs in the ledger, and its **Issue**
- the subagents it spawned

Everything else is outside the flow:

- **Other PRs.** Never list, review, merge, rebase, approve runs on, comment on or close them, even
  when green.
- **Other specs.** Never edit another feature's `tasks.md` or spec. Only exception: the bugfix
  flow's patch to a **Closed** owning spec, via `speckit-bugfix-patch`.
- **Other worktrees and branches.** Never touch them.
- **Red `main`, or a failure from code this flow did not write.** Do not fix it. Escalate as
  *blocked by work outside my flow*, and say what you checked.
- **A defect in someone else's code.** List it under *Follow-ups not done* in the ledger; it is
  copied into the handoff.
- **This worktree and its branch.** Never pass `--delete-branch`, never delete either. The user
  removes the worktree in micold IDE, which also cleans up the branch.
