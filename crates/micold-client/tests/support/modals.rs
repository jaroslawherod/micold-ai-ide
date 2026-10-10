//! The modal table shared by the overlay tests (issue #700): every modal dialog the client opens,
//! with the name it registers under, the message that cancels it, and how to put the state into
//! "this dialog is open". Written out rather than derived from the registry, so each test that reads
//! it holds the code under test against an independent statement of the set.
//!
//! A dialog is open when the state it draws from is there, so a row has to build that state. A test
//! with dialogs of its own (the registry's, which also lists those with bespoke state) extends
//! `modals()` rather than copying it.

use micold_client::app::Message;
use micold_client::app::State;
use micold_client::features::help::Msg as HelpMsg;
use micold_client::features::project::Msg as ProjectMsg;
use micold_client::features::project::RenameDraft;
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::session::PendingLinkOpen;
use micold_client::features::worktree::Msg as WorktreeMsg;
use micold_client::features::worktree::WorktreeRenameDraft;
use micold_core::selector::Selector;
use micold_core::session::SessionId;
use std::path::PathBuf;

/// One modal dialog: its registered name, the message that cancels it, and how to open it.
pub struct Modal {
    pub id: &'static str,
    pub cancel: Message,
    pub open: fn(&mut State),
}

/// Which dialog is open, by name.
pub fn open_dialog(state: &State) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(state).map(|open| open.id().as_str())
}

/// A state with just that dialog open.
pub fn opened(open: fn(&mut State)) -> State {
    let mut state = State::default();
    open(&mut state);
    state
}

/// The eleven dialogs every overlay test knows about, in registration order.
pub fn modals() -> Vec<Modal> {
    fn row(id: &'static str, cancel: Message, open: fn(&mut State)) -> Modal {
        Modal { id, cancel, open }
    }
    vec![
        row("about", Message::Help(HelpMsg::AboutClosed), |s| {
            s.help.about_open = true
        }),
        row(
            "project_selector",
            Message::Project(ProjectMsg::SelectorClosed),
            |s| s.project.selector = Some(Selector::open_at(PathBuf::from("/tmp"))),
        ),
        row(
            "rename_project",
            Message::Project(ProjectMsg::RenameCancelled),
            |s| {
                s.project.rename_draft = Some(RenameDraft {
                    path: PathBuf::from("/tmp"),
                    text: String::new(),
                    error: None,
                })
            },
        ),
        row(
            "add_worktree",
            Message::WorktreeForm(micold_client::features::worktree_form::Msg::Cancelled),
            |s| s.worktree_form.form = Some(Default::default()),
        ),
        row(
            "confirm_worktree_delete",
            Message::Worktree(WorktreeMsg::DeleteCancelled),
            |s| s.worktree.delete_target = Some("wt".to_string()),
        ),
        row(
            "rename_worktree",
            Message::Worktree(WorktreeMsg::RenameCancelled),
            |s| {
                s.worktree.rename_draft = Some(WorktreeRenameDraft {
                    dir_name: "wt".to_string(),
                    text: String::new(),
                    error: None,
                })
            },
        ),
        row(
            "confirm_session_remove",
            Message::Session(SessionMsg::RemoveCancelled),
            |s| s.session.remove_target = Some(SessionId::new()),
        ),
        row(
            "confirm_discard_pending",
            Message::Changes(micold_client::features::changes::Msg::DiscardCancelled),
            open_discard_pending,
        ),
        row(
            "confirm_forget_project",
            Message::Project(ProjectMsg::ForgetCancelled),
            |s| s.project.forget_target = Some(PathBuf::from("/p")),
        ),
        row(
            "confirm_link_open",
            Message::Session(SessionMsg::LinkOpenDeclined),
            |s| {
                s.session.pending_link_open = Some(PendingLinkOpen {
                    session: SessionId::new(),
                    link: a_sandboxed_link(),
                })
            },
        ),
        row(
            "confirm_agent_request",
            Message::AgentConfirm(micold_client::features::agent_confirm::Msg::Dismissed),
            |s| s.agent_confirm.pending = vec![an_agent_request()],
        ),
    ]
}

/// A resolved sandboxed file link: one that translated to a host path and so needs a confirmation.
pub fn a_sandboxed_link() -> micold_core::link::ResolvedLink {
    micold_core::link::ResolvedLink {
        link: micold_core::link::Link {
            address: "file:///work/p/a.md".to_string(),
            origin: micold_core::link::LinkOrigin::Detected,
            cells: Vec::new(),
        },
        display: "/home/u/p/a.md".to_string(),
        target: micold_core::link::Target::HostPath("/home/u/p/a.md".to_string()),
        needs_confirmation: true,
    }
}

/// An agent's destructive request, pending an answer (feature 034, FR-014).
pub fn an_agent_request() -> micold_client::features::agent_confirm::Prompt {
    micold_client::features::agent_confirm::Prompt {
        id: 1,
        project: PathBuf::from("/p"),
        caller_label: "planner".to_string(),
        operation: micold_core::protocol::messages::ConfirmOperation::DeleteSession,
        target_label: "reviewer".to_string(),
    }
}

/// Open the Changes view's **Discard pending…** confirmation (feature 482, S3): a view with one
/// pending comment, then the press.
pub fn open_discard_pending(state: &mut State) {
    use micold_client::features::changes::{self, Msg as ChangesMsg};
    use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
    let project = PathBuf::from("/p");
    let _ = changes::update(
        &mut state.changes,
        ChangesMsg::Opened {
            project: project.clone(),
            entry: micold_core::session::SessionLocation::Default,
        },
    );
    let _ = changes::update(
        &mut state.changes,
        ChangesMsg::ReviewChanged {
            project,
            worktree_dir: String::new(),
            comments: vec![ReviewComment {
                id: CommentId::new(),
                path: micold_core::review::RelPath::from_native("a.rs").unwrap(),
                side: micold_core::review::Side::New,
                range: micold_core::review::LineRange::new(1, 1).unwrap(),
                quote: vec!["a".into()],
                text: "t".into(),
                state: CommentState::Pending,
                created: 1,
            }],
            sending: false,
        },
    );
    let _ = changes::update(&mut state.changes, ChangesMsg::DiscardPendingPressed);
}
