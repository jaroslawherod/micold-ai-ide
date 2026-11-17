//! The `link` suite (#1981, FR-012, US3-1, US3-2): a test daemon in the orchestrator's pod links
//! over the shared socket and the orchestrator lists it. Names here are `link-*` and no test
//! assumes an empty registry: `restart` runs first and owns that assumption. Skips unless run by
//! scripts/e2e-kind.sh.

mod concurrent;
mod register;

use std::time::Duration;

/// Offline shows within the 9 s liveness deadline plus a check interval; this is the margin.
pub const SETTLE: Duration = Duration::from_secs(60);
