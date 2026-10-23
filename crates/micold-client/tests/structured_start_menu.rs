//! T038 (feature 938, FR-022, US6 scenarios 1–2): the start list offers "Claude Code (structured)"
//! right after "Claude Code", with its availability, only where Claude Code is supported, and the
//! chevron shows even with Claude Code as the only CLI. The plain press and every other entry
//! start a terminal session.

#[path = "support/mod.rs"]
mod support;

use micold_client::app::{Message, State};
use micold_client::features::session::{
    additional_opened, session_create, AvailabilityKey, AvailabilitySource, BypassStart,
    CliAvailability, CliEntryAvailability, Msg, PressTarget, StartGate, StartIntent, StartMenu,
};

use micold_core::protocol::messages::ClientMsg;
use micold_core::session::{AiCli, PermissionMode, SessionKind, SessionLocation};
use std::path::{Path, PathBuf};

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

// ---- #942 T054: the structured create flow carries a mode; Bypass is confirmed first (FR-014) ----

fn mode_of(msg: &ClientMsg) -> PermissionMode {
    match msg {
        ClientMsg::SessionCreate {
            permission_mode, ..
        } => *permission_mode,
        other => panic!("a SessionCreate, got {other:?}"),
    }
}

fn create(kind: SessionKind, mode: PermissionMode) -> ClientMsg {
    session_create(
        1,
        PathBuf::from(ROW),
        &SessionLocation::Worktree("feat".into()),
        AiCli::ClaudeCode,
        kind,
        mode,
        false,
    )
}

#[test]
fn a_structured_create_carries_the_chosen_mode_and_none_chosen_is_default() {
    let mut state = State::default();
    assert_eq!(
        state.session.start_gate(SessionKind::Structured),
        StartGate::Create(PermissionMode::Default),
        "none chosen"
    );
    for mode in [
        PermissionMode::AcceptEdits,
        PermissionMode::Plan,
        PermissionMode::Default,
    ] {
        state.update(Message::Session(Msg::StartModeChosen(mode)));
        assert_eq!(
            state.session.start_gate(SessionKind::Structured),
            StartGate::Create(mode)
        );
        assert_eq!(mode_of(&create(SessionKind::Structured, mode)), mode);
    }
}

/// #942 M8 (ledger decision 19): choosing a mode row leaves the start list open where it was, so
/// the structured entry can be pressed next.
#[test]
fn choosing_a_mode_leaves_the_start_list_open() {
    let mut state = State::default();
    state.session.start_menu = bypass_start().menu;
    state.update(Message::Session(Msg::StartModeChosen(PermissionMode::Plan)));
    assert_eq!(state.session.start_menu, bypass_start().menu);
    assert_eq!(state.session.start_mode, PermissionMode::Plan);
}

#[test]
fn a_terminal_create_sends_default_whatever_was_chosen() {
    let mut state = State::default();
    state.update(Message::Session(Msg::StartModeChosen(
        PermissionMode::Bypass,
    )));
    assert_eq!(
        state.session.start_gate(SessionKind::Terminal),
        StartGate::Create(PermissionMode::Default)
    );
    for mode in [PermissionMode::Plan, PermissionMode::Bypass] {
        assert_eq!(
            mode_of(&create(SessionKind::Terminal, mode)),
            PermissionMode::Default
        );
    }
}

fn bypass_start() -> BypassStart {
    BypassStart {
        location: SessionLocation::Worktree("feat".into()),
        provider: AiCli::ClaudeCode,
        menu: Some(StartMenu {
            location: SessionLocation::Worktree("feat".into()),
            anchor: (40, 80),
        }),
    }
}

#[test]
fn choosing_bypass_enters_a_confirmation_and_only_its_confirm_creates() {
    let mut state = State::default();
    state.update(Message::Session(Msg::StartModeChosen(
        PermissionMode::Bypass,
    )));
    assert_eq!(
        state.session.start_gate(SessionKind::Structured),
        StartGate::ConfirmBypass,
        "no create yet"
    );

    state.update(Message::Session(Msg::BypassStartOpened(bypass_start())));
    assert_eq!(state.session.bypass_start, Some(bypass_start()));
    assert_eq!(
        state.session.bypass_start_request(),
        Some((
            SessionLocation::Worktree("feat".into()),
            AiCli::ClaudeCode,
            PermissionMode::Bypass
        )),
        "what the confirm creates"
    );

    state.update(Message::Session(Msg::BypassStartConfirmed));
    assert_eq!(state.session.bypass_start, None);
    assert_eq!(state.session.bypass_start_request(), None, "once");
    assert_eq!(
        state.session.start_gate(SessionKind::Structured),
        StartGate::Create(PermissionMode::Default),
        "Bypass is the confirmed start's only: the next is in Default"
    );
    assert_eq!(
        mode_of(&create(SessionKind::Structured, PermissionMode::Bypass)),
        PermissionMode::Bypass
    );
}

#[test]
fn cancelling_the_bypass_confirmation_returns_to_the_choice() {
    let mut state = State::default();
    state.update(Message::Session(Msg::StartModeChosen(
        PermissionMode::Bypass,
    )));
    state.update(Message::Session(Msg::BypassStartOpened(bypass_start())));
    state.update(Message::Session(Msg::BypassStartCancelled));
    assert_eq!(state.session.bypass_start, None);
    assert_eq!(state.session.bypass_start_request(), None);
    assert_eq!(
        state.session.start_menu,
        bypass_start().menu,
        "the start list is open again where it was"
    );
    assert_eq!(
        state.session.start_mode,
        PermissionMode::Bypass,
        "with the choice as it was"
    );
}

#[test]
fn an_additional_session_keeps_the_mode_of_the_start_that_opened_it() {
    let mut state = State::default();
    state.update(Message::Session(Msg::AdditionalOpened {
        location: SessionLocation::Worktree("feat".into()),
        provider: Some(AiCli::ClaudeCode),
        kind: SessionKind::Structured,
        permission_mode: PermissionMode::Plan,
    }));
    assert_eq!(state.session.additional_mode, PermissionMode::Plan);
    state.update(Message::Session(Msg::StartModeChosen(
        PermissionMode::Bypass,
    )));
    assert_eq!(
        state.session.additional_mode,
        PermissionMode::Plan,
        "a later choice does not reach a confirmation already open"
    );
    state.update(Message::Session(Msg::AdditionalCancelled));
    assert_eq!(state.session.additional_mode, PermissionMode::Default);
}

#[test]
fn the_rows_another_session_confirmation_creates_in_default() {
    let mut state = State::default();
    state.update(Message::Session(Msg::StartModeChosen(
        PermissionMode::Bypass,
    )));
    additional_opened(
        &mut state,
        SessionLocation::Worktree("feat".into()),
        None,
        SessionKind::Terminal,
    );
    assert_eq!(state.session.additional_mode, PermissionMode::Default);
}
