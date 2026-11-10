# Contract: `scripts/autopilot/run-status.sh`

```
run-status.sh [--json] [<worktree-dir>]
```
Default directory: the current one. Read-only (it may run `git fetch -q origin`, as
`handoff-check.sh` does).

## Exit codes
| Code | Meaning |
|---|---|
| 0 | a state was printed (every one of the seven, including `dead-no-ledger`) |
| 2 | usage error, or `<worktree-dir>` is not a git worktree |
| 3 | several ledgers name the branch: each listed on stderr, no state |

## Environment
`AUTOPILOT_PR_STATE` path of the PR-state helper (default `$(dirname "$0")/pr-state.sh`, called for open PRs only; any `STUCK` makes the run `stuck`);
`AUTOPILOT_NOW` epoch seconds replacing the clock; `AUTOPILOT_LIMIT_*` are read by the helper only.

## JSON (`--json`): one object, one line
```json
{"state":"asking","underlying_state":"asking","issue":"#1812","flow":"feature","phase":"plan",
 "next_step":"plan unit","ledger":"specs/1812-x/autopilot.md","supervised":true,
 "prs":[{"number":1810,"state":"OPEN"}],
 "since":1700000000,"seconds_in_state":120,"dwell_source":"ledger",
 "escalation":{"category":"1 Product or scope decision","banner":"...",
   "questions":[{"number":1,"question":"...","options":[{"label":"A","description":"..."}],
                 "recommendation":"A","evidence":"..."}]},
 "notes":[]}
```
`escalation` is `null` unless the state is `asking` or `blocked`. With no ledger: `ledger` null,
`issue`/`flow`/`phase`/`next_step` empty, `prs` `[]`.

## Text
`key: value` lines in the same order, one per field; PRs as `PR #<n> <state>`; escalation as
`question N: …`, `option N.k: …`, `recommended N: …`, `evidence N: …`. First line `STATE <state>`.

## State order
See data-model.md *State transitions*.
