# Plan drift, M1

Recorded against `scripts/autopilot/plan-drift.sh specs/1948-orchestrator-postgres-store M1`.

## UNPLANNED (changed, named by no M1 task)

- `Cargo.toml`: adds `sha2 = "0.10"` to the workspace dependencies. T005 requires `hash_of(token)`
  to be the SHA-256 hex, and `micold-core` had only `sha1`; `sha2` is the same RustCrypto family.
- `crates/micold-core/Cargo.toml`: `sha2 = { workspace = true }` for `TokenHash::hash_of`.
- `Cargo.lock`: the resolved `sqlx` 0.8.6 tree (T001) and `sha2`.
- The spike found `sqlx::migrate!` needs the `macros` feature, so `crates/micold-daemon/Cargo.toml`
  carries `macros` beside T001's list; plan.md's feature list was corrected (research.md R2).
- Not flagged by the script but beyond the task text: `JsonRegistryStore` and `MemoryRegistryStore`
  override `upsert` and `remove` (T004 says the defaults compile unchanged; they do) so that two
  writers upserting different ids lose neither (T003's case), and both stores' `load` now orders by
  `(tenant, id)` (contract rule). Both are in `crates/micold-core/src/orchestrator/store.rs`.

## MISSING (planned, no change in the diff)

- `crates/micold-daemon/tests/orchestrator/store_spike.rs`: the T001 probe, created and deleted
  inside the milestone, so the diff never shows it.
- `crates/micold-daemon/src/orchestrator/mod.rs`: T004 says its doubles compile unchanged; they do,
  so the file is untouched.
- `research.md` and `contracts/store-traits.md`: under `specs/`, changed (R2 outcome; the 2048-byte
  sentence), which the script does not compare.
