//! The showcase is never installed (feature 020, T006 — FR-018, FR-018a, SC-008).
//!
//! This is the requirement with the worst failure mode in the feature: getting it wrong ships a
//! development tool to end users. The artifacts that decide it are two declarative files nobody
//! re-reads once written — the Debian asset list and the desktop entry — so it is simultaneously the
//! cheapest thing to automate and the least safe thing to leave to a person remembering to look.
//!
//! cargo-deb ships **only** the listed assets when `assets` is present. That is why the showcase is
//! excluded today, and it is also why the guards below matter as much as the scan: an `assets` list
//! that had been emptied or moved would make a naive check pass while cargo-deb quietly reverted to
//! shipping every binary in the crate.
//!
//! The rule is a function over its inputs, and two tests drive it against deliberately-broken
//! manifests. Without those, this file would be a gate nobody had ever seen fail — which is the
//! failure mode the whole feature exists to remove.

use std::fs;
use std::path::{Path, PathBuf};

/// The showcase binary's name, as `Cargo.toml` declares it.
const SHOWCASE_BIN: &str = "micold-showcase";

fn repo_root() -> PathBuf {
    // tests/ -> crates/micold-client -> crates -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("repo root above crates/micold-client")
        .to_path_buf()
}

fn manifest_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml")
}

fn desktop_path() -> PathBuf {
    repo_root().join("packaging/micold-ai-ide.desktop")
}

/// The `[package.metadata.deb] assets` list, verbatim, or `None` when the section is absent.
///
/// Read as text rather than parsed: the property is about what a *declarative file says*, and a TOML
/// parser would add a dependency to assert something a substring already settles.
///
/// The closing bracket is found **at depth**, not by the first `]`. Every entry in this list is
/// itself an array, so a naive `find(']')` returns only the first asset — which is how the first
/// draft of this function passed a manifest that shipped the showcase. The synthetic tests below are
/// what caught it, and are the reason they exist.
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

/// Whether a file's text names the showcase — by binary name or by build-output path.
fn names_showcase(text: &str, bin: &str) -> bool {
    text.contains(bin)
}

/// One violation, phrased so the failure says what to do about it.
#[derive(Debug, PartialEq)]
struct Violation(String);

/// The rule. Takes its inputs so its failure behaviour can be demonstrated (see the two
/// `a_manifest_that_ships_it_fails` / `a_desktop_entry_that_launches_it_fails` tests below).
fn violations(manifest: &str, desktop: &str, bin: &str) -> Vec<Violation> {
    let mut out = Vec::new();

    match deb_assets(manifest) {
        None => out.push(Violation(
            "Cargo.toml has no `[package.metadata.deb] assets` list — cargo-deb falls back to \
             shipping every binary in the crate, which would include the showcase (FR-018)"
                .into(),
        )),
        Some(assets) if assets.trim().is_empty() => out.push(Violation(
            "Cargo.toml's `[package.metadata.deb] assets` list is empty — an empty list is not an \
             exclusion, it is an unstated default (FR-018a)"
                .into(),
        )),
        Some(assets) => {
            if names_showcase(assets, bin) {
                out.push(Violation(format!(
                    "Cargo.toml's `[package.metadata.deb] assets` names `{bin}` — the showcase is a \
                     development tool and MUST NOT reach an end user through an installation \
                     (FR-018). Remove the asset; do not relax this check."
                )));
            }
        }
    }

    if names_showcase(desktop, bin) {
        out.push(Violation(format!(
            "packaging/micold-ai-ide.desktop names `{bin}` — the desktop entry launches the \
             application, never the showcase (FR-018)"
        )));
    }

    out
}

// ---------------------------------------------------------------------------------------------
// The real files
// ---------------------------------------------------------------------------------------------

