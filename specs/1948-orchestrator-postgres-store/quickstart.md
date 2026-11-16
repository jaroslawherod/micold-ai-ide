# Quickstart: orchestrator store

No UI, so no visual pass; §B is a by-hand local-mode check. Part A is run by CI; the commands run in a worktree that has `mise`.

## A. Automated

```bash
mise run check                                   # fast loop while editing
mise run test-core                               # core types, memory stores, choose_store, config
scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_   # SQLite + memory, no server
MICOLD_TEST_POSTGRES_URL=postgres://postgres:pw@localhost:5432/postgres \
  scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_ # adds PostgreSQL
mise run gate                                    # CI's order; CI runs the Postgres service container
```

Expected: every `store_*` test passes on memory and SQLite; with the URL set, on PostgreSQL too;
without the URL, PostgreSQL tests print `skipped: MICOLD_TEST_POSTGRES_URL not set` (CI sets
`MICOLD_REQUIRE_POSTGRES=1`, where that skip fails).

Recorded (M10, 2026-11-16, cloud container, PostgreSQL 16 local): `cargo test -p micold-daemon
--test orchestrator store_` with `MICOLD_TEST_POSTGRES_URL` and `MICOLD_REQUIRE_POSTGRES=1` set:
129 passed (memory, JSON, SQLite, PostgreSQL). `test-core` and the gate ran through
`scripts/gate.sh` (no `mise` there); see the ledger's *Review rounds* for the run.
Use a password that is not a substring of the error text: with `postgres:postgres`,
`store_slow::a_save_lost_with_the_database_is_reported_on_stderr_without_the_password` fails
whenever the logged error names `sqlx_postgres` (it found the "password" in the module path).

## B. By hand (local mode)

The data directory is `~/.local/share/micold-ai-ide` (`XDG_DATA_HOME` honoured); a daemon links
with `schema = 1`, `[daemon]`, `[listen] socket = "auto"` and `[orchestrator_link] enabled = true`
(`socket` = the orchestrator's, `$XDG_RUNTIME_DIR/micold/orchestrator.sock`).

1. `micold-daemon orchestrator` with no `[database]`: `~/.local/share/micold-ai-ide/orchestrator/orchestrator.db` appears with mode 0600.
   **Recorded**: appears, mode `600` (with `-wal`/`-shm` beside it).
2. Link a daemon, stop the orchestrator, start it again: the `daemons` row survives and the daemon
   is listed again (offline, then online when it relinks). **Recorded**: row `('default', 'local',
   'qs box', 'online', 'unowned')`; after a restart the row is back and online; with its owner set
   to `alice` by hand before the restart, the relink kept `alice`; with the daemon stopped the row
   went `offline`, still `alice`.
3. Put a v1 `registry.json` beside it before the first start: it is imported and renamed `registry.json.imported`.
   **Recorded**: imported as `('old', 'offline', 'unowned')`, file renamed.
4. `micold-daemon config check <file> --files` with `[database] url_file = "/nonexistent"`: fails
   (exit 2) naming the key and path. Without `--files` it exits 0 and lists the path as
   `(missing)`. **Recorded**: `c.toml:8:12: database.url_file /nonexistent: missing`, exit 2.
5. Newer schema: insert version 99 into `_sqlx_migrations` (`sqlite3 orchestrator.db "insert into _sqlx_migrations(version,description,success,checksum,execution_time) values (99,'x',1,x'00',0)"`, or Python's `sqlite3` where the CLI is absent), start: refused, both versions in the message; the file's checksum (`sha256sum`) is unchanged by the attempt.
   **Recorded**: `database schema version 99 is newer than this build's 1`, exit 1, checksum unchanged.
