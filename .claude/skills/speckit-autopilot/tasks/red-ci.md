# Task: fix a red CI run (the unit that opened the PR)

When: the orchestrator continues you with a `RED <n> <run> <log>` line: CI failed in this flow's
code.

1. `grep` the log for the failure; never read it whole.
2. Run `systematic-debugging` on it. Do not guess-fix.
3. Fix, run the full gate again ([gate.md](gate.md)), commit, push.

After the third failed attempt, escalate (category 5).

Hands on: return `DONE` again with the PR number.
