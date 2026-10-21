//! T034 (feature 939, SC-004, research R12): a transcript holding the history bound of events
//! (5,000), every card unfolded, opens within one second, and each streamed delta's update, view and
//! layout stays under 100 ms, so the view never stops answering input for longer.
//!
//! Release-build numbers, as `micold-core/tests/typeahead_budget.rs`'s (BUG-003): run with
//! `cargo test --release -p micold-client --test transcript_open_budget`, CI's release-budget step.
//! A debug build is many times slower by a ratio that moves with load, so it is not SC-004's number.

mod support;

use iced::advanced::layout;
use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::Tree;
use iced::advanced::{mouse, Renderer as _};
use iced::{Rectangle, Size};
use micold_client::features::structured_transcript::State;
use micold_client::ui::transcript::{BlockCache, BlockRenderers};
use micold_core::session::SessionId;
use micold_core::structured::MAX_EVENTS;
use std::time::{Duration, Instant};
use support::layout::renderer;
use support::transcript::{fixture_lines, open_every_card, sequenced, view_of};

const OPEN_BUDGET: Duration = Duration::from_secs(1);
const DELTA_BUDGET: Duration = Duration::from_millis(100);
const WINDOW: Size = Size::new(1280.0, 800.0);

/// `MAX_EVENTS` lines: the tool-turn recording repeated, each copy's message and tool use ids its
/// own, so every copy pairs its own result and opens its own cards.
fn history() -> Vec<String> {
    let turn = fixture_lines("tool-turn");
    (0..)
        .flat_map(|copy| {
            turn.iter()
                .map(move |line| {
                    line.replace("toolu_", &format!("toolu_{copy}_"))
                        .replace("msg_", &format!("msg_{copy}_"))
                })
                .collect::<Vec<_>>()
        })
        .take(MAX_EVENTS)
        .collect()
}

/// Lay `state`'s view out over `tree` and draw one frame of it.
fn frame(
    state: &State,
    session: SessionId,
    cache: &BlockCache,
    tree: &mut Option<Tree>,
    renderer: &mut iced::Renderer,
    draw: bool,
) {
    let mut element = view_of(state, session, &BlockRenderers::PLAIN, cache);
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

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "a release-build budget (SC-004, as BUG-003)"
)]
fn a_full_history_with_every_card_open_opens_within_a_second_and_each_delta_within_100_ms() {
    let session = SessionId::new();
    let history = sequenced(&history(), 0);
    let cache = BlockCache::default();
    let mut renderer = renderer();
    let mut tree = None;

    let started = Instant::now();
    let mut state = State::default();
    state.apply(session, 0, &history, false);
    open_every_card(&mut state, session);
    frame(&state, session, &cache, &mut tree, &mut renderer, true);
    let opened = started.elapsed();
    eprintln!("opened in {opened:?}");
    assert!(
        state.transcript(session).unwrap().held() == MAX_EVENTS,
        "the transcript holds the bound"
    );
    assert!(
        opened < OPEN_BUDGET,
        "opening a {MAX_EVENTS}-event transcript, every card open, took {opened:?}"
    );

    // A reply streamed in on top, one event at a time, as the wire delivers it live.
    let deltas = sequenced(&fixture_lines("partial-deltas"), MAX_EVENTS as u64);
    let mut slowest = Duration::ZERO;
    for event in deltas {
        let started = Instant::now();
        state.apply(session, 0, std::slice::from_ref(&event), true);
        frame(&state, session, &cache, &mut tree, &mut renderer, false);
        slowest = slowest.max(started.elapsed());
    }
    eprintln!("slowest delta in {slowest:?}");
    assert!(
        slowest < DELTA_BUDGET,
        "the slowest delta's update, view and layout took {slowest:?}"
    );
}
