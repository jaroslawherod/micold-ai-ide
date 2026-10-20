//! T001 — shared test scaffolding for multi-project state (feature 008).
//!
//! Builds a `Workspace` with two-or-more projects, each holding sessions in caller-chosen
//! lifecycles, with no filesystem access. Included via `mod support;` from the feature-008
//! integration tests. Compiles under `cargo test --no-default-features` (pure core only).

#![allow(dead_code)]

/// Headless layout measurement (feature 019). Kept in its own module because it needs a renderer,
/// while everything else here is render-free core scaffolding.
pub mod covered_states;
pub mod jira_picker;
pub mod layout;
/// The modal table the overlay tests share (issue #700).
pub mod modals;
/// The sidebar's project at `/repo` (features 029, 803).
pub mod sidebar_fixture;
pub mod spawn_tree;
/// A `cdk` tooltip driven event by event (feature 038).
pub mod tooltip;

// Session and workspace builders: one copy, shared with the core tests (issue #716).
#[path = "../../../micold-core/tests/support/session_fixtures.rs"]
mod session_fixtures;
#[allow(unused_imports)]
pub use session_fixtures::*;

/// File `answer` under `key`, the way a reply to the request that named it lands (feature 033,
/// contract C2).
pub fn hold(
    state: &mut micold_client::app::State,
    key: micold_client::features::session::AvailabilityKey,
    answer: micold_client::features::session::CliAvailability,
) {
    state.session.availability.asked(0, key);
    state.session.availability.answered(0, answer);
}

/// File `answer` as the home directory's, the way a reply to a `cwd: None` request lands (feature
/// 033). Every row reads it while its own directory has no answer (FR-005).
pub fn hold_home(
    state: &mut micold_client::app::State,
    answer: micold_client::features::session::CliAvailability,
) {
    hold(
        state,
        micold_client::features::session::AvailabilityKey::Home,
        answer,
    );
}
