//! A selection of transcript text (feature 1550, `specs/1550-transcript-text-selection/contracts/selection-model.md`).
//!
//! Render-free: a position names a unit of shown text by the row it is drawn in, its place in that
//! row and a byte offset into its shown text, and positions order in reading order (S-1). The
//! client maps hits to positions and positions to highlighted ranges; nothing here lays out text.

use std::ops::Range;

use unicode_segmentation::{GraphemeCursor, UnicodeSegmentation};

use crate::structured::transcript::EntryId;

/// A unit's place inside its row: segments assigned in drawing order (block, list item, table row,
/// cell, line …). Lexicographic order is drawing order: `[1] < [1, 0] < [2]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextPath(Vec<u16>);

impl TextPath {
    pub fn new(segments: impl Into<Vec<u16>>) -> Self {
        Self(segments.into())
    }

    pub fn segments(&self) -> &[u16] {
        &self.0
    }
}

/// What part of an entry's rows a unit is in: the entry's own units, then the `k`-th request
/// block or resolution row drawn under its card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Under {
    Body,
    Prompt(u32),
}

/// The transcript row a unit is drawn in. The derived order is the drawing order: the omitted
/// notice, each entry (its body, then what is drawn under it), then the request blocks at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RowPos {
    Omitted,
    Entry(EntryId, Under),
    End(u32),
}

/// A place in the shown text: the derived order (row, then path, then offset) is reading order.
/// `offset` is a byte offset into the unit's shown text, on a grapheme boundary (S-4): build a
/// position from a hit with [`TextPos::in_text`], which snaps it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TextPos {
    pub row: RowPos,
    pub path: TextPath,
    pub offset: usize,
}

impl TextPos {
    /// The position at `offset` in the unit whose shown text is `text`, snapped to a grapheme
    /// boundary.
    pub fn in_text(row: RowPos, path: TextPath, text: &str, offset: usize) -> Self {
        Self {
            row,
            path,
            offset: snap(text, offset),
        }
    }
}

/// The unit a selection grows by: from the click count (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Granularity {
    Char,
    Word,
    Line,
}

/// One end of a selection: where it is, and the word or line around it the granularity expands
/// it to (`at.offset..at.offset` for [`Granularity::Char`]).
#[derive(Debug, Clone, PartialEq, Eq)]
struct End {
    at: TextPos,
    around: Range<usize>,
}

impl End {
    fn new(at: TextPos, granularity: Granularity, expand: Range<usize>) -> Self {
        let around = match granularity {
            Granularity::Char => at.offset..at.offset,
            Granularity::Word | Granularity::Line => {
                expand.start.min(at.offset)..expand.end.max(at.offset)
            }
        };
        Self { at, around }
    }

    fn start(&self) -> TextPos {
        TextPos {
            offset: self.around.start,
            ..self.at.clone()
        }
    }

    fn end(&self) -> TextPos {
        TextPos {
            offset: self.around.end,
            ..self.at.clone()
        }
    }
}

/// A selection: where the gesture started (`anchor`) and where it is now (`focus`). An empty
/// selection is a value ([`Selection::is_empty`]), not a separate flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selection {
    anchor: End,
    focus: End,
    granularity: Granularity,
    dragging: bool,
}

impl Selection {
    /// A selection started at `at`, dragging. `expand` is the word or line around `at` for
    /// [`Granularity::Word`] and [`Granularity::Line`], ignored for [`Granularity::Char`].
    pub fn start(at: TextPos, granularity: Granularity, expand: Range<usize>) -> Self {
        let anchor = End::new(at, granularity, expand);
        Self {
            focus: anchor.clone(),
            anchor,
            granularity,
            dragging: true,
        }
    }

    /// Moves the focus to `to`, expanded by the selection's granularity with `expand`.
    pub fn extend(&mut self, to: TextPos, expand: Range<usize>) {
        self.focus = End::new(to, self.granularity, expand);
    }

    /// The gesture ended; the selection is kept.
    pub fn release(&mut self) {
        self.dragging = false;
    }

