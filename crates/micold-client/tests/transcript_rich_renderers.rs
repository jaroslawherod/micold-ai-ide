//! T035 (feature 940, FR-002, FR-013, FR-018, FR-020–FR-022; contracts/rich-renderers.md
//! RR-1–RR-3): `BlockRenderers::RICH` draws block content only. Over a replayed markdown, edit and
//! tool turn and its render cache, the entries, their order, fold state, statuses and headers are
//! `PLAIN`'s but for the `+N −M` of Edit/MultiEdit/Write cards; assistant text and thinking are
//! drawn from their cached documents, edit inputs as their diffs, and tool results, streaming or
//! malformed inputs and cache misses as `PLAIN` draws them.

mod support;

use std::cell::Cell;

use iced::Size;
use micold_client::features::structured_transcript::State;
use micold_client::features::transcript_rich::DerivedBody;
use micold_client::ui::syntax::GRAMMAR;
use micold_client::ui::transcript::{
    edit_meta, tool_header_counted, AssistantText, Block, BlockCache, BlockRef, BlockRenderers,
    RenderCtx, ThinkingText, ToolInput, ToolResult, EDIT_TOOLS,
};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{Entry, EntryKind, ToolCard, ToolStatus};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent};
use serde_json::json;
use support::layout::{painted_text_settled_at, renderer};
use support::transcript::{ctx, fixture_lines, open_every_card, sequenced, view_of, Mounted};

/// Tall enough that the whole transcript is in view, so every entry is painted.
const ROOM: Size = Size::new(900.0, 60_000.0);

/// The turns replayed one after another: formatted text, edits with thinking, and a `Read`.
const TURNS: [&str; 3] = ["markdown-turn", "edit-turn", "tool-turn"];

/// `lines` as one window's transcript of `session`, its render cache refreshed as update does.
fn replayed(session: SessionId, lines: &[String]) -> (State, BlockCache) {
    let mut state = State::default();
    state.apply(session, 0, &sequenced(lines, 0), false);
    let mut cache = BlockCache::default();
    let _jobs = cache.refresh(session, state.transcript(session).unwrap(), &GRAMMAR);
    (state, cache)
}

fn turns() -> Vec<String> {
    TURNS.iter().flat_map(|name| fixture_lines(name)).collect()
}

fn painted(element: Block) -> Vec<String> {
    let (painted, _) = painted_text_settled_at(element, &mut renderer(), ROOM);
    painted.into_iter().map(|p| p.content).collect()
}

fn painted_view(
    state: &State,
    session: SessionId,
    renderers: &BlockRenderers,
    cache: &BlockCache,
) -> Vec<String> {
    let (painted, _) = painted_text_settled_at(
        view_of(state, session, renderers, cache),
        &mut renderer(),
        ROOM,
    );
    painted.into_iter().map(|p| p.content).collect()
}

/// `left == right`, or the first place they part.
#[track_caller]
fn assert_same(left: &[String], right: &[String], what: &str) {
    let at = left.iter().zip(right).position(|(l, r)| l != r);
    assert!(
        left == right,
        "{what}: they part at {at:?} (lengths {} and {}): {:?} vs {:?}",
        left.len(),
        right.len(),
        at.map(|i| &left[i.saturating_sub(1)..(i + 2).min(left.len())]),
        at.map(|i| &right[i.saturating_sub(1)..(i + 2).min(right.len())]),
    );
}

fn marker(_: &AssistantText<'_>, _: &RenderCtx<'_>) -> Block {
    iced::widget::text("«text»").into()
}

fn thinking_marker(_: &ThinkingText<'_>, _: &RenderCtx<'_>) -> Block {
    iced::widget::text("«thinking»").into()
}

fn input_marker(_: &ToolInput<'_>, _: &RenderCtx<'_>) -> Block {
    iced::widget::text("«input»").into()
}

/// `renderers` with its text, thinking and input blocks replaced by markers, so what is left is the
/// entries' frame (headers, statuses, footers) and the tool results.
fn frame_of(renderers: BlockRenderers) -> BlockRenderers {
    BlockRenderers {
        assistant_text: marker,
        thinking: thinking_marker,
        tool_input: input_marker,
        ..renderers
    }
}

fn entries(state: &State, session: SessionId) -> Vec<&Entry> {
    state.transcript(session).unwrap().entries().collect()
}

fn edit_cards<'s>(state: &'s State, session: SessionId) -> Vec<(&'s Entry, &'s ToolCard)> {
    entries(state, session)
        .into_iter()
        .filter_map(|e| match e.kind() {
            EntryKind::ToolCall(card) if EDIT_TOOLS.contains(&card.name.as_str()) => {
                Some((e, card))
            }
            _ => None,
        })
        .collect()
}

