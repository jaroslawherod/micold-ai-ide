//! A desktop notification registers nothing and reads nothing from outside (feature 039).
//!
//! The notification for "a session needs you" is deliberately inert: clicking it only brings the
//! app forward (FR-015a), so there is no toast activator, no URL protocol handler, and no
//! command-line argument the client interprets. And the daemon stays out of the desktop session
//! altogether (FR-007): it never links a notification crate, so it cannot notify from a headless
//! host. The one thing that *must* be registered is the Windows Application User Model ID, because
//! Windows shows a toast only for an ID that a Start-menu shortcut carries — and that string lives
//! in two files that nothing else ties together.
//!
//! Each of those is a promise about what is *absent* (or about two copies agreeing), the kind that
//! decays quietly: a protocol handler is a plausible way to "make the click open the session", and
//! it would be a new attack surface nobody decided on. So they are checked properties here.
//!
//! # What is scanned, and what is not
//!
//! The installer script, the macOS plist template, every Rust file of the client (for the command
//! line, every one of them: an argument read in a module is as read as one in `main.rs`) and the
//! daemon's manifest. Comments are skipped, so the files may explain the decision in their own words.
//! Not `tests/`: this file names the very strings it forbids.

use std::fs;
use std::path::{Path, PathBuf};

const ISS: &str = "packaging/windows/micold-ai-ide.iss";
const PLIST: &str = "packaging/macos/Info.plist.in";
const WINDOWS_RS: &str = "crates/micold-client/src/shell/desktop_notify/windows.rs";
const MAIN_RS: &str = "crates/micold-client/src/main.rs";
const DAEMON_TOML: &str = "crates/micold-daemon/Cargo.toml";

const CLIENT_SRC: &str = "crates/micold-client/src";

const SCANNED: &[&str] = &[ISS, PLIST, WINDOWS_RS, MAIN_RS, DAEMON_TOML];

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Numbered, trimmed lines that are not comments according to `is_comment`.
fn code_lines(text: &str, is_comment: fn(&str) -> bool) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter(|(_, l)| !is_comment(l))
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// Inno Setup comments start with `;`.
fn inno_comment(line: &str) -> bool {
    line.trim_start().starts_with(';')
}

/// TOML comments start with `#`.
fn toml_comment(line: &str) -> bool {
    line.trim_start().starts_with('#')
}

/// A Rust line comment (`//`, which takes in `///` and `//!`). Block comments are removed first,
/// by [`strip_rust_block_comments`]: a line that starts with `*` may be code, the continuation of
/// a product.
fn rust_comment(line: &str) -> bool {
    line.trim_start().starts_with("//")
}

/// Replaces every `/* … */` span, which may cross lines, with nothing but its newlines so that line
/// numbers stay true.
fn strip_rust_block_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find("*/").map_or(after.len(), |e| e + 2);
        out.extend(after[..end].chars().filter(|c| *c == '\n'));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The code lines of a Rust file: its comments of both kinds left out.
fn rust_code_lines(text: &str) -> Vec<(usize, String)> {
    code_lines(&strip_rust_block_comments(text), rust_comment)
}

/// Every `.rs` file under `rel`, repo-relative, sorted.
fn rust_files_under(rel: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut dirs = vec![repo_root().join(rel)];
    while let Some(dir) = dirs.pop() {
        let entries =
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let relative = path
                    .strip_prefix(repo_root())
                    .expect("under the repository")
                    .to_string_lossy()
                    .replace('\\', "/");
                found.push(relative);
            }
        }
    }
    found.sort();
    found
}

/// What U48 forbids: the ways a Rust program reads its command line.
const ARGUMENT_READERS: &[&str] = &["env::args", "args_os", "clap", "pico_args"];

/// Replaces every `<!-- … -->` span, which may cross lines, with nothing but its newlines so
/// that line numbers stay true.
fn strip_xml_comments(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let end = after.find("-->").map_or(after.len(), |e| e + 3);
        out.extend(after[..end].chars().filter(|c| *c == '\n'));
        rest = &after[end..];
    }
    out.push_str(rest);
    out
}

/// The `file:line: text` of every non-comment line that contains one of `needles`.
fn offending(
    rel: &str,
    lines: &[(usize, String)],
    needles: &[&str],
    ignore_case: bool,
) -> Vec<String> {
    let mut found = Vec::new();
    for (n, text) in lines {
        let hay = if ignore_case {
            text.to_lowercase()
        } else {
            text.clone()
        };
        for needle in needles {
            let needle_cmp = if ignore_case {
                needle.to_lowercase()
            } else {
                (*needle).to_string()
            };
            if hay.contains(&needle_cmp) {
                found.push(format!("{rel}:{n}: names `{needle}` — {text}"));
            }
        }
    }
    found
}

/// The quoted value of `APP_USER_MODEL_ID` in the client's Windows backend.
fn app_user_model_id_from_source() -> String {
    let source = read(WINDOWS_RS);
    let (n, line) = rust_code_lines(&source)
        .into_iter()
        .find(|(_, l)| l.contains("const APP_USER_MODEL_ID"))
        .unwrap_or_else(|| panic!("{WINDOWS_RS}: no `const APP_USER_MODEL_ID` line"));
    let after_eq = line
        .split_once('=')
        .unwrap_or_else(|| panic!("{WINDOWS_RS}:{n}: no `=` in `{line}`"))
        .1;
    let mut quoted = after_eq.split('"');
    quoted.next();
    quoted
        .next()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| panic!("{WINDOWS_RS}:{n}: no quoted value in `{line}`"))
        .to_string()
}

