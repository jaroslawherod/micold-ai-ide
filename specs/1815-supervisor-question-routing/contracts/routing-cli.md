# Contract: routing commands (#1815)

All `route` subcommands take `--ledger <file>` like the rest of `supervisor.sh`, never call `gh`,
MCP or `run-status.sh`, and exit 2 on a usage error or malformed ledger (as `supervisor.sh`).

| Command | Prints | Exit |
|---|---|---|
| `route gate <category>` | `user-only` for a category starting `2`, `3` or `4`; `judge` for `1`, `5`, `6` (the skill then decides by subject, FR-003) | 0; 2 unknown |
| `route add <issue> <session> <escalation-json-file>` | one `R<issue>.Q<n>` per question, written `pending` (no-op for keys already present, so a restart never asks twice) | 0 |
| `route next` | up to 4 `pending` keys with their question, options, recommendation, evidence, ascending issue then number; empty when none | 0 |
| `route state <key> <pending\|asked\|answered\|resolved>` | nothing | 0; 2 unknown state or key |
| `route answer <key> <via> <text>` | nothing; sets State `answered` (`via` `user` or `parent`) or `resolved` (`via` `supervisor`) and Answer | 0; 2 |
| `route compose <issue>` | the answer message: `<n>. <answer>` per question (a free-text escalation: the bare answer), plus the leading `--by supervisor` word only when every row is `resolved` by the supervisor | 0 complete; 1 incomplete, prints `missing: <n> ...` (FR-008a) |
| `route sent <issue> <epoch>` | nothing; sets Sent on the issue's rows | 0 |
| `route overdue --now <epoch> --ceiling <s>` | issues whose Sent is set and older than the ceiling | 0 |
| `route target --sessions <file> --me <id>` | `parent <id>` when that row has a non-empty `parent`, else `user` | 0; 2 file unreadable |
| `route drop <issue>` | nothing; removes the issue's rows | 0 |
| `route table` | the Routing section | 0 |

Rows are never written by hand. A ledger without `## Routing` gains an empty one on the first `add`.

## `answer.sh [--by <who>] <ledger> <answer-text>`
`--by` default `user`; its value is written in the By cell of every row it adds. The rest of the
contract of `specs/1812-supervisor-run-status/contracts/supervised-escalation.md` is unchanged
(exit 0, 2, 3).

## Messages to a worker (`send_session_input`)
- `resume --by supervisor <answer>`: the supervisor resolved it. `tasks/resume.md` runs
  `answer.sh --by supervisor <ledger> "<answer>"`.
- `resume <answer>`: relayed from the user (or the parent): By `user`.
A batch is one line `<n>. <answer>` per question number. The supervisor sends only when `route
compose` exits 0.

## Upward relay (conditional, FR-012)
`route target` says `parent <id>`: the skill sends the banner to that session with
`send_session_input` and holds the rows `asked` with Via `parent`; the parent's reply goes through
`route answer ... parent`. A `not_found` refusal: `ledger set <issue> Note "<refusal>"`, then the
question is asked of the user (Via `user`), never retried.
