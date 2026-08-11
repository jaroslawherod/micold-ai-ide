//! Client-side text selection anchored to absolute line identities ([`LineId`]).
//!
//! Selection is pure, client-owned state (spec 010 FR-018): because both ends are anchored to
//! stable, absolute [`LineId`]s rather than viewport rows, the selection is invariant under
//! scrolling and new output — a run of fresh lines scrolling in never shifts, grows, or corrupts an
//! in-progress or held selection. The renderer asks [`Selection::contains`] "is this cell
//! selected?" and the app asks [`Selection::text`] "give me the selected text".
//!
//! This module deliberately does **not** own the grid. Text-dependent operations (word expansion,
//! text extraction) take a **line-text provider** closure `impl Fn(LineId) -> Option<String>` that
//! yields a line's display text (one `char` per cell), so the module stays decoupled from the grid
//! cache's concrete type. A `None` from the provider is treated as an empty line.

use crate::features::Outcome;
use micold_core::protocol::grid::LineId;

/// A single selection endpoint: an absolute line identity plus a cell column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Anchor {
    /// The stable, absolute line this endpoint sits on.
    pub line: LineId,
    /// The cell column within that line.
    pub col: u16,
}

impl Anchor {
    /// Convenience constructor.
    pub fn new(line: LineId, col: u16) -> Self {
        Self { line, col }
    }
}

/// Selection granularity, mirroring the three mouse gestures the app already distinguishes
/// (single = char, double = word, triple = line). Defined locally to keep this module decoupled
/// from `app`'s `SelectKind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectGranularity {
    /// Character-range selection.
    Char,
    /// Word (maximal run) selection — expands each end to word boundaries.
    Word,
    /// Whole-line selection — column 0 to the end of the line's text.
    Line,
}

/// A text selection anchored to absolute line coordinates.
///
/// Stores the raw click ("fixed") anchor, the raw moving anchor, the granularity, and the
/// normalized + granularity-expanded inclusive bounds. Because the bounds are keyed by [`LineId`],
/// [`contains`](Self::contains) and [`text`](Self::text) are unaffected by the appearance of new,
/// higher-`LineId` output (FR-018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    /// The fixed anchor (where the drag began), raw/unexpanded.
    start: Anchor,
    /// The moving anchor (last extended-to cell), raw/unexpanded.
    current: Anchor,
    /// How the raw anchors are expanded into bounds.
    granularity: SelectGranularity,
    /// Cached normalized, expanded, inclusive bounds `(top_left, bottom_right)`.
    bounds: (Anchor, Anchor),
}

impl Selection {
    /// Begin a selection at `anchor` with the given granularity. `Word`/`Line` expand their bounds
    /// immediately using the provider.
    pub fn start(
        anchor: Anchor,
        granularity: SelectGranularity,
        line_text: impl Fn(LineId) -> Option<String>,
    ) -> Self {
        let bounds = expand(anchor, anchor, granularity, &line_text);
        Self {
            start: anchor,
            current: anchor,
            granularity,
            bounds,
        }
    }

    /// Extend the moving end of the selection to `anchor`, re-expanding for `Word`/`Line`. The
    /// fixed (start) anchor is unchanged, so dragging back and forth is stable.
    pub fn update(&mut self, anchor: Anchor, line_text: impl Fn(LineId) -> Option<String>) {
        self.current = anchor;
        self.bounds = expand(self.start, anchor, self.granularity, &line_text);
    }

    /// The granularity this selection was started with.
    pub fn granularity(&self) -> SelectGranularity {
        self.granularity
    }

    /// The normalized, inclusive bounds as `(top_left, bottom_right)`, regardless of drag
    /// direction. Both endpoints denote selected cells (inclusive on both ends).
    pub fn bounds(&self) -> (Anchor, Anchor) {
        self.bounds
    }

