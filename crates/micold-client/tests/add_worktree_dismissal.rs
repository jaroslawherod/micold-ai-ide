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

/// U3 — a create in flight over an open Settings draft still leaves the scrim with nothing to do.
///
/// `on_escape` falls back to cancelling the Settings draft when the registry answers nothing. The
/// registry answers nothing both when no dialog is open and when the open one refuses, so without
/// a distinction a scrim click during the create would throw away the user's unsaved settings
/// behind a dialog that stayed put (plan.md, Bugfix BUG-001, "Trap: `on_escape`'s fallback").
#[test]
fn a_create_in_flight_over_a_settings_draft_does_not_fall_through_to_the_draft() {
    let mut state = State::default();
    state.update(Message::Settings(
        micold_client::features::settings::Msg::Opened,
    ));
    state.update(Message::WorktreeForm(FormMsg::Opened));
    state.update(Message::WorktreeForm(FormMsg::CreateStarted(
        Default::default(),
    )));
    assert!(
        state.settings.settings_draft.is_some(),
        "precondition: the Settings draft is still open behind the add-worktree dialog"
    );

    assert_eq!(
        on_escape(&state),
        None,
        "the refusing dialog on top must stop the Settings fallback, or a click on its scrim \
         discards the draft behind it (FR-010a)"
    );
}

/// U4 — the protection lasts exactly as long as the create: once it fails, Escape closes again.
///
/// A failed create returns the form to `Editing` with the error on it, and the user may just want
/// to walk away. Holding the dialog past the operation would outlive the reason for it (FR-010a,
/// "for exactly the span of the operation").
#[test]
fn a_form_whose_create_failed_is_dismissed_by_escape_again() {
    let mut state = form_creating();
    state.update(Message::WorktreeForm(FormMsg::CreateFailed(
        "git failed to create the worktree".into(),
    )));

    assert_eq!(
        micold_client::overlay::registry::escape(&state),
        Some(Message::WorktreeForm(FormMsg::Cancelled)),
        "a create that has failed is over; Escape must close its form again"
    );
    assert_eq!(
        on_escape(&state),
        Some(Message::WorktreeForm(FormMsg::Cancelled)),
        "a create that has failed is over; the scrim must close its form again"
    );
}

/// U5 — the in-dialog Cancel still closes a form whose create is in flight.
///
/// With Escape and the scrim both refused, the button is the only way out (FR-010a). It closes
/// the overlay and nothing else: the create runs on in the session service (FR-010b, ledger D1).
#[test]
fn the_in_dialog_cancel_closes_a_form_whose_create_is_in_flight() {
    let mut state = form_creating();

    state.update(Message::WorktreeForm(FormMsg::Cancelled));

    assert!(
        state.worktree_form.form.is_none(),
        "Cancel must close the add-worktree form even while its create runs — it is the only way \
         out once Escape and the scrim are refused (FR-010a)"
    );
}

/// The notification on screen, as (level, message), or `None` when nothing was raised.
fn notice(state: &State) -> Option<(micold_core::notify::Level, String)> {
    state
        .notifications
        .queue
        .visible()
        .map(|n| (n.level, n.message.clone()))
}

/// A state whose add-worktree form was cancelled while its create was still running.
fn cancelled_mid_create() -> State {
    let mut state = form_creating();
    state.update(Message::WorktreeForm(FormMsg::Cancelled));
    state
}

/// U6 — a create that fails after its form was cancelled reports the failure as a notification.
///
/// Cancel closed the overlay and did not stop the create (FR-010b). With the form gone, the
/// failure has no error line to land on; without a notification the user is back where the bug
/// report started, unable to tell whether the worktree exists.
#[test]
fn a_create_failing_after_cancel_is_reported_as_an_error_notification() {
    let mut state = cancelled_mid_create();

    state.update(Message::WorktreeForm(FormMsg::CreateFailed(
        "git failed to create the worktree".into(),
    )));

    let (level, message) = notice(&state)
        .expect("a create that failed after its form closed must still be reported (FR-010b)");
    assert_eq!(
        level,
        micold_core::notify::Level::Error,
        "a failed create is an error"
    );
    assert!(
        message.contains("git failed to create the worktree"),
        "the notification must carry the failure's own message, got {message:?}"
    );
}

