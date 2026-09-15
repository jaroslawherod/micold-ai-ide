//! SC-002 (feature 031): links in real AI CLI and command-line output are recognised with exactly
//! the right start and end, and nothing else is.
//!
//! The corpus is `tests/fixtures/link_corpus.txt`; its header documents the format. Every entry is
//! lent to `link_at` as rows, and every cell of it is asked for the link under it, so a scheme-less
//! address anywhere in the corpus fails the same way a missed or mis-cut one does.

use std::ops::Range;

use micold_core::link::{
    line::link_at, resolve::resolve, CellSpan, Link, LinkContext, LinkOrigin, LinkRows, Target,
};

const CORPUS: &str = include_str!("fixtures/link_corpus.txt");
const OPEN: char = '⟦';
const CLOSE: char = '⟧';

/// One output line: its text without delimiters, the char ranges of its expected links, and the
/// width it soft-wraps at, if any.
struct Line {
    /// Its line in the corpus file, for failure messages.
    at: usize,
    text: Vec<char>,
    links: Vec<Range<usize>>,
    wrap: Option<usize>,
}

/// One corpus entry, read on its own.
struct Entry {
    hard: bool,
    lines: Vec<Line>,
}

fn corpus() -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    let mut open: Option<Entry> = None;
    let mut wrap = None;
    for (index, raw) in CORPUS.lines().enumerate() {
        if raw.starts_with('#') {
            continue;
        }
        if raw.is_empty() {
            entries.extend(open.take());
            continue;
        }
        let entry = open.get_or_insert_with(|| Entry {
            hard: false,
            lines: Vec::new(),
        });
        if raw == "@hard" {
            entry.hard = true;
        } else if let Some(width) = raw.strip_prefix("@wrap ") {
            wrap = Some(width.parse().expect("@wrap takes a width"));
        } else {
            entry.lines.push(line(index + 1, raw, wrap.take()));
        }
    }
    entries.extend(open);
    entries
}

fn line(at: usize, raw: &str, wrap: Option<usize>) -> Line {
    let mut text = Vec::new();
    let mut links = Vec::new();
    let mut start = None;
    for c in raw.chars() {
        match c {
            OPEN => start = Some(text.len()),
            CLOSE => links.push(start.take().expect("⟧ closes a ⟦")..text.len()),
            c => text.push(c),
        }
    }
    Line {
        at,
        text,
        links,
        wrap,
    }
}

/// The rows an entry prints, with one empty row above and below it.
struct Rows {
    rows: Vec<(String, bool)>,
    /// Where each output line's char `i` lands: `(row, col)`.
    cells: Vec<Vec<(i64, u16)>>,
}

impl Rows {
    fn of(entry: &Entry) -> Self {
        let mut rows = vec![(String::new(), false)];
        let mut cells = Vec::new();
        for line in &entry.lines {
            let width = line.wrap.unwrap_or(line.text.len().max(1));
            let first = rows.len();
            let chunks: Vec<&[char]> = line.text.chunks(width).collect();
            let last = chunks.len().saturating_sub(1);
            for (index, chunk) in chunks.iter().enumerate() {
                rows.push((chunk.iter().collect(), index < last));
            }
            cells.push(
                (0..line.text.len())
                    .map(|i| ((first + i / width) as i64, (i % width) as u16))
                    .collect(),
            );
        }
        rows.push((String::new(), false));
        Self { rows, cells }
    }

    fn get(&self, row: i64) -> Option<&(String, bool)> {
        usize::try_from(row).ok().and_then(|row| self.rows.get(row))
    }

    /// The link expected at char range `range` of output line `line`.
    fn expected(&self, entry: &Entry, line: usize, range: &Range<usize>) -> Link {
        let mut cells: Vec<CellSpan> = Vec::new();
        for &(row, col) in &self.cells[line][range.clone()] {
            match cells.last_mut() {
                Some(span) if span.row == row => span.cols.end = col + 1,
                _ => cells.push(CellSpan {
                    row,
                    cols: col..col + 1,
                }),
            }
        }
        Link {
            address: entry.lines[line].text[range.clone()].iter().collect(),
            origin: LinkOrigin::Detected,
            cells,
        }
    }
}

impl LinkRows for Rows {
    fn text(&self, row: i64) -> Option<&str> {
        self.get(row).map(|(text, _)| text.as_str())
    }

    fn wrapped(&self, row: i64) -> bool {
        self.get(row).is_some_and(|(_, wrapped)| *wrapped)
    }

    fn hyperlink(&self, _row: i64, _col: u16) -> Option<&str> {
        None
    }

    fn spacer(&self, _row: i64, _col: u16) -> bool {
        false
    }
}

fn local() -> LinkContext {
    LinkContext {
        host_names: vec!["devbox".to_string()],
        windows_host: false,
        sandbox: None,
    }
}

#[test]
fn every_address_in_the_corpus_is_found_with_its_exact_span_and_nothing_else() {
    let entries = corpus();
    let output_lines: usize = entries.iter().map(|entry| entry.lines.len()).sum();
    assert!(
        output_lines >= 50,
        "the corpus holds at least 50 real lines, not {output_lines}"
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.lines.iter().any(|line| line.wrap.is_some())),
        "the corpus holds soft-wrapped addresses"
    );

    for entry in entries.iter().filter(|entry| !entry.hard) {
        let rows = Rows::of(entry);
        for (index, line) in entry.lines.iter().enumerate() {
            for (i, &(row, col)) in rows.cells[index].iter().enumerate() {
                let expected = line
                    .links
                    .iter()
                    .find(|range| range.contains(&i))
                    .map(|range| rows.expected(entry, index, range));
                assert_eq!(
                    link_at(&rows, row, col),
                    expected,
                    "corpus line {}, char {i} ({:?}): the link under it is exactly the one marked, or none",
                    line.at,
                    line.text[i]
                );
            }
            for range in &line.links {
                let link = rows.expected(entry, index, range);
                let address = link.address.clone();
                let file = address.to_ascii_lowercase().starts_with("file:");
                let Some(resolved) = resolve(link, &local()) else {
                    assert!(file, "{address} is a web or mail address, so it resolves");
                    continue;
                };
                let opens = match &resolved.target {
                    Target::Url(opens) | Target::HostPath(opens) => opens,
                    Target::Unreachable(_) => panic!("{address} is on this machine"),
                };
                assert_eq!(
                    &resolved.display, opens,
                    "the hint for {address} shows exactly what opens"
                );
                if !file {
                    assert_eq!(opens, &address, "{address} opens exactly as marked");
                }
            }
        }
    }
}
