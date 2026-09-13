//! The Windows installer's version comes from the build, never from the script (feature 030,
//! FR-001, FR-013).
//!
//! `scripts/windows-installer.sh` passes `/DAppVersion=` from `[workspace.package]` in `Cargo.toml`,
//! the same source every other package reads. A version typed into the `.iss` would build fine and
//! go stale at the next release, and Windows compares it to decide whether a run is an upgrade.
//!
//! Text scans, as in `windows_installer_is_per_user.rs`: directives are `Name=Value` lines.

use std::fs;
use std::path::{Path, PathBuf};

fn iss_path() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("packaging/windows/micold-ai-ide.iss")
}

fn iss() -> String {
    let path = iss_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The value of every `name=value` directive called `name`, in order, skipping `;` comments.
fn directive_values<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim())
        .collect()
}

/// Every `<digits>.<digits>.<digits>` run in `text`, the shape a typed-in version takes.
fn semver_literals(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    while start < bytes.len() {
        if !bytes[start].is_ascii_digit() || (start > 0 && bytes[start - 1].is_ascii_digit()) {
            start += 1;
            continue;
        }
        let mut end = start;
        let mut groups = 0;
        loop {
            let group_start = end;
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if end == group_start {
                break;
            }
            groups += 1;
            if groups == 3 || end >= bytes.len() || bytes[end] != b'.' {
                break;
            }
            end += 1;
        }
        if groups == 3 {
            found.push(text[start..end].to_string());
            start = end;
        } else {
            start += 1;
        }
    }
    found
}

const INJECTED_VERSION: &str = "{#AppVersion}";

#[test]
fn version_is_the_injected_define() {
    let script = iss();

    for directive in ["AppVersion", "VersionInfoVersion"] {
        assert_eq!(
            directive_values(&script, directive),
            vec![INJECTED_VERSION],
            "{directive} must be `{INJECTED_VERSION}`, passed by the build from Cargo.toml (FR-013)"
        );
    }
    assert_eq!(
        semver_literals(&script),
        Vec::<String>::new(),
        "the .iss must not spell out a version; it goes stale at the next release (FR-013)"
    );
}

/// The published asset name. `site/stage.sh` and the install guide link it by this exact shape.
const OUTPUT_BASE_FILENAME: &str = "micold-ai-ide-{#AppVersion}-{#Arch}-setup";

#[test]
fn setup_file_name_carries_version_and_arch() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "OutputBaseFilename"),
        vec![OUTPUT_BASE_FILENAME],
        "the setup exe must be named for its version and architecture, one per arch (FR-001)"
    );
}
