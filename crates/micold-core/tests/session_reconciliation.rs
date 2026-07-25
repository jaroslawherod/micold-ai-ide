//! T065/T066 (bugfix 002/BUG-001) — session reconciliation from AI CLI provider transcripts
//! (FR-020b, SC-010).
//!
//! `reconcile_sessions_from_transcripts` (`src/main.rs`) is the real function the app calls at
//! every project-open site (`boot()`, `Message::FolderChosen`, `Message::KnownProjectReopened`).
//! `src/main.rs` is the GUI binary and cannot be linked from an integration test, so this mirrors
//! it here from the same public provider seam — the established pattern for main-loop I/O-boundary
//! logic in this crate (see `tests/session_title_sync.rs`'s `sync_once`).

use micold_core::provider::{AiCliProvider, ClaudeProvider};
use micold_core::session::{Session, SessionId, SessionLabel, SessionLocation, TerminalMode};
use micold_core::workspace::Workspace;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

/// Mirrors `reconcile_sessions_from_transcripts` (`src/main.rs`). `locations` stands in for
/// `[SessionLocation::Default] + every currently-Valid worktree` — main.rs derives that list from
/// `State::worktrees`; here it is passed directly since this test targets the reconciliation
/// logic itself, not worktree discovery (already covered by `tests/worktree_discovery.rs`).
fn reconcile(
    workspace: &mut Workspace,
    repo: &Path,
    config_dir: &Path,
    locations: &[SessionLocation],
) {
    let provider = ClaudeProvider;
    let mut seen: HashSet<Uuid> = workspace
        .sessions
        .get(repo)
        .map(|list| list.iter().map(|s| s.id.0).collect())
        .unwrap_or_default();

    let mut reconstructed = Vec::new();
    for location in locations {
        let cwd = location.cwd(repo);
        for session_id in provider.discover_transcript_session_ids(config_dir, &cwd) {
            if !seen.insert(session_id) {
                continue;
            }
            // Bugfix BUG-003 (FR-020c): a closed/removed session's durable marker suppresses
            // reconciliation regardless of what the app's own (possibly empty) store contains.
            if provider.is_archived(config_dir, &cwd, session_id) {
                continue;
            }
            let label = match provider.read_title(config_dir, &cwd, session_id) {
                Some(title) => SessionLabel::Named(title),
                None => SessionLabel::Pending,
            };
            reconstructed.push(Session::restored(
                SessionId::from_uuid(session_id),
                location.clone(),
                label,
                TerminalMode::AiCli,
            ));
        }
    }
    if !reconstructed.is_empty() {
        workspace
            .sessions
            .entry(repo.to_path_buf())
            .or_default()
            .extend(reconstructed);
    }
}

/// Write a fake `claude` transcript file directly, bypassing any real `claude` process — mirrors
/// how `tests/session_title_sync.rs` and `tests/ai_cli_provider.rs` fabricate transcripts.
fn write_transcript(config_dir: &Path, cwd: &Path, session_id: Uuid, title: Option<&str>) {
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let dir = config_dir.join("projects").join(encoded);
    std::fs::create_dir_all(&dir).unwrap();
    let line = match title {
        Some(title) => format!(r#"{{"type":"ai-title","aiTitle":"{title}"}}"#),
        // No title record — still "has a conversation" (session_has_conversation only checks
        // the transcript's existence, not its contents).
        None => r#"{"type":"user","message":"hi"}"#.to_string(),
    };
    std::fs::write(dir.join(format!("{session_id}.jsonl")), line).unwrap();
}

#[test]
fn orphan_transcripts_are_reconstructed_with_correct_location_and_title() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let root_id = Uuid::new_v4();
    let worktree_id = Uuid::new_v4();
    let worktree_location = SessionLocation::Worktree("feat-x".to_string());

    write_transcript(config.path(), &repo, root_id, Some("Root session"));
    write_transcript(
        config.path(),
        &worktree_location.cwd(&repo),
        worktree_id,
        None,
    );

    let mut ws = Workspace::empty();
    let locations = vec![SessionLocation::Default, worktree_location.clone()];
    reconcile(&mut ws, &repo, config.path(), &locations);

    let sessions = ws.sessions.get(&repo).expect("sessions reconstructed");
    assert_eq!(sessions.len(), 2);

    let root = sessions
        .iter()
        .find(|s| s.id.0 == root_id)
        .expect("root session found");
    assert_eq!(root.location, SessionLocation::Default);
    assert_eq!(root.label, SessionLabel::Named("Root session".to_string()));

    let worktree = sessions
        .iter()
        .find(|s| s.id.0 == worktree_id)
        .expect("worktree session found");
    assert_eq!(worktree.location, worktree_location);
    assert_eq!(worktree.label, SessionLabel::Pending, "no title record yet");
}

#[test]
fn a_transcript_matching_an_existing_record_is_not_duplicated() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let known_id = Uuid::new_v4();
    write_transcript(config.path(), &repo, known_id, Some("Known"));

    let mut ws = Workspace::empty();
    ws.sessions.insert(
        repo.clone(),
        vec![Session::restored(
            SessionId::from_uuid(known_id),
            SessionLocation::Default,
            SessionLabel::Named("Known".to_string()),
            TerminalMode::AiCli,
        )],
    );

    reconcile(&mut ws, &repo, config.path(), &[SessionLocation::Default]);

    assert_eq!(
        ws.sessions.get(&repo).unwrap().len(),
        1,
        "an already-persisted session must not be duplicated"
    );
}

#[test]
fn no_transcripts_leaves_workspace_sessions_untouched() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let mut ws = Workspace::empty();

    reconcile(&mut ws, &repo, config.path(), &[SessionLocation::Default]);

    assert!(!ws.sessions.contains_key(&repo));
}

// --- Bugfix BUG-003: the marker survives total loss of the app's own store (T069) ---

#[test]
fn a_marked_archived_transcript_is_never_reconstructed_even_with_an_empty_store() {
    let config = tempdir().unwrap();
    let repo = PathBuf::from("/repo");
    let closed_id = Uuid::new_v4();

    write_transcript(config.path(), &repo, closed_id, Some("Closed session"));
    ClaudeProvider
        .mark_archived(config.path(), &repo, closed_id)
        .unwrap();

    // Simulates total loss of the app's own store: an entirely empty `Workspace`, with no
    // knowledge whatsoever that this session was ever closed. The marker alone must suppress it.
    let mut ws = Workspace::empty();
    reconcile(&mut ws, &repo, config.path(), &[SessionLocation::Default]);

    assert!(
        !ws.sessions.contains_key(&repo),
        "a session with a durable archived marker must never be reconstructed, \
         regardless of what the app's own store remembers"
    );
}
