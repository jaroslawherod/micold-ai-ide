//! `006` BUG-007 (FR-003a): a program that asks the terminal for its default colours is told the
//! colours the pane is painted with under the scheme a client last reported.
//!
//! `claude`'s `auto` theme — and `vim`, `bat`, `delta` — pick a palette from the `OSC 11` reply. The
//! daemon used to answer it from a fixed xterm table whose fallback is light grey, so a dark pane
//! was reported as light and those programs drew near-black text on it.
//!
//! Each test drives a `Term` wired with the daemon's real [`DaemonListener`], over a writer the test
//! keeps a handle to, and reads back what the listener wrote to the "PTY".

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{terminal_defaults, Rgb};
use micold_daemon::terminal::{DaemonListener, SharedWriter, TerminalColors, VtSignals};

/// The bytes a listener wrote back to its PTY.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl Write for Captured {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Captured {
    /// Everything written so far, emptied.
    fn take(&self) -> String {
        String::from_utf8(std::mem::take(&mut *self.0.lock().unwrap())).unwrap()
    }
}

struct Dims;

impl Dimensions for Dims {
    fn total_lines(&self) -> usize {
        24
    }
    fn screen_lines(&self) -> usize {
        24
    }
    fn columns(&self) -> usize {
        80
    }
}

/// A terminal whose listener answers queries for `colors`.
struct QueriedTerm {
    term: Term<DaemonListener>,
    parser: Processor,
    replies: Captured,
}

impl QueriedTerm {
    fn new(colors: TerminalColors) -> Self {
        let replies = Captured::default();
        let writer: SharedWriter = Arc::new(Mutex::new(Box::new(replies.clone())));
        let size = Arc::new(Mutex::new(WindowSize {
            num_lines: 24,
            num_cols: 80,
            cell_width: 0,
            cell_height: 0,
        }));
        let listener = DaemonListener::new(writer, size, VtSignals::default(), colors);
        Self {
            term: Term::new(Config::default(), &Dims, listener),
            parser: Processor::new(),
            replies,
        }
    }

    /// Feed `query` to the emulator and return what the listener wrote back.
    fn ask(&mut self, query: &str) -> String {
        self.parser.advance(&mut self.term, query.as_bytes());
        self.replies.take()
    }
}

/// `colors` set to `scheme`.
fn colors_for(scheme: ColorScheme) -> TerminalColors {
    let colors = TerminalColors::default();
    colors.set(scheme);
    colors
}

/// The xterm reply form for dynamic colour `code`: 16-bit channels, each byte doubled.
fn reply(code: u8, rgb: Rgb, terminator: &str) -> String {
    format!(
        "\x1b]{code};rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}{terminator}",
        r = rgb.r,
        g = rgb.g,
        b = rgb.b
    )
}

const BEL: &str = "\x07";
const ST: &str = "\x1b\\";

#[test]
fn a_dark_pane_answers_a_background_query_with_the_dark_surface() {
    let mut term = QueriedTerm::new(colors_for(ColorScheme::Dark));

    assert_eq!(
        term.ask("\x1b]11;?\x07"),
        reply(11, terminal_defaults(ColorScheme::Dark).background, BEL),
        "a program asking a dark pane for its background must be told it is dark"
    );
}

#[test]
fn a_dark_pane_answers_a_foreground_query_with_the_dark_on_surface() {
    let mut term = QueriedTerm::new(colors_for(ColorScheme::Dark));

    assert_eq!(
        term.ask("\x1b]10;?\x1b\\"),
        reply(10, terminal_defaults(ColorScheme::Dark).foreground, ST),
        "a program asking a dark pane for its default text colour must get the text colour drawn on it"
    );
}

/// The client draws the cursor block in the default foreground, so that is the cursor colour.
#[test]
fn a_dark_pane_answers_a_cursor_query_with_the_dark_on_surface() {
    let mut term = QueriedTerm::new(colors_for(ColorScheme::Dark));

    assert_eq!(
        term.ask("\x1b]12;?\x07"),
        reply(12, terminal_defaults(ColorScheme::Dark).foreground, BEL),
        "a program asking a dark pane for its cursor colour must get the colour the cursor is drawn in"
    );
}

/// The other side of U1: the answer follows the scheme, not a constant that happens to be dark.
#[test]
fn a_light_pane_answers_a_background_query_with_the_light_surface() {
    let mut term = QueriedTerm::new(colors_for(ColorScheme::Light));

    assert_eq!(
        term.ask("\x1b]11;?\x07"),
        reply(11, terminal_defaults(ColorScheme::Light).background, BEL),
        "a program asking a light pane for its background must be told it is light"
    );
}

/// A scheme reported while a session is running reaches that session's next query: the listener reads
/// the shared value when it answers, not when it was built.
#[test]
fn a_scheme_changed_after_the_listener_was_built_answers_the_next_query() {
    let colors = colors_for(ColorScheme::Dark);
    let mut term = QueriedTerm::new(colors.clone());
    term.ask("\x1b]11;?\x07");

    colors.set(ColorScheme::Light);

    assert_eq!(
        term.ask("\x1b]11;?\x07"),
        reply(11, terminal_defaults(ColorScheme::Light).background, BEL),
        "after the user switches to light, a running program's next query must be told light"
    );
}

/// Before any client has reported, the answer is the light scheme's — the scheme the application
/// resolves when it knows nothing about the OS preference (003 FR-018).
#[test]
fn before_any_scheme_is_reported_the_answer_is_light() {
    let mut term = QueriedTerm::new(TerminalColors::default());

    assert_eq!(
        term.ask("\x1b]11;?\x07"),
        reply(11, terminal_defaults(ColorScheme::Light).background, BEL),
        "with no scheme reported yet, the service answers for the light scheme"
    );
}

/// Palette queries are not the defaults: `OSC 4` keeps the xterm table whatever the scheme.
#[test]
fn a_palette_query_is_still_answered_from_the_xterm_table() {
    const XTERM_RED: Rgb = Rgb { r: 205, g: 0, b: 0 };
    for scheme in [ColorScheme::Light, ColorScheme::Dark] {
        let mut term = QueriedTerm::new(colors_for(scheme));

        assert_eq!(
            term.ask("\x1b]4;1;?\x07"),
            format!(
                "\x1b]4;1;rgb:{r:02x}{r:02x}/{g:02x}{g:02x}/{b:02x}{b:02x}{BEL}",
                r = XTERM_RED.r,
                g = XTERM_RED.g,
                b = XTERM_RED.b
            ),
            "palette entry 1 is xterm red under {scheme:?}"
        );
    }
}
