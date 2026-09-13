//! Installing the application starts nothing and registers nothing (feature 028, FR-025).
//!
//! The clarification that shaped US6 was a decision not to close the logout-survival gap with a
//! background registration: macOS behaviour stays "sessions survive closing the window, not logging
//! out", and the answer for someone who needs more is the container placement. Dragging the app to
//! `Applications` therefore installs an application, full stop — no launch agent, no launch daemon,
//! no login item, nothing that runs when the user is not running it.
//!
//! That is a promise made in `docs/user-guide/install-macos.md`, and a promise about what software
//! does *not* do is the kind that decays quietly. Registering a login item is two lines and a
//! plausible bug fix for "the daemon isn't running after I log in" — the fix that would make the
//! documentation false without anyone noticing it had. So it is a checked property here rather than
//! a claim there.
//!
//! # What is scanned, and what is not
//!
//! The packaging inputs (what ships) and every crate's `src/` (what runs). Not `tests/`: this file
//! has to name the very APIs it forbids, and a scan that read itself would fail on its own
//! vocabulary. Not the documentation, which is allowed — required, in fact — to discuss them.

use std::fs;
use std::path::{Path, PathBuf};

/// Ways to register something that outlives the application, with the name to report.
///
/// `launchctl` and the two directory names cover the shell/plist route; `SMAppService` and
/// `SMLoginItemSetEnabled` cover the ServiceManagement API route, which is what a Rust binding or
/// an `objc` message send would name.
const REGISTRATIONS: &[(&str, &str)] = &[
    ("LaunchAgents", "a launch agent payload"),
    ("LaunchDaemons", "a launch daemon payload"),
    ("launchctl", "a launchctl invocation"),
    ("SMAppService", "a ServiceManagement registration"),
    ("SMLoginItemSetEnabled", "a login item registration"),
    ("NSLoginItem", "a login item declaration"),
];

/// The files that describe what the installed application is.
const PACKAGING: &[&str] = &[
    "packaging/macos/Info.plist.in",
    "packaging/macos/entitlements.plist",
    "scripts/macos-bundle.sh",
    "scripts/macos-dmg.sh",
];

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every `.rs` file under `crates/*/src`, as `(relative path, contents)`.
fn sources() -> Vec<(String, String)> {
    let crates = repo_root().join("crates");
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = fs::read_dir(&crates)
        .unwrap_or_else(|e| panic!("read {}: {e}", crates.display()))
        .filter_map(Result::ok)
        .map(|e| e.path().join("src"))
        .filter(|p| p.is_dir())
        .collect();

    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display())) {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let code = fs::read_to_string(&path).expect("read source");
                let rel = path
                    .strip_prefix(repo_root().join("crates"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((rel, code));
            }
        }
    }
    out
}

/// Lines that are only comments do not register anything, and this file's subject is one a comment
/// has to be able to name — `install_location.rs` and `startup.rs` both explain the decision.
fn is_comment(line: &str) -> bool {
    let l = line.trim_start();
    l.starts_with("//") || l.starts_with("/*") || l.starts_with('*') || l.starts_with('#')
}

fn hits(name: &str, code: &str) -> Vec<(usize, String)> {
    code.lines()
        .enumerate()
        .filter(|(_, line)| !is_comment(line))
        .filter(|(_, line)| line.contains(name))
        .map(|(i, line)| (i + 1, line.trim().to_string()))
        .collect()
}

#[test]
fn the_bundle_registers_nothing_that_runs_on_its_own() {
    let mut found: Vec<String> = Vec::new();
    for rel in PACKAGING {
        let path = repo_root().join(rel);
        let source =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for (needle, what) in REGISTRATIONS {
            for (line, text) in hits(needle, &source) {
                found.push(format!("{rel}:{line}: {what} — {text}"));
            }
        }
    }

    assert!(
        found.is_empty(),
        "the macOS bundle would register something that runs without the user running it:\n  {}\n\n\
         FR-025: installing this application starts nothing. `docs/user-guide/install-macos.md` \
         tells users exactly that, and the removal instructions there say the app in the Trash is \
         the whole of it. Anything registered here outlives the Trash.",
        found.join("\n  ")
    );
}

#[test]
fn no_source_file_registers_a_login_item_or_launch_agent() {
    let mut found: Vec<String> = Vec::new();
    for (rel, code) in sources() {
        for (needle, what) in REGISTRATIONS {
            for (line, text) in hits(needle, &code) {
                found.push(format!("{rel}:{line}: {what} — {text}"));
            }
        }
    }

    assert!(
        found.is_empty(),
        "a source file registers something with the operating system's launch machinery:\n  {}\n\n\
         The daemon is spawned by the client and lives as long as the user's session, which is what \
         US6 and the container placement are the answer to. Registering it at login is a different \
         product decision (spec clarification, session 2026-08-27) and not one a bug fix makes.",
        found.join("\n  ")
    );
}

#[test]
fn the_scan_reads_what_it_claims_to() {
    // A scan pointed at nothing passes forever.
    for rel in PACKAGING {
        let path = repo_root().join(rel);
        assert!(
            path.is_file(),
            "{rel} does not exist; the scan covers nothing"
        );
    }
    let sources = sources();
    assert!(
        sources.len() > 50,
        "found {} source files under crates/*/src — the walk is broken, not the tree",
        sources.len()
    );
    assert!(
        sources
            .iter()
            .any(|(rel, _)| rel.contains("micold-daemon/src")),
        "the daemon's sources are not in the scan, and they are the ones that would register"
    );

    // And it can see a registration when there is one.
    let planted = "        SMAppService::mainApp().register();";
    assert_eq!(hits("SMAppService", planted).len(), 1);
    assert!(hits("SMAppService", "// SMAppService is deliberately not used").is_empty());
}
