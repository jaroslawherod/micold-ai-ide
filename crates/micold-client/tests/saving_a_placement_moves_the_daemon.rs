//! Saving a different place for sessions to run has to move them (BUG-003 — T159–T161).
//!
//! FR-032 asked for a confirmation before the service restarts and FR-033 for the restart itself.
//! Neither was ever scheduled: `data-model.md` §7 filed both under `SandboxState`, `/speckit.tasks`
//! generated nothing for them, and the shipped form recorded the gap as supporting text — "Takes
//! effect the next time the application starts". So choosing **In a container** wrote
//! `settings.json` and did nothing else, and the placement the user had selected was not the one
//! their sessions were in.
//!
//! The rules below are the pure half of the fix, which is where they belong: *whether* a save moves
//! the service, and what a declined confirmation leaves behind, are decisions about state. The
//! effect they authorise — the connection torn down and re-dialled in the new placement — is the
//! shell's, and `main.rs`'s own tests hold that end (T162).

use micold_client::app::{on_escape, Message, State};
use micold_client::features::settings::{
    self, Msg as SettingsMsg, PendingPlacementChange, PlacementStep, SettingsDraft,
};
use micold_core::sandbox::placement::PlacementKind;

/// The Settings view open on a valid draft: the service running where `in_force` says, the
/// placement select showing `chosen`, and an unrelated edit in another section so that "the save
/// applied nothing" can be about more than the placement.
fn open_settings(in_force: PlacementKind, chosen: PlacementKind) -> State {
    let mut draft = SettingsDraft::default();
    draft.terminal.scrollback_lines = "9000".into();
    draft.environment.timeout_secs = "5".into();
    draft.daemon.placement = chosen;

    State {
        settings: settings::State {
            placement_in_force: in_force,
            settings_draft: Some(draft),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The draft as it stands, for the assertions about what a declined save left behind.
fn draft(state: &State) -> &SettingsDraft {
    state
        .settings
        .settings_draft
        .as_ref()
        .expect("the Settings view is open")
}

// ---------------------------------------------------------------------------------------------
// T159 — a save that moves the service asks first, and applies nothing until it is answered
// ---------------------------------------------------------------------------------------------

/// FR-032a. The step is computed against what is *running*, not against what was last written:
/// after an accepted fallback (FR-035a) the stored choice is the container and the service is on
/// the host, and asking again is exactly right — the user is retrying, not repeating themselves.
#[test]
fn a_save_that_moves_the_service_asks_first() {
    assert_eq!(
        settings::placement_step(PlacementKind::HostProcess, PlacementKind::LocalSandbox),
        PlacementStep::Confirm {
            from: PlacementKind::HostProcess,
            to: PlacementKind::LocalSandbox,
        },
        "choosing the container while sessions run on the host is the change FR-032 asks about"
    );
    assert_eq!(
        settings::placement_step(PlacementKind::LocalSandbox, PlacementKind::HostProcess),
        PlacementStep::Confirm {
            from: PlacementKind::LocalSandbox,
            to: PlacementKind::HostProcess,
        },
        "the way back out of the container restarts the service just as much"
    );
}

/// FR-032. The confirmation is a *gate*: while it is up the form is untouched, the view is still
/// open behind it, and nothing has been written. The bug this replaces did the opposite — it
/// applied the whole save and asked nothing.
#[test]
fn asking_applies_nothing() {
    let mut state = open_settings(PlacementKind::HostProcess, PlacementKind::LocalSandbox);

    state.update(Message::Settings(SettingsMsg::PlacementChangeRequested {
        from: PlacementKind::HostProcess,
        to: PlacementKind::LocalSandbox,
    }));

    assert_eq!(
        state.settings.pending_placement,
        Some(PendingPlacementChange {
            from: PlacementKind::HostProcess,
            to: PlacementKind::LocalSandbox,
        }),
        "the change the user is being asked about has to be *the* change, or the dialog cannot \
         name where sessions are going (FR-032)"
    );
    assert_eq!(
        state.settings.placement_in_force,
        PlacementKind::HostProcess,
        "the service moved before the user answered — the confirmation would be a notification"
    );
    assert_eq!(
        draft(&state).daemon.placement,
        PlacementKind::LocalSandbox,
        "the form still shows what the user chose; an unanswered question is not a rejection"
    );
    assert_eq!(
        draft(&state).terminal.scrollback_lines,
        "9000",
        "every other edit is still in the draft, unwritten and unlost"
    );
}

// ---------------------------------------------------------------------------------------------
// T160 — a save that changes nothing asks nothing, and neither does choosing then cancelling
// ---------------------------------------------------------------------------------------------

/// FR-032a. A save is not a restart. Every other reason to press Save — a scrollback size, a theme,
/// an environment script — must not put a dialog about the session service in front of the user.
#[test]
fn a_save_that_leaves_the_service_where_it_is_asks_nothing() {
    for kind in PlacementKind::SELECTABLE {
        assert_eq!(
            settings::placement_step(kind, kind),
            PlacementStep::Leave,
            "{kind:?} saved over {kind:?} is not a change, and only a change may ask (FR-032a)"
        );
    }
}

/// FR-032a, the other half: a placement chosen and then abandoned was never applied, so there is
/// nothing to confirm. Cancel discards the draft, and the pending question — if the user somehow
/// left one open — goes with it.
#[test]
fn choosing_a_placement_and_then_cancelling_asks_nothing() {
    let mut state = open_settings(PlacementKind::HostProcess, PlacementKind::HostProcess);

    state.update(Message::Settings(SettingsMsg::PlacementChanged(
        PlacementKind::LocalSandbox,
    )));
    state.update(Message::Settings(SettingsMsg::Cancelled));

    assert!(
        state.settings.pending_placement.is_none(),
        "choosing is not saving — the question is asked by Save, and Save was never pressed"
    );
    assert_eq!(
        state.settings.placement_in_force,
        PlacementKind::HostProcess,
        "an abandoned draft moved the service"
    );
    assert!(
        state.settings.settings_draft.is_none(),
        "Cancel closes the view, as it always has"
    );
}

// ---------------------------------------------------------------------------------------------
// T161 — declining leaves the entire save unapplied
// ---------------------------------------------------------------------------------------------

/// FR-032b. The failure this forbids is a save that applies its other seven fields and then asks
/// about the eighth: the user declines, the dialog goes away, and the theme they were only
/// half-sure about is now written to disk. Declining means nothing happened.
#[test]
fn declining_leaves_the_whole_save_unapplied() {
    let mut state = open_settings(PlacementKind::HostProcess, PlacementKind::LocalSandbox);

    state.update(Message::Settings(SettingsMsg::PlacementChangeRequested {
        from: PlacementKind::HostProcess,
        to: PlacementKind::LocalSandbox,
    }));
    state.update(Message::Settings(SettingsMsg::PlacementChangeCancelled));

    assert!(
        state.settings.pending_placement.is_none(),
        "the question was answered; the dialog has to close"
    );
    assert_eq!(
        state.settings.placement_in_force,
        PlacementKind::HostProcess,
        "a declined move moved the service anyway"
    );
    assert!(
        state.settings.settings_draft.is_some(),
        "the surface stays open: the user said no to the move, not to the form (FR-032b)"
    );
    assert_eq!(
        draft(&state).daemon.placement,
        PlacementKind::LocalSandbox,
        "the draft keeps every edit the user had made — including the one they declined to apply, \
         which is theirs to reconsider rather than ours to revert (FR-032b)"
    );
    assert_eq!(
        draft(&state).terminal.scrollback_lines,
        "9000",
        "an unrelated edit was discarded by declining a question about the session service"
    );
}

/// Escape belongs to the topmost surface, and while the confirmation is up that is the
/// confirmation — not the form underneath it. Without this, dismissing the question would close
/// Settings and take the draft with it, which is the very loss FR-032b is about.
#[test]
fn escape_answers_the_confirmation_not_the_form() {
    let mut state = open_settings(PlacementKind::HostProcess, PlacementKind::LocalSandbox);

    assert_eq!(
        on_escape(&state),
        Some(Message::Settings(SettingsMsg::Cancelled)),
        "with no dialog open Escape still leaves the Settings view"
    );

    state.update(Message::Settings(SettingsMsg::PlacementChangeRequested {
        from: PlacementKind::HostProcess,
        to: PlacementKind::LocalSandbox,
    }));

    assert_eq!(
        on_escape(&state),
        Some(Message::Settings(SettingsMsg::PlacementChangeCancelled)),
        "Escape closed the form out from under an unanswered dialog"
    );
}

/// Confirming clears the question. What it authorises — the service stopped and brought back up —
/// is an effect, and effects are the shell's; the reducer's whole job here is to stop asking.
#[test]
fn confirming_closes_the_question() {
    let mut state = open_settings(PlacementKind::HostProcess, PlacementKind::LocalSandbox);

    state.update(Message::Settings(SettingsMsg::PlacementChangeRequested {
        from: PlacementKind::HostProcess,
        to: PlacementKind::LocalSandbox,
    }));
    state.update(Message::Settings(SettingsMsg::PlacementChangeConfirmed));

    assert!(
        state.settings.pending_placement.is_none(),
        "the dialog outlived the answer"
    );
}

// --- What the question actually says (FR-033) ----------------------------------------------

/// FR-033 asks the confirmation to say **how many sessions stop** and that they become resumable.
/// It is the difference between a dialog a user can answer and one they can only guess at: "the
/// service restarts" is a fact about the application, "your four running sessions stop" is a fact
/// about their afternoon.
#[test]
fn the_question_counts_the_sessions_it_would_stop() {
    let consequence = settings::placement_move_consequence(
        PlacementKind::HostProcess,
        PlacementKind::LocalSandbox,
        4,
    );

    assert!(
        consequence.contains('4'),
        "the user is asked to stop some unstated number of sessions: {consequence}"
    );
    assert!(
        consequence.contains("resum"),
        "nothing says the sessions come back, so declining is the only safe answer: {consequence}"
    );
    assert!(
        consequence.contains("container"),
        "the sentence does not name where the sessions are going: {consequence}"
    );
}

/// One session is not "1 sessions". Small, and the reason it is asserted rather than left to
/// review: this string is built once and read by everyone who ever changes a placement.
#[test]
fn one_session_reads_as_one_session() {
    let consequence = settings::placement_move_consequence(
        PlacementKind::LocalSandbox,
        PlacementKind::HostProcess,
        1,
    );

    assert!(
        consequence.contains("1 session ") || consequence.contains("1 session."),
        "plural agreement: {consequence}"
    );
    assert!(
        !consequence.contains("1 sessions"),
        "plural agreement: {consequence}"
    );
}

/// With nothing running there is nothing to lose, and the sentence must not invent a loss — a
/// dialog that warns about stopping zero sessions teaches the user to dismiss it unread.
#[test]
fn with_nothing_running_the_question_warns_about_nothing() {
    let consequence = settings::placement_move_consequence(
        PlacementKind::HostProcess,
        PlacementKind::LocalSandbox,
        0,
    );

    assert!(
        !consequence.contains('0'),
        "a warning about stopping zero sessions: {consequence}"
    );
    assert!(
        consequence.contains("container"),
        "the sentence still has to say where sessions will run: {consequence}"
    );
}