    /// Whether the cell at `(line, col)` falls inside the selection (for render highlighting).
    ///
    /// Uses standard terminal semantics: a multi-line selection covers the first line from its
    /// start column to end, every intermediate line in full, and the last line from its start to
    /// the end column. Both endpoint columns are inclusive.
    pub fn contains(&self, line: LineId, col: u16) -> bool {
        let (top, bot) = self.bounds;
        if line < top.line || line > bot.line {
            return false;
        }
        if top.line == bot.line {
            return col >= top.col && col <= bot.col;
        }
        if line == top.line {
            return col >= top.col;
        }
        if line == bot.line {
            return col <= bot.col;
        }
        // Strictly-intermediate line: fully selected.
        true
    }

    /// Extract the selected text via the provider. Multi-line selections join with `'\n'`; each
    /// line's trailing whitespace is trimmed (standard terminal copy behavior). Columns are
    /// respected: the first line starts at the start column, the last ends at the end column, and
    /// intermediate lines are taken in full.
    pub fn text(&self, line_text: impl Fn(LineId) -> Option<String>) -> String {
        let (top, bot) = self.bounds;
        let mut out = String::new();
        let mut line = top.line;
        let mut first = true;
        while line <= bot.line {
            let chars = line_chars(&line_text, line);
            let start_col = if line == top.line {
                top.col as usize
            } else {
                0
            };
            let end_col = if line == bot.line {
                bot.col as usize
            } else {
                chars.len().saturating_sub(1)
            };

            let segment: String = if start_col >= chars.len() {
                String::new()
            } else {
                let end = (end_col + 1).min(chars.len());
                if start_col >= end {
                    String::new()
                } else {
                    chars[start_col..end].iter().collect()
                }
            };

            if !first {
                out.push('\n');
            }
            out.push_str(segment.trim_end());
            first = false;

            line = LineId(line.0 + 1);
        }
        out
    }
}

/// What copying the current selection asks of the clipboard, or nothing to do (feature 021, T045 —
/// FR-015a, contract C2).
///
/// The decision lives here, with the selection: whether there *is* a selection, and whether it
/// resolves to text worth writing. Before this the terminal's copy action made both calls inline in
/// the shell, one line above the `iced::clipboard::write` it guarded — which is the arrangement
/// FR-017 objects to, since it puts a feature's rule where nothing can reach it without a window.
///
/// Returning [`Outcome::ClipboardWrite`] rather than performing the write is FR-015a: the operation
/// returns a deferred task rather than a value, so it cannot be a synchronous service capability,
/// and an effect request the shell interprets is the sanctioned alternative.
pub fn copy_request(
    selection: Option<&Selection>,
    line_text: impl Fn(LineId) -> Option<String>,
) -> Option<Outcome> {
    let text = selection?.text(line_text);
    (!text.is_empty()).then_some(Outcome::ClipboardWrite(text))
}

/// The line's display text as a per-cell `char` vector; a missing line is an empty line.
fn line_chars<F: Fn(LineId) -> Option<String>>(provider: &F, line: LineId) -> Vec<char> {
    provider(line).unwrap_or_default().chars().collect()
}

/// The inclusive `(start_col, end_col)` of the word containing `col`.
///
/// A "word" is the maximal run of cells sharing the clicked cell's whitespace class — i.e. clicking
/// a non-whitespace cell selects the maximal run of non-whitespace, and clicking whitespace selects
/// the maximal run of whitespace. A column past the end of the line's text (or a missing line)
/// expands to just that single cell.
fn word_bounds<F: Fn(LineId) -> Option<String>>(
    provider: &F,
    line: LineId,
    col: u16,
) -> (u16, u16) {
    let chars = line_chars(provider, line);
    let idx = col as usize;
    if idx >= chars.len() {
        return (col, col);
    }
    let ws = chars[idx].is_whitespace();
    let mut start = idx;
    while start > 0 && chars[start - 1].is_whitespace() == ws {
        start -= 1;
    }
    let mut end = idx;
    while end + 1 < chars.len() && chars[end + 1].is_whitespace() == ws {
        end += 1;
    }
    (start as u16, end as u16)
}

/// The last cell column of a line (`0` for an empty/missing line).
fn line_last_col<F: Fn(LineId) -> Option<String>>(provider: &F, line: LineId) -> u16 {
    line_chars(provider, line).len().saturating_sub(1) as u16
}

