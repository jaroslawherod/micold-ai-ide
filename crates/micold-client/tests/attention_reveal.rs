//! A click on a desktop notification opens the session (feature 039, US3; contract
//! `desktop-notification.md` N5, N6; research R6).
//!
//! Driven through the root, as the shell drives it: `reveal_session` for
//! `DaemonMsg::RevealSession`, which answers with the messages the shell dispatches. What a
//! backend reports ([`NotifierEvent`]) and how the window is raised ([`raise_plan`]) are pure
//! functions of the feature.

use micold_client::app::{Message, State};
use micold_client::features::attention::{
    in_view, notifier_event, raise_plan, row_unread, NotifierEvent, RaiseStep,
};
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::session::Msg as SessionMsg;
use micold_core::notify::{Level, Notification};
use micold_core::project::{Availability, Project};
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};
use std::path::{Path, PathBuf};

const REPO: &str = "/repo";
const OTHER: &str = "/other";

fn add_project(state: &mut State, path: &str) {
    state.workspace.projects.push(Project::new(
        PathBuf::from(path),
        true,
        Availability::Available,
    ));
}

/// Record a session of `project` and return its id.
fn add_session(state: &mut State, project: &str) -> SessionId {
    let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let id = session.id;
    state
        .workspace
        .sessions
        .entry(PathBuf::from(project))
        .or_default()
        .push(session);
    id
}

/// `REPO` active and `OTHER` known, each with one session; `REPO`'s is the selected one.
fn two_projects() -> (State, SessionId, SessionId) {
    let mut state = State::default();
    add_project(&mut state, REPO);
    add_project(&mut state, OTHER);
    state.workspace.active = Some(PathBuf::from(REPO));
    let in_repo = add_session(&mut state, REPO);
    let in_other = add_session(&mut state, OTHER);
    state.session.active = Some(in_repo);
    (state, in_repo, in_other)
}

fn said(state: &State) -> (Option<Notification>, usize) {
    let queue = &state.notifications.queue;
    (queue.visible().cloned(), queue.pending())
}

/// U131 (FR-011): a click is one request to the service, naming what the notification named.
#[test]
fn an_activated_notification_yields_one_session_reveal() {
    let session = SessionId::new();
    assert_eq!(
        notifier_event(NotifierEvent::Activated {
            project: PathBuf::from(OTHER),
            session,
        }),
        ClientMsg::SessionReveal {
            project: PathBuf::from(OTHER),
            session,
        }
    );
}

/// U132 (FR-011, FR-014, US3-2, A38): a session of a project that is not the active one — the
/// project is reopened, then the session selected, and nothing else is sent.
#[test]
fn a_reveal_for_a_background_project_reopens_it_then_selects_the_session() {
    let (mut state, _, in_other) = two_projects();

    let messages = state.reveal_session(Path::new(OTHER), in_other);

    assert_eq!(
        messages,
        vec![
            Message::Project(ProjectMsg::Reopened(PathBuf::from(OTHER))),
            Message::Session(SessionMsg::Selected(in_other)),
        ]
    );
    assert_eq!(said(&state), (None, 0), "no notice");
}

/// U132 (FR-014, M6 review A F3): the switch a reveal asks for arrives at the revealed session,
/// not at the one the project remembered — so the switch starts no other session.
#[test]
fn the_switch_a_reveal_asks_for_arrives_at_the_revealed_session() {
    let (mut state, _, in_other) = two_projects();
    let remembered = add_session(&mut state, OTHER);
    state
        .workspace
        .foreground_by_project
        .insert(PathBuf::from(OTHER), remembered);

    let messages = state.reveal_session(Path::new(OTHER), in_other);
    assert_eq!(
        messages.first(),
        Some(&Message::Project(ProjectMsg::Reopened(PathBuf::from(
            OTHER
        ))))
    );
    // What the shell does with `Reopened`.
    let _ = state
        .switch_active(Path::new(OTHER))
        .expect("the switch is accepted");

    assert_eq!(
        state.session.active,
        Some(in_other),
        "the switch restores the revealed session, so it is the one session started"
    );
}