/// The `+N −M` of every edit card, from the cache, in transcript order.
fn metas(state: &State, cache: &BlockCache, session: SessionId) -> Vec<String> {
    edit_cards(state, session)
        .into_iter()
        .map(|(e, _)| {
            let (added, removed) = cache.edit_counts(session, e.key()).expect("a known diff");
            edit_meta(added, removed)
        })
        .collect()
}

fn block_ref(session: SessionId, entry: &Entry, streaming: bool) -> BlockRef<'_> {
    BlockRef {
        session,
        key: entry.key(),
        revision: entry.revision(),
        streaming,
    }
}

/// RR-1, RR-2, FR-020, FR-022: folded, then open, the frame RICH draws is PLAIN's plus each edit
/// card's `+N −M`, in the same place; the edit cards start folded.
#[test]
fn rich_keeps_entries_order_folds_statuses_and_headers_but_adds_edit_counts() {
    let session = SessionId::new();
    let (mut state, cache) = replayed(session, &turns());
    let edits = edit_cards(&state, session);
    assert!(edits.len() >= 3, "edit-turn records a Write and two Edits");
    for (entry, _) in &edits {
        assert!(
            !state.is_open(session, entry.key()),
            "an edit card starts folded (FR-020)"
        );
    }
    let metas = metas(&state, &cache, session);

    for open in [false, true] {
        if open {
            open_every_card(&mut state, session);
        }
        // The counts come from the cache, which PLAIN is drawn without, as #939's window was.
        let plain = painted_view(
            &state,
            session,
            &frame_of(BlockRenderers::PLAIN),
            &BlockCache::default(),
        );
        let rich = painted_view(&state, session, &frame_of(BlockRenderers::RICH), &cache);
        let shown: Vec<&String> = rich.iter().filter(|p| metas.contains(p)).collect();
        assert_eq!(
            shown.len(),
            metas.len(),
            "each edit card's header shows its +N −M (open: {open}): {rich:#?}"
        );
        let without: Vec<String> = rich.into_iter().filter(|p| !metas.contains(p)).collect();
        assert_same(
            &without,
            &plain,
            &format!(
                "but for the counts RICH draws PLAIN's entries, headers and results (open: {open})"
            ),
        );
    }
}

/// The text `text` entry draws through RICH, as assistant text and as thinking.
fn rich_text(
    session: SessionId,
    entry: &Entry,
    text: &str,
    ctx: &RenderCtx<'_>,
) -> [Vec<String>; 2] {
    let block = block_ref(session, entry, false);
    [
        painted((BlockRenderers::RICH.assistant_text)(
            &AssistantText { block, text },
            ctx,
        )),
        painted((BlockRenderers::RICH.thinking)(
            &ThinkingText { block, text },
            ctx,
        )),
    ]
}

/// RR-1, FR-002: assistant text is drawn from the cached document, formatted, without its markdown
/// punctuation; thinking the same way.
#[test]
fn text_and_thinking_are_drawn_from_the_cached_document() {
    let session = SessionId::new();
    let (state, cache) = replayed(session, &turns());
    let ctx = ctx(&cache);
    let (mut texts, mut thoughts) = (0, 0);
    for entry in entries(&state, session) {
        let text = match entry.kind() {
            EntryKind::AssistantText { text, .. } => {
                texts += 1;
                text
            }
            EntryKind::Thinking { text, .. } => {
                thoughts += 1;
                text
            }
            _ => continue,
        };
        assert!(
            matches!(
                cache.get(session, entry.key()).map(|d| &d.body),
                Some(DerivedBody::Text(_))
            ),
            "the cache derived a document for {:?}",
            entry.key()
        );
        let [as_text, as_thinking] = rich_text(session, entry, text, &ctx);
        assert_eq!(
            as_text, as_thinking,
            "thinking is formatted as assistant text is"
        );
    }
    assert!(
        texts > 0 && thoughts > 0,
        "the turns hold text and thinking"
    );

    let (entry, text) = entries(&state, session)
        .into_iter()
        .find_map(|e| match e.kind() {
            EntryKind::AssistantText { text, .. } if text.contains("**") => Some((e, text)),
            _ => None,
        })
        .expect("markdown-turn's reply");
    let [drawn, _] = rich_text(session, entry, text, &ctx);
    let plain = painted((BlockRenderers::PLAIN.assistant_text)(
        &AssistantText {
            block: block_ref(session, entry, false),
            text,
        },
        &ctx,
    ));
    assert_eq!(plain.len(), 1, "PLAIN draws the text as one paragraph");
    assert!(
        drawn.iter().any(|p| p == "Brewing Tea"),
        "the heading is its own text, without its `#`: {drawn:#?}"
    );
    assert!(
        drawn
            .iter()
            .any(|p| p.contains("println!(\"Steeping...\");")),
        "the code block's lines are drawn: {drawn:#?}"
    );
    assert!(
        !drawn.iter().any(|p| p.contains("**")
            || p.contains("~~")
            || p.starts_with('#')
            || p.contains("```")),
        "no formatting punctuation is drawn: {drawn:#?}"
    );
}

