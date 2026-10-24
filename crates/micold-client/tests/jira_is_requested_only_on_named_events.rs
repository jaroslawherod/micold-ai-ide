//! Jira is contacted only on the named events (feature 533, FR-001, FR-020, SC-001, FR-019).
//!
//! A request passes one gate (`jira_request` in `shell/issues.rs`): the switch and the project's
//! connection, checked by running it in `jira_request_path` (`main_tests.rs`). The load is reached
//! only when the reducer accepted `SourceChanged(Issue)` or `IssueRetry` (the allow-list of
//! `issues_are_requested_only_on_named_events.rs`). Scanned here: the search is called only when a
//! search is due, the adapter is constructed in `Capabilities::real()` and nowhere else, and no Jira
//! issue type is serialisable.

#[path = "support/source_scan.rs"]
mod source_scan;

use source_scan::rs_files;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("canonicalize repository root")
}

fn code_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.starts_with("//"))
        .collect()
}

/// Every non-comment line of the client's sources that contains `needle`, as `(file, line)`.
fn occurrences(needle: &str) -> Vec<(String, String)> {
    let root = root();
    let mut found = Vec::new();
    for file in rs_files(&root.join("crates/micold-client/src")) {
        let rel = file
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        for line in code_lines(&file) {
            if line.contains(needle) {
                found.push((rel.clone(), line));
            }
        }
    }
    found
}

/// What no behaviour reveals: a second caller of the capability would bypass the gate that
/// `jira_request_path` runs. The file, not the line, is pinned, so a refactor inside it passes.
#[test]
fn the_source_is_asked_for_only_in_the_gate_module_and_once() {
    let uses = occurrences(".jira_source()");
    assert_eq!(uses.len(), 1, "{uses:?}");
    assert_eq!(
        uses[0].0, "crates/micold-client/src/shell/issues.rs",
        "{uses:?}"
    );
}

// The gate itself (switch off, no connection, no token file, a rotated token) is run by
// `jira_request_path` in `main_tests.rs`; what stays here is the structure no behaviour reveals.

/// The body of `name` in the shell's issue module, up to the next top-level item.
fn function_body(name: &str) -> String {
    let path = root().join("crates/micold-client/src/shell/issues.rs");
    let text = fs::read_to_string(path).unwrap();
    let start = text.find(name).unwrap_or_else(|| panic!("{name} exists"));
    let rest = &text[start..];
    rest[..rest.find("\n}\n").expect("the function ends")].to_string()
}

#[test]
fn the_search_function_is_called_only_when_a_search_is_due() {
    let calls: Vec<_> = occurrences("start_jira_search(")
        .into_iter()
        .filter(|(_, line)| !line.starts_with("fn start_jira_search("))
        .collect();
    assert_eq!(calls.len(), 1, "{calls:?}");
    let dispatcher = function_body("fn start_issue_search(");
    assert!(dispatcher.contains("start_jira_search("), "{dispatcher}");
    let reached_from = ["on_issue_search_due", "retry_issue_search"];
    for name in reached_from {
        assert!(
            function_body(&format!("fn {name}(")).contains("start_issue_search("),
            "{name}"
        );
    }
}

#[test]
fn the_adapter_is_constructed_only_in_the_real_capabilities() {
    let built = occurrences("JiraCloud::");
    assert_eq!(
        built,
        vec![(
            "crates/micold-client/src/shell/capabilities.rs".to_string(),
            "jira_source: Some(Arc::new(crate::shell::jira_http::JiraCloud::new())),".to_string()
        )],
        "{built:?}"
    );
}

#[test]
fn no_jira_issue_type_derives_serialize() {
    let root = root();
    for file in [
        "crates/micold-core/src/github.rs",
        "crates/micold-core/src/jira.rs",
    ] {
        let lines = code_lines(&root.join(file));
        for (i, line) in lines.iter().enumerate() {
            let names = [
                "pub struct Issue {",
                "pub struct IssuePage {",
                "pub struct IssueListing {",
            ];
            if names.contains(&line.as_str()) {
                let derives: String = lines[..i]
                    .iter()
                    .rev()
                    .take_while(|l| l.starts_with("#["))
                    .cloned()
                    .collect();
                assert!(!derives.contains("Serialize"), "{file}: {line}: {derives}");
            }
        }
    }
}

/// Feature 1101 (FR-005): a status read starts on two events only. `trigger` in
/// `shell/jira_refresh.rs` is the one starter; it is called from the window-focus handler and from
/// the project-open path, and the two public entry points are called from one place each.
#[test]
fn a_status_read_starts_only_from_focus_and_project_open() {
    let calls: Vec<_> = occurrences("trigger(app, Trigger::")
        .into_iter()
        .map(|(file, line)| format!("{file}: {line}"))
        .collect();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert!(calls
        .iter()
        .all(|c| c.starts_with("crates/micold-client/src/shell/jira_refresh.rs")));
    // The driver tests in `main_tests.rs` call the entry points too; they start no read of their own.
    assert_eq!(
        occurrences("jira_refresh::on_focus(")
            .iter()
            .filter(|(file, _)| !file.ends_with("main_tests.rs"))
            .count(),
        1
    );
    // The driver tests in `main_tests.rs` call the entry points too; they start no read of their own.
    assert_eq!(
        occurrences("jira_refresh::after_update(")
            .iter()
            .filter(|(file, _)| !file.ends_with("main_tests.rs"))
            .count(),
        1
    );
    // The status read reaches Jira through the same gate as the picker's.
    assert_eq!(occurrences(".jira_source()").len(), 1);
}

/// Feature 1186 (FR-004, FR-016): the advanced selector's modules, its view among them, log nothing and notify nothing,
/// so no log line, toast or notification can take the draft, the last run query, a `UserQuery` or a
/// `ScopedQuery`. Checked as "none at all", which also catches a macro whose arguments span lines.
#[test]
fn the_advanced_query_is_never_logged_or_notified() {
    let root = root();
    let calls = [
        "tracing::",
        "log::",
        "info!(",
        "warn!(",
        "error!(",
        "debug!(",
        "trace!(",
        "eprintln!(",
        "println!(",
        "notify",
        "toast",
        "Notification",
    ];
    // Each module with what shows it holds the query: the scoped query, or the view's draft.
    for (file, holds) in [
        (
            "crates/micold-client/src/features/worktree_form/jira_query.rs",
            "ScopedQuery",
        ),
        (
            "crates/micold-client/src/shell/jira_query.rs",
            "ScopedQuery",
        ),
        ("crates/micold-client/src/ui/jira_query.rs", "query.draft"),
    ] {
        let lines = code_lines(&root.join(file));
        assert!(
            lines.iter().any(|l| l.contains(holds)),
            "{file}: the scan reads the module that holds the query"
        );
        let offenders: Vec<_> = lines
            .iter()
            .filter(|l| calls.iter().any(|c| l.contains(c)))
            .collect();
        assert!(offenders.is_empty(), "{file}: {offenders:?}");
    }
}
