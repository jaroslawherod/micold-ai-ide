//! The third label kind: a label derived from the conversation's first turn (feature 032, T003 —
//! data-model *Transitions*).
//!
//! A session's label is a title the AI CLI recorded (`Named`), a label derived from what the user
//! first typed (`Derived`), or neither (`Pending`). The order matters and is one way: a title
//! replaces a label, a label only ever fills an empty row, and nothing goes back to `Pending`.

use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};

fn session_with(label: SessionLabel) -> Session {
    Session::restored(
        SessionId::from_uuid(uuid::Uuid::from_u128(0x32)),
        SessionLocation::Default,
        label,
        TerminalMode::AiCli,
        AiCli::ClaudeCode,
    )
}

const FIRST_TURN: &str = "Why does the sidebar read New session?";

#[test]
fn a_derived_label_displays_its_text_like_a_title() {
    assert_eq!(
        SessionLabel::Derived(FIRST_TURN.into()).display(),
        FIRST_TURN,
        "a label is shown in the same place and the same way as a title (D7)"
    );
}

#[test]
fn a_pending_session_takes_a_derived_label_and_reports_the_change() {
    let mut session = session_with(SessionLabel::Pending);
    assert!(
        session.set_derived_label(FIRST_TURN),
        "the caller persists only on a change, so the change must be reported"
    );
    assert_eq!(session.label, SessionLabel::Derived(FIRST_TURN.into()));
}

#[test]
fn a_named_session_is_never_given_a_derived_label() {
    let mut session = session_with(SessionLabel::Named("The CLI's title".into()));
    assert!(
        !session.set_derived_label(FIRST_TURN),
        "a title outranks a label (FR-005)"
    );
    assert_eq!(session.label, SessionLabel::Named("The CLI's title".into()));
}

#[test]
fn a_derived_label_is_never_replaced_by_another_derived_label() {
    let mut session = session_with(SessionLabel::Derived("The first thing typed".into()));
    assert!(
        !session.set_derived_label("Something typed later"),
        "a label is derived once and then remembered (FR-007)"
    );
    assert_eq!(
        session.label,
        SessionLabel::Derived("The first thing typed".into())
    );
}

#[test]
fn an_empty_derived_label_changes_nothing() {
    let mut session = session_with(SessionLabel::Pending);
    assert!(
        !session.set_derived_label(""),
        "an empty label would render as a blank row; \"New session\" is the truthful reading (FR-004)"
    );
    assert_eq!(session.label, SessionLabel::Pending);
}

#[test]
fn a_title_replaces_a_derived_label() {
    let mut session = session_with(SessionLabel::Derived(FIRST_TURN.into()));
    session.set_title("The CLI's title");
    assert_eq!(
        session.label,
        SessionLabel::Named("The CLI's title".into()),
        "a title that arrives later replaces the label (FR-006)"
    );
}
