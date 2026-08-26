//! T064 (bugfix BUG-002, completes T054) — session label stays in sync with the AI CLI
//! provider's session name (FR-011a, SC-009).
//!
//! Exercises the real read path (`AiCliProvider::read_title` over a temp conversation record)
//! wired to the pure reducer (`Message::SessionTitleUpdated` → `set_title`), headlessly — no
//! `claude`, no GUI. This is the behaviour the main-loop terminal poll drives at runtime.

use micold_client::app::{Message, State};
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionLocation};
use std::path::{Path, PathBuf};

/// The runtime sync step, mirrored from the main loop (`sync_session_titles`, one of the five
/// cwd-resolution call sites, research.md R2): read the provider's current title for a session
/// and, when it differs from the current label, emit `SessionTitleUpdated`. The cwd decision
/// itself delegates to `SessionLocation::cwd` (`src/session.rs`), the single authoritative
/// implementation, rather than re-deriving the `Worktree`/`Default` branch here.
fn sync_once(state: &mut State, config_dir: &Path, project: &Path) {
    let mut updates = Vec::new();
    for session in state.active_sessions() {
        // Per session (feature 026, T059): the title comes from the CLI the session actually runs.
        // One hoisted provider would read every Copilot session's title out of `claude`'s
        // transcript directory, find nothing, and leave every one of them labelled "New session".
        let provider = session.provider.provider();
        let cwd = session.location.cwd(project);
        if let Some(title) = provider.read_title(config_dir, &cwd, session.id.0) {
            if session.label.display() != title {
                updates.push((session.id, title));
            }
        }
    }
    for (id, title) in updates {
        state.update(Message::SessionTitleUpdated { id, title });
    }
}

fn state_with_active_session(project: &Path, worktree_dir: &str) -> (State, Session) {
    state_with_active_session_on(project, worktree_dir, AiCli::ClaudeCode)
}

fn state_with_active_session_on(
    project: &Path,
    worktree_dir: &str,
    provider: AiCli,
) -> (State, Session) {
    let mut state = State::default();
    state.workspace.projects.push(Project {
        path: project.to_path_buf(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(project.to_path_buf());
    let session = Session::start_new(
        SessionLocation::Worktree(worktree_dir.to_string()),
        provider,
    );
    state.update(Message::SessionStarted(session.clone()));
    (state, session)
}

/// Write a title where `claude` records one, spelling the layout out here rather than asking the
/// provider for a path.
///
/// The path arithmetic left the seam in feature 026 — it is `claude`'s own layout and nothing
/// else's, and having it on the trait is what made the trait un-substitutable. Restating it in the
/// fixture is the right place for it: this file exercises the *read* path, so the write side is
/// setup, and `micold-core/tests/ai_cli_provider.rs` is what holds the two spellings to agreement.
fn write_title(config_dir: &Path, project: &Path, worktree_dir: &str, id: uuid::Uuid, title: &str) {
    let cwd = project.join(".claude/worktrees").join(worktree_dir);
    let encoded: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let path = config_dir
        .join("projects")
        .join(encoded)
        .join(format!("{id}.jsonl"));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(r#"{{"type":"ai-title","aiTitle":"{title}"}}"#),
    )
    .unwrap();
}

#[test]
fn label_goes_pending_then_named_then_resyncs_on_change() {
    let config = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo");
    let worktree = "feat-x";
    let (mut state, session) = state_with_active_session(&project, worktree);

    // Before the provider records a title, the label is the neutral placeholder.
    assert_eq!(state.active_sessions()[0].label.display(), "New session");

    // Provider assigns a title → a sync reconciles the label from Pending to Named.
    write_title(
        config.path(),
        &project,
        worktree,
        session.id.0,
        "Investigate flaky test",
    );
    sync_once(&mut state, config.path(), &project);
    assert_eq!(
        state.active_sessions()[0].label.display(),
        "Investigate flaky test"
    );

    // Provider later changes the title → the label re-syncs (never stays diverged, SC-009).
    write_title(
        config.path(),
        &project,
        worktree,
        session.id.0,
        "Fix flaky test race",
    );
    sync_once(&mut state, config.path(), &project);
    assert_eq!(
        state.active_sessions()[0].label.display(),
        "Fix flaky test race"
    );
}

#[test]
fn sync_is_noop_when_provider_has_no_title() {
    let config = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo");
    let (mut state, _session) = state_with_active_session(&project, "feat-x");

    // No transcript on disk → read fails silently → label untouched (must not fail the session).
    sync_once(&mut state, config.path(), &project);
    assert_eq!(state.active_sessions()[0].label.display(), "New session");
}

// ---------------------------------------------------------------------------------------
// Feature 026 (T059) — a Copilot title reaches the row through the same seam
// ---------------------------------------------------------------------------------------

/// Write a Copilot session's `workspace.yaml`, with or without the `name:` key.
///
/// Copilot keys a conversation by session id alone — the working directory only ever enters
/// through the per-cwd *index*, which the title path does not read — so unlike `claude`'s
/// transcript there is no cwd in this address.
fn write_copilot_title(config_dir: &Path, id: uuid::Uuid, title: Option<&str>) {
    let dir = config_dir.join("session-state").join(id.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    let mut yaml = format!("id: {id}\ncwd: /repo\nclient_name: github/cli\n");
    if let Some(title) = title {
        yaml.push_str(&format!("name: {title}\n"));
    }
    std::fs::write(dir.join("workspace.yaml"), yaml).unwrap();
}

#[test]
fn a_copilot_title_reaches_the_row() {
    let config = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo");
    let (mut state, session) = state_with_active_session_on(&project, "feat-x", AiCli::Copilot);

    assert_eq!(state.active_sessions()[0].label.display(), "New session");

    write_copilot_title(config.path(), session.id.0, Some("Add the login page"));
    sync_once(&mut state, config.path(), &project);

    assert_eq!(
        state.active_sessions()[0].label.display(),
        "Add the login page",
        "the title came from Copilot's own store, through the seam — a hoisted `claude` would have \
         looked in a transcript directory that has never heard of this session"
    );
}

#[test]
fn a_copilot_session_with_no_title_yet_keeps_its_placeholder() {
    // FR-017: the `name:` key is absent until Copilot has summarised the conversation. That is the
    // ordinary early state of every session, not a failure, so the label stays `Pending`.
    let config = tempfile::tempdir().unwrap();
    let project = PathBuf::from("/repo");
    let (mut state, session) = state_with_active_session_on(&project, "feat-x", AiCli::Copilot);

    write_copilot_title(config.path(), session.id.0, None);
    sync_once(&mut state, config.path(), &project);

    assert_eq!(state.active_sessions()[0].label.display(), "New session");
}
