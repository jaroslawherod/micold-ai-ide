# Plan drift, M4

Recorded against `scripts/autopilot/plan-drift.sh specs/1981-kube-e2e M4` (merge a59a74729).

## UNPLANNED
- `crates/micold-e2e/tests/{ops/endpoints.rs,ops/security.rs,refusal/corrupt_registry.rs,restart/persistence.rs}`: the suite files T023–T030 describe by behaviour rather than path.
- `deploy/helm/micold/{README.md,values.yaml,values.schema.json,templates/_helpers.tpl,templates/deployment.yaml,templates/pvc.yaml}`: persistence toggle and seed hooks the restart and refusal suites need.
- `docs/development/kube-e2e.md`: suite documentation.

## MISSING
- Paths a task names relative to the feature directory (`contracts/…`, `ci/…` values files, in-pod `registry.json`) or tools it only invokes; none is a file the milestone had to create at the repo root.
