//! Shared test support for the framer tests: a deterministically-driven VT `Term` (no PTY, no
//! threads, no timing) so grid-framing behaviour can be asserted exactly.

use std::sync::{Arc, Mutex};

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use micold_daemon::terminal::{DaemonListener, SharedTerm, SharedWriter, VtSignals};

struct Dims {
    rows: usize,
    cols: usize,
}

impl Dimensions for Dims {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// A VT `Term` wired exactly like a real session's (same `DaemonListener`), but fed bytes directly
/// instead of from a PTY. The parser persists across [`Self::feed`] calls, so split escape sequences
/// behave.
pub struct DrivenTerm {
    pub term: SharedTerm,
    parser: Processor,
}

impl DrivenTerm {
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Self {
        let writer: SharedWriter = Arc::new(Mutex::new(Box::new(Vec::new())));
        let size = Arc::new(Mutex::new(WindowSize {
            num_lines: rows as u16,
            num_cols: cols as u16,
            cell_width: 0,
            cell_height: 0,
        }));
        let listener = DaemonListener::new(writer, size, VtSignals::default());
        let config = Config {
            scrolling_history: scrollback,
            ..Config::default()
        };
        let term = Arc::new(FairMutex::new(Term::new(
            config,
            &Dims { rows, cols },
            listener,
        )));
        Self {
            term,
            parser: Processor::new(),
        }
    }

    /// Feed raw VT bytes into the emulator.
    pub fn feed(&mut self, bytes: &str) {
        let mut term = self.term.lock();
        self.parser.advance(&mut *term, bytes.as_bytes());
    }

    /// Feed `n` numbered lines (`line<start>\r\n` …), the common scrolling workload.
    pub fn feed_lines(&mut self, start: usize, n: usize) {
        let mut s = String::new();
        for i in start..start + n {
            s.push_str(&format!("line{i}\r\n"));
        }
        self.feed(&s);
    }
}
