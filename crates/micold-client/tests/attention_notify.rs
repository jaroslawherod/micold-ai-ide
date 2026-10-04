//! Which attention events this window claims, and the desktop notification it shows for the one it
//! is granted (feature 039, US1; contract `desktop-notification.md` N1, N2, N4; research R3, R4).
//!
//! Driven through the root, as the shell drives it: `attention_on_welcome` and
//! `attention_on_catalog_changed` for the two kinds of catalog snapshot, `attention_granted` for
//! `DaemonMsg::AttentionGranted`. The notifier is a recording one, so no system is asked anything.

use micold_client::app::State;
use micold_client::features::attention::{DesktopNotification, DesktopNotifier, NotifyError};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ClientMsg, ProjectSnapshot, SessionSummary, WireLifecycle,
};
use micold_core::session::{AiCli, Session, SessionId, SessionLabel, SessionLocation};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// A notifier that records every notification it is asked to show, and answers with `answer`.
struct Recording {
    shown: Mutex<Vec<DesktopNotification>>,
    answer: Result<(), NotifyError>,
}

impl Recording {
    fn accepting() -> Self {
        Self {
            shown: Mutex::new(Vec::new()),
            answer: Ok(()),
        }
    }

    fn refusing() -> Self {
        Self {
            shown: Mutex::new(Vec::new()),
            answer: Err(NotifyError::NoService("nobody owns the name".to_string())),
        }
    }

    fn shown(&self) -> Vec<DesktopNotification> {
        self.shown.lock().unwrap().clone()
    }
}

impl DesktopNotifier for Recording {
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError> {
        self.shown.lock().unwrap().push(notification);
        self.answer.clone()
    }
}

const REPO: &str = "/repo";
const OTHER: &str = "/other";

fn add_project(state: &mut State, path: &str, name: &str) {
    state.workspace.projects.push(Project {
        path: PathBuf::from(path),
        display_name: name.to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
}

/// Record a session of `project` at `location`, labelled `label`, and return its id.
fn add_session(
    state: &mut State,
    project: &str,
    location: SessionLocation,
    label: &str,
) -> SessionId {
    let mut session = Session::start_new(location, AiCli::ClaudeCode);
    session.label = SessionLabel::Named(label.to_string());
    let id = session.id;
    state
        .workspace
        .sessions
        .entry(PathBuf::from(project))
        .or_default()
        .push(session);
    id
}

/// `REPO` active, named "repo", and nothing else.
fn repo_state() -> State {
    let mut state = State::default();
    add_project(&mut state, REPO, "repo");
    state.workspace.active = Some(PathBuf::from(REPO));
    state
}

fn summary(id: SessionId, seq: u64, activity: ActivitySignal) -> SessionSummary {
    SessionSummary {
        id,
        worktree_dir: None,
        title: SessionLabel::Pending,
        lifecycle: WireLifecycle::Running,
        activity,
        provider: AiCli::ClaudeCode,
        input_serial: 0,
        live_shells: Vec::new(),
        attention_seq: seq,
        unread: false,
    }
}

/// A catalog of `REPO` holding `sessions`.
fn catalog(sessions: Vec<SessionSummary>) -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from(REPO)),
        projects: vec![ProjectSnapshot {
            path: PathBuf::from(REPO),
            display_name: "repo".to_string(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions,
        }],
    }
}

fn working(id: SessionId, seq: u64) -> CatalogSnapshot {
    catalog(vec![summary(id, seq, ActivitySignal::Working)])
}

fn awaiting(id: SessionId, seq: u64) -> CatalogSnapshot {
    catalog(vec![summary(id, seq, ActivitySignal::AwaitingInput)])
}

