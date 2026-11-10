# Data model: supervisor run status

## Run state
Closed set, in precedence order: `dead-no-ledger`, `done`, `blocked`, `asking`, `stuck`,
`waiting-ci`, `running`.

| Field | Source | Notes |
|---|---|---|
| `state` | resolver | one of the seven |
| `underlying_state` | resolver | the state `stuck` replaced; else equal to `state` |
| `issue` | ledger `- **Issue**:` | `#<n>` or `none` |
| `flow` | ledger `- **Kind**:` | feature, bug, bugfix, chore (quick = chore) |
| `phase`, `next_step` | ledger | empty string when no ledger |
| `ledger` | lookup | path relative to the worktree, or null |
| `supervised` | `supervised.sh` | bool |
| `prs[]` | ledger numbers + GitHub | `{number, state}`, state from `gh`; `UNKNOWN` when `gh` fails |
| `since`, `seconds_in_state`, `dwell_source` | see Dwell | `dwell_source` is `pr-state` or `ledger` |
| `escalation` | ledger section | present only when state is `asking` or `blocked`, else null |
| `notes[]` | resolver | e.g. `branch-start.sh is next`, handoff-check lines when `done` is refused |

## Escalation
`escalation.questions[]`, each `{number, question, options[], recommendation, evidence}`; `options[]`
elements `{label, description}`; any missing field is `""` or `[]`, never dropped. `escalation.category`
and `escalation.banner` are strings (may be empty).

## Dwell
`stuck` needs an open PR whose `pr-state.sh` line ends `STUCK` (exit 1). `since` = `now - DWELL`
from that helper when it ran; else the ledger's last commit time, or its mtime when it has
uncommitted changes. `seconds_in_state = now - since`; `now` honours `AUTOPILOT_NOW` (epoch
seconds, the helper's own test hook) for tests.

## State transitions (resolver, first match wins)
1. no ledger names the branch, or ledger only on `origin/main` -> `dead-no-ledger` (several ledgers: error)
2. phase `done` and `handoff-check.sh` OK -> `done`
3. escalation non-empty with `blocked by work outside my flow` -> `blocked`
4. escalation non-empty -> `asking`
5. open PR and `pr-state.sh` STUCK -> `stuck` (underlying `waiting-ci`)
6. open PR -> `waiting-ci`
7. else `running`

`resume.sh` mapping (FR-001b): exit 0 one ledger -> steps 3-7; exit 4 (`done`, last PR unmerged) ->
step 6 when its PR is open, else 7; `LEDGER-ON-MAIN` -> 1; exit 3 -> error.

## Supervised marker
`AUTOPILOT_SUPERVISED` (env) or `- **Supervised**: yes` (ledger header line, after **Effort**).

## Open escalation and Decisions row
Format in [contracts/supervised-escalation.md](contracts/supervised-escalation.md). A Decisions row
written by `answer.sh`: `| D<n> | <phase> | <question> | <answer> | user | supervised answer <date> |`,
`<n>` one past the highest existing `D<n>`.
