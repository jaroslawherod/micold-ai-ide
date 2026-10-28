//! T034 (feature 939, SC-004, research R12): a transcript holding the history bound of events
//! (5,000), every card unfolded, opens within one second, and each streamed delta's update, view and
//! layout stays under 100 ms, so the view never stops answering input for longer.
//!
//! Release-build numbers, as `micold-core/tests/typeahead_budget.rs`'s (BUG-003): run with
//! `cargo test --release -p micold-client --test transcript_open_budget`, CI's release-budget step.
//! A debug build is many times slower by a ratio that moves with load, so it is not SC-004's number.
//!
//! T036 (feature 940, SC-004, research R12): the same budgets with the window's rich renderers over
//! a history of markdown and edit turns — every reply formatted with its code blocks, every Edit and
//! Write card open on its diff — the render cache refreshed after each applied batch as update does.

mod support;

use iced::advanced::layout;
use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::Tree;
use iced::advanced::{mouse, Renderer as _};
use iced::{Rectangle, Size};
use micold_client::features::structured_transcript::State;
use micold_client::features::transcript_selection::{
    copy_text, units, Click, SelectMsg, State as SelectionState, TranscriptView,
};
use micold_client::ui::syntax::GRAMMAR;
use micold_client::ui::transcript::{view, BlockCache, BlockRenderers, RenderCtx};
use micold_core::session::SessionId;
use micold_core::structured::selection::{RowPos, Selection, TextPos, Under};
use micold_core::structured::MAX_EVENTS;
use std::time::{Duration, Instant};
use support::layout::renderer;
use support::transcript::{ctx, fixture_lines, open_every_card, sequenced};

const OPEN_BUDGET: Duration = Duration::from_secs(1);
const DELTA_BUDGET: Duration = Duration::from_millis(100);
const WINDOW: Size = Size::new(1280.0, 800.0);

/// Held by each budget for its whole run. The harness runs a binary's tests in parallel, and a budget
/// measured beside another one's work measures the contention: on an idle machine the parallel run
/// failed the delta budget four times in six, the serial one never (#1453).
static ONE_BUDGET_AT_A_TIME: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The lock above; a budget that failed still lets the next one run.
fn alone() -> std::sync::MutexGuard<'static, ()> {
    ONE_BUDGET_AT_A_TIME
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The recordings a history repeats, and the renderers it is drawn with.
#[derive(Clone, Copy)]
struct Kind {
    turns: &'static [&'static str],
    renderers: &'static BlockRenderers,
    /// Whether the render cache is refreshed after each applied batch, as the window's update does
    /// for the rich renderers (RR-4).
    refresh: bool,
}

/// #939: tool turns drawn as plain text.
const PLAIN: Kind = Kind {
    turns: &["tool-turn"],
    renderers: &BlockRenderers::PLAIN,
    refresh: false,
};

/// #940: markdown and edit turns drawn rich.
const RICH: Kind = Kind {
    turns: &["markdown-turn", "edit-turn"],
    renderers: &BlockRenderers::RICH,
    refresh: true,
};

/// `turn`'s lines with its message and tool use ids made `copy`'s own.
fn renamed(turn: &[String], copy: &str) -> Vec<String> {
    turn.iter()
        .map(|line| {
            line.replace("toolu_", &format!("toolu_{copy}_"))
                .replace("msg_", &format!("msg_{copy}_"))
        })
        .collect()
}

/// `MAX_EVENTS` lines: `turns`' recordings repeated in turn, each copy's message and tool use ids
/// its own, so every copy pairs its own result and opens its own cards.
fn history(turns: &[&str]) -> Vec<String> {
    let turns: Vec<Vec<String>> = turns.iter().map(|name| fixture_lines(name)).collect();
    (0..)
        .flat_map(|copy: usize| renamed(&turns[copy % turns.len()], &copy.to_string()))
        .take(MAX_EVENTS)
        .collect()
}

/// Lay `state`'s view out over `tree` and draw one frame of it.
fn frame(
    state: &State,
    session: SessionId,
    (renderers, cache): (&BlockRenderers, &BlockCache),
    tree: &mut Option<Tree>,
    renderer: &mut iced::Renderer,
    draw: bool,
) {
    frame_selected(
        state,
        session,
        (renderers, cache),
        tree,
        renderer,
        draw,
        None,
    );
}

