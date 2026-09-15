//! Which link is under a cell (research R4, R5; contract link-recognition §2).

use std::ops::Range;

use super::{
    detect::{detect, ends_an_address, TRAILING_PUNCTUATION},
    CellSpan, Link, LinkOrigin, LinkRows,
};

/// How many rows a logical line reaches above and below the pointer row (research R4; contract L4).
const MAX_ROWS_EACH_WAY: i64 = 64;

/// The link under the cell at `row`, `col`, if any (contract link-recognition §2).
pub fn link_at(rows: &impl LinkRows, row: i64, col: u16) -> Option<Link> {
    rows.text(row)?;
    let line = LogicalLine::around(rows, row);
    match rows.hyperlink(row, col) {
        Some(uri) => declared_at(rows, &line, uri, row, col),
        None => detected_at(&line, row, col),
    }
}

/// The rows joined by soft wraps around one row, as one string (research R4).
struct LogicalLine {
    text: Vec<char>,
    /// The cells each char of `text` covers: one, or two for a wide char with its spacer.
    cells: Vec<(i64, Range<u16>)>,
    /// The line may go on above its first row: the cap stopped the walk, or the row above is
    /// unavailable (contract L4, L7).
    cut_above: bool,
    /// The line goes on below its last row, past the cap or into an unavailable row.
    cut_below: bool,
}

impl LogicalLine {
    /// The logical line holding `row`: back while the row above soft-wraps into this one, then
    /// forward while this row soft-wraps into the next, at most [`MAX_ROWS_EACH_WAY`] rows each way.
    fn around(rows: &impl LinkRows, row: i64) -> Self {
        let continues_above = |row: i64| rows.wrapped(row - 1) && rows.text(row - 1).is_some();
        let continues_below = |row: i64| rows.wrapped(row) && rows.text(row + 1).is_some();
        let mut first = row;
        while first > row - MAX_ROWS_EACH_WAY && continues_above(first) {
            first -= 1;
        }
        let mut last = first;
        while last < row + MAX_ROWS_EACH_WAY && continues_below(last) {
            last += 1;
        }
        let mut text = Vec::new();
        let mut cells: Vec<(i64, Range<u16>)> = Vec::new();
        for current in first..=last {
            for (col, c) in (0u16..).zip(rows.text(current).unwrap_or_default().chars()) {
                if !rows.spacer(current, col) {
                    text.push(c);
                    cells.push((current, col..col + 1));
                } else if let Some((_, cols)) = cells.last_mut().filter(|(row, _)| *row == current)
                {
                    // A spacer is the second half of the char before it on its row (contract L5).
                    cols.end = col + 1;
                }
            }
        }
        Self {
            text,
            cells,
            cut_above: rows.text(first - 1).is_none() || rows.wrapped(first - 1),
            cut_below: rows.wrapped(last),
        }
    }

    /// Where the char in the cell at `row`, `col` sits in `text`.
    fn index_of(&self, row: i64, col: u16) -> Option<usize> {
        self.cells
            .iter()
            .position(|(r, cols)| *r == row && cols.contains(&col))
    }

    /// The cells the chars in `range` sit in, one span per row.
    fn spans(&self, range: Range<usize>) -> Vec<CellSpan> {
        let mut spans: Vec<CellSpan> = Vec::new();
        for (row, cols) in &self.cells[range] {
            match spans.last_mut() {
                Some(span) if span.row == *row => span.cols.end = cols.end,
                _ => spans.push(CellSpan {
                    row: *row,
                    cols: cols.clone(),
                }),
            }
        }
        spans
    }
}