#[test]
fn the_installable_package_contains_no_showcase() {
    let manifest = fs::read_to_string(manifest_path()).expect("read the client manifest");
    let desktop = fs::read_to_string(desktop_path()).expect("read the desktop entry");

    let found = violations(&manifest, &desktop, SHOWCASE_BIN);
    assert!(
        found.is_empty(),
        "the showcase would be installed (SC-008):\n{}",
        found
            .iter()
            .map(|v| format!("  {}", v.0))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The manifest *does* declare the showcase as a binary — it has to, or there would be nothing to
/// run. What must not happen is the asset list naming it. Asserting the declaration exists keeps
/// this file honest: it proves the scan is looking at a manifest that really does build a showcase,
/// so `the_installable_package_contains_no_showcase` is a statement about packaging rather than
/// about a binary that was quietly deleted.
#[test]
fn the_manifest_declares_the_showcase_binary_but_does_not_ship_it() {
    let manifest = fs::read_to_string(manifest_path()).expect("read the client manifest");
    assert!(
        manifest.contains(&format!("name = \"{SHOWCASE_BIN}\"")),
        "the manifest no longer declares a `{SHOWCASE_BIN}` binary — if the showcase was renamed, \
         SHOWCASE_BIN must move with it, or this file checks the exclusion of something that does \
         not exist"
    );
    let assets = deb_assets(&manifest).expect("the deb asset list");
    assert!(
        !assets.contains(SHOWCASE_BIN),
        "declared as a binary and listed as an asset — the second is the part that ships it"
    );
}

// ---------------------------------------------------------------------------------------------
// The synthetic Red: the rule really does fail
// ---------------------------------------------------------------------------------------------

const HEALTHY_MANIFEST: &str = r#"
[package]
name = "micold-client"

[package.metadata.deb]
assets = [
    ["target/release/micold-ai-ide", "usr/bin/", "755"],
    ["target/release/micold-daemon", "usr/bin/", "755"],
]
"#;

const HEALTHY_DESKTOP: &str = "[Desktop Entry]\nExec=micold-ai-ide\n";

#[test]
fn the_healthy_pair_passes() {
    assert_eq!(
        violations(HEALTHY_MANIFEST, HEALTHY_DESKTOP, SHOWCASE_BIN),
        vec![]
    );
}

#[test]
fn a_manifest_that_ships_it_fails() {
    let shipping = HEALTHY_MANIFEST.replace(
        r#"["target/release/micold-daemon", "usr/bin/", "755"],"#,
        r#"["target/release/micold-daemon", "usr/bin/", "755"],
    ["target/release/micold-showcase", "usr/bin/", "755"],"#,
    );
    let found = violations(&shipping, HEALTHY_DESKTOP, SHOWCASE_BIN);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one violation, got {found:?}"
    );
    assert!(
        found[0].0.contains("Cargo.toml") && found[0].0.contains(SHOWCASE_BIN),
        "the failure must name the file and the binary: {}",
        found[0].0
    );
}

#[test]
fn a_desktop_entry_that_launches_it_fails() {
    let found = violations(
        HEALTHY_MANIFEST,
        "[Desktop Entry]\nExec=micold-showcase\n",
        SHOWCASE_BIN,
    );
    assert_eq!(
        found.len(),
        1,
        "expected exactly one violation, got {found:?}"
    );
    assert!(
        found[0].0.contains(".desktop"),
        "the failure must name the desktop entry: {}",
        found[0].0
    );
}

#[test]
fn a_missing_asset_list_fails_rather_than_passing() {
    let found = violations("[package]\nname = \"x\"\n", HEALTHY_DESKTOP, SHOWCASE_BIN);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one violation, got {found:?}"
    );
    assert!(
        found[0].0.contains("no `[package.metadata.deb] assets`"),
        "a manifest with no asset list must fail loudly — cargo-deb's fallback ships everything: {}",
        found[0].0
    );
}

#[test]
fn an_emptied_asset_list_fails_rather_than_passing() {
    let emptied = "[package.metadata.deb]\nassets = [\n]\n";
    let found = violations(emptied, HEALTHY_DESKTOP, SHOWCASE_BIN);
    assert_eq!(
        found.len(),
        1,
        "expected exactly one violation, got {found:?}"
    );
    assert!(found[0].0.contains("empty"), "{}", found[0].0);
}

/// Both files have to exist for any of the above to mean anything.
#[test]
fn both_packaging_artifacts_exist() {
    assert!(
        manifest_path().is_file(),
        "{} not found",
        manifest_path().display()
    );
    assert!(
        desktop_path().is_file(),
        "{} not found — if packaging moved, this file's paths must move with it, or the exclusion \
         goes unchecked",
        desktop_path().display()
    );
}
