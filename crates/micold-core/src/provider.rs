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
use std::io;
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

    /// The directory holding every conversation transcript for sessions run in `cwd` (bugfix
    /// 002/BUG-001, FR-020b) — the parent directory of [`Self::transcript_path`]'s file, exposed
    /// separately so a caller can *discover* session ids it has no persisted record for by
    /// listing this directory's transcript files, rather than only checking one known id at a
    /// time. Pure path derivation (no I/O).
    fn transcript_dir(&self, config_dir: &Path, cwd: &Path) -> PathBuf;

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

    /// Path of the durable close/remove suppression marker for a session running in `cwd`
    /// (bugfix BUG-003, FR-020c) — a sentinel file beside [`Self::transcript_path`]'s file, in
    /// the provider's own storage rather than the app's. Pure path derivation (no I/O).
    fn archived_marker_path(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> PathBuf {
        self.transcript_dir(config_dir, cwd)
            .join(format!("{session_id}.archived"))
    }

    /// Record that the user closed or removed this session (bugfix BUG-003, FR-015a/FR-015c):
    /// write an empty marker at [`Self::archived_marker_path`]. Best-effort I/O — never fails the
    /// caller (mirrors the non-fatal-save posture elsewhere in this app); a failure here just
    /// means this one session may resurface via reconciliation on a future project open.
    fn mark_archived(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> io::Result<()> {
        let path = self.archived_marker_path(config_dir, cwd, session_id);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, "")
    }

    /// Whether the user has closed or removed this session (bugfix BUG-003, FR-020c) — the
    /// durable check reconciliation (FR-020b) consults, independent of the app's own store.
    fn is_archived(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        self.archived_marker_path(config_dir, cwd, session_id)
            .exists()
    }

    /// Every session id this provider has recorded a conversation for under `cwd` (bugfix
    /// 002/BUG-001, FR-020b) — discovered by listing [`Self::transcript_dir`]'s transcript
    /// files and parsing each one's filename as a session id. Best-effort (mirrors
    /// [`Self::read_title`]): a missing/unreadable directory or an unparseable filename simply
    /// contributes nothing, never an error, so discovery never fails a project open.
    fn discover_transcript_session_ids(&self, config_dir: &Path, cwd: &Path) -> Vec<Uuid> {
        let Ok(entries) = std::fs::read_dir(self.transcript_dir(config_dir, cwd)) else {
            return Vec::new();
        };
        entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                    return None;
                }
                path.file_stem()?.to_str()?.parse::<Uuid>().ok()
            })
            .collect()
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
        self.transcript_dir(config_dir, cwd)
            .join(format!("{session_id}.jsonl"))
    }

    fn transcript_dir(&self, config_dir: &Path, cwd: &Path) -> PathBuf {
        // `<config>/projects/<encoded-cwd>/`, where `<encoded-cwd>` is the worktree path with
        // every non-alphanumeric char replaced by `-` (research R6).
        let encoded: String = cwd
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        config_dir.join("projects").join(encoded)
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

// ---------------------------------------------------------------------------------------
// In-memory fake for unit tests. Public (not `#[cfg(test)]`) so integration tests in
// `tests/` can share it, matching `FakeGit` (FR-019, feature 021 T048). Pure — no process,
// no transcript on disk.
// ---------------------------------------------------------------------------------------

use std::cell::RefCell;
use std::collections::BTreeMap;

/// An in-memory [`AiCliProvider`] for tests: canned paths and titles, and a record of the
/// launches it was asked for.
///
/// The point of faking this one is not the path arithmetic — [`ClaudeProvider`]'s own tests cover
/// that — but everything *above* the seam: that session wiring asks the provider rather than
/// hardcoding `claude`, and asks it with the right id and mode. So the fake answers with a
/// distinctive command name and logs every `launch_args` call.
///
/// `RefCell`, unlike the store fakes: no consumer boxes a provider across threads.
#[derive(Debug, Default)]
pub struct FakeAiCliProvider {
    inner: RefCell<FakeProviderState>,
}

#[derive(Debug, Default)]
struct FakeProviderState {
    /// The configured base directory, or `None` for "cannot be determined".
    config_dir: Option<PathBuf>,
    /// Titles keyed by transcript contents, consulted by `parse_title`.
    titles: BTreeMap<String, String>,
    /// Transcript contents by path — what the fake has "on disk".
    transcripts: BTreeMap<PathBuf, String>,
    /// Every `(session_id, mode)` passed to `launch_args`, in call order.
    launches: Vec<(Uuid, LaunchMode)>,
}

impl FakeAiCliProvider {
    /// A provider with no config directory and no recorded conversations.
    pub fn new() -> Self {
        Self::default()
    }

    /// The directory `config_dir` reports.
    pub fn with_config_dir(self, dir: impl Into<PathBuf>) -> Self {
        self.inner.borrow_mut().config_dir = Some(dir.into());
        self
    }

    /// Make `parse_title` answer `title` for exactly these transcript contents.
    pub fn with_title(self, transcript: &str, title: &str) -> Self {
        self.inner
            .borrow_mut()
            .titles
            .insert(transcript.to_string(), title.to_string());
        self
    }

    /// Place a transcript at `path` — what
    /// [`AiCliProvider::has_recorded_conversation`] and `read_title` would find on disk.
    pub fn with_transcript(self, path: impl Into<PathBuf>, contents: &str) -> Self {
        self.inner
            .borrow_mut()
            .transcripts
            .insert(path.into(), contents.to_string());
        self
    }

    /// Every `(session_id, mode)` passed to `launch_args`, in call order.
    pub fn launches(&self) -> Vec<(Uuid, LaunchMode)> {
        self.inner.borrow().launches.clone()
    }
}

impl AiCliProvider for FakeAiCliProvider {
    fn command(&self) -> &'static str {
        // Deliberately not `claude`: a caller that hardcoded the real name would still pass a
        // test that asserted the real name, and this is the difference.
        "fake-ai-cli"
    }

    fn launch_args(&self, session_id: Uuid, mode: LaunchMode) -> Vec<String> {
        self.inner.borrow_mut().launches.push((session_id, mode));
        let flag = match mode {
            LaunchMode::Fresh => "--fresh",
            LaunchMode::Resume => "--resume",
        };
        vec![flag.to_string(), session_id.to_string()]
    }

    fn config_dir(&self) -> Option<PathBuf> {
        self.inner.borrow().config_dir.clone()
    }

    fn transcript_path(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> PathBuf {
        self.transcript_dir(config_dir, cwd)
            .join(format!("{session_id}.jsonl"))
    }

    fn transcript_dir(&self, config_dir: &Path, cwd: &Path) -> PathBuf {
        // One flat, reversible encoding, so a test can predict the path without repeating the
        // real provider's escaping rules.
        config_dir
            .join("transcripts")
            .join(cwd.to_string_lossy().replace(['/', '\\'], "_"))
    }

    fn parse_title(&self, transcript: &str) -> Option<String> {
        self.inner.borrow().titles.get(transcript).cloned()
    }

    fn has_recorded_conversation(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> bool {
        // Overridden, like `read_title` below, because the default consults the real filesystem —
        // the one thing a fake exists to avoid. A fake that inherited them would still answer
        // plausibly (`false`, `None`) while quietly making a syscall, which is how SC-005's "zero
        // real filesystem access" gets satisfied on paper and not in fact.
        let path = self.transcript_path(config_dir, cwd, session_id);
        self.inner.borrow().transcripts.contains_key(&path)
    }

    fn read_title(&self, config_dir: &Path, cwd: &Path, session_id: Uuid) -> Option<String> {
        let path = self.transcript_path(config_dir, cwd, session_id);
        let contents = self.inner.borrow().transcripts.get(&path).cloned()?;
        self.parse_title(&contents)
    }
}
