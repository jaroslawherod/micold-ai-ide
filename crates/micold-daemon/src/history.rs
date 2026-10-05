//! A session terminal's history across a restart of its process (feature 041): capturing a `Term`
//! into `micold_core::terminal_history`'s snapshot, and seeding a fresh `Term` from one.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};
use std::time::Instant;

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::{Dimensions, GridCell, Row};
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Attr, ClearMode, Color, Handler, NamedColor, Rgb};
use chrono::{DateTime, Local};
use micold_core::session::SessionId;
use micold_core::terminal_history::schedule::SaveSchedule;
use micold_core::terminal_history::text::separator_line;
use micold_core::terminal_history::{
    HistoryColor, HistorySnapshot, HistoryStyle, LogicalLine, StyleFlags, StyleRun,
};

use crate::supervisor::PtySession;

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

/// What a fresh `Term` is given before its process starts (data-model §6).
#[derive(Debug, Clone)]
pub enum Seed {
    /// Nothing: the terminal starts empty.
    None,
    /// The earlier history, then the separator naming `at`, the time of this start.
    History {
        snapshot: HistorySnapshot,
        at: DateTime<Local>,
    },
}

/// Writes `seed` into `term` through its `vte::ansi::Handler`, keeping at most the most recent
/// `limit` + screen rows lines, and leaves the screen blank with the cursor at home (R3, R17).
pub fn seed<T: EventListener>(term: &mut Term<T>, seed: Seed, limit: usize) {
    let Seed::History { snapshot, at } = seed else {
        return;
    };
    let keep = limit + term.screen_lines();
    let skip = snapshot.lines.len().saturating_sub(keep);
    for line in &snapshot.lines[skip..] {
        write_line(term, line);
    }

    let text = separator_line(&at.format(SEPARATOR_TIME).to_string(), term.columns());
    let dim = HistoryStyle {
        flags: StyleFlags::DIM,
        ..HistoryStyle::default()
    };
    let separator = LogicalLine {
        runs: vec![StyleRun {
            chars: text.chars().count() as u32,
            style: dim,
        }],
        text,
    };
    write_line(term, &separator);

    // Move the seeded rows off the screen into the history, so the new process starts on a blank
    // screen at home on every platform (R17); clearing does not move the cursor.
    term.clear_screen(ClearMode::All);
    term.goto(0, 0);
}

/// The separator's time: `YYYY-MM-DD HH:MM ±HH:MM`.
const SEPARATOR_TIME: &str = "%Y-%m-%d %H:%M %:z";

/// Each style flag and the attribute that sets it.
const FLAG_ATTRIBUTES: [(StyleFlags, Attr); 7] = [
    (StyleFlags::BOLD, Attr::Bold),
    (StyleFlags::DIM, Attr::Dim),
    (StyleFlags::ITALIC, Attr::Italic),
    (StyleFlags::UNDERLINE, Attr::Underline),
    (StyleFlags::INVERSE, Attr::Reverse),
    (StyleFlags::STRIKETHROUGH, Attr::Strike),
    (StyleFlags::HIDDEN, Attr::Hidden),
];

/// Writes `line` in its styles at the cursor, then resets the attributes and moves to the start of
/// the next row.
fn write_line<T: EventListener>(term: &mut Term<T>, line: &LogicalLine) {
    let mut chars = line.text.chars().filter(|c| !c.is_control());
    for run in &line.runs {
        set_style(term, run.style);
        for c in chars.by_ref().take(run.chars as usize) {
            term.input(c);
        }
    }
    term.terminal_attribute(Attr::Reset);
    for c in chars {
        term.input(c);
    }
    term.carriage_return();
    term.linefeed();
}

fn set_style<T: EventListener>(term: &mut Term<T>, style: HistoryStyle) {
    term.terminal_attribute(Attr::Reset);
    if let Some(color) = term_color(style.fg) {
        term.terminal_attribute(Attr::Foreground(color));
    }
    if let Some(color) = term_color(style.bg) {
        term.terminal_attribute(Attr::Background(color));
    }
    for (flag, attribute) in FLAG_ATTRIBUTES {
        if style.flags.contains(flag) {
            term.terminal_attribute(attribute);
        }
    }
}

/// A snapshot colour as the `Term`'s; `None` for the default (or an index outside its palette).
fn term_color(color: HistoryColor) -> Option<Color> {
    match color {
        HistoryColor::Default => None,
        HistoryColor::Basic(index) => BASIC_COLORS
            .get(usize::from(index))
            .copied()
            .map(Color::Named),
        HistoryColor::Dim(index) => DIM_COLORS
            .get(usize::from(index))
            .copied()
            .map(Color::Named),
        HistoryColor::Indexed(index) => Some(Color::Indexed(index)),
        HistoryColor::Rgb(r, g, b) => Some(Color::Spec(Rgb { r, g, b })),
    }
}

