//! Feature 039, milestone M6: a click on a notification is sent to the service as `SessionReveal`,
//! and the service forwards it as `RevealSession` to exactly one window (wire W3, research R6).
//!
//! Same harness as `attention_claims.rs`: `server::serve_connection` over in-memory duplexes, one
//! per simulated window, all sharing one `DaemonState`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use futures_util::SinkExt;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use micold_core::session::SessionId;

#[path = "support/attention.rs"]
mod attention;
use attention::{connect, next_frame, session_id, Service, Window};

const NONCE: u64 = 0x039;

impl Service {
    /// Every file of the store with its bytes: what the service keeps on disk.
    fn stored(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        std::fs::read_dir(self.store.path())
            .expect("the store directory is read")
            .map(|entry| entry.expect("a directory entry").path())
            .filter(|path| path.is_file())
            .map(|path| {
                let bytes = std::fs::read(&path).expect("a store file is read");
                (path, bytes)
            })
            .collect()
    }
}

/// has been sent the forward, or was not going to be.
async fn exchange(window: &mut Window, msg: Option<ClientMsg>) -> Vec<DaemonMsg> {
    if let Some(msg) = msg {
        window
            .send(Frame::Control(msg))
            .await
            .expect("the message is sent");
    }
    window
        .send(Frame::Control(ClientMsg::Ping { nonce: NONCE }))
        .await
        .expect("the ping is sent");
    let mut before_the_pong = Vec::new();
    loop {
        match next_frame(window).await {
            Some(Frame::Control(DaemonMsg::Pong { nonce })) if nonce == NONCE => {
                return before_the_pong;
            }
            Some(Frame::Control(msg)) => before_the_pong.push(msg),
            Some(Frame::Grid(_)) => {}
            None => panic!("the service closed the connection of a window"),
        }
    }
}

fn reveal(project: &Path, session: SessionId) -> ClientMsg {
    ClientMsg::SessionReveal {
        project: project.to_path_buf(),
        session,
        activation: None,
    }
}

fn forwarded(project: &Path, session: SessionId) -> Vec<DaemonMsg> {
    vec![DaemonMsg::RevealSession {
        project: project.to_path_buf(),
        session,
        activation: None,
    }]
}

async fn reports_focus(window: &mut Window) {
    let answers = exchange(
        window,
        Some(ClientMsg::WindowView {
            focused: true,
            in_view: None,
        }),
    )
    .await;
    assert!(answers.is_empty(), "a view report is not answered");
}

async fn attaches(window: &mut Window, project: &Path) {
    let answers = exchange(
        window,
        Some(ClientMsg::Attach {
            project: project.to_path_buf(),
            force: false,
        }),
    )
    .await;
    assert!(
        answers
            .iter()
            .any(|m| matches!(m, DaemonMsg::Attached { .. })),
        "the window is attached to the project: {answers:?}"
    );
}

/// U97 (FR-012, US3-6, A42): with two windows, the reveal goes to the one attached to the
/// session's project and to no other — not to the sender, and not to the one with focus.
#[tokio::test]
async fn a_reveal_is_forwarded_to_the_window_attached_to_the_project_and_to_no_other() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut holder = connect(&service.state, "holder").await;
    let mut clicked = connect(&service.state, "clicked").await;
    attaches(&mut holder, &project).await;
    reports_focus(&mut clicked).await;

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, a))).await;
    let to_the_holder = exchange(&mut holder, None).await;

    assert_eq!(
        to_the_holder,
        forwarded(&project, a),
        "the window that holds the project is told to show the session, once"
    );
    assert!(
        to_the_sender.is_empty(),
        "the window that was clicked is sent nothing: {to_the_sender:?}"
    );
}

/// U98 (FR-012): with the project attached nowhere, the reveal goes to the window that last
/// reported keyboard focus.
#[tokio::test]
async fn with_the_project_attached_nowhere_a_reveal_goes_to_the_window_that_last_reported_focus() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut earlier = connect(&service.state, "earlier").await;
    let mut latest = connect(&service.state, "latest").await;
    let mut clicked = connect(&service.state, "clicked").await;
    reports_focus(&mut earlier).await;
    reports_focus(&mut latest).await;

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, a))).await;

    assert_eq!(
        exchange(&mut latest, None).await,
        forwarded(&project, a),
        "the window that had focus last is told to show the session"
    );
    assert!(
        exchange(&mut earlier, None).await.is_empty(),
        "a window that had focus before it is sent nothing"
    );
    assert!(
        to_the_sender.is_empty(),
        "nor is the sender: {to_the_sender:?}"
    );
}

