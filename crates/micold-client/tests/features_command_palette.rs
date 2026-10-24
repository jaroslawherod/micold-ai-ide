//! #943 T050, T051 — the command palette and the model picker, render-free (FR-007–FR-010,
//! FR-013, FR-014, FR-018; US2 scenarios 1–8, US3 scenarios 1, 2, 4, 5).
//!
//! Drives the window's state as the shell does: the composer's editor reports the draft
//! (`composers.edited` then `composer_edited`), keys go through `palette_key` before
//! `composer_key`, and a send asks `send_intent` what the draft means.

mod support;

use micold_client::app::State as Window;
use micold_client::features::command_palette::{
    group_title, model_header, palette_key, row_details, row_match, row_tag, use_typed, PaletteKey,
    PaletteLine, SendIntent, DEFAULT_MODEL, FALLBACK_NOTE, NO_MATCH, OPENS_IN_TERMINAL,
};
use micold_client::features::composer::{composer_key, KeyIntent};
use micold_client::keymap::{Key, KeyInput, Mods, NamedKey, PhysicalKey};
use micold_core::protocol::messages::TurnPhase;
use micold_core::session::{AiCli, Session, SessionId, SessionKind, SessionLocation};
use micold_core::structured::commands::CommandKind;
use micold_core::structured::model::{ModelChoice, ModelName, MODEL_SHORTCUTS};
use micold_core::structured::{
    parse_line, ReportedCommand, ReportedModel, SequencedEvent, StructuredEvent,
};
use micold_core::typeahead::Direction;

fn press(named: NamedKey) -> KeyInput {
    KeyInput {
        key: Key::Named(named),
        mods: Mods::NONE,
        text: None,
        physical: PhysicalKey::Other,
    }
}

fn command(name: &str, builtin: bool) -> ReportedCommand {
    ReportedCommand {
        name: name.into(),
        description: Some(format!("What {name} does")),
        argument_hint: None,
        aliases: vec![],
        builtin,
    }
}

/// The history of a started session: an init naming two skills, then the `initialize` answer.
fn started(models: Vec<ReportedModel>) -> Vec<SequencedEvent> {
    let init = parse_line(
        r#"{"type":"system","subtype":"init","session_id":"s","model":"claude-sonnet-4-5","slash_commands":["speckit-plan","speckit-specify","review","compact","clear","model"],"skills":["speckit-plan","speckit-specify"]}"#,
    );
    let reported = StructuredEvent::CommandsReported {
        commands: vec![
            command("speckit-plan", false),
            command("speckit-specify", false),
            command("review", false),
            command("compact", true),
            command("clear", true),
            command("model", true),
        ],
        models,
    };
    [init, reported]
        .into_iter()
        .zip(0..)
        .map(|(event, seq)| SequencedEvent { seq, event })
        .collect()
}

fn structured_session() -> Session {
    Session::start_new_of_kind(
        SessionLocation::Worktree("feat-00".into()),
        AiCli::ClaudeCode,
        SessionKind::Structured,
    )
}

