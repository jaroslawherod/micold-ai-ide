//! T033 (feature 939, research R9, FR-016, US3 scenarios 4–5): following is held by iced's own
//! scrollable state. After `snap_to(END)` the offset is relative, resolved against the new content
//! height at each layout, so growth keeps the end in view with no task per event; after a scroll to
//! an absolute offset, growth below leaves the visible content where it was.
//!
//! Pinned headless on the real view, one widget tree kept across frames as the runtime keeps it.

mod support;

use iced::advanced::widget::operation::scrollable::{scroll_to, snap_to};
use iced::widget::scrollable::{AbsoluteOffset, RelativeOffset};
use iced::Size;
use micold_client::features::structured_transcript::State;
use micold_client::ui::transcript::{transcript_scroll_id, BlockCache, BlockRenderers};
use micold_core::session::SessionId;
use micold_core::structured::{SequencedEvent, StructuredEvent};
use support::transcript::{view_of, Mounted};

const ROOM: Size = Size::new(400.0, 300.0);

/// `n` raw rows from `first` on: content that grows the transcript by whole entries.
fn rows(first: u64, n: u64) -> Vec<SequencedEvent> {
    (first..first + n)
        .map(|seq| SequencedEvent {
            seq,
            event: StructuredEvent::Unreadable {
                text: format!("line {seq} of a transcript taller than its pane"),
            },
        })
        .collect()
}

fn grown(session: SessionId) -> State {
    let mut state = State::default();
    state.apply(session, 0, &rows(0, 30), true);
    state
}

#[test]
fn after_snap_to_end_growth_keeps_the_end_in_view() {
    let session = SessionId::new();
    let mut state = grown(session);
    let cache = BlockCache::default();
    let mut mounted = Mounted::new(ROOM);

    let (_, before) = mounted.frame(
        view_of(&state, session, &BlockRenderers::PLAIN, &cache),
        vec![Box::new(snap_to::<()>(
            transcript_scroll_id(session),
            RelativeOffset::END.into(),
        ))],
    );
    assert!(
        before.content > before.viewport,
        "{before:?}: taller than its pane"
    );
    assert!(before.at_end(), "{before:?}");

    for step in 0..3 {
        state.apply(session, 0, &rows(30 + step * 5, 5), true);
        let (_, after) = mounted.frame(
            view_of(&state, session, &BlockRenderers::PLAIN, &cache),
            vec![],
        );
        assert!(
            after.content > before.content,
            "{after:?}: the content grew"
        );
        assert!(
            after.at_end(),
            "step {step}: {after:?} keeps the end in view"
        );
    }
}

#[test]
fn after_a_scroll_to_an_offset_growth_below_moves_nothing() {
    let session = SessionId::new();
    let mut state = grown(session);
    let cache = BlockCache::default();
    let mut mounted = Mounted::new(ROOM);

    let id = transcript_scroll_id(session);
    let (_, before) = mounted.frame(
        view_of(&state, session, &BlockRenderers::PLAIN, &cache),
        vec![
            Box::new(snap_to::<()>(id.clone(), RelativeOffset::END.into())),
            Box::new(scroll_to::<()>(
                id,
                AbsoluteOffset { x: 0.0, y: 50.0 }.into(),
            )),
        ],
    );
    assert_eq!(before.offset, 50.0, "{before:?}");

    state.apply(session, 0, &rows(30, 10), true);
    let (_, after) = mounted.frame(
        view_of(&state, session, &BlockRenderers::PLAIN, &cache),
        vec![],
    );
    assert!(
        after.content > before.content,
        "{after:?}: the content grew"
    );
    assert_eq!(
        after.offset, 50.0,
        "{after:?}: the visible content stayed put"
    );
    assert!(!after.at_end());
}
