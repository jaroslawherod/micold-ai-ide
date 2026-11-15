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