/// U7 — the failure notification names the stage the create had reached when it was cancelled.
///
/// FR-009 makes the failed stage identifiable in the dialog; FR-010b carries that into the
/// notification, since "setting up submodules" and "creating the branch" leave different things
/// behind on disk.
#[test]
fn a_failure_after_cancel_names_the_stage_it_failed_at() {
    use micold_core::worktree::CreateStage;
    let mut state = form_creating();
    state.update(Message::WorktreeForm(FormMsg::CreateStageChanged(
        CreateStage::SettingUpSubmodules,
        None,
    )));
    state.update(Message::WorktreeForm(FormMsg::Cancelled));

    state.update(Message::WorktreeForm(FormMsg::CreateFailed(
        "git failed to fetch a submodule".into(),
    )));

    let stage = CreateStage::SettingUpSubmodules.label(&Default::default());
    let (_, message) = notice(&state).expect("the failure must be reported (FR-010b)");
    assert!(
        message.contains(stage),
        "the notification must name the stage the create failed at ({stage:?}), got {message:?}"
    );
}

/// U8 — a stage reported after Cancel still counts: the failure names where the create really was.
///
/// The create runs on after its form closed, so the daemon keeps reporting progress. A failure in
/// the submodule fetch must not be blamed on the branch creation the form last showed.
#[test]
fn a_stage_reported_after_cancel_is_the_one_a_failure_names() {
    use micold_core::worktree::CreateStage;
    let mut state = form_creating();
    state.update(Message::WorktreeForm(FormMsg::CreateStageChanged(
        CreateStage::CreatingWorktree,
        None,
    )));
    state.update(Message::WorktreeForm(FormMsg::Cancelled));

    state.update(Message::WorktreeForm(FormMsg::CreateStageChanged(
        CreateStage::SettingUpSubmodules,
        None,
    )));
    state.update(Message::WorktreeForm(FormMsg::CreateFailed(
        "git failed to fetch a submodule".into(),
    )));

    let stage = CreateStage::SettingUpSubmodules.label(&Default::default());
    let (_, message) = notice(&state).expect("the failure must be reported (FR-010b)");
    assert!(
        message.contains(stage),
        "the notification must name the stage reached after Cancel ({stage:?}), got {message:?}"
    );
}

/// A worktree as the daemon's success reply describes it.
fn worktree_named(dir_name: &str) -> micold_core::worktree::Worktree {
    micold_core::worktree::Worktree {
        dir_name: dir_name.into(),
        path: std::path::PathBuf::from("/repo/demo/.claude/worktrees").join(dir_name),
        branch: None,
        status: micold_core::worktree::WorktreeStatus::Valid,
        included: false,
    }
}

/// U9 — a create that succeeds after its form was cancelled says so, naming the worktree.
///
/// The user who pressed Cancel cannot know the create was not aborted unless something tells them
/// (FR-010b): the worktree appearing in the sidebar is easy to miss, and the dialog that would
/// have closed on success is already gone.
#[test]
fn a_create_succeeding_after_cancel_is_reported_naming_the_worktree() {
    let mut state = cancelled_mid_create();

    state.update(Message::WorktreeForm(FormMsg::Created(worktree_named(
        "feat-probe-two",
    ))));

    let (level, message) = notice(&state)
        .expect("a create that succeeded after its form closed must still be reported (FR-010b)");
    assert_eq!(
        level,
        micold_core::notify::Level::Info,
        "a successful create is not an error"
    );
    assert!(
        message.contains("feat-probe-two"),
        "the notification must name the created worktree, got {message:?}"
    );
}

/// U10 — with the form open, outcomes keep their in-dialog presentation and raise no notification.
///
/// The dialog shows a failure on its error line and closes on success (FR-009, FR-010); a
/// notification on top of either would say the same thing twice (FR-010b).
#[test]
fn outcomes_with_the_form_open_raise_no_notification() {
    let mut failed = form_creating();
    failed.update(Message::WorktreeForm(FormMsg::CreateFailed(
        "git failed to create the worktree".into(),
    )));
    assert_eq!(
        notice(&failed),
        None,
        "a failure the open dialog shows must not also be a notification"
    );
    assert!(
        failed.worktree_form.worktree_error.is_some(),
        "the open dialog must still show the failure on its own error line"
    );

    let mut created = form_creating();
    created.update(Message::WorktreeForm(FormMsg::Created(worktree_named(
        "feat-probe-two",
    ))));
    assert_eq!(
        notice(&created),
        None,
        "a success the open dialog closes on must not also be a notification"
    );
}