/// [`frame`] with `selected` drawn as the session's selection (feature 1550).
fn frame_selected(
    state: &State,
    session: SessionId,
    (renderers, cache): (&BlockRenderers, &BlockCache),
    tree: &mut Option<Tree>,
    renderer: &mut iced::Renderer,
    draw: bool,
    selected: Option<&Selection>,
) {
    let ctx = RenderCtx {
        selected,
        ..ctx(cache)
    };
    let mut element = view(state, session, renderers, ctx);
    let tree = match tree {
        Some(tree) => {
            tree.diff(element.as_widget());
            tree
        }
        None => tree.insert(Tree::new(element.as_widget())),
    };
    let node =
        element
            .as_widget_mut()
            .layout(tree, renderer, &layout::Limits::new(Size::ZERO, WINDOW));
    if draw {
        let viewport = Rectangle::with_size(WINDOW);
        renderer.reset(viewport);
        element.as_widget().draw(
            tree,
            renderer,
            &iced::Theme::Light,
            &iced::advanced::renderer::Style::default(),
            layout::Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        let _ = renderer.screenshot(Size::new(1280, 800), 1.0, iced::Color::WHITE);
    }
}

/// A full history opened with every card unfolded, and the state, caches and renderer it left.
struct Opened {
    kind: Kind,
    session: SessionId,
    state: State,
    cache: BlockCache,
    renderer: iced::Renderer,
    tree: Option<Tree>,
    elapsed: Duration,
}

/// Apply `events` to `session`'s transcript, and refresh its render cache when `kind` draws from it.
fn apply(
    kind: Kind,
    (state, cache): (&mut State, &mut BlockCache),
    session: SessionId,
    events: &[micold_core::structured::SequencedEvent],
    live: bool,
) {
    state.apply(session, 0, events, live);
    if kind.refresh {
        let transcript = state.transcript(session).expect("applied");
        let _jobs = cache.refresh(session, transcript, &GRAMMAR);
    }
}

fn opened(kind: Kind) -> Opened {
    let session = SessionId::new();
    let history = sequenced(&history(kind.turns), 0);
    let mut cache = BlockCache::default();
    let mut renderer = renderer();
    let mut tree = None;

    let started = Instant::now();
    let mut state = State::default();
    apply(kind, (&mut state, &mut cache), session, &history, false);
    open_every_card(&mut state, session);
    frame(
        &state,
        session,
        (kind.renderers, &cache),
        &mut tree,
        &mut renderer,
        true,
    );
    let elapsed = started.elapsed();
    eprintln!("opened in {elapsed:?}");
    Opened {
        kind,
        session,
        state,
        cache,
        renderer,
        tree,
        elapsed,
    }
}

fn opens_within_a_second(kind: Kind) {
    let _alone = alone();
    let opened = opened(kind);
    assert!(
        opened.state.transcript(opened.session).unwrap().held() == MAX_EVENTS,
        "the transcript holds the bound"
    );
    assert!(
        opened.elapsed < OPEN_BUDGET,
        "opening a {MAX_EVENTS}-event transcript, every card open, took {:?}",
        opened.elapsed
    );
}

/// A reply streamed in on top of a full history, one event at a time, as the wire delivers it live:
/// `turn`'s recording, its ids its own.
fn each_delta_within_100_ms(kind: Kind, turn: &str) {
    let _alone = alone();
    let Opened {
        kind,
        session,
        mut state,
        mut cache,
        mut renderer,
        mut tree,
        ..
    } = opened(kind);

    let deltas = sequenced(&renamed(&fixture_lines(turn), "live"), MAX_EVENTS as u64);
    let mut slowest = Duration::ZERO;
    for event in deltas {
        let started = Instant::now();
        let event = std::slice::from_ref(&event);
        apply(kind, (&mut state, &mut cache), session, event, true);
        frame(
            &state,
            session,
            (kind.renderers, &cache),
            &mut tree,
            &mut renderer,
            false,
        );
        slowest = slowest.max(started.elapsed());
    }
    eprintln!("slowest delta in {slowest:?}");
    assert!(
        slowest < DELTA_BUDGET,
        "the slowest delta's update, view and layout took {slowest:?}"
    );
}

/// The view the selection model reads of `opened`'s transcript.
fn selection_view(o: &Opened) -> TranscriptView<'_> {
    TranscriptView {
        session: o.session,
        transcript: o.state.transcript(o.session).expect("opened"),
        open: o.state.open_cards(o.session).expect("opened"),
        rich: &o.cache,
        prompts: None,
        insights: None,
    }
}

