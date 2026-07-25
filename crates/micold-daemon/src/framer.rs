//! Shadow-diff grid framer (plan W3, tasks T032/T033/T033a).
//!
//! Turns a session's live [`Term`] grid into [`GridFrame`]s for the wire. The design is what buys
//! the streaming size win, so the rules are load-bearing, not polish:
//!
//! - **Stable line ids.** Each logical line keeps its [`LineId`] as it scrolls up into history, so a
//!   delta says "line N changed" instead of resending the screen on every scroll (the ~11×
//!   reduction, T035). Without the vendored VT patch, ids come from tracking how many lines have
//!   scrolled off the top — detected by aligning this tick's line hashes against last tick's
//!   ([`Framer::eviction_count`]). Lines only ever leave from the top and arrive at the bottom, so
//!   the alignment is a single shift.
//! - **Depth-1 dirty + fixed tick.** The caller ticks at a fixed cadence and only frames when the
//!   session's dirty flag was set; a burst of output between ticks collapses to one frame.
//! - **Snapshot on attach / resnapshot triggers.** A newly-attached client gets a `full` frame.
//!   Resize and alt-screen enter/exit bump the [`GridFrame::generation`], which forces a full frame
//!   because line identities are no longer comparable (data-model F4).
//! - **`oldest_available` on every frame.** The trim watermark ships on every frame so the client
//!   can evict its cache and clamp scrollback requests without asking (data-model I3).
//!
//! Bounded scrollback retention (T033) is a property of the VT `Term` itself: it is constructed with
//! a fixed `scrolling_history`, so oldest-first discard happens in the emulator even while detached.
//! The framer just reports the resulting `oldest_available`. Scrollback-by-range (T033a) reads any
//! retained line by id straight from the grid.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::TermMode;
use alacritty_terminal::vte::ansi::{Color as AnsiColor, CursorShape};
use micold_core::protocol::grid::{
    CellExtras, GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine,
    WireStyle,
};
use micold_core::session::SessionId;

use crate::terminal::SharedTerm;

/// Per-session grid framer. Holds the shadow of the last frame's viewport and the scroll-off
/// watermark used to keep line ids stable.
pub struct Framer {
    session: SessionId,
    /// Monotonic per-session frame sequence.
    seq: u64,
    /// Bumped on resize / alt-screen toggle — forces a full frame (line ids are incomparable).
    generation: u64,
    /// Lines discarded from the top of the buffer over the session's life (monotonic). The oldest
    /// retained line's id equals this value.
    scrolled_off: i64,
    /// Hashes of the **history** lines (above the screen), oldest-first, as of the last tick — the
    /// alignment reference for detecting how many scrolled off. History is immutable once written, so
    /// aligning on it is exact; the screen is excluded because its bottom line is edited in place
    /// (which would defeat the alignment).
    prev_history_hashes: Vec<u64>,
    /// The viewport lines last sent, `id -> content hash`, for delta computation.
    shadow: HashMap<LineId, u64>,
    /// Last viewport dimensions, to detect resize.
    last_cols: u16,
    last_rows: u16,
    /// Whether the previous frame was on the alternate screen (toggles bump the generation).
    last_alt_screen: bool,
    /// True until the first frame is produced (forces the first frame to be full).
    fresh: bool,
}

impl Framer {
    /// A framer for `session`, before any frame has been produced.
    pub fn new(session: SessionId) -> Self {
        Self {
            session,
            seq: 0,
            generation: 0,
            scrolled_off: 0,
            prev_history_hashes: Vec::new(),
            shadow: HashMap::new(),
            last_cols: 0,
            last_rows: 0,
            last_alt_screen: false,
            fresh: true,
        }
    }

