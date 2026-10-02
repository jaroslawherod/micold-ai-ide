//! An issue's reporter never reaches a log (feature 038, FR-025, contracts/issue-fields.md §5).
//!
//! Two halves. `Issue`'s `Debug` output redacts the reporter, so a `{:?}` anywhere — a log line, a
//! panic message, a test failure pasted into a bug report — cannot print it. And the sources that
//! hold issues are scanned for logging calls that name an issue, a reporter or a description, the
//! way `ci_gate_covers_every_job.rs` scans the workflow: a text scan, because the rule is about
//! what is written in the call.

use std::fs;
use std::path::{Path, PathBuf};

use micold_core::github::Issue;

/// U28 — `{:?}` shows what identifies the issue, and not who reported it.
#[test]
fn debug_output_redacts_the_reporter() {
    const REPORTER: &str = "a-private-login";
    let issue = Issue::new(
        518,
        "Show the reporter".to_string(),
        vec!["enhancement".to_string()],
        "t".to_string(),
    )
    .reported_by(REPORTER);
    assert_eq!(
        issue.reporter(),
        REPORTER,
        "the fixture issue holds the login"
    );

    let debug = format!("{issue:?}");
    assert!(debug.contains("518"), "the number is printed: {debug}");
    assert!(
        debug.contains("Show the reporter"),
        "the title is printed: {debug}"
    );
    assert!(
        debug.contains("enhancement"),
        "the labels are printed: {debug}"
    );
    assert!(
        !debug.contains(REPORTER),
        "the reporter's login is not printed: {debug}"
    );
    let pretty = format!("{issue:#?}");
    assert!(
        !pretty.contains(REPORTER),
        "nor in the pretty form: {pretty}"
    );
}

/// The logging macros of `tracing` and `log`, as they appear at a call site.
const LOG_MACROS: &[&str] = &["trace!", "debug!", "info!", "warn!", "error!", "event!"];

/// Words a logging call must not contain: each names issue data held only while the form is open.
const FORBIDDEN: &[&str] = &["issue", "reporter", "description"];

/// Files and directories whose logging calls are checked, relative to `crates/`.
const SCANNED: &[&str] = &[
    "micold-core/src/github.rs",
    "micold-client/src/shell/issues.rs",
    "micold-client/src/features/worktree_form.rs",
    "micold-client/src/main.rs",
    "micold-client/src/ui",
];

/// Logging calls allowed to contain a forbidden word, with the reason. Empty today.
const ALLOWED: &[(&str, &str)] = &[];

fn crates_dir() -> PathBuf {
    // tests/ -> micold-core/ -> crates/
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

/// Every `.rs` file at or under `path`.
fn rust_files(path: &Path, found: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let entries = fs::read_dir(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for entry in entries {
            rust_files(&entry.expect("a directory entry").path(), found);
        }
    } else if path.extension().is_some_and(|ext| ext == "rs") {
        found.push(path.to_path_buf());
    }
}

/// The text of every logging call in `source`: from the macro's name to its closing parenthesis.
fn logging_calls(source: &str) -> Vec<&str> {
    let mut calls = Vec::new();
    for name in LOG_MACROS {
        let opener = format!("{name}(");
        let mut from = 0;
        while let Some(at) = source[from..].find(&opener) {
            let start = from + at;
            let args = start + opener.len();
            // A longer identifier that merely ends in the macro's name is not a logging call.
            let is_call = !source[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric() || c == '_');
            let mut depth = 1usize;
            let mut end = source.len();
            for (i, c) in source[args..].char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                if depth == 0 {
                    end = args + i + 1;
                    break;
                }
            }
            if is_call {
                calls.push(&source[start..end]);
            }
            from = args;
        }
    }
    calls
}

/// The forbidden word a logging call contains, if any.
fn names_issue_data(call: &str) -> Option<&'static str> {
    let lower = call.to_lowercase();
    FORBIDDEN.iter().copied().find(|word| lower.contains(word))
}

/// The scan itself is held to an example: without this, a scanner that finds nothing would pass.
#[test]
fn the_scan_finds_a_logging_call_that_names_issue_data() {
    let source = r#"
        fn load() {
            tracing::warn!(repo = %repo, "could not list: {}", (error));
            log::info!("picked {:?}", picked_issue);
            tracing::debug!(
                login = %row.reporter(),
                "row built"
            );
            my_info!("not a logging macro: issue");
            let description = 1; // not inside a call
        }
    "#;
    let calls = logging_calls(source);
    assert_eq!(
        calls.len(),
        3,
        "three logging calls, each with its whole argument list: {calls:?}"
    );
    assert!(
        calls.iter().any(|c| c.ends_with("(error))")),
        "nested parentheses stay inside the call: {calls:?}"
    );
    let named: Vec<Option<&str>> = calls.iter().map(|c| names_issue_data(c)).collect();
    assert_eq!(
        named.iter().flatten().count(),
        2,
        "the call that prints an issue and the one that prints a reporter are found: {named:?}"
    );
    assert!(
        named.contains(&Some("issue")) && named.contains(&Some("reporter")),
        "each is reported by the word it names: {named:?}"
    );
}

/// U29 — no logging call in the code that holds issues names an issue, a reporter or a
/// description.
#[test]
fn no_logging_call_names_issue_data() {
    let crates = crates_dir();
    let mut files = Vec::new();
    for scanned in SCANNED {
        let path = scanned
            .split('/')
            .fold(crates.clone(), |path, part| path.join(part));
        assert!(path.exists(), "{} exists", path.display());
        rust_files(&path, &mut files);
    }
    assert!(
        files.len() > SCANNED.len(),
        "the `ui` directory contributes its files: {} found",
        files.len()
    );

    let mut findings = Vec::new();
    for file in &files {
        let source =
            fs::read_to_string(file).unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
        for call in logging_calls(&source) {
            if ALLOWED.iter().any(|(allowed, _)| call.contains(allowed)) {
                continue;
            }
            if let Some(word) = names_issue_data(call) {
                let name = file.file_name().unwrap_or_default().to_string_lossy();
                findings.push(format!("{name}: names `{word}` in `{call}`"));
            }
        }
    }
    assert!(
        findings.is_empty(),
        "issues, reporters and descriptions are never logged (FR-025):\n{}",
        findings.join("\n")
    );
}
