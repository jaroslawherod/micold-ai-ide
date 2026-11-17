# Plan drift, M2

Recorded against `scripts/autopilot/plan-drift.sh specs/1981-kube-e2e M2` (merge 7cd3811f7).

## UNPLANNED
- `crates/micold-daemon/tests/orchestrator/no_outbound.rs`: existing guard test updated for the ops listener (the only new listener is inbound, on `--ops-addr`).
- `deploy/helm/micold/{README.md,values.yaml,values.schema.json}`: `ops.enabled`/`ops.port` values and their docs, part of T013's probes.

## MISSING
- `contracts/ops-http.md` is the spec's own contract, not a repo file. `scripts/diff-coverage.py` is named as a tool to run; coverage was measured with cargo-llvm-cov (ledger Plan notes).
