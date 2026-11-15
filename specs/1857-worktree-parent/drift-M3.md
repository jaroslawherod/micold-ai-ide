# Drift, milestone M3

- tasks.md has no `## Milestones` block for M3 (the milestone list is one prose line, as for M1b),
  so `brief.py milestone` and `plan-drift.sh` have no M3 entry; this file is the record.
- T026 said "issue bodies": the notes on #1813, #1814, #1816 and #1817 were delivered as comments.
  T026 now says so.
- T024 names rows by `ref` (the directory name in `list_worktrees` rows, tools.rs `worktree_rows`),
  not `name`: real rows carry `ref`, `display_name` and `path` and no `name`. `--check` also takes a
  row's `path`, because the supervisor ledger's `Worktree` cell is a path.
- `--orphans` needs the unfiltered `list_worktrees`: the daemon's `parent: <ref>` filter keeps only
  rows whose parent state is `known`, so removed-parent rows never appear in a filtered list.
- Files beyond T023-T026, T033: `scripts/autopilot/supervisor.sh` (the `Parent` field, a ninth
  ledger column; an eight-column ledger refuses `Parent`), `scripts/tests/supervisor.test.sh` (ledger
  header), and `specs/1813-supervisor-skill/contracts/supervisor-cli.md` (field list).
