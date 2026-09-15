//! The Windows installer copes with an app that is running, and keeps the user's data (feature 030,
//! FR-007, FR-009, FR-023).
//!
//! Replacing the exes needs the app closed and the daemon stopped; a miss surfaces only as a
//! "files in use" failure on a user's machine. Uninstalling must remove the daemon's runtime dir and
//! nothing the user owns. Each of these is written in `packaging/windows/micold-ai-ide.iss`, so they
//! are asserted there. Text scans, as in `windows_installer_is_per_user.rs`.

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

/// The value of every `name=value` directive called `name`, in order (names are case-insensitive).
fn directive_values<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    directive_lines(script)
        .into_iter()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| key.trim().eq_ignore_ascii_case(name))
        .map(|(_, value)| value.trim())
        .collect()
}

#[test]
fn app_mutex_matches_the_running_app() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "AppMutex"),
        vec![micold_core::process::APP_MUTEX_NAME],
        "setup detects a running app only through the mutex the app holds (FR-009)"
    );
}

#[test]
fn closes_the_app_window_when_the_user_continues() {
    let script = iss();

    assert_eq!(
        directive_values(&script, "CloseApplications"),
        vec!["force"],
        "setup must close a still-open app window through Restart Manager, not leave its exe locked \
         (FR-009)"
    );
}

/// The `[Code]` section's lines, with `//` comments removed. It runs to the next `[Section]` header or
/// the end of the script.
fn code_section(script: &str) -> Vec<&str> {
    let mut in_code = false;
    let mut lines = Vec::new();
    for line in script.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_code = trimmed.eq_ignore_ascii_case("[Code]");
            continue;
        }
        if in_code {
            lines.push(line.split("//").next().unwrap_or_default());
        }
    }
    lines
}

/// The body of the Pascal routine `name`: its header line through the unindented `end;` that closes
/// it. `None` when the section does not define it.
fn routine_body(code: &[&str], name: &str) -> Option<String> {
    let header = |line: &str| {
        let lower = line.trim().to_ascii_lowercase();
        ["function ", "procedure "].iter().any(|kind| {
            lower
                .strip_prefix(kind)
                .is_some_and(|rest| rest.starts_with(&name.to_ascii_lowercase()))
                && lower[kind.len() + name.len()..].starts_with(['(', ':', ';'])
        })
    };
    let start = code.iter().position(|line| header(line))?;
    let end = code[start..]
        .iter()
        .position(|line| line.trim_end().eq_ignore_ascii_case("end;"))
        .map(|offset| start + offset)?;
    Some(code[start..=end].join("\n"))
}

#[test]
fn install_and_uninstall_stop_the_daemon_first() {
    let script = iss();
    let code = code_section(&script);

    for hook in ["PrepareToInstall", "InitializeUninstall"] {
        let body = routine_body(&code, hook)
            .unwrap_or_else(|| panic!("[Code] must define `{hook}` (FR-009, FR-023)"));
        assert!(
            body.contains("StopDaemon"),
            "`{hook}` must call `StopDaemon`: Restart Manager cannot close the windowless daemon, which \
             keeps its exe locked (FR-009, FR-023); the body is:\n{body}"
        );
    }
}

/// The non-comment entries under every `[name]` header (case-insensitive), in order.
fn section_entries<'a>(script: &'a str, name: &str) -> Vec<&'a str> {
    let mut inside = false;
    let mut entries = Vec::new();
    for line in directive_lines(script) {
        if line.starts_with('[') && line.ends_with(']') {
            inside = line.eq_ignore_ascii_case(&format!("[{name}]"));
        } else if inside {
            entries.push(line);
        }
    }
    entries
}

/// The daemon's runtime dir: its pid record and socket, rebuilt on every start.
const RUNTIME_DIR_ENTRY: &str = r#"Type: filesandordirs; Name: "{localappdata}\micold-ai-ide\run""#;

#[test]
fn uninstall_removes_only_the_runtime_dir() {
    let script = iss();

    assert_eq!(
        section_entries(&script, "UninstallDelete"),
        vec![RUNTIME_DIR_ENTRY],
        "uninstall must remove the daemon's runtime dir and nothing else of the user's (FR-007)"
    );
}

/// Where the user's settings and session data live. Uninstall and upgrade never delete them.
const USER_DATA_DIRS: [&str; 2] = ["{userappdata}", r"{localappdata}\micold-ai-ide\data"];

#[test]
fn never_deletes_user_data() {
    let script = iss();

    for section in ["UninstallDelete", "InstallDelete"] {
        let offending: Vec<&str> = section_entries(&script, section)
            .into_iter()
            .filter(|entry| {
                let entry = entry.to_ascii_lowercase();
                USER_DATA_DIRS
                    .iter()
                    .any(|dir| entry.contains(&dir.to_ascii_lowercase()))
            })
            .collect();
        assert!(
            offending.is_empty(),
            "[{section}] must not delete the user's settings or session data (FR-007); found \
             {offending:?}"
        );
    }
}

/// What the ready page tells the user before they confirm (FR-009: the prompt names the consequence).
const SESSIONS_STOP_NOTICE: &str = "Running sessions will be stopped.";

#[test]
fn ready_page_says_sessions_will_stop() {
    let script = iss();
    let code = code_section(&script);

    let body = routine_body(&code, "UpdateReadyMemo").expect(
        "[Code] must define `UpdateReadyMemo` to add the notice to the ready page (FR-009)",
    );
    assert!(
        body.contains(SESSIONS_STOP_NOTICE),
        "the ready page must say `{SESSIONS_STOP_NOTICE}` before setup stops the daemon (FR-009); the \
         body is:\n{body}"
    );
}
