//! What this window last told the session service it has in view, in isolation (feature 039,
//! W1.1; feature 021, SC-004).
//!
//! The file builds only `features::attention::State` and hands it the facts as values, so nothing
//! here depends on how the application arrives at them — that is `tests/attention_view_report.rs`,
//! which asks the same questions of `app::State`.
//!
//! `Welcome` is not named below because this feature never sees it: a connection that has been
//! told nothing is a `State` whose `sent_view` is `None`, which is what `State::default()` is and
//! what `connection_started` restores.

use micold_client::features::attention::{connection_started, view_report, State};
use micold_core::attention::ViewFacts;
use micold_core::protocol::messages::WindowView;
use micold_core::session::SessionId;

/// A focused window whose main area shows `selected`.
fn showing(selected: Option<SessionId>) -> ViewFacts {
    ViewFacts {
        window_focused: true,
        main_area_taken: false,
        selected,
    }
}

#[test]
fn the_first_report_of_a_connection_is_sent_also_when_nothing_is_in_view() {
    // U111 (FR-019, US2 scenario 23): the service must hear from every window once, or it cannot
    // tell a window that shows nothing from one that has not spoken yet.
    let mut state = State::default();
    let nothing_in_view = WindowView {
        focused: true,
        in_view: None,
    };

    assert_eq!(
        view_report(&mut state, showing(None)),
        Some(nothing_in_view),
        "a connection that has been told nothing is told, whatever there is to say"
    );
    assert_eq!(
        state.sent_view,
        Some(nothing_in_view),
        "the report handed back is recorded as the one sent"
    );
}

#[test]
fn a_report_is_sent_again_only_when_the_derived_value_differs_from_the_last_one_sent() {
    // U112 (FR-019).
    let first = SessionId::new();
    let second = SessionId::new();
    let mut state = State::default();
    let _ = view_report(&mut state, showing(Some(first)));

    assert_eq!(
        view_report(&mut state, showing(Some(first))),
        None,
        "the same session in view is not news"
    );
    assert_eq!(
        view_report(&mut state, showing(Some(second))),
        Some(WindowView {
            focused: true,
            in_view: Some(second),
        }),
        "another session in view is"
    );
    assert_eq!(
        view_report(&mut state, showing(Some(second))),
        None,
        "and is news once"
    );
}

#[test]
fn losing_focus_reports_an_unfocused_window_with_nothing_in_view() {
    // U113 (US1 scenario 3, US2 scenario 6): the session is still selected, and nobody sees it.
    let session = SessionId::new();
    let mut state = State::default();
    let _ = view_report(&mut state, showing(Some(session)));

    let unfocused = ViewFacts {
        window_focused: false,
        ..showing(Some(session))
    };

    assert_eq!(
        view_report(&mut state, unfocused),
        Some(WindowView {
            focused: false,
            in_view: None,
        })
    );
}

#[test]
fn opening_settings_reports_nothing_in_view_and_leaving_it_reports_the_session_again() {
    // U114 (US1 scenario 9, US2 scenario 11).
    let session = SessionId::new();
    let mut state = State::default();
    let _ = view_report(&mut state, showing(Some(session)));

    let settings_open = ViewFacts {
        main_area_taken: true,
        ..showing(Some(session))
    };
    assert_eq!(
        view_report(&mut state, settings_open),
        Some(WindowView {
            focused: true,
            in_view: None,
        }),
        "Settings fills the main area, so the focused window shows the session to nobody"
    );

    assert_eq!(
        view_report(&mut state, showing(Some(session))),
        Some(WindowView {
            focused: true,
            in_view: Some(session),
        }),
        "leaving Settings puts the session back in view"
    );
}

#[test]
fn a_reconnect_forgets_what_was_sent_so_the_next_report_goes_out_whatever_its_value() {
    // U115 (FR-006): the service a window reconnects to may be a restarted one that knows nothing
    // of the last report, so an unchanged value has to be said again.
    let session = SessionId::new();
    let mut state = State::default();
    let _ = view_report(&mut state, showing(Some(session)));

    connection_started(&mut state);

    assert_eq!(
        state.sent_view, None,
        "a new connection has been sent nothing"
    );
    assert_eq!(
        view_report(&mut state, showing(Some(session))),
        Some(WindowView {
            focused: true,
            in_view: Some(session),
        }),
        "the value did not change, and it is reported again"
    );
}
