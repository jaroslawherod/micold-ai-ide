//! Feature 026, BUG-001 (FR-002, FR-004 scenario 4) — a stored default that is not installed is
//! *said*, not only worked around.
//!
//! Three of the four clauses shipped: the press starts nothing, substitutes nothing, and leaves the
//! stored default as the user left it. The fourth — telling them why they got a list instead of a
//! session — did not, and the gate that covers the other three could not see it, because
//! `StartIntent::OfferChoice` returned the same value for this press as for a deliberate press on
//! the chevron. The reason was gone before anything could draw it.
//!
//! So the reason travels with the press, and the reducer re-checks it against the answer the row
//! holds for its own directory (feature 033; home's while its own is pending, FR-005) before
//! saying anything. That re-check is what
//! [`a_default_that_turned_out_to_be_installed_says_nothing`] holds: the flag says why the user
//! pressed, never what is true now.

#[path = "support/mod.rs"]
mod support;

use micold_client::app::{drain, interpret, State};
use micold_client::features::session::{
    start_menu_toggled, AvailabilityKey, AvailabilitySource, CliAvailability, PressTarget,
    StartIntent,
};
use micold_core::cli_reason::{start_refusal, start_refusal_unknown, AttemptDir, Place, SpawnEnv};
use micold_core::notify::Level;
use micold_core::session::{AiCli, SessionLocation};
use micold_core::terminal::LaunchMode;
use std::path::Path;

/// A row with no answer of its own, so it reads the home answer (feature 033, FR-005).
const ROW: &str = "/repo";

/// The six states of feature 037's FR-001, one per row of contract W2.
const STATES: [SpawnEnv; 6] = [
    SpawnEnv::IncludeOff,
    SpawnEnv::NoScriptPath,
    SpawnEnv::ScriptNotFound,
    SpawnEnv::ScriptFailed,
    SpawnEnv::ScriptTimedOut,
    SpawnEnv::Applied,
];

/// An image reference, for the answers of a service that runs in a container.
const IMAGE: &str = "ghcr.io/acme/dev:1";

/// A home answer that carries no environment state (037 FR-011): the service did not say why.
///
/// No project is open, so the list is judged by the home answer directly.
fn state_with(default_ai_cli: AiCli, available: &[AiCli]) -> State {
    let mut state = state_holding(
        default_ai_cli,
        available,
        None,
        AvailabilitySource::ThisComputer,
    );
    state.workspace.active = None;
    state
}

/// The project at [`ROW`] open, with a home answer in `env` settled at `source`. The row has no
/// answer of its own, so it reads this one (033 FR-005).
fn state_holding(
    default_ai_cli: AiCli,
    available: &[AiCli],
    env: Option<SpawnEnv>,
    source: AvailabilitySource,
) -> State {
    let mut state = State::default();
    state.session.default_ai_cli = default_ai_cli;
    state.workspace.active = Some(ROW.into());
    support::hold_home(
        &mut state,
        CliAvailability {
            available: available.to_vec(),
            source,
            env,
            asked_for: AvailabilityKey::Home,
        },
    );
    state
}

/// File an answer of the row's own, the way the reply to a request naming [`ROW`] lands.
fn hold_row(state: &mut State, available: &[AiCli], env: Option<SpawnEnv>) {
    state
        .session
        .availability
        .asked(1, AvailabilityKey::Dir(ROW.into()));
    state.session.availability.answered(
        1,
        CliAvailability {
            available: available.to_vec(),
            source: AvailabilitySource::ThisComputer,
            env,
            asked_for: AvailabilityKey::Home,
        },
    );
}

/// Open the list the way a press does: the reducer's answer, drained into the state.
fn open(state: &mut State, unavailable_default: Option<AiCli>) {
    let outcomes = start_menu_toggled(state, SessionLocation::Default, unavailable_default);
    drain(outcomes, |outcome| interpret(state, outcome));
}

/// Everything the queue holds, shown or waiting: the visible message and how many wait behind it.
fn said(state: &State) -> (Option<String>, usize) {
    let queue = &state.notifications.queue;
    (queue.visible().map(|n| n.message.clone()), queue.pending())
}