    /// Produce the next frame from `term`. `force_full` requests a full snapshot (used on attach so a
    /// newly-connected client gets the whole screen). Returns a delta when nothing structural
    /// changed, else a full frame.
    pub fn frame(
        &mut self,
        term: &SharedTerm,
        force_full: bool,
        input_serial: Option<u64>,
    ) -> GridFrame {
        let term = term.lock();
        let grid = term.grid();
        let cols = grid.columns() as u16;
        let rows = grid.screen_lines() as u16;
        let history = grid.history_size();
        let mode = *term.mode();
        let alt_screen = mode.contains(TermMode::ALT_SCREEN);

        // Structural changes make prior line ids incomparable (resize / alt-screen toggle).
        let structural =
            cols != self.last_cols || rows != self.last_rows || alt_screen != self.last_alt_screen;

        // Hash the HISTORY lines (oldest-first) for eviction alignment. History line at buffer
        // offset `o` (0 = oldest) is grid `Line(o - history)`; the screen lines (offsets ≥ history)
        // are excluded because the live bottom line is edited in place, which would break alignment.
        let mut hist_hashes = Vec::with_capacity(history);
        for o in 0..history {
            hist_hashes.push(hash_row(&grid[Line(o as i32 - history as i32)], cols));
        }

        // Advance the scroll-off watermark by however many history lines left the top since last
        // tick. A structural change re-bases ids, so it drops the alignment reference instead.
        let mut resnapshot = structural;
        if structural {
            self.prev_history_hashes.clear();
        }
        match Self::eviction_count(&self.prev_history_hashes, &hist_hashes) {
            Some(evicted) => self.scrolled_off += evicted as i64,
            // History failed to align (e.g. a screen-clearing reset wiped it): resnapshot rather
            // than risk mis-keying. Ids stay monotonic; the client drops its cache on the bump.
            None => resnapshot = true,
        }
        if resnapshot && !self.fresh {
            self.generation += 1;
        }

        let full = force_full || resnapshot || self.fresh;
        let oldest_available = LineId(self.scrolled_off);
        let viewport_top = LineId(self.scrolled_off + history as i64);

        // Build the viewport lines (screen rows 0..rows), interning styles per frame.
        let mut styles: Vec<WireStyle> = Vec::new();
        let mut style_index: HashMap<WireStyle, u16> = HashMap::new();
        let mut hyperlinks: Vec<String> = Vec::new();
        let mut link_index: HashMap<String, u16> = HashMap::new();

        let mut lines: Vec<WireLine> = Vec::new();
        let mut new_shadow: HashMap<LineId, u64> = HashMap::new();
        for row in 0..rows {
            let id = LineId(viewport_top.0 + row as i64);
            let grid_row = &grid[Line(row as i32)];
            let hash = hash_row(grid_row, cols);
            new_shadow.insert(id, hash);

            let changed = full || self.shadow.get(&id) != Some(&hash);
            if !changed {
                continue;
            }
            lines.push(build_wire_line(
                id,
                grid_row,
                cols,
                &mut styles,
                &mut style_index,
                &mut hyperlinks,
                &mut link_index,
            ));
        }

        let cursor = build_cursor(&term, viewport_top, rows);

        // Commit state for the next tick.
        self.shadow = new_shadow;
        self.prev_history_hashes = hist_hashes;
        self.last_cols = cols;
        self.last_rows = rows;
        self.last_alt_screen = alt_screen;
        self.fresh = false;
        self.seq += 1;

        GridFrame {
            session: self.session,
            seq: self.seq,
            generation: self.generation,
            full,
            viewport_top,
            oldest_available,
            cols,
            rows,
            cursor,
            styles,
            hyperlinks,
            lines,
            mode: mode.bits(),
            input_serial,
        }
    }

    /// Read a contiguous range of retained lines by id, for a client scrolling into history without
    /// holding all of it (T033a, FR-017). Clamps `from` up to `oldest_available` rather than
    /// erroring, and returns fewer than `count` lines near the live edge (advisory). The bool is
    /// `more`: whether older lines than `from` remain retained.
    #[allow(clippy::type_complexity)]
    pub fn scrollback_range(
        &self,
        term: &SharedTerm,
        from: LineId,
        count: usize,
    ) -> (Vec<WireLine>, Vec<WireStyle>, Vec<String>, bool) {
        let term = term.lock();
        let grid = term.grid();
        let cols = grid.columns() as u16;
        let history = grid.history_size();
        let rows = grid.screen_lines();
        let retained = history + rows;

        // Clamp the start up to the oldest retained line (never error past the watermark).
        let oldest = self.scrolled_off;
        let start_id = from.0.max(oldest);
        let start_offset = (start_id - oldest).max(0) as usize;
        if start_offset >= retained {
            return (Vec::new(), Vec::new(), Vec::new(), false);
        }
        let end_offset = (start_offset + count).min(retained);

        let mut styles: Vec<WireStyle> = Vec::new();
        let mut style_index: HashMap<WireStyle, u16> = HashMap::new();
        let mut hyperlinks: Vec<String> = Vec::new();
        let mut link_index: HashMap<String, u16> = HashMap::new();

        let mut lines = Vec::with_capacity(end_offset - start_offset);
        for o in start_offset..end_offset {
            let id = LineId(oldest + o as i64);
            let line = Line(o as i32 - history as i32);
            lines.push(build_wire_line(
                id,
                &grid[line],
                cols,
                &mut styles,
                &mut style_index,
                &mut hyperlinks,
                &mut link_index,
            ));
        }
        // `more` is true when there is retained content older than the returned range.
        let more = start_offset > 0;
        (lines, styles, hyperlinks, more)
    }

