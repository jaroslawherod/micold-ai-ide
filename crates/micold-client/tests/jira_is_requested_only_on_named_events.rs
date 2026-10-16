//! Jira is contacted only on the named events (feature 533, FR-001, FR-020, SC-001, FR-019).
//!
//! The Jira source is reached through one capability (`Capabilities::jira_source`), called in one
//! function (`start_jira_load` in `shell/issues.rs`), which `start_issue_load` calls — and that is
//! called only when the reducer accepted `SourceChanged(Issue)` or `IssueRetry` (the allow-list of
//! `issues_are_requested_only_on_named_events.rs`). The function checks the switch and the
//! project's connection before it asks for the source. The adapter is constructed in
//! `Capabilities::real()` and nowhere else, and no Jira issue type is serialisable.

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

#[test]
fn the_source_is_asked_for_only_inside_the_load_function() {
    let uses = occurrences(".jira_source()");
    assert_eq!(
        uses,
        vec![(
            "crates/micold-client/src/shell/issues.rs".to_string(),
            "let source = app.caps.jira_source();".to_string()
        )],
        "{uses:?}"
    );
}

#[test]
fn the_one_gate_checks_the_switch_and_the_connection_before_the_source() {
    let path = root().join("crates/micold-client/src/shell/issues.rs");
    let text = fs::read_to_string(path).unwrap();
    let start = text.find("fn jira_request(").expect("jira_request exists");
    let body = &text[start..];
    let source = body.find(".jira_source()").expect("asks for the source");
    let switch = body.find("jira_enabled").expect("checks the switch");
    let connection = body
        .find(".jira_connections")
        .expect("looks the connection up");
    assert!(switch < source && connection < source, "checks come first");
}

#[test]
fn the_load_function_is_called_only_by_the_general_load_path() {
    let calls: Vec<_> = occurrences("start_jira_load(")
        .into_iter()
        .filter(|(_, line)| !line.starts_with("fn start_jira_load("))
        .collect();
    assert_eq!(
        calls,
        vec![(
            "crates/micold-client/src/shell/issues.rs".to_string(),
            "return start_jira_load(app, seq);".to_string()
        )],
        "{calls:?}"
    );
}

/// The body of `name` in the shell's issue module, up to the next top-level item.
fn function_body(name: &str) -> String {
    let path = root().join("crates/micold-client/src/shell/issues.rs");
    let text = fs::read_to_string(path).unwrap();
    let start = text.find(name).unwrap_or_else(|| panic!("{name} exists"));
    let rest = &text[start..];
    rest[..rest.find("\n}\n").expect("the function ends")].to_string()
}

#[test]
fn the_search_is_asked_for_only_inside_the_search_function_behind_the_gate() {
    let uses = occurrences(".search_open(connection");
    let files: Vec<_> = uses.iter().map(|(file, _)| file.as_str()).collect();
    assert_eq!(
        files,
        ["crates/micold-client/src/shell/issues.rs"],
        "{uses:?}"
    );
    let search = function_body("fn start_jira_search(");
    assert!(
        search.contains("jira_request("),
        "the search goes through the same gate as the load"
    );
    assert!(
        search.find("jira_request(") < search.find(".search_open(connection"),
        "the gate comes first"
    );
    assert!(function_body("fn start_jira_load(").contains("jira_request("));
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