#[test]
fn the_press_carries_the_reason_the_list_is_opening() {
    // The value the view dispatches on. Both halves offer the same CLIs; only one of them is
    // answering a question the user did not ask.
    let state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    assert_eq!(
        state
            .session
            .start_intent(PressTarget::Primary, Path::new(ROW)),
        StartIntent::OfferChoice {
            providers: vec![AiCli::ClaudeCode],
            unavailable_default: Some(AiCli::Copilot),
        },
        "the primary half opened a list the user did not ask for, and has to be able to say why"
    );
    assert_eq!(
        state
            .session
            .start_intent(PressTarget::Secondary, Path::new(ROW)),
        StartIntent::OfferChoice {
            providers: vec![AiCli::ClaudeCode],
            unavailable_default: None,
        },
        "the chevron asked for the list; nothing about it needs explaining"
    );
}

#[test]
fn an_unavailable_default_is_named_when_the_list_opens_in_its_place() {
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    open(&mut state, Some(AiCli::Copilot));

    let visible = state
        .notifications
        .queue
        .visible()
        .expect("the user pressed start and got a menu; nothing yet says why");
    // 037 U72 (FR-002, research R8): this answer carries no state, so no cause is claimed.
    assert_eq!(visible.message, start_refusal_unknown(AiCli::Copilot));
    assert_eq!(
        visible.level,
        Level::Error,
        "the session they asked for did not start — that is the level errors get, and the ten \
         seconds that go with it"
    );
    assert!(
        state.session.start_menu.is_some(),
        "and it still offers what is available; saying so replaces nothing (FR-002)"
    );
}

#[test]
fn the_sentence_names_the_cli_the_way_a_menu_does() {
    // FR-010's register, applied to FR-002's sentence: "GitHub Copilot", not "copilot". The two
    // strings are distinct for every provider, so a leak is observable rather than a coincidence.
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    open(&mut state, Some(AiCli::Copilot));

    let message = state
        .notifications
        .queue
        .visible()
        .expect("said something")
        .message
        .clone();
    assert!(message.contains(AiCli::Copilot.provider().display_name()));
    assert!(
        !message.contains(AiCli::Copilot.provider().command()),
        "a sentence, not a shell error: {message}"
    );
}

#[test]
fn the_chevron_opens_the_same_list_and_says_nothing() {
    // The noise this rule exists to avoid. A user whose default is uninstalled and who knows it
    // opens the override list as often as anyone else, and does not need telling every time.
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    open(&mut state, None);

    assert!(state.session.start_menu.is_some(), "the list still opens");
    assert_eq!(state.notifications.queue.visible(), None);
}

#[test]
fn a_default_that_turned_out_to_be_installed_says_nothing() {
    // The press published the reason, but the row's answer already lists the CLI — an event since
    // the press's own reading (one of contract C1's) brought it in. The reason is stale by one
    // event, and the reducer is what settles it — a banner naming a CLI the list offers is a lie.
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode, AiCli::Copilot]);

    open(&mut state, Some(AiCli::Copilot));

    assert!(state.session.start_menu.is_some());
    assert_eq!(state.notifications.queue.visible(), None);
}

#[test]
fn closing_the_list_again_says_nothing() {
    // `start_menu_toggled` is a toggle, and the primary half publishes the same message on the
    // press that closes an open list. Announcing there would tell the user why a list they just
    // dismissed had opened.
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    open(&mut state, Some(AiCli::Copilot));
    state.notifications.queue.dismiss();
    open(&mut state, Some(AiCli::Copilot));

    assert!(
        state.session.start_menu.is_none(),
        "the second press closed it"
    );
    assert_eq!(state.notifications.queue.visible(), None);
}

#[test]
fn an_unavailable_pi_default_is_named_as_pi_coding_agent() {
    // Feature 029, T032 (FR-001a). The same press, the same sentence, the third CLI: nothing about
    // it is Pi-specific, which is the point — the name comes from the provider, so a new CLI gets
    // the explanation without the reducer learning its name.
    let mut state = state_with(AiCli::Pi, &[AiCli::ClaudeCode, AiCli::Copilot]);

    open(&mut state, Some(AiCli::Pi));

    let message = state
        .notifications
        .queue
        .visible()
        .expect("the user pressed start on an uninstalled Pi default and got a menu")
        .message
        .clone();
    assert_eq!(message, start_refusal_unknown(AiCli::Pi));
    assert!(message.contains("Pi Coding Agent"), "{message}");
    assert!(
        !message
            .split(|c: char| !c.is_alphanumeric())
            .any(|word| word == AiCli::Pi.provider().command()),
        "a sentence, not a shell error: {message}"
    );
}

