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
