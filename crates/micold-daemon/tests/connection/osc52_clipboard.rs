//! #1720: a program that copies with OSC 52 (an AI CLI's own selection) reaches the daemon as a
//! pending clipboard store, so it can be sent on to the window showing the session.
//!
//! Drives a `Term` wired with the daemon's real [`DaemonListener`] and reads the store back from
//! the shared [`VtSignals`].

use std::io;
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::WindowSize;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::term::{Config, Osc52, Term};
use alacritty_terminal::vte::ansi::Processor;
use micold_daemon::terminal::{
    DaemonListener, SharedWriter, TerminalColors, VtSignals, MAX_CLIPBOARD_STORE,
};

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

/// What the terminal wrote back to the program.
#[derive(Clone, Default)]
struct Replies(Arc<Mutex<Vec<u8>>>);

impl io::Write for Replies {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Feed `output` to a terminal and return what its signals hold afterwards.
fn feed(output: &str) -> VtSignals {
    feed_with(Config::default(), output).0
}

/// Feed `output` to a terminal built with `config`; return its signals and what it wrote back.
fn feed_with(config: Config, output: &str) -> (VtSignals, Vec<u8>) {
    let signals = VtSignals::default();
    let replies = Replies::default();
    let writer: SharedWriter = Arc::new(Mutex::new(Box::new(replies.clone())));
    let size = Arc::new(Mutex::new(WindowSize {
        num_lines: 24,
        num_cols: 80,
        cell_width: 0,
        cell_height: 0,
    }));
    let listener = DaemonListener::new(writer, size, signals.clone(), TerminalColors::default());
    let mut term = Term::new(config, &Dims, listener);
    let mut parser: Processor = Processor::new();
    parser.advance(&mut term, output.as_bytes());
    let written = replies.0.lock().unwrap().clone();
    (signals, written)
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
    // Even with reads enabled in the emulator, the daemon's listener answers none: nothing is
    // written back to the program, and nothing is stored.
    let config = Config {
        osc52: Osc52::CopyPaste,
        ..Config::default()
    };
    let (signals, written) = feed_with(config, "\x1b]52;c;?\x07");

    assert!(
        written.is_empty(),
        "the clipboard was answered: {written:?}"
    );
    assert_eq!(signals.take_clipboard(), None);
}

#[test]
fn a_store_too_large_for_one_frame_is_dropped() {
    use base64::Engine as _;
    let text = "x".repeat(MAX_CLIPBOARD_STORE + 1);
    let encoded = base64::engine::general_purpose::STANDARD.encode(&text);
    let signals = feed(&format!("\x1b]52;c;{encoded}\x07"));

    assert_eq!(signals.take_clipboard(), None);
}