/// Feature 033 (FR-008, U36): the notice judges the default by the list's own directory. A default
/// the row's directory provides is not missing there, whatever the home directory says.
#[test]
fn a_default_the_lists_directory_provides_is_not_reported_missing() {
    let mut state = state_with(AiCli::Pi, &[AiCli::ClaudeCode]);
    state.workspace.active = Some(ROW.into());
    hold_row(&mut state, &[AiCli::ClaudeCode, AiCli::Pi], None);

    open(&mut state, Some(AiCli::Pi));

    assert_eq!(
        state
            .notifications
            .queue
            .visible()
            .map(|n| n.message.clone()),
        None,
        "Pi is installed where a session from this list would run, so nothing is missing"
    );
}

// ---------------------------------------------------------------------------------------------
// Feature 037, surface U3: the message gives the reason of the answer the row is drawn from
// ---------------------------------------------------------------------------------------------

/// 037 U69 (FR-008, FR-012, contract W4 row U3): one notification, and it is the sentence
/// `cli_reason` writes for the state of the answer the row offers from.
#[test]
fn in_each_state_the_press_says_that_states_refusal_once() {
    for env in STATES {
        let mut state = state_holding(
            AiCli::Pi,
            &[AiCli::ClaudeCode],
            Some(env),
            AvailabilitySource::ThisComputer,
        );

        open(&mut state, Some(AiCli::Pi));

        assert_eq!(
            said(&state),
            (
                Some(start_refusal(
                    AiCli::Pi,
                    env,
                    Place::ThisComputer,
                    AttemptDir::Home,
                    LaunchMode::Fresh,
                )),
                0
            ),
            "{env:?}: the reason is the state of the answer in use, said once"
        );
    }
}

/// 037 U71 (FR-008, FR-015): the press says why and offers what is available. It starts nothing
/// and rewrites nothing, in every state.
#[test]
fn in_each_state_the_press_opens_the_list_starts_nothing_and_keeps_the_stored_default() {
    for env in STATES {
        let mut state = state_holding(
            AiCli::Pi,
            &[AiCli::ClaudeCode, AiCli::Copilot],
            Some(env),
            AvailabilitySource::ThisComputer,
        );
        let sessions_before = state.workspace.sessions.clone();

        let outcomes = start_menu_toggled(&mut state, SessionLocation::Default, Some(AiCli::Pi));

        assert_eq!(
            outcomes,
            vec![micold_client::features::notifications::error(
                start_refusal(
                    AiCli::Pi,
                    env,
                    Place::ThisComputer,
                    AttemptDir::Home,
                    LaunchMode::Fresh,
                )
            )],
            "{env:?}: the notice is the only thing the press asks for; nothing in it is a start"
        );
        assert_eq!(
            state
                .session
                .start_menu
                .as_ref()
                .map(|menu| menu.location.clone()),
            Some(SessionLocation::Default),
            "{env:?}: the list is open on the row that was pressed"
        );
        assert_eq!(
            state.session.offered_providers(Some(Path::new(ROW))),
            vec![AiCli::ClaudeCode, AiCli::Copilot],
            "{env:?}: and it offers the CLIs a session there would find"
        );
        assert_eq!(
            state.workspace.sessions, sessions_before,
            "{env:?}: no session was made"
        );
        assert_eq!(
            state.session.default_ai_cli,
            AiCli::Pi,
            "{env:?}: the stored default is the user's, and a missing CLI does not rewrite it"
        );
    }
}