/// A window with one structured session; `history` (if any) is its transcript.
fn window(history: Option<Vec<SequencedEvent>>) -> (Window, SessionId) {
    let session = structured_session();
    let id = session.id;
    let mut workspace = support::workspace_with(vec![("/fixture/project", vec![session])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut window = Window {
        workspace,
        ..Window::default()
    };
    if let Some(events) = history {
        window.structured_transcript.apply(id, 0, &events, true);
    }
    (window, id)
}

/// The user typed `text` with the caret at its end (one line).
fn typed(window: &mut Window, session: SessionId, text: &str) {
    window
        .composers
        .edited(session, text.to_string(), (0, text.len()));
    window.composer_edited(session);
}

fn rows(window: &Window, session: SessionId) -> Vec<String> {
    window
        .composers
        .palette
        .palette(session)
        .expect("the palette is open")
        .rows()
        .map(|c| c.name.clone())
        .collect()
}

fn highlighted(window: &Window, session: SessionId) -> Option<String> {
    let palette = window.composers.palette.palette(session)?;
    let at = palette.highlight()?;
    palette.rows().nth(at).map(|c| c.name.clone())
}

fn draft(window: &Window, session: SessionId) -> (String, (usize, usize)) {
    window
        .composers
        .draft(session)
        .map(|d| (d.text.clone(), d.caret))
        .unwrap_or_default()
}

// ---- T050: the palette (US2) ----

#[test]
fn a_slash_in_an_empty_draft_opens_the_palette_grouped_by_kind() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/");
    let palette = w.composers.palette.palette(s).expect("open");
    let kinds: Vec<CommandKind> = palette.rows().map(|c| c.kind).collect();
    let mut sorted = kinds.clone();
    sorted.sort();
    assert_eq!(kinds, sorted, "grouped skills, custom, built-in, app");
    let names = rows(&w, s);
    assert_eq!(
        &names[..3],
        ["speckit-plan", "speckit-specify", "review"],
        "skills first, then custom commands"
    );
    assert!(names.contains(&"model".to_string()), "the app's /model");
    assert!(names.contains(&"mcp".to_string()), "terminal-only commands");
    assert_eq!(palette.source_note(), None, "the session's own list");
    assert_eq!(highlighted(&w, s).as_deref(), Some("speckit-plan"));
}

#[test]
fn typing_narrows_and_selects_the_first_match_and_the_arrows_move_it() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/sp");
    assert_eq!(rows(&w, s), ["speckit-plan", "speckit-specify"]);
    assert_eq!(highlighted(&w, s).as_deref(), Some("speckit-plan"));

    let down = palette_key(w.composers.palette.palette(s), &press(NamedKey::ArrowDown));
    assert_eq!(down, Some(PaletteKey::Down));
    w.palette_key(s, PaletteKey::Down);
    assert_eq!(highlighted(&w, s).as_deref(), Some("speckit-specify"));
    w.palette_key(s, PaletteKey::Down);
    assert_eq!(
        highlighted(&w, s).as_deref(),
        Some("speckit-specify"),
        "no wrap past the last"
    );
    w.palette_key(s, PaletteKey::Up);
    assert_eq!(highlighted(&w, s).as_deref(), Some("speckit-plan"));
}

#[test]
fn tab_enter_and_a_click_fill_the_command_and_close_without_sending() {
    for how in ["tab", "enter", "click"] {
        let (mut w, s) = window(Some(started(vec![])));
        typed(&mut w, s, "/rev");
        match how {
            "click" => w.palette_clicked(s, 0),
            key => {
                let named = if key == "tab" {
                    NamedKey::Tab
                } else {
                    NamedKey::Enter
                };
                let k = palette_key(w.composers.palette.palette(s), &press(named))
                    .expect("the palette takes it");
                assert_eq!(k, PaletteKey::Accept);
                w.palette_key(s, k);
            }
        }
        assert_eq!(draft(&w, s), ("/review ".to_string(), (0, 8)), "{how}");
        assert!(w.composers.palette.palette(s).is_none(), "{how}: closed");
        assert!(
            w.composers.composer(s).is_some_and(|c| c.pending.is_none()),
            "{how}: nothing sent"
        );
    }
}

#[test]
fn escape_closes_the_palette_keeps_the_draft_and_does_not_interrupt() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/sp");
    let k = palette_key(w.composers.palette.palette(s), &press(NamedKey::Escape));
    assert_eq!(
        k,
        Some(PaletteKey::Close),
        "the palette's, not composer_key's Interrupt"
    );
    w.palette_key(s, PaletteKey::Close);
    assert!(w.composers.palette.palette(s).is_none());
    assert_eq!(draft(&w, s).0, "/sp");
    w.composer_edited(s);
    assert!(
        w.composers.palette.palette(s).is_none(),
        "the same draft does not reopen it"
    );
    typed(&mut w, s, "/spe");
    assert!(
        w.composers.palette.palette(s).is_some(),
        "typing reopens it"
    );
}

