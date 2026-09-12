//! Per-session renderable grid cache (client side of the grid stream).
//!
//! A client applies [`GridFrame`]s to a [`GridCache`] and the renderer queries it. The cache turns
//! the wire's compact, **frame-relative** representation into a self-contained, frame-independent
//! model that stays renderable no matter what later frames carry.
//!
//! ## Correctness rules (mirrors `micold_core::protocol::grid` and the daemon framer)
//!
//! 1. **Styles are interned per frame.** A [`WireLine`]'s `runs[i].style` is an index into *that*
//!    frame's [`GridFrame::styles`] palette — valid only for that frame. When a line is applied it is
//!    resolved: each RLE run's index becomes a concrete [`WireStyle`], and each
//!    `CellExtras::hyperlink` index becomes the concrete URI [`String`] from
//!    [`GridFrame::hyperlinks`]. No frame-relative index is ever stored, so a line cached from frame N
//!    stays correct after frame N+1 arrives with a different palette.
//! 2. **The visible screen is `rows` contiguous line ids from `viewport_top`.** Screen row `i`
//!    (0-based, top to bottom) is [`LineId`]`(viewport_top.0 + i)`.
//! 3. **`generation`.** A frame whose `generation` differs from the cache's current generation makes
//!    prior line identities meaningless: the cache clears and applies the frame as a fresh snapshot,
//!    regardless of `full`.
//! 4. **`seq` ordering.** `seq` is monotonic per session. A delta whose `seq <= current` is stale or
//!    reordered and is ignored. A `full` snapshot (or a generation change) always applies and resets
//!    the sequence.
//! 5. **`oldest_available`.** After applying a frame, any cached line with `id < oldest_available` is
//!    evicted (scrollback trim, data-model I3).

use micold_core::protocol::grid::{
    GridFrame, LineId, WireColor, WireCursor, WireCursorShape, WireLine, WireStyle,
};
use std::collections::HashMap;

/// Fallback style for a run whose interned index is missing from the frame palette (a protocol
/// violation that must never panic the renderer). Plain default colors, no flags.
const FALLBACK_STYLE: WireStyle = WireStyle {
    fg: WireColor::Named(0),
    bg: WireColor::Named(0),
    flags: 0,
    underline_color: None,
};

/// A resolved sparse per-cell extra: combining marks and/or a concrete hyperlink URI at one column.
/// Frame-independent — the hyperlink index has already been resolved against the frame's palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedExtra {
    /// The cell column this applies to.
    pub col: u16,
    /// Combining marks / ZWJ sequences attached to the cell.
    pub zerowidth: Vec<char>,
    /// The concrete hyperlink URI, when the cell is part of a hyperlink.
    pub hyperlink: Option<String>,
}

/// A cached line, fully resolved and independent of any frame's palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedLine {
    /// Stable absolute identity.
    pub id: LineId,
    /// One `char` per cell (wide-char trailing cells keep their spacer sentinel).
    pub text: String,
    /// Per-run concrete styles: `(run length in cells, resolved style)`. `sum(len)` == cell count.
    pub runs: Vec<(u16, WireStyle)>,
    /// Resolved sparse extras (zerowidth marks + concrete hyperlink URIs), by column.
    pub extras: Vec<CachedExtra>,
    /// Whether the line soft-wraps into the next.
    pub wrapped: bool,
}

/// A per-session cache of resolved grid lines plus the last-applied viewport metadata.
#[derive(Debug, Clone)]
pub struct GridCache {
    cols: u16,
    rows: u16,
    cursor: WireCursor,
    mode: u32,
    seq: u64,
    generation: u64,
    viewport_top: LineId,
    oldest_available: LineId,
    /// Resolved lines keyed by stable [`LineId`]; holds the visible screen and any still-cached
    /// scrollback that has not been trimmed by `oldest_available`.
    lines: HashMap<LineId, CachedLine>,
}

