//! End-to-end suites for the orchestrator on a kind cluster (#1981, FR-010).
//!
//! `scripts/e2e-kind.sh` creates the cluster, installs the chart and runs each suite under
//! `tests/` with `MICOLD_E2E=1`. Anywhere else (`cargo test --workspace`) every suite test skips
//! and prints why; [`env::cluster_or_skip`] is the one gate. [`cluster`] wraps the `kubectl` calls
//! the suites make. Environment: `specs/1981-kube-e2e/contracts/harness-env.md`.

pub mod cluster;
pub mod env;
