//! T063 (bugfix BUG-002) — the AI CLI provider abstraction (FR-024).
//!
//! Verifies the provider seam consolidates every provider-specific detail: launch command +
//! args, conversation-transcript location, recorded-conversation detection, and session-title
//! extraction. `ClaudeProvider` is the concrete default. Pure + I/O-boundary behaviour is
//! exercised headlessly (a real temp transcript file — no `claude`, no GUI).

use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::terminal::LaunchMode;
use std::path::Path;
use uuid::Uuid;

fn fixed_id() -> Uuid {
    Uuid::parse_str("00000000-0000-4000-8000-000000000abc").unwrap()
}

#[test]
fn command_is_claude() {
    assert_eq!(ClaudeProvider.command(), "claude");
}

#[test]
fn launch_args_fresh_uses_session_id_flag() {
    let id = fixed_id();
    assert_eq!(
        ClaudeProvider.launch_args(id, LaunchMode::Fresh),
        vec!["--session-id".to_string(), id.to_string()]
    );
}

#[test]
fn launch_args_resume_uses_resume_flag() {
    let id = fixed_id();
    assert_eq!(
        ClaudeProvider.launch_args(id, LaunchMode::Resume),
        vec!["--resume".to_string(), id.to_string()]
    );
}

#[test]
fn transcript_path_encodes_cwd_and_appends_jsonl() {
    let id = fixed_id();
    let config = Path::new("/cfg");
    let cwd = Path::new("/home/u/proj/.claude/worktrees/feat-x");
    let path = ClaudeProvider.transcript_path(config, cwd, id);

    // <config>/projects/<encoded-cwd>/<id>.jsonl, encoded-cwd = every non-alphanumeric → '-'.
    let encoded: String = "/home/u/proj/.claude/worktrees/feat-x"
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    assert_eq!(
        path,
        Path::new("/cfg")
            .join("projects")
            .join(&encoded)
            .join(format!("{id}.jsonl"))
    );
    // Sanity: no path separators survived inside the encoded segment.
    assert!(!encoded.contains('/'));
}

#[test]
fn parse_title_returns_latest_ai_title() {
    let transcript = concat!(
        r#"{"type":"user","text":"hi"}"#,
        "\n",
        r#"{"type":"ai-title","aiTitle":"First title"}"#,
        "\n",
        r#"{"type":"assistant","text":"..."}"#,
        "\n",
        r#"{"type":"ai-title","aiTitle":"Second title"}"#,
        "\n",
    );
    assert_eq!(
        ClaudeProvider.parse_title(transcript),
        Some("Second title".to_string())
    );
}

#[test]
fn parse_title_none_when_no_title_record() {
    let transcript = concat!(
        r#"{"type":"user","text":"hi"}"#,
        "\n",
        r#"{"type":"assistant","text":"there"}"#,
        "\n",
    );
    assert_eq!(ClaudeProvider.parse_title(transcript), None);
}

#[test]
fn parse_title_skips_malformed_lines_and_empties() {
    let transcript = concat!(
        "\n",
        "not json at all\n",
        r#"{"type":"ai-title","aiTitle":"Good"}"#,
        "\n",
        "{ broken json\n",
    );
    assert_eq!(
        ClaudeProvider.parse_title(transcript),
        Some("Good".to_string())
    );
}

#[test]
fn parse_title_ignores_empty_ai_title() {
    let transcript = r#"{"type":"ai-title","aiTitle":""}"#;
    assert_eq!(ClaudeProvider.parse_title(transcript), None);
}

#[test]
fn read_title_reads_from_the_transcript_file() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    let cwd = Path::new("/home/u/proj/.claude/worktrees/feat-x");

    let path = ClaudeProvider.transcript_path(config.path(), cwd, id);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, r#"{"type":"ai-title","aiTitle":"From disk"}"#).unwrap();

    assert_eq!(
        ClaudeProvider.read_title(config.path(), cwd, id),
        Some("From disk".to_string())
    );
    assert!(ClaudeProvider.has_recorded_conversation(config.path(), cwd, id));
}

#[test]
fn read_title_is_none_when_transcript_missing() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    let cwd = Path::new("/home/u/proj/.claude/worktrees/feat-x");

    // Nothing written — a missing/unreadable transcript never errors, it is simply absent.
    assert_eq!(ClaudeProvider.read_title(config.path(), cwd, id), None);
    assert!(!ClaudeProvider.has_recorded_conversation(config.path(), cwd, id));
}

// --- Bugfix BUG-003: durable close/remove suppression marker (T068) ---

#[test]
fn mark_archived_then_is_archived_reflects_it() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    let cwd = Path::new("/home/u/proj/.claude/worktrees/feat-x");

    assert!(
        !ClaudeProvider.is_archived(config.path(), cwd, id),
        "no marker written yet"
    );

    ClaudeProvider
        .mark_archived(config.path(), cwd, id)
        .unwrap();

    assert!(
        ClaudeProvider.is_archived(config.path(), cwd, id),
        "marker written by mark_archived must be reflected by is_archived"
    );
}

#[test]
fn archived_marker_is_never_discovered_as_a_transcript() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    let cwd = Path::new("/home/u/proj/.claude/worktrees/feat-x");

    // Only a marker exists for this id — no `.jsonl` transcript was ever written.
    ClaudeProvider
        .mark_archived(config.path(), cwd, id)
        .unwrap();

    assert!(
        ClaudeProvider
            .discover_transcript_session_ids(config.path(), cwd)
            .is_empty(),
        "a `.archived` marker must never be misread as a `.jsonl` transcript"
    );
}
