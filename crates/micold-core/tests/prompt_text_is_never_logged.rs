//! The files that read a prompt or a label never log one (feature 1120, FR-020, T046).
//!
//! `markup`, `overview_text` and `hook_activity` turn a session's prompt, todos and labels into the
//! Overview card's text. That text can hold anything the user typed, so none of these files may
//! write to a log, the terminal or a debug print. A text scan answers that exactly: a log or print
//! in them is a macro call with one of these paths.

use std::fs;
use std::path::Path;

/// The files that see prompt or label text.
const FILES: &[&str] = &[
    "src/markup.rs",
    "src/overview_text.rs",
    "src/hook_activity.rs",
];

/// A log or print in Rust source.
const LOGGING: &[&str] = &["tracing::", "log::", "println!", "eprintln!", "dbg!"];

#[test]
fn no_prompt_or_label_reading_file_logs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    for file in FILES {
        let source = fs::read_to_string(root.join(file))
            .unwrap_or_else(|e| panic!("cannot read {file}: {e}"));
        for (n, line) in source.lines().enumerate() {
            if let Some(what) = LOGGING.iter().find(|what| line.contains(*what)) {
                found.push(format!("{file}:{}: {what} in `{}`", n + 1, line.trim()));
            }
        }
    }
    assert!(
        found.is_empty(),
        "FR-020: a file that reads prompt or label text logs or prints:\n  {}",
        found.join("\n  ")
    );
}
