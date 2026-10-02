//! A pull request's status lives only in memory (feature 040, FR-032, SC-011, data-model §1): the
//! type cannot be serialised, and what it prints for a log holds neither the title nor the address.

use std::path::PathBuf;

use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};

/// The source text of the `PullRequestStatus` item: its attributes and its `pub struct` line.
fn declaration() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/pull_request.rs");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with("pub struct PullRequestStatus"))
        .expect("src/pull_request.rs declares `pub struct PullRequestStatus`");
    // The attributes and doc comments directly above the struct belong to it.
    let first = lines[..at]
        .iter()
        .rposition(|line| !(line.starts_with("#[") || line.starts_with("///")))
        .map_or(0, |blank| blank + 1);
    lines[first..=at].join("\n")
}

#[test]
fn the_status_derives_no_serialisation_and_no_debug() {
    let declaration = declaration();
    assert!(
        declaration.contains("#[derive("),
        "the gate reads the derive line above the struct; found:\n{declaration}"
    );
    for forbidden in ["Serialize", "Deserialize", "Debug"] {
        assert!(
            !declaration.contains(forbidden),
            "PullRequestStatus must not derive {forbidden}: a derived one would write the title \
             and the address to a stored file or a log (FR-032); found:\n{declaration}"
        );
    }
}

#[test]
fn debug_output_holds_the_number_and_the_enums_and_neither_title_nor_address() {
    const TITLE: &str = "Rework the billing export";
    const URL: &str = "https://github.com/acme/widgets/pull/4711";
    const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";
    let status = PullRequestStatus {
        number: 4711,
        title: TITLE.to_string(),
        url: URL.to_string(),
        state: PrState::Open {
            checks: CheckStatus::Failing,
        },
        review: ReviewState::ChangesRequested,
        head: HEAD.to_string(),
    };

    let printed = format!("{status:?}");
    let pretty = format!("{status:#?}");

    for output in [&printed, &pretty] {
        assert!(output.contains("4711"), "the number is printed: {output}");
        assert!(
            output.contains("Open") && output.contains("Failing"),
            "the state and its check status are printed: {output}"
        );
        assert!(
            output.contains("ChangesRequested"),
            "the review state is printed: {output}"
        );
        assert!(
            !output.contains(TITLE) && !output.contains("billing"),
            "the title is never printed (FR-032): {output}"
        );
        assert!(
            !output.contains(URL) && !output.contains("github.com"),
            "the address is never printed (FR-032): {output}"
        );
        assert!(
            !output.contains(HEAD),
            "data-model §1: Debug prints number, state and review and nothing else: {output}"
        );
    }
}
