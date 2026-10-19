//! Feature 802, FR-001 (T002): the CLI a start chose survives the confirmation that start opened.

use micold_client::app::State;
use micold_client::features::session::{
    additional_cancelled, additional_confirmed, additional_opened,
};
use micold_core::session::{AiCli, Session, SessionKind, SessionLocation};
use std::path::PathBuf;

fn taken() -> SessionLocation {
    SessionLocation::Worktree("feat-a".into())
}

fn state_with_an_occupant() -> State {
    let mut st = State::default();
    let project = PathBuf::from("/a");
    st.workspace.active = Some(project.clone());
    st.workspace.sessions.insert(
        project,
        vec![Session::start_new(taken(), AiCli::ClaudeCode)],
    );
    st
}

#[test]
fn a_start_that_opens_the_confirmation_keeps_its_cli() {
    let mut st = state_with_an_occupant();
    assert!(
        !st.start_offered(&taken()),
        "an occupied location sends no create"
    );
    additional_opened(
        &mut st,
        taken(),
        Some(AiCli::Copilot),
        SessionKind::Terminal,
    );
    assert_eq!(st.session.additional_target, Some(taken()));
    assert_eq!(st.additional_request(), Some((taken(), AiCli::Copilot)));
}

#[test]
fn a_free_location_offers_the_start() {
    let mut st = State::default();
    st.workspace.active = Some(PathBuf::from("/a"));
    assert!(st.start_offered(&taken()));
}

#[test]
fn declining_clears_the_target_and_the_cli() {
    let mut st = state_with_an_occupant();
    additional_opened(
        &mut st,
        taken(),
        Some(AiCli::Copilot),
        SessionKind::Terminal,
    );
    additional_cancelled(&mut st);
    assert_eq!(st.session.additional_target, None);
    assert_eq!(st.session.additional_provider, None);
}

#[test]
fn confirming_clears_the_target_and_the_cli() {
    let mut st = state_with_an_occupant();
    additional_opened(
        &mut st,
        taken(),
        Some(AiCli::Copilot),
        SessionKind::Terminal,
    );
    additional_confirmed(&mut st);
    assert_eq!(st.session.additional_target, None);
    assert_eq!(st.session.additional_provider, None);
}

#[test]
fn a_confirmation_opened_without_a_cli_uses_the_default() {
    let mut st = state_with_an_occupant();
    additional_opened(&mut st, taken(), None, SessionKind::Terminal);
    let (_, provider) = st.additional_request().unwrap();
    assert_eq!(provider, st.session.default_ai_cli);
}

#[test]
fn a_structured_start_keeps_its_kind_until_the_dialog_ends() {
    let mut st = state_with_an_occupant();
    additional_opened(
        &mut st,
        taken(),
        Some(AiCli::ClaudeCode),
        SessionKind::Structured,
    );
    assert_eq!(st.session.additional_kind, SessionKind::Structured);
    additional_confirmed(&mut st);
    assert_eq!(st.session.additional_kind, SessionKind::Terminal);
}