#[test]
fn deleting_the_slash_or_leaving_the_name_closes_the_palette() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/sp");
    typed(&mut w, s, "");
    assert!(w.composers.palette.palette(s).is_none(), "the / deleted");

    typed(&mut w, s, "/sp");
    typed(&mut w, s, "/sp ");
    assert!(
        w.composers.palette.palette(s).is_none(),
        "caret past the name"
    );

    typed(&mut w, s, "/sp");
    w.composers.edited(s, "/sp".into(), (0, 0));
    w.composer_edited(s);
    assert!(
        w.composers.palette.palette(s).is_none(),
        "caret before the /"
    );

    for text in ["x/sp", "/ sp"] {
        typed(&mut w, s, text);
        assert!(w.composers.palette.palette(s).is_none(), "{text}");
    }
}

#[test]
fn palette_key_is_none_while_closed_so_composer_key_runs() {
    let enter = press(NamedKey::Enter);
    let escape = press(NamedKey::Escape);
    assert_eq!(palette_key(None, &enter), None);
    assert_eq!(palette_key(None, &escape), None);
    assert_eq!(
        composer_key(&enter, (0, 0), 1, false, TurnPhase::Idle),
        KeyIntent::Send
    );
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/sp");
    assert_eq!(
        palette_key(w.composers.palette.palette(s), &press(NamedKey::ArrowLeft)),
        None,
        "keys the palette does not use stay the editor's"
    );
}

#[test]
fn no_match_says_so_and_enter_sends_as_typed() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/zzz");
    let palette = w.composers.palette.palette(s).expect("open");
    assert_eq!(palette.rows().count(), 0);
    assert_eq!(palette.no_match_note(), Some(NO_MATCH));
    assert_eq!(
        palette_key(Some(palette), &press(NamedKey::Enter)),
        None,
        "Enter is the composer's send"
    );
    assert_eq!(w.send_intent(s), SendIntent::Message);
}

#[test]
fn a_terminal_only_row_carries_its_words() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/mc");
    let palette = w.composers.palette.palette(s).expect("open");
    let mcp = palette.rows().next().expect("mcp listed");
    assert_eq!(mcp.name, "mcp");
    assert_eq!(row_tag(mcp), Some(OPENS_IN_TERMINAL));
    let review = w.command_list(s);
    let review = review
        .commands()
        .iter()
        .find(|c| c.name == "review")
        .expect("listed");
    assert_eq!(row_tag(review), None);
}

#[test]
fn a_session_with_no_reported_list_offers_the_fallback_with_its_note() {
    let (mut w, s) = window(None);
    typed(&mut w, s, "/");
    let palette = w.composers.palette.palette(s).expect("open");
    assert_eq!(palette.source_note(), Some(FALLBACK_NOTE));
    let names = rows(&w, s);
    for name in ["compact", "clear", "model", "mcp"] {
        assert!(names.contains(&name.to_string()), "{name}");
    }
}

// ---- T051: the model picker (US3) ----

fn reported_models() -> Vec<ReportedModel> {
    ["default", "opus", "haiku"]
        .into_iter()
        .map(|value| ReportedModel {
            value: value.into(),
            display_name: Some(value.to_uppercase()),
            description: None,
        })
        .collect()
}

fn named(name: &str) -> ModelChoice {
    ModelChoice::Named(ModelName::parse(name).expect("valid"))
}

fn set_choice(w: &mut Window, s: SessionId, choice: ModelChoice) {
    let path = w.workspace.active.clone().expect("active");
    let session = w
        .workspace
        .sessions
        .get_mut(&path)
        .and_then(|l| l.iter_mut().find(|x| x.id == s))
        .expect("held");
    session.model = choice;
}

fn picker_items(w: &Window, s: SessionId) -> Vec<ModelChoice> {
    w.composers
        .palette
        .picker(s)
        .expect("the picker is open")
        .items()
        .iter()
        .map(|i| i.choice.clone())
        .collect()
}

#[test]
fn the_picker_lists_default_and_the_reported_models_and_marks_the_current() {
    let (mut w, s) = window(Some(started(reported_models())));
    set_choice(&mut w, s, named("opus"));
    w.open_model_picker(s);
    assert_eq!(
        picker_items(&w, s),
        [ModelChoice::Default, named("opus"), named("haiku")],
        "Default once, then the CLI's models"
    );
    let picker = w.composers.palette.picker(s).expect("open");
    let marked: Vec<bool> = picker
        .items()
        .iter()
        .map(|i| picker.is_current(i))
        .collect();
    assert_eq!(marked, [false, true, false]);
}

