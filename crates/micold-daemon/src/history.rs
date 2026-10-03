//! A session terminal's history across a restart of its process (feature 041): capturing a `Term`
//! into `micold_core::terminal_history`'s snapshot, and seeding a fresh `Term` from one.

use alacritty_terminal::grid::{Dimensions, GridCell, Row};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Color, NamedColor};
use micold_core::terminal_history::{
    HistoryColor, HistorySnapshot, HistoryStyle, LogicalLine, StyleFlags, StyleRun,
};

/// The 16 basic colours, in `HistoryColor::Basic` order.
const BASIC_COLORS: [NamedColor; 16] = [
    NamedColor::Black,
    NamedColor::Red,
    NamedColor::Green,
    NamedColor::Yellow,
    NamedColor::Blue,
    NamedColor::Magenta,
    NamedColor::Cyan,
    NamedColor::White,
    NamedColor::BrightBlack,
    NamedColor::BrightRed,
    NamedColor::BrightGreen,
    NamedColor::BrightYellow,
    NamedColor::BrightBlue,
    NamedColor::BrightMagenta,
    NamedColor::BrightCyan,
    NamedColor::BrightWhite,
];

/// The 8 dim colours, in `HistoryColor::Dim` order.
const DIM_COLORS: [NamedColor; 8] = [
    NamedColor::DimBlack,
    NamedColor::DimRed,
    NamedColor::DimGreen,
    NamedColor::DimYellow,
    NamedColor::DimBlue,
    NamedColor::DimMagenta,
    NamedColor::DimCyan,
    NamedColor::DimWhite,
];

/// Each cell flag a snapshot keeps, and its counterpart. Every underline kind is underline.
const STYLE_FLAGS: [(Flags, StyleFlags); 7] = [
    (Flags::BOLD, StyleFlags::BOLD),
    (Flags::DIM, StyleFlags::DIM),
    (Flags::ITALIC, StyleFlags::ITALIC),
    (Flags::ALL_UNDERLINES, StyleFlags::UNDERLINE),
    (Flags::INVERSE, StyleFlags::INVERSE),
    (Flags::STRIKEOUT, StyleFlags::STRIKETHROUGH),
    (Flags::HIDDEN, StyleFlags::HIDDEN),
];

/// What `term` holds: its history rows, then its screen rows down to the last one that shows
/// anything, rows joined by the wrap flag as one line (research R2, data-model §1).
pub fn capture<T>(term: &Term<T>) -> HistorySnapshot {
    let grid = term.grid();
    let columns = grid.columns();
    let history = grid.history_size();
    // Buffer offset `o` (0 = oldest retained) is grid `Line(o - history)`, as in
    // `Framer::plain_tail`, whose "last row that shows anything" rule this follows.
    let row = |offset: usize| &grid[Line(offset as i32 - history as i32)];
    let mut end = history + grid.screen_lines();
    while end > 0 && shows_nothing(row(end - 1), columns) {
        end -= 1;
    }

    let mut lines = Vec::new();
    let mut line = LineBuilder::default();
    for offset in 0..end {
        let row = row(offset);
        let wraps = row[Column(columns - 1)].flags.contains(Flags::WRAPLINE);
        line.push_row(row, columns, wraps);
        if !wraps {
            lines.push(line.finish());
        }
    }
    if !line.is_empty() {
        lines.push(line.finish());
    }
    HistorySnapshot { lines }
}

/// Whether `row` has no visible character (the test of `Framer::plain_tail`'s `plain_row`).
fn shows_nothing(row: &Row<Cell>, columns: usize) -> bool {
    (0..columns).all(|column| {
        let cell = &row[Column(column)];
        is_spacer(cell)
            || ((cell.c.is_whitespace() || cell.c.is_control()) && cell.zerowidth().is_none())
    })
}

fn is_spacer(cell: &Cell) -> bool {
    cell.flags
        .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
}

/// One logical line, built from the grid rows that make it up.
#[derive(Default)]
struct LineBuilder {
    line: LogicalLine,
}

impl LineBuilder {
    /// Appends `row`'s characters. A row that does not wrap ends at its last non-blank cell.
    fn push_row(&mut self, row: &Row<Cell>, columns: usize, wraps: bool) {
        let cells: Vec<&Cell> = (0..columns)
            .map(|column| &row[Column(column)])
            .filter(|cell| !is_spacer(cell))
            .collect();
        let shown = if wraps {
            cells.len()
        } else {
            cells
                .iter()
                .rposition(|cell| !cell.is_empty())
                .map_or(0, |last| last + 1)
        };
        for cell in &cells[..shown] {
            self.push_cell(cell);
        }
    }

