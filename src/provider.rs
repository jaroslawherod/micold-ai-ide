//! AI CLI provider abstraction (FR-024, bugfix BUG-002).
//!
//! The AI coding CLI that backs a session is reached through a single seam so another provider
//! can be added later without touching the session model, persistence, sidebar, or terminal
//! wiring. `claude` (Claude Code) is the default and only provider this version.
//!
//! Pure + unit-testable: this module never spawns a process (the real PTY launch lives behind
//! [`crate::terminal::TerminalBackend`]); its I/O is limited to best-effort reads of the
//! provider's own on-disk conversation transcript. Contract: the `claude` provider profile in
//! `specs/005-worktree-session-terminal/contracts/claude-cli.md`.

use crate::terminal::LaunchMode;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Abstraction over an AI coding CLI (FR-024). Consolidates every provider-specific detail: the
/// launch command + argument shape, how the app-owned session id is passed / resumed, where the
/// conversation transcript lives, how a recorded conversation is detected, and how the session
/// name/title is extracted for the sidebar label (FR-011a).
///
/// Adding a provider means adding an impl of this trait — nothing above the seam changes.
pub trait AiCliProvider {
    /// The executable to spawn (looked up on `PATH`).
    fn command(&self) -> &'static str;

    /// The argument vector for a launch, given the app-owned session id and fresh/resume mode.
    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String>;

    /// The provider's base config directory (environment-derived), or `None` when it cannot be
    /// determined (callers treat that as "uncertain" rather than "absent").
    fn config_dir(&self) -> Option<PathBuf>;

    /// The conversation-transcript path for a session running in `cwd`. Pure path derivation
    /// (no I/O), so the encoding is unit-testable.
    fn transcript_path(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> PathBuf;

    /// Extract the latest provider-supplied session title from raw transcript contents
    /// (best-effort, pure). `None` when no non-empty title has been recorded or the contents are
    /// unparseable.
    fn parse_title(&self, transcript: &str) -> Option<String>;

    /// Whether the provider has recorded a conversation for this session (its transcript exists).
    fn has_recorded_conversation(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        self.transcript_path(config_dir, cwd, session_id).exists()
    }

    /// Read the current session title from disk (best-effort I/O). NEVER errors: any missing
    /// file / read / parse failure yields `None`, so a title read never fails the session
    /// (FR-011a).
    fn read_title(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String> {
        let contents =
            std::fs::read_to_string(self.transcript_path(config_dir, cwd, session_id)).ok()?;
        self.parse_title(&contents)
    }
}

/// The default AI CLI provider: Anthropic's `claude` (Claude Code) CLI. Verified against
/// v2.1.210 (research R6). See the `claude` profile in `contracts/claude-cli.md`.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeProvider;

impl ClaudeProvider {
    /// Environment variable overriding the default `~/.claude` config directory.
    const CONFIG_DIR_ENV: &'static str = "CLAUDE_CONFIG_DIR";
}

impl AiCliProvider for ClaudeProvider {
    fn command(&self) -> &'static str {
        "claude"
    }

    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String> {
        let id = session_id.to_string();
        match mode {
            // The app owns the id up front (research R6).
            LaunchMode::Fresh => vec!["--session-id".to_string(), id],
            // Resume a specific session — never `--continue` (claude-cli.md).
            LaunchMode::Resume => vec!["--resume".to_string(), id],
        }
    }

    fn config_dir(&self) -> Option<PathBuf> {
        if let Ok(dir) = std::env::var(Self::CONFIG_DIR_ENV) {
            if !dir.is_empty() {
                return Some(PathBuf::from(dir));
            }
        }
        directories::UserDirs::new().map(|d| d.home_dir().join(".claude"))
    }

    fn transcript_path(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> PathBuf {
        // `<config>/projects/<encoded-cwd>/<session-id>.jsonl`, where `<encoded-cwd>` is the
        // worktree path with every non-alphanumeric char replaced by `-` (research R6).
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        config_dir
            .join("projects")
            .join(encoded)
            .join(format!("{session_id}.jsonl"))
    }

    fn parse_title(&self, transcript: &str) -> Option<String> {
        // The transcript is JSONL; the LATEST `{"type":"ai-title","aiTitle":"…"}` record wins
        // (the title grows/changes with the conversation). Best-effort: blank/unparseable lines
        // are skipped, and an empty `aiTitle` is ignored.
        let mut latest = None;
        for line in transcript.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if value.get("type").and_then(|t| t.as_str()) != Some("ai-title") {
                continue;
            }
            if let Some(title) = value.get("aiTitle").and_then(|t| t.as_str()) {
                if !title.is_empty() {
                    latest = Some(title.to_string());
                }
            }
        }
        latest
    }
}
