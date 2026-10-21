//! T031 (feature 939, FR-022, contracts/block-renderers.md R-1, R-2): each block kind is drawn by
//! one replaceable renderer. Swapping `assistant_text` for a marker changes only what assistant text
//! blocks draw: the marker appears once per `AssistantText` entry, and every other entry, the order
//! and the fold state are as `BlockRenderers::PLAIN` draws them.

mod support;

use iced::Size;
use micold_client::ui::transcript::{AssistantText, Block, BlockCache, BlockRenderers, RenderCtx};
use micold_core::session::SessionId;
use micold_core::structured::transcript::EntryKind;
use support::layout::{painted_text_settled_at, renderer};
use support::transcript::{open_every_card, state_of, view_of};

const MARKER: &str = "«assistant text drawn by the marker»";

fn marker(_block: &AssistantText<'_>, _ctx: &RenderCtx<'_>) -> Block {
    iced::widget::text(MARKER).into()
}

/// Tall enough that the whole transcript is in view, so both passes paint every entry.
const ROOM: Size = Size::new(900.0, 40_000.0);

fn painted(renderers: &BlockRenderers) -> (Vec<String>, Vec<String>) {
    let session = SessionId::new();
    let mut state = state_of(session, "tool-turn");
    open_every_card(&mut state, session);
    let texts: Vec<String> = state
        .transcript(session)
        .unwrap()
        .entries()
        .filter_map(|e| match e.kind() {
            EntryKind::AssistantText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    let cache = BlockCache::default();
    let (painted, _) = painted_text_settled_at(
        view_of(&state, session, renderers, &cache),
        &mut renderer(),
        ROOM,
    );
    (painted.into_iter().map(|p| p.content).collect(), texts)
}

#[test]
fn swapping_one_renderer_changes_only_that_kinds_drawing() {
    let (plain, texts) = painted(&BlockRenderers::PLAIN);
    let (marked, _) = painted(&BlockRenderers {
        assistant_text: marker,
        ..BlockRenderers::PLAIN
    });
    assert!(
        !texts.is_empty(),
        "the tool-turn fixture holds assistant text"
    );

    assert_eq!(
        marked.iter().filter(|p| *p == MARKER).count(),
        texts.len(),
        "one marker per assistant text entry: {marked:#?}"
    );
    for text in &texts {
        assert!(plain.contains(text), "PLAIN draws {text:?}");
        assert!(!marked.contains(text), "the marker replaced {text:?}");
    }

    // Everything else — tool cards open with their input and result, footers, raw rows — is drawn
    // the same, in the same order.
    let others = |painted: &[String], drop: &dyn Fn(&String) -> bool| {
        painted
            .iter()
            .filter(|p| !drop(p))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(
        others(&marked, &|p| p == MARKER),
        others(&plain, &|p| texts.contains(p)),
    );
}

#[test]
fn a_folded_card_runs_no_block_renderer() {
    let session = SessionId::new();
    let state = state_of(session, "tool-turn");
    let cache = BlockCache::default();
    // Folded cards hold their input, result and thinking unseen: those renderers do not run.
    let renderers = BlockRenderers {
        tool_input: |_, _| unreachable!("a folded card draws no input"),
        tool_result: |_, _| unreachable!("a folded card draws no result"),
        thinking: |_, _| unreachable!("a folded card draws no thinking"),
        assistant_text: BlockRenderers::PLAIN.assistant_text,
    };
    let (painted, _) = painted_text_settled_at(
        view_of(&state, session, &renderers, &cache),
        &mut renderer(),
        ROOM,
    );
    assert!(!painted.is_empty());
}
