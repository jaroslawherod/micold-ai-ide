//! Every release bundle ships the font licence set (feature 1260, T021 — FR-008, SC-004).
//!
//! The fonts are compiled into the binary with `include_bytes!`, so every package carries them; the
//! SIL Open Font License requires the licence text to travel with the font, so every package must
//! carry that too. Four declarative files decide what a package holds — the deb and rpm asset lists,
//! the macOS bundle script and the Windows installer's `[Files]` — and none of them is re-read once
//! written. This test re-reads them.
//!
//! The rule takes the four files' text, so the synthetic cases at the end can show it failing.

#[path = "support/packaging.rs"]
mod packaging;

use packaging::{
    basename, deb_assets, iss_file_entries, manifest_path, repo_root, rpm_assets, FONT_LICENCES,
};
use std::fs;

/// Where each bundle puts the licence set (research R5).
const DEB_DEST: &str = "usr/share/doc/micold-ai-ide/fonts/";
const RPM_DEST: &str = "/usr/share/doc/micold-ai-ide/fonts/";
const MACOS_DEST: &str = "Contents/Resources/licenses/fonts";
const WINDOWS_DEST: &str = r"{app}\licenses\fonts";

/// The four packaging inputs, as text.
struct Bundles<'a> {
    manifest: &'a str,
    macos_script: &'a str,
    iss: &'a str,
}

/// Every `(file, bundle)` pair where the bundle does not ship the licence file to its licence
/// directory, phrased as `<bundle> does not ship assets/fonts/<file> to <dest>`.
fn missing(bundles: &Bundles, licences: &[&str]) -> Vec<String> {
    let deb = deb_assets(bundles.manifest).unwrap_or("");
    let rpm = rpm_assets(bundles.manifest).unwrap_or("");
    let macos_code: Vec<&str> = bundles
        .macos_script
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect();
    let iss = iss_file_entries(bundles.iss);

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
        if !macos_code.iter().any(|l| l.contains(&source))
            || !macos_code.iter().any(|l| l.contains(MACOS_DEST))
        {
            out.push(format!(
                "the macOS .app (scripts/macos-bundle.sh) does not ship {path} to {MACOS_DEST}"
            ));
        }
        let iss_ships = iss.iter().any(|(src, dest)| {
            basename(src) == basename(path) && dest.eq_ignore_ascii_case(WINDOWS_DEST)
        });
        if !iss_ships {
            out.push(format!(
                "the Windows installer (packaging/windows/micold-ai-ide.iss) does not ship \
                 {path} to {WINDOWS_DEST}"
            ));
        }
    }
    out
}

fn read(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
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
    let macos_script = read("scripts/macos-bundle.sh");
    let iss = read("packaging/windows/micold-ai-ide.iss");
    let bundles = Bundles {
        manifest: &manifest,
        macos_script: &macos_script,
        iss: &iss,
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

const HEALTHY_MACOS: &str = r#"
FONT_LICENCES=("assets/fonts/LICENSE")
FONT_LICENCE_DIR="Contents/Resources/licenses/fonts"
"#;

const HEALTHY_ISS: &str = r#"
[Files]
Source: "{#FontsDir}\LICENSE"; DestDir: "{app}\licenses\fonts"; Flags: ignoreversion
"#;

const HEALTHY: Bundles = Bundles {
    manifest: HEALTHY_MANIFEST,
    macos_script: HEALTHY_MACOS,
    iss: HEALTHY_ISS,
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

    assert_eq!(found.len(), 4, "one failure per bundle: {found:?}");
    for bundle in ["deb", "rpm", "macOS", "Windows"] {
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
    let iss = HEALTHY_ISS.replace(r"{app}\licenses\fonts", "{app}");
    let bundles = Bundles {
        manifest: &manifest,
        iss: &iss,
        ..HEALTHY
    };

    let found = missing(&bundles, &["assets/fonts/LICENSE"]);

    assert_eq!(found.len(), 3, "deb, rpm and Windows: {found:?}");
}

/// A comment explaining the licence copy is not the copy.
#[test]
fn a_macos_script_that_names_the_licence_only_in_a_comment_fails() {
    let bundles = Bundles {
        macos_script: "# FONT_LICENCES=(\"assets/fonts/LICENSE\")\nFONT_LICENCE_DIR=\"Contents/Resources/licenses/fonts\"\n",
        ..HEALTHY
    };

    let found = missing(&bundles, &["assets/fonts/LICENSE"]);

    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("macOS"), "{}", found[0]);
}
