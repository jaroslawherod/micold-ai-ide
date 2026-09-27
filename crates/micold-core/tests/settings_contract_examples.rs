//! Every `settings.json` example in a settings contract loads as written (006 BUG-009).
//!
//! The contracts are where a user or a later feature copies a settings document from. Feature 006's
//! showed `"theme": "FollowSystem"` and feature 027's `"theme": "System"`, but `ThemePreference`
//! serializes snake_case, so neither parsed: a `settings.json` written from either example was
//! moved to `settings.json.bak` and every setting in it was replaced by the defaults. Nothing
//! failed, because nothing read the examples.
//!
//! This loads each example through the real [`JsonFileSettingsStore`], so an example stays a
//! document the app accepts. The contracts it reads are therefore test inputs rather than
//! documentation, and `.gitattributes` says so (`-micold-docs`), as it does for the other specs a
//! test reads — otherwise an edit to an example would skip the suite that checks it.

use std::fs;
use std::path::{Path, PathBuf};

use micold_core::settings::{JsonFileSettingsStore, SettingsStore};
use micold_core::store::LoadStatus;
use micold_core::theme::ThemePreference;

/// The contracts that show an on-disk `settings.json`. Named rather than globbed, for the same
/// reason the `.gitattributes` carve-outs are: each one is a file this test makes a test input.
const CONTRACTS: &[&str] = &[
    "specs/003-material-design-layout/contracts/settings-schema.md",
    "specs/006-real-terminal-emulator/contracts/settings-schema.md",
    "specs/027-sandboxed-daemon-runtime/contracts/sandbox-settings-schema.md",
];

fn repo_root() -> PathBuf {
    // tests/ -> micold-core/ -> crates/ -> repo root
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The body of every ```` ```json ```` or ```` ```jsonc ```` fence that holds a settings document,
/// recognised by its `settings_version` key.
fn settings_examples(markdown: &str) -> Vec<String> {
    let mut examples = Vec::new();
    let mut current: Option<String> = None;
    for line in markdown.lines() {
        let fence = line.trim_start();
        match current.as_mut() {
            None if fence == "```json" || fence == "```jsonc" => current = Some(String::new()),
            None => {}
            Some(_) if fence.starts_with("```") => {
                let body = current.take().unwrap();
                if body.contains("\"settings_version\"") {
                    examples.push(body);
                }
            }
            Some(body) => {
                body.push_str(strip_line_comment(line));
                body.push('\n');
            }
        }
    }
    examples
}

/// `line` without a trailing `// …` comment. A `//` inside a string literal is kept.
fn strip_line_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;
    let bytes = line.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if in_string {
            match b {
                _ if escaped => escaped = false,
                b'\\' => escaped = true,
                b'"' => in_string = false,
                _ => {}
            }
        } else if b == b'"' {
            in_string = true;
        } else if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
            return &line[..i];
        }
    }
    line
}

#[test]
fn every_settings_contract_example_loads_without_recovery() {
    let mut failures = Vec::new();
    for contract in CONTRACTS {
        let path = repo_root().join(contract);
        let markdown =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        let examples = settings_examples(&markdown);
        assert!(
            !examples.is_empty(),
            "{contract} shows no settings document — the scan is looking at the wrong file"
        );

        for example in examples {
            let dir = tempfile::tempdir().unwrap();
            let file = dir.path().join("settings.json");
            fs::write(&file, &example).unwrap();
            let outcome = JsonFileSettingsStore::at(file.clone()).load();

            // The store reports only that it recovered, so name the theme's own parse error when
            // there is one: it is the field this test exists for.
            let written: serde_json::Value = serde_json::from_str(&example)
                .unwrap_or_else(|e| panic!("{contract}: the example is not JSON: {e}\n{example}"));
            let theme = written.get("theme");
            let theme_error = theme
                .and_then(|t| serde_json::from_value::<ThemePreference>(t.clone()).err())
                .map(|e| format!(" (theme: {e})"))
                .unwrap_or_default();

            if outcome.status != LoadStatus::Loaded {
                failures.push(format!(
                    "{contract}: the example loads as {:?}{theme_error}, moved to .bak: {}, so a \
                     file written from it loses every setting:\n{example}",
                    outcome.status,
                    file.with_extension("json.bak").exists(),
                ));
                continue;
            }

            // The theme the example names is the theme that loads, spelled the same way back.
            if let Some(theme) = theme {
                let loaded = serde_json::to_value(outcome.settings.theme).unwrap();
                if &loaded != theme {
                    failures.push(format!(
                        "{contract}: the example's theme {theme} loads as {loaded}"
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
