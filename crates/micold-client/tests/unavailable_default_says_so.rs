//! Feature 026, BUG-001 (FR-002, FR-004 scenario 4) — a stored default that is not installed is
//! *said*, not only worked around.
//!
//! Three of the four clauses shipped: the press starts nothing, substitutes nothing, and leaves the
//! stored default as the user left it. The fourth — telling them why they got a list instead of a
//! session — did not, and the gate that covers the other three could not see it, because
//! `StartIntent::OfferChoice` returned the same value for this press as for a deliberate press on
//! the chevron. The reason was gone before anything could draw it.
//!
//! So the reason travels with the press, and the reducer re-checks it against the availability set
//! **the same press refreshed** before saying anything. That ordering is what
//! [`a_default_that_turned_out_to_be_installed_says_nothing`] holds: the flag says why the user
//! pressed, never what is true now.

use micold_client::app::{drain, interpret, State};
use micold_client::features::session::{start_menu_toggled, PressTarget, StartIntent};
use micold_core::notify::Level;
use micold_core::session::{AiCli, SessionLocation};

/// The daemon says this when a start fails on a missing CLI (`micold-daemon/src/state.rs`, T088).
/// The application now says it one step earlier, when it can tell *before* launching anything —
/// deliberately in the same words, since it is the same fact about the same CLI.
const SENTENCE: &str =
    "GitHub Copilot isn't installed. Install it, or start this session on another AI CLI.";

fn state_with(default_ai_cli: AiCli, available: &[AiCli]) -> State {
    State {
        default_ai_cli,
        available_providers: available.to_vec(),
        ..State::default()
    }
}

/// Open the list the way a press does: the reducer's answer, drained into the state.
fn open(state: &mut State, unavailable_default: Option<AiCli>) {
    let outcomes = start_menu_toggled(state, SessionLocation::Default, unavailable_default);
    drain(outcomes, |outcome| interpret(state, outcome));
}

#[test]
fn the_press_carries_the_reason_the_list_is_opening() {
    // The value the view dispatches on. Both halves offer the same CLIs; only one of them is
    // answering a question the user did not ask.
    let state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    assert_eq!(
        state.start_intent(PressTarget::Primary),
        StartIntent::OfferChoice {
            providers: vec![AiCli::ClaudeCode],
            unavailable_default: Some(AiCli::Copilot),
        },
        "the primary half opened a list the user did not ask for, and has to be able to say why"
    );
    assert_eq!(
        state.start_intent(PressTarget::Secondary),
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
        .notify
        .visible()
        .expect("the user pressed start and got a menu; nothing yet says why");
    assert_eq!(visible.message, SENTENCE);
    assert_eq!(
        visible.level,
        Level::Error,
        "the session they asked for did not start — that is the level errors get, and the ten \
         seconds that go with it"
    );
    assert!(
        state.session_start_menu.is_some(),
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
        .notify
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

    assert!(state.session_start_menu.is_some(), "the list still opens");
    assert_eq!(state.notify.visible(), None);
}

#[test]
fn a_default_that_turned_out_to_be_installed_says_nothing() {
    // The press published the reason; the binary then re-probed `PATH` on the same message and
    // found the CLI (research R11's second named event). The reason is stale by one event, and the
    // reducer is what settles it — a banner naming a CLI the list is about to offer is a lie.
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode, AiCli::Copilot]);

    open(&mut state, Some(AiCli::Copilot));

    assert!(state.session_start_menu.is_some());
    assert_eq!(state.notify.visible(), None);
}

#[test]
fn closing_the_list_again_says_nothing() {
    // `start_menu_toggled` is a toggle, and the primary half publishes the same message on the
    // press that closes an open list. Announcing there would tell the user why a list they just
    // dismissed had opened.
    let mut state = state_with(AiCli::Copilot, &[AiCli::ClaudeCode]);

    open(&mut state, Some(AiCli::Copilot));
    state.notify.dismiss();
    open(&mut state, Some(AiCli::Copilot));

    assert!(
        state.session_start_menu.is_none(),
        "the second press closed it"
    );
    assert_eq!(state.notify.visible(), None);
}
