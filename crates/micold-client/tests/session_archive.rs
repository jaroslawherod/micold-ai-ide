//! T067 (bugfix BUG-003) — closing a session archives it (stops the process, keeps the record)
//! rather than deleting it outright, and archived sessions are excluded from whatever the
//! sidebar renders from (`State::active_sessions()`).

use micold_client::app::State;
use micold_client::features::sidebar::SidebarEntry;
use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLifecycle, SessionLocation, TerminalMode,
};
use micold_core::workspace::Workspace;
use std::path::PathBuf;

fn state_with(project: &str, sessions: Vec<Session>) -> State {
    let mut ws = Workspace::empty();
    ws.projects.push(Project {
        path: PathBuf::from(project),
        display_name: "proj".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    ws.active = Some(PathBuf::from(project));
    ws.sessions.insert(PathBuf::from(project), sessions);
    State {
        workspace: ws,
        ..State::default()
    }
}

#[test]
fn archive_stops_the_process_and_keeps_the_record() {
    let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    session.mark_running();

    session.archive();

    assert_eq!(session.lifecycle, SessionLifecycle::Idle);
    assert!(
        session.archived,
        "archive() must flag the session as archived"
    );
}

#[test]
fn archived_sessions_are_hidden_from_the_sidebar() {
    let mut archived = Session::restored(
        SessionId::new(),
        SessionLocation::Default,
        SessionLabel::Pending,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    );
    archived.archive();
    let visible = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let visible_id = visible.id;

    let state = state_with("/repo", vec![archived, visible]);

    let default_node = state
        .sidebar_entries()
        .into_iter()
        .find_map(|entry| match entry {
            SidebarEntry::Default(node) => Some(node),
            SidebarEntry::Worktree(_) => None,
        })
        .expect("a Default entry is always present when a project is active");

    let ids: Vec<_> = default_node.sessions.iter().map(|s| s.id).collect();
    assert_eq!(
        ids,
        vec![visible_id],
        "an archived session must not appear in the sidebar"
    );
}