/// U133 (FR-011, US3-1, A37, A41): a session of the active project — selected, and nothing else.
#[test]
fn a_reveal_for_the_active_project_selects_the_session_alone() {
    let (mut state, _, _) = two_projects();
    let second = add_session(&mut state, REPO);

    let messages = state.reveal_session(Path::new(REPO), second);

    assert_eq!(
        messages,
        vec![Message::Session(SessionMsg::Selected(second))]
    );
    assert_eq!(said(&state), (None, 0), "no notice");
}

/// U134 (FR-013, US3-4, A40): a session that is gone — one notice, and no selection changes.
#[test]
fn a_reveal_for_a_session_that_is_gone_pushes_the_notice_and_changes_no_selection() {
    let (mut state, in_repo, _) = two_projects();

    let messages = state.reveal_session(Path::new(OTHER), SessionId::new());

    assert_eq!(messages, Vec::new(), "nothing is dispatched");
    assert_eq!(
        said(&state),
        (
            Some(Notification::new(
                Level::Info,
                "That session is no longer available."
            )),
            0
        )
    );
    assert_eq!(state.workspace.active, Some(PathBuf::from(REPO)));
    assert_eq!(state.session.active, Some(in_repo));
}

/// U134 (FR-013, Edge: project forgotten): the same for a project this window does not know, and
/// for one whose folder is unavailable.
#[test]
fn a_reveal_for_a_forgotten_or_unavailable_project_pushes_the_notice() {
    const NOTICE: &str = "That session is no longer available.";
    let (mut state, _, in_other) = two_projects();

    let for_a_forgotten_project = state.reveal_session(Path::new("/forgotten"), in_other);
    assert!(for_a_forgotten_project.is_empty());
    assert_eq!(said(&state).0.map(|n| n.message), Some(NOTICE.to_string()));

    state.notifications.queue.dismiss();
    state.workspace.projects[1].availability = Availability::Unavailable;
    let for_an_unavailable_folder = state.reveal_session(Path::new(OTHER), in_other);
    assert!(for_an_unavailable_folder.is_empty());
    assert_eq!(said(&state).0.map(|n| n.message), Some(NOTICE.to_string()));
}

/// U135 (FR-019, US3-3, A39): the session a reveal shows is in view once its messages are
/// handled, so its row carries no unread mark — before the service has answered.
#[test]
fn the_session_shown_by_a_reveal_is_in_view_so_its_row_is_not_unread() {
    let (mut state, _, _) = two_projects();
    let second = add_session(&mut state, REPO);
    state
        .workspace
        .find_session_mut(second)
        .expect("the session is known")
        .1
        .unread = true;
    let _ = state.view_report(true);
    let unread = |state: &State| {
        let (_, session) = state.workspace.find_session(second).expect("known");
        row_unread(session, in_view(&state.attention))
    };
    assert!(
        unread(&state),
        "precondition: marked while another is shown"
    );

    for message in state.reveal_session(Path::new(REPO), second) {
        state.update(message);
    }
    let _ = state.view_report(true);

    assert!(!unread(&state), "the session is in view");
}

/// U136 (FR-011, N6): off Wayland the window is un-minimised and takes keyboard focus.
#[test]
fn off_wayland_the_window_is_unminimised_then_focused() {
    assert_eq!(
        raise_plan(false),
        vec![RaiseStep::Unminimize, RaiseStep::Focus]
    );
}

/// U137 (FR-011, FR-015, N6): on Wayland a window cannot take focus, so it asks for attention.
#[test]
fn on_wayland_the_window_is_unminimised_then_asks_for_attention() {
    assert_eq!(
        raise_plan(true),
        vec![RaiseStep::Unminimize, RaiseStep::RequestAttention]
    );
}