#[test]
fn a_snapshot_with_a_higher_sequence_yields_one_claim() {
    // U117 (FR-001, US1 scenario 1).
    let mut state = repo_state();
    let b = add_session(&mut state, REPO, SessionLocation::Default, "B");
    let _ = state.attention_on_welcome(&working(b, 3), true);

    assert_eq!(
        state.attention_on_catalog_changed(&awaiting(b, 4), true),
        vec![ClientMsg::AttentionClaim { session: b, seq: 4 }]
    );
}

#[test]
fn the_first_snapshot_yields_no_claim_and_nothing_is_shown() {
    // U118, A13 (FR-005, US1 scenario 13): a change that happened with no window open.
    let mut state = repo_state();
    let b = add_session(&mut state, REPO, SessionLocation::Default, "B");

    // Nothing is shown by construction: `attention_on_welcome` takes no notifier and returns only
    // the claims to send, so the assertion is that no claim comes back.
    assert_eq!(state.attention_on_welcome(&awaiting(b, 9), true), vec![]);
}

#[test]
fn without_a_grant_nothing_is_shown() {
    // U120 (FR-006a, N1): a claim is a request; only the service's grant raises a notification.
    let mut state = repo_state();
    let b = add_session(&mut state, REPO, SessionLocation::Default, "B");
    let _ = state.attention_on_welcome(&working(b, 1), true);

    // `attention_on_catalog_changed` takes no notifier: what it returns is only a claim message,
    // and a show happens only in `attention_granted` (covered by the tests below).
    assert_eq!(
        state.attention_on_catalog_changed(&awaiting(b, 2), true),
        vec![ClientMsg::AttentionClaim { session: b, seq: 2 }],
        "the change is claimed, and only claimed"
    );
}

#[test]
fn a_grant_shows_one_notification_named_as_the_sidebar_names_the_session() {
    // U119, A1 (FR-004, US1 scenario 1, N1, N2): the worktree carries a rename, which is the name
    // its sidebar row shows.
    let mut state = repo_state();
    let b = add_session(
        &mut state,
        REPO,
        SessionLocation::Worktree("feat-x".to_string()),
        "Fix the parser",
    );
    state
        .workspace
        .set_worktree_name("feat-x", "Parser work")
        .unwrap();
    let notifier = Recording::accepting();

    assert_eq!(state.attention_granted(b, &notifier), None);

    assert_eq!(
        notifier.shown(),
        vec![DesktopNotification {
            title: "Fix the parser is waiting for input".to_string(),
            body: "repo \u{2014} Parser work".to_string(),
            project: PathBuf::from(REPO),
            session: b,
        }]
    );
}

#[test]
fn a_worktree_with_no_rename_is_named_by_its_derived_name() {
    // U119 (FR-004): the name `worktree_display_name` derives from the directory.
    let mut state = repo_state();
    let b = add_session(
        &mut state,
        REPO,
        SessionLocation::Worktree("feat-x".to_string()),
        "B",
    );
    let notifier = Recording::accepting();

    let _ = state.attention_granted(b, &notifier);

    assert_eq!(notifier.shown()[0].body, "repo \u{2014} X");
}

#[test]
fn a_session_of_the_default_entry_is_named_by_the_default_entrys_name() {
    // A7 (FR-004, US1 scenario 7).
    let mut state = repo_state();
    let b = add_session(&mut state, REPO, SessionLocation::Default, "B");
    let notifier = Recording::accepting();

    let _ = state.attention_granted(b, &notifier);

    assert_eq!(notifier.shown()[0].body, "repo \u{2014} Default");
}

#[test]
fn a_session_of_a_project_that_is_not_the_active_one_is_named_by_its_own_project() {
    // U124, A4 (FR-004, US1 scenario 4): its own project's name, and its own project's rename of
    // the worktree — not the active project's.
    let mut state = repo_state();
    add_project(&mut state, OTHER, "other");
    let b = add_session(
        &mut state,
        OTHER,
        SessionLocation::Worktree("feat-x".to_string()),
        "B",
    );
    state
        .workspace
        .set_worktree_name("feat-x", "Active's name")
        .unwrap();
    state
        .workspace
        .worktree_names
        .entry(PathBuf::from(OTHER))
        .or_default()
        .insert("feat-x".to_string(), "Other's name".to_string());
    let notifier = Recording::accepting();

    let _ = state.attention_granted(b, &notifier);

    let shown = notifier.shown();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].body, "other \u{2014} Other's name");
    assert_eq!(shown[0].project, Path::new(OTHER));
}

