# Contract: `[database]` and backend choice

```toml
[database]
url_file  = "/run/secrets/micold/database-url"   # file holding postgres://user:password@host:5432/db
ca_file   = "/run/secrets/micold/database-ca.pem" # optional: CA for TLS to the server (verify-full)
pool_size = 5                                     # optional, 1..=64, default 5
```

`deny_unknown_fields`. Values are paths and a number; the URL is never in the TOML.

## Resolution table (`choose_store`)

| `[database]` | Bind | Result |
|---|---|---|
| present | any | PostgreSQL from `url_file` |
| absent | loopback only | SQLite at `<data_dir>/orchestrator/orchestrator.db` |
| absent | network | refuse: `database.url_file` is required when the orchestrator listens on a network address |

Until #1732 adds a listener key, the bind scope the orchestrator passes is `LoopbackOnly`.

## `config check` rules

1. `url_file` set but missing, unreadable or blank after trimming: fails `database.url_file <path>: <state>`.
2. URL scheme not `postgres`/`postgresql`: fails `database.url_file <path>: scheme "<scheme>" is not postgres or postgresql`; the rest of the URL is not printed.
3. `ca_file` set but not an existing file: fails `database.ca_file <path>: missing`.
4. `pool_size` outside 1..=64: fails naming the key.
5. `[database]` without `url_file`: fails naming `database.url_file`.
6. D6: `choose_store` fails when there is no `[database]` table and the bind is a network address, naming `database.url_file`. `config check` asks it with `LoopbackOnly` until #1732, so it cannot fail there yet.
7. Resolved view: `database.url_file = <path> (readable|missing|empty)`, `database.ca_file = <path> (…)`, `database.pool_size = N`; no URL, host or password.

## Errors at start

- Unreachable or refused: `orchestrator database <host>:<port> unreachable: <cause>`; the cause text is
  the driver's with the URL removed; the password never appears (tested by grep).
- SQLite path not writable, or WAL not granted: `orchestrator database <path>: <cause>`.
- Newer schema: `database schema version <db> is newer than this build's <build>`; nothing changed.
- Edited migration: `migration <version> <description> was edited after it was applied`.
- Un-imported `registry.json` with rows present: `registry import refused: <file> has records but the database registry already has <n> rows`.