/// FR-013, FR-021: an edit card's input is its diff — the path and every changed line, not the
/// JSON; every tool result, a `Read`'s included, and every other input is PLAIN's.
#[test]
fn edit_inputs_are_diffs_and_results_stay_plain() {
    let session = SessionId::new();
    let (state, cache) = replayed(session, &turns());
    let ctx = ctx(&cache);
    let (mut read, mut diffs) = (false, 0);
    for entry in entries(&state, session) {
        let EntryKind::ToolCall(card) = entry.kind() else {
            continue;
        };
        let block = block_ref(session, entry, card.streaming);
        let input = ToolInput {
            block,
            tool: &card.name,
            input_json: &card.input,
        };
        let rich = painted((BlockRenderers::RICH.tool_input)(&input, &ctx));
        match cache.get(session, entry.key()).map(|d| &d.body) {
            Some(DerivedBody::Diff(Some(diff))) => {
                diffs += 1;
                assert!(rich.contains(&diff.path), "the path is drawn: {rich:#?}");
                for line in diff.hunks.iter().flat_map(|h| &h.lines) {
                    assert!(
                        rich.iter().any(|p| p.trim_end() == line.text.trim_end()),
                        "{:?} is drawn as a line: {rich:#?}",
                        line.text
                    );
                }
                assert!(
                    !rich
                        .iter()
                        .any(|p| p.contains("old_string") || p.contains("\"content\"")),
                    "an edit's input is not its JSON: {rich:#?}"
                );
            }
            _ => assert_eq!(
                rich,
                painted((BlockRenderers::PLAIN.tool_input)(&input, &ctx)),
                "{}'s input is PLAIN's",
                card.name
            ),
        }
        if let Some(result) = &card.result {
            read |= card.name == "Read";
            let result = ToolResult {
                block,
                tool: Some(&card.name),
                parts: &result.parts,
                is_error: result.is_error,
            };
            assert_eq!(
                painted((BlockRenderers::RICH.tool_result)(&result, &ctx)),
                painted((BlockRenderers::PLAIN.tool_result)(&result, &ctx)),
                "{}'s result is drawn as #939 draws it",
                card.name
            );
        }
    }
    assert!(diffs >= 3, "edit-turn's Write and two Edits are diffs");
    assert!(read, "tool-turn holds a Read result");
}

fn assistant(block: ContentBlock) -> StructuredEvent {
    StructuredEvent::Assistant {
        message_id: None,
        raw: json!({"type": "assistant", "message": {"content": []}}).to_string(),
        blocks: vec![block],
        parent_tool_use_id: None,
    }
}

/// The one tool card of `state`'s transcript, and its input drawn by RICH and by PLAIN.
fn input_drawn(
    state: &State,
    cache: &BlockCache,
    session: SessionId,
) -> (Vec<String>, Vec<String>) {
    let ctx = ctx(cache);
    let (entry, card) = edit_cards(state, session)
        .pop()
        .expect("the transcript holds an edit card");
    let input = ToolInput {
        block: block_ref(session, entry, card.streaming),
        tool: &card.name,
        input_json: &card.input,
    };
    (
        painted((BlockRenderers::RICH.tool_input)(&input, &ctx)),
        painted((BlockRenderers::PLAIN.tool_input)(&input, &ctx)),
    )
}

/// FR-018: while an Edit's input streams, and when it has the wrong shape, it is drawn as #939
/// draws it, without an error.
#[test]
fn a_streaming_or_malformed_edit_input_is_drawn_as_plain() {
    // edit-turn up to the middle of its first tool use's input.
    let lines = fixture_lines("edit-turn");
    let streaming = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("input_json_delta"))
        .nth(4)
        .map(|(i, _)| i)
        .expect("a streamed input");
    let session = SessionId::new();
    let (state, cache) = replayed(session, &lines[..streaming]);
    assert!(
        edit_cards(&state, session).iter().all(|(_, c)| c.streaming),
        "the card's input is still streaming"
    );
    let (rich, plain) = input_drawn(&state, &cache, session);
    assert_eq!(rich, plain, "a streaming input is PLAIN's");

    let session = SessionId::new();
    let mut state = State::default();
    let malformed = assistant(ContentBlock::ToolUse {
        id: "toolu_bad".into(),
        name: "Edit".into(),
        input: json!({"file_path": 3, "old_string": ["a"]}).to_string(),
    });
    state.apply(
        session,
        0,
        &[SequencedEvent {
            seq: 0,
            event: malformed,
        }],
        false,
    );
    let mut cache = BlockCache::default();
    let _jobs = cache.refresh(session, state.transcript(session).unwrap(), &GRAMMAR);
    let (rich, plain) = input_drawn(&state, &cache, session);
    assert_eq!(
        rich, plain,
        "a malformed input is PLAIN's, without an error"
    );
}

