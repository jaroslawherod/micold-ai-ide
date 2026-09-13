//! The install guide covers the Windows package end to end (feature 030, FR-010, FR-017, SC-006).
//!
//! A first-time Windows user goes from the guide to a running session without another page, and the
//! guide states the limits the package cannot lift. Text scans of `docs/`, no Markdown parser.

use std::fs;
use std::path::{Path, PathBuf};

fn doc_path(relative: &str) -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs")
        .join(relative)
}

fn doc(relative: &str) -> String {
    let path = doc_path(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

const WINDOWS_GUIDE: &str = "user-guide/install-windows.md";

/// The text of every Markdown heading in `page`, without its `#` markers.
fn headings(page: &str) -> Vec<&str> {
    page.lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim())
        .collect()
}

/// Download to removal, then what the package cannot do (SC-006).
const REQUIRED_HEADINGS: [&str; 5] = ["Download", "Install", "Upgrading", "Removing", "Limits"];

#[test]
fn windows_guide_has_the_lifecycle_headings() {
    let page = doc(WINDOWS_GUIDE);
    let present = headings(&page);

    let missing: Vec<&str> = REQUIRED_HEADINGS
        .into_iter()
        .filter(|heading| !present.contains(heading))
        .collect();
    assert!(
        missing.is_empty(),
        "{WINDOWS_GUIDE} must have the headings {REQUIRED_HEADINGS:?} (FR-017, SC-006); missing \
         {missing:?}, found {present:?}"
    );
}

const INSTALL_PAGE: &str = "install.md";

/// What the install page said while Windows had no package; either wording now misleads (SC-006).
const NO_PACKAGE_CLAIMS: [&str; 2] = [
    "no packaged build for macOS or Windows",
    "no packaged build for Windows",
];

#[test]
fn install_page_no_longer_says_windows_has_no_package() {
    let page = doc(INSTALL_PAGE);

    let stale: Vec<&str> = NO_PACKAGE_CLAIMS
        .into_iter()
        .filter(|claim| page.contains(claim))
        .collect();
    assert!(
        stale.is_empty(),
        "{INSTALL_PAGE} must not say Windows has no package now that releases carry one (FR-017, \
         SC-006); still says {stale:?}"
    );
}

/// The body under the `## name` heading, up to the next heading of the same level.
fn section<'a>(page: &'a str, name: &str) -> Option<&'a str> {
    let header = format!("## {name}\n");
    let start = page.find(&header)? + header.len();
    let rest = &page[start..];
    Some(rest.find("\n## ").map_or(rest, |end| &rest[..end]))
}

/// Sessions end at sign-out on Windows; only the container placement outlives it (FR-017, US4-AS2).
const LOGOUT_LIMIT: &str = "logging out";
const CONTAINER_PLACEMENT_PAGE: &str = "sandboxed-daemon.md";

#[test]
fn windows_guide_limits_say_sessions_end_at_logout() {
    let page = doc(WINDOWS_GUIDE);
    let limits =
        section(&page, "Limits").expect("the Windows guide must have a `## Limits` section");

    for needle in [LOGOUT_LIMIT, CONTAINER_PLACEMENT_PAGE] {
        assert!(
            limits.contains(needle),
            "the Limits section of {WINDOWS_GUIDE} must say sessions do not survive logging out \
             unless the service runs in a container, mentioning `{needle}` (FR-017); it is:\n{limits}"
        );
    }
}

/// The two Windows gatekeepers an unsigned installer meets, and the way round the one that has no
/// per-app override (FR-010).
const GATEKEEPER_MENTIONS: [&str; 3] = ["SmartScreen", "Smart App Control", "from source"];

#[test]
fn windows_guide_names_smartscreen_and_smart_app_control() {
    let page = doc(WINDOWS_GUIDE);

    let missing: Vec<&str> = GATEKEEPER_MENTIONS
        .into_iter()
        .filter(|mention| !page.contains(mention))
        .collect();
    assert!(
        missing.is_empty(),
        "{WINDOWS_GUIDE} must mention {GATEKEEPER_MENTIONS:?} (FR-010, FR-017); missing {missing:?}"
    );
}

/// The install page summarises; the Windows guide is where a first-time user finishes (SC-006).
const WINDOWS_GUIDE_LINK: &str = "(user-guide/install-windows.md)";

#[test]
fn install_page_links_the_windows_guide() {
    let page = doc(INSTALL_PAGE);

    assert!(
        page.contains(WINDOWS_GUIDE_LINK),
        "{INSTALL_PAGE} must link the Windows guide as `{WINDOWS_GUIDE_LINK}` (FR-017, SC-006)"
    );
}
