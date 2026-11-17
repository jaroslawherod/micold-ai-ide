//! The `restart` suite (#1981, FR-012, US3-3): the registry survives the orchestrator's pod being
//! deleted. Runs first among the suites that link daemons: it expects a fresh, empty volume.
//! Skips unless run by scripts/e2e-kind.sh.

mod persistence;
