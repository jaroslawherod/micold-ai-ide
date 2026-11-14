//! #1720: a program that copies with OSC 52 (an AI CLI's own selection) reaches the daemon as a
//! pending clipboard store, so it can be sent on to the window showing the session.
//!
//! Drives a `Term` wired with the daemon's real [`DaemonListener`] and reads the store back from
//! the shared [`VtSignals`].

use std::io;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use micold_daemon::terminal::{DaemonListener, SharedWriter, TerminalColors, VtSignals};

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

/// Feed `output` to a terminal and return what its signals hold afterwards.
fn feed(output: &str) -> VtSignals {
    let signals = VtSignals::default();
    let writer: SharedWriter = Arc::new(Mutex::new(Box::new(io::sink())));
    let size = Arc::new(Mutex::new(WindowSize {
        num_lines: 24,
        num_cols: 80,
        cell_width: 0,
        cell_height: 0,
    }));
    let listener = DaemonListener::new(writer, size, signals.clone(), TerminalColors::default());
    let mut term = Term::new(Config::default(), &Dims, listener);
    let mut parser: Processor = Processor::new();
    parser.advance(&mut term, output.as_bytes());
    signals
}

#[test]
fn an_osc52_store_is_kept_decoded_and_taken_once() {
    // "hello" in base64.
    let signals = feed("\x1b]52;c;aGVsbG8=\x07");

    assert_eq!(signals.take_clipboard().as_deref(), Some("hello"));
    assert_eq!(signals.take_clipboard(), None, "a store is taken once");
}

#[test]
fn the_latest_store_wins() {
    let signals = feed("\x1b]52;c;Zmlyc3Q=\x07\x1b]52;c;c2Vjb25k\x07");

    assert_eq!(signals.take_clipboard().as_deref(), Some("second"));
}

#[test]
fn an_osc52_read_request_is_never_answered() {
    let signals = feed("\x1b]52;c;?\x07");

    assert_eq!(signals.take_clipboard(), None);
}
