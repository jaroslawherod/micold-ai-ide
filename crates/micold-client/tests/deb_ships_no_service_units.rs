//! The package installs no service-manager artefact (feature 028, T014 — packaging contract
//! §1.1–1.2, FR-002).
//!
//! §1.1 lists what an install may leave behind — two executables, the desktop entry, the icons,
//! documentation — and then says what it may not: "a unit, plist, login item, scheduled task, or
//! any other service-manager artefact for the session service". §1.2 names the two files by path.
//!
//! Both are decided by one declarative list nobody re-reads once written, which is the same
//! argument `packaging_excludes_showcase.rs` makes for guarding it from a test rather than from
//! review. The failure mode here is quieter than the showcase's, and worse: a unit file reinstated
//! by a merge would restore socket activation to every upgraded machine without anyone running a
//! command, and the daemon it activated would be one this feature has taught not to expect it.
//!
//! Two properties, because a unit can be reinstated by either half. A *destination* under
//! `usr/lib/systemd` is a unit by definition, whatever it was called; a *source* under
//! `packaging/micold-daemon.` is one of the two files feature 010 shipped, whatever it is
//! installed as.

use std::fs;
use std::path::{Path, PathBuf};

/// Any destination beneath this is a service-manager artefact by definition (§1.2).
const UNIT_DESTINATION: &str = "usr/lib/systemd";

/// The source files feature 010 shipped as units, which T019 deletes (§1.2).
const UNIT_SOURCE: &str = "packaging/micold-daemon.";

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

/// The `[package.metadata.deb] assets` list, verbatim, or `None` when the section is absent.
///
/// Read as text rather than parsed, and the closing bracket found *at depth* — both for the reasons
/// `packaging_excludes_showcase.rs` writes down at length. Every entry is itself an array, so the
/// first `]` is the end of the first asset, not the end of the list.
fn deb_assets(manifest: &str) -> Option<&str> {
    let after = manifest.split_once("[package.metadata.deb]")?.1;
    let open = after.find("assets = [")?;
    let rest = &after[open + "assets = [".len()..];
    let mut depth = 0usize;
    for (i, c) in rest.char_indices() {
        match c {
            '[' => depth += 1,
            ']' if depth == 0 => return Some(&rest[..i]),
            ']' => depth -= 1,
            _ => {}
        }
    }
    None
}

/// One violation, phrased so the failure says what to do about it.
#[derive(Debug, PartialEq)]
struct Violation(String);

/// The rule. Takes the manifest text so its failure behaviour can be demonstrated below.
fn violations(manifest: &str) -> Vec<Violation> {
    let Some(assets) = deb_assets(manifest) else {
        return vec![Violation(
            "Cargo.toml has no `[package.metadata.deb] assets` list — cargo-deb then ships every \
             binary in the crate, and this check would be asserting nothing"
                .into(),
        )];
    };

    let mut out = Vec::new();
    if assets.contains(UNIT_DESTINATION) {
        out.push(Violation(format!(
            "an asset installs to `{UNIT_DESTINATION}` — an install MUST NOT leave a unit behind \
             (packaging contract §1.1, §1.2). The application is the only thing that starts the \
             session service; a unit is a second starter."
        )));
    }
    if assets.contains(UNIT_SOURCE) {
        out.push(Violation(format!(
            "an asset is sourced from `{UNIT_SOURCE}*` — those are the systemd units feature 010 \
             shipped, and this feature removes them (packaging contract §1.2)"
        )));
    }
    out
}

// ---------------------------------------------------------------------------------------------
// The real manifest
// ---------------------------------------------------------------------------------------------

#[test]
fn the_package_installs_no_service_unit() {
    let manifest = fs::read_to_string(manifest_path()).expect("read the client manifest");

    let found = violations(&manifest);
    assert!(
        found.is_empty(),
        "installing the package would register a service (packaging contract §1):\n{}",
        found
            .iter()
            .map(|v| format!("  {}", v.0))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The asset list still ships the things it is *supposed* to (§1.1), so the test above is a
/// statement about units rather than about a list somebody emptied.
#[test]
fn the_two_executables_and_the_desktop_entry_are_still_shipped() {
    let manifest = fs::read_to_string(manifest_path()).expect("read the client manifest");
    let assets = deb_assets(&manifest).expect("the deb asset list");
    for required in [
        "target/release/micold-ai-ide",
        "target/release/micold-daemon",
        "micold-ai-ide.desktop",
    ] {
        assert!(
            assets.contains(required),
            "the asset list no longer ships `{required}` — §1.1 says an install leaves the two \
             executables and the desktop entry behind, so an empty-by-accident list is a \
             different bug wearing this one's clothes"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The synthetic Red: the rule really does fail
// ---------------------------------------------------------------------------------------------

const HEALTHY: &str = r#"
[package]
name = "micold-client"

[package.metadata.deb]
assets = [
    ["target/release/micold-ai-ide", "usr/bin/", "755"],
    ["target/release/micold-daemon", "usr/bin/", "755"],
    ["../../packaging/micold-ai-ide.desktop", "usr/share/applications/micold-ai-ide.desktop", "644"],
]
"#;

#[test]
fn the_healthy_manifest_passes() {
    assert_eq!(violations(HEALTHY), vec![]);
}

#[test]
fn a_manifest_that_installs_a_unit_fails() {
    let shipping = HEALTHY.replace(
        r#"["target/release/micold-daemon", "usr/bin/", "755"],"#,
        r#"["target/release/micold-daemon", "usr/bin/", "755"],
    ["../../packaging/micold-daemon.socket", "usr/lib/systemd/user/micold-daemon.socket", "644"],"#,
    );
    let found = violations(&shipping);
    assert_eq!(
        found.len(),
        2,
        "a reinstated unit trips both halves of the rule — its destination and its source: {found:?}"
    );
}

/// A unit installed from somewhere else, or under a different name, is still a unit. The
/// destination half exists so the check is not a check on one filename.
#[test]
fn a_unit_installed_from_a_different_source_still_fails() {
    let renamed = HEALTHY.replace(
        r#"["target/release/micold-daemon", "usr/bin/", "755"],"#,
        r#"["target/release/micold-daemon", "usr/bin/", "755"],
    ["../../contrib/session-host.service", "usr/lib/systemd/user/session-host.service", "644"],"#,
    );
    let found = violations(&renamed);
    assert_eq!(found.len(), 1, "expected exactly one violation: {found:?}");
    assert!(found[0].0.contains(UNIT_DESTINATION), "{}", found[0].0);
}

#[test]
fn a_missing_asset_list_fails_rather_than_passing() {
    let found = violations("[package]\nname = \"x\"\n");
    assert_eq!(found.len(), 1, "expected exactly one violation: {found:?}");
    assert!(
        found[0].0.contains("no `[package.metadata.deb] assets`"),
        "{}",
        found[0].0
    );
}

/// The unit files themselves are gone from the repository (T019). If they came back, the asset
/// list is one line away from shipping them again.
#[test]
fn the_unit_files_are_not_in_the_repository() {
    let packaging = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/micold-client")
        .join("packaging");
    for unit in ["micold-daemon.service", "micold-daemon.socket"] {
        let path = packaging.join(unit);
        assert!(
            !path.exists(),
            "{} still exists — this feature removes the systemd units, and a file kept 'just in \
             case' is a file somebody re-adds to the asset list (packaging contract §1.2)",
            path.display()
        );
    }
}