/// The start of the first unit of the `n`-th entry from `n` on that draws one.
fn unit_at(view: &TranscriptView<'_>, n: usize) -> TextPos {
    view.transcript
        .entries()
        .skip(n)
        .find_map(|e| {
            let row = RowPos::Entry(e.id(), Under::Body);
            let unit = units(&row, view).into_iter().next()?;
            Some(TextPos {
                row,
                path: unit.path,
                offset: 0,
            })
        })
        .expect("an entry with a unit")
}

/// #1550 SC-003 (research R12): on a full rich history, every card open, select-all and the copy
/// of it take under a second.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-003, as BUG-003)"
)]
fn select_all_and_copy_of_a_full_history_take_under_a_second() {
    let _alone = alone();
    let opened = opened(RICH);
    let view = selection_view(&opened);
    let session = opened.session;
    let mut selections = SelectionState::default();
    let started = Instant::now();
    selections.update(SelectMsg::SelectAll { session }, Some(&view));
    let selection = selections.selection(session).expect("all is selected");
    let copied = copy_text(&view, selection);
    let elapsed = started.elapsed();
    eprintln!("select-all and copy in {elapsed:?}, {} bytes", copied.len());
    assert!(!copied.is_empty());
    assert!(
        elapsed < OPEN_BUDGET,
        "select-all and copy of a {MAX_EVENTS}-event transcript took {elapsed:?}"
    );
}

/// #1550 SC-003 (research R12): one drag step on a full rich history — the hit, the update and
/// the view and layout of the entries it changed — takes under 100 ms.
#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-003, as BUG-003)"
)]
fn each_drag_step_on_a_full_history_updates_within_100_ms() {
    let _alone = alone();
    let mut opened = opened(RICH);
    let session = opened.session;
    let mut selections = SelectionState::default();
    let hit = |pos: TextPos| SelectMsg::Hit {
        session,
        pos,
        click: Click::Single,
        line: None,
    };
    let held = opened
        .state
        .transcript(session)
        .expect("opened")
        .entries()
        .len();
    let steps: Vec<TextPos> = {
        let view = selection_view(&opened);
        (1..=6).map(|k| unit_at(&view, k * held / 8)).collect()
    };
    selections.update(
        SelectMsg::Press {
            session,
            clicks: Click::Single,
        },
        None,
    );
    let mut slowest = Duration::ZERO;
    for (k, pos) in steps.into_iter().enumerate() {
        let started = Instant::now();
        selections.update(hit(pos), Some(&selection_view(&opened)));
        frame_selected(
            &opened.state,
            session,
            (opened.kind.renderers, &opened.cache),
            &mut opened.tree,
            &mut opened.renderer,
            false,
            selections.selection(session),
        );
        // The first hit starts the selection; the steps after it are the drag.
        if k > 0 {
            slowest = slowest.max(started.elapsed());
        }
    }
    selections.update(SelectMsg::Release { session }, None);
    assert!(selections.selecting(session), "the drag selected text");
    eprintln!("slowest drag step in {slowest:?}");
    assert!(
        slowest < DELTA_BUDGET,
        "the slowest drag step's update, view and layout took {slowest:?}"
    );
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-004, as BUG-003)"
)]
fn a_full_history_with_every_card_open_opens_within_a_second() {
    opens_within_a_second(PLAIN);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-004, as BUG-003)"
)]
fn each_streamed_delta_on_a_full_history_updates_within_100_ms() {
    each_delta_within_100_ms(PLAIN, "partial-deltas");
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-004, as BUG-003)"
)]
fn a_full_rich_history_with_every_card_open_opens_within_a_second() {
    opens_within_a_second(RICH);
}

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-004, as BUG-003)"
)]
fn each_streamed_markdown_delta_on_a_full_rich_history_updates_within_100_ms() {
    each_delta_within_100_ms(RICH, "markdown-turn");
}
