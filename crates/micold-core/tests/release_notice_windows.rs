//! The release page tells a Windows user which setup to pick and how past SmartScreen (feature 030,
//! FR-010, FR-015).
//!
//! The installer ships unsigned, so SmartScreen stops the first run with a dialog whose way through
//! is hidden behind **More info**. The `publish` job appends `.github/release-notice-windows.md` to
//! the release body; these assert what that notice says. Text scans, no Markdown parser.

use std::fs;
use std::path::{Path, PathBuf};

fn notice_path() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/release-notice-windows.md")
}

fn notice() -> String {
    let path = notice_path();
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The notice shares the release body with the changelog and the other platforms' notices; past this
/// it stops being a notice.
const MAX_NON_EMPTY_LINES: usize = 5;

#[test]
fn notice_is_at_most_five_lines() {
    let text = notice();

    let lines = text.lines().filter(|line| !line.trim().is_empty()).count();
    assert!(
        lines <= MAX_NON_EMPTY_LINES,
        "the Windows release notice must stay within {MAX_NON_EMPTY_LINES} non-empty lines (FR-010); \
         it has {lines}"
    );
}

/// What a user needs from the notice: the two architectures to choose between, the two SmartScreen
/// buttons in the order they appear, and where the full guide is.
const REQUIRED_MENTIONS: [&str; 5] = ["x64", "ARM64", "More info", "Run anyway", "install-windows"];

#[test]
fn notice_names_arches_smartscreen_steps_and_guide() {
    let text = notice();

    let missing: Vec<&str> = REQUIRED_MENTIONS
        .into_iter()
        .filter(|mention| !text.contains(mention))
        .collect();
    assert!(
        missing.is_empty(),
        "the Windows release notice must mention {REQUIRED_MENTIONS:?} (FR-010, FR-015); missing \
         {missing:?}"
    );
}

fn release_workflow() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(".github/workflows/release.yml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The `publish` job's lines: from its two-space-indented header to the next job's.
fn publish_job(workflow: &str) -> String {
    workflow
        .lines()
        .skip_while(|line| *line != "  publish:")
        .skip(1)
        .take_while(|line| {
            !(line.starts_with("  ") && !line.starts_with("   ") && line.trim_end().ends_with(':'))
        })
        .filter(|line| !line.trim_start().starts_with('#'))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn publish_appends_the_windows_notice() {
    // A notice file nothing reads never reaches the release page. Comments are skipped so a step
    // that only mentions the file does not count.
    let job = publish_job(&release_workflow());
    assert!(
        job.contains("release-notice-windows.md"),
        "the `publish` job in release.yml must append `.github/release-notice-windows.md` to the \
         release body (FR-010); it does not reference it"
    );
}
