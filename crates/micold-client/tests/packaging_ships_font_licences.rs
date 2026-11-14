//! Every release bundle ships the font licence set (feature 1260, T021 — FR-008, SC-004).
//!
//! The fonts are compiled into the binary with `include_bytes!`, so every package carries them; the
//! SIL Open Font License requires the licence text to travel with the font, so every package must
//! carry that too. The deb and rpm asset lists decide what a package holds, and neither is re-read
//! once written. This test re-reads them. (Linux is the only supported platform since #1897.)
//!
//! The rule takes the manifest's text, so the synthetic cases at the end can show it failing.

use crate::packaging;

use packaging::{deb_assets, manifest_path, repo_root, rpm_assets, FONT_LICENCES};
use std::fs;

/// Where each bundle puts the licence set (research R5).
const DEB_DEST: &str = "usr/share/doc/micold-ai-ide/fonts/";
const RPM_DEST: &str = "/usr/share/doc/micold-ai-ide/fonts/";

/// The packaging inputs, as text.
struct Bundles<'a> {
    manifest: &'a str,
}

/// Every `(file, bundle)` pair where the bundle does not ship the licence file to its licence
/// directory, phrased as `<bundle> does not ship assets/fonts/<file> to <dest>`.
fn missing(bundles: &Bundles, licences: &[&str]) -> Vec<String> {
    let deb = deb_assets(bundles.manifest).unwrap_or("");
    let rpm = rpm_assets(bundles.manifest).unwrap_or("");

    let mut out = Vec::new();
    for path in licences {
        let source = format!("{path}\"");
        // One asset per line in both lists: the entry naming the source must name the destination.
        let entry_ships = |list: &str, dest: &str| {
            list.lines()
                .any(|l| l.contains(&source) && l.contains(dest))
        };
        if !entry_ships(deb, DEB_DEST) {
            out.push(format!(
                "the deb (`[package.metadata.deb] assets`) does not ship {path} to {DEB_DEST}"
            ));
        }
        if !entry_ships(rpm, RPM_DEST) {
            out.push(format!(
                "the rpm (`[package.metadata.generate-rpm] assets`) does not ship {path} to {RPM_DEST}"
            ));
        }
    }
    out
}

#[test]
fn the_licence_set_is_committed_beside_the_fonts() {
    for licence in FONT_LICENCES {
        let path = repo_root().join(licence);
        assert!(
            path.is_file(),
            "{} is not in the repository — every bundle ships it (FR-008)",
            path.display()
        );
    }
}

#[test]
fn every_release_bundle_ships_the_font_licence_set() {
    let manifest = fs::read_to_string(manifest_path()).expect("read the client manifest");
    let bundles = Bundles {
        manifest: &manifest,
    };

    let found = missing(&bundles, FONT_LICENCES);

    assert!(
        found.is_empty(),
        "a release bundle would ship a font without its licence (FR-008, SC-004):\n  {}",
        found.join("\n  ")
    );
}

// ---------------------------------------------------------------------------------------------
// The synthetic Red: the rule really does fail
// ---------------------------------------------------------------------------------------------

const HEALTHY_MANIFEST: &str = r#"
[package.metadata.deb]
assets = [
    ["../../assets/fonts/LICENSE", "usr/share/doc/micold-ai-ide/fonts/LICENSE", "644"],
]

[package.metadata.generate-rpm]
assets = [
    { source = "assets/fonts/LICENSE", dest = "/usr/share/doc/micold-ai-ide/fonts/LICENSE", mode = "644", doc = true },
]
"#;

const HEALTHY: Bundles = Bundles {
    manifest: HEALTHY_MANIFEST,
};

#[test]
fn the_healthy_bundles_pass() {
    assert_eq!(
        missing(&HEALTHY, &["assets/fonts/LICENSE"]),
        Vec::<String>::new()
    );
}

#[test]
fn a_licence_missing_from_every_bundle_is_named_once_per_bundle() {
    let found = missing(&HEALTHY, &["assets/fonts/LICENSE-FiraMono-OFL.txt"]);

    assert_eq!(found.len(), 2, "one failure per bundle: {found:?}");
    for bundle in ["deb", "rpm"] {
        assert!(
            found
                .iter()
                .any(|m| m.contains(bundle) && m.contains("LICENSE-FiraMono-OFL.txt")),
            "the failure must name the bundle `{bundle}` and the file: {found:?}"
        );
    }
}

#[test]
fn a_licence_shipped_to_the_wrong_place_fails() {
    let manifest = HEALTHY_MANIFEST.replace(
        "usr/share/doc/micold-ai-ide/fonts/",
        "usr/share/doc/micold-ai-ide/",
    );
    let bundles = Bundles {
        manifest: &manifest,
    };

    let found = missing(&bundles, &["assets/fonts/LICENSE"]);

    assert_eq!(found.len(), 2, "deb and rpm: {found:?}");
}
