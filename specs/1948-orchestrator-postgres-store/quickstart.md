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

## B. By hand (local mode)

1. `micold-daemon orchestrator` with no `[database]`: `~/.local/share/micold/orchestrator/orchestrator.db` appears with mode 0600.
2. Link a daemon, stop the orchestrator, start it again: `micold-daemon` lists the daemon (offline, then online).
3. Put a v1 `registry.json` beside it before the first start: it is imported and renamed `registry.json.imported`.
4. `micold-daemon config check` with `[database] url_file = "/nonexistent"`: fails naming the key and path.
5. Newer schema: `sqlite3 orchestrator.db "insert into _sqlx_migrations(version,description,success,checksum,execution_time) values (99,'x',1,x'00',0)"`, start: refused, both versions in the message; the file's checksum (`sha256sum`) is unchanged by the attempt.
