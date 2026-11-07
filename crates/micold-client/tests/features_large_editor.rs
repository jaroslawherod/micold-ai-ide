//! The composer's large editor, in isolation (feature 1703, US1): the reducer over the window
//! state and the key map of its dialog.

use micold_client::app::{EditorAction, State};
use micold_client::features::large_editor::{editor_key, update, Msg};
use micold_client::keymap::{Key, KeyInput, Mods, NamedKey, PhysicalKey};
use micold_core::session::SessionId;
use micold_core::structured::attachment::admit;

fn press(named: NamedKey, mods: Mods) -> KeyInput {
    KeyInput {
        key: Key::Named(named),
        mods,
        text: None,
        physical: PhysicalKey::Other,
    }
}

fn with_draft(text: &str) -> (State, SessionId) {
    let mut state = State::default();
    let session = SessionId::new();
    let end = (
        text.matches('\n').count(),
        text.rsplit('\n').next().unwrap().len(),
    );
    state.composers.edited(session, text.to_owned(), end);
    (state, session)
}

fn typed(text: &str) -> Msg {
    Msg::Edited(EditorAction(iced::widget::text_editor::Action::Edit(
        iced::widget::text_editor::Edit::Paste(text.to_owned().into()),
    )))
}

fn draft(state: &State, session: SessionId) -> String {
    state
        .composers
        .draft(session)
        .map(|d| d.text.clone())
        .unwrap_or_default()
}

fn open_text(state: &State) -> String {
    state.large_editor.open.as_ref().unwrap().content.text()
}

#[test]
fn requested_opens_with_the_draft_and_the_cursor_at_its_end() {
    let (mut state, session) = with_draft("one\ntwo");
    update(&mut state, Msg::Requested(session));
    let open = state.large_editor.open.as_ref().expect("open");
    assert_eq!(open.session, session);
    assert_eq!(open.content.text(), "one\ntwo");
    assert_eq!(open.content.caret(), (1, 3));
}

#[test]
fn an_empty_draft_opens_an_empty_editor() {
    let mut state = State::default();
    let session = SessionId::new();
    update(&mut state, Msg::Requested(session));
    assert_eq!(open_text(&state), "");
}

#[test]
fn a_second_request_while_open_is_ignored() {
    let (mut state, a) = with_draft("a");
    let (_, b) = with_draft("b");
    update(&mut state, Msg::Requested(a));
    update(&mut state, typed(" more"));
    update(&mut state, Msg::Requested(b));
    let open = state.large_editor.open.as_ref().unwrap();
    assert_eq!(open.session, a);
    assert_eq!(open.content.text(), "a more");
}

#[test]
fn done_writes_the_text_to_the_bound_session_only() {
    let (mut state, a) = with_draft("a");
    let b = SessionId::new();
    state.composers.edited(b, "b".to_owned(), (0, 1));
    update(&mut state, Msg::Requested(a));
    update(&mut state, typed("!"));
    update(&mut state, Msg::Done);
    assert!(state.large_editor.open.is_none());
    assert_eq!(draft(&state, a), "a!");
    assert_eq!(draft(&state, b), "b");
}

#[test]
fn done_writes_to_a_session_that_ended_meanwhile() {
    let (mut state, a) = with_draft("a");
    update(&mut state, Msg::Requested(a));
    update(&mut state, typed("b"));
    // The session ended: its composer is forgotten, as closing or removing it does.
    state.forget_structured(a);
    assert!(state.composers.draft(a).is_none(), "the composer is gone");
    update(&mut state, Msg::Done);
    assert!(state.large_editor.open.is_none());
    assert_eq!(
        draft(&state, a),
        "ab",
        "Done still writes the text, without a panic"
    );
}

#[test]
fn cancelled_leaves_the_draft_unchanged() {
    let (mut state, a) = with_draft("keep");
    update(&mut state, Msg::Requested(a));
    update(&mut state, typed(" lost"));
    update(&mut state, Msg::Cancelled);
    assert!(state.large_editor.open.is_none());
    assert_eq!(draft(&state, a), "keep");
}

/// A draft holding `text` and two pictures.
fn with_two_images(text: &str) -> (State, SessionId, Vec<String>) {
    let (mut state, session) = with_draft("");
    let images = ["one.png", "two.png"]
        .map(|name| admit(&[], name, b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec()).expect("a PNG"))
        .to_vec();
    state.composers.taken(session, text.to_owned(), images);
    let names = image_names(&state, session);
    assert_eq!(names.len(), 2);
    (state, session, names)
}

fn image_names(state: &State, session: SessionId) -> Vec<String> {
    state
        .composers
        .draft(session)
        .unwrap()
        .images
        .iter()
        .map(|i| format!("{i:?}"))
        .collect()
}

#[test]
fn attachments_are_never_read_or_written() {
    for finish in [Msg::Done, Msg::Cancelled] {
        let (mut state, a, before) = with_two_images("t");
        update(&mut state, Msg::Requested(a));
        update(&mut state, typed("x"));
        update(&mut state, finish.clone());
        assert_eq!(
            image_names(&state, a),
            before,
            "the same two pictures after {finish:?}"
        );
    }
}

#[test]
fn hover_follows_the_pointer_per_session() {
    let mut state = State::default();
    let (a, b) = (SessionId::new(), SessionId::new());
    update(&mut state, Msg::Hovered(a, true));
    update(&mut state, Msg::Hovered(b, false));
    assert_eq!(state.large_editor.hovered, Some(a));
    update(&mut state, Msg::Hovered(a, false));
    assert_eq!(state.large_editor.hovered, None);
}

#[test]
fn ctrl_or_cmd_enter_is_done_and_escape_is_cancel() {
    let ctrl = Mods {
        ctrl: true,
        ..Mods::NONE
    };
    let cmd = Mods {
        logo: true,
        ..Mods::NONE
    };
    assert_eq!(editor_key(&press(NamedKey::Enter, ctrl)), Some(Msg::Done));
    assert_eq!(editor_key(&press(NamedKey::Enter, cmd)), Some(Msg::Done));
    assert_eq!(
        editor_key(&press(NamedKey::Escape, Mods::NONE)),
        Some(Msg::Cancelled)
    );
    assert_eq!(editor_key(&press(NamedKey::Enter, Mods::NONE)), None);
    let shift = Mods {
        shift: true,
        ..Mods::NONE
    };
    assert_eq!(editor_key(&press(NamedKey::Enter, shift)), None);
}
