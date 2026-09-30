# Resuming an autopilot run

Run `scripts/autopilot/resume.sh`. It finds this worktree's ledger (working tree first, where an
unpushed ledger is newest; then `origin/main`) and prints its phase, next step, open escalation, and
GitHub's state for each recorded PR.

| Exit | Meaning | Do |
|---|---|---|
| 0 | `LEDGER …` | Resume at the first unfinished step. `HANDOVER` lines: dispatch a unit of that phase to continue from the ledger's *Handover*. |
| 0 | `LEDGER-ON-MAIN …` | Run `scripts/autopilot/branch-start.sh`, then `resume.sh` again. |
| 2 | `NONE` | Say there is no run to resume here, and stop. |
| 3 | several ledgers | Ask with one `AskUserQuestion`: each option names a ledger's feature, phase and next step. Recommend the most recently committed one. |
| 4 | `RECORD-PR-PENDING <ledger> <pr\|none>` | The run finished but its record PR never merged. Wait on and merge `<pr>`; on `none`, dispatch the record unit with the ledger's last PR and its merge SHA. Then run the handoff. |

**When GitHub and the ledger disagree, GitHub is right.** Fix the ledger. A milestone marked merged
whose PR is open, or whose changes are missing from `origin/main`, goes back to a milestone unit.
If a commit on `main` reverted it, escalate (category 6). A non-empty *Open escalation* means the
question was never answered: ask it again, then dispatch a fresh unit of that phase with the answer.