/// The periodic saver's memory (feature 041, research R6): one [`SaveSchedule`] per covered live
/// terminal, and the failures already logged in this service run. It does no I/O and takes no lock
/// of the session state; [`crate::state::DaemonState::save_due_at`] does the saving.
#[derive(Default)]
pub struct Saver {
    schedules: HashMap<SessionId, Tracked>,
    /// Failures already logged, by session and reason (FR-007).
    logged: HashSet<(SessionId, String)>,
}

/// A schedule and the process it is of. A restarted process counts its output from zero again,
/// so its schedule is a new one.
struct Tracked {
    pty: Weak<PtySession>,
    schedule: SaveSchedule,
}

impl Saver {
    /// The terminals due at `now` among the covered live `terminals`, with each one's output
    /// count. The schedule of a terminal that is no longer live is dropped.
    pub fn due(
        &mut self,
        now: Instant,
        terminals: &[(SessionId, Arc<PtySession>)],
    ) -> Vec<(SessionId, Arc<PtySession>)> {
        self.schedules
            .retain(|id, _| terminals.iter().any(|(live, _)| live == id));
        let mut due = Vec::new();
        for (id, pty) in terminals {
            let count = pty.signals().output_count();
            let tracked = self.schedules.entry(*id).or_insert_with(|| Tracked {
                pty: Arc::downgrade(pty),
                schedule: SaveSchedule::new(count),
            });
            if !Weak::ptr_eq(&tracked.pty, &Arc::downgrade(pty)) {
                *tracked = Tracked {
                    pty: Arc::downgrade(pty),
                    schedule: SaveSchedule::new(count),
                };
            }
            if tracked.schedule.due(now, count) {
                due.push((*id, Arc::clone(pty)));
            }
        }
        due
    }

    /// `id` was saved at `now` as of `output_count`.
    pub fn saved(&mut self, id: SessionId, now: Instant, output_count: u64) {
        if let Some(tracked) = self.schedules.get_mut(&id) {
            tracked.schedule.saved(now, output_count);
        }
    }

