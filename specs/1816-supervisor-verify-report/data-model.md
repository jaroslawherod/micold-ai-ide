# Data model: supervisor verify handoff and report

## Verdict
| Field | Source | Notes |
|---|---|---|
| `verdict` | `verify-run.sh` | `VERIFIED` or `NOT DONE`, a closed pair; exit 0 / 1 |
| remaining items | handoff lines, phase line, issue lines | one `- ` line each; empty only when `VERIFIED` |

Remaining item kinds (the tests assert each): `uncommitted work: <n> path(s)`, `<n> commit(s) not on
origin/main`, `PR #<n> is <state>` (`OPEN`, `CLOSED`, `UNKNOWN`), `ledger phase is <phase>`,
`no ledger`, `issue #<n> is open`, `issue #<n> still labelled in-progress`, `issue #<n> lacks
flow:<flow>`, `issue #<n> state UNKNOWN`.

## Follow-up action
A closed set of six words, the first token of the first output line: `DONE` (verified; nothing
sent), `SEND` (live session; the message follows), `RESUME` (stopped session), `ESCALATE` (cap
reached, block written), `HOLD` (already escalated; nothing sent), `SKIP` (same remaining set
already sent under this check id).

## Supervisor ledger (read; `follow-up.sh` also writes)
| Part | Shape | Used by |
|---|---|---|
| Opt-in | the exact line `- **Auto-remove**: yes`; any other value or absence is no | `safe-to-remove.sh`, `close-out-report.sh` read |
| `## Interventions` | table `\| Run \| When \| What \|`; Run is the run's branch | report reads |
| `## Follow-ups` | table `\| Run \| Count \| Remaining hash \| Check \|`, one row per branch | `follow-up.sh` reads and writes |
| `## Escalations` | one `### <branch>` block per run: Category 5, Question, Options, Recommended, Evidence (the fields of `templates/autopilot-ledger.md`) | `follow-up.sh` writes once; report reads |

Count rules: no row reads as 0; `SEND` and `RESUME` each add 1; `DONE`, `SKIP`, `HOLD` and
`ESCALATE` add 0; the escalation fires when the count is already 3 and the run is still
`NOT DONE`. A run that turns `VERIFIED` keeps its row (the count stops).

## Report entry
| Field | Source |
|---|---|
| issue | run-status `issue` |
| PRs | run-status `prs`: `#<n> <state>` |
| decisions | the run ledger's *Decisions* table: By `user` counts as user, every other row as agent |
| interventions | supervisor ledger rows for the run's branch, else `none` |
| tokens | run ledger `- **Tokens**: <n>`, else `unknown` |
| remaining | the verdict's lines (Not done only) |

## Safe-to-remove entry
`SAFE <path>` or `KEEP <path> <reason>`; reasons: `not verified: <first remaining line>`, `live
session`, `no session list given`, `not a git worktree`, `several ledgers`. `REMOVE <path>` only after `SAFE` and only
with opt-in.
