//! The image rows of the read-rules table (specs/806 contracts/daemon-settings.md, FR-016, SC-005):
//! what a stored `image` becomes, which stored settings stay as they were, and what is written.
//! Pure — no runtime, no network.

use micold_core::daemons::{DaemonRuntime, MigrationNotice};
use micold_core::sandbox::image::{ImageChoice, OwnImage};
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::{MilliCpus, NetworkPosture};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};

/// A v4 document with one `sandbox` block holding `image_fields` besides the unchanged settings.
fn document(image_fields: &str) -> String {
    format!(
        r#"{{
  "settings_version": 4,
  "daemon": {{
    "placement": "local_sandbox",
    "sandbox": {{
      "runtime": "podman",
      "network": "outbound",
      "survive_logout": true,
      "credentials": [],
      "budget": {{ "cpus_milli": 3000, "memory_bytes": 8589934592, "pids": 512, "storage_bytes": null }}{image_fields}
    }}
  }}
}}"#
    )
}

fn image(kind: &str, reference: &str) -> String {
    format!(r#", "image": {{ "kind": "{kind}", "path": null, "reference": "{reference}" }}"#)
}

fn load(document: &str) -> Settings {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(&path, document).unwrap();
    JsonFileSettingsStore::at(path).load().settings
}

fn notice(settings: &Settings) -> Option<MigrationNotice> {
    settings.daemons.iter().find_map(|e| match &e.runtime {
        DaemonRuntime::Container(c) => c.migration,
        _ => None,
    })
}

fn migrated(fields: &str) -> (ImageChoice, Option<MigrationNotice>) {
    let settings = load(&document(fields));
    (settings.daemon.sandbox.image.clone(), notice(&settings))
}

#[test]
fn no_image_is_the_basic_image_without_a_notice() {
    assert_eq!(migrated(""), (ImageChoice::Basic, None));
}

#[test]
fn the_released_repositories_move_to_basic_at_any_tag_or_digest() {
    let digest = format!("@sha256:{}", "0".repeat(64));
    for reference in [
        "ghcr.io/jaroslawherod/micold-daemon:0.9.0".to_string(),
        "ghcr.io/jaroslawherod/micold-daemon".to_string(),
        format!("ghcr.io/jaroslawherod/micold-daemon{digest}"),
        "ghcr.io/micold/micold-daemon:0.10.0".to_string(),
    ] {
        assert_eq!(
            migrated(&image("registry", &reference)),
            (
                ImageChoice::Basic,
                Some(MigrationNotice::MovedOffReleasedImage)
            ),
            "{reference}"
        );
    }
}

#[test]
fn another_registry_reference_is_my_own_image_with_the_same_reference() {
    let (choice, notice) = migrated(&image("registry", "registry.example.com/team/box:2"));
    assert_eq!(notice, None);
    assert!(matches!(choice, ImageChoice::Own(OwnImage::Reference(_))));
    assert_eq!(choice.reference(), "registry.example.com/team/box:2");
}

#[test]
fn an_imported_file_is_my_own_image_with_the_same_archive() {
    let fields =
        r#", "image": { "kind": "imported_file", "path": "/tmp/box.tar", "reference": "" }"#;
    let (choice, notice) = migrated(fields);
    assert_eq!(notice, None);
    match choice {
        ImageChoice::Own(OwnImage::Archive(path)) => {
            assert_eq!(path, std::path::PathBuf::from("/tmp/box.tar"));
        }
        other => panic!("an archive stays an archive: {other:?}"),
    }
}

#[test]
fn a_local_build_is_my_own_image_with_the_same_reference() {
    let (choice, notice) = migrated(&image("local_build", "micold-daemon:dev"));
    assert_eq!(notice, None);
    assert!(matches!(choice, ImageChoice::Own(OwnImage::Reference(_))));
    assert_eq!(choice.reference(), "micold-daemon:dev");
}

#[test]
fn a_stored_image_choice_wins_over_the_old_image() {
    let fields = format!(
        r#", "image_choice": "basic"{}"#,
        image("registry", "registry.example.com/team/box:2")
    );
    assert_eq!(migrated(&fields), (ImageChoice::Basic, None));
}

#[test]
fn nothing_but_the_image_changes() {
    let settings = load(&document(&image(
        "registry",
        "ghcr.io/jaroslawherod/micold-daemon:0.9.0",
    )));
    let profile = &settings.daemon.sandbox;
    assert_eq!(profile.runtime, RuntimeKind::Podman);
    assert_eq!(profile.network, NetworkPosture::Outbound);
    assert!(profile.survive_logout);
    assert!(profile.credentials.is_empty());
    assert_eq!(profile.budget.cpus_milli, Some(MilliCpus(3000)));
    assert_eq!(
        profile.budget.memory_bytes.map(|b| b.0),
        Some(8_589_934_592)
    );
    assert_eq!(profile.budget.pids, Some(512));
    for entry in settings.daemons.iter() {
        if let DaemonRuntime::Container(c) = &entry.runtime {
            // The migrated entry follows the same rules and keeps the same settings.
            assert_eq!(c.profile.image, ImageChoice::Basic);
            assert_eq!(c.profile.network, NetworkPosture::Outbound);
            assert!(c.profile.survive_logout);
            assert_eq!(c.migration, Some(MigrationNotice::MovedOffReleasedImage));
        }
    }
}

#[test]
fn the_written_file_has_image_choice_and_the_notice_and_never_image() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    std::fs::write(
        &path,
        document(&image(
            "registry",
            "ghcr.io/jaroslawherod/micold-daemon:0.9.0",
        )),
    )
    .unwrap();
    let store = JsonFileSettingsStore::at(path.clone());
    let loaded = store.load().settings;
    store.save(&loaded).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(written.contains("image_choice"), "{written}");
    assert!(written.contains("moved_off_released_image"), "{written}");
    assert!(!written.contains("\"image\":"), "{written}");

    // With no notice (a user's own image) the key is omitted.
    std::fs::write(&path, document("")).unwrap();
    let plain = store.load().settings;
    store.save(&plain).unwrap();
    let written = std::fs::read_to_string(&path).unwrap();
    assert!(!written.contains("migration"), "{written}");
}