    /// A save of `id` failed at `now` for `reason`: it is due again later. Returns whether this is
    /// the first time the reason is seen for the session, so it is logged once (FR-007).
    pub fn failed(&mut self, id: SessionId, now: Instant, reason: &str) -> bool {
        if let Some(tracked) = self.schedules.get_mut(&id) {
            tracked.schedule.failed(now);
        }
        self.logged.insert((id, reason.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::event::VoidListener;
    use alacritty_terminal::index::Point;
    use alacritty_terminal::term::Config;
    use alacritty_terminal::vte::ansi::Processor;
    use chrono::TimeZone;
    use micold_core::terminal_history::text::separator_line;
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

    /// The time of the start a seed is for, in the local zone.
    fn at() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, 2, 14, 31, 0).unwrap()
    }

    /// The separator `seed` writes for `at()` at `columns`, in the dim style.
    fn separator(columns: usize) -> LogicalLine {
        let offset = at().format("%:z");
        let text = separator_line(&format!("2026-10-02 14:31 {offset}"), columns);
        LogicalLine {
            runs: vec![run(text.chars().count() as u32, flags(StyleFlags::DIM))],
            text,
        }
    }

    /// `text` in the default style.
    fn plain(text: &str) -> LogicalLine {
        let chars = text.chars().count() as u32;
        LogicalLine {
            text: text.to_string(),
            runs: if chars == 0 {
                vec![]
            } else {
                vec![run(chars, HistoryStyle::default())]
            },
        }
    }

    fn numbered(lines: std::ops::Range<usize>) -> Vec<LogicalLine> {
        lines.map(|index| plain(&format!("line{index}"))).collect()
    }

    fn history(lines: Vec<LogicalLine>) -> Seed {
        Seed::History {
            snapshot: HistorySnapshot { lines },
            at: at(),
        }
    }

    #[test]
    fn capture_after_a_seed_is_the_input_lines_then_the_separator_in_the_dim_style() {
        let columns = 60;
        let red_bold = HistoryStyle {
            fg: HistoryColor::Basic(1),
            flags: StyleFlags::BOLD,
            ..HistoryStyle::default()
        };
        let mixed = HistoryStyle {
            fg: HistoryColor::Rgb(1, 2, 3),
            bg: HistoryColor::Indexed(17),
            flags: StyleFlags::ITALIC.with(StyleFlags::UNDERLINE),
        };
        let lines = vec![
            plain("plain"),
            LogicalLine {
                text: "red bold".to_string(),
                runs: vec![run(3, red_bold), run(5, HistoryStyle::default())],
            },
            plain(""),
            LogicalLine {
                text: "rgb dim".to_string(),
                runs: vec![run(4, mixed), run(3, fg(HistoryColor::Dim(2)))],
            },
        ];
        let mut term = term(columns, 5, HISTORY_LIMIT);

        seed(&mut term, history(lines.clone()), HISTORY_LIMIT);

        let mut expected = lines;
        expected.push(separator(columns));
        assert_eq!(capture(&term).lines, expected);
    }

    #[test]
    fn after_a_seed_the_screen_is_blank_at_home_with_attributes_reset_and_the_lines_in_history() {
        let (columns, rows) = (20, 4);
        let lines = vec![
            plain("first"),
            LogicalLine {
                text: "red".to_string(),
                runs: vec![run(3, fg(HistoryColor::Basic(1)))],
            },
        ];
        let seeded_rows = lines.len() + 1;
        let mut term = term(columns, rows, HISTORY_LIMIT);

        seed(&mut term, history(lines), HISTORY_LIMIT);

        let grid = term.grid();
        assert_eq!(
            grid.history_size(),
            seeded_rows,
            "lines and separator in history"
        );
        assert!(
            (0..rows).all(|line| shows_nothing(&grid[Line(line as i32)], columns)),
            "screen is blank"
        );
        assert_eq!(
            grid.cursor.point,
            Point::new(Line(0), Column(0)),
            "cursor at home"
        );
        feed(&mut term, "x");
        assert_eq!(
            capture(&term).lines.last(),
            Some(&plain("x")),
            "attributes reset"
        );
    }

    #[test]
    fn a_snapshot_longer_than_the_history_limit_leaves_the_most_recent_lines() {
        const LIMIT: usize = 10;
        // The separator takes one history row, so `LIMIT - 1` lines fill the history exactly.
        let fits = numbered(0..LIMIT - 1);
        let mut at_the_limit = term(20, 3, LIMIT);
        seed(&mut at_the_limit, history(fits.clone()), LIMIT);
        let mut expected = fits;
        expected.push(separator(20));
        assert_eq!(
            capture(&at_the_limit).lines,
            expected,
            "exactly at the limit"
        );

        let mut longer = term(20, 3, LIMIT);
        seed(&mut longer, history(numbered(0..30)), LIMIT);
        let mut expected = numbered(30 - (LIMIT - 1)..30);
        expected.push(separator(20));
        assert_eq!(capture(&longer).lines, expected, "longer than the limit");
    }

    #[test]
    fn seeding_at_a_narrower_width_wraps_and_a_later_capture_gives_the_same_logical_lines() {
        let columns = 6;
        let lines = vec![
            plain("abcdefghijklmnop"),
            LogicalLine {
                text: "redredredred".to_string(),
                runs: vec![run(12, fg(HistoryColor::Basic(1)))],
            },
        ];
        let mut term = term(columns, 4, HISTORY_LIMIT);

        seed(&mut term, history(lines.clone()), HISTORY_LIMIT);

        // 16 characters take 3 rows of 6, 12 take 2, the separator 1.
        assert_eq!(term.grid().history_size(), 3 + 2 + 1, "the lines wrapped");
        let mut expected = lines;
        expected.push(separator(columns));
        assert_eq!(capture(&term).lines, expected);
    }

    #[test]
    fn seed_none_leaves_the_term_untouched() {
        let mut term = term(20, 4, HISTORY_LIMIT);
        feed(&mut term, "before\r\nlast");
        let before = capture(&term);
        let cursor = term.grid().cursor.point;

        seed(&mut term, Seed::None, HISTORY_LIMIT);

        assert_eq!(capture(&term), before);
        assert_eq!(term.grid().cursor.point, cursor);
    }

    #[test]
    fn a_second_seed_after_more_output_keeps_the_first_separator() {
        let columns = 60;
        let mut first = term(columns, 5, HISTORY_LIMIT);
        seed(&mut first, history(vec![plain("earlier")]), HISTORY_LIMIT);
        feed(&mut first, "more\r\n");
        let carried = capture(&first);

        let mut second = term(columns, 5, HISTORY_LIMIT);
        seed(
            &mut second,
            Seed::History {
                snapshot: carried,
                at: at(),
            },
            HISTORY_LIMIT,
        );

        assert_eq!(
            capture(&second).lines,
            [
                plain("earlier"),
                separator(columns),
                plain("more"),
                separator(columns)
            ]
        );
    }
}
