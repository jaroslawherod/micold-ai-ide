//! Which link is under a cell (research R4, R5; contract link-recognition §2).

use super::{CellSpan, Link, LinkOrigin, LinkRows};

/// The link under the cell at `row`, `col`, if any (contract link-recognition §2).
pub fn link_at(rows: &impl LinkRows, row: i64, col: u16) -> Option<Link> {
    let uri = rows.hyperlink(row, col)?;
    let width = rows.text(row)?.chars().count() as u16;
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

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::*;
    use crate::link::{CellSpan, LinkOrigin};

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
}