/// The non-comment lines of the installer's `[Icons]` section.
fn icons_section(iss: &str) -> Vec<(usize, String)> {
    let mut in_icons = false;
    let mut out = Vec::new();
    for (n, line) in code_lines(iss, inno_comment) {
        if line.starts_with('[') {
            in_icons = line.eq_ignore_ascii_case("[Icons]");
        } else if in_icons && !line.is_empty() {
            out.push((n, line));
        }
    }
    out
}

// U46 (FR-029): the Start-menu shortcut carries the same Application User Model ID the client
// passes to the toast.
#[test]
fn the_start_menu_shortcut_carries_the_id_the_client_uses() {
    let id = app_user_model_id_from_source();
    let iss = read(ISS);
    let icons = icons_section(&iss);
    let shortcut = icons
        .iter()
        .find(|(_, l)| l.contains("Name: \"{autoprograms}\\"))
        .unwrap_or_else(|| {
            panic!("{ISS}: no Start-menu shortcut (Name starting `{{autoprograms}}\\`) in [Icons]")
        });
    let wanted = format!("AppUserModelID: \"{id}\"");
    assert!(
        shortcut.1.contains(&wanted),
        "{ISS}:{}: the Start-menu shortcut does not carry `{wanted}` — {}\n\n\
         FR-029: Windows shows a toast only for an Application User Model ID that a Start-menu \
         shortcut carries, and `{WINDOWS_RS}` passes `{id}` to the toast.",
        shortcut.0,
        shortcut.1
    );
}

// U47 (FR-015a): clicking the notification only focuses the app, so nothing is registered that
// could deliver an activation to it — on Windows or on macOS.
#[test]
fn the_packaging_registers_no_activator_and_no_protocol_handler() {
    let mut found = offending(
        ISS,
        &code_lines(&read(ISS), inno_comment),
        &[
            "ToastActivatorCLSID",
            "AppUserModelToastActivatorCLSID",
            "CustomActivator",
            "URL Protocol",
            "Software\\Classes",
        ],
        true,
    );
    let plist = strip_xml_comments(&read(PLIST));
    found.extend(offending(
        PLIST,
        &code_lines(&plist, |_| false),
        &["CFBundleURLTypes", "CFBundleURLSchemes"],
        false,
    ));
    assert!(
        found.is_empty(),
        "the packaging registers a toast activator or a protocol handler:\n  {}\n\n\
         FR-015a: the notification click only brings the app forward, and nothing outside the \
         app can start it with an instruction.",
        found.join("\n  ")
    );
}

// U48 (FR-015a, contract N8): the client has no command line to interpret.
#[test]
fn the_client_reads_no_command_line_argument() {
    let files = rust_files_under(CLIENT_SRC);
    assert!(
        files.iter().any(|f| f == MAIN_RS),
        "the walk of {CLIENT_SRC} does not reach {MAIN_RS}: {files:?}"
    );
    let found: Vec<String> = files
        .iter()
        .flat_map(|rel| offending(rel, &rust_code_lines(&read(rel)), ARGUMENT_READERS, false))
        .collect();
    assert!(
        found.is_empty(),
        "the client reads a command-line argument:\n  {}\n\n\
         Contract N8: a notification click carries no instruction, so the client interprets no \
         argument that one could be smuggled in through.",
        found.join("\n  ")
    );
}

// U49 (FR-007): the daemon never notifies the desktop; it has no notification crate.
#[test]
fn the_daemon_depends_on_no_notification_crate() {
    let found = offending(
        DAEMON_TOML,
        &code_lines(&read(DAEMON_TOML), toml_comment),
        &["zbus", "mac-usernotifications", "tauri-winrt-notification"],
        false,
    );
    assert!(
        found.is_empty(),
        "the daemon's manifest names a desktop notification crate:\n  {}\n\n\
         FR-007: the daemon runs on hosts with no desktop session; the client notifies.",
        found.join("\n  ")
    );
}

#[test]
fn the_scan_reads_what_it_claims_to() {
    // A scan pointed at nothing passes forever.
    for rel in SCANNED {
        let path = repo_root().join(rel);
        assert!(
            path.is_file(),
            "{rel} does not exist; the scan covers nothing"
        );
        assert!(
            !read(rel).trim().is_empty(),
            "{rel} is empty; the scan covers nothing"
        );
    }
    assert!(
        !icons_section(&read(ISS)).is_empty(),
        "{ISS}: no [Icons] lines"
    );

    // And it can see a violation when there is one.
    let planted =
        "<key>a</key>\n<!-- CFBundleURLTypes\nstill comment -->\n<key>CFBundleURLTypes</key>";
    let lines = code_lines(&strip_xml_comments(planted), |_| false);
    assert_eq!(
        offending("x", &lines, &["CFBundleURLTypes"], false).len(),
        1
    );
    // A line that starts with `*` is code when it continues an expression, and a block comment
    // that crosses lines is a comment on every one of them (issue #572).
    let planted = "fn main() {\n    let n = 2\n        * std::env::args().count();\n    /* env::args\n    args_os */\n}";
    assert_eq!(
        offending("x", &rust_code_lines(planted), ARGUMENT_READERS, false),
        vec!["x:3: names `env::args` — * std::env::args().count();".to_string()],
        "the continuation line is read as code, and the block comment is not"
    );
    assert!(offending(
        "x",
        &code_lines("; URL Protocol", inno_comment),
        &["URL Protocol"],
        true
    )
    .is_empty());
}