#[test]
fn without_reported_models_the_picker_offers_the_shortcuts_and_a_typed_name() {
    let (mut w, s) = window(Some(started(vec![])));
    w.open_model_picker(s);
    let mut expected = vec![ModelChoice::Default];
    expected.extend(MODEL_SHORTCUTS.iter().map(|m| named(m)));
    assert_eq!(picker_items(&w, s), expected);

    w.model_picker_typed(s, "claude-opus-4-1".into());
    let chosen = w.model_picked(s, None);
    assert_eq!(chosen, Some(named("claude-opus-4-1")), "the typed name");
    assert!(w.composers.palette.picker(s).is_none(), "closed");

    w.open_model_picker(s);
    w.model_picker_typed(s, "two words".into());
    assert_eq!(w.model_picked(s, None), None, "not a model name");
    assert!(
        w.composers
            .composer(s)
            .and_then(|c| c.notice.as_deref())
            .is_some_and(|n| n.contains("model name")),
        "the refusal is said"
    );
}

#[test]
fn picking_an_item_answers_its_choice() {
    let (mut w, s) = window(Some(started(reported_models())));
    w.open_model_picker(s);
    assert_eq!(w.model_picked(s, Some(2)), Some(named("haiku")));
    w.open_model_picker(s);
    assert_eq!(w.model_picked(s, Some(0)), Some(ModelChoice::Default));
}

#[test]
fn sending_slash_model_opens_the_picker_and_sends_nothing() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/model");
    assert_eq!(w.send_intent(s), SendIntent::Nothing);
    assert!(w.composers.palette.picker(s).is_some(), "the picker opened");
    assert_eq!(draft(&w, s).0, "", "the command is used up");

    typed(&mut w, s, "/model haiku");
    assert_eq!(w.send_intent(s), SendIntent::SetModel(named("haiku")));
    assert_eq!(
        draft(&w, s).0,
        "/model haiku",
        "kept until the request went out (the shell clears it)"
    );

    typed(&mut w, s, "/model default");
    assert_eq!(w.send_intent(s), SendIntent::SetModel(ModelChoice::Default));

    typed(&mut w, s, "plain words");
    assert_eq!(w.send_intent(s), SendIntent::Message);
}

#[test]
fn the_header_says_the_reported_model_and_whose_choice_it_is() {
    assert_eq!(
        model_header(Some("claude-sonnet-4-5"), &ModelChoice::Default),
        "claude-sonnet-4-5 · Default"
    );
    assert_eq!(
        model_header(Some("claude-sonnet-4-5"), &named("opus")),
        "claude-sonnet-4-5 · your choice: opus"
    );
    assert_eq!(
        model_header(None, &ModelChoice::Default),
        "Model not reported yet · Default"
    );

    let (mut w, s) = window(Some(started(vec![])));
    assert_eq!(w.model_header(s), "claude-sonnet-4-5 · Default");
    // A pick on a busy session: the choice is persisted at once, the CLI's model changes at the
    // turn's end (US3 scenario 4).
    set_choice(&mut w, s, named("haiku"));
    assert_eq!(w.model_header(s), "claude-sonnet-4-5 · your choice: haiku");
    w.structured_transcript.apply(
        s,
        0,
        &[SequencedEvent {
            seq: 2,
            event: StructuredEvent::ModelChanged {
                model: "claude-haiku-4-5".into(),
            },
        }],
        true,
    );
    assert_eq!(w.model_header(s), "claude-haiku-4-5 · your choice: haiku");
}

// ---- T056, T057: what the window draws (M8) ----

fn lines(w: &Window, s: SessionId) -> Vec<PaletteLine> {
    w.composers.palette.palette(s).expect("open").lines()
}