    pub fn dragging(&self) -> bool {
        self.dragging
    }

    pub fn granularity(&self) -> Granularity {
        self.granularity
    }

    pub fn anchor(&self) -> &TextPos {
        &self.anchor.at
    }

    pub fn focus(&self) -> &TextPos {
        &self.focus.at
    }

    /// Nothing is selected: the expanded start is the end.
    pub fn is_empty(&self) -> bool {
        let (start, end) = self.bounds();
        start == end
    }

    /// The selected range, normalised (start before end) and expanded by the granularity.
    pub fn bounds(&self) -> (TextPos, TextPos) {
        // Each end covers its word or line; the selection runs from the earlier start to the
        // later end, whichever end the gesture started at.
        let start = self.anchor.start().min(self.focus.start());
        let end = self.anchor.end().max(self.focus.end());
        (start, end)
    }

    /// The part of the selection inside `row`, `None` when none is (or nothing is selected).
    pub fn in_row(&self, row: &RowPos) -> Option<RowSelection> {
        let (start, end) = self.bounds();
        if start == end || *row < start.row || *row > end.row {
            return None;
        }
        Some(match (*row == start.row, *row == end.row) {
            (true, true) => {
                RowSelection::Within((start.path, start.offset), (end.path, end.offset))
            }
            (true, false) => RowSelection::From(start.path, start.offset),
            (false, true) => RowSelection::To(end.path, end.offset),
            (false, false) => RowSelection::Whole,
        })
    }
}

/// The part of a selection inside one row: all of it, from a place to its end, from its start to
/// a place, or between two places. Part of an entry's `lazy` key, so `Hash` + `Eq`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RowSelection {
    Whole,
    From(TextPath, usize),
    To(TextPath, usize),
    Within((TextPath, usize), (TextPath, usize)),
}

/// The selected byte range of the unit at `path` whose shown text is `len` bytes long, `None`
/// when none of it is selected.
pub fn range_in(part: &RowSelection, path: &TextPath, len: usize) -> Option<Range<usize>> {
    let from = |p: &TextPath, offset: usize| match path.cmp(p) {
        std::cmp::Ordering::Less => None,
        std::cmp::Ordering::Equal => Some(offset),
        std::cmp::Ordering::Greater => Some(0),
    };
    let to = |p: &TextPath, offset: usize| match path.cmp(p) {
        std::cmp::Ordering::Less => Some(len),
        std::cmp::Ordering::Equal => Some(offset),
        std::cmp::Ordering::Greater => None,
    };
    let (start, end) = match part {
        RowSelection::Whole => (0, len),
        RowSelection::From(p, offset) => (from(p, *offset)?, len),
        RowSelection::To(p, offset) => (0, to(p, *offset)?),
        RowSelection::Within((p, a), (q, b)) => (from(p, *a)?, to(q, *b)?),
    };
    let (start, end) = (start.min(len), end.min(len));
    (start < end).then_some(start..end)
}

/// The grapheme boundary of `text` nearest `offset` (the earlier on a tie), `text.len()` past
/// the end (S-4).
pub fn snap(text: &str, offset: usize) -> usize {
    if offset >= text.len() {
        return text.len();
    }
    // A grapheme boundary is a char boundary: look from the char `offset` falls in, so only the
    // clusters around it are read, whatever the text's length.
    let floor = (0..=offset)
        .rev()
        .find(|&i| text.is_char_boundary(i))
        .unwrap_or(0);
    let cursor = || GraphemeCursor::new(floor, text.len(), true);
    let on_boundary = cursor().is_boundary(text, 0).unwrap_or(true);
    if on_boundary && floor == offset {
        return offset;
    }
    let before = if on_boundary {
        floor
    } else {
        cursor().prev_boundary(text, 0).ok().flatten().unwrap_or(0)
    };
    let after = cursor()
        .next_boundary(text, 0)
        .ok()
        .flatten()
        .unwrap_or(text.len());
    if offset - before <= after - offset {
        before
    } else {
        after
    }
}