/// FR-018, US2 scenario 7: a failed Edit keeps its diff, and its card is marked failed as #939
/// marks it.
#[test]
fn a_failed_edit_keeps_its_diff_and_its_failed_mark() {
    let lines: Vec<String> = fixture_lines("edit-turn")
        .into_iter()
        .map(|l| {
            l.replace(
                "\"type\":\"tool_result\",",
                "\"type\":\"tool_result\",\"is_error\":true,",
            )
        })
        .collect();
    let session = SessionId::new();
    let (state, cache) = replayed(session, &lines);
    let (entry, card) = edit_cards(&state, session)
        .into_iter()
        .find(|(_, c)| c.name == "Edit")
        .expect("an Edit");
    assert_eq!(card.status, ToolStatus::Failed, "the result is an error");
    let counts = cache.edit_counts(session, entry.key());
    assert!(counts.is_some(), "the failed Edit keeps its diff");
    let header = tool_header_counted(card, counts);
    assert!(header.failed, "and its failed mark");
    assert!(header.meta.is_some(), "and its +N −M");
    let (rich, plain) = input_drawn(&state, &cache, session);
    assert_ne!(rich, plain, "its input is drawn as the diff");
}

/// Contract *Renderers*: on a cache miss every block is drawn as PLAIN draws it.
#[test]
fn a_cache_miss_draws_plain() {
    let session = SessionId::new();
    let (mut state, _) = replayed(session, &turns());
    open_every_card(&mut state, session);
    let empty = BlockCache::default();
    // An empty text block draws nothing at all, where PLAIN draws an empty paragraph.
    let drawn = |renderers| -> Vec<String> {
        painted_view(&state, session, renderers, &empty)
            .into_iter()
            .filter(|p| !p.is_empty())
            .collect()
    };
    assert_same(
        &drawn(&BlockRenderers::RICH),
        &drawn(&BlockRenderers::PLAIN),
        "a cache miss",
    );
}

thread_local! {
    static DRAWN: Cell<usize> = const { Cell::new(0) };
}

fn counting(block: &AssistantText<'_>, ctx: &RenderCtx<'_>) -> Block {
    DRAWN.with(|n| n.set(n.get() + 1));
    (BlockRenderers::RICH.assistant_text)(block, ctx)
}

const COUNTING: BlockRenderers = BlockRenderers {
    assistant_text: counting,
    ..BlockRenderers::RICH
};

/// RR-3: an entry's lazy key holds its cache stamp: a frame with nothing changed rebuilds no entry,
/// and a copy state (which bumps the stamp) rebuilds that entry only.
#[test]
fn the_lazy_key_changes_with_the_stamp_and_only_then() {
    let session = SessionId::new();
    let (state, mut cache) = replayed(session, &fixture_lines("markdown-turn"));
    let mut mounted = Mounted::new(ROOM);
    let mut frame = |cache: &BlockCache| {
        let before = DRAWN.with(Cell::get);
        mounted.frame(view_of(&state, session, &COUNTING, cache), Vec::new());
        DRAWN.with(Cell::get) - before
    };
    assert!(frame(&cache) > 0, "the first frame draws the reply");
    assert_eq!(frame(&cache), 0, "nothing changed: no entry is rebuilt");

    let (key, code) = entries(&state, session)
        .into_iter()
        .find_map(|e| match cache.get(session, e.key()).map(|d| &d.body) {
            Some(DerivedBody::Text(doc)) => doc.code_blocks().next().map(|(id, ..)| (e.key(), id)),
            _ => None,
        })
        .expect("markdown-turn's reply holds a code block");
    let stamp = cache.get(session, key).unwrap().stamp;
    cache
        .copy(session, key, code)
        .expect("the code block copies");
    assert_ne!(
        cache.get(session, key).unwrap().stamp,
        stamp,
        "a copy bumps the stamp"
    );
    assert_eq!(
        frame(&cache),
        1,
        "the stamp changed: that entry, and only it, is rebuilt"
    );
    assert_eq!(frame(&cache), 0, "and not again");
}
