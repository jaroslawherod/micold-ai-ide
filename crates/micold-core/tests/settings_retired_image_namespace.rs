//! A settings file that pins the retired image namespace is repaired when it is read
//! (feature 027, FR-024). Pure — runs under `cargo test --no-default-features`. See
//! `specs/027-sandboxed-daemon-runtime/contracts/sandbox-settings-schema.md`.
//!
//! The gap this closes was found by running the app, not by reading it. Correcting `DEFAULT_IMAGE`
//! fixed every user with no `sandbox` block on disk — which is every user the unit tests model, and
//! nobody who had ever opened the sandbox section. Their file holds the old namespace as a value,
//! and a serde default cannot reach a field that is present.

use micold_core::sandbox::image::{ImageSourceKind, DEFAULT_IMAGE};
use micold_core::settings::{JsonFileSettingsStore, SettingsStore};

/// A v4 document as the app wrote it before 0.12.0 — trimmed to the fields under test, which the
/// schema's "missing means default" rule (S-2) makes a legal file.
fn document(kind: &str, reference: &str) -> String {
    format!(
        r#"{{
  "settings_version": 4,
  "daemon": {{
    "placement": "local_sandbox",
    "sandbox": {{
      "runtime": "docker",
      "network": "no_outbound",
      "survive_logout": false,
      "credentials": [],
      "image": {{ "kind": "{kind}", "path": null, "reference": "{reference}" }}
    }}
  }}
}}"#
    )
}

fn load(document: &str) -> micold_core::settings::Settings {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, document).unwrap();
    JsonFileSettingsStore::at(path).load().settings
}

#[test]
fn a_stored_reference_into_the_retired_namespace_loads_as_the_current_default() {
    let settings = load(&document("registry", "ghcr.io/micold/micold-daemon:0.10.0"));
    assert_eq!(settings.daemon.sandbox.image.reference, DEFAULT_IMAGE);
}

#[test]
fn a_stored_reference_the_user_chose_survives_the_load() {
    let chosen = "registry.example.com/team/micold-daemon:2.0.0";
    let settings = load(&document("registry", chosen));
    assert_eq!(settings.daemon.sandbox.image.reference, chosen);
}

#[test]
fn a_local_build_keeps_its_name_even_in_the_retired_namespace() {
    let settings = load(&document(
        "local_build",
        "ghcr.io/micold/micold-daemon:0.10.0",
    ));
    assert_eq!(
        settings.daemon.sandbox.image.kind,
        ImageSourceKind::LocalBuild
    );
    assert_eq!(
        settings.daemon.sandbox.image.reference, "ghcr.io/micold/micold-daemon:0.10.0",
        "an image already on this machine is not made unreachable to fix a pull nobody does"
    );
}