/// Normalize two anchors into `(top_left, bottom_right)` order (line-major, then column).
fn normalized(a: Anchor, b: Anchor) -> (Anchor, Anchor) {
    if (a.line, a.col) <= (b.line, b.col) {
        (a, b)
    } else {
        (b, a)
    }
}

/// Normalize the raw anchors and expand them per granularity into inclusive bounds.
fn expand<F: Fn(LineId) -> Option<String>>(
    start: Anchor,
    current: Anchor,
    granularity: SelectGranularity,
    provider: &F,
) -> (Anchor, Anchor) {
    let (mut top, mut bot) = normalized(start, current);
    match granularity {
        SelectGranularity::Char => {}
        SelectGranularity::Word => {
            // Extend the top-left end leftward to its word start and the bottom-right end
            // rightward to its word end.
            let (word_start, _) = word_bounds(provider, top.line, top.col);
            let (_, word_end) = word_bounds(provider, bot.line, bot.col);
            top.col = word_start;
            bot.col = word_end;
        }
        SelectGranularity::Line => {
            top.col = 0;
            bot.col = line_last_col(provider, bot.line);
        }
    }
    (top, bot)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a provider closure from `(LineId raw, text)` pairs; unknown lines return `None`.
    fn provider(lines: &[(i64, &str)]) -> impl Fn(LineId) -> Option<String> {
        let owned: Vec<(i64, String)> = lines.iter().map(|(n, t)| (*n, t.to_string())).collect();
        move |id: LineId| {
            owned
                .iter()
                .find(|(n, _)| *n == id.0)
                .map(|(_, t)| t.clone())
        }
    }

    fn a(line: i64, col: u16) -> Anchor {
        Anchor::new(LineId(line), col)
    }

    #[test]
    fn char_selection_spans_multiple_lines() {
        let p = provider(&[
            (100, "hello world"),
            (101, "second line here"),
            (102, "third"),
        ]);
        let mut sel = Selection::start(a(100, 6), SelectGranularity::Char, &p);
        sel.update(a(102, 2), &p);

        // First (partial) line: from the start col to the end.
        assert!(sel.contains(LineId(100), 6));
        assert!(sel.contains(LineId(100), 10));
        assert!(!sel.contains(LineId(100), 5));

        // Middle line: fully selected.
        assert!(sel.contains(LineId(101), 0));
        assert!(sel.contains(LineId(101), 15));

        // Last (partial) line: up to and including the end col.
        assert!(sel.contains(LineId(102), 0));
        assert!(sel.contains(LineId(102), 2));
        assert!(!sel.contains(LineId(102), 3));

        // Outside the line range.
        assert!(!sel.contains(LineId(99), 0));
        assert!(!sel.contains(LineId(103), 0));
    }

    #[test]
    fn direction_independent() {
        let p = provider(&[
            (100, "hello world"),
            (101, "second line here"),
            (102, "third"),
        ]);
        let forward = {
            let mut s = Selection::start(a(100, 6), SelectGranularity::Char, &p);
            s.update(a(102, 2), &p);
            s
        };
        let backward = {
            let mut s = Selection::start(a(102, 2), SelectGranularity::Char, &p);
            s.update(a(100, 6), &p);
            s
        };

        assert_eq!(forward.bounds(), backward.bounds());
        for &(line, col) in &[(99, 0), (100, 6), (100, 5), (101, 3), (102, 2), (102, 3)] {
            assert_eq!(
                forward.contains(LineId(line), col),
                backward.contains(LineId(line), col),
                "mismatch at ({line}, {col})"
            );
        }
    }

    #[test]
    fn fr018_invariant_under_new_output() {
        // Selection made over lines 100..=102.
        let before = provider(&[
            (100, "hello world"),
            (101, "second line"),
            (102, "third row"),
        ]);
        let mut sel = Selection::start(a(100, 6), SelectGranularity::Char, &before);
        sel.update(a(102, 4), &before);

        let text_before = sel.text(&before);

        // New output scrolls in: lines 103.. now exist. The provider gains higher-LineId lines.
        let after = provider(&[
            (100, "hello world"),
            (101, "second line"),
            (102, "third row"),
            (103, "brand new output"),
            (104, "even newer"),
        ]);

        // contains is anchored to LineIds — untouched by the new lines.
        assert!(sel.contains(LineId(100), 6));
        assert!(sel.contains(LineId(101), 0));
        assert!(sel.contains(LineId(102), 4));
        assert!(!sel.contains(LineId(103), 0));

        // text extraction reads the same cells and yields identical output.
        assert_eq!(sel.text(&after), text_before);
    }

    #[test]
    fn text_extraction_joins_and_trims() {
        let p = provider(&[
            (100, "hello world   "), // trailing whitespace
            (101, "mid"),
            (102, "third"),
        ]);
        let mut sel = Selection::start(a(100, 6), SelectGranularity::Char, &p);
        sel.update(a(102, 2), &p);

        // Line 100: "world   " -> trimmed "world"; line 101 full; line 102 cols 0..=2 "thi".
        assert_eq!(sel.text(&p), "world\nmid\nthi");
    }

    #[test]
    fn single_line_text_has_no_newline() {
        let p = provider(&[(100, "hello world")]);
        let mut sel = Selection::start(a(100, 0), SelectGranularity::Char, &p);
        sel.update(a(100, 4), &p);
        assert_eq!(sel.text(&p), "hello");
    }

    #[test]
    fn word_expansion_selects_whole_word() {
        let p = provider(&[(200, "foo bar baz")]);
        // Column 5 is the 'a' in "bar" (cols 4..=6).
        let sel = Selection::start(a(200, 5), SelectGranularity::Word, &p);
        assert_eq!(sel.bounds(), (a(200, 4), a(200, 6)));
        assert!(!sel.contains(LineId(200), 3)); // the space before
        assert!(sel.contains(LineId(200), 4));
        assert!(sel.contains(LineId(200), 6));
        assert!(!sel.contains(LineId(200), 7)); // the space after
        assert_eq!(sel.text(&p), "bar");
    }

    #[test]
    fn word_expansion_re_expands_on_update() {
        let p = provider(&[(200, "foo bar baz")]);
        // Start inside "foo", drag to inside "baz": bounds cover foo-start..baz-end.
        let mut sel = Selection::start(a(200, 1), SelectGranularity::Word, &p);
        sel.update(a(200, 9), &p);
        assert_eq!(sel.bounds(), (a(200, 0), a(200, 10)));
        assert_eq!(sel.text(&p), "foo bar baz");
    }

    #[test]
    fn line_granularity_selects_whole_line() {
        let p = provider(&[(200, "foo bar baz")]);
        let sel = Selection::start(a(200, 5), SelectGranularity::Line, &p);
        assert_eq!(sel.bounds(), (a(200, 0), a(200, 10)));
        assert!(sel.contains(LineId(200), 0));
        assert!(sel.contains(LineId(200), 10));
        assert_eq!(sel.text(&p), "foo bar baz");
    }

    #[test]
    fn degenerate_missing_and_empty_lines_do_not_panic() {
        // Provider knows nothing: every line is treated as empty.
        let empty = provider(&[]);
        let sel = Selection::start(a(300, 5), SelectGranularity::Word, &empty);
        // Single-cell word on a missing line; no panic.
        assert!(sel.contains(LineId(300), 5));
        assert_eq!(sel.text(&empty), "");

        // Line granularity over an empty line collapses to col 0.
        let with_empty = provider(&[(300, "")]);
        let line_sel = Selection::start(a(300, 0), SelectGranularity::Line, &with_empty);
        assert_eq!(line_sel.bounds(), (a(300, 0), a(300, 0)));
        assert_eq!(line_sel.text(&with_empty), "");

        // Char selection whose start col is past the line's text extracts nothing for that line.
        let short = provider(&[(300, "hi")]);
        let mut past = Selection::start(a(300, 5), SelectGranularity::Char, &short);
        past.update(a(300, 9), &short);
        assert_eq!(past.text(&short), "");
    }
}
