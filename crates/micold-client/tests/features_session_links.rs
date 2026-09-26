//! Opening a link, as the session feature sees it (feature 031, T013 — FR-010, FR-015).
//!
//! The reducer decides what an activation asks for and what a failed open tells the user; the shell
//! performs the open (`shell/links.rs`). So these tests read only the returned outcomes and the
//! notification queue, and nothing here reaches an opener.

use micold_client::app::{Message, State};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::{OpenFailure, OpenRequest, Outcome};
use micold_core::link::{CellSpan, Link, LinkOrigin, ResolvedLink, Target};
use micold_core::notify::Level;

const ADDRESS: &str = "https://example.com/docs?q=a%20b#top";

fn web_link(address: &str) -> ResolvedLink {
    ResolvedLink {
        link: Link {
            address: address.to_string(),
            origin: LinkOrigin::Detected,
            cells: vec![CellSpan {
                row: 0,
                cols: 0..address.len() as u16,
            }],
        },
        display: address.to_string(),
        target: Target::Url(address.to_string()),
        needs_confirmation: false,
    }
}

/// Every notification the queue holds, visible or pending, in arrival order.
fn notifications(state: &mut State) -> Vec<(Level, String)> {
    let mut seen = Vec::new();
    while let Some(n) = state.notifications.queue.visible().cloned() {
        seen.push((n.level, n.message));
        state.notifications.queue.dismiss();
    }
    seen
}

fn finished(address: &str, result: Result<(), OpenFailure>) -> SessionMsg {
    SessionMsg::LinkOpenFinished {
        address: address.to_string(),
        result,
    }
}

/// U73 (T5): an activated web or mail address asks the shell to open exactly that address.
#[test]
fn activating_a_url_link_asks_to_open_that_url_verbatim() {
    for address in [ADDRESS, "mailto:team@example.com"] {
        let mut state = State::default();
        let outcomes = micold_client::features::session::update(
            &mut state,
            SessionMsg::LinkActivated(web_link(address)),
        );
        assert_eq!(
            outcomes,
            vec![Outcome::OpenLink(OpenRequest::Url(address.to_string()))],
            "an activation of {address} is one open request for the address as written (FR-010)"
        );
        assert!(
            notifications(&mut state).is_empty(),
            "asking to open notifies nothing by itself"
        );
    }
}

/// U74 (T12): no application set up to open the address is said in the §5 words.
#[test]
fn an_open_with_no_application_notifies_that_nothing_is_set_up() {
    let mut state = State::default();
    state.update(Message::Session(finished(
        ADDRESS,
        Err(OpenFailure::NoApplication),
    )));
    assert_eq!(
        notifications(&mut state),
        vec![(
            Level::Error,
            format!("Couldn't open {ADDRESS}: no application is set up to open it")
        )],
        "one error notification per failed activation, naming the address (FR-015)"
    );
}

/// U75 (T12): a launch failure passes the opener's own reason through.
#[test]
fn a_failed_launch_notifies_the_reason() {
    let mut state = State::default();
    state.update(Message::Session(finished(
        ADDRESS,
        Err(OpenFailure::LaunchFailed(
            "xdg-open exited with status 4".to_string(),
        )),
    )));
    assert_eq!(
        notifications(&mut state),
        vec![(
            Level::Error,
            format!("Couldn't open {ADDRESS}: xdg-open exited with status 4")
        )],
        "the reason is the opener's, after the address (FR-015)"
    );
}

/// A host path the pane resolved from `address`, ready to open without asking.
fn path_link(address: &str, path: &str) -> ResolvedLink {
    ResolvedLink {
        target: Target::HostPath(path.to_string()),
        display: path.to_string(),
        ..web_link(address)
    }
}

/// U78 (T6): an activated file link asks the shell to open that path, and says which address it
/// came from, because that is what a failure is reported against.
#[test]
fn activating_a_file_link_asks_to_open_its_host_path() {
    let address = "file:///home/u/My%20Doc.pdf";
    let mut state = State::default();
    let outcomes = micold_client::features::session::update(
        &mut state,
        SessionMsg::LinkActivated(path_link(address, "/home/u/My Doc.pdf")),
    );
    assert_eq!(
        outcomes,
        vec![Outcome::OpenLink(OpenRequest::Path {
            path: "/home/u/My Doc.pdf".to_string(),
            address: address.to_string(),
        })],
        "the shell is asked for the decoded path, with the address the program printed (FR-010)"
    );
    assert!(
        notifications(&mut state).is_empty(),
        "asking to open notifies nothing by itself"
    );
}

/// A path that still awaits the user's confirmation opens nothing here (contract link-opening O3).
///
/// M6 adds the confirmation surface; until it does, no resolver marks a `HostPath` this way. The
/// reducer refuses it anyway, so the day the flag is set the confirmation cannot be bypassed
/// (review A finding 5).
#[test]
fn a_host_path_that_needs_confirmation_opens_nothing_yet() {
    let link = ResolvedLink {
        needs_confirmation: true,
        ..path_link("file://box/work/a.md", "/home/u/work/a.md")
    };
    let mut state = State::default();
    let outcomes =
        micold_client::features::session::update(&mut state, SessionMsg::LinkActivated(link));
    assert!(
        outcomes.is_empty(),
        "a path awaiting confirmation is not opened behind the user's back: {outcomes:?}"
    );
}

