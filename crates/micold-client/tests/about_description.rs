//! The About dialog describes the application, not a library inside it (feature 001, BUG-001 —
//! FR-009, contract C5).
//!
//! The description used to come from `AppMetadata::from_env()`, which lives in `micold-core` and
//! read `env!("CARGO_PKG_DESCRIPTION")` there. `env!` expands where it is *written*, so the dialog
//! showed `micold-core`'s manifest — "Render-free shared domain model…" — no matter which crate
//! called it. `metadata.rs`'s own tests could not see it: from inside `micold-core` the answer looks
//! right.
//!
//! So this reads what the dialog actually paints, from the client, and holds it against two
//! manifests: the client's `extended-description`, which is the copy written for a user (and the
//! one the `.deb` ships), and `micold-core`'s `description`, which is the property that broke.

mod support;

use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::sandbox::Sandbox;
use micold_core::env_include::EnvIncludeOutcome;
use support::layout as lay;

/// The value of `key = "…"` in a manifest — enough TOML for the two single-line strings read here.
fn manifest_string(manifest: &str, key: &str) -> String {
    let line = manifest
        .lines()
        .find(|l| l.trim_start().starts_with(&format!("{key} =")))
        .unwrap_or_else(|| panic!("the manifest has no `{key}` line"));
    let value = line.split_once('=').expect("a key = value line").1.trim();
    value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or_else(|| panic!("`{key}` is not a one-line string: {value}"))
        .to_string()
}

/// The application's one-line description: the first sentence of the client's
/// `extended-description`. One line is what FR-009 asks for, and the rest of that paragraph is a
/// second sentence about session CLIs that belongs in a package listing, not an identity box.
fn application_description() -> String {
    let extended = manifest_string(include_str!("../Cargo.toml"), "extended-description");
    let end = extended.find(". ").map_or(extended.len(), |i| i + 1);
    extended[..end].to_string()
}

/// Everything the About dialog paints, settled past its entrance.
fn painted_about() -> Vec<String> {
    let mut state = State::default();
    state.update(Message::Help(HelpMsg::AboutOpened));
    let mut renderer = lay::renderer();
    let element = micold_client::ui::view(
        &state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &Sandbox::default(),
    );
    lay::painted_text_settled(element, &mut renderer)
        .into_iter()
        .map(|t| t.content)
        .collect()
}

#[test]
fn about_paints_the_applications_own_description() {
    let expected = application_description();
    let painted = painted_about();
    assert!(
        painted.iter().any(|t| t == &expected),
        "the About dialog must describe the application as its package listing does — \
         {expected:?}; painted: {painted:?}"
    );
}

/// Version and license are workspace-wide, so they read the same from any crate — but they are now
/// resolved in the client, so the client is where they are held (FR-007, FR-008, SC-003).
#[test]
fn about_paints_the_applications_version_and_license() {
    let root = include_str!("../../../Cargo.toml");
    let painted = painted_about();
    for expected in [
        format!("Version {}", env!("CARGO_PKG_VERSION")),
        format!("License: {}", manifest_string(root, "license")),
    ] {
        assert!(
            painted.iter().any(|t| t == &expected),
            "the About dialog must paint {expected:?}; painted: {painted:?}"
        );
    }
    assert!(
        painted
            .iter()
            .any(|t| t == &format!("Version {}", manifest_string(root, "version"))),
        "the painted version must be the workspace's packaged version"
    );
}

#[test]
fn about_does_not_paint_a_library_crates_description() {
    let core = manifest_string(include_str!("../../micold-core/Cargo.toml"), "description");
    let client = manifest_string(include_str!("../Cargo.toml"), "description");
    let painted = painted_about();
    for (crate_name, description) in [("micold-core", core), ("micold-client", client)] {
        assert!(
            !painted.iter().any(|t| t == &description),
            "the About dialog paints {crate_name}'s package description ({description:?}) — words \
             written for the crate's maintainers, not for a user (BUG-001)"
        );
    }
}
