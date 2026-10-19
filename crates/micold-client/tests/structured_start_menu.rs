//! T038 (feature 938, FR-022, US6 scenarios 1–2): the start list offers "Claude Code (structured)"
//! right after "Claude Code", with its availability, only where Claude Code is supported, and the
//! chevron shows even with Claude Code as the only CLI. The plain press and every other entry
//! start a terminal session.

#[path = "support/mod.rs"]
mod support;

use micold_client::app::State;
use micold_client::features::session::{
    AvailabilityKey, AvailabilitySource, CliAvailability, CliEntryAvailability, PressTarget,
    StartIntent,
};

use micold_core::session::{AiCli, SessionKind};
use std::path::Path;

const ROW: &str = "/repo";

fn state_with(available: &[AiCli]) -> State {
    let mut state = State::default();
    support::hold(
        &mut state,
        AvailabilityKey::Dir(ROW.into()),
        CliAvailability {
            available: available.to_vec(),
            source: AvailabilitySource::ThisComputer,
            env: None,
            asked_for: AvailabilityKey::Dir(ROW.into()),
        },
    );
    state
}

#[test]
fn the_structured_entry_follows_claude_code_with_its_availability() {
    let state = state_with(&[AiCli::ClaudeCode]);
    let entries = state.session.start_menu_entries(Path::new(ROW));
    let at = entries
        .iter()
        .position(|e| e.cli == AiCli::ClaudeCode && e.kind == SessionKind::Terminal)
        .expect("the plain Claude Code entry");
    let next = &entries[at + 1];
    assert_eq!(
        (next.cli, next.kind),
        (AiCli::ClaudeCode, SessionKind::Structured)
    );
    assert_eq!(next.availability, entries[at].availability);
    assert_eq!(next.availability, CliEntryAvailability::Available);
}

#[test]
fn an_unavailable_claude_code_makes_the_structured_entry_unavailable_too() {
    let state = state_with(&[AiCli::Copilot]);
    let entries = state.session.start_menu_entries(Path::new(ROW));
    let structured = entries
        .iter()
        .find(|e| e.kind == SessionKind::Structured)
        .expect("offered, disabled");
    assert!(matches!(
        structured.availability,
        CliEntryAvailability::Unavailable { .. }
    ));
}

#[test]
fn only_claude_code_has_a_structured_entry_and_every_other_is_terminal() {
    let state = state_with(&AiCli::ALL);
    let entries = state.session.start_menu_entries(Path::new(ROW));
    let structured: Vec<AiCli> = entries
        .iter()
        .filter(|e| e.kind == SessionKind::Structured)
        .map(|e| e.cli)
        .collect();
    assert_eq!(structured, vec![AiCli::ClaudeCode]);
    let terminal: Vec<AiCli> = entries
        .iter()
        .filter(|e| e.kind == SessionKind::Terminal)
        .map(|e| e.cli)
        .collect();
    assert_eq!(terminal, AiCli::ALL);
}

#[test]
fn the_chevron_shows_with_claude_code_as_the_only_cli() {
    let state = state_with(&[AiCli::ClaudeCode]);
    assert!(state
        .session
        .start_affordance_offers_a_choice(Path::new(ROW)));
}

#[test]
fn the_chevron_stays_absent_when_the_only_cli_has_no_structured_mode() {
    let state = state_with(&[AiCli::Copilot]);
    assert!(!state
        .session
        .start_affordance_offers_a_choice(Path::new(ROW)));
}

#[test]
fn the_plain_press_still_starts_the_default_in_one_press() {
    let state = state_with(&[AiCli::ClaudeCode]);
    assert_eq!(
        state
            .session
            .start_intent(PressTarget::Primary, Path::new(ROW)),
        StartIntent::Start(AiCli::ClaudeCode)
    );
}
