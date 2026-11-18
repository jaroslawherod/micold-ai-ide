//! The daemon never notifies the desktop (feature 039, FR-007).
//!
//! The daemon runs on hosts with no desktop session, so its manifest names no desktop notification
//! crate. This is a promise about what is *absent*, the kind that decays quietly, so it is a checked
//! property: the manifest is scanned with its comments skipped. (The desktop client's half of this
//! file went with the iced app, #1959.)

use std::fs;
use std::path::Path;

const DAEMON_TOML: &str = "crates/micold-daemon/Cargo.toml";

/// Desktop notification crates the daemon must not depend on.
const NOTIFICATION_CRATES: &[&str] = &["zbus", "mac-usernotifications", "tauri-winrt-notification"];

#[test]
fn the_daemon_depends_on_no_notification_crate() {
    // tests/ -> micold-core/ -> crates/ -> repo root
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(DAEMON_TOML);
    let text = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        text.contains("[dependencies]"),
        "{DAEMON_TOML} has no [dependencies]; the scan covers nothing"
    );
    let found: Vec<String> = text
        .lines()
        .enumerate()
        .filter(|(_, l)| !l.trim_start().starts_with('#'))
        .flat_map(|(n, l)| {
            NOTIFICATION_CRATES
                .iter()
                .filter(move |c| l.contains(**c))
                .map(move |c| format!("{DAEMON_TOML}:{}: names `{c}` — {}", n + 1, l.trim()))
        })
        .collect();
    assert!(
        found.is_empty(),
        "the daemon's manifest names a desktop notification crate:\n  {}",
        found.join("\n  ")
    );
}