/// U79 (T8): a sandboxed path outside every shared location opens nothing and says why.
#[test]
fn activating_an_unreachable_link_opens_nothing_and_says_the_sandbox_does_not_share_it() {
    let address = "file:///tmp/x";
    let link = ResolvedLink {
        target: Target::Unreachable(micold_core::link::Reason::NotShared),
        display: "/tmp/x — not reachable from this machine".to_string(),
        ..web_link(address)
    };
    let mut state = State::default();
    let outcomes =
        micold_client::features::session::update(&mut state, SessionMsg::LinkActivated(link));
    assert!(
        outcomes.is_empty(),
        "nothing is opened for a path this machine cannot reach: {outcomes:?}"
    );
    assert_eq!(
        notifications(&mut state),
        vec![(
            Level::Error,
            format!(
                "Couldn't open {address}: the sandbox doesn't share that location with this machine"
            )
        )],
        "the notice names the address the program printed, not the hint's suffix (FR-015, FR-018)"
    );
}

/// U77: a file link to something that is not there.
#[test]
fn an_open_of_a_file_that_is_not_there_notifies_that_it_does_not_exist() {
    let address = "file:///home/u/gone.txt";
    let mut state = State::default();
    state.update(Message::Session(finished(
        address,
        Err(OpenFailure::NotFound),
    )));
    assert_eq!(
        notifications(&mut state),
        vec![(
            Level::Error,
            format!("Couldn't open {address}: the file doesn't exist on this machine")
        )],
        "US3 scenario 5: the user is told the file is gone, not that no application is set up"
    );
}

/// U76 (T13): an open that worked is silent.
#[test]
fn an_open_that_worked_notifies_nothing() {
    let mut state = State::default();
    let outcomes = micold_client::features::session::update(&mut state, finished(ADDRESS, Ok(())));
    assert!(outcomes.is_empty(), "a finished open asks for nothing more");
    assert!(
        notifications(&mut state).is_empty(),
        "the browser opening is the feedback; a notification would be noise"
    );
}

// ---------------------------------------------------------------------------------------
// The confirmation before a sandboxed file opens (feature 031, T059 — FR-018a, T7/T9/T10)
// ---------------------------------------------------------------------------------------

use micold_client::features::session::{
    link_open_confirmed, ConfirmLinkOpenDialog, PendingLinkOpen,
};
use micold_client::overlay::registry::Registered;
use micold_client::overlay::FloatingSurface;
use micold_core::session::{AiCli, Session, SessionLocation};

const CONTAINER: &str = "file:///work/proj/notes.md";
const HOST: &str = "/home/u/proj/notes.md";

/// A link the sandbox resolved: a host path that must be confirmed first (FR-018a).
fn sandboxed_link() -> ResolvedLink {
    ResolvedLink {
        needs_confirmation: true,
        ..path_link(CONTAINER, HOST)
    }
}

