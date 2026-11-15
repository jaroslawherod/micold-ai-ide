# Plan drift, M3

Recorded against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M3`.

## UNPLANNED (changed, named by no M3 task)

- `crates/micold-core/src/orchestrator/auth.rs`: `MemoryAuthStore::redeem_invite` refuses an unknown
  redeeming user, and the trait doc says so (M1 review B F2; the follow-up allowed either this or a
  contract note, and the SQL store refuses, so the in-memory one must too for the shared suite).
- `crates/micold-daemon/src/orchestrator/db/registry.rs`: uses the new shared `StoreHandle::run`,
  `begin_write` and `to_i64` instead of its own copies (no behaviour change). `db/mod.rs` gains those
  helpers (T025 names only the exports).
- `crates/micold-daemon/tests/orchestrator/main.rs`: registers `store_scan` (T022) and
  `store_sql_errors`.
- `crates/micold-daemon/tests/orchestrator/store_sql_errors.rs` (new, UNPLANNED): the failure arms for
  CI diff coverage: every auth method and the audit store over dropped tables is `Unavailable`; rows
  that do not read back are `Corrupt`; a failed rotation keeps the old token active; each bootstrap,
  invite and enrollment refusal names its cause; six threads redeeming one invite leave one winner; a
  call from async code is `Unavailable`.
- `crates/micold-daemon/tests/orchestrator/store_suite.rs` (changed): the `sqlite` module also runs
  `auth_tests!` and `audit_tests!` (T021); `an_invite_is_redeemed_once` now creates its users and
  asserts an unknown redeeming user is refused (the case failed on the memory backend first).
  `store_support.rs` (`FullBackend` for `SqliteBackend`) is T021's factory.
- `specs/1948-orchestrator-postgres-store/contracts/store-traits.md`: `redeem_invite` line says
  `Refused` also when `by` is not a user.
- `crates/micold-daemon/src/orchestrator/db/auth.rs`, `audit.rs` (T023, T024) and
  `tests/orchestrator/store_scan.rs` (T022) are planned; the script lists them only by its path rules.

## Departures from plan.md

- `rotate_refresh` on a revoked token commits the family revocation on purpose (that is the contract:
  reuse revokes the family) and then returns `Refused`; every other refusal rolls its transaction back.
- Every multi-statement store call opens its transaction with a no-op write (`begin_write`), as
  `SqlRegistryStore::save` already did, so SQLite never refuses a read-to-write upgrade.
