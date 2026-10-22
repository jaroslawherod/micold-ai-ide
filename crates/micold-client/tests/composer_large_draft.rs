//! A 10,000-line draft stays responsive (feature 941, T056, FR-008, SC-007).
//!
//! Each keystroke into a structured session's composer is what the shell does with it: the
//! editor performs the action, and the composer takes the text and caret it now shows. Typing,
//! a line break, a deletion and a paste, at the start, middle and end of a 10,000-line draft, each
//! finish under 100 ms.
//!
//! A timing test: run in a release build, on demand.
//! `cargo test -p micold-client --release --test composer_large_draft -- --ignored`

use std::sync::Arc;
use std::time::{Duration, Instant};

use iced::widget::text_editor::{Action, Edit, Motion};
use micold_client::app::{EditorAction, State};
use micold_core::session::SessionId;

/// The longest one keystroke may take (SC-007).
const BUDGET: Duration = Duration::from_millis(100);

/// The draft's length (FR-008).
const LINES: usize = 10_000;

/// One keystroke as the shell runs it; how long it took.
fn keystroke(state: &mut State, session: SessionId, action: Action) -> Duration {
    let started = Instant::now();
    let editor = state.composers.editor_mut(session);
    let action = EditorAction(action);
    if action.0.is_edit() {
        let text = editor.perform(&action);
        let caret = editor.caret();
        state.composers.edited(session, text, caret);
    } else {
        editor.act(&action);
    }
    started.elapsed()
}

#[test]
#[ignore = "timing: run in release with --ignored"]
fn every_keystroke_into_a_ten_thousand_line_draft_takes_under_100_ms() {
    let mut state = State::default();
    let session = SessionId::new();
    let draft: Vec<String> = (1..=LINES)
        .map(|n| format!("line {n}: fn example() {{ let value = {n}; }}"))
        .collect();
    // The draft arrives in one paste: one action, not typing, so it is not held to the budget.
    keystroke(
        &mut state,
        session,
        Action::Edit(Edit::Paste(Arc::new(draft.join("\n")))),
    );
    assert_eq!(state.composers.editor(session).line_count(), LINES);

    let mut slowest = Duration::ZERO;
    for place in [Motion::DocumentStart, Motion::PageDown, Motion::DocumentEnd] {
        keystroke(&mut state, session, Action::Move(place));
        for action in [
            Action::Edit(Edit::Insert('x')),
            Action::Edit(Edit::Insert('y')),
            Action::Edit(Edit::Enter),
            Action::Edit(Edit::Backspace),
            Action::Edit(Edit::Delete),
            Action::Edit(Edit::Paste(Arc::new("pasted\ntwo lines".to_owned()))),
            Action::Move(Motion::Up),
            Action::Move(Motion::Down),
        ] {
            slowest = slowest.max(keystroke(&mut state, session, action));
        }
    }
    assert!(
        slowest < BUDGET,
        "the slowest keystroke took {slowest:?}, over {BUDGET:?}"
    );
    let draft = state.composers.draft(session).expect("the draft is kept");
    assert!(
        draft.text.lines().count() > LINES,
        "the edits reached the draft"
    );
}
