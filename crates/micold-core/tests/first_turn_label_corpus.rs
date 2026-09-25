//! What every `claude` and GitHub Copilot conversation on this machine would read as, for
//! quickstart §B1 (feature 032, T025 and T038 — SC-001, SC-002, SC-005).
//!
//! Not a test of the code: a read-only report over the developer's real transcripts, so a human can
//! check that each untitled row gets a label that is what they typed, and count how many rows still
//! read "New session". It asserts nothing about the machine, and it is ignored and gated twice so
//! that no CI run and no plain `cargo test -- --ignored` ever reads a real home directory:
//!
//! ```sh
//! MICOLD_LABEL_CORPUS=1 scripts/build-lock.sh cargo test -p micold-core \
//!   --test first_turn_label_corpus -- --ignored --nocapture
//! ```
//!
//! The Copilot report is **evidence, not a verdict**: it says what each session directory on this
//! machine holds, which is how a human checks the shapes the fixtures were modelled on. Whether
//! SC-008 and SC-009 hold is decided by the fixture tests in `copilot_provider.rs` and
//! `untitled_session_labels.rs`, over sessions Copilot's own index lists — the only ones the
//! application ever shows (D10). Most of the directories walked here are listed by no index and are
//! therefore no row at all.
//!
//! The label comes from the same `read_prefix` + `claude_first_turn` path
//! `ClaudeProvider::read_label` takes. The title check is "has a non-empty `ai-title` record", the
//! rule `read_title` applies; it is repeated here because `read_title` finds a transcript by its
//! worktree path, which a directory listing cannot give back.

use std::path::Path;

use micold_core::first_turn::{claude_first_turn, copilot_first_turn, read_prefix};
use micold_core::provider::{AiCliProvider, ClaudeProvider, CopilotProvider};

/// Whether `transcript` has an `ai-title` record with a non-empty title.
fn has_title(transcript: &Path) -> bool {
    std::fs::read_to_string(transcript)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .any(|record| {
            record.get("type").and_then(|t| t.as_str()) == Some("ai-title")
                && record
                    .get("aiTitle")
                    .and_then(|t| t.as_str())
                    .is_some_and(|t| !t.is_empty())
        })
}

#[test]
#[ignore = "reads the developer's real claude transcripts; run with MICOLD_LABEL_CORPUS=1"]
fn report_what_every_claude_conversation_reads_as() {
    if std::env::var_os("MICOLD_LABEL_CORPUS").is_none() {
        eprintln!("MICOLD_LABEL_CORPUS is unset: nothing read");
        return;
    }
    let Some(config) = ClaudeProvider.config_dir() else {
        eprintln!("no claude config dir: nothing read");
        return;
    };
    let mut transcripts: Vec<_> = std::fs::read_dir(config.join("projects"))
        .into_iter()
        .flatten()
        .flatten()
        .filter(|dir| dir.path().is_dir())
        .flat_map(|dir| {
            std::fs::read_dir(dir.path())
                .into_iter()
                .flatten()
                .flatten()
        })
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "jsonl"))
        .collect();
    transcripts.sort();

    let (mut titled, mut labelled, mut neither) = (0, 0, 0);
    for transcript in &transcripts {
        let name = transcript
            .strip_prefix(&config)
            .unwrap_or(transcript)
            .display();
        if has_title(transcript) {
            titled += 1;
            println!("title   {name}");
            continue;
        }
        match read_prefix(transcript).and_then(|prefix| claude_first_turn(&prefix)) {
            Some(label) => {
                labelled += 1;
                println!("label   {name}  {label}");
            }
            None => {
                neither += 1;
                println!("neither {name}");
            }
        }
    }
    println!(
        "{} transcripts: {titled} titled, {labelled} labelled, {neither} neither",
        transcripts.len()
    );
}

#[test]
#[ignore = "reads the developer's real copilot session state; run with MICOLD_LABEL_CORPUS=1"]
fn report_what_every_copilot_conversation_reads_as() {
    if std::env::var_os("MICOLD_LABEL_CORPUS").is_none() {
        eprintln!("MICOLD_LABEL_CORPUS is unset: nothing read");
        return;
    }
    let Some(config) = CopilotProvider.config_dir() else {
        eprintln!("no copilot config dir: nothing read");
        return;
    };
    let mut sessions: Vec<_> = std::fs::read_dir(config.join("session-state"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    sessions.sort();

    let (mut titled, mut labelled, mut neither) = (0, 0, 0);
    for dir in &sessions {
        let name = dir.file_name().unwrap_or(dir.as_os_str()).to_string_lossy();
        // The same two keys `read_title` reads, in the same order (`name:`, then the older
        // `summary:`), taken from the file rather than through the provider: the provider finds a
        // session by id, and a directory listing is all this report has.
        let workspace = std::fs::read_to_string(dir.join("workspace.yaml")).unwrap_or_default();
        let title = ["name", "summary"]
            .iter()
            .find_map(|key| yaml_title(&workspace, key));
        if let Some(title) = title {
            titled += 1;
            println!("title   {name}  {title}");
            continue;
        }
        match read_prefix(&dir.join("events.jsonl")).and_then(|prefix| copilot_first_turn(&prefix))
        {
            Some(label) => {
                labelled += 1;
                println!("label   {name}  {label}");
            }
            None => {
                neither += 1;
                println!("neither {name}");
            }
        }
    }
    println!(
        "{} copilot sessions: {titled} titled, {labelled} labelled, {neither} neither",
        sessions.len()
    );
}

/// The reporting copy of the provider's scalar rule: a top-level `key:` with a plain or quoted
/// one-line value, and nothing else (a block scalar is no title).
fn yaml_title(contents: &str, key: &str) -> Option<String> {
    let value = contents
        .lines()
        .filter(|line| !line.starts_with(char::is_whitespace))
        .find_map(|line| line.strip_prefix(key)?.strip_prefix(':'))?
        .trim();
    let unquoted = match (value.chars().next(), value.chars().last()) {
        (Some('\''), Some('\'')) | (Some('"'), Some('"')) if value.len() >= 2 => {
            &value[1..value.len() - 1]
        }
        _ => value,
    };
    (!unquoted.is_empty() && !unquoted.starts_with(['|', '>'])).then(|| unquoted.to_string())
}
