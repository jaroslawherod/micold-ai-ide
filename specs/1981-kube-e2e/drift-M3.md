# Plan drift, M3

Recorded against `scripts/autopilot/plan-drift.sh specs/1981-kube-e2e M3` (merge 7bcb978d5).

## UNPLANNED
- `.gitattributes`, `mise.toml`: kube-e2e change attribute and the `e2e-kind` mise task (T019/T020 wiring).
- `Cargo.lock`, `crates/micold-e2e/src/lib.rs`, `crates/micold-e2e/tests/ops/smoke.rs`: the new e2e crate's library (`cluster_or_skip`) and first smoke suite.
- `docs/SUMMARY.md`: links the new developer page.
- `scripts/tests/install-kube-tools.test.sh`: test for the kind/kubectl downloads added to `install-kube-tools.sh`.

## MISSING
- `scripts/build-lock.sh`, `scripts/check-user-guide-updated.sh`: invoked, not changed. Paths a task names relative to the feature directory (`contracts/…`, `ci/…` values files, in-pod `registry.json`) or tools it only invokes; none is a file the milestone had to create at the repo root.
