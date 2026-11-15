# The supervisor's close-out step

When a run the supervisor started stops, the supervisor does not trust the run's own "done": it verifies,
follows up, reports and lists. Four read-only scripts in `scripts/autopilot/` do the judging; the
supervisor makes every Micold call (`send_message`, `resume_session`, `delete_worktree`) itself,
because the scripts have no session transport. Spec: `specs/1816-supervisor-verify-report`.

1. **Verify**: `verify-run.sh <worktree>` prints `VERIFIED`, or `NOT DONE` and one `- ` line per thing that
   remains (open PR, dirty tree, unmerged commit, issue open, `in-progress` or the `flow:` label
   missing, state `UNKNOWN`). Exit 3: several ledgers name the branch, no verdict. A run is done only on `VERIFIED`.
2. **Follow up**: `follow-up.sh <worktree> --supervisor-ledger <file> --session live|stopped
   [--session-id <id>] [--check <id>]` prints the next action: `DONE`, `SEND <id>` (live session: send
   the message under it), `RESUME <id>` (stopped: resume with it), `SKIP`, `HOLD`, or `ESCALATE` after
   three counted follow-ups. Do what it says and nothing more; `ESCALATE` goes to the user as a category 5 question.
3. **Report**: `close-out-report.sh <worktree>... --supervisor-ledger <file> [--live <path>]...
   [--no-live-sessions] --parent <your ref> --rows <list_worktrees rows file>` prints Completed, Not
   done, Escalated and Safe to remove. Pass `--parent` and `--rows` so only your own runs are reported
   (FR-016): save the unfiltered `list_worktrees` result to a file and let `runs-of-parent.sh` select. A run
   of another parent or with no parent is not yours: do not verify, follow up, report or remove it.
4. **Safe list**: `safe-to-remove.sh <worktree>... [--live <path>]... [--no-live-sessions]
   [--supervisor-ledger <file>] [--parent … --rows …]` prints `SAFE <path>` or `KEEP <path> <reason>`.
   SAFE means `VERIFIED` (so clean, nothing unpushed) and no live session. Pass `--live` for every
   worktree a session works in, or `--no-live-sessions` after you checked none does; with neither, every
   worktree is KEEP `no session list given`.

## Removal

Removal is the user's click. Only when the supervisor ledger has the line `- **Auto-remove**: yes` (any
other value or none is no opt-in) does `safe-to-remove.sh` add a `REMOVE <path>` line after a `SAFE` one. Then call the app's own
`delete_worktree` for that path, which still asks confirmation when nobody pre-approved it. Never
remove a worktree without a `REMOVE` line, never with `git worktree remove` or `rm`, never one under `KEEP`.
The scripts remove nothing.

## Order

Verify each stopped run; follow up the `NOT DONE`; when all runs are `DONE`, `HOLD` or escalated, run the
report and the safe list once, then show the user the report.
