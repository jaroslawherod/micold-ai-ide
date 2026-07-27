//! T038/US2 — local interactions (scroll, select, resize) are served from local state and issue
//! **zero round trips** to the daemon (FR-020, SC-004/005).
//!
//! FR-020 requires that "rendering, scrolling, and selection MUST be served from local state so no
//! interaction stalls on communication with the service." The client enforces this structurally: the
//! per-session [`GridCache`], the [`Selection`] model, and the scroll math ([`target_offset_delta`])
//! are pure projections that hold **no daemon handle at all** — they cannot send a request or await a
//! reply. This test drives that local stack directly and proves:
//!
//! - a scroll reads the visible screen, any line, and the scrollback watermarks from the cache alone;
//! - a text selection is built and its text extracted purely from cached lines;
//! - that selection is unperturbed when new output arrives, so no re-fetch is ever needed (FR-018);
//! - a resize (a generation bump + full reship) is absorbed by the cache locally, with no exchange
//!   required to render the new dimensions.
//!
//! (Pane resize also dispatches a single fire-and-forget `SessionResize` and returns immediately —
//! it never blocks on a reply — so it is not a round trip either; that send is covered where the
//! outbox is driven, not here.)

use micold_client::grid::GridCache;
use micold_client::selection::{Anchor, SelectGranularity, Selection};
use micold_client::ui::target_offset_delta;
use micold_core::protocol::grid::{
    GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine, WireStyle,
};
use micold_core::session::SessionId;

fn a_style() -> WireStyle {
    WireStyle {
        fg: WireColor::Named(7),
        bg: WireColor::Named(0),
        flags: 0,
        underline_color: None,
    }
}

fn a_cursor(line: i64) -> WireCursor {
    WireCursor {
        line: LineId(line),
        col: 0,
        shape: WireCursorShape::Block,
        visible: true,
        blinking: false,
    }
}

fn wire_line(id: i64, text: &str) -> WireLine {
    let len = text.chars().count() as u16;
    WireLine {
        id: LineId(id),
        text: text.to_string(),
        runs: vec![StyleRun { len, style: 0 }],
        extras: Vec::new(),
        wrapped: false,
    }
}

/// A full snapshot frame at `viewport_top`, one style in the palette.
fn snapshot(session: SessionId, viewport_top: i64, lines: Vec<WireLine>) -> GridFrame {
    let rows = lines.len() as u16;
    GridFrame {
        session,
        seq: 1,
        generation: 0,
        full: true,
        viewport_top: LineId(viewport_top),
        oldest_available: LineId(0),
        cols: 80,
        rows,
        cursor: a_cursor(viewport_top),
        styles: vec![a_style()],
        hyperlinks: Vec::new(),
        lines,
        mode: 0,
        input_serial: None,
    }
}

#[test]
fn scrolling_reads_come_from_the_local_grid_cache() {
    let mut cache = GridCache::new();
    let session = SessionId::new();
    cache.apply(&snapshot(
        session,
        0,
        vec![
            wire_line(0, "row0"),
            wire_line(1, "row1"),
            wire_line(2, "row2"),
        ],
    ));

    // The visible screen, any line by id, and the scrollback watermarks all resolve from the cache —
    // exactly what a scroll gesture consults — with no daemon involved.
    let screen = cache.screen();
    assert_eq!(screen.len(), 3);
    assert_eq!(screen[1].unwrap().text, "row1");
    assert_eq!(cache.line(LineId(2)).unwrap().text, "row2");
    assert_eq!(cache.viewport_top(), LineId(0));
    assert_eq!(cache.oldest_available(), LineId(0));

    // The scrollbar-drag math is a pure offset computation, never an exchange.
    assert_eq!(target_offset_delta(0, 5), 5);
    assert_eq!(target_offset_delta(10, 3), -7);
}

#[test]
fn a_selection_is_built_and_extracted_from_cached_lines_only() {
    let mut cache = GridCache::new();
    let session = SessionId::new();
    cache.apply(&snapshot(
        session,
        0,
        vec![wire_line(0, "hello"), wire_line(1, "world")],
    ));

    // The line provider reads ONLY the local cache — the Selection model never reaches past it.
    let provider = |id: LineId| cache.line(id).map(|l| l.text.clone());
    let mut sel = Selection::start(Anchor::new(LineId(0), 0), SelectGranularity::Char, provider);
    sel.update(Anchor::new(LineId(1), 5), provider);

    let text = sel.text(provider);
    assert!(
        text.contains("hello"),
        "selection text from cache: {text:?}"
    );
    assert!(
        text.contains("world"),
        "selection text from cache: {text:?}"
    );
    assert!(sel.contains(LineId(0), 0));
}

#[test]
fn a_selection_survives_new_output_without_any_refetch() {
    let mut cache = GridCache::new();
    let session = SessionId::new();
    cache.apply(&snapshot(
        session,
        0,
        vec![wire_line(0, "keep me"), wire_line(1, "line one")],
    ));

    // Select the first line. The provider borrows the cache only for the duration of the call — the
    // Selection stores absolute LineIds, not the provider, so the cache is free to change afterward.
    let sel = {
        let provider = |id: LineId| cache.line(id).map(|l| l.text.clone());
        Selection::start(Anchor::new(LineId(0), 0), SelectGranularity::Line, provider)
    };

    // New output arrives as a delta with a higher LineId — the kind of thing that would corrupt an
    // offset-based selection. Anchored to LineId(0), ours is untouched, and needs no re-fetch.
    let mut delta = snapshot(session, 0, vec![wire_line(2, "new output")]);
    delta.full = false;
    delta.seq = 2;
    cache.apply(&delta);

    let provider = |id: LineId| cache.line(id).map(|l| l.text.clone());
    assert!(
        sel.text(provider).contains("keep me"),
        "selection stays anchored across new output (FR-018)"
    );
    assert!(sel.contains(LineId(0), 0));
}

#[test]
fn a_resize_generation_bump_is_absorbed_locally() {
    let mut cache = GridCache::new();
    let session = SessionId::new();
    cache.apply(&snapshot(
        session,
        0,
        vec![wire_line(0, "before"), wire_line(1, "resize")],
    ));

    // A resize bumps the generation and reships a full frame at the new size; the cache adopts the new
    // dimensions on its own, with no client->daemon exchange required to render them.
    let mut resized = snapshot(session, 0, vec![wire_line(0, "widerrow")]);
    resized.seq = 2;
    resized.generation = 1;
    resized.cols = 120;
    resized.rows = 1;
    cache.apply(&resized);

    assert_eq!(cache.cols(), 120);
    assert_eq!(cache.rows(), 1);
    assert_eq!(cache.generation(), 1);
    assert_eq!(cache.screen()[0].unwrap().text, "widerrow");
}
