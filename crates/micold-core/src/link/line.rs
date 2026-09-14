//! Which link is under a cell (research R4, R5; contract link-recognition §2).

use std::ops::Range;

use super::{detect::detect, CellSpan, Link, LinkOrigin, LinkRows};

/// The link under the cell at `row`, `col`, if any (contract link-recognition §2).
pub fn link_at(rows: &impl LinkRows, row: i64, col: u16) -> Option<Link> {
    let text = rows.text(row)?;
    let Some(uri) = rows.hyperlink(row, col) else {
        return detected_at(&LogicalLine::around(rows, row), row, col);
    };
    let width = text.chars().count() as u16;
    let same = |col: u16| rows.hyperlink(row, col) == Some(uri);
    let start = (0..col)
        .rev()
        .take_while(|&col| same(col))
        .last()
        .unwrap_or(col);
    let end = (col..width)
        .take_while(|&col| same(col))
        .last()
        .unwrap_or(col)
        + 1;
    Some(Link {
        address: uri.to_string(),
        origin: LinkOrigin::Declared,
        cells: vec![CellSpan {
            row,
            cols: start..end,
        }],
    })
}

/// The rows joined by soft wraps around one row, as one string (research R4).
struct LogicalLine {
    text: Vec<char>,
    /// The cell each char of `text` sits in.
    cells: Vec<(i64, u16)>,
}

impl LogicalLine {
    /// The logical line holding `row`: back while the row above soft-wraps into this one, then
    /// forward while this row soft-wraps into the next.
    fn around(rows: &impl LinkRows, row: i64) -> Self {
        let mut first = row;
        while rows.wrapped(first - 1) && rows.text(first - 1).is_some() {
            first -= 1;
        }
        let mut line = Self {
            text: Vec::new(),
            cells: Vec::new(),
        };
        let mut current = first;
        while let Some(text) = rows.text(current) {
            for (col, c) in text.chars().enumerate() {
                line.text.push(c);
                line.cells.push((current, col as u16));
            }
            if !rows.wrapped(current) {
                break;
            }
            current += 1;
        }
        line
    }

    /// The cells the chars in `range` sit in, one span per row.
    fn spans(&self, range: Range<usize>) -> Vec<CellSpan> {
        let mut spans: Vec<CellSpan> = Vec::new();
        for &(row, col) in &self.cells[range] {
            match spans.last_mut() {
                Some(span) if span.row == row => span.cols.end = col + 1,
                _ => spans.push(CellSpan {
                    row,
                    cols: col..col + 1,
                }),
            }
        }
        spans
    }
}

/// The detected address in `line` holding the cell at `row`, `col` (contract L2).
fn detected_at(line: &LogicalLine, row: i64, col: u16) -> Option<Link> {
    let index = line.cells.iter().position(|&cell| cell == (row, col))?;
    let text: String = line.text.iter().collect();
    let range = detect(&text)
        .into_iter()
        .find(|range| range.contains(&index))?;
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
        text: &'static str,
        wrapped: bool,
        declared: Vec<(Range<u16>, &'static str)>,
    }

    fn row(text: &'static str) -> Row {
        Row {
            text,
            wrapped: false,
            declared: Vec::new(),
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
            self.get(row).map(|row| row.text)
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
}
