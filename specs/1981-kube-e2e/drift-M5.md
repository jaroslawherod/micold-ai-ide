# Plan drift, M5

Recorded against `scripts/autopilot/plan-drift.sh specs/1981-kube-e2e M5` (merge 0b0c74a23).

## UNPLANNED
- `crates/micold-e2e/Cargo.toml`: `[[test]]` entries for the network and ui placeholder suites (T031).
- `scripts/e2e-kind.sh`: lists the new suites so the harness knows them (T032).
