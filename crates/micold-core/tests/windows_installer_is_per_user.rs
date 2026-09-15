//! The Windows installer installs for one user and leaves the machine alone (feature 030, FR-004,
//! FR-006, FR-008, FR-010, FR-011, FR-015).
//!
//! Every property here is a single directive in `packaging/windows/micold-ai-ide.iss`, and every one
//! of them fails silently: an installer that asks for elevation, relaunches the app, or writes a
//! Run key still installs cleanly, and the Windows smoke test would pass. So the directives are
//! asserted where they are written.
//!
//! # Why text, not a parser
//!
//! Inno Setup directives are `Name=Value` lines under `[Section]` headers. A line scan answers
//! "which value does this directive have" exactly, and no Inno Setup parser exists as a crate.

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

/// The script's non-comment lines, trimmed.
fn directive_lines(script: &str) -> Vec<&str> {
    script
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with(';'))
        .collect()
}

/// The value of every `name=value` directive called `name`, in order (Inno Setup names are
/// case-insensitive).
fn directive_values<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    directive_lines(script)
        .into_iter()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim())
        .collect()
}

#[test]
fn installs_without_elevation() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "PrivilegesRequired"),
        vec!["lowest"],
        "the installer must run as the signed-in user, with no UAC prompt (FR-004)"
    );
    assert_eq!(
        directive_values(&script, "PrivilegesRequiredOverridesAllowed"),
        Vec::<&str>::new(),
        "the user must never be offered an all-users (elevated) install (FR-004)"
    );
}

/// The installer's identity. Windows keys the uninstall entry on it, so an upgrade replaces the
/// installed copy only while this never changes. The leading `{{` is Inno Setup's escaped `{`.
const PINNED_APP_ID: &str = "{{1B19A6AC-4C91-4033-88EA-F7F283127C8A}";

#[test]
fn app_id_is_pinned() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "AppId"),
        vec![PINNED_APP_ID],
        "AppId is pinned forever: a new value installs a second copy beside the old one instead of \
         upgrading it (FR-006, FR-008)"
    );
}

#[test]
fn never_relaunches_the_app() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "RestartApplications"),
        vec!["no"],
        "setup must not relaunch the app it closed: the installer never starts anything on its own \
         (FR-011)"
    );
}

/// The `[Name]` section headers in the script.
fn sections(script: &str) -> Vec<&str> {
    directive_lines(script)
        .into_iter()
        .filter(|line| line.starts_with('[') && line.ends_with(']'))
        .collect()
}

#[test]
fn writes_no_registry_entries() {
    let script = iss();

    let registry: Vec<&str> = sections(&script)
        .into_iter()
        .filter(|section| section.eq_ignore_ascii_case("[Registry]"))
        .collect();

    assert!(
        registry.is_empty(),
        "the installer must not write registry values of its own: no Run key, no PATH edit, no file \
         associations (FR-011); found {registry:?}"
    );
}

#[test]
fn is_not_signed() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "SignTool"),
        Vec::<&str>::new(),
        "the installer ships unsigned; a `SignTool` directive fails every build that has no signing \
         tool configured (FR-010)"
    );
}

/// The script as Inno Setup's preprocessor would emit it for `/DArch=<arch>`, with every other
/// define (`AppVersion`, `BinDir`) present.
///
/// Handles exactly what the script uses: `#if`/`#elif` on `Arch == "<value>"`, `#ifndef` on a
/// define (never true here, since the build passes all three), `#else`, `#endif` and `#error`. Any
/// other condition panics, so a new one cannot be misread as false.
fn preprocess(script: &str, arch: &str) -> String {
    // One frame per open `#if`: (this branch is live, an earlier branch was taken).
    let mut frames: Vec<(bool, bool)> = Vec::new();
    let live = |frames: &[(bool, bool)]| frames.iter().all(|(on, _)| *on);
    let condition = |expr: &str| -> bool {
        let (name, value) = expr
            .split_once("==")
            .unwrap_or_else(|| panic!("unsupported preprocessor condition `{expr}`"));
        assert_eq!(
            name.trim(),
            "Arch",
            "unsupported preprocessor condition `{expr}`"
        );
        value.trim().trim_matches('"') == arch
    };
    let mut out = String::new();
    for line in script.lines() {
        let trimmed = line.trim();
        if let Some(expr) = trimmed.strip_prefix("#if ") {
            let on = condition(expr);
            frames.push((on, on));
        } else if trimmed.starts_with("#ifndef ") {
            frames.push((false, false));
        } else if let Some(expr) = trimmed.strip_prefix("#elif ") {
            let (_, taken) = frames.pop().expect("#elif without #if");
            let on = !taken && condition(expr);
            frames.push((on, taken || on));
        } else if trimmed == "#else" {
            let (_, taken) = frames.pop().expect("#else without #if");
            frames.push((!taken, true));
        } else if trimmed == "#endif" {
            frames.pop().expect("#endif without #if");
        } else if live(&frames) {
            assert!(
                !trimmed.starts_with("#error"),
                "the script refuses /DArch={arch}: {trimmed}"
            );
            out.push_str(line);
            out.push('\n');
        }
    }
    assert!(frames.is_empty(), "unterminated #if in the script");
    out
}

#[test]
fn architecture_follows_the_arch_define() {
    let script = iss();

    for (arch, allowed) in [("x64", "x64compatible and not arm64"), ("arm64", "arm64")] {
        let emitted = preprocess(&script, arch);
        assert_eq!(
            directive_values(&emitted, "ArchitecturesAllowed"),
            vec![allowed],
            "/DArch={arch} must allow exactly `{allowed}`, so each package refuses the other \
             architecture (FR-015)"
        );
        assert_eq!(
            directive_values(&emitted, "ArchitecturesInstallIn64BitMode"),
            vec![allowed],
            "/DArch={arch} must install in 64-bit mode on `{allowed}` (FR-015)"
        );
    }
}