/// 037 U72 (FR-002, contract W5, research R8): an answer without a state gives no cause, in the
/// row's own answer as in the home answer.
#[test]
fn an_answer_without_a_state_says_only_that_the_cli_would_not_be_found() {
    // The home answer has a state. The row's own has none, and the row's own is the one in use.
    let mut state = state_holding(
        AiCli::Pi,
        &[AiCli::ClaudeCode],
        Some(SpawnEnv::IncludeOff),
        AvailabilitySource::ThisComputer,
    );
    hold_row(&mut state, &[AiCli::ClaudeCode], None);

    open(&mut state, Some(AiCli::Pi));

    assert_eq!(
        said(&state),
        (Some(start_refusal_unknown(AiCli::Pi)), 0),
        "the home answer's reason is not borrowed for an offer drawn from the row's answer \
         (FR-012)"
    );
}

/// 037 U73 (FR-004a): the directory named is the one the answer in use was asked for.
#[test]
fn a_row_on_the_home_answer_names_the_home_directory_and_its_own_answer_names_its_own() {
    let mut state = state_holding(
        AiCli::Pi,
        &[AiCli::ClaudeCode],
        Some(SpawnEnv::ScriptTimedOut),
        AvailabilitySource::ThisComputer,
    );

    open(&mut state, Some(AiCli::Pi));

    let (on_home, _) = said(&state);
    assert_eq!(
        on_home,
        Some(start_refusal(
            AiCli::Pi,
            SpawnEnv::ScriptTimedOut,
            Place::ThisComputer,
            AttemptDir::Home,
            LaunchMode::Fresh,
        ))
    );
    assert!(
        on_home
            .as_deref()
            .is_some_and(|m| m.contains("timed out for your home directory") && !m.contains(ROW)),
        "the attempt this row reports was made for the home directory, not for {ROW}: {on_home:?}"
    );

    // The row's own answer arrives, in another state. Close the list and press again.
    hold_row(
        &mut state,
        &[AiCli::ClaudeCode],
        Some(SpawnEnv::ScriptFailed),
    );
    state.notifications.queue.dismiss();
    open(&mut state, Some(AiCli::Pi));
    assert!(
        state.session.start_menu.is_none(),
        "fixture: this closed it"
    );
    open(&mut state, Some(AiCli::Pi));

    let (on_its_own, waiting) = said(&state);
    assert_eq!(
        (on_its_own.clone(), waiting),
        (
            Some(start_refusal(
                AiCli::Pi,
                SpawnEnv::ScriptFailed,
                Place::ThisComputer,
                AttemptDir::Dir(Path::new(ROW)),
                LaunchMode::Fresh,
            )),
            0
        )
    );
    assert!(
        on_its_own.as_deref().is_some_and(|m| {
            m.contains(&format!("exited with an error for {ROW},")) && !m.contains("home directory")
        }),
        "the row's own answer reports the attempt made in its own directory: {on_its_own:?}"
    );
}

/// 037 U74 (FR-005, FR-008): an answer settled in an image gives the image form in every state,
/// and 027's own sentence where the image itself lacks the CLI.
#[test]
fn an_answer_settled_in_an_image_says_the_image_form() {
    for env in STATES {
        let mut state = state_holding(
            AiCli::Copilot,
            &[AiCli::ClaudeCode],
            Some(env),
            AvailabilitySource::Image(IMAGE.to_string()),
        );

        open(&mut state, Some(AiCli::Copilot));

        let (message, waiting) = said(&state);
        assert_eq!(
            (message.clone(), waiting),
            (
                Some(start_refusal(
                    AiCli::Copilot,
                    env,
                    Place::Image(IMAGE),
                    AttemptDir::Home,
                    LaunchMode::Fresh,
                )),
                0
            ),
            "{env:?}"
        );
        assert!(
            message.as_deref().is_some_and(|m| m.contains(IMAGE)),
            "{env:?}: the image is where sessions run, so the sentence names it: {message:?}"
        );
    }

    let mut state = state_holding(
        AiCli::Copilot,
        &[AiCli::ClaudeCode],
        Some(SpawnEnv::Applied),
        AvailabilitySource::Image(IMAGE.to_string()),
    );
    open(&mut state, Some(AiCli::Copilot));
    assert_eq!(
        said(&state).0.as_deref(),
        Some(
            "GitHub Copilot isn't in ghcr.io/acme/dev:1, where sessions run. Choose an image that \
             provides it, or start this session on another AI CLI."
        ),
        "027's sentence for an image that lacks the CLI, byte for byte"
    );
}
