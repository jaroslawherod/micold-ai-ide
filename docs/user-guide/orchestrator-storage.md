# Orchestrator storage: local mode or PostgreSQL

The orchestrator (`micold-daemon orchestrator`, see [Orchestrator mode](orchestrator-mode.md)) keeps
its registry of daemons, its users and sign-in records and its audit log in a database. It picks the
database from its configuration file: `--config <path>` after `orchestrator`, or `daemon.toml` in your
data directory when that exists.

## Local mode (the default)

With no `[database]` table, the orchestrator uses a SQLite file:
`<data_dir>/orchestrator/orchestrator.db`. It creates the file on first start, readable by your user
only (mode `0600`), and needs no database server and no other setting. This is the right choice for an
orchestrator on your own machine.

Local mode is allowed only while the orchestrator listens on this machine alone. An orchestrator
listening on a network address must have a `[database]` table; without one it refuses to start, and
`micold-daemon config check` fails naming `database.url_file`.

An orchestrator from an earlier version kept its registry in
`<data_dir>/orchestrator/registry.json`. On its first start the orchestrator imports that file into
the database (either backend) in one step, then renames it `registry.json.imported` in the same
directory, keeping its permissions. Later starts find no `registry.json` and import nothing; keep or
delete the `.imported` file as you like.

The import refuses, and the orchestrator does not start, when:

- the file cannot be read or parsed: the error names the file, and nothing is imported;
- the database registry already has daemons in it: the error names the file and the row count
  (`cannot import <path>: the database registry already has <n> row(s)`), and nothing is imported.

If the orchestrator stopped after the import but before the rename, the next start recognises the
file by its digest and only renames it.

## PostgreSQL

To share an orchestrator, point it at a PostgreSQL server:

```toml
[database]
url_file = "/etc/micold/secrets/database-url"   # required
ca_file  = "/etc/micold/secrets/database-ca.pem" # optional
pool_size = 5                                    # optional, 1 to 64
```

| Key | Meaning |
|---|---|
| `url_file` | A file holding the whole connection URL, password included, for example `postgres://micold:<password>@db.example.org:5432/micold`. Surrounding whitespace is ignored. The URL never appears in the configuration file itself, in `config check` output or in any error. |
| `ca_file` | The certificate authority of the server. When set, the connection uses TLS and the server's certificate must chain to it and match the host name. When absent, TLS is used if the server offers it. |
| `pool_size` | How many connections the orchestrator keeps open. Default 5. |

`micold-daemon config check` fails when `url_file` is missing, unreadable or empty, when its URL is not
`postgres://` or `postgresql://`, or when `ca_file` does not exist. It shows each path and whether the
file is readable, never the URL.

If the server cannot be reached at start, the orchestrator exits with
`orchestrator database <host>:<port> unreachable: <cause>`; the password is never printed. If the
server goes away while the orchestrator runs, it keeps running: a change that cannot be saved is
reported on its error output, and the next call connects again.

## What is stored, and what is never stored

Stored: the daemons the orchestrator knows (name, kind, version, last seen), users and their linked
sign-in identities, invites, refresh tokens and enrollment records (as hashes, never the token
itself), certificate serials and revocations, and the audit log. Every row carries the tenant it
belongs to.

Never stored: private keys, raw tokens or passwords, and the database URL.

## Upgrades and downgrades

The orchestrator brings the database schema forward on start. A database whose
schema is newer than the running build is refused, with
`database schema version <db> is newer than this build's <build>`, and nothing is changed: run the
newer build, or restore a backup taken before the upgrade.