    fn push_cell(&mut self, cell: &Cell) {
        let base = if cell.c.is_control() { ' ' } else { cell.c };
        self.line.text.push(base);
        let marks = cell.zerowidth().unwrap_or_default();
        self.line.text.extend(marks.iter().copied());
        let chars = 1 + marks.len() as u32;
        let style = style_of(cell);
        match self.line.runs.last_mut() {
            Some(run) if run.style == style => run.chars += chars,
            _ => self.line.runs.push(StyleRun { chars, style }),
        }
    }

    fn is_empty(&self) -> bool {
        self.line.text.is_empty()
    }

    fn finish(&mut self) -> LogicalLine {
        std::mem::take(&mut self.line)
    }
}

fn style_of(cell: &Cell) -> HistoryStyle {
    let flags = STYLE_FLAGS
        .iter()
        .filter(|(cell_flag, _)| cell.flags.intersects(*cell_flag))
        .fold(StyleFlags::default(), |flags, &(_, flag)| flags.with(flag));
    HistoryStyle {
        fg: history_color(cell.fg),
        bg: history_color(cell.bg),
        flags,
    }
}

/// A cell colour in the snapshot's numbering; a named colour with no counterpart is `Default`.
fn history_color(color: Color) -> HistoryColor {
    match color {
        Color::Spec(rgb) => HistoryColor::Rgb(rgb.r, rgb.g, rgb.b),
        Color::Indexed(index) => HistoryColor::Indexed(index),
        Color::Named(named) => {
            if let Some(index) = BASIC_COLORS.iter().position(|&basic| basic == named) {
                HistoryColor::Basic(index as u8)
            } else if let Some(index) = DIM_COLORS.iter().position(|&dim| dim == named) {
                HistoryColor::Dim(index as u8)
            } else {
                HistoryColor::Default
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::term::Config;
    use alacritty_terminal::vte::ansi::Processor;
    use micold_core::terminal_history::{HistoryColor, HistoryStyle, StyleFlags, StyleRun};

    const HISTORY_LIMIT: usize = 100;

    struct Size {
        columns: usize,
        rows: usize,
    }

    impl Dimensions for Size {
        fn total_lines(&self) -> usize {
            self.rows
        }
        fn screen_lines(&self) -> usize {
            self.rows
        }
        fn columns(&self) -> usize {
            self.columns
        }
    }

    /// A `Term` with no process behind it, `columns` by `rows`, keeping `limit` history rows.
    fn term(columns: usize, rows: usize, limit: usize) -> Term<VoidListener> {
        let config = Config {
            scrolling_history: limit,
            ..Config::default()
        };
        Term::new(config, &Size { columns, rows }, VoidListener)
    }

    /// Feeds `bytes` to `term` through the VT parser, as a process's output would be.
    fn feed(term: &mut Term<VoidListener>, bytes: &str) {
        let mut parser: Processor = Processor::new();
        parser.advance(term, bytes.as_bytes());
    }

    fn texts(snapshot: &HistorySnapshot) -> Vec<&str> {
        snapshot
            .lines
            .iter()
            .map(|line| line.text.as_str())
            .collect()
    }

    fn run(chars: u32, style: HistoryStyle) -> StyleRun {
        StyleRun { chars, style }
    }

    fn fg(color: HistoryColor) -> HistoryStyle {
        HistoryStyle {
            fg: color,
            ..HistoryStyle::default()
        }
    }

    fn bg(color: HistoryColor) -> HistoryStyle {
        HistoryStyle {
            bg: color,
            ..HistoryStyle::default()
        }
    }

    fn flags(flag: StyleFlags) -> HistoryStyle {
        HistoryStyle {
            flags: flag,
            ..HistoryStyle::default()
        }
    }

    #[test]
    fn capture_returns_the_history_rows_then_the_screen_rows_in_order() {
        let mut term = term(10, 3, HISTORY_LIMIT);
        feed(&mut term, "one\r\ntwo\r\nthree\r\nfour\r\nfive");

        let snapshot = capture(&term);

        assert_eq!(texts(&snapshot), ["one", "two", "three", "four", "five"]);
    }

    #[test]
    fn each_of_the_16_basic_colours_is_captured_as_foreground_and_as_background() {
        let mut term = term(10, 20, HISTORY_LIMIT);
        for index in 0..16u8 {
            let (fg_code, bg_code) = if index < 8 {
                (30 + index, 40 + index)
            } else {
                (90 + index - 8, 100 + index - 8)
            };
            feed(
                &mut term,
                &format!("\x1b[{fg_code}mF\x1b[0m\x1b[{bg_code}mB\x1b[0m\r\n"),
            );
        }

        let snapshot = capture(&term);

        assert_eq!(snapshot.lines.len(), 16);
        for (index, line) in (0..16u8).zip(&snapshot.lines) {
            let color = HistoryColor::Basic(index);
            assert_eq!(line.text, "FB", "colour {index}");
            assert_eq!(
                line.runs,
                [run(1, fg(color)), run(1, bg(color))],
                "colour {index}"
            );
        }
    }

    #[test]
    fn an_indexed_and_an_rgb_colour_are_captured_as_foreground_and_as_background() {
        let mut term = term(10, 3, HISTORY_LIMIT);
        feed(
            &mut term,
            "\x1b[38;5;200mA\x1b[0m\x1b[48;5;201mB\x1b[0m\
             \x1b[38;2;1;2;3mC\x1b[0m\x1b[48;2;4;5;6mD\x1b[0m",
        );

        let snapshot = capture(&term);

        assert_eq!(texts(&snapshot), ["ABCD"]);
        assert_eq!(
            snapshot.lines[0].runs,
            [
                run(1, fg(HistoryColor::Indexed(200))),
                run(1, bg(HistoryColor::Indexed(201))),
                run(1, fg(HistoryColor::Rgb(1, 2, 3))),
                run(1, bg(HistoryColor::Rgb(4, 5, 6))),
            ]
        );
    }

    #[test]
    fn each_style_flag_is_captured_and_every_underline_kind_is_underline() {
        // SGR bold, dim, italic, underline, curly underline, inverse, strikethrough, hidden.
        let cases = [
            ("1", StyleFlags::BOLD),
            ("2", StyleFlags::DIM),
            ("3", StyleFlags::ITALIC),
            ("4", StyleFlags::UNDERLINE),
            ("4:3", StyleFlags::UNDERLINE),
            ("7", StyleFlags::INVERSE),
            ("9", StyleFlags::STRIKETHROUGH),
            ("8", StyleFlags::HIDDEN),
        ];
        let mut term = term(20, 3, HISTORY_LIMIT);
        for (sgr, _) in cases {
            feed(&mut term, &format!("\x1b[{sgr}mx\x1b[0m-"));
        }

        let snapshot = capture(&term);

        let expected: Vec<StyleRun> = cases
            .iter()
            .flat_map(|&(_, flag)| [run(1, flags(flag)), run(1, HistoryStyle::default())])
            .collect();
        assert_eq!(texts(&snapshot), ["x-x-x-x-x-x-x-x-"]);
        assert_eq!(snapshot.lines[0].runs, expected);
    }

    #[test]
    fn two_rows_joined_by_the_wrap_flag_are_one_logical_line() {
        let mut term = term(5, 4, HISTORY_LIMIT);
        feed(&mut term, "abcdefgh\r\nnext");

        let snapshot = capture(&term);

        assert_eq!(texts(&snapshot), ["abcdefgh", "next"]);
        assert_eq!(snapshot.lines[0].runs, [run(8, HistoryStyle::default())]);
    }

    #[test]
    fn a_wide_character_counts_as_one_character_and_its_spacer_is_skipped() {
        // In 4 columns the second line's wide character does not fit after `abc`: it wraps,
        // leaving a leading spacer in the last column.
        let mut term = term(4, 4, HISTORY_LIMIT);
        feed(&mut term, "a\u{4e16}b\r\nabc\u{4e16}");

        let snapshot = capture(&term);

        assert_eq!(texts(&snapshot), ["a\u{4e16}b", "abc\u{4e16}"]);
        assert_eq!(snapshot.lines[0].runs, [run(3, HistoryStyle::default())]);
        assert_eq!(snapshot.lines[1].runs, [run(4, HistoryStyle::default())]);
    }

    #[test]
    fn a_zero_width_character_follows_its_base_character() {
        let mut term = term(10, 3, HISTORY_LIMIT);
        feed(&mut term, "e\u{301}x");

        let snapshot = capture(&term);

        assert_eq!(texts(&snapshot), ["e\u{301}x"]);
        assert_eq!(snapshot.lines[0].runs, [run(3, HistoryStyle::default())]);
    }

    #[test]
    fn trailing_empty_screen_rows_are_not_captured() {
        let mut term = term(10, 6, HISTORY_LIMIT);
        feed(&mut term, "a\r\n\r\nb\r\n");

        let snapshot = capture(&term);

        assert_eq!(texts(&snapshot), ["a", "", "b"]);
        assert_eq!(snapshot.lines[1].runs, []);
    }

    #[test]
    fn a_term_that_printed_nothing_gives_an_empty_snapshot() {
        let term = term(10, 3, HISTORY_LIMIT);

        assert_eq!(capture(&term), HistorySnapshot::default());
    }
}
