# Cycle Log: PostgreSQL store for the orchestrator — #1948

Append only. Newest last. Every entry's `red` block is the evidence that the test existed and
failed before the implementation, or says why it had none.

## Baseline

- suite: not run locally before M1; `origin/main` at `f5f8e812a` is CI's green merge gate.

## M1 Cycle 1: StoreError variants and per-record defaults (T002, T004)

- test: `crates/micold-core/src/orchestrator/store.rs` tests
  `the_new_error_variants_keep_eq_and_clone_and_name_no_url`,
  `the_default_upsert_replaces_by_tenant_and_id_and_keeps_the_others`,
  `the_default_remove_returns_true_then_false` (new)
- red: `scripts/build-lock.sh cargo test -p micold-core --lib orchestrator::store` -> 13 compile
  errors (`no variant named Refused`, `cannot find type SchemaError`, `no method named upsert`)
- green: `Unavailable`, `Refused`, `Schema(SchemaError)`, `RegistryStore::upsert`/`remove` default
  bodies; 13 passed, 0 failed
- refactor: the default bodies became `upsert_into`/`remove_from` helpers shared with the
  `MemoryRegistryStore` and `JsonRegistryStore` overrides (below)
- notes: the two default-body tests run on a `LoadSaveOnly` double, so they keep testing the
  defaults after the stores override them

## M1 Cycle 2: token, id, role, provider, serial and detail types (T005, T006, T009, T010)

- test: `auth.rs` tests `a_user_id_rejects_blank_and_the_unowned_sentinel`,
  `a_token_hash_is_exactly_64_lower_case_hex_chars`, `hash_of_is_the_sha256_hex_of_the_token`,
  `role_and_provider_round_trip_their_stored_text`, `a_cert_serial_is_non_empty_hex_in_lower_case`;
  `audit.rs` test `a_detail_of_2048_bytes_is_accepted_and_one_more_is_refused` (new)
- red: `cargo test -p micold-core --lib orchestrator::au` -> 30 compile errors (`cannot find type
  UserId / TokenHash / Role / Provider / CertSerial / NewAuditEntry`)
- green: the types and traits in `auth.rs` and `audit.rs`, `UNOWNED` in `record.rs`; 6 passed
- refactor: none needed
- notes: `sha2` 0.10 added to `micold-core` for `TokenHash::hash_of`; the SHA-256("abc") vector pins it

## M1 Cycle 3: the shared suite on the in-memory and JSON stores (T003, T007, T008, T011, T012, T013)

- test: `crates/micold-daemon/tests/orchestrator/store_suite.rs`, 6 registry cases on `memory` and
  `json`, 7 auth and 4 audit cases on `memory` (new)
- red: none on arrival, test-after admission for the in-memory auth and audit stores and the
  registry overrides: the suite was written beside finished implementations. Deliberate mutants
  instead: (a) `at >= invite.expires_at` -> `at >` in `redeem_invite`:
  `store_suite::memory::an_invite_is_redeemed_once ... FAILED`; (b) the JSON store's write gate
  removed from `upsert`: `store_suite::json::two_writers_upserting_different_ids_lose_neither ...
  FAILED` in 3 of 3 runs. Both restored from copies.
- green: `MemoryAuthStore`, `MemoryAuditStore`, ordered `load`, `MemoryRegistryStore` upsert and
  remove under one lock, `JsonRegistryStore` write gate; 23 passed, 0 failed