    /// The oldest retained line's id (the eviction watermark). Equals `scrolled_off`.
    pub fn oldest_available(&self) -> LineId {
        LineId(self.scrolled_off)
    }

    /// The newest retained line's id, given the session's current term.
    pub fn newest(&self, term: &SharedTerm) -> LineId {
        let term = term.lock();
        let grid = term.grid();
        let retained = grid.history_size() + grid.screen_lines();
        LineId(self.scrolled_off + retained as i64 - 1)
    }

    /// How many lines left the top of the buffer between `prev` and `curr`. Lines only ever leave
    /// from the top and arrive at the bottom, so the new buffer is the old one shifted by some `e`:
    /// `curr[0..k] == prev[e..e+k]`. Returns the smallest such `e`, or `None` when no shift aligns
    /// (a full redraw), signalling a resnapshot.
    fn eviction_count(prev: &[u64], curr: &[u64]) -> Option<usize> {
        if prev.is_empty() {
            return Some(0);
        }
        for e in 0..=prev.len() {
            let overlap = (prev.len() - e).min(curr.len());
            if overlap == 0 {
                // Everything retained was evicted; only consistent if we shifted past the end.
                return Some(e);
            }
            if prev[e..e + overlap] == curr[..overlap] {
                return Some(e);
            }
        }
        None
    }
}

/// A stable content hash of one grid row (char + style per cell). Style is included so a colour-only
/// change still resends the line, and so eviction alignment is not fooled by same-text/different-style.
fn hash_row(row: &alacritty_terminal::grid::Row<Cell>, cols: u16) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for col in 0..cols as usize {
        let cell = &row[Column(col)];
        cell.c.hash(&mut h);
        style_key(cell).hash(&mut h);
    }
    h.finish()
}

/// A compact, hashable key for a cell's style (fg, bg, flags, underline color) — enough to detect
/// every visible change the wire style carries. The underline colour must be included: it is part of
/// [`WireStyle`], so omitting it here would let an underline-colour-only change slip past the
/// shadow-diff and leave the client rendering a stale colour. `0` is a safe "no override" sentinel
/// because [`color_key`] never returns `0` (every variant sets a high tag byte).
fn style_key(cell: &Cell) -> (u64, u64, u16, u64) {
    let underline = cell.underline_color().map(color_key).unwrap_or(0);
    (
        color_key(cell.fg),
        color_key(cell.bg),
        cell.flags.bits(),
        underline,
    )
}

fn color_key(c: AnsiColor) -> u64 {
    match c {
        AnsiColor::Named(n) => 0x01_00_00_00_00 | n as u64,
        AnsiColor::Indexed(i) => 0x02_00_00_00_00 | i as u64,
        AnsiColor::Spec(rgb) => {
            0x03_00_00_00_00 | ((rgb.r as u64) << 16) | ((rgb.g as u64) << 8) | rgb.b as u64
        }
    }
}

fn wire_color(c: AnsiColor) -> WireColor {
    match c {
        // The discriminant rides the wire verbatim; the client decodes it against the same
        // `NamedColor` enum. `as u16` (not `u8`) because the specials go up to 268 — truncating
        // `Background` (257) to a `u8` yielded 1 = ANSI red, painting every default cell red.
        AnsiColor::Named(n) => WireColor::Named(n as u16),
        AnsiColor::Indexed(i) => WireColor::Indexed(i),
        AnsiColor::Spec(rgb) => WireColor::Rgb(rgb.r, rgb.g, rgb.b),
    }
}

/// Intern a style into the frame palette, returning its index (rule 2 — interned per frame).
fn intern_style(
    style: WireStyle,
    styles: &mut Vec<WireStyle>,
    index: &mut HashMap<WireStyle, u16>,
) -> u16 {
    if let Some(&i) = index.get(&style) {
        return i;
    }
    let i = styles.len() as u16;
    styles.push(style);
    index.insert(style, i);
    i
}

