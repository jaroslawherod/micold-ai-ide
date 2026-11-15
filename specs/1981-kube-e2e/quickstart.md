# Quickstart (#1981)

Part A (no cluster): `mise run chart-check` (helm lint, kubeconform, conformance); `cargo test -p micold-daemon orchestrator`.
Part B (Linux with docker, kind, kubectl, helm): `mise run e2e-kind`; one suite: `MICOLD_E2E_SUITES=ops mise run e2e-kind`; keep the cluster: `MICOLD_E2E_KEEP=1 mise run e2e-kind`. Logs on failure: `target/e2e-logs/micold-e2e/`.
Adding a suite: create `crates/micold-e2e/tests/<name>/`, add `<name>` to the harness suite list and a values file if the chart needs one.
