//! Whether a CLI would first ask the user to trust a session's folder (feature 034, FR-017,
//! research R12).
//!
//! Claude Code and Copilot show a trust question in a folder they have not trusted yet, and their
//! output settles on it, so a first prompt typed then would answer it on the user's behalf. The
//! service reads each CLI's own record, read-only, and types nothing when the CLI would ask. A file
//! that is absent, unreadable or malformed trusts nothing.

use std::path::{Path, PathBuf};

use serde_json::Value;

use super::binding::{read_json, ConfigLocations};
use crate::provider::FolderTrust;

/// Whether a CLI keeping `trust` would ask whether to trust `cwd` before its first prompt. A folder
/// the record trusts trusts every folder below it, so a worktree under a trusted project is trusted.
pub fn would_ask_trust(trust: FolderTrust, locations: &ConfigLocations, cwd: &Path) -> bool {
    let trusted = match trust {
        FolderTrust::NeverAsks => return false,
        FolderTrust::ClaudeProjects => claude_trusted(locations),
        FolderTrust::CopilotTrustedFolders => copilot_trusted(locations),
        FolderTrust::CodexProjects => codex_trusted(locations),
    };
    let canonical = cwd.canonicalize().ok();
    !trusted.iter().any(|folder| {
        cwd.starts_with(folder) || canonical.as_deref().is_some_and(|c| c.starts_with(folder))
    })
}

/// The projects `.claude.json` records with `hasTrustDialogAccepted: true`.
fn claude_trusted(locations: &ConfigLocations) -> Vec<PathBuf> {
    let Some(doc) = locations.claude_json().and_then(|path| read_json(&path)) else {
        return Vec::new();
    };
    doc.get("projects")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(_, project)| project.get("hasTrustDialogAccepted") == Some(&Value::Bool(true)))
        .map(|(path, _)| PathBuf::from(path))
        .collect()
}

/// The folders Copilot's `config.json` lists in `trustedFolders`. Copilot starts that file with
/// `//` comment lines, which are skipped.
fn copilot_trusted(locations: &ConfigLocations) -> Vec<PathBuf> {
    let Some(dir) = &locations.copilot_config_dir else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(dir.join("config.json")) else {
        return Vec::new();
    };
    let body: String = text
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    let Ok(doc) = serde_json::from_str::<Value>(&body) else {
        return Vec::new();
    };
    doc.get("trustedFolders")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect()
}

/// The projects Codex's `config.toml` records as `[projects."<path>"]` with
/// `trust_level = "trusted"`. A small line reader, not a TOML parser: it only needs these two
/// shapes, and anything it does not recognise trusts nothing.
fn codex_trusted(locations: &ConfigLocations) -> Vec<PathBuf> {
    let Some(dir) = &locations.codex_home else {
        return Vec::new();
    };
    let Ok(text) = std::fs::read_to_string(dir.join("config.toml")) else {
        return Vec::new();
    };
    let mut trusted = Vec::new();
    let mut current: Option<String> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(header) = line.strip_prefix('[') {
            current = project_header(header);
        } else if let Some(path) = &current {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.split('#').next().unwrap_or("").trim();
            if key.trim() == "trust_level" && matches!(value, "\"trusted\"" | "'trusted'") {
                trusted.push(PathBuf::from(path));
            }
        }
    }
    trusted
}

/// The path of a `projects."<path>"]` table header (the text after the opening bracket).
fn project_header(header: &str) -> Option<String> {
    // Everything after the closing bracket (a trailing comment) is ignored.
    let rest = header.strip_prefix("projects.")?;
    let rest = rest[..rest.rfind(']')?].trim_end();
    if let Some(literal) = rest.strip_prefix('\'').and_then(|r| r.strip_suffix('\'')) {
        return Some(literal.to_string());
    }
    let basic = rest.strip_prefix('"').and_then(|r| r.strip_suffix('"'))?;
    Some(basic.replace("\\\\", "\\").replace("\\\"", "\""))
}
