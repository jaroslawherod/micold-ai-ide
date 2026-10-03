# Task: resume a run (orchestrator)

When: the argument is `resume`. **Never** rebuild progress from `gh pr list` or from memory.

Run `scripts/autopilot/resume.sh`. It finds this worktree's ledger (working tree first, where an
unpushed ledger is newest; then `origin/main`) and prints its phase, next step, open escalation,
and GitHub's state for each recorded PR.

| Exit | Meaning | Do |
|---|---|---|
| 0 | `LEDGER …` | Read the flow file of the ledger's **Kind**, and resume at the first unfinished step. `HANDOVER` lines: dispatch a unit of that kind to continue from the ledger's *Handover*. |
| 0 | `LEDGER-ON-MAIN …` | Run `scripts/autopilot/branch-start.sh`, then `resume.sh` again. |
| 2 | `NONE` | Say there is no run to resume here, and stop. |
| 3 | several ledgers | Ask with one `AskUserQuestion`: each option names a ledger's feature, phase and next step. Recommend the most recently committed one. |
| 4 | `FINAL-PR-PENDING <ledger> <pr\|none>` | The ledger reads `done` but the run's last PR never merged. Wait on and merge `<pr>`, then run the handoff. On `none`, dispatch the flow's last unit (close, or the bugfix's milestone) to open it. |

- **When GitHub and the ledger disagree, GitHub is right.** Have the next unit fix the ledger. A
  milestone marked merged whose PR is open, or whose changes are missing from `origin/main`, goes
  back to a milestone unit. If a commit on `main` reverted it, escalate (category 6).
- A non-empty *Open escalation* means the question was never answered: ask it again, then dispatch
  a fresh unit of that kind with the answer.
- **An older ledger:** **Kind** `quick` is the chore flow; **Kind** `bug` with a ledger under
  `bugs/` is the bugfix flow; a **Phase** like `4-milestones` names the unit by its old number.

Next: [dispatch.md](dispatch.md) or [merge.md](merge.md), as the first unfinished step says.
