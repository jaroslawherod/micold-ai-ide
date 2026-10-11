//! The user's own Claude Code status line, and the relay file that keeps it (feature 490,
//! research R4; FR-013, FR-020).
//!
//! The settings file the app passes with `--settings` replaces the user's `statusLine` rather than
//! merging with it, so the relay runs the user's command itself. Only the `statusLine` key of each
//! settings file is deserialised: the rest of the document (which may hold secrets in `env`) is
//! skipped, never kept, logged or sent.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::mcp::binding::ConfigLocations;

/// The user's effective status line: the first command status line Claude Code would use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserStatusLine {
    /// The shell command Claude Code would run.
    pub command: String,
    /// Extra horizontal spacing, carried into the session's own `statusLine` block.
    pub padding: Option<u32>,
    /// Re-run interval in seconds, carried into the session's own `statusLine` block.
    pub refresh_interval: Option<u32>,
}

/// What the relay reads, written owner-only beside the session's settings file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayConfig {
    /// `http://127.0.0.1:<port>/status/<session-uuid>`.
    pub url: String,
    /// The session's hook token, sent as a bearer token.
    pub token: String,
    /// The user's own status-line command, run with the same input.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_command: Option<String>,
}

/// One settings file, of which only `statusLine` is read.
#[derive(Deserialize)]
struct SettingsFile {
    #[serde(rename = "statusLine")]
    status_line: Option<serde_json::Value>,
}

/// A `statusLine` object as Claude Code documents it.
#[derive(Deserialize)]
struct StatusLineBlock {
    #[serde(rename = "type")]
    kind: String,
    command: String,
    padding: Option<u32>,
    #[serde(rename = "refreshInterval")]
    refresh_interval: Option<u32>,
}

impl UserStatusLine {
    /// The first of `cwd/.claude/settings.local.json`, `cwd/.claude/settings.json` and
    /// `<claude settings dir>/settings.json` whose `statusLine` is a `type: "command"` object with
    /// a non-empty `command`. A file that is absent, unreadable or malformed names nothing.
    pub fn resolve(locations: &ConfigLocations, cwd: &Path) -> Option<Self> {
        let project = cwd.join(".claude");
        let candidates: [Option<PathBuf>; 3] = [
            Some(project.join("settings.local.json")),
            Some(project.join("settings.json")),
            locations
                .claude_settings_dir()
                .map(|dir| dir.join("settings.json")),
        ];
        candidates.into_iter().flatten().find_map(|path| read(&path))
    }
}

fn read(path: &Path) -> Option<UserStatusLine> {
    let text = std::fs::read(path).ok()?;
    let file: SettingsFile = serde_json::from_slice(&text).ok()?;
    let block: StatusLineBlock = serde_json::from_value(file.status_line?).ok()?;
    (block.kind == "command" && !block.command.trim().is_empty()).then(|| UserStatusLine {
        command: block.command,
        padding: block.padding,
        refresh_interval: block.refresh_interval,
    })
}
