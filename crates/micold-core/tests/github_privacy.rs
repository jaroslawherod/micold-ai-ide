//! An issue's reporter never reaches a log (feature 038, FR-025, contracts/issue-fields.md §5).
//!
//! Two halves. `Issue`'s `Debug` output redacts the reporter, so a `{:?}` anywhere — a log line, a
//! panic message, a test failure pasted into a bug report — cannot print it. And the sources that
//! hold issues are scanned for logging calls that name an issue, a reporter or a description, the
//! way `ci_gate_covers_every_job.rs` scans the workflow: a text scan, because the rule is about
//! what is written in the call.

use std::fs;
use std::path::{Path, PathBuf};

use micold_core::github::{parse_list_page, parse_search, GithubRepo, Issue};

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

/// U28 — `{:?}` does not print what the issue's body says.
#[test]
fn debug_output_redacts_the_description() {
    const DESCRIPTION: &str = "a-private-sentence about the customer";
    let issue = Issue::new(518, "Show the description".to_string(), vec![], "t".to_string())
        .described(DESCRIPTION);
    assert_eq!(
        issue.description(),
        DESCRIPTION,
        "the fixture issue holds the description"
    );

    for debug in [format!("{issue:?}"), format!("{issue:#?}")] {
        assert!(debug.contains("518"), "the number is printed: {debug}");
        assert!(
            debug.contains("Show the description"),
            "the title is printed: {debug}"
        );
        assert!(
            debug.contains("description: \"<redacted>\""),
            "the description's place is marked: {debug}"
        );
        assert!(
            !debug.contains("a-private-sentence") && !debug.contains("customer"),
            "no part of the description is printed: {debug}"
        );
    }
}

/// U30 — a page that cannot be read gives an error that says so and repeats none of the page: a
/// body in it reaches neither the form's message nor a log.
#[test]
fn a_malformed_page_with_a_body_gives_an_error_without_it() {
    const BODY: &str = "a-private-sentence";
    let repo = GithubRepo::from_remote_url("https://github.com/o/r").expect("a GitHub remote");
    let pages = [
        // A node without a number, beside its body.
        format!(
            r#"{{"data":{{"repository":{{"issues":{{"totalCount":1,
                "pageInfo":{{"hasNextPage":false,"endCursor":null}},
                "nodes":[{{"title":"T","bodyText":"{BODY}"}}]}}}}}}}}"#
        ),
        // A body of the wrong type, and no issue list.
        format!(r#"{{"data":{{"repository":{{"bodyText":"{BODY}"}}}}}}"#),
        // Not JSON: cut inside the body.
        format!(r#"{{"data":{{"repository":{{"issues":{{"nodes":[{{"bodyText":"{BODY}"#),
        // Not JSON: an unexpected token after the body.
        format!(r#"{{"data":{{"bodyText":"{BODY}" {BODY} }}}}"#),
    ];
    for page in &pages {
        for (parser, result) in [
            ("parse_list_page", parse_list_page(page.as_bytes()).map(drop)),
            ("parse_search", parse_search(page.as_bytes()).map(drop)),
        ] {
            let Err(error) = result else {
                // A search answer has no issue list to miss: a shape it reads as "no hits" is fine.
                assert_eq!(parser, "parse_search", "{parser} refuses {page}");
                continue;
            };
            // `message` is the error's display form: what the form shows under the field.
            for shown in [
                format!("{error:?}"),
                format!("{error:#?}"),
                error.message(&repo),
            ] {
                assert!(
                    !shown.contains(BODY) && !shown.contains("private"),
                    "{parser}'s error carries no part of the body: {shown}"
                );
            }
        }
    }
}

/// U81 — `Issue` cannot be serialized: neither derived nor implemented by hand. A source check,
/// because the absence of an impl cannot be asserted on a value.
#[test]
fn an_issue_has_no_serialize() {
    let path = crates_dir().join("micold-core").join("src").join("github.rs");
    let source =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let at = source
        .find("pub struct Issue {")
        .expect("`Issue` is declared in github.rs");
    let attributes: Vec<&str> = source[..at]
        .rsplit("\n\n")
        .next()
        .expect("the lines above the declaration")
        .lines()
        .filter(|line| line.trim_start().starts_with("#["))
        .collect();
    let attributes = attributes.join("\n");
    assert!(
        attributes.contains("#[derive("),
        "the scan sees the derive above `Issue`: {attributes}"
    );
    assert!(
        !attributes.contains("Serialize"),
        "`Issue` derives no `Serialize` (FR-025): {attributes}"
    );
    let compact: String = source.split_whitespace().collect::<Vec<_>>().join(" ");
    for hand_written in ["Serialize for Issue", "Serialize for &Issue"] {
        assert!(
            !compact.contains(hand_written),
            "`Issue` implements no `Serialize` by hand (FR-025)"
        );
    }
    assert!(
        !compact.contains("serde::Serialize") && !compact.contains("use serde::{Serialize"),
        "nothing in github.rs is serializable, so no type holding an `Issue` can be"
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
