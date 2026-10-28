//! #1554 T022 — the effort picker beside the model switcher, render-free (FR-001–FR-003, FR-012;
//! US1 scenarios 1, 2, 6).
//!
//! Drives the window's state as the shell does: the trigger toggles the picker, the arrows move its
//! highlight, a row (or `Enter` on the highlight) picks, and the choice to send comes back from
//! [`Window::effort_picked`]. What the service then holds arrives on the catalog, as the model's
//! does; the send itself and a refusal's notice are the shell's (`shell/composer.rs` tests).

mod support;

use micold_client::app::State as Window;
use micold_client::features::effort_picker::{
    availability_note, effort_label, EffortRow, CURRENT, NO_EFFORT_NOTE,
};
use micold_core::session::{AiCli, Session, SessionId, SessionKind, SessionLocation};
use micold_core::structured::effort::{EffortChoice, EffortLevel, EffortSupport, EFFORT_LEVELS};
use micold_core::structured::{ReportedModel, SequencedEvent, StructuredEvent};
use micold_core::typeahead::Direction;

fn level(name: &str) -> EffortLevel {
    EffortLevel::parse(name).expect("a level")
}

fn chosen(name: &str) -> EffortChoice {
    EffortChoice::Level(level(name))
}

fn support(supported: bool, levels: &[&str]) -> EffortSupport {
    EffortSupport {
        supported,
        levels: levels.iter().map(|l| level(l)).collect(),
    }
}

fn session_of(cli: AiCli, kind: SessionKind) -> Session {
    Session::start_new_of_kind(SessionLocation::Worktree("feat-00".into()), cli, kind)
}

