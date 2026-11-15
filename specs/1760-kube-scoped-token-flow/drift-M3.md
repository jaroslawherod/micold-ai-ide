# Plan drift, M3

| Path | Reason |
|---|---|
| `crates/micold-client/tests/kube_real_toolchain.rs` | UNPLANNED: the shared `Shell` helper moved from here into `kube_real_support/mod.rs` so `kube_real_scoped_token` can use it. |
| `.github/workflows/ci.yml` | MISSING, verified unchanged: job `kube-cluster` runs `scripts/test-kube.sh`, which exports `MICOLD_KUBE_TEST_REQUIRED=1` (line 27) and runs the `kube_real_` filter, so the new test is covered with no workflow edit. |
