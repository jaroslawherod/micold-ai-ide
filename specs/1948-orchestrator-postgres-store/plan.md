# Implementation Plan: PostgreSQL store for the orchestrator (SQLite for local mode)

**Branch**: `claude/project-thread-ns6vcg` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: `specs/1948-orchestrator-postgres-store/spec.md` (issue #1948, part of #1722, M3). Parent
plan: `specs/1722-rn-client-k8s/plan.md` *Storage: PostgreSQL*, *Toward multi-user*, *Scalability path*.

## Summary

Three store traits (`RegistryStore`, `AuthStore`, `AuditStore`) live in `micold-core` with an
in-memory implementation each. `micold-daemon` gains one SQL implementation of all three over
`sqlx`'s `Any` driver, so PostgreSQL (network) and SQLite (local mode) share one body of queries and
differ only in their migration directory and connection set-up. Traits stay **synchronous**: the
SQL stores own a small private tokio runtime and `block_on` it, so the existing `RegistryStore`
contract, `RegistrySaver` and the `SaveWorker` thread of #1724 do not change, and the link tasks
never wait on a database call (FR-014). Migrations are `sqlx::migrate!` (forward only, transaction per
migration, checksum, lock), fronted by a read-only check that refuses a newer schema before anything
is written. Every row carries `tenant_id`; daemon, project and session rows carry `owner_user_id`.
Pre-user records get the sentinel owner `unowned`, which the first owner user adopts. A one-shot
import moves `registry.json` into an empty registry. CI gets a Postgres service container.

## Technical Context

**Language/Version**: Rust (MSRV 1.97, workspace edition 2021).

**Primary Dependencies**: new `sqlx` 0.8 (`default-features = false`, features `runtime-tokio`,
`tls-rustls-ring-webpki`, `postgres`, `sqlite`, `any`, `migrate`, `macros` (`sqlx::migrate!` lives behind it; spike T001)), in `micold-daemon`
only; `micold-core` stays free of it. `micold-core` gains `sha2` 0.10 (RustCrypto, like `sha1`) for `TokenHash::hash_of`. Existing: `tokio`, `serde`, `tracing`; `sha1` (already in `Cargo.lock`, used by `micold-core`) is added to
`micold-daemon`'s dependencies for the import digest, as `{ workspace = true }`. Task 1 is a spike that pins the exact `sqlx` version and proves `$n`
placeholders, `ON CONFLICT`, `RETURNING` and `Migrator` run on `Any` over both drivers (fallback in
research.md R2).

**Storage**: PostgreSQL 16+ (network), SQLite via bundled `libsqlite3` (local mode, WAL, 0600).

**Testing**: one shared suite (`crates/micold-daemon/tests/orchestrator/store_suite.rs`) run against
memory, SQLite and PostgreSQL; migration, schema-scan, import, config and FR-014 tests beside it.

**Target Platform**: Linux only (#1897). **Project Type**: Rust workspace (core + daemon crates).

**Performance Goals**: no store call on a link task; registry save off the link path within the
existing `SaveWorker`; a heartbeat answered inside 9 s while a store call is stuck.

**Constraints**: no secret in the database or in any output; local mode needs no server; one
orchestrator replica in v1 (a second may start, FR-003 covers its migration).

**Scale/Scope**: tens of daemons, a handful of users; tables sized for that, indexed by
`(tenant_id, …)`.

## Constitution Check

Constitution text read at `.specify/memory/constitution.md` (still the pre-#1949 text).

| Principle | Status |
|---|---|
| I. Test-First | PASS: `tasks` orders every test before its code; the shared suite is written red against the memory store first ([Test strategy](#test-strategy)). |
| II. Multi-Session | PASS: the store holds no session state; session rows are schema only, keyed by `(tenant_id, id)`, no cross-session column. Per-record upsert means concurrent writers do not clobber each other (FR-015). |
| III. Worktree Integration | N/A: no file or VCS operation. |
| IV. Local-First (NON-NEGOTIABLE) | **Deviation, justified** (Complexity Tracking). Local mode (SQLite file, no server, offline) satisfies the text. A PostgreSQL network deployment is opt-in and needs #1949 to amend the text before it ships. |
| V. Rust + iced Stack | PASS for this feature: Rust only, invalid states typed (`Role`, `TokenHash`, `OwnerId`). No UI, so no iced work (iced is being sunset). |
| VI. Cross-Platform Parity | **Deviation, justified** (Complexity Tracking): Linux only per #1897. |
| VII. Documentation | PASS: user-guide page `docs/user-guide/orchestrator-storage.md`, `[database]` rows in `docs/user-guide/daemon-configuration.md`, CI note in `docs/development/ci-pipeline.md`, same PR as the code. |
| VIII. Reusable UI | N/A: no UI. |

## Complexity Tracking

| Violation | Why needed | Simpler alternative rejected because |
|---|---|---|
| IV: an external PostgreSQL for network deployments | jaro 2026-11-14/15: network deployments need a shared relational store; the Kubernetes orchestrator has no volume | SQLite on a volume blocks any later second replica and needs a PVC; local mode keeps SQLite, so Principle IV holds wherever the app runs alone. #1949 amends the text; nothing here waits on it, a network deployment does |
| VI: Linux only | jaro 2026-11-14 (#1897) | Keeping macOS/Windows paths for a feature that only the Linux orchestrator uses is dead code |

## Project Structure

### Documentation (this feature)

```text
specs/1948-orchestrator-postgres-store/
├── spec.md  plan.md  research.md  data-model.md  quickstart.md
├── contracts/
│   ├── store-traits.md   # the three traits, error type, semantics
│   ├── schema.md         # tables, columns, constraints, per dialect differences
│   └── config.md         # [database], backend choice, config check rules
├── checklists/requirements.md
└── tasks.md              # next unit
```

### Source Code

```text
crates/micold-core/src/orchestrator/
├── store.rs       # EDIT: RegistryStore gains upsert, remove, reassign_unowned (default bodies);
│                  #   StoreError gains Unavailable, Refused variants; JsonRegistryStore, MemoryRegistryStore stay
├── record.rs      # EDIT: DaemonRecord.owner: OwnerId (serde default `unowned`); OwnerId, UNOWNED
├── auth.rs        # NEW: AuthStore trait + types (UserId, Role, User, Identity, Invite, TokenHash,
│                  #   RefreshToken, CertSerial, Revocation, Enrollment); MemoryAuthStore
├── audit.rs       # NEW: AuditStore trait, AuditEntry, NewAuditEntry; MemoryAuditStore
├── backend.rs     # NEW: BindScope, StoreChoice, choose_store() (pure FR-009 rule)
└── mod.rs         # EDIT: re-exports

crates/micold-core/src/daemon_config/
├── mod.rs         # EDIT: `[database]` -> DatabaseConfig { url_file, ca_file, pool_size }
├── files.rs       # EDIT: url_file / ca_file checks (key + path in the error, URL scheme check)
└── resolved.rs    # EDIT: database shown as paths and states only

crates/micold-daemon/
├── Cargo.toml     # EDIT: sqlx, sha1 (workspace)
├── migrations/
│   ├── postgres/0001_orchestrator_schema.sql   # NEW
│   └── sqlite/0001_orchestrator_schema.sql     # NEW
└── src/orchestrator/
    ├── mod.rs     # EDIT: run() opens the stores (spawn_blocking), imports registry.json, then serve(); serve loads via spawn_blocking
    └── db/
        ├── mod.rs      # NEW: StoreHandle { rt: Option<Runtime>, pool }; open(); connect options; error mapping; Drop
        ├── migrate.rs  # NEW: check_not_newer(), run_forward(); error type naming versions
        ├── registry.rs # NEW: impl RegistryStore for SqlRegistryStore
        ├── auth.rs     # NEW: impl AuthStore for SqlAuthStore
        ├── audit.rs    # NEW: impl AuditStore for SqlAuditStore
        └── import.rs   # NEW: registry.json one-shot import

crates/micold-daemon/tests/orchestrator/      # existing target; new modules registered in main.rs
├── store_suite.rs        # shared suite, three backends
├── store_support.rs      # backend factories, Postgres per-test database, FlakyProxy
├── store_migrations.rs   # newer / edited / chain / concurrent / half-way failure
├── store_scan.rs         # no private key (FR-006), schema shape (FR-007), audit append-only
├── store_import.rs       # registry.json import (FR-010)
├── store_config.rs       # config check, backend choice (FR-008, FR-009)
└── store_slow.rs         # FR-014 and FR-016
crates/micold-daemon/tests/fixtures/orchestrator/{postgres,sqlite}/v0001.sql   # chain-test seed data
.github/workflows/ci.yml       # EDIT: Postgres service container on the `test` job
docs/…                         # as in the Constitution Check, VII
```

**Structure Decision**: traits and in-memory implementations in `micold-core` (render-free, no
`sqlx`); the SQL implementation, migrations and the start-up wiring in `micold-daemon`, where the
plan of #1722 puts `migrations/`. New tests join the existing `tests/orchestrator/` target, so no
new test executable is linked.

## Design

### Trait shape (FR-001, FR-015, FR-014)

Synchronous, `Send + Sync`, `&self`. `RegistryStore` keeps `load` and `save` and gains
`upsert(&DaemonRecord)`, `remove(&TenantId, &WireDaemonId)` and `reassign_unowned(&TenantId, &UserId) -> u64`, each with a
default body built on `load` and `save`, so `JsonRegistryStore`, `MemoryRegistryStore` and the test
doubles in `orchestrator/mod.rs` compile unchanged. The SQL store overrides all three with one
statement each. Full signatures: [contracts/store-traits.md](contracts/store-traits.md).

The SQL stores hold an `Arc<StoreHandle>` (the daemon-side connection handle; the `[database]` config
type is `DatabaseConfig`) with a private multi-thread tokio runtime (2 workers) and the `AnyPool`. A
method is `self.db.block_on(async { … })`. Rules that make this safe:

- **Every call that runs in a runtime context goes through `spawn_blocking`.** `serve` is an `async fn`
  on the main multi-thread runtime, so the callers are: `serve`'s `store.load()` before it binds
  (wrapped: `tokio::task::spawn_blocking({ let s = Arc::clone(&store); move || s.load() })`, the join
  error mapped to `io::Error`); `StoreHandle::open` and the registry import in `run()`; and the later
  auth and audit callers (#1729, #1733, #1734), which do the same. `SaveWorker` is a plain thread and
  calls the store directly. `StoreHandle::block_on` detects a current tokio context
  (`Handle::try_current`) and returns `StoreError::Unavailable` with a clear message instead of
  panicking: a guard for a missed call site, not the supported path.
- **One place opens the database: `run()`**, inside `spawn_blocking`, from the `[database]` config and
  `choose_store`, before `serve`. It then imports `registry.json` in the same blocking task and hands
  `serve` the `Arc<dyn RegistryStore>`. `run_with` (tests) takes any store, so a test starts it on a
  SQL store (`store_suite.rs`: `run_with_sql_store_starts_and_restores`) to prove `serve`'s load works.
- **Drop rule.** `StoreHandle` holds the runtime as `Option<Runtime>`; its `Drop` takes it and calls
  `shutdown_background()`, so the last `Arc` dropping inside async code (the end of `serve`, a failed
  start) never panics with "Cannot drop a runtime in a context where blocking is not allowed". A test
  drops the last handle inside `#[tokio::test]`.
- Every pool has `acquire_timeout` 5 s and connect timeout 5 s, so a dead database returns an error
  in bounded time (FR-016); the pool reconnects on the next call, which is the "retry on the next
  call" of FR-016.

FR-014 holds structurally (links never call the store) and is proven by a test that parks the save
thread in a store call and shows other daemons' heartbeats answered (`store_slow.rs`).

### Tenant scope and owners (FR-007)

`AuthStore` and `AuditStore` values are built for one tenant (`SqlAuthStore::new(db, tenant)`); every
statement binds `tenant_id` in its `WHERE` or its inserted row. v1 builds the one `default` tenant
(`DEFAULT_TENANT`). Two such stores on one pool with different tenants see disjoint rows (US6 scenario 2).

**`RegistryStore` is not tenant-bound**, because `DaemonRecord.tenant` (#1724: the daemon announces it)
already carries each record's tenant and `Registry`/`SaveWorker` save every tenant's records in one
snapshot. The SQL store writes `tenant_id` from `record.tenant`; `load` returns every tenant's records
(ordered by `tenant_id, id`); `save` replaces the whole set across tenants in one transaction;
`upsert`, `remove` and `reassign_unowned` take or read the tenant explicitly. A record under another
tenant therefore survives a restart exactly as with `JsonRegistryStore`. The shared suite has a case
with a non-default tenant record, and the same id under two tenants. The `registry.json` import
writes each record under its own `tenant`.

**`owner_user_id` before any user exists** (the question the clarify unit left to the plan):

- `owner_user_id` is `TEXT NOT NULL` on `daemons`, `projects`, `sessions`, with **no foreign key**.
- Records written before a user exists (a daemon that links or enrolls, an imported `registry.json`
  record) get the sentinel `unowned` (`micold_core::orchestrator::UNOWNED`). It is the whole of FR-007
  for them: the column is filled, v1's single tenant has no second owner to confuse it with, and no
  fake user row is invented.
- The first user with role owner (created by #1733's bootstrap) calls
  `RegistryStore::reassign_unowned(owner)`, one transaction updating `daemons`, `projects`,
  `sessions` of the tenant `WHERE owner_user_id = 'unowned'`. The call is idempotent and returns the
  rows moved. #1733 owns the call site; this feature ships and tests the method.
- `upsert` and `save` never write `owner_user_id` on conflict: an existing owner survives a relink or
  a whole-list save. `DaemonRecord` carries `owner` (default `unowned`, serde default so an old
  `registry.json` loads) and the store writes it only on insert.
- A `UserId` that equals the sentinel text is refused at creation (`UserId::new` rejects `unowned`).

### Migrations (FR-003, FR-004, FR-005, SC-002)

Two directories because the dialects differ in a few types (`BIGSERIAL` against `INTEGER PRIMARY KEY
AUTOINCREMENT` for the audit sequence, the trigger syntax); both are applied by
`sqlx::migrate!("./migrations/<dialect>")`, selected by the connected backend. `0001` creates the whole
v1 schema ([contracts/schema.md](contracts/schema.md)). Rules enforced by sqlx and verified by tests:

- Forward only: no `.down.sql` files exist; a test fails the build if one appears.
- Each migration runs in its own transaction (a half-way failure leaves the previous version).
- PostgreSQL takes an advisory lock for the run, so two starters serialize and the second sees the
  migration applied; SQLite has no advisory lock in sqlx's migrator, so `run_forward` retries once on a busy or "already exists" failure after re-reading `_sqlx_migrations`, and the second starter then finds the migration applied (the spike, task 1, probes two concurrent runs); `busy_timeout` is 10 s.
- A recorded checksum that differs from the shipped file is refused, naming the migration.

**Newer schema**, which sqlx reports only as a missing-version error after it has already created its
table and taken locks: `migrate::check_not_newer` runs first, read-only. SQLite opens the file with
`read_only(true)` and no WAL pragma; PostgreSQL uses a `READ ONLY` transaction. It selects
`max(version)` from `_sqlx_migrations` when that table exists and compares with the build's newest
(`MIGRATOR.migrations.last()`). On a newer database it returns
`MigrateError::DatabaseNewer { database, build }` whose text names both versions; nothing was written,
so the SQLite file is byte-identical (a test hashes the file before and after, SC-002).

### Backend choice and config (FR-008, FR-009, FR-016)

`[database]` joins `DaemonConfig` beside `[orchestrator]` (`url_file`, `ca_file`, `pool_size`, default
5, range 1 to 64, `deny_unknown_fields`; the config type is `DatabaseConfig`). `config check` verifies the files exist and are non-empty,
reads the URL, trims it and checks the scheme is `postgres` or `postgresql`; its errors name the key
and the path, and for a bad scheme only the scheme, never the URL ([contracts/config.md](contracts/config.md)).
The resolved view shows paths and `readable`/`missing`/`empty` states, as `[orchestrator]` does.

The choice itself is the pure `choose_store(database: Option<&DatabaseConfig>, bind: BindScope, data_dir)`
in `micold-core`: table present, PostgreSQL on any bind; table absent and `LoopbackOnly`, SQLite
at `<data_dir>/orchestrator/orchestrator.db`; table absent and `Network`, an error naming
`database.url_file`. Today `run()` passes `LoopbackOnly` (the orchestrator listens on a Unix socket).
The orchestrator has no network listener or `bind` key yet (#1732 adds it), so `config check` cannot
learn the bind from the file: it calls the same function with the scope its caller supplies and
`config check` passes `LoopbackOnly` until #1732 adds the key. The network-bind clause of FR-009 is
therefore unit-tested at `choose_store` and wired end to end by #1732; recorded as a gap in
[Follow-ups](#follow-ups-and-known-gaps), not hidden.

SQLite set-up: create `orchestrator/` with `owner_only::ensure_dir`, create the file 0600 before sqlx
opens it, `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`. If the pragma does not report
`wal` (a network filesystem) or the open fails, the start fails naming the path. PostgreSQL set-up:
`PgConnectOptions` from the URL; with `ca_file`, `ssl_root_cert` and `ssl_mode=VerifyFull`; without it,
`prefer`. An unreachable or refused server maps to `StoreError::Unavailable` whose text carries host
and port and the driver cause with the URL stripped (the password never enters a string we format;
a test greps the error for the password).

### Registry import (FR-010)

At start, in `run()`'s blocking task after `StoreHandle::open` and before `serve` loads the registry:

1. `registry.json` absent: nothing. (`registry.json.imported` present: nothing.)
2. Present: parse it with `JsonRegistryStore::load`; a corrupt file fails the start with the existing
   `StoreError::Corrupt` naming it; nothing is imported.
3. Count the `daemons` rows (all tenants). More than zero and no matching import marker: refuse, naming the
   file and the count.
4. Otherwise one transaction inserts every record (owner `unowned` unless the file carries one) and
   an `imports` row `(tenant_id, source, digest, imported_at)` (tenant `default`: the file is one source), with `digest` the SHA-1 of the file.
5. Rename the file to `registry.json.imported` (same directory, same mode).

If the process dies between 4 and 5 the next start finds rows and a marker whose digest matches the file:
it only renames. That is why `imports` exists; without it the crash window would refuse a good import.

### Audit is append-only (FR-011)

`AuditStore` has `append` and read methods (`recent`, `by_actor`), no update or delete. In the
database both dialects get a trigger on `audit_log` that raises on `UPDATE` and `DELETE`, so the
guarantee holds for any role; docs say to grant the orchestrator's role `INSERT, SELECT` only
(the migration contains no `GRANT`, and a test greps the migration text for `GRANT` and for
`UPDATE`/`DELETE` privileges). Tests issue raw `UPDATE` and `DELETE` and expect the trigger's error.
Retention by partition drop stays out of scope (it needs the owner's privileges, not the app's).

### No private key in the database (FR-006, SC-003)

The types carry hashes (`TokenHash`, hex of SHA-256 computed by the caller, 64 chars), serials
(`CertSerial`, hex text) and public fields; none holds a `String` that could be a PEM block except
free text (`name`, `reason`, `detail`) which the scan test also covers. `store_scan.rs`, after running
the full shared suite's writes plus one write per store method, lists every table (`information_schema`
or `sqlite_master`) and every column, casts each to text and fails on `-----BEGIN` or `PRIVATE KEY`; it
also fails on a column name matching `private`, `secret`, or ending `_key` (no exception list). It runs
on both backends. A test seeds a free-text field with a PEM block to prove the scanner can fail.

### Test strategy

| Requirement | Layer | Test |
|---|---|---|
| FR-001, FR-002, FR-007, FR-015, US1, US6 | shared suite, 3 backends (memory, SQLite, Postgres), reopen between write and read | `store_suite.rs` |
| FR-003, FR-004, FR-005, SC-002 | daemon integration, SQLite and Postgres | `store_migrations.rs` (concurrent open, newer, edited, half-way, byte-identical file) |
| FR-013 | daemon integration | chain test over `migrations/<dialect>/*.sql`, seed `tests/fixtures/orchestrator/<dialect>/vNNNN.sql`; fails if a migration has no fixture |
| FR-006, SC-003, FR-011 | daemon integration | `store_scan.rs` |
| FR-008, FR-009 | core unit (`choose_store`, config parse and checks) plus daemon `config check` run | `store_config.rs`, `daemon_config` tests |
| FR-010 | daemon integration, both backends | `store_import.rs` (import, again, corrupt, non-empty, crash window) |
| FR-014, FR-016 | daemon integration | `store_slow.rs` (parked store, `FlakyProxy` in front of Postgres, unreachable host, password absent) |
| FR-012, SC-004 | CI | Postgres service container on the `test` job; `MICOLD_REQUIRE_POSTGRES=1` makes a missing URL a failure, not a skip |
| SC-005 | daemon integration | start with only a data directory, no `[database]` |
| Core pure rules (types, memory stores, `choose_store`, `UserId::new`) | core unit | `#[cfg(test)]` beside each module |

There is no client or geometry-gate layer and no visual pass: no UI. Real-Postgres tests read
`MICOLD_TEST_POSTGRES_URL`; each test creates and drops its own database. Cloud threads without Postgres
skip them locally (the skip prints why) and CI is the proof. No `kube_real` test is involved.

### CI (FR-012)

The `test` job gets `services: postgres` (`postgres:17`, user, password and db as job env, health check
`pg_isready`, port 5432 mapped) and `MICOLD_TEST_POSTGRES_URL` plus `MICOLD_REQUIRE_POSTGRES=1`. The
`linux-daemons` musl build already compiles `micold-daemon`, so it proves the bundled SQLite builds under
zigbuild. `scripts/gate.sh` sets nothing: local runs skip Postgres tests with a message, unless the URL is set.

### Requirement map

| FR | Where |
|---|---|
| FR-001 | [Trait shape](#trait-shape-fr-001-fr-015-fr-014), `store.rs`, `auth.rs`, `audit.rs`, `db/*` |
| FR-002 | `db/*`, `store_suite.rs` reopen step |
| FR-003–FR-005 | [Migrations](#migrations-fr-003-fr-004-fr-005-sc-002), `db/migrate.rs` |
| FR-006 | [No private key](#no-private-key-in-the-database-fr-006-sc-003), data-model.md |
| FR-007 | [Tenant scope and owners](#tenant-scope-and-owners-fr-007), schema.md |
| FR-008, FR-009 | [Backend choice and config](#backend-choice-and-config-fr-008-fr-009-fr-016), config.md |
| FR-010 | [Registry import](#registry-import-fr-010) |
| FR-011 | [Audit](#audit-is-append-only-fr-011) |
| FR-012, FR-013 | [CI](#ci-fr-012), chain test |
| FR-014 | Trait shape, `store_slow.rs` |
| FR-015 | `RegistryStore::upsert/remove`, SQL `ON CONFLICT` |
| FR-016 | Backend choice and config, pool timeouts, `store_slow.rs` |

### Follow-ups and known gaps

- FR-009's network-bind clause in `config check` waits for #1732's listener key; the rule and its
  unit test ship here.
- `SaveWorker` still saves whole snapshots (replace semantics, one replica). Moving it to
  `upsert`/`remove` diffs belongs to the issue that adds a second replica.
- Calling `reassign_unowned` belongs to #1733 (first owner user).
- Project and session writers come with later issues; only their tables ship.