/// A window holding `session`; `models` (if any) is what its CLI reported.
fn window_with(session: Session, models: Option<Vec<ReportedModel>>) -> (Window, SessionId) {
    let id = session.id;
    let mut workspace = support::workspace_with(vec![("/fixture/project", vec![session])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut window = Window {
        workspace,
        ..Window::default()
    };
    if let Some(models) = models {
        let reported = StructuredEvent::CommandsReported {
            commands: vec![],
            models,
        };
        let events = [SequencedEvent {
            seq: 0,
            event: reported,
        }];
        window.structured_transcript.apply(id, 0, &events, true);
    }
    (window, id)
}

fn window(models: Option<Vec<ReportedModel>>) -> (Window, SessionId) {
    window_with(
        session_of(AiCli::ClaudeCode, SessionKind::Structured),
        models,
    )
}

/// The CLI's default model, reporting `support` for its effort.
fn default_model(support: Option<EffortSupport>) -> Vec<ReportedModel> {
    vec![ReportedModel {
        value: "default".into(),
        display_name: Some("Default".into()),
        description: None,
        effort: support,
    }]
}

fn set_effort(w: &mut Window, s: SessionId, effort: EffortChoice) {
    let path = w.workspace.active.clone().expect("active");
    let session = w
        .workspace
        .sessions
        .get_mut(&path)
        .and_then(|l| l.iter_mut().find(|x| x.id == s))
        .expect("held");
    session.effort = effort;
}

fn rows(w: &Window, s: SessionId) -> Vec<EffortRow> {
    w.effort_picker
        .picker(s)
        .expect("the picker is open")
        .rows()
        .to_vec()
}

fn choices(w: &Window, s: SessionId) -> Vec<EffortChoice> {
    rows(w, s).into_iter().map(|r| r.choice).collect()
}

fn current(w: &Window, s: SessionId) -> Vec<EffortChoice> {
    rows(w, s)
        .into_iter()
        .filter(|r| r.current)
        .map(|r| r.choice)
        .collect()
}

fn highlighted(w: &Window, s: SessionId) -> Option<EffortChoice> {
    let picker = w.effort_picker.picker(s)?;
    picker
        .rows()
        .get(picker.highlight()?)
        .map(|r| r.choice.clone())
}

fn all_levels() -> Vec<EffortChoice> {
    std::iter::once(EffortChoice::Default)
        .chain(EFFORT_LEVELS.iter().map(|l| chosen(l)))
        .collect()
}

// ---- the trigger (FR-001, US1 scenario 1) ----

#[test]
fn the_trigger_names_the_choice() {
    assert_eq!(effort_label(&EffortChoice::Default), "Effort: Default");
    assert_eq!(effort_label(&chosen("high")), "Effort: high");
    let (mut w, s) = window(None);
    assert_eq!(w.effort_header(s), "Effort: Default", "a new session's");
    set_effort(&mut w, s, chosen("xhigh"));
    assert_eq!(w.effort_header(s), "Effort: xhigh", "the catalog's choice");
}

#[test]
fn the_control_is_offered_only_for_a_structured_claude_code_session() {
    let (w, s) = window(None);
    assert!(w.offers_effort(s), "a structured Claude Code session");
    for (cli, kind) in [
        (AiCli::ClaudeCode, SessionKind::Terminal),
        (AiCli::Copilot, SessionKind::Structured),
        (AiCli::Codex, SessionKind::Structured),
    ] {
        let (mut w, s) = window_with(session_of(cli, kind), None);
        assert!(
            !w.offers_effort(s),
            "{cli:?} {kind:?} offers no effort (FR-012)"
        );
        w.open_effort_picker(s);
        assert!(w.effort_picker.picker(s).is_none(), "nor opens a picker");
    }
}

// ---- the rows (FR-002, US1 scenario 2) ----

#[test]
fn with_nothing_reported_the_picker_offers_default_and_the_five_levels() {
    for models in [None, Some(default_model(None))] {
        let (mut w, s) = window(models);
        w.open_effort_picker(s);
        assert_eq!(choices(&w, s), all_levels(), "least to most, after Default");
        assert_eq!(current(&w, s), [EffortChoice::Default], "Default marked");
        assert_eq!(highlighted(&w, s), Some(EffortChoice::Default));
    }
}

#[test]
fn the_reported_levels_are_offered_in_the_clis_order_and_the_choice_marked() {
    let (mut w, s) = window(Some(default_model(Some(support(true, &["low", "high"])))));
    set_effort(&mut w, s, chosen("high"));
    w.open_effort_picker(s);
    assert_eq!(
        choices(&w, s),
        [EffortChoice::Default, chosen("low"), chosen("high")]
    );
    assert_eq!(current(&w, s), [chosen("high")], "one row marked");
    assert_eq!(
        highlighted(&w, s),
        Some(chosen("high")),
        "the keyboard starts on it"
    );
    let marked = rows(&w, s).into_iter().find(|r| r.current).expect("marked");
    assert_eq!(marked.label(), "high");
    assert_eq!(marked.mark(), Some(CURRENT), "the mark is in words");
    assert_eq!(rows(&w, s)[0].label(), "Default");
    assert_eq!(rows(&w, s)[0].mark(), None);
}

#[test]
fn a_model_without_effort_still_offers_the_five_levels() {
    let (mut w, s) = window(Some(default_model(Some(support(false, &[])))));
    w.open_effort_picker(s);
    assert_eq!(
        choices(&w, s),
        all_levels(),
        "the choice can still be changed (FR-003)"
    );
}

#[test]
fn a_stored_level_the_model_does_not_list_is_still_a_row_marked_current() {
    let (mut w, s) = window(Some(default_model(Some(support(true, &["low", "high"])))));
    set_effort(&mut w, s, chosen("max"));
    w.open_effort_picker(s);
    assert_eq!(
        choices(&w, s),
        [
            EffortChoice::Default,
            chosen("low"),
            chosen("high"),
            chosen("max")
        ],
        "the stored level after the offered ones (FR-002)"
    );
    assert_eq!(current(&w, s), [chosen("max")], "marked current");
    assert_eq!(
        highlighted(&w, s),
        Some(chosen("max")),
        "Enter keeps the choice, not Default"
    );
    assert_eq!(w.effort_picked(s, None, false), None, "nothing to ask");
}

// ---- open, close, move and pick ----

#[test]
fn the_trigger_toggles_and_a_dismissal_closes() {
    let (mut w, s) = window(None);
    w.effort_picker_toggled(s);
    assert!(w.effort_picker.picker(s).is_some(), "opened");
    w.effort_picker_toggled(s);
    assert!(w.effort_picker.picker(s).is_none(), "closed again");
    w.open_effort_picker(s);
    w.effort_picker.close(s);
    assert!(w.effort_picker.picker(s).is_none(), "dismissed");
}

#[test]
fn the_arrows_move_the_highlight_and_enter_picks_it() {
    let (mut w, s) = window(None);
    w.open_effort_picker(s);
    w.effort_picker_moved(s, Direction::Next);
    w.effort_picker_moved(s, Direction::Next);
    w.effort_picker_moved(s, Direction::Next);
    assert_eq!(highlighted(&w, s), Some(chosen("high")));
    w.effort_picker_moved(s, Direction::Prev);
    assert_eq!(highlighted(&w, s), Some(chosen("medium")));
    assert_eq!(
        w.effort_picked(s, None, false),
        Some(chosen("medium")),
        "Enter"
    );
    assert!(w.effort_picker.picker(s).is_none(), "a pick closes it");
}

#[test]
fn a_pressed_row_is_the_choice_and_default_is_one() {
    let (mut w, s) = window(None);
    set_effort(&mut w, s, chosen("high"));
    w.open_effort_picker(s);
    assert_eq!(
        w.effort_picked(s, Some(0), false),
        Some(EffortChoice::Default),
        "back to Claude Code's default (US1 scenario 6)"
    );
    w.open_effort_picker(s);
    assert_eq!(w.effort_picked(s, Some(5), false), Some(chosen("max")));
    assert_eq!(w.effort_picked(s, Some(1), false), None, "nothing open");
}

#[test]
fn picking_the_current_choice_closes_and_asks_nothing() {
    let (mut w, s) = window(None);
    set_effort(&mut w, s, chosen("high"));
    w.open_effort_picker(s);
    assert_eq!(w.effort_picked(s, None, false), None, "already the choice");
    assert!(w.effort_picker.picker(s).is_none());
}

#[test]
fn picking_the_reported_choice_while_a_pick_is_in_flight_is_sent() {
    // `high` picked, then Default before the catalog echoes `high`: the catalog still says
    // Default, which is stale (review A M4 F3).
    let (mut w, s) = window(None);
    w.open_effort_picker(s);
    assert_eq!(
        w.effort_picked(s, Some(0), true),
        Some(EffortChoice::Default),
        "the latest pick wins over one still unanswered"
    );
}

#[test]
fn the_picker_is_per_session() {
    let (mut w, s) = window(None);
    let other = SessionId::new();
    w.open_effort_picker(s);
    assert!(w.effort_picker.picker(other).is_none());
    w.effort_picker.forget(s);
    assert!(
        w.effort_picker.picker(s).is_none(),
        "forgotten with its session"
    );
}

// ---- the availability note (FR-003) ----

#[test]
fn the_note_says_when_the_model_does_not_take_the_choice() {
    assert_eq!(
        availability_note(&chosen("high"), None),
        None,
        "unknown: sent as is"
    );
    assert_eq!(
        availability_note(&EffortChoice::Default, Some(&support(false, &[]))).as_deref(),
        Some(NO_EFFORT_NOTE),
        "said whatever the choice, Default too (FR-003)"
    );
    assert_eq!(
        availability_note(&EffortChoice::Default, Some(&support(true, &["low"]))),
        None,
        "Default applies on a model that takes effort"
    );
    assert_eq!(
        availability_note(&chosen("high"), Some(&support(false, &[]))).as_deref(),
        Some("This model does not support reasoning effort; Claude Code's default applies"),
        "a level on a model without effort"
    );
    assert_eq!(
        NO_EFFORT_NOTE,
        "This model does not support reasoning effort; Claude Code's default applies",
        "the note's words"
    );
    assert_eq!(
        availability_note(&chosen("high"), Some(&support(true, &["low", "medium"]))).as_deref(),
        Some("high is not available for this model; Claude Code's default applies"),
        "a level the model does not list"
    );
    assert_eq!(
        availability_note(&chosen("low"), Some(&support(true, &["low", "medium"]))),
        None,
        "a level the model lists"
    );
}

#[test]
fn a_model_without_effort_is_said_at_default_too() {
    let (w, s) = window(Some(default_model(Some(support(false, &[])))));
    assert_eq!(w.effort_choice(s), EffortChoice::Default);
    assert_eq!(w.effort_note(s).as_deref(), Some(NO_EFFORT_NOTE));
}

#[test]
fn an_open_picker_stops_taking_the_keyboard_once_the_session_is_a_terminal() {
    let (mut w, s) = window(None);
    w.session.active = Some(s);
    w.open_effort_picker(s);
    assert!(!w.terminal_focused(), "the open list has the keys");
    let path = w.workspace.active.clone().expect("active");
    w.workspace
        .sessions
        .get_mut(&path)
        .and_then(|l| l.iter_mut().find(|x| x.id == s))
        .expect("held")
        .kind = SessionKind::Terminal;
    assert!(
        w.terminal_focused(),
        "no list is drawn for a terminal session, so the terminal has the keys (review A M4 F1)"
    );
}

#[test]
fn the_window_notes_its_session_models_support() {
    let (mut w, s) = window(Some(default_model(Some(support(true, &["low"])))));
    assert_eq!(w.effort_note(s), None, "Default");
    set_effort(&mut w, s, chosen("max"));
    assert_eq!(
        w.effort_note(s).as_deref(),
        Some("max is not available for this model; Claude Code's default applies")
    );
}
