//! The add-worktree dialog holds its ground while a create is in flight (feature 013, BUG-001).
//!
//! A create in a repository with submodules can run for half a minute. The dialog used to be an
//! ordinary dialog for all of it, so a stray click on the scrim closed it, taking the progress
//! display with it and leaving no word on whether the worktree was ever created. FR-010a makes it
//! non-dismissible for exactly the span of the operation — the in-dialog Cancel stays the one way
//! out — and FR-010b reports an outcome that lands after that Cancel.
//!
//! Driven through the public entry points the window uses: `State::update`, the overlay registry
//! (what the Escape key consults) and `app::on_escape` (what the scrim is wired to).

use micold_client::app::{on_escape, Message, State};
use micold_client::features::worktree_form::Msg as FormMsg;

/// A state with the add-worktree form open and nothing in flight.
fn form_open() -> State {
    let mut state = State::default();
    state.update(Message::WorktreeForm(FormMsg::Opened));
    state
}

/// U1 — with nothing in flight the form is an ordinary dialog, closed by Escape and the scrim.
///
/// The "no creation in progress" side of FR-010a: protecting the form while it is idle would turn
/// every accidental open into a dialog the user has to find the button for.
#[test]
fn an_idle_form_is_dismissed_by_escape_and_the_scrim() {
    let state = form_open();

    assert_eq!(
        micold_client::overlay::registry::escape(&state),
        Some(Message::WorktreeForm(FormMsg::Cancelled)),
        "Escape must cancel an idle add-worktree form, as it cancels every other dialog"
    );
    assert_eq!(
        on_escape(&state),
        Some(Message::WorktreeForm(FormMsg::Cancelled)),
        "the scrim must cancel an idle add-worktree form, exactly as Escape does"
    );
}

/// A state whose add-worktree form has sent its create and is waiting on the daemon.
fn form_creating() -> State {
    let mut state = form_open();
    state.update(Message::WorktreeForm(FormMsg::CreateStarted(
        Default::default(),
    )));
    state
}

/// U2 — while a create is in flight, nothing implicit closes the form (FR-010a).
///
/// The reported case: thirty seconds into a submodule fetch, a click on the scrim closed the
/// dialog and took the progress display with it, and nothing afterwards said whether the worktree
/// existed. Escape reaches the same cancellation, so it is held to the same rule.
#[test]
fn a_form_whose_create_is_in_flight_is_dismissed_by_neither_escape_nor_the_scrim() {
    let state = form_creating();

    assert_eq!(
        micold_client::overlay::registry::escape(&state),
        None,
        "Escape cancelled a form whose create was still running — the progress display is gone \
         and the outcome will have nowhere to land (FR-010a)"
    );
    assert_eq!(
        on_escape(&state),
        None,
        "a click on the scrim cancelled a form whose create was still running (FR-010a)"
    );
}