/// A state with one session, which is the active one.
fn with_a_session() -> (State, micold_core::session::SessionId) {
    let mut state = State::default();
    let session = Session::start_new(
        SessionLocation::Worktree("feat-a".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    state
        .workspace
        .sessions
        .insert(std::path::PathBuf::from("/a"), vec![session]);
    state.workspace.active = Some(std::path::PathBuf::from("/a"));
    state.session.active = Some(id);
    (state, id)
}

/// U80 (T7): the confirmation opens, naming the host path, and nothing is opened yet.
#[test]
fn a_link_that_needs_confirmation_opens_the_confirm_surface_and_nothing_else() {
    let (mut state, id) = with_a_session();
    let outcomes = micold_client::features::session::update(
        &mut state,
        SessionMsg::LinkActivated(sandboxed_link()),
    );
    assert!(
        outcomes.is_empty(),
        "the sandbox's file is not opened before the user says so: {outcomes:?}"
    );
    assert_eq!(
        state.session.pending_link_open,
        Some(PendingLinkOpen {
            session: id,
            link: sandboxed_link(),
        }),
        "the link is captured with the session it came from, so a later confirmation acts on what \
         was activated rather than on whatever is under the pointer then (FR-017, FR-018a)"
    );
    let open = ConfirmLinkOpenDialog::open_in(&state).expect("the confirm surface is showing");
    assert_eq!(
        open.id(),
        micold_client::overlay::SurfaceId::new("confirm_link_open"),
        "and it is the registered surface, not an ad-hoc one (contract link-opening §4)"
    );
    assert_eq!(
        state
            .session
            .pending_link_open
            .as_ref()
            .map(|p| p.link.display.as_str()),
        Some(HOST),
        "what the dialog shows is the host path that will open (FR-018a)"
    );
}

/// U81 (T9): confirming, with the session and the sandbox both still there, opens the host path.
#[test]
fn confirming_while_the_session_and_sandbox_live_opens_the_host_path() {
    let (mut state, _) = with_a_session();
    micold_client::features::session::update(
        &mut state,
        SessionMsg::LinkActivated(sandboxed_link()),
    );
    let outcomes = link_open_confirmed(&mut state, true);
    assert_eq!(
        outcomes,
        vec![Outcome::OpenLink(OpenRequest::Path {
            path: HOST.to_string(),
            address: CONTAINER.to_string(),
        })],
        "the confirmed open is the one the reducer refused before, unchanged (T6, T9)"
    );
    assert!(
        notifications(&mut state).is_empty(),
        "a confirmed open says nothing until the opener answers"
    );
    assert_eq!(
        state.session.pending_link_open, None,
        "the confirmation is spent, so the surface closes"
    );
}

/// U82 (T9): the session closed while the confirmation was up.
#[test]
fn confirming_after_the_session_closed_opens_nothing_and_says_so() {
    let (mut state, _) = with_a_session();
    micold_client::features::session::update(
        &mut state,
        SessionMsg::LinkActivated(sandboxed_link()),
    );
    state.workspace.sessions.clear();
    let outcomes = link_open_confirmed(&mut state, true);
    assert!(
        outcomes.is_empty(),
        "the session that printed the path is gone: {outcomes:?}"
    );
    assert_eq!(
        notifications(&mut state),
        vec![(
            Level::Error,
            format!("Couldn't open {HOST}: the session has closed")
        )],
        "the user is told why nothing happened, naming the path they confirmed (edge case \
         \u{201c}Pending opens\u{201d}, contract link-opening \u{a7}4)"
    );
    assert_eq!(state.session.pending_link_open, None);
}

/// U83 (T9): the sandbox stopped while the confirmation was up.
#[test]
fn confirming_after_the_sandbox_stopped_opens_nothing_and_says_so() {
    let (mut state, _) = with_a_session();
    micold_client::features::session::update(
        &mut state,
        SessionMsg::LinkActivated(sandboxed_link()),
    );
    let outcomes = link_open_confirmed(&mut state, false);
    assert!(
        outcomes.is_empty(),
        "the container that held the file is gone: {outcomes:?}"
    );
    assert_eq!(
        notifications(&mut state),
        vec![(
            Level::Error,
            format!("Couldn't open {HOST}: the sandbox has stopped")
        )],
        "a stopped sandbox is a different reason from a closed session, and the user acts on it \
         differently"
    );
    assert_eq!(state.session.pending_link_open, None);
}

/// U84 (T10): declining opens nothing at all.
#[test]
fn declining_opens_nothing_and_clears_the_pending_open() {
    let (mut state, _) = with_a_session();
    micold_client::features::session::update(
        &mut state,
        SessionMsg::LinkActivated(sandboxed_link()),
    );
    let outcomes =
        micold_client::features::session::update(&mut state, SessionMsg::LinkOpenDeclined);
    assert!(outcomes.is_empty(), "declining opens nothing: {outcomes:?}");
    assert_eq!(
        state.session.pending_link_open, None,
        "and the surface closes, so the next activation is not answered by this one's Open button"
    );
    assert!(
        notifications(&mut state).is_empty(),
        "the user declined; telling them they declined is noise"
    );
    assert!(
        ConfirmLinkOpenDialog::open_in(&state).is_none(),
        "nothing is left showing"
    );
}

/// Review B finding 8: the reducer's own `LinkOpenConfirmed` arm is the fallback, and it is inert
/// on purpose.
///
/// Whether the sandbox is live is the binary's fact, so the answer is performed by the route through
/// `main.rs` and `shell/links.rs`. If that route were ever removed, this pins what is left: the
/// question stays on screen and nothing opens, rather than the confirmation being bypassed.
#[test]
fn the_reducer_alone_leaves_the_question_up_and_opens_nothing() {
    let (mut state, session) = with_a_session();
    state.session.pending_link_open = Some(PendingLinkOpen {
        session,
        link: sandboxed_link(),
    });

    let outcomes =
        micold_client::features::session::update(&mut state, SessionMsg::LinkOpenConfirmed);

    assert!(
        outcomes.is_empty(),
        "the reducer alone asks for nothing: it cannot see whether the sandbox is still there"
    );
    assert!(
        state.session.pending_link_open.is_some(),
        "so the question is still pending, which is what keeps the dialog up"
    );
    assert!(notifications(&mut state).is_empty(), "and says nothing");
}

/// A confirmation with nothing pending opens nothing, rather than reaching for the last link.
#[test]
fn confirming_with_nothing_pending_opens_nothing() {
    let (mut state, _) = with_a_session();
    assert!(link_open_confirmed(&mut state, true).is_empty());
    assert!(notifications(&mut state).is_empty());
}