- refactor: none needed
- notes: `load` now returns records ordered by `(tenant, id)` in both stores (the contract's rule;
  the suite's `load_is_ordered_by_tenant_then_id` case)

## M1 Cycle 4: sqlx spike (T001)

- test: throwaway `store_spike.rs` probe, deleted (not a kept test)
- red: `cargo test ... store_spike` -> `cannot find migrate in sqlx` (the `macros` feature was
  missing from the plan)
- green: with `macros`, both probes passed on SQLite and PostgreSQL 16; outcome in research.md R2
- refactor: none needed

## M2 Cycle 1: SQLite handle, schema and registry store (T014–T020)

- test: `crates/micold-daemon/tests/orchestrator/store_migrations.rs` (5 cases: file 0600 and dir
  0700, v1 applied once, WAL and foreign keys on; second open changes nothing; no `.down.sql`; open
  inside `#[tokio::test]` is `Unavailable`; dropping the last handle inside a runtime does not
  panic) and the `sqlite` module of `store_suite.rs` (6 registry cases, `SqliteBackend` in
  `store_support.rs`) (new)
- red: `cargo test -p micold-daemon --test orchestrator --no-run` -> `error[E0432]: unresolved
  import micold_daemon::orchestrator::db` (2 errors, the module did not exist)
- green: `db/mod.rs` (`StoreHandle`), `db/migrate.rs`, `db/registry.rs`, `migrations/sqlite/
  0001_orchestrator_schema.sql`; `cargo test ... store_` 34 passed. The first green run failed two
  cases for real reasons, fixed: `Handle::try_current` is also `Ok` on a `spawn_blocking` thread,
  so the plan's guard would have refused the supported path; `block_on` now catches tokio's own
  "within a runtime" refusal (`run_on`). And an early failure in `open_sqlite` dropped the runtime
  inside async code; it now calls `shutdown_background()`.
- refactor: none needed
- notes: `load` orders with `ORDER BY tenant_id, id` (follow-up); sqlx's `Any` takes the PRAGMAs per
  connection through `after_connect`, with `WAL` verified from the pragma's answer.

## M2 Cycle 2: review A fixes (T014, T018)

- test: `store_migrations.rs` `a_directory_with_a_space_and_non_ascii_letters_opens`,
  `an_existing_loose_file_is_tightened_to_owner_only` (new)
- red: shown after the fact by reverting each fix in turn (the tests were written after the code, test-after admission), then restoring it:
  (a) `sqlite_url` with the raw path instead of percent-encoding:
  `a_directory_with_a_space_and_non_ascii_letters_opens ... FAILED`, `Unavailable { target:
  ".../josé data?x#y/orchestrator/orchestrator.db", reason: "error with configuration: unknown query
  parameter `x#y/orchestrator/orchestrator.db?mode` while parsing connection URL" }`;
  (b) `create_owner_only` without `set_permissions` on an existing file:
  `an_existing_loose_file_is_tightened_to_owner_only ... FAILED`, `assertion left == right failed
  left: 420 right: 384` (0644 stays, 0600 expected). Both fixes restored byte for byte.
- green: 36 passed; both tests pass again with the fixes restored
- refactor: `kind` written with `name_of` like `health`; `save` opens with a no-op `DELETE` so it holds
  the write lock from its start; `map_error` maps only constraint violations to `Refused`;
  `run_forward` retries only an "already exists", "locked" or "busy" failure; `SqliteBackend::reopen`
  drops the old handle first

## M2 Cycle 3: error-path tests added after the fact (CI diff coverage)

Seven tests in `store_migrations.rs` (see drift-M2.md) cover the schema-error mapping, unopenable
paths, `map_error` and the registry's corrupt-row and driver-error branches. They were written
after the code, for the diff-coverage gate, against behaviour that already existed: each passed on
first run and none was shown red by breaking its branch.

## M3 Cycle 1: SQL auth and audit stores on SQLite (T021-T025)

- test: `store_suite.rs` `sqlite` module now runs `auth_tests!` (7 cases) and `audit_tests!` (4 cases)
  through `SqliteBackend: FullBackend`; `store_scan.rs` `a_raw_update_of_the_audit_log_fails`,
  `a_raw_delete_from_the_audit_log_fails`, `no_migration_text_grants_anything` (new)
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator --no-run` ->
  `error[E0432]: unresolved imports micold_daemon::orchestrator::db::SqlAuditStore, SqlAuthStore`
  (2 errors; the stores did not exist). The `store_scan` cases need no new code (the migration's
  triggers exist since M2), so they were green on first run and pin FR-011.
- green: `db/auth.rs`, `db/audit.rs`, exports; `cargo test ... store_` 57 passed, 1 failed (below)
- refactor: `StoreHandle::run`, `begin_write`, `to_i64`/`to_u64` shared with `registry.rs`

## M3 Cycle 2: redeem_invite refuses an unknown redeeming user (M1 review B F2)

- test: `an_invite_is_redeemed_once` (shared suite) creates its users and asserts
  `redeem_invite(tok-1, ghost)` is `Refused`
- red: `store_suite::memory::an_invite_is_redeemed_once ... FAILED`: `expected Refused, got
  Ok(Invite { id: "i1", ... redeemed_by: Some(UserId("ghost")), redeemed_at: Some(1011) })`
  (the SQL backend already passed: it checks the user)
- green: `MemoryAuthStore::redeem_invite` checks the user; contract line and trait doc updated;
  66 passed
- refactor: none

## M3 Cycle 3: failure arms (CI diff coverage), `store_sql_errors.rs`

Nine tests (see drift-M3.md) written after the code, for the diff-coverage gate: they passed on first
run and none was shown red by breaking its branch. The behaviours they pin (rollback of a failed
rotation, one CAS winner, `Corrupt` on unreadable rows, `Unavailable` on lost tables) were each
exercised by reading the code path, not by a failing run first. Test-after admission, as in M2 cycle 3.
- review A MINOR fixed after the fact: `put_refresh` wrote `revoked_at` as NULL where the memory store
  keeps the token as given; now bound. `a_stored_refresh_token_keeps_the_revocation_it_arrived_with`
  was written with the fix (test-after, no red run).

## M4 Cycle 1: PostgreSQL store (T026-T029)

- test: `store_support.rs` `PostgresDatabase` / `PostgresBackend` (own database per case, skip or
  fail by `MICOLD_REQUIRE_POSTGRES`); `store_suite.rs` `postgres` module (17 cases);
  `store_migrations.rs` `both_dialects_list_the_same_versions_and_descriptions`,
  `an_unreachable_postgres_server_is_unavailable_naming_host_and_port_not_the_password`,
  `opening_postgres_applies_the_v1_schema_once`; `store_scan.rs` PostgreSQL append-only case
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator --no-run` ->
  `error[E0599]: no associated function or constant named open_postgres found for struct StoreHandle`
  (4 call sites); the dialect-parity case had no `migrations/postgres/` directory
- green: `migrations/postgres/0001_orchestrator_schema.sql`, `POSTGRES_MIGRATOR`,
  `StoreHandle::open_postgres`; with `MICOLD_TEST_POSTGRES_URL` on a local PostgreSQL 16 and
  `MICOLD_REQUIRE_POSTGRES=1`: 20 postgres cases passed; `store_` 90 passed without it (skips).
  No query in `db/registry.rs`, `db/auth.rs`, `db/audit.rs` needed a dialect change (`$N`
  placeholders, `ON CONFLICT`, `RETURNING` are common to both).
- refactor: `StoreHandle::start` shared by both opens; `run_forward` takes its migrator.
- error-arm cases (`a_url_that_does_not_parse…`, `a_missing_ca_file…`, `a_present_ca_file…`) were
  written with the code (test-after), for diff coverage.

## M5 Cycle 1: migration safety (T032-T035)

- test: fixtures `tests/fixtures/orchestrator/{sqlite,postgres}/v0001.sql` (a row in every table);
  `store_migrations.rs` M5 cases, each over SQLite and PostgreSQL:
  `a_newer_database_is_refused_naming_both_versions_and_left_unchanged`,
  `an_older_database_applies_the_missing_migrations_in_order`,
  `an_edited_migration_is_refused_naming_version_and_description`,
  `a_migration_that_fails_half_way_leaves_the_previous_version`,
  `two_starters_on_one_database_apply_each_migration_once`,
  `every_shipped_version_upgrades_to_the_newest_with_its_data_intact`. The pre-M5
  `a_database_from_a_newer_build_is_a_failed_schema_error_naming_the_version` is replaced by the first.
- red: `scripts/build-lock.sh cargo test -p micold-daemon --test orchestrator store_migrations`
  (PostgreSQL 16 local, `MICOLD_REQUIRE_POSTGRES=1`): 4 failed for the expected reasons —
  newer: `Failed { version: 0, reason: "migration 999 was previously applied but is missing…" }`;
  half way: `version` 0, not 2; edited: old text `migration 1 (orchestrator schema) differs…`;
  two starters: `while executing migration 1: … table daemons already exists` (the retry matched
  only `Execute`, not `ExecuteMigration`). The older-database and chain cases passed at once (v1 is
  the only shipped version; they pin behaviour sqlx already gives).
- green: `migrate::check_not_newer` (read-only SQLite connection before the WAL pool; PostgreSQL
  `READ ONLY` transaction), called by `StoreHandle::start` before `run_forward`; `run_forward`
  retries on `ExecuteMigration` too; `ExecuteMigration`/`VersionMissing`/`Dirty` carry their version
  in `Failed`; `SchemaError` texts are those of contracts/config.md, *Errors at start*. 25 passed;
  the race case 5 of 5 runs.
- refactor: none

## M5 Cycle 2: review A fixes

- red: the newer-schema case's SQLite fingerprint now lists the directory; with the fix reverted it
  failed `["orchestrator.db", "orchestrator.db-shm", "orchestrator.db-wal"]` vs `["orchestrator.db"]`
  (the read-only check of a WAL file creates the sidecars).
- green: a refused check removes the sidecars it created; `run_forward` retries once per migration
  plus one (one retry was enough only while one migration ships); a schema error from the run
  re-checks for a newer database (PostgreSQL's lock is taken only in the run); `sqlite_url` takes
  the mode; `Dirty`/`VersionMissing` no longer claim "rolled back" with a version. The retry bound
  and the re-check were written with the fix (test-after: one shipped migration cannot race twice).
- refactor: `shipped_migrations` shared with `migration_names`; `newest_shipped` per dialect.

## M7 Cycle 1: `[database]` config and the backend rule

- T039–T041 (core config tests, `choose_store` tests, `store_config` integration): test-after. The
  types and checks were written in the same pass as the tests; red was not observed.
- green: `DatabaseConfig`, rules D1–D5, the resolved view, `orchestrator::backend::choose_store`,
  `config check` wiring. 13 core cases and 4 `store_config` cases pass; the full gate is green.
- refactor: none
