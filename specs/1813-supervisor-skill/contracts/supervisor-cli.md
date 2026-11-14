# Contract: `scripts/autopilot/supervisor.sh`

All subcommands read the ledger path from `--ledger <file>` (default `.autopilot/supervisor.md`),
write only to stdout (and the ledger for `ledger ...`), and exit 0 on a printed answer, 2 on a usage
error or malformed ledger.

| Subcommand | Output |
|---|---|
| `gate [--cap N] [--running N] [--min-mem-mib N] [--max-load F]` | `PASS` or `FAIL <cap\|memory\|load>: <reason>`. Readings from `AUTOPILOT_SUP_MEM_MIB`, `AUTOPILOT_SUP_LOAD`, `AUTOPILOT_SUP_CORES`, else the machine. Exit 0 on PASS, 1 on FAIL. |
| `map <run-status-json-file\|-> --started <epoch> [--now <epoch>] [--grace S]` or `map --exit <2\|3> ...` | one ledger state (`done`, `running`, `lost`); `--exit 2\|3` (run-status failed) gives `lost`; Grace defaults to the ledger header when `--ledger` is given |
| `touches <issue-body-file> [--labels "a b"]` | comma separated paths, `-` when none; first line `declared`, `inferred` or `none` on stderr |
| `overlap <touches-a> <touches-b>` | `yes` or `no` |
| `next [--live-sessions "ids"]` | candidates only (the skill runs `gate --running <occupied + starts so far>` before each `create_worktree`, so one reading never admits several starts): issue numbers, one per line, ascending: queued, within cap minus occupied, and no overlap with any occupying row or an earlier pick. Prints nothing when none. |
| `check-owner --live-sessions "ids" --me <id>` | `OWN`, `FREE` (no owner or owner not listed; read-only, the skill then calls `ledger owner <id>`), or `HELD <owner>` (exit 1) |
| `ledger init --owner <id> --cap N [--grace S] --source <text> <issues...>` | creates the file; refuses when it exists |
| `ledger owner <id>` | sets the Owner header |
| `ledger set <issue> <field> <value>` | updates one cell; `State` values are checked against the seven names |
| `ledger get <issue> <field>` | prints the cell |
| `ledger table` | the short report table for the advance |

`supervisor-watch.sh --ledger <f> [--interval S] [--ceiling S]`: prints `CHANGED <issue> <old> <new>`
or `CEILING` and exits 0. `AUTOPILOT_RUN_STATUS` replaces the `run-status.sh` path (fixtures);
`AUTOPILOT_WATCH_CEILING` the default ceiling (600). Tests pass `--interval 1 --ceiling 2`.