/// The maximal run of cells in `line` declaring `uri` that holds the cell at `row`, `col` (contract
/// L1).
fn declared_at(
    rows: &impl LinkRows,
    line: &LogicalLine,
    uri: &str,
    row: i64,
    col: u16,
) -> Option<Link> {
    let index = line.index_of(row, col)?;
    let same = |index: &usize| {
        let (row, ref cols) = line.cells[*index];
        rows.hyperlink(row, cols.start) == Some(uri)
    };
    let start = (0..index).rev().take_while(same).last().unwrap_or(index);
    let end = (index..line.cells.len())
        .take_while(same)
        .last()
        .unwrap_or(index)
        + 1;
    Some(Link {
        address: uri.to_string(),
        origin: LinkOrigin::Declared,
        cells: line.spans(start..end),
    })
}

/// The detected address in `line` holding the cell at `row`, `col` (contract L2).
fn detected_at(line: &LogicalLine, row: i64, col: u16) -> Option<Link> {
    let index = line.index_of(row, col)?;
    let text: String = line.text.iter().collect();
    let range = detect(&text)
        .into_iter()
        .find(|range| range.contains(&index))?;
    let reaches_a_cut = (line.cut_above
        && !line.text[..range.start].iter().any(|&c| ends_an_address(c)))
        || (line.cut_below
            && line.text[range.end..]
                .iter()
                .all(|c| TRAILING_PUNCTUATION.contains(c)));
    if reaches_a_cut {
        return None;
    }
    Some(Link {
        address: line.text[range.clone()].iter().collect(),
        origin: LinkOrigin::Detected,
        cells: line.spans(range),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "https://a.example";

    /// One row of the fake grid: its text, whether it soft-wraps, and the URIs declared on it.
    struct Row {
        text: String,
        wrapped: bool,
        declared: Vec<(Range<u16>, &'static str)>,
        spacers: Vec<u16>,
    }

    fn row(text: &str) -> Row {
        Row {
            text: text.to_string(),
            wrapped: false,
            declared: Vec::new(),
            spacers: Vec::new(),
        }
    }

    impl Row {
        fn wrapped(mut self) -> Self {
            self.wrapped = true;
            self
        }

        fn declare(mut self, cols: Range<u16>, uri: &'static str) -> Self {
            self.declared.push((cols, uri));
            self
        }

        fn spacer(mut self, col: u16) -> Self {
            self.spacers.push(col);
            self
        }
    }

    /// Rows lent to `link_at`, the first of them at `first`; every other row is unavailable.
    struct Rows {
        first: i64,
        rows: Vec<Row>,
    }

    impl Rows {
        fn new(first: i64, rows: Vec<Row>) -> Self {
            Self { first, rows }
        }

        fn get(&self, row: i64) -> Option<&Row> {
            usize::try_from(row - self.first)
                .ok()
                .and_then(|index| self.rows.get(index))
        }
    }

    impl LinkRows for Rows {
        fn text(&self, row: i64) -> Option<&str> {
            self.get(row).map(|row| row.text.as_str())
        }

        fn wrapped(&self, row: i64) -> bool {
            self.get(row).is_some_and(|row| row.wrapped)
        }

        fn hyperlink(&self, row: i64, col: u16) -> Option<&str> {
            self.get(row)?
                .declared
                .iter()
                .find(|(cols, _)| cols.contains(&col))
                .map(|(_, uri)| *uri)
        }

        fn spacer(&self, row: i64, col: u16) -> bool {
            self.get(row).is_some_and(|row| row.spacers.contains(&col))
        }
    }

    /// `text` soft-wrapped into rows `width` cells wide, every row but the last wrapping.
    fn soft_wrapped(text: &str, width: usize) -> Vec<Row> {
        let chars: Vec<char> = text.chars().collect();
        let mut rows: Vec<Row> = chars
            .chunks(width)
            .map(|chunk| row(&chunk.iter().collect::<String>()).wrapped())
            .collect();
        if let Some(last) = rows.last_mut() {
            last.wrapped = false;
        }
        rows
    }

    fn declared(cells: Vec<CellSpan>) -> Option<Link> {
        Some(Link {
            address: ADDRESS.to_string(),
            origin: LinkOrigin::Declared,
            cells,
        })
    }

    fn span(row: i64, cols: Range<u16>) -> CellSpan {
        CellSpan { row, cols }
    }

    #[test]
    fn any_cell_of_a_declared_run_returns_the_whole_run() {
        let rows = Rows::new(0, vec![row("Open the docs now").declare(5..13, ADDRESS)]);
        for col in 5..13 {
            assert_eq!(
                link_at(&rows, 0, col),
                declared(vec![span(0, 5..13)]),
                "column {col} lies in the declared run, so the link is the whole run"
            );
        }
    }

    #[test]
    fn two_same_uri_runs_apart_are_two_links() {
        let rows = Rows::new(
            0,
            vec![row("docs and docs")
                .declare(0..4, ADDRESS)
                .declare(9..13, ADDRESS)],
        );
        assert_eq!(
            link_at(&rows, 0, 1),
            declared(vec![span(0, 0..4)]),
            "the first run ends at the undeclared cell after it"
        );
        assert_eq!(
            link_at(&rows, 0, 10),
            declared(vec![span(0, 9..13)]),
            "the second run starts after the undeclared cell before it, though its URI is the same"
        );
    }

    #[test]
    fn adjacent_runs_with_different_uris_are_two_links() {
        const OTHER: &str = "https://b.example";
        let rows = Rows::new(
            0,
            vec![row("onetwo").declare(0..3, ADDRESS).declare(3..6, OTHER)],
        );
        assert_eq!(
            link_at(&rows, 0, 2),
            declared(vec![span(0, 0..3)]),
            "the first run ends where the next URI starts"
        );
        assert_eq!(
            link_at(&rows, 0, 3),
            Some(Link {
                address: OTHER.to_string(),
                origin: LinkOrigin::Declared,
                cells: vec![span(0, 3..6)],
            }),
            "the second run opens its own address"
        );
    }

    #[test]
    fn a_detected_address_on_one_row_returns_its_cells() {
        let rows = Rows::new(0, vec![row("See https://a.example/x now")]);
        assert_eq!(
            link_at(&rows, 0, 6),
            Some(Link {
                address: "https://a.example/x".to_string(),
                origin: LinkOrigin::Detected,
                cells: vec![span(0, 4..23)],
            }),
            "a cell inside a recognised address gives that address and the cells it covers"
        );
    }

    #[test]
    fn a_detected_address_over_soft_wrapped_rows_covers_every_row() {
        let rows = Rows::new(
            0,
            vec![row("See https://a.exa").wrapped(), row("mple/x now")],
        );
        let link = Some(Link {
            address: "https://a.example/x".to_string(),
            origin: LinkOrigin::Detected,
            cells: vec![span(0, 4..17), span(1, 0..6)],
        });
        assert_eq!(
            link_at(&rows, 0, 6),
            link,
            "from the first row, the address continues onto the row it soft-wraps into"
        );
        assert_eq!(
            link_at(&rows, 1, 2),
            link,
            "from the second row, the address is the same link, starting on the row above"
        );
    }

    #[test]
    fn a_declared_uri_wins_over_the_address_its_text_shows() {
        let rows = Rows::new(0, vec![row("https://b.example/x").declare(0..19, ADDRESS)]);
        assert_eq!(
            link_at(&rows, 0, 3),
            declared(vec![span(0, 0..19)]),
            "the program chose the target, so the declared URI opens, not the address the text shows"
        );
    }

    #[test]
    fn a_declared_run_continues_across_a_soft_wrap_and_stops_at_a_line_break() {
        let rows = Rows::new(
            0,
            vec![
                row("Open the do").wrapped().declare(9..11, ADDRESS),
                row("cs now").declare(0..2, ADDRESS),
                row("cs again").declare(0..2, ADDRESS),
            ],
        );
        let link = declared(vec![span(0, 9..11), span(1, 0..2)]);
        assert_eq!(
            link_at(&rows, 0, 10),
            link,
            "from the first row, the run continues onto the row it soft-wraps into"
        );
        assert_eq!(
            link_at(&rows, 1, 0),
            link,
            "from the second row, the run is the same link, starting on the row above"
        );
        assert_eq!(
            link_at(&rows, 2, 1),
            declared(vec![span(2, 0..2)]),
            "a real line break ends the run, though the next row starts with the same URI"
        );
    }

    #[test]
    fn rows_apart_by_a_line_break_are_never_joined() {
        let rows = Rows::new(0, vec![row("See https://a.example/do"), row("cs now")]);
        assert_eq!(
            link_at(&rows, 0, 6),
            Some(Link {
                address: "https://a.example/do".to_string(),
                origin: LinkOrigin::Detected,
                cells: vec![span(0, 4..24)],
            }),
            "the address ends at the line break; the next row's text is not part of it"
        );
        assert_eq!(
            link_at(&rows, 1, 1),
            None,
            "the row after a line break starts a new line, with no address on it"
        );
    }

    #[test]
    fn a_line_is_joined_64_rows_each_way_and_a_candidate_reaching_the_cap_is_dropped() {
        const WIDTH: usize = 4;
        let web = |fill: usize| format!("https://a.example/{}", "p".repeat(fill));

        let within = web(496);
        let rows = Rows::new(0, soft_wrapped(&format!(" {within} "), WIDTH));
        let mut cells = vec![span(0, 1..4)];
        cells.extend((1..128).map(|row| span(row, 0..4)));
        cells.push(span(128, 0..3));
        assert_eq!(
            link_at(&rows, 64, 0),
            Some(Link {
                address: within,
                origin: LinkOrigin::Detected,
                cells,
            }),
            "an address reaching exactly 64 rows above and below the pointer row is joined whole"
        );

        let past_the_bottom = web(499);
        let rows = Rows::new(0, soft_wrapped(&format!(" {past_the_bottom} now"), WIDTH));
        assert_eq!(
            link_at(&rows, 64, 0),
            None,
            "an address running past 64 rows below the pointer row is cut there, so it is dropped"
        );

        let past_the_top = format!(
            "See https://a.example/?next=https://b.example/{} now",
            "p".repeat(244)
        );
        let rows = Rows::new(0, soft_wrapped(&past_the_top, WIDTH));
        assert_eq!(
            link_at(&rows, 71, 0),
            None,
            "the cap 64 rows above cuts the address; the part below it is not offered as a link"
        );
    }

    #[test]
    fn a_candidate_touching_an_unavailable_row_is_dropped() {
        let rows = Rows::new(0, vec![row("See https://a.exa").wrapped()]);
        assert_eq!(
            link_at(&rows, 0, 6),
            None,
            "the row below is unavailable, so the address may go on there and is dropped"
        );

        let rows = Rows::new(0, vec![row("See https://a.example ").wrapped()]);
        assert_eq!(
            link_at(&rows, 0, 6),
            Some(Link {
                address: ADDRESS.to_string(),
                origin: LinkOrigin::Detected,
                cells: vec![span(0, 4..21)],
            }),
            "an address ending before the last column is whole, though the row below is unavailable"
        );

        let rows = Rows::new(0, vec![row("https://a.example now")]);
        assert_eq!(
            link_at(&rows, 0, 3),
            None,
            "the row above the first available row is unknown, so an address at column 0 may have started there"
        );
    }

    #[test]
    fn a_candidate_only_punctuation_away_from_a_cut_is_dropped() {
        let rows = Rows::new(0, vec![row("See https://a.example/x.").wrapped()]);
        assert_eq!(
            link_at(&rows, 0, 6),
            None,
            "the full stop may be the middle of an address going on below, so the address is cut"
        );

        let rows = Rows::new(0, vec![row("See https://a.example/x. ").wrapped()]);
        assert_eq!(
            link_at(&rows, 0, 6),
            Some(Link {
                address: "https://a.example/x".to_string(),
                origin: LinkOrigin::Detected,
                cells: vec![span(0, 4..23)],
            }),
            "a space after the punctuation ends the address before the cut"
        );
    }

    #[test]
    fn a_candidate_that_may_sit_inside_an_address_cut_above_is_dropped() {
        let rows = Rows::new(1, vec![row("=https://b.example/y now")]);
        assert_eq!(
            link_at(&rows, 1, 6),
            None,
            "the row above is unavailable and only address characters come before the scheme, so it may be the tail of `?next=https://…`"
        );

        let rows = Rows::new(1, vec![row("x https://b.example/y now")]);
        assert_eq!(
            link_at(&rows, 1, 6),
            Some(Link {
                address: "https://b.example/y".to_string(),
                origin: LinkOrigin::Detected,
                cells: vec![span(1, 2..21)],
            }),
            "a space before the scheme ends whatever began above, so the address is whole"
        );
    }

    #[test]
    fn a_wide_characters_spacer_cell_belongs_to_the_link() {
        // One char per cell: each wide char's second cell is a spacer holding a space.
        let rows = Rows::new(
            0,
            vec![
                row("See https://例 え .jp now").spacer(13).spacer(15),
                row("日 本 docs").spacer(1).spacer(3).declare(0..4, ADDRESS),
            ],
        );
        let detected = Some(Link {
            address: "https://例え.jp".to_string(),
            origin: LinkOrigin::Detected,
            cells: vec![span(0, 4..19)],
        });
        for col in [6, 13, 15] {
            assert_eq!(
                link_at(&rows, 0, col),
                detected,
                "column {col}: the address reads past the wide chars' spacers and covers them"
            );
        }
        assert_eq!(
            link_at(&rows, 1, 3),
            declared(vec![span(1, 0..4)]),
            "the spacer after the last wide char of a declared run is part of the run"
        );
    }

    #[test]
    fn a_wide_char_wrapped_onto_the_next_row_leaves_padding_the_address_reads_across() {
        // The last cell of the first row is padding: the wide char did not fit, so it wrapped.
        let rows = Rows::new(
            0,
            vec![
                row("See https://a.example/ ").spacer(22).wrapped(),
                row("例 え  now").spacer(1).spacer(3),
            ],
        );
        let link = Some(Link {
            address: "https://a.example/例え".to_string(),
            origin: LinkOrigin::Detected,
            cells: vec![span(0, 4..23), span(1, 0..4)],
        });
        assert_eq!(
            link_at(&rows, 0, 22),
            link,
            "the padding is not a space ending the address, and it is covered by the link"
        );
        assert_eq!(
            link_at(&rows, 1, 2),
            link,
            "from the next row, the address is the same link"
        );
    }

    #[test]
    fn a_plain_text_cell_is_no_link() {
        let rows = Rows::new(
            0,
            vec![row("See https://a.example and docs").declare(26..30, ADDRESS)],
        );
        for col in [1, 3, 21, 24] {
            assert_eq!(
                link_at(&rows, 0, col),
                None,
                "column {col} is plain text beside a detected address and a declared run"
            );
        }
    }

    #[test]
    fn a_scrollback_row_resolves_like_a_viewport_row() {
        let lines = || [row("See https://a.exa").wrapped(), row("mple/x now")];
        let in_view = link_at(
            &Rows::new(0, std::iter::once(row("")).chain(lines()).collect()),
            2,
            2,
        );
        let in_scrollback = link_at(
            &Rows::new(-2, lines().into_iter().chain([row("")]).collect()),
            -1,
            2,
        );
        let moved = |link: Option<Link>, by: i64| {
            link.map(|link| Link {
                cells: link
                    .cells
                    .into_iter()
                    .map(|cell| span(cell.row + by, cell.cols))
                    .collect(),
                ..link
            })
        };
        assert!(in_view.is_some(), "the address is found in the viewport");
        assert_eq!(
            in_scrollback,
            moved(in_view, -3),
            "the same rows three lines up, in scrollback, give the same link on the rows above"
        );
    }
}
