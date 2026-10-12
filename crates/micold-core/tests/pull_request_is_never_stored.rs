//! A pull request's status lives only in memory (feature 040, FR-032, SC-011, data-model §1): the
//! type cannot be serialised, and what it prints for a log holds neither the title nor the address.

use std::path::PathBuf;

use micold_core::pull_request::{
    BranchPrState, BranchPullRequest, CheckStatus, PrState, PullRequestStatus, ReviewState,
};

/// The source text of the `PullRequestStatus` item: its attributes and its `pub struct` line.
fn declaration() -> String {
    declaration_of("PullRequestStatus")
}

/// The source text of the struct `name`: its attributes and its `pub struct` line.
fn declaration_of(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/pull_request.rs");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let lines: Vec<&str> = source.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.starts_with(&format!("pub struct {name}")))
        .unwrap_or_else(|| panic!("src/pull_request.rs declares `pub struct {name}`"));
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

#[test]
fn a_listed_pull_request_derives_no_serialisation_and_prints_no_title_or_address() {
    let declaration = declaration_of("BranchPullRequest");
    for forbidden in ["Serialize", "Deserialize", "Debug"] {
        assert!(
            !declaration.contains(forbidden),
            "BranchPullRequest must not derive {forbidden}; found:\n{declaration}"
        );
    }
    let listed = BranchPullRequest {
        number: 4712,
        title: "Rework the billing export".to_string(),
        url: "https://github.com/acme/widgets/pull/4712".to_string(),
        state: BranchPrState::Merged,
        created_at: "2026-10-10T09:00:00Z".to_string(),
        base: "main".to_string(),
        head: "0123456789abcdef0123456789abcdef01234567".to_string(),
    };
    for output in [format!("{listed:?}"), format!("{listed:#?}")] {
        assert!(output.contains("4712") && output.contains("Merged"), "{output}");
        for secret in ["billing", "github.com", "0123456789", "main"] {
            assert!(!output.contains(secret), "{secret} printed: {output}");
        }
    }
}