#[test]
fn an_empty_name_lists_each_group_under_its_header() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/");
    let headers: Vec<&str> = lines(&w, s)
        .iter()
        .filter_map(|l| match l {
            PaletteLine::Group(title) => Some(*title),
            PaletteLine::Command { .. } => None,
        })
        .collect();
    assert_eq!(
        headers,
        [
            group_title(CommandKind::Skill),
            group_title(CommandKind::Custom),
            group_title(CommandKind::BuiltIn),
            group_title(CommandKind::App),
        ],
        "one header per group, in the palette's order"
    );
    assert_eq!(group_title(CommandKind::Skill), "Skills");
    assert_eq!(group_title(CommandKind::Custom), "Custom commands");
    assert_eq!(group_title(CommandKind::BuiltIn), "Built-in commands");
    assert_eq!(group_title(CommandKind::App), "In this app");
    let first = lines(&w, s).into_iter().nth(1).expect("a command");
    assert_eq!(
        first,
        PaletteLine::Command {
            label: "/speckit-plan".into(),
            matched: None,
            details: Some("What speckit-plan does".into()),
        }
    );
}

#[test]
fn a_typed_name_lists_its_matches_without_headers_and_marks_the_match() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/SPEC");
    let lines = lines(&w, s);
    assert!(lines
        .iter()
        .all(|l| matches!(l, PaletteLine::Command { .. })));
    let PaletteLine::Command { label, matched, .. } = &lines[0] else {
        unreachable!()
    };
    assert_eq!(label, "/speckit-plan");
    assert_eq!(matched.clone(), Some(1..5), "the typed name, past the `/`");
    assert_eq!(row_match("my-review", "rev"), Some(3..6));
    assert_eq!(
        row_match("respec-spec", "spec"),
        Some(7..11),
        "the word's start, as core's filter ranked it"
    );
    assert_eq!(row_match("my-review", "view"), Some(5..9), "mid-word");
    assert_eq!(row_match("review", "x"), None);
}

#[test]
fn a_terminal_only_row_says_so_before_its_description() {
    let (mut w, s) = window(Some(started(vec![])));
    typed(&mut w, s, "/mcp");
    let mcp = w
        .composers
        .palette
        .palette(s)
        .and_then(|p| p.rows().next().cloned())
        .expect("mcp listed");
    let details = row_details(&mcp).expect("details");
    assert!(
        details.starts_with(OPENS_IN_TERMINAL),
        "the words come first, so a cut description never takes them: {details}"
    );
    let list = w.command_list(s);
    let review = list
        .commands()
        .iter()
        .find(|c| c.name == "review")
        .expect("listed");
    assert_eq!(row_details(review).as_deref(), Some("What review does"));
}

#[test]
fn the_picker_rows_end_with_the_typed_name_and_enter_takes_the_highlight_or_it() {
    let (mut w, s) = window(Some(started(reported_models())));
    set_choice(&mut w, s, named("haiku"));
    w.open_model_picker(s);
    let picker = w.composers.palette.picker(s).expect("open");
    let labels: Vec<String> = picker.rows().into_iter().map(|r| r.label).collect();
    assert_eq!(labels, [DEFAULT_MODEL, "OPUS", "HAIKU"]);
    assert_eq!(
        picker.current_row(),
        Some(2),
        "the current choice is marked"
    );
    assert_eq!(
        picker.enter_row(),
        None,
        "nothing highlighted, nothing typed"
    );

    w.model_picker_typed(s, "claude-opus-4-1".into());
    let picker = w.composers.palette.picker(s).expect("open");
    let rows = picker.rows();
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[3].label, use_typed("claude-opus-4-1"));
    assert_eq!(rows[3].item, None, "the typed name is no item");
    assert_eq!(rows[1].item, Some(1));
    assert_eq!(picker.enter_row(), Some(3), "Enter takes the typed name");
    assert_eq!(use_typed("x"), "Use x");

    w.model_picker_moved(s, Direction::Next);
    let picker = w.composers.palette.picker(s).expect("open");
    assert_eq!(picker.enter_row(), Some(0), "the arrows highlight an item");
    for _ in 0..3 {
        w.model_picker_moved(s, Direction::Next);
    }
    let picker = w.composers.palette.picker(s).expect("open");
    assert_eq!(
        picker.enter_row(),
        Some(3),
        "the arrows reach the typed row too"
    );
    assert_eq!(w.model_picked(s, None), Some(named("claude-opus-4-1")));
}
