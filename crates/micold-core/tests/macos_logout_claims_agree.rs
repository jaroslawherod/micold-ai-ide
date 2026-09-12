//! The three places that answer "what happens when I log out?" on macOS say the same thing (FR-032).
//!
//! The question has one answer and three audiences: someone installing the application
//! (`docs/user-guide/install-macos.md`), someone reading about the service itself
//! (`docs/daemon.md`), and whoever next edits the code that implements it
//! (`crates/micold-core/src/logout_survival.rs`). Three statements of one fact drift — not all at
//! once, but one at a time, and the one that is wrong is the one nobody was reading when it changed.
//!
//! # Why a shared sentence rather than a shared meaning
//!
//! A gate that checks each file "says something equivalent" has to enumerate the phrasings that
//! count, and that list is a second thing to maintain — one that fails open, since an unlisted
//! phrasing reads as agreement. So the claim is one sentence, written once here, required verbatim
//! in all three. Drift is then not caught but impossible: there is nothing to drift from.
//!
//! Comparison is over flattened text, so a file may wrap the sentence across lines however its
//! line length demands, and a Rust file may carry it as `//!`. Nothing else is normalised — the
//! sentence is deliberately free of emphasis and links so that it can be pasted unchanged into
//! prose that has both around it.

use std::fs;
use std::path::{Path, PathBuf};

/// The answer. One sentence, in the words the user guide needs, because that is the audience with
/// no other source — the other two files can carry more context around it.
const CLAIM: &str = "On macOS, sessions survive closing the window but not logging out when the \
                     session service runs directly on your computer; running it in a container is \
                     the supported way to survive logout there.";

/// The three files that answer the question, and who reads each.
const SOURCES: &[(&str, &str)] = &[
    (
        "docs/user-guide/install-macos.md",
        "someone installing the application",
    ),
    ("docs/daemon.md", "someone reading about the service"),
    (
        "crates/micold-core/src/logout_survival.rs",
        "whoever next edits the code that implements it",
    ),
];

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// One long line: doc-comment markers dropped, every run of whitespace collapsed to one space.
///
/// This is what makes the check indifferent to line length and to Markdown-versus-Rust, and it is
/// the whole of the normalisation. In particular `**bold**` survives as `**bold**`, which is why
/// [`CLAIM`] carries no emphasis.
fn flattened(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let line = line.trim_start();
            line.strip_prefix("//!")
                .or_else(|| line.strip_prefix("///"))
                .unwrap_or(line)
                .trim()
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// [`CLAIM`] under the same normalisation, so the constant may wrap in this file too.
fn claim() -> String {
    flattened(CLAIM)
}

#[test]
fn every_place_that_answers_the_logout_question_gives_the_same_answer() {
    let claim = claim();
    let mut missing: Vec<String> = Vec::new();

    for (rel, audience) in SOURCES {
        let path = repo_root().join(rel);
        let source =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        if !flattened(&source).contains(&claim) {
            missing.push(format!("  {rel} (read by {audience})"));
        }
    }

    assert!(
        missing.is_empty(),
        "the macOS logout answer is missing from:\n{}\n\nEvery one of these must contain this \
         sentence verbatim (wrapping is free):\n\n  {claim}\n\nFR-032: one answer, three \
         audiences. If the answer has changed, change it here first — this constant is the \
         definition — and then in all three.",
        missing.join("\n")
    );
}

#[test]
fn the_answer_is_stated_once_per_file() {
    // A file that says it twice will one day have one of them edited. Not a formatting rule: the
    // second copy is the one that goes stale, and this gate would still pass on the first.
    for (rel, _) in SOURCES {
        let path = repo_root().join(rel);
        let source = fs::read_to_string(&path).expect("read source");
        let count = flattened(&source).matches(&claim()).count();
        assert_eq!(
            count, 1,
            "{rel} states the macOS logout answer {count} times"
        );
    }
}

#[test]
fn the_scan_reads_what_it_claims_to() {
    for (rel, _) in SOURCES {
        let path = repo_root().join(rel);
        assert!(
            path.is_file(),
            "{rel} does not exist; the scan covers nothing"
        );
    }

    // Flattening joins wrapped lines and drops `//!`, and does nothing else.
    assert_eq!(flattened("//! one\n//!   two\n   three  "), "one two three");
    assert_eq!(flattened("*a*\n**b**"), "*a* **b**");
    // And the claim is a sentence, not an accidentally-empty constant.
    assert!(
        claim().len() > 80,
        "the claim is too short to be the answer"
    );
}