/// U99 (FR-012): with no window attached to the project and none that reported focus, the reveal
/// goes back to the sender.
#[tokio::test]
async fn with_no_holder_and_no_focused_window_a_reveal_goes_back_to_the_sender() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut clicked = connect(&service.state, "clicked").await;
    let mut other = connect(&service.state, "other").await;

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, a))).await;

    assert_eq!(to_the_sender, forwarded(&project, a));
    assert!(
        exchange(&mut other, None).await.is_empty(),
        "the other window is sent nothing"
    );
}

/// U100 (FR-013, W3.2): the service forwards without checking that the session or the project
/// exists; the window that receives it decides.
#[tokio::test]
async fn a_reveal_is_forwarded_for_a_session_and_a_project_that_do_not_exist() {
    let service = Service::with_sessions(&[session_id(0xA)]);
    let project = service.project();
    let gone = session_id(0xDEAD);
    let nowhere = Path::new("/no/such/project");
    let mut window = connect(&service.state, "window").await;

    assert_eq!(
        exchange(&mut window, Some(reveal(&project, gone))).await,
        forwarded(&project, gone),
        "a session the service does not know"
    );
    assert_eq!(
        exchange(&mut window, Some(reveal(nowhere, gone))).await,
        forwarded(nowhere, gone),
        "a project the service does not know"
    );
}

/// U101 (FR-014, W3.3, A41): a reveal changes no session, no attachment and nothing stored, and
/// no window is sent anything but the one forward.
#[tokio::test]
async fn a_reveal_changes_no_session_no_attachment_and_nothing_stored() {
    let (a, b) = (session_id(0xA), session_id(0xB));
    let service = Service::with_sessions(&[a, b]);
    let project = service.project();
    let mut holder = connect(&service.state, "holder").await;
    let mut clicked = connect(&service.state, "clicked").await;
    attaches(&mut holder, &project).await;
    // The holder has `a` in view; `b` is the session the click names.
    let _ = exchange(
        &mut holder,
        Some(ClientMsg::WindowView {
            focused: true,
            in_view: Some(a),
        }),
    )
    .await;
    service.state.persist_attention();
    let catalog_before = service.state.catalog_snapshot();
    let stored_before = service.stored();

    let to_the_sender = exchange(&mut clicked, Some(reveal(&project, b))).await;
    let to_the_holder = exchange(&mut holder, None).await;
    service.state.persist_attention();

    assert_eq!(to_the_holder, forwarded(&project, b));
    assert!(
        to_the_sender.is_empty(),
        "no catalog push and no reply reaches the sender: {to_the_sender:?}"
    );
    assert_eq!(
        service.state.catalog_snapshot(),
        catalog_before,
        "no session changed"
    );
    assert!(
        service.state.is_attached(&project),
        "the project is still attached"
    );
    let second_attach = exchange(
        &mut clicked,
        Some(ClientMsg::Attach {
            project: project.clone(),
            force: false,
        }),
    )
    .await;
    assert!(
        !second_attach
            .iter()
            .any(|m| matches!(m, DaemonMsg::Attached { .. })),
        "and still by the window that held it: the sender's own attach is refused"
    );
    assert_eq!(service.stored(), stored_before, "nothing stored changed");
}

/// U102 (FR-011, W3.4): the activation token reaches the target connection unchanged, also when
/// the target is not the sender; the service does not read it.
#[tokio::test]
async fn the_activation_token_reaches_the_target_unchanged_also_when_it_is_not_the_sender() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let project = service.project();
    let mut holder = connect(&service.state, "holder").await;
    let mut clicked = connect(&service.state, "clicked").await;
    attaches(&mut holder, &project).await;
    // Not a token anything would accept: it is forwarded, not interpreted.
    let token = " odd token \u{1F511}\n".to_string();
    let msg = ClientMsg::SessionReveal {
        project: project.clone(),
        session: a,
        activation: Some(token.clone()),
    };

    let to_the_sender = exchange(&mut clicked, Some(msg.clone())).await;
    let to_the_holder = exchange(&mut holder, None).await;
    let to_itself = exchange(&mut clicked, Some(msg)).await;

    assert_eq!(
        to_the_holder,
        vec![DaemonMsg::RevealSession {
            project: project.clone(),
            session: a,
            activation: Some(token),
        }],
        "the target, which is not the sender, is sent the token as it was"
    );
    assert!(to_the_sender.is_empty(), "{to_the_sender:?}");
    assert!(
        to_itself.is_empty(),
        "the holder still holds the project, so the sender is again sent nothing"
    );
}
