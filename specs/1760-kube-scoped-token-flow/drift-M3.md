# Plan drift, M3

| Path | Reason |
|---|---|
| `crates/micold-client/tests/kube_real_toolchain.rs` | UNPLANNED: the shared `Shell` helper moved from here into `kube_real_support/mod.rs` so `kube_real_scoped_token` can use it. |
| `.github/workflows/ci.yml` | MISSING, verified unchanged: job `kube-cluster` runs `scripts/test-kube.sh`, which exports `MICOLD_KUBE_TEST_REQUIRED=1` (line 27) and runs the `kube_real_` filter, so the new test is covered with no workflow edit. |
| `crates/micold-core/src/daemon_config/files.rs`, `crates/micold-core/src/daemon_config/tests.rs` | UNPLANNED, production fix found by the kind test: V14 refused the pod's own startup Secret files, which mount 0440 (`defaultMode`, `fsGroup` = the daemon uid). For `kind = "kubernetes"` V14 now counts only other bits (0o007); host and container keep 0o077. Test `v14_a_group_readable_secret_passes_in_a_pod_only`. |
| `specs/1728-orchestrator-config-section/contracts/config-check.md` | V14 row notes the pod exception (Review A r2 F1). |