/// Convert one grid row to a [`WireLine`]: one `char` per cell, RLE style runs, and sparse extras
/// (zerowidth marks + hyperlinks) hoisted out of the hot path.
fn build_wire_line(
    id: LineId,
    row: &alacritty_terminal::grid::Row<Cell>,
    cols: u16,
    styles: &mut Vec<WireStyle>,
    style_index: &mut HashMap<WireStyle, u16>,
    hyperlinks: &mut Vec<String>,
    link_index: &mut HashMap<String, u16>,
) -> WireLine {
    let mut text = String::with_capacity(cols as usize);
    let mut runs: Vec<StyleRun> = Vec::new();
    let mut extras: Vec<CellExtras> = Vec::new();
    let mut wrapped = false;

    let mut run_style: Option<u16> = None;
    let mut run_len: u16 = 0;

    for col in 0..cols as usize {
        let cell = &row[Column(col)];
        text.push(cell.c);

        let ul = cell.underline_color().map(wire_color);
        let style = WireStyle {
            fg: wire_color(cell.fg),
            bg: wire_color(cell.bg),
            flags: cell.flags.bits(),
            underline_color: ul,
        };
        let idx = intern_style(style, styles, style_index);
        match run_style {
            Some(prev) if prev == idx => run_len += 1,
            Some(prev) => {
                runs.push(StyleRun {
                    len: run_len,
                    style: prev,
                });
                run_style = Some(idx);
                run_len = 1;
            }
            None => {
                run_style = Some(idx);
                run_len = 1;
            }
        }

        // Sparse per-cell data: combining marks and hyperlinks only.
        let zerowidth = cell.zerowidth().map(<[char]>::to_vec).unwrap_or_default();
        let hyperlink = cell.hyperlink().map(|link| {
            let uri = link.uri().to_string();
            if let Some(&i) = link_index.get(&uri) {
                i
            } else {
                let i = hyperlinks.len() as u16;
                link_index.insert(uri.clone(), i);
                hyperlinks.push(uri);
                i
            }
        });
        if !zerowidth.is_empty() || hyperlink.is_some() {
            extras.push(CellExtras {
                col: col as u16,
                zerowidth,
                hyperlink,
            });
        }

        if col + 1 == cols as usize {
            wrapped = cell.flags.contains(Flags::WRAPLINE);
        }
    }
    if let Some(prev) = run_style {
        runs.push(StyleRun {
            len: run_len,
            style: prev,
        });
    }

    WireLine {
        id,
        text,
        runs,
        extras,
        wrapped,
    }
}

/// Build the wire cursor, anchoring its line to a [`LineId`] so it survives scrolling.
fn build_cursor(
    term: &alacritty_terminal::term::Term<crate::terminal::DaemonListener>,
    viewport_top: LineId,
    rows: u16,
) -> WireCursor {
    let content = term.renderable_content();
    let Point { line, column } = content.cursor.point;
    // The daemon frames the live screen (display_offset 0), so the cursor line is within the
    // viewport; clamp defensively.
    let row = line.0.clamp(0, rows.saturating_sub(1) as i32) as i64;
    let mode = content.mode;
    WireCursor {
        line: LineId(viewport_top.0 + row),
        col: column.0 as u16,
        shape: wire_cursor_shape(content.cursor.shape),
        visible: mode.contains(TermMode::SHOW_CURSOR),
        // vte 0.15 exposes no cursor-blink terminal mode; the client decides blink cadence.
        blinking: false,
    }
}

fn wire_cursor_shape(shape: CursorShape) -> WireCursorShape {
    match shape {
        CursorShape::Block => WireCursorShape::Block,
        CursorShape::Underline => WireCursorShape::Underline,
        CursorShape::Beam => WireCursorShape::Beam,
        CursorShape::HollowBlock => WireCursorShape::HollowBlock,
        CursorShape::Hidden => WireCursorShape::Hidden,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::NamedColor;

    // Regression: the specials (`Foreground` = 256, `Background` = 257) must ride the wire as their
    // real discriminant, not a `u8`-truncated one. Truncation turned every default-background cell
    // into `Named(1)` = ANSI red, so the whole terminal rendered black-on-red (the client decodes
    // `Named(n)` against this same enum).
    #[test]
    fn named_specials_are_not_truncated() {
        assert_eq!(
            wire_color(AnsiColor::Named(NamedColor::Background)),
            WireColor::Named(NamedColor::Background as u16),
        );
        assert_eq!(
            wire_color(AnsiColor::Named(NamedColor::Foreground)),
            WireColor::Named(NamedColor::Foreground as u16),
        );
        // The value that was corrupted: Background must NOT collapse onto ANSI red (index 1).
        assert_ne!(
            wire_color(AnsiColor::Named(NamedColor::Background)),
            WireColor::Named(NamedColor::Red as u16),
        );
    }

    // The ANSI-16 names (0..=15) fit a `u8` and must be unchanged by the widening.
    #[test]
    fn ansi16_names_round_trip() {
        for named in [NamedColor::Black, NamedColor::Red, NamedColor::BrightWhite] {
            assert_eq!(
                wire_color(AnsiColor::Named(named)),
                WireColor::Named(named as u16),
            );
        }
    }
}
