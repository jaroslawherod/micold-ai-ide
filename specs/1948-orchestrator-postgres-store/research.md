# Research: PostgreSQL store for the orchestrator

## R1 Sync traits over a private runtime, not async traits

- **Decision**: traits stay synchronous; SQL stores `block_on` a private 2-worker runtime.
- **Why**: FR-001 keeps the #1724 `RegistryStore` contract valid for current callers (`RegistrySaver`,
  `SaveWorker`, `GatedStore` doubles, `JsonRegistryStore`). The only caller is already a dedicated
  thread, so blocking there cannot stall links (FR-014).
- **Rejected**: async traits (`async-trait` or RPITIT): breaks `Arc<dyn RegistryStore>` object use
  and every existing implementor, adds a crate or an object-safety workaround. Sharing the main runtime
  with `block_in_place`: ties store latency to link tasks' workers.
- **Cost**: async callers must use `spawn_blocking`; `block_on` returns an error, not a panic, inside
  a runtime.

## R2 One implementation over `sqlx::Any`

- **Decision**: `AnyPool`, SQL in the common subset: `$n` placeholders, `ON CONFLICT … DO UPDATE`,
  `RETURNING`, `TEXT`/`BIGINT` columns only (hashes as hex text, times as unix seconds, flags as 0/1).
- **Why**: every query is written and reviewed once; the shared suite runs the same code on both.
- **Rejected**: a generic `Store<DB: Database>` with per-dialect SQL (about double the query code);
  two hand-written stores (drift); an ORM such as Diesel or SeaORM (sync API or more layers than the
  queries need, as the 1722 plan decided).
- **Risk and fallback**: if task 1's spike shows `Any` cannot take `$n` on SQLite, `Migrator` on
  `AnyConnection`, or a needed type, fall back to the generic-over-`DB` form behind the same
  `StoreHandle` wrapper; the traits and tests do not change.

## R3 Runtime-checked queries, not `query!`

- **Decision**: `sqlx::query` and `query_as` with `.bind`.
- **Why**: compile-time checking needs `DATABASE_URL` or a committed `.sqlx` cache per dialect; cloud
  threads and `mise run check` have no database, and `Any` does not support the macros.
- **Rejected**: `query!` with offline data (two caches to keep in step with two dialects). The shared
  suite on both backends plays the checking role.
- **Note**: this narrows the 1722 plan's "compile-time-checked queries"; the parent plan is a
  statement of intent, this records why it is not followed.

## R4 `sqlx::migrate!` with a read-only newer-schema pre-check

- **Decision**: sqlx's migrator for apply, checksum, transaction and lock; our `check_not_newer`
  first.
- **Why**: sqlx already gives FR-003 (lock, per-migration transaction, forward only) and FR-005
  (checksum). It does not give FR-004's message or its no-write guarantee (it creates
  `_sqlx_migrations` and locks before it notices a missing version).
- **Rejected**: a hand-written migrator (reimplements locking and checksums); `ignore_missing = true`
  (silently runs an old build on a newer schema).

## R5 Two migration directories

- **Decision**: `migrations/postgres/` and `migrations/sqlite/`, same file names and versions.
- **Why**: audit sequence type and trigger syntax differ. A test asserts both directories list the
  same versions and descriptions, so a migration cannot ship for one dialect only.
- **Rejected**: one shared directory with a lowest-common-denominator audit table (the audit sequence
  could not be a native serial, and the append-only trigger differs anyway).

## R6 Sentinel `unowned` owner for pre-user records

- **Decision**: `owner_user_id = 'unowned'`, adopted by the first owner user via `reassign_unowned`.
- **Why**: FR-007 wants the column filled; FR-010 imports records before any user exists; v1 has one
  tenant and one owner. A sentinel needs no fake user and no nullable column; the adoption is one
  idempotent statement.
- **Rejected**: nullable owner (breaks "every record carries an owner", every query needs `IS NULL`
  handling); a seeded `system` user row (a user that can never sign in, and a foreign key to maintain);
  deferring the import until a user exists (the import then depends on sign-in, #1733).

## R7 Append-only by trigger, not by `GRANT` alone

- **Decision**: a trigger rejecting `UPDATE` and `DELETE` on `audit_log`, plus a docs rule to grant
  `INSERT, SELECT` only.
- **Why**: a migration cannot know the orchestrator's role name, and SQLite has no roles. The trigger
  holds for any role that is not the table owner; the docs cover the rest.
- **Rejected**: `REVOKE`/`GRANT` in the migration (role unknown, absent in SQLite); a trait-only
  guarantee (does not protect against another code path).

## R8 Import marker table

- **Decision**: an `imports` row written in the import transaction; the file is renamed after.
- **Why**: closes the crash window between commit and rename without a false "registry not empty"
  refusal on the next start.
- **Rejected**: rename first (a failed import loses the file's name); delete the file (irrecoverable).

## R9 Hashes as hex text, times as unix seconds

- **Decision**: `TEXT` hex for hashes and serials, `BIGINT` unix seconds for times.
- **Why**: avoids `BYTEA`/`BLOB` and timestamp type differences under `Any`; matches `last_seen`.
- **Rejected**: native `timestamptz` (no common `Any` mapping).

## R10 SQLite specifics

- **Decision**: bundled SQLite, WAL, file created 0600, `busy_timeout` 10 s, refuse if WAL is not
  granted.
- **Why**: no system library needed; WAL keeps readers and the writer apart; a non-WAL answer is the
  sign of a network filesystem (spec edge case).
- **Rejected**: `sqlite-unbundled` (needs a matching system library in the distroless image and the
  musl bundle); `rusqlite` beside `sqlx` (a second SQLite binding).
