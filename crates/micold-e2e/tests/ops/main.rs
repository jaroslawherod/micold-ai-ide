//! The `ops` suite (#1981, FR-010, US2-4): the deployed orchestrator's ops listener, reached
//! through `kubectl port-forward` to its Service, and the pod the chart rendered. Skips unless
//! run by scripts/e2e-kind.sh.

mod endpoints;
mod security;
mod smoke;
