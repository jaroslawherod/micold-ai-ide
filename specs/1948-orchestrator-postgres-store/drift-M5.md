# Plan drift, M5

Recorded against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M5`.

## UNPLANNED (changed, named by no M5 task)

- `crates/micold-core/src/orchestrator/store.rs`: `SchemaError` texts aligned to `contracts/config.md` (T034).
- `crates/micold-daemon/Cargo.toml`, `Cargo.lock`: dev-dependency `sha2` for the SC-002 byte-identity check (T033).
- `crates/micold-daemon/tests/orchestrator/no_outbound.rs`: `DATABASE_LINES` allows the new `AnyConnection::connect(` call in `db/migrate.rs`.
- `crates/micold-daemon/tests/fixtures/orchestrator/{sqlite,postgres}/v0001.sql`: T032 names them; reported only as a path-matching artefact.

## MISSING

- `contracts/config.md`: named by T034 for its text only; no code path.

## Departures from tasks.md

- T035 says `run_forward` "retries once"; it retries up to once per migration plus one, so a
  second shipped migration still converges (review A round 1; cycle-log M5 cycle 2).
