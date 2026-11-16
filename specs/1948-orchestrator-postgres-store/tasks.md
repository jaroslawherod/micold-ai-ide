---
description: "Task list for the PostgreSQL store for the orchestrator (SQLite for local mode)"
---

# Tasks: PostgreSQL store for the orchestrator (SQLite for local mode)

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md),
[data-model.md](data-model.md), [contracts/](contracts/), [quickstart.md](quickstart.md)

**Tests**: Constitution Principle I. Within each milestone a test task precedes the code it covers;
the shared suite is written red before the store that satisfies it.

**Local runs**: the cloud thread has no `mise` and no Postgres: use `scripts/gate.sh`; Postgres tests
skip locally with a message (CI sets `MICOLD_REQUIRE_POSTGRES=1`, where a skip fails). Iterate with
`cargo check` and one targeted test, never `cargo test --workspace`.

**Constitution**: Principle IV (network PostgreSQL) and VI (Linux only) deviations are justified in
plan.md, *Complexity Tracking* (jaro, #1897, #1949); a network PostgreSQL deployment waits on #1949
merging, nothing here does. `mise run gate` and CI's `linux-daemons` job prove the bundled SQLite and
`sqlx` still build for the existing targets.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallel (different files, no dependency on an unfinished task).
- **[USn]**: user story of spec.md (US1 survive a restart, US2 newer schema refused, US3 no private
  key, US4 config, US5 import, US6 tenant and owner columns). Setup and Foundational carry none.

## Phase 1: Setup

- [X] T001 Spike (research.md R2): add `sqlx` 0.8 (`default-features = false`, features
  `runtime-tokio`, `tls-rustls-ring-webpki`, `postgres`, `sqlite`, `any`, `migrate`) and
  `sha1 = { workspace = true }` to `crates/micold-daemon/Cargo.toml`; write a throwaway probe
  `crates/micold-daemon/tests/orchestrator/store_spike.rs` proving `$n` placeholders, `ON CONFLICT`,
  `RETURNING` and `sqlx::migrate!` over `Any` on SQLite (and on PostgreSQL when
  `MICOLD_TEST_POSTGRES_URL` is set). Record the pinned version and the outcome in research.md R2,
  also probe two concurrent `Migrator::run` calls on one SQLite file and record whether the second fails (plan.md, *Migrations*, retry rule); apply R2's fallback and correct plan.md if a statement form fails, then delete the probe.

## Phase 2: Foundational (registry contract, shared suite scaffold)

- [X] T002 [P] Write failing unit tests in `crates/micold-core/src/orchestrator/store.rs`
  (`#[cfg(test)]`): `StoreError::Unavailable { target, reason }`, `Refused { reason }` and
  `Schema(SchemaError)` (`DatabaseNewer { database: i64, build: i64 }`, `ChecksumChanged { version:
  i64, description }`, `Failed { version, reason }`) keep `Eq` and `Clone`, and their `Display`
  holds no URL; the default `upsert` replaces by `(tenant, id)` and keeps other records; the default
  `remove(&TenantId, &WireDaemonId)` returns true then false.
- [X] T003 [P] Create the shared-suite scaffold `crates/micold-daemon/tests/orchestrator/store_support.rs`
  (a `Backend` factory with `registry()` and `reopen()`; memory and `JsonRegistryStore` factories)
  and `store_suite.rs` registry cases, register both in `tests/orchestrator/main.rs`: upsert inserts
  then updates, `remove` true/false, `save` replaces the whole set across tenants in one call,
  `load` ordered by `(tenant_id, id)`, a non-default tenant record and the same id under two tenants
  survive `reopen`, two writers upserting different ids lose neither (FR-015, FR-001, US1).
- [X] T004 Implement T002 in `crates/micold-core/src/orchestrator/store.rs`: the three `StoreError`
  variants, `SchemaError`, and `RegistryStore::upsert` and `remove` with default bodies built on
  `load` and `save` (`JsonRegistryStore`, `MemoryRegistryStore` and the doubles in
  `crates/micold-daemon/src/orchestrator/mod.rs` compile unchanged); re-export in
  `crates/micold-core/src/orchestrator/mod.rs`; T002 and T003 green.

---

## Phase 3: User Story 1, in-memory stores for auth and audit (Priority: P1) 🎯 MVP

**Goal**: the three traits exist with in-memory implementations behind one suite (US1 scenario 3).

- [X] T005 [P] [US1] Failing unit tests in `crates/micold-core/src/orchestrator/auth.rs`
  (`#[cfg(test)]`): `UserId::new` rejects blank and the text `unowned`; `TokenHash::from_hex` accepts
  exactly "64 hex chars" (lower-case) and rejects any other length or character, `hash_of(token)` is
  the SHA-256 hex; `Role` and `Provider` round-trip `owner`/`operator`/`viewer` and
  `github`/`google` text.
- [X] T006 [P] [US1] Failing unit tests in `crates/micold-core/src/orchestrator/audit.rs`
  (`#[cfg(test)]`): `NewAuditEntry::new` accepts `detail` of "free text capped at 2 KiB" (2048 bytes)
  and returns `StoreError::Refused` above it; also correct the cap sentence in
  contracts/store-traits.md to say "refused above 2048 bytes".
- [X] T007 [US1] Auth cases in `crates/micold-daemon/tests/orchestrator/store_suite.rs` (red): create,
  read, list (removed users excluded), `set_role`, soft `remove_user` that revokes the user's refresh
  tokens in the same call; `link_identity` Refused when linked to another user; `redeem_invite` once
  (Refused when expired, unknown or redeemed); bootstrap `set`, `consume` (Refused on mismatch or
  consumed, `set_bootstrap` Refused after consume); `rotate_refresh` (Refused when the old is not
  active, reuse of a revoked one revokes the family); serials, revocations, `is_revoked`; enrollment
  redeemed once; two `AuthStore` values for different tenants on one backend see disjoint rows (US6
  scenario 2); every case ends with `reopen` (FR-002).
- [X] T008 [US1] Audit cases in `store_suite.rs` (red): `append` returns the entry with a strictly
  increasing `seq`, `recent(limit, before)` newest first by keyset, `by_actor`, tenant isolation,
  survives `reopen`.
- [X] T009 [US1] Implement `crates/micold-core/src/orchestrator/auth.rs`: `UserId`, `Role`,
  `Provider`, `TokenHash` ("64 hex chars"), `CertSerial` (hex text), `User`, `NewUser`, `Identity`,
  `Invite`, `NewInvite`, `Bootstrap`, `RefreshToken`, `Revocation`, `NewEnrollment` and the
  `AuthStore` trait exactly as contracts/store-traits.md; add `pub const UNOWNED: &str = "unowned"`
  to `record.rs`; T005 green.
- [X] T010 [P] [US1] Implement `crates/micold-core/src/orchestrator/audit.rs`: `AuditEntry`,
  `NewAuditEntry` (`new` enforces the 2048-byte cap), `AuditStore` trait; T006 green.
- [X] T011 [US1] `MemoryAuthStore` in `auth.rs` (`Mutex` over collections, tenant-keyed, same
  `Refused` and ordering rules, `fail(bool)` switch returning `Unavailable`).
- [X] T012 [US1] `MemoryAuditStore` in `audit.rs` with the same `fail(bool)` switch.
- [X] T013 [US1] Re-export the new types in `crates/micold-core/src/orchestrator/mod.rs`; add the memory
  auth and audit factories to `store_support.rs`; T007 and T008 green on memory.

---

## Phase 4: User Story 1, SQLite store (Priority: P1)

**Goal**: local mode keeps registry, users and audit entries across a restart (US1 scenario 2).

- [x] T014 [P] [US1] Failing tests in `crates/micold-daemon/tests/orchestrator/store_migrations.rs`
  (new, registered in `main.rs`): opening an empty SQLite path creates `orchestrator/` and the file with
  mode 0600, applies `0001_orchestrator_schema`, reports `journal_mode=wal`, `foreign_keys` on; a
  second open changes nothing; no `.down.sql` exists under `crates/micold-daemon/migrations/`; opening
  inside `#[tokio::test]` returns `StoreError::Unavailable` instead of panicking (the `block_on`
  guard) and dropping the last handle inside `#[tokio::test]` does not panic (Drop rule).
- [x] T015 [P] [US1] Add the SQLite factory (temp dir, reopen by reopening the file) to `store_support.rs`
  and run the registry cases of T003 on it (red).
- [x] T016 [US1] `crates/micold-daemon/migrations/sqlite/0001_orchestrator_schema.sql`: the whole v1
  schema of contracts/schema.md with `INTEGER PRIMARY KEY AUTOINCREMENT` for `audit_log.seq`, the two
  `RAISE(ABORT, 'audit_log is append-only')` triggers, every table's primary key leading with
  `tenant_id`, the `CHECK` constraints verbatim (`health IN ('online','offline','needs-update')`,
  `role IN ('owner','operator','viewer')`, `provider IN ('github','google')`, `email_verified IN
  (0,1)`), no `GRANT`.
- [x] T017 [US1] `crates/micold-daemon/src/orchestrator/db/migrate.rs`: `MIGRATOR` via
  `sqlx::migrate!("./migrations/sqlite")` and `run_forward`.
- [x] T018 [US1] `crates/micold-daemon/src/orchestrator/db/mod.rs`: `StoreHandle { rt:
  Option<Runtime>, pool }` with a private 2-worker multi-thread runtime, `block_on` that returns
  `Unavailable` under `Handle::try_current`, `Drop` using `shutdown_background()`, `acquire_timeout`
  5 s and connect timeout 5 s, SQLite open (`owner_only::ensure_dir`, file created 0600 before sqlx,
  `journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout` 10 s, failure naming
  the path when WAL is not granted), error mapping to `StoreError` with no URL in any string.
- [x] T019 [US1] `crates/micold-daemon/src/orchestrator/db/registry.rs`: `SqlRegistryStore` over
  `Arc<StoreHandle>`: `load` ordered by `(tenant_id, id)`, `save` (upsert each, delete the rest, one
  transaction), `upsert` (`ON CONFLICT` updating every column except `tenant_id` and
  `owner_user_id`), `remove`; `tenant_id` from `record.tenant`; `owner_user_id` written as `UNOWNED`
  on insert (M10 carries the real owner).
- [x] T020 [US1] Declare `pub mod db;` in `crates/micold-daemon/src/orchestrator/mod.rs` so the
  integration target reaches it; T014 and T015 green.

---

## Phase 5: User Story 1, SQL auth and audit stores (Priority: P1)

**Goal**: users and audit entries survive a restart in local mode (US1 scenario 2 complete).

- [x] T021 [P] [US1] Run the auth and audit cases (T007, T008) on the SQLite factory in
  `store_support.rs` (red).
- [x] T022 [P] [US1] `crates/micold-daemon/tests/orchestrator/store_scan.rs` (new, registered): raw
  `UPDATE` and `DELETE` on `audit_log` fail with `audit_log is append-only` (FR-011); the migration
  texts under `crates/micold-daemon/migrations/` contain no `GRANT`.
- [x] T023 [US1] `crates/micold-daemon/src/orchestrator/db/auth.rs`: `SqlAuthStore::new(db, tenant)`;
  every statement binds `tenant_id`; `redeem_invite` and `redeem_enrollment` are one compare-and-set
  `UPDATE … WHERE redeemed_at IS NULL` (and not expired) so exactly one caller wins;
  `rotate_refresh` revokes the old and inserts the new in one transaction; `remove_user` soft-removes
  and revokes in one transaction; `consume_bootstrap` likewise.
- [x] T024 [P] [US1] `crates/micold-daemon/src/orchestrator/db/audit.rs`: `SqlAuditStore::new(db,
  tenant)`: `append` returns the stored row with `seq`, `recent` keyset by `seq`, `by_actor`; no
  update or delete statement exists.
- [x] T025 [US1] Export both from `db/mod.rs`; T021 and T022 green on SQLite.

---

## Phase 6: User Story 1, PostgreSQL store and CI (Priority: P1)

**Goal**: the same suite passes on PostgreSQL in CI (US1 scenario 1; FR-012, SC-004).

- [x] T026 [P] [US1] Postgres factory in `store_support.rs`: reads `MICOLD_TEST_POSTGRES_URL`, each
  test creates and drops its own database; unset prints `skipped: MICOLD_TEST_POSTGRES_URL not set`,
  and fails instead when `MICOLD_REQUIRE_POSTGRES=1`; every shared-suite case runs on it.
- [x] T027 [P] [US1] Test in `store_migrations.rs`: `migrations/postgres/` and `migrations/sqlite/`
  list the same versions and descriptions; and (red, before T029) a store-level case that opening a
  refused local port returns `StoreError::Unavailable` carrying `host:port`, with no password in its
  `Display` (FR-016).
- [x] T028 [US1] `crates/micold-daemon/migrations/postgres/0001_orchestrator_schema.sql`: same schema,
  `BIGSERIAL PRIMARY KEY` for `audit_log.seq`, a `plpgsql` function plus `CREATE TRIGGER` raising
  `audit_log is append-only` on `BEFORE UPDATE OR DELETE`, no `GRANT`.
- [x] T029 [US1] `db/migrate.rs` and `db/mod.rs`: a `MIGRATOR` per backend, `PgConnectOptions` from
  the URL (with `ca_file`: `ssl_root_cert` and `VerifyFull`; without: `prefer`), `pool_size`,
  `install_default_drivers`; adjust any statement in `db/registry.rs`, `db/auth.rs`, `db/audit.rs`
  whose form differs per dialect; an unreachable server maps to `StoreError::Unavailable` carrying
  `host:port` and the driver cause with the URL stripped (T027's unreachable case green).
- [x] T030 [P] [US1] `.github/workflows/ci.yml`: `services: postgres` (`postgres:17`, user, password and
  db as job env, `pg_isready` health check, port 5432) on the `test` job, with
  `MICOLD_TEST_POSTGRES_URL` and `MICOLD_REQUIRE_POSTGRES=1`.
- [x] T031 [P] [US1] `docs/development/ci-pipeline.md`: note the Postgres service container and the
  `MICOLD_REQUIRE_POSTGRES` rule.

---

## Phase 7: User Story 2, a newer schema is refused (Priority: P1)

**Goal**: forward-only, checksummed, all-or-nothing migrations (US2; FR-003, FR-004, FR-005, FR-013, SC-002).

- [x] T032 [P] [US2] Seed fixtures `crates/micold-daemon/tests/fixtures/orchestrator/sqlite/v0001.sql`
  and `…/postgres/v0001.sql`: rows in every table at schema version 1.
- [x] T033 [US2] Failing tests in `store_migrations.rs`, SQLite and PostgreSQL: a database with a
  recorded version above the build's newest is refused, the message names both versions and the
  SQLite file's SHA-256 is identical before and after (PostgreSQL: `_sqlx_migrations` untouched);
  an older database applies the missing migrations in order; an edited migration is refused naming
  version and description; a migration that fails half way (a test-only `Migrator` over a temp
  directory) leaves the previous version; two threads opening one database at once apply each
  migration exactly once; the chain test seeds each `tests/fixtures/orchestrator/<dialect>/vNNNN.sql`
  at version N, upgrades to the newest and finds the data intact, and fails when a migration has no
  fixture.
- [x] T034 [US2] `db/migrate.rs`: `check_not_newer()` read-only (SQLite `read_only(true)` and no WAL
  pragma; PostgreSQL `READ ONLY` transaction) comparing `max(version)` of `_sqlx_migrations`, when
  the table exists, with `MIGRATOR.migrations.last()`; `MigrateError::DatabaseNewer { database,
  build }` and the `SchemaError` mapping with the texts of contracts/config.md, *Errors at start*.
- [x] T035 [US2] `db/mod.rs` `open()`: call `check_not_newer` before `run_forward`; `run_forward`
  retries once after re-reading `_sqlx_migrations` on a SQLite busy or "already exists" failure;
  map a checksum mismatch to `SchemaError::ChecksumChanged`; T033 green.

---

## Phase 8: User Story 3, no private key reaches the database (Priority: P1)

**Goal**: a scan of every column after the full suite finds no key material (US3; FR-006, SC-003).

- [x] T036 [P] [US3] Failing unit tests in `auth.rs`: `CertSerial::new` rejects non-hex text (a PEM
  block included); `TokenHash::from_hex` rejects a PEM block.
- [x] T037 [US3] Scan test in `store_scan.rs` and a table-and-column lister in `store_support.rs`
  (`information_schema` or `sqlite_master`): after the shared suite's writes plus one write per store
  method, cast every column of every table to text and fail on `-----BEGIN` or `PRIVATE KEY`; fail on
  a column name matching `private`, `secret` or ending `_key`, no exception list; both backends; a
  negative control seeds a free-text field with a PEM block and expects the scanner to fail.
- [x] T038 [US3] Make T036 pass in `auth.rs` (`CertSerial::new` validates hex); T037 green on both
  backends.

---

## Phase 9: User Story 4, database config and backend choice (Priority: P2)

**Goal**: `[database]` parses and checks, and the backend rule exists (US4 scenarios 1, 2, 4; FR-008, FR-009).

- [x] T039 [P] [US4] Failing tests in `crates/micold-core/src/daemon_config/tests.rs`: `[database]`
  with `url_file`, `ca_file`, `pool_size` parses and `deny_unknown_fields` rejects another key;
  `pool_size` default 5, outside 1..=64 fails naming the key; `[database]` without `url_file` fails
  naming `database.url_file`; `url_file` missing, unreadable or blank after trimming fails
  `database.url_file <path>: <state>`; a scheme other than `postgres` or `postgresql` fails naming
  only the scheme, never the rest of the URL; a missing `ca_file` fails `database.ca_file <path>:
  missing`; the resolved view shows `database.url_file = <path> (readable|missing|empty)`,
  `database.ca_file`, `database.pool_size`, never the URL, host or password.
- [x] T040 [P] [US4] Failing unit tests in `crates/micold-core/src/orchestrator/backend.rs` (new):
  `choose_store(Option<&DatabaseConfig>, BindScope, data_dir)` returns PostgreSQL when the table is
  present on any bind, SQLite at `<data_dir>/orchestrator/orchestrator.db` when absent and
  `LoopbackOnly`, and an error naming `database.url_file` when absent and `Network`.
- [x] T041 [P] [US4] `crates/micold-daemon/tests/orchestrator/store_config.rs` (new, registered): (red, before T042) a
  `config check` run on files with and without `[database]` passes or fails as T039 states, naming
  key and path.
- [x] T042 [US4] `DatabaseConfig { url_file, ca_file, pool_size }` in
  `crates/micold-core/src/daemon_config/mod.rs`, the checks in `files.rs`, the view in `resolved.rs`;
  T039 and T041 green.
- [x] T043 [US4] `backend.rs`: `BindScope`, `StoreChoice`, `choose_store`, re-exported in
  `orchestrator/mod.rs`; `config check` calls it with `LoopbackOnly` (until #1732 adds the bind key);
  T040 green.
- [x] T044 [P] [US4] `docs/user-guide/daemon-configuration.md`: `[database]` rows (`url_file`,
  `ca_file`, `pool_size`, what a missing key does).

---

## Phase 10: User Story 4, the orchestrator starts on the chosen store (Priority: P2)

**Goal**: start with only a data directory, or a PostgreSQL URL from config (US4 scenario 3; FR-009, FR-014, FR-016, SC-005).

- [x] T045 [P] [US4] `FlakyProxy` (a TCP proxy that can drop and resume) in `store_support.rs`.
- [x] T046 [US4] Failing tests in `store_slow.rs` (new, registered): an unreachable host fails the
  start with `orchestrator database <host>:<port> unreachable: <cause>` and the error text does not
  contain the password (grep); through `FlakyProxy`, a database lost after start makes store calls
  return `Unavailable` while the orchestrator keeps running, and the next call after the proxy
  resumes succeeds (FR-016); with the save thread parked inside a store call, other daemons'
  heartbeats are still answered inside 9 s (FR-014).
- [x] T047 [US4] Failing tests in `store_config.rs` and `store_suite.rs`: starting with only a data
  directory and no `[database]` creates `orchestrator/orchestrator.db` mode 0600, a linked daemon
  is listed (offline) after a restart (SC-005); `run_with_sql_store_starts_and_restores` starts
  `run_with` on a SQL store and `serve`'s load works; dropping the last handle inside async code
  does not panic.
- [x] T048 [US4] `db/mod.rs`: `open(config, bind, data_dir)` driven by `choose_store`, reading
  `url_file` (trimmed) and `ca_file`.
- [x] T049 [US4] `crates/micold-daemon/src/orchestrator/mod.rs`: `run()` opens the stores inside
  `spawn_blocking` before `serve`, passing `LoopbackOnly`; `serve` loads through
  `spawn_blocking`, the join error mapped to `io::Error`; T046 and T047 green.
- [x] T050 [P] [US4] `docs/user-guide/orchestrator-storage.md` (new): local mode versus PostgreSQL,
  the `[database]` keys, what is stored and never stored, newer-schema refusal; link it from
  `docs/user-guide/orchestrator-mode.md`.

---

## Phase 11: User Story 5, import an existing `registry.json` once (Priority: P2)

**Goal**: an operator's JSON registry moves into the database exactly once (US5; FR-010).

- [x] T051 [P] [US5] Failing tests in `crates/micold-daemon/tests/orchestrator/store_import.rs` (new,
  registered), both backends: an un-imported file with an empty registry imports every record under
  its own `tenant` and is renamed `registry.json.imported`; a second start imports nothing and
  changes nothing; a corrupt file fails the start naming it and imports nothing; rows present with
  no matching marker refuse, naming the file and the row count; rows plus a marker whose digest
  matches the file (a crash after the import transaction) only rename.
- [x] T052 [US5] `crates/micold-daemon/src/orchestrator/db/import.rs`: parse with
  `JsonRegistryStore::load`, count `daemons` rows over all tenants, one transaction inserting every
  record (owner `UNOWNED`) and an `imports` row `(tenant_id, source, digest, imported_at)` with
  `digest` the SHA-1 of the file, then rename (same directory, same mode).
- [x] T053 [US5] Call the import in `run()` after `open` and before `serve`'s load
  (`crates/micold-daemon/src/orchestrator/mod.rs`); T051 green.
- [x] T054 [P] [US5] `docs/user-guide/orchestrator-storage.md`: the import and the `.imported` suffix.

---

## Phase 12: User Story 6, ownership columns from the start (Priority: P3)

**Goal**: owners and tenants round-trip, and the first owner adopts pre-user records (US6; FR-007, FR-046).

- [x] T055 [P] [US6] Failing tests in `crates/micold-core/src/orchestrator/record.rs`: `OwnerId` is
  `unowned` or a `UserId`; `DaemonRecord.owner` defaults to `unowned` when absent, so an old
  `registry.json` loads.
- [x] T056 [P] [US6] Failing cases in `store_suite.rs`: an existing owner survives a relink `upsert`
  and a whole-list `save`; `reassign_unowned(&TenantId, &UserId)` moves every `unowned` row of the
  tenant and returns the count, a second call returns 0, another tenant's rows stay.
- [x] T057 [P] [US6] Test in `store_scan.rs`: every table except `_sqlx_migrations` has `tenant_id`,
  and `daemons`, `projects`, `sessions` have `owner_user_id`, on both backends (US6 scenario 1).
- [x] T058 [US6] `record.rs`: `OwnerId`, `DaemonRecord.owner` (serde default `unowned`); `store.rs`:
  `RegistryStore::reassign_unowned` with a default body over `load` and `save`; memory and JSON
  stores green.
- [x] T059 [US6] `db/registry.rs`: write `owner_user_id` from the record on insert only, and override
  `reassign_unowned` with one transaction updating `daemons`, `projects`, `sessions` of the tenant
  `WHERE owner_user_id = 'unowned'`; `db/import.rs` keeps an owner the file carries; T056 and T057
  green.
- [x] T060 [US6] Run quickstart §A and §B (`specs/1948-orchestrator-postgres-store/quickstart.md`);
  record each step's result there and fix the doc where it is wrong.

---

## Dependencies and order

Setup, then Foundational, then M1 to M10 in order: each milestone builds on the previous store layer
(memory, then SQLite, then PostgreSQL, then migration safety, then config and wiring, then import,
then owners). Within a milestone, test tasks precede the code they cover; `[P]` tasks touch disjoint
files.

## Implementation Strategy

MVP is M1 (traits, memory stores, shared suite), then M2 to M3 make local mode real, M4 adds
PostgreSQL and its CI proof. M5 to M7 harden (migrations, key scan, config). M8 wires the running
orchestrator, M9 imports, M10 adds owners. Every milestone merges to `main` with the suite green;
the orchestrator keeps using `JsonRegistryStore` until M8 wires the SQL store, so no milestone
changes behaviour users see before M8. User-guide pages arrive with M7, M8 and M9.

## Milestones

Each milestone merges to `main` on its own, through one PR (speckit-autopilot). The daemon crate is
not a `micold-user-facing` path, so the user-guide gate fires only on M7 to M9, which carry the
guide tasks; M1 to M6 and M10 use the `docs-not-needed` label.

### M1 — Stores for registry, auth and audit, in memory 🎯 MVP

- **Tasks**: T001–T013
- **Deliverable**: `RegistryStore` has per-record `upsert` and `remove`, `StoreError` has its new variants, `AuthStore` and `AuditStore` exist in `micold-core` with in-memory implementations, and one shared suite passes on the in-memory and JSON stores, tenant isolation included; the sqlx spike is recorded in research.md.
- **Satisfies**: US1 acceptance scenario 3, US6 scenario 2 (auth and audit); FR-001, FR-002, FR-006 (types), FR-007, FR-015
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite && scripts/build-lock.sh cargo test -p micold-core orchestrator::store && scripts/build-lock.sh cargo test -p micold-core orchestrator::auth && scripts/build-lock.sh cargo test -p micold-core orchestrator::audit`
- **Depends on**: —
- **Tier**: full

### M2 — SQLite handle, schema and registry store

- **Tasks**: T014–T020
- **Deliverable**: a SQLite file (owner-only, WAL) migrates to the v1 schema and holds the registry across reopen, passing the shared registry suite.
- **Satisfies**: US1 acceptance scenario 2 (registry); FR-002, FR-003 (forward, transactional), FR-007
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_migrations && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite`
- **Depends on**: M1
- **Tier**: full

### M3 — SQLite auth and audit stores

- **Tasks**: T021–T025
- **Deliverable**: users, invites, tokens, serials, enrollments and audit entries survive a reopen on SQLite, and the audit log rejects update and delete.
- **Satisfies**: US1 acceptance scenario 2; FR-002, FR-011
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_scan`
- **Depends on**: M1, M2
- **Tier**: full

### M4 — PostgreSQL store and CI service container

- **Tasks**: T026–T031
- **Deliverable**: the shared suite passes on PostgreSQL and CI runs it against a Postgres service container that fails, not skips, when missing.
- **Satisfies**: US1 acceptance scenario 1; FR-012, SC-004
- **Verify**: `MICOLD_TEST_POSTGRES_URL=postgres://… scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_`; CI `test` job green
- **Depends on**: M3
- **Tier**: full

### M5 — Migration safety

- **Tasks**: T032–T035
- **Deliverable**: a newer schema, an edited migration, a half-way failure and a concurrent start are each handled as the spec says, and every shipped schema version upgrades with data intact.
- **Satisfies**: US2 acceptance scenarios 1–2; FR-003, FR-004, FR-005, FR-013, SC-002
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_migrations` (SQLite; PostgreSQL with the URL)
- **Depends on**: M4
- **Tier**: full

### M6 — No private key in any table

- **Tasks**: T036–T038
- **Deliverable**: a scan of every column after exercising every store method finds no key material, and the scanner is proven able to fail.
- **Satisfies**: US3 acceptance scenarios 1–2; FR-006, SC-003
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_scan`
- **Depends on**: M4
- **Tier**: full

### M7 — `[database]` config and the backend rule

- **Tasks**: T039–T044
- **Deliverable**: `config check` accepts and validates `[database]`, shows paths and states only, and `choose_store` returns the table of contracts/config.md.
- **Satisfies**: US4 acceptance scenarios 1, 2, 4; FR-008, FR-009 (rule)
- **Verify**: `mise run test-core && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_config`
- **Depends on**: M4
- **Tier**: full

### M8 — The orchestrator starts on the chosen store

- **Tasks**: T045–T050
- **Deliverable**: `micold-daemon orchestrator` with no `[database]` creates the SQLite file and keeps its registry across a restart; with a `url_file` it uses PostgreSQL; an unreachable database fails the start without the password; a slow store call leaves heartbeats answered.
- **Satisfies**: US4 acceptance scenario 3; FR-009, FR-014, FR-016, SC-005
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_slow && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_config`; quickstart §B steps 1–2
- **Depends on**: M5, M7
- **Tier**: full

### M9 — Import `registry.json` once

- **Tasks**: T051–T054
- **Deliverable**: an un-imported `registry.json` is imported into an empty database registry at start and renamed `registry.json.imported`; corrupt files and non-empty registries refuse.
- **Satisfies**: US5 acceptance scenarios 1–4; FR-010
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_import`; quickstart §B step 3
- **Depends on**: M8
- **Tier**: full

### M10 — Ownership columns and adoption

- **Tasks**: T055–T060
- **Deliverable**: daemon records carry an owner that survives relinks, `reassign_unowned` hands pre-user records to the first owner, every data table has `tenant_id` and the three owned tables `owner_user_id`, and quickstart §A and §B are recorded.
- **Satisfies**: US6 acceptance scenarios 1–2; FR-007
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite && scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_scan`; quickstart §A and §B
- **Depends on**: M9
- **Tier**: full

---

### M11 — TDD remediation (tests and evidence only)

- **Tasks**: T061–T070
- **Deliverable**: tdd/verification.md F1–F12 cleared: FR-008 scan restored, local-mode save-failure assertion back, PostgreSQL skips reported as ignored, red evidence for every TEST_AFTER row, start-level US5.2/US5.4 tests, shared fixtures, pinned assertions, split eager cases, baseline recorded. No production behaviour change.
- **Satisfies**: verification.md F1–F12 (US1, US3, US4, US5)
- **Verify**: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator`; each task's own Verify
- **Depends on**: M10
- **Tier**: full

## Phase 13: TDD remediation

Source: [tdd/verification.md](tdd/verification.md), verdict **FAIL** at `40309293a`. The feature is
not done until T061 to T064 (F1 to F4) are cleared. F1 and F2 are weakened existing tests; F3 and F4
are the silent PostgreSQL skip and the missing red evidence. Fixes here are separate, explicit work:
the audit changed no test or source.

- [x] T061 [US4] F1 (HIGH): restore the FR-008 guard in `crates/micold-daemon/tests/orchestrator/no_outbound.rs:44-55,231-245`.
  Blank `.connect_with(` and `AnyConnection::connect(` under `orchestrator/db/` only where the receiver
  is a sqlx pool, options or connection type (not for any text match), or have `netns.rs` run the SQL
  store and prove it dials only the configured database. Verify:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator no_outbound` passes, and the
  same command fails when a `std::net::TcpStream::connect(` or a non-sqlx `.connect_with(` is added
  to `db/mod.rs` (add it, observe, revert).
- [x] T062 [US4] F2 (HIGH): put the removed assertion back for the default local mode. A failed save
  in a process, or in `run_with` with captured stderr, must produce `a registry change could not be saved`
  without PostgreSQL (`restart.rs:147`; `save.rs:91`). Verify:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator restart` passes with
  `MICOLD_TEST_POSTGRES_URL` unset, and fails when the `eprintln!` at `save.rs:91` is deleted.
- [x] T063 [US1] F3 (HIGH): make a skipped PostgreSQL case visible instead of `ok`. Replace the
  `let Some(..) else { return }` exits (`store_suite.rs:246,583,676`, `store_migrations.rs:326`,
  `store_scan.rs:43,199,226,233,248,271`, `store_slow.rs:55,75,201`, `store_import.rs:77`) with a
  mechanism that reports them as skipped or ignored, and keep CI running them. Verify:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator` without the URL reports the
  PostgreSQL cases as ignored (0 `ok` with zero assertions), and with
  `MICOLD_TEST_POSTGRES_URL=... MICOLD_REQUIRE_POSTGRES=1` it runs all of them (record the count from
  the CI `test` job in `tdd/cycle-log.md`).
- [x] T064 [US3] F4 (HIGH, process): add the missing M6 entry to `tdd/cycle-log.md` and give each
  `TEST_AFTER` behavior in `verification.md` rows 4, 6, 7, 9, 11, 13, 15 to 19, 24 to 26 a recorded red
  by deliberate mutant (one change, observe the named test fail, restore, re-run green). Priority
  order: row 17 (add a `private_key` column to a migration; `sqlite_holds_no_key_material` must fail),
  row 9 (drop an audit trigger; `a_raw_update_of_the_audit_log_fails` must fail), then rows 18, 19, 24
  and 25 (config validation, start wiring, `reassign_unowned`, owner on insert), which the audit did
  not sample. Verify: every row has a command and a failing test name in the log, and
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator && scripts/build-lock.sh
  cargo test -p micold-core --lib orchestrator::` is green after each restore.
- [x] T065 [US5] F5 (MED): add start-level tests in `crates/micold-daemon/tests/orchestrator/store_config.rs`
  for US5.2 (a second `run_configured` start imports nothing and leaves `registry.json.imported`) and
  US5.4 (an un-imported file beside a database that has rows fails the start naming the file and the
  row count). Verify: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator
  store_config` passes, and fails when `import.rs:82` is mutated to `rows > 1000000`.
- [x] T066 [P] F6 (MED): reuse the shared fixtures. `store_sql_errors.rs:14-68` and
  `store_import.rs:18-34` import `T`, `uid`, `hash`, `new_user`, `refresh_token`, `invite`, `daemon`
  and `keys` from `store_suite` and drop their copies. Verify:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_` passes and `mise run
  duplication` does not grow.
- [x] T067 [P] F7, F8 (MED): replace `!reason.is_empty()` and the `rsplit_once(':')` check
  (`store_migrations.rs:186,292,340,667`) with the pinned value (the host and port, the failing
  version), and restore the exact message in `store_config.rs:172-175`. Verify: the same targets pass,
  and each fails when the named text or port is changed in `db/mod.rs`.
- [x] T068 [P] F9 (MED): split the eager cases (`store_suite.rs:309,398,448,498`,
  `store_sql_errors.rs:85,173`) so each rule has a test name (refresh-token reuse, expired token,
  family revocation are separate cases). Verify:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_suite` passes with more
  cases per backend, and mutant 7 of `verification.md` fails exactly one of them.
- [x] T069 [P] F10, F11, F12 (LOW): in `store_slow.rs:175-191` assert on the test clock's ordering,
  not wall time; make the M5 "older database" and chain cases run a two-migration `Migrator` so
  ordering can fail (`store_migrations.rs`, M5 cases); stop calling `map_error` and positional `INSERT`
  literals from tests (`store_migrations.rs:215-238`). Verify: the same targets pass, and swapping the
  order of the two test migrations fails the older-database case.
- [x] T070 Record a pre-feature baseline in `tdd/cycle-log.md` (the log's "Baseline" says no local run
  before M1): run `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator` at
  `f5f8e812a~1` or note why it cannot be reproduced. Verify: the Baseline section holds a count.

## Phase 14: TDD remediation (re-verify at `cae46cf`)

Source: [tdd/verification.md](tdd/verification.md), verdict **FAIL**. The feature is not done until
T071 (N1) is cleared and the `TEST_AFTER` rows (F4) are either waived in `spec.md`/`autopilot.md` as
accepted history or the owner accepts the mutant reds recorded in M11. The audit changed no test or
source.

- [ ] T071 [US1] N1 (HIGH): give the migration retry bound a deterministic test. Make the migrator
  see a concurrent starter on demand (a hook or a second connection holding the lock between
  migrations) so `db/migrate.rs:71` is exercised every run. Verify:
  `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_migrations` passes, and
  fails on every one of 5 runs with `migrator.migrations.len() + 1` replaced by `0` (revert after).
- [ ] T072 [P] N2 (MED): in `store_slow.rs:179-189` replace `waited < 9` with a check against
  `LIVENESS_DEADLINE` that can fail (the daemon is not marked offline while the save is parked), or
  drop it. Verify: `... --test orchestrator store_slow` passes and fails when the store call blocks
  the heartbeat path.
- [ ] T073 [P] N3 (MED): split `store_sql_errors.rs:287,304` into one case per refusal and pin the
  `Refused` reason by a constant the store exports, not by substring. Verify:
  `... --test orchestrator store_sql_errors` passes with more cases.
- [ ] T074 [P] N4 (MED): replace the fixed 2 s in `store_config.rs:245-249` with a wait on the
  imported file or row. Verify: `... --test orchestrator store_config` passes 5 times in a row under
  load.
- [ ] T075 [P] N5, N6, N7 (LOW): document the password requirement at `store_support.rs:145` (or build
  the URL with one), name the columns in `store_sql_errors.rs:182-188,231`, and drop serde's wording
  from `store_config.rs:163-169`. Verify: `... --test orchestrator store_` passes.