/// What a run of `text` between Unicode word bounds is, for joining runs into what a
/// double-click selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Run {
    Word,
    Space,
    Punctuation,
}

fn run_of(piece: &str) -> Run {
    if piece.chars().all(char::is_whitespace) {
        Run::Space
    } else if piece.chars().all(is_punctuation) {
        Run::Punctuation
    } else {
        Run::Word
    }
}

/// ASCII and Latin-1 punctuation, general punctuation (dashes, quotes, ellipsis), CJK and
/// full-width punctuation.
fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(
            c,
            '\u{A1}' | '\u{A7}' | '\u{AB}' | '\u{B6}' | '\u{B7}' | '\u{BB}' | '\u{BF}'
        )
        || ('\u{2010}'..='\u{2027}').contains(&c)
        || ('\u{2030}'..='\u{205E}').contains(&c)
        || ('\u{3001}'..='\u{3003}').contains(&c)
        || ('\u{3008}'..='\u{3011}').contains(&c)
        || ('\u{FF01}'..='\u{FF0F}').contains(&c)
        || ('\u{FF1A}'..='\u{FF20}').contains(&c)
        || ('\u{FF3B}'..='\u{FF40}').contains(&c)
        || ('\u{FF5B}'..='\u{FF65}').contains(&c)
}

/// The word around `offset` by Unicode word bounds, inside its `\n`-delimited line: a word, a
/// whitespace run or a punctuation run (R4). At a line's end the line's last run; empty at an
/// empty line and for empty text.
pub fn word_at(text: &str, offset: usize) -> Range<usize> {
    let line = source_line_at(text, offset);
    let (base, offset) = (line.start, snap(text, offset) - line.start);
    let text = &text[line];
    let pieces: Vec<(usize, &str)> = text.split_word_bound_indices().collect();
    let Some(at) = pieces
        .iter()
        .position(|(start, piece)| offset < start + piece.len())
        .or_else(|| pieces.len().checked_sub(1))
    else {
        return base..base;
    };
    let run = run_of(pieces[at].1);
    let span = |i: usize| base + pieces[i].0..base + pieces[i].0 + pieces[i].1.len();
    if run == Run::Word {
        return span(at);
    }
    let same = |i: &usize| run_of(pieces[*i].1) == run;
    let first = (0..at).rev().take_while(same).last().unwrap_or(at);
    let last = (at + 1..pieces.len()).take_while(same).last().unwrap_or(at);
    span(first).start..span(last).end
}