impl GridCache {
    /// An empty cache, before any frame has been applied.
    pub fn new() -> Self {
        Self {
            cols: 0,
            rows: 0,
            cursor: WireCursor {
                line: LineId(0),
                col: 0,
                shape: WireCursorShape::Hidden,
                visible: false,
                blinking: false,
            },
            mode: 0,
            seq: 0,
            generation: 0,
            viewport_top: LineId(0),
            oldest_available: LineId(0),
            lines: HashMap::new(),
        }
    }

    /// Apply a frame, honoring generation/seq ordering and the `oldest_available` trim.
    ///
    /// A `full` snapshot or a generation change resets the cache (clear, then apply as a snapshot). A
    /// delta upserts only its changed lines by [`LineId`]; a delta with `seq <= current` is ignored.
    pub fn apply(&mut self, frame: &GridFrame) {
        let reset = frame.full || frame.generation != self.generation;

        // A delta must be strictly newer; a reset (full or generation change) always applies.
        if !reset && frame.seq <= self.seq {
            return;
        }

        if reset {
            self.lines.clear();
            self.generation = frame.generation;
        }

        self.seq = frame.seq;
        self.cols = frame.cols;
        self.rows = frame.rows;
        self.cursor = frame.cursor;
        self.mode = frame.mode;
        self.viewport_top = frame.viewport_top;
        self.oldest_available = frame.oldest_available;

        for line in &frame.lines {
            self.lines.insert(
                line.id,
                resolve_line(line, &frame.styles, &frame.hyperlinks),
            );
        }

        // Scrollback trim: drop anything below the watermark (data-model I3).
        let watermark = frame.oldest_available.0;
        self.lines.retain(|id, _| id.0 >= watermark);
    }

    /// Insert fetched scrollback lines (a `ScrollbackResponse`), resolved against that response's
    /// own per-response palette. History is immutable once scrolled off, so these upsert by
    /// `LineId` alongside the viewport without disturbing it (no cursor/geometry/seq change). Lines
    /// below the current trim watermark are ignored.
    pub fn apply_scrollback(
        &mut self,
        lines: &[WireLine],
        styles: &[WireStyle],
        hyperlinks: &[String],
    ) {
        let watermark = self.oldest_available.0;
        for line in lines {
            if line.id.0 >= watermark {
                self.lines
                    .insert(line.id, resolve_line(line, styles, hyperlinks));
            }
        }
    }

    /// Viewport width in cells (last applied frame).
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Viewport height in cells (last applied frame).
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// The last-applied cursor.
    pub fn cursor(&self) -> WireCursor {
        self.cursor
    }

    /// The last-applied `TermMode` bits (raw).
    pub fn mode(&self) -> u32 {
        self.mode
    }

    /// The last-applied per-session frame sequence.
    pub fn seq(&self) -> u64 {
        self.seq
    }

    /// The current generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The line id at the top of the viewport.
    pub fn viewport_top(&self) -> LineId {
        self.viewport_top
    }

    /// The trim watermark: the oldest retained line id.
    pub fn oldest_available(&self) -> LineId {
        self.oldest_available
    }

    /// Look up a cached line by id (visible screen or still-cached scrollback).
    pub fn line(&self, id: LineId) -> Option<&CachedLine> {
        self.lines.get(&id)
    }

    /// The visible screen, top to bottom: exactly `rows` entries. `None` where a row's line is not
    /// (yet) cached.
    pub fn screen(&self) -> Vec<Option<&CachedLine>> {
        (0..self.rows)
            .map(|i| self.lines.get(&LineId(self.viewport_top.0 + i as i64)))
            .collect()
    }

