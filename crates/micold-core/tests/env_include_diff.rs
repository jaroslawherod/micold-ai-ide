//! Pure tests for the environment-include diff/parse/merge functions (feature 011, research
//! R3/R6, contracts/env-include-resolution.md). No subprocess — runs under `cargo test
//! --no-default-features`.

use micold_core::env_include::{diff_env, merge_with_term, parse_env_dump};
use std::collections::HashMap;

#[test]
fn parse_env_dump_splits_nul_delimited_pairs() {
    let dump = b"FOO=bar\0BAZ=qux\0";
    let parsed = parse_env_dump(dump);
    assert_eq!(parsed.get("FOO"), Some(&"bar".to_string()));
    assert_eq!(parsed.get("BAZ"), Some(&"qux".to_string()));
    assert_eq!(parsed.len(), 2);
}

#[test]
fn parse_env_dump_handles_value_containing_equals() {
    let dump = b"CONNECTION_STRING=key=value;other=1\0";
    let parsed = parse_env_dump(dump);
    assert_eq!(
        parsed.get("CONNECTION_STRING"),
        Some(&"key=value;other=1".to_string())
    );
}

#[test]
fn parse_env_dump_handles_empty_input() {
    assert!(parse_env_dump(b"").is_empty());
}

#[test]
fn diff_env_reports_new_and_changed_keys() {
    let mut baseline = HashMap::new();
    baseline.insert("EXISTING".to_string(), "old".to_string());
    baseline.insert("UNCHANGED".to_string(), "same".to_string());

    let mut attempt = baseline.clone();
    attempt.insert("EXISTING".to_string(), "new".to_string());
    attempt.insert("BRAND_NEW".to_string(), "value".to_string());

    let diff_map: HashMap<_, _> = diff_env(&baseline, &attempt).into_iter().collect();

    assert_eq!(diff_map.get("EXISTING"), Some(&"new".to_string()));
    assert_eq!(diff_map.get("BRAND_NEW"), Some(&"value".to_string()));
    assert!(!diff_map.contains_key("UNCHANGED"));
}

#[test]
fn diff_env_does_not_report_removed_keys() {
    let mut baseline = HashMap::new();
    baseline.insert("REMOVED".to_string(), "was-here".to_string());
    let attempt = HashMap::new();

    assert!(diff_env(&baseline, &attempt).is_empty());
}

#[test]
fn diff_env_empty_baseline_and_attempt_yields_empty_diff() {
    assert!(diff_env(&HashMap::new(), &HashMap::new()).is_empty());
}

// merge_with_term (T010) — TERM precedence, FR-009.
#[test]
fn merge_with_term_appends_term_when_absent() {
    let vars = vec![("FOO".to_string(), "bar".to_string())];
    let merged = merge_with_term(&vars);
    assert_eq!(
        merged.last(),
        Some(&("TERM".to_string(), "xterm-256color".to_string()))
    );
    assert_eq!(merged.len(), 2);
}

#[test]
fn merge_with_term_hardcoded_value_wins_over_captured_term() {
    let vars = vec![
        ("TERM".to_string(), "captured-bad-value".to_string()),
        ("OTHER".to_string(), "value".to_string()),
    ];
    let merged = merge_with_term(&vars);

    let term_entries: Vec<_> = merged.iter().filter(|(k, _)| k == "TERM").collect();
    assert_eq!(
        term_entries.len(),
        1,
        "exactly one TERM entry must survive the merge"
    );
    assert_eq!(term_entries[0].1, "xterm-256color");
    assert!(merged.contains(&("OTHER".to_string(), "value".to_string())));
}
