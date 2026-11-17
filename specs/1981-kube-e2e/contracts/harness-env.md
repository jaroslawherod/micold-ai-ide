# Harness environment contract

| Variable | Default | Meaning |
|---|---|---|
| `MICOLD_E2E` | unset | `1` set by the harness; tests skip with a printed reason when unset |
| `MICOLD_E2E_REQUIRED` | unset | `1`: a skip becomes a failure |
| `MICOLD_E2E_KEEP` | unset | `1`: keep cluster and namespace |
| `MICOLD_E2E_CLUSTER` | `micold-e2e` | kind cluster name |
| `MICOLD_E2E_SUITES` | all enabled | comma list of `ops,link,restart,refusal,network,ui` |
| `MICOLD_E2E_MATRIX` | `1` with the default suite selection, else `0` | `1`/`0` forces the seeded-breakage matrix (M4) on or off |
| `MICOLD_E2E_NAMESPACE` | `micold-e2e` | passed to tests by the harness |
| `MICOLD_E2E_LOG_DIR` | `target/e2e-logs/<cluster>` | failure logs, uploaded by CI |

Exit status is the suite's; missing tool exits 1 naming it; non-Linux exits 0 with a skip line.