    /// The one scrollback range worth asking the daemon for, with the view scrolled back `offset`
    /// lines and `inflight` already asked for and not yet answered. `None` when every revealed row
    /// is cached or already on its way.
    ///
    /// # Why this is a function rather than three lines at the call site (`010` BUG-021)
    ///
    /// It used to be three lines at the call site, and they asked for
    /// `first_uncached_line .. viewport_top` — everything from the revealed line down to the *live
    /// tail*. That range is correct and it is the wrong size: it grows with scroll depth, so 600
    /// lines back asks for 600 lines, once per wheel notch, and the total work over a gesture is
    /// quadratic in how far it goes. The daemon serves each faithfully and serially under the
    /// session's terminal lock, which is how a three-second gesture left it saturated for fifteen
    /// seconds of silence and the pane blank throughout.
    ///
    /// What a scrolled-back view actually needs is the rows it reveals. So the range returned here
    /// is bounded by construction — at most two viewports, whatever the depth:
    ///
    /// * it starts at the first revealed line that is neither cached nor already asked for;
    /// * it runs forward only to the end of that uncached run, capped at one viewport and at
    ///   `viewport_top` — asking past a line already here re-fetches what the last answer brought;
    /// * and it extends one viewport *further into history*, which is the direction the gesture is
    ///   travelling, so the next thirty-odd notches find their lines cached instead of asking again
    ///   for each one.
    ///
    /// `inflight` is what makes the prefetch worth having. During a fast gesture no response has
    /// arrived yet, so without it every notch would compute the same uncached run and ask for it
    /// again; with it, one request covers a viewport of travel and the notches in between send
    /// nothing.
    pub fn needed_scrollback(
        &self,
        offset: usize,
        inflight: &[std::ops::Range<LineId>],
    ) -> Option<std::ops::Range<LineId>> {
        let vt = self.viewport_top.0;
        let oldest = self.oldest_available.0;
        let rows = self.rows as i64;
        let top = (vt - offset as i64).max(oldest);

        // "Have it, or it is coming." Either way, asking again buys nothing.
        let held = |lid: i64| {
            self.lines.contains_key(&LineId(lid))
                || inflight.iter().any(|r| r.contains(&LineId(lid)))
        };

        let bottom = (top + rows).min(vt);
        let from = (top..bottom).find(|&lid| !held(lid))?;

        let mut end = from;
        while end < (from + rows).min(vt) && !held(end) {
            end += 1;
        }
        let mut start = from;
        while start > (from - rows).max(oldest) && !held(start - 1) {
            start -= 1;
        }
        Some(LineId(start)..LineId(end))
    }
}