#[test]
fn a_failure_to_show_is_logged_once_per_run_and_pushes_no_notice() {
    // U121, U122 (FR-010, N4).
    let mut state = repo_state();
    let b = add_session(&mut state, REPO, SessionLocation::Default, "B");
    let notifier = Recording::refusing();
    let notices = state.notifications.clone();

    let first = state.attention_granted(b, &notifier);
    assert!(
        first
            .as_deref()
            .is_some_and(|line| line.contains("nobody owns the name")),
        "the first failure is handed back to be logged, with the system's reason: {first:?}"
    );
    assert_eq!(
        state.notifications, notices,
        "and no in-app notice is pushed"
    );

    assert_eq!(
        state.attention_granted(b, &notifier),
        None,
        "a second failure in the same run is not logged"
    );
    assert_eq!(state.notifications, notices);
    assert_eq!(
        notifier.shown().len(),
        2,
        "each grant is still shown once: no retry, no skip"
    );
}

#[test]
fn a_reported_failure_to_show_is_logged_once_per_run() {
    // Review A F1: the shell shows off the update thread and reports the result back.
    let mut state = repo_state();
    let refused = || Err(NotifyError::NoService("nobody owns the name".to_string()));

    let first = state.attention_shown(refused());
    assert!(
        first
            .as_deref()
            .is_some_and(|line| line.contains("nobody owns the name")),
        "the first failure is handed back to be logged, with the system's reason: {first:?}"
    );
    assert_eq!(
        state.attention_shown(refused()),
        None,
        "a second failure in the same run is not logged"
    );
    assert_eq!(
        state.attention_shown(Ok(())),
        None,
        "a success logs nothing"
    );
}

#[test]
fn a_reported_success_logs_nothing_and_leaves_the_first_failure_to_be_logged() {
    let mut state = repo_state();
    assert_eq!(state.attention_shown(Ok(())), None);
    let first = state.attention_shown(Err(NotifyError::Refused("blocked".to_string())));
    assert!(
        first
            .as_deref()
            .is_some_and(|line| line.contains("blocked")),
        "the first failure is handed back with the system's reason: {first:?}"
    );
}

#[test]
fn the_first_snapshot_after_a_reconnect_is_observed_as_a_reconnect() {
    // U123, A11 (FR-006, US1 scenario 11): a session last seen working and found awaiting input
    // with a higher sequence is claimed once...
    let mut state = repo_state();
    let b = add_session(&mut state, REPO, SessionLocation::Default, "B");
    let _ = state.attention_on_welcome(&working(b, 1), true);

    assert_eq!(
        state.attention_on_welcome(&awaiting(b, 2), true),
        vec![ClientMsg::AttentionClaim { session: b, seq: 2 }]
    );

    // ...and, unlike a live change, not when this window has the session in view: what the user
    // can see after reconnecting needs no notification.
    let mut live = repo_state();
    let b = add_session(&mut live, REPO, SessionLocation::Default, "B");
    live.session.active = Some(b);
    let mut reconnected = live.clone();
    let _ = live.attention_on_welcome(&working(b, 1), true);
    let _ = reconnected.attention_on_welcome(&working(b, 1), true);

    assert_eq!(
        live.attention_on_catalog_changed(&awaiting(b, 2), true)
            .len(),
        1
    );
    assert_eq!(
        reconnected.attention_on_welcome(&awaiting(b, 2), true),
        vec![],
        "the snapshot of a `Welcome` is observed with `Phase::Reconnected`"
    );
}