/// The `\n`-delimited line around `offset`, without its line break.
pub fn source_line_at(text: &str, offset: usize) -> Range<usize> {
    let offset = snap(text, offset);
    let start = text[..offset].rfind('\n').map_or(0, |i| i + 1);
    let end = text[offset..].find('\n').map_or(text.len(), |i| offset + i);
    start..end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(row: u32, path: &[u16], offset: usize) -> TextPos {
        TextPos {
            row: RowPos::End(row),
            path: TextPath::new(path),
            offset,
        }
    }

    fn char_selection(from: TextPos, to: TextPos) -> Selection {
        let mut selection = Selection::start(from, Granularity::Char, 0..0);
        selection.extend(to, 0..0);
        selection
    }

    #[test]
    fn a_selection_extended_backwards_has_normalised_bounds() {
        let selection = char_selection(at(2, &[1], 4), at(0, &[3], 1));
        assert_eq!(selection.bounds(), (at(0, &[3], 1), at(2, &[1], 4)));
        assert_eq!(selection.anchor(), &at(2, &[1], 4), "the anchor stays");
        assert_eq!(selection.focus(), &at(0, &[3], 1));
        assert!(selection.dragging());
        let mut released = selection.clone();
        released.release();
        assert!(!released.dragging());
        assert_eq!(released.bounds(), selection.bounds(), "release keeps it");
    }

    #[test]
    fn a_selection_is_empty_iff_its_expanded_start_is_its_end() {
        let click = Selection::start(at(1, &[0], 3), Granularity::Char, 0..9);
        assert!(click.is_empty(), "a click selects nothing");
        let word = Selection::start(at(1, &[0], 3), Granularity::Word, 1..5);
        assert!(!word.is_empty(), "a double-click selects its word");
        assert_eq!(word.bounds(), (at(1, &[0], 1), at(1, &[0], 5)));
        assert!(!char_selection(at(1, &[0], 3), at(1, &[0], 4)).is_empty());
        assert!(char_selection(at(1, &[0], 3), at(1, &[0], 3)).is_empty());
    }

    #[test]
    fn in_row_gives_each_rows_part() {
        let selection = char_selection(at(1, &[2], 3), at(3, &[0, 1], 5));
        assert_eq!(selection.in_row(&RowPos::End(0)), None, "before");
        assert_eq!(
            selection.in_row(&RowPos::End(1)),
            Some(RowSelection::From(TextPath::new([2]), 3))
        );
        assert_eq!(selection.in_row(&RowPos::End(2)), Some(RowSelection::Whole));
        assert_eq!(
            selection.in_row(&RowPos::End(3)),
            Some(RowSelection::To(TextPath::new([0, 1]), 5))
        );
        assert_eq!(selection.in_row(&RowPos::End(4)), None, "after");
        assert_eq!(selection.in_row(&RowPos::Omitted), None);
        let one_row = char_selection(at(2, &[4], 6), at(2, &[1], 2));
        assert_eq!(
            one_row.in_row(&RowPos::End(2)),
            Some(RowSelection::Within(
                (TextPath::new([1]), 2),
                (TextPath::new([4]), 6)
            ))
        );
        let empty = char_selection(at(2, &[1], 2), at(2, &[1], 2));
        assert_eq!(
            empty.in_row(&RowPos::End(2)),
            None,
            "an empty one holds no row"
        );
    }

    #[test]
    fn range_in_slices_each_unit_of_a_row() {
        let p = |s: &[u16]| TextPath::new(s);
        // Units before, at and after the end place `[2]`, offset 3, of a 10-byte unit.
        let from = RowSelection::From(p(&[2]), 3);
        assert_eq!(range_in(&from, &p(&[1]), 10), None);
        assert_eq!(range_in(&from, &p(&[2]), 10), Some(3..10));
        assert_eq!(range_in(&from, &p(&[2, 0]), 10), Some(0..10));
        assert_eq!(range_in(&from, &p(&[2]), 2), None, "clamped: nothing left");
        let to = RowSelection::To(p(&[2]), 3);
        assert_eq!(range_in(&to, &p(&[1]), 10), Some(0..10));
        assert_eq!(range_in(&to, &p(&[2]), 10), Some(0..3));
        assert_eq!(range_in(&to, &p(&[3]), 10), None);
        assert_eq!(range_in(&to, &p(&[2]), 2), Some(0..2), "clamped to len");
        let within = RowSelection::Within((p(&[1]), 4), (p(&[3]), 2));
        assert_eq!(range_in(&within, &p(&[0]), 10), None);
        assert_eq!(range_in(&within, &p(&[1]), 10), Some(4..10));
        assert_eq!(range_in(&within, &p(&[2]), 10), Some(0..10));
        assert_eq!(range_in(&within, &p(&[3]), 10), Some(0..2));
        assert_eq!(range_in(&within, &p(&[4]), 10), None);
        let one_unit = RowSelection::Within((p(&[1]), 4), (p(&[1]), 7));
        assert_eq!(range_in(&one_unit, &p(&[1]), 10), Some(4..7));
        assert_eq!(range_in(&one_unit, &p(&[1]), 5), Some(4..5));
        assert_eq!(range_in(&RowSelection::Whole, &p(&[9]), 10), Some(0..10));
        assert_eq!(
            range_in(&RowSelection::Whole, &p(&[9]), 0),
            None,
            "an empty slice"
        );
        assert_eq!(range_in(&to, &p(&[2]), 0), None);
    }
}