impl Default for GridCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve one wire line against its frame's palettes into a frame-independent [`CachedLine`].
fn resolve_line(line: &WireLine, styles: &[WireStyle], hyperlinks: &[String]) -> CachedLine {
    let runs = line
        .runs
        .iter()
        .map(|run| {
            let style = styles
                .get(run.style as usize)
                .copied()
                .unwrap_or(FALLBACK_STYLE);
            (run.len, style)
        })
        .collect();

    let extras = line
        .extras
        .iter()
        .map(|extra| CachedExtra {
            col: extra.col,
            zerowidth: extra.zerowidth.clone(),
            hyperlink: extra
                .hyperlink
                .and_then(|i| hyperlinks.get(i as usize).cloned()),
        })
        .collect();

    CachedLine {
        id: line.id,
        text: line.text.clone(),
        runs,
        extras,
        wrapped: line.wrapped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::protocol::grid::{CellExtras, StyleRun};
    use micold_core::session::SessionId;

    fn a_cursor(line: i64) -> WireCursor {
        WireCursor {
            line: LineId(line),
            col: 0,
            shape: WireCursorShape::Block,
            visible: true,
            blinking: false,
        }
    }

    fn style(fg: u16) -> WireStyle {
        WireStyle {
            fg: WireColor::Named(fg),
            bg: WireColor::Named(0),
            flags: 0,
            underline_color: None,
        }
    }

    /// A single-run line covering all its cells with palette index `style_idx`.
    fn line(id: i64, text: &str, style_idx: u16) -> WireLine {
        let cols = text.chars().count() as u16;
        WireLine {
            id: LineId(id),
            text: text.to_string(),
            runs: vec![StyleRun {
                len: cols,
                style: style_idx,
            }],
            extras: Vec::new(),
            wrapped: false,
        }
    }

    /// A bare frame; the caller fills in the fields that matter to the test.
    fn frame(seq: u64, generation: u64, full: bool, viewport_top: i64, rows: u16) -> GridFrame {
        GridFrame {
            session: SessionId::new(),
            seq,
            generation,
            full,
            viewport_top: LineId(viewport_top),
            oldest_available: LineId(0),
            cols: 80,
            rows,
            cursor: a_cursor(viewport_top),
            styles: Vec::new(),
            hyperlinks: Vec::new(),
            lines: Vec::new(),
            mode: 0,
            input_serial: None,
        }
    }

    // --- `010` BUG-021: the range a scrolled-back view asks for ---------------------------------

    /// A cache holding its `rows` visible lines at `viewport_top` and none of its history — the
    /// state a window is in the moment a scroll-back gesture starts.
    fn at_the_tail(viewport_top: i64, rows: u16, oldest: i64) -> GridCache {
        let mut f = frame(1, 1, true, viewport_top, rows);
        f.oldest_available = LineId(oldest);
        f.lines = (0..rows as i64)
            .map(|i| line(viewport_top + i, "x", 0))
            .collect();
        let mut cache = GridCache::new();
        cache.apply(&f);
        cache
    }

    fn len(range: &std::ops::Range<LineId>) -> i64 {
        range.end.0 - range.start.0
    }

    /// The reported defect, stated as the one thing that was wrong: the range ran from the revealed
    /// line down to the *live tail*, so its size was the scroll depth.
    #[test]
    fn a_scrolled_view_asks_for_the_rows_it_reveals_not_the_history_beneath_them() {
        let cache = at_the_tail(4000, 69, 0);

        let deep = cache
            .needed_scrollback(600, &[])
            .expect("600 lines back reveals history nothing has fetched");

        assert!(
            deep.end.0 < 4000,
            "a request that reaches the live tail is the bug: it asks for {} lines to reveal 69",
            len(&deep)
        );
        assert!(
            len(&deep) <= 2 * 69,
            "the revealed rows plus one viewport of prefetch is the whole of what is needed; \
             this asked for {}",
            len(&deep)
        );
    }

    /// And the same claim where it actually bit: the cost of a gesture grew with how far it went,
    /// which is what turned a three-second scroll into fifteen seconds of daemon work.
    #[test]
    fn the_size_of_a_request_does_not_grow_with_the_depth_of_the_scroll() {
        let cache = at_the_tail(4000, 69, 0);

        let shallow = cache.needed_scrollback(100, &[]).expect("100 back");
        let deep = cache.needed_scrollback(600, &[]).expect("600 back");
        let deeper = cache.needed_scrollback(3000, &[]).expect("3000 back");

        assert_eq!(
            len(&deep),
            len(&shallow),
            "600 lines back is not 6x the work"
        );
        assert_eq!(len(&deeper), len(&shallow), "nor is 3000 lines back 30x it");
    }

    /// The other half of the storm: during a fast gesture no answer has arrived yet, so every notch
    /// computed the same un-cached run and asked for it again. A range already on its way is one
    /// this view already has coming.
    #[test]
    fn a_range_already_asked_for_is_not_asked_for_again() {
        let cache = at_the_tail(4000, 69, 0);
        let first = cache.needed_scrollback(600, &[]).expect("600 back");

        assert_eq!(
            cache.needed_scrollback(600, std::slice::from_ref(&first)),
            None,
            "the same position must not re-ask for what it just asked for"
        );
        assert_eq!(
            cache.needed_scrollback(610, std::slice::from_ref(&first)),
            None,
            "and neither must the next five notches — that is what the prefetch is for"
        );
    }

    /// When the gesture does travel past what is on its way, the next request picks up where that
    /// one stops. An overlap would re-fetch lines the previous answer is already bringing.
    #[test]
    fn the_next_request_starts_where_the_last_one_stops() {
        let cache = at_the_tail(4000, 69, 0);
        let first = cache.needed_scrollback(600, &[]).expect("600 back");

        let next = cache
            .needed_scrollback(700, std::slice::from_ref(&first))
            .expect("100 lines further back is past the prefetch");

        assert!(
            next.end <= first.start,
            "{next:?} overlaps {first:?} — those lines are already coming"
        );
    }

    /// Nothing is asked for when the revealed rows are already here. Without this the view would
    /// re-request its own scrollback on every notch of a scroll back *down* towards the tail.
    #[test]
    fn a_view_whose_revealed_rows_are_cached_asks_for_nothing() {
        let mut cache = at_the_tail(4000, 69, 0);
        let fetched: Vec<WireLine> = (3990..4000).map(|id| line(id, "old", 0)).collect();
        cache.apply_scrollback(&fetched, &[style(1)], &[]);

        assert_eq!(cache.needed_scrollback(10, &[]), None);
    }

    /// The trim watermark bounds the request from below. Asking past it is asking for lines the
    /// daemon has already dropped — served as an empty response, which would then clear the
    /// in-flight record and let the same empty request be made again on the next notch.
    #[test]
    fn a_request_never_reaches_below_the_oldest_retained_line() {
        let cache = at_the_tail(4000, 69, 3350);

        let deep = cache
            .needed_scrollback(1000, &[])
            .expect("clamped to the watermark, but still revealing un-cached lines");

        assert!(
            deep.start.0 >= 3350,
            "{deep:?} starts below the oldest line the daemon still has"
        );
    }

    #[test]
    fn full_snapshot_populates_the_screen_in_viewport_order() {
        let mut cache = GridCache::new();
        let mut f = frame(1, 0, true, 100, 3);
        f.styles = vec![style(7)];
        f.lines = vec![
            line(100, "row0", 0),
            line(101, "row1", 0),
            line(102, "row2", 0),
        ];
        cache.apply(&f);

        assert_eq!(cache.rows(), 3);
        assert_eq!(cache.cols(), 80);
        assert_eq!(cache.viewport_top(), LineId(100));
        assert_eq!(cache.seq(), 1);

        let screen = cache.screen();
        assert_eq!(screen.len(), 3);
        assert_eq!(screen[0].unwrap().text, "row0");
        assert_eq!(screen[1].unwrap().text, "row1");
        assert_eq!(screen[2].unwrap().text, "row2");

        assert_eq!(cache.line(LineId(101)).unwrap().text, "row1");
        assert!(cache.line(LineId(999)).is_none());
    }

    #[test]
    fn delta_upserts_only_changed_lines_and_keeps_the_rest() {
        let mut cache = GridCache::new();
        let mut snap = frame(1, 0, true, 0, 3);
        snap.styles = vec![style(7)];
        snap.lines = vec![line(0, "aaa", 0), line(1, "bbb", 0), line(2, "ccc", 0)];
        cache.apply(&snap);

        // Delta changes only line id 1; ids 0 and 2 are untouched.
        let mut delta = frame(2, 0, false, 0, 3);
        delta.styles = vec![style(7)];
        delta.lines = vec![line(1, "BBB", 0)];
        cache.apply(&delta);

        let screen = cache.screen();
        assert_eq!(screen[0].unwrap().text, "aaa", "untouched line survives");
        assert_eq!(screen[1].unwrap().text, "BBB", "changed line upserted");
        assert_eq!(screen[2].unwrap().text, "ccc", "untouched line survives");
        assert_eq!(cache.seq(), 2);
    }

    #[test]
    fn styles_are_resolved_and_a_later_palette_does_not_corrupt_earlier_lines() {
        let mut cache = GridCache::new();

        // A multi-run line: 2 cells of style 0 (fg 1), then 3 cells of style 1 (fg 2).
        let mut f = frame(1, 0, true, 0, 1);
        f.styles = vec![style(1), style(2)];
        f.lines = vec![WireLine {
            id: LineId(0),
            text: "ffsss".to_string(),
            runs: vec![StyleRun { len: 2, style: 0 }, StyleRun { len: 3, style: 1 }],
            extras: Vec::new(),
            wrapped: false,
        }];
        cache.apply(&f);

        let cached = cache.line(LineId(0)).unwrap().clone();
        assert_eq!(cached.runs.len(), 2);
        assert_eq!(
            cached.runs[0],
            (2, style(1)),
            "first run carries concrete style"
        );
        assert_eq!(
            cached.runs[1],
            (3, style(2)),
            "second run carries concrete style"
        );

        // A later delta on a DIFFERENT line ships a different palette where index 0/1 mean other
        // styles. The earlier cached line must keep its already-resolved styles.
        let mut g = frame(2, 0, false, 0, 1);
        g.styles = vec![style(9), style(8)];
        g.lines = vec![line(5, "xxxxx", 0)]; // style index 0 here == fg 9
        cache.apply(&g);

        let still = cache.line(LineId(0)).unwrap();
        assert_eq!(
            still.runs[0],
            (2, style(1)),
            "earlier line uncorrupted by new palette"
        );
        assert_eq!(
            still.runs[1],
            (3, style(2)),
            "earlier line uncorrupted by new palette"
        );
        assert_eq!(cache.line(LineId(5)).unwrap().runs[0], (5, style(9)));
    }

    #[test]
    fn hyperlink_index_resolves_to_the_uri_string() {
        let mut cache = GridCache::new();
        let mut f = frame(1, 0, true, 0, 1);
        f.styles = vec![style(7)];
        f.hyperlinks = vec!["https://example.com".to_string()];
        f.lines = vec![WireLine {
            id: LineId(0),
            text: "link".to_string(),
            runs: vec![StyleRun { len: 4, style: 0 }],
            extras: vec![CellExtras {
                col: 1,
                zerowidth: Vec::new(),
                hyperlink: Some(0),
            }],
            wrapped: false,
        }];
        cache.apply(&f);

        let cached = cache.line(LineId(0)).unwrap();
        assert_eq!(cached.extras.len(), 1);
        assert_eq!(cached.extras[0].col, 1);
        assert_eq!(
            cached.extras[0].hyperlink.as_deref(),
            Some("https://example.com")
        );
    }

    #[test]
    fn stale_delta_is_ignored_but_a_newer_one_applies() {
        let mut cache = GridCache::new();
        let mut snap = frame(5, 0, true, 0, 1);
        snap.styles = vec![style(7)];
        snap.lines = vec![line(0, "keep", 0)];
        cache.apply(&snap);
        assert_eq!(cache.seq(), 5);

        // Stale delta (seq <= current) is dropped.
        let mut stale = frame(5, 0, false, 0, 1);
        stale.styles = vec![style(7)];
        stale.lines = vec![line(0, "STALE", 0)];
        cache.apply(&stale);
        assert_eq!(cache.line(LineId(0)).unwrap().text, "keep");
        assert_eq!(cache.seq(), 5);

        // A newer delta applies.
        let mut fresh = frame(6, 0, false, 0, 1);
        fresh.styles = vec![style(7)];
        fresh.lines = vec![line(0, "fresh", 0)];
        cache.apply(&fresh);
        assert_eq!(cache.line(LineId(0)).unwrap().text, "fresh");
        assert_eq!(cache.seq(), 6);
    }

    #[test]
    fn generation_change_forces_a_full_reset_even_without_full_flag() {
        let mut cache = GridCache::new();
        let mut snap = frame(1, 0, true, 0, 2);
        snap.styles = vec![style(7)];
        snap.lines = vec![line(0, "old0", 0), line(1, "old1", 0)];
        cache.apply(&snap);

        // A delta (full == false) with a new generation: line ids are incomparable, so the prior
        // lines must be discarded and the frame applied as a snapshot.
        let mut regen = frame(2, 1, false, 50, 2);
        regen.styles = vec![style(7)];
        regen.lines = vec![line(50, "new0", 0), line(51, "new1", 0)];
        cache.apply(&regen);

        assert_eq!(cache.generation(), 1);
        assert!(
            cache.line(LineId(0)).is_none(),
            "old generation lines cleared"
        );
        assert!(
            cache.line(LineId(1)).is_none(),
            "old generation lines cleared"
        );
        assert_eq!(cache.screen()[0].unwrap().text, "new0");
        assert_eq!(cache.screen()[1].unwrap().text, "new1");
    }

    #[test]
    fn advancing_oldest_available_evicts_lines_below_it() {
        let mut cache = GridCache::new();
        // Snapshot with viewport at 10 but scrollback retained down to 8.
        let mut snap = frame(1, 0, true, 10, 2);
        snap.oldest_available = LineId(8);
        snap.styles = vec![style(7)];
        snap.lines = vec![
            line(8, "hist8", 0),
            line(9, "hist9", 0),
            line(10, "vis10", 0),
            line(11, "vis11", 0),
        ];
        cache.apply(&snap);
        assert!(cache.line(LineId(8)).is_some());
        assert!(cache.line(LineId(9)).is_some());

        // A delta advances the trim watermark to 10; ids 8 and 9 must be evicted.
        let mut delta = frame(2, 0, false, 10, 2);
        delta.oldest_available = LineId(10);
        delta.styles = vec![style(7)];
        cache.apply(&delta);

        assert!(
            cache.line(LineId(8)).is_none(),
            "trimmed scrollback evicted"
        );
        assert!(
            cache.line(LineId(9)).is_none(),
            "trimmed scrollback evicted"
        );
        assert!(cache.line(LineId(10)).is_some(), "visible lines retained");
        assert_eq!(cache.oldest_available(), LineId(10));
    }

    #[test]
    fn screen_has_none_for_not_yet_cached_rows() {
        let mut cache = GridCache::new();
        // A frame that reports 3 rows but only carries lines for rows 0 and 2.
        let mut f = frame(1, 0, true, 0, 3);
        f.styles = vec![style(7)];
        f.lines = vec![line(0, "top", 0), line(2, "bottom", 0)];
        cache.apply(&f);

        let screen = cache.screen();
        assert_eq!(screen.len(), 3);
        assert_eq!(screen[0].unwrap().text, "top");
        assert!(screen[1].is_none(), "uncached row is a gap");
        assert_eq!(screen[2].unwrap().text, "bottom");
    }

    #[test]
    fn apply_scrollback_resolves_history_against_its_own_palette_and_honours_the_watermark() {
        let mut cache = GridCache::new();
        // Viewport 100..102, history retained down to 90.
        let mut f = frame(1, 0, true, 100, 2);
        f.oldest_available = LineId(90);
        f.styles = vec![style(7)];
        f.lines = vec![line(100, "vis0", 0), line(101, "vis1", 0)];
        cache.apply(&f);
        assert!(cache.line(LineId(95)).is_none(), "history not fetched yet");

        // A ScrollbackResponse for 90..=99 carries its OWN palette (index 0 = fg 3, distinct from
        // the frame's fg 7) — the history must resolve against it, not the frame's.
        let styles = vec![style(3)];
        let history: Vec<WireLine> = (90..100).map(|i| line(i, &format!("h{i}"), 0)).collect();
        cache.apply_scrollback(&history, &styles, &[]);

        let l = cache.line(LineId(95)).expect("history line inserted");
        assert_eq!(l.text, "h95");
        assert_eq!(
            l.runs[0].1,
            style(3),
            "resolved against the response palette"
        );

        // A line below the trim watermark is ignored.
        cache.apply_scrollback(&[line(80, "old", 0)], &styles, &[]);
        assert!(
            cache.line(LineId(80)).is_none(),
            "a line below oldest_available is dropped"
        );
    }
}
