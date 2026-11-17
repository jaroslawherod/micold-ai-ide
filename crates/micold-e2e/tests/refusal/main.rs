//! The `refusal` suite (#1981, FR-012, US3-4): an orchestrator with a corrupt registry refuses to
//! start. scripts/e2e-kind.sh reinstalls the chart with `ci/corrupt-registry.yaml` first; skips
//! unless run by it.

mod corrupt_registry;
