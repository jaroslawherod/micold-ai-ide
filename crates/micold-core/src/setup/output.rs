//! A setup run's output as lines (feature 1289, research R7, data-model §OutputBuffer).
//!
//! The command's raw stdout and stderr bytes come in as they are read. They are split into lines as
//! a terminal would show them: `\n` and `\r\n` end a line, and a lone `\r` starts the line over, so
//! a progress bar keeps only its last state. ANSI CSI and OSC sequences are removed, invalid UTF-8
//! is decoded lossily, and only the last [`KEEP_LINES`] lines are kept (FR-021). Every line has an
//! index that keeps counting past the cap, so a watcher asks for "everything from line N".

use std::collections::VecDeque;

pub use crate::process::Stream;

/// How many lines a run keeps (FR-021: at least the last 10,000).
pub const KEEP_LINES: usize = 10_000;

/// One stream's unfinished line.
#[derive(Debug, Default, Clone)]
struct Partial {
    bytes: Vec<u8>,
    /// A `\r` was the last byte seen: the next byte either ends the line (`\n`) or starts it over.
    after_cr: bool,
}

/// The kept lines of one run, and the unfinished line of each stream.
#[derive(Debug, Default, Clone)]
pub struct OutputBuffer {
    lines: VecDeque<String>,
    /// The index of `lines[0]`, which is also how many lines were dropped.
    first_index: u64,
    stdout: Partial,
    stderr: Partial,
}

impl OutputBuffer {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Take `bytes` read from `stream`, and return the lines they completed, with their indices.
    pub fn push(&mut self, stream: Stream, bytes: &[u8]) -> Vec<(u64, String)> {
        let mut done = Vec::new();
        for &byte in bytes {
            let partial = self.partial(stream);
            match byte {
                b'\n' => {
                    let line = std::mem::take(&mut partial.bytes);
                    partial.after_cr = false;
                    done.push(self.keep(&line));
                }
                b'\r' => partial.after_cr = true,
                _ => {
                    if std::mem::take(&mut partial.after_cr) {
                        partial.bytes.clear();
                    }
                    partial.bytes.push(byte);
                }
            }
        }
        done
    }

    /// The run ended: keep each stream's unfinished line, stdout's first, and return them.
    pub fn finish(&mut self) -> Vec<(u64, String)> {
        let mut done = Vec::new();
        for stream in [Stream::Stdout, Stream::Stderr] {
            let partial = std::mem::take(self.partial(stream));
            if !partial.bytes.is_empty() {
                done.push(self.keep(&partial.bytes));
            }
        }
        done
    }

    /// The lines from `index` on, and the index of the first one returned: `index`, or the oldest
    /// line kept when `index`'s line was dropped, or [`Self::next_index`] past the end.
    #[must_use]
    pub fn since(&self, index: u64) -> (u64, Vec<String>) {
        let start = index.clamp(self.first_index, self.next_index());
        let skip = usize::try_from(start - self.first_index).unwrap_or(usize::MAX);
        (start, self.lines.iter().skip(skip).cloned().collect())
    }

    /// The index of the oldest line kept, which is also how many lines were dropped.
    #[must_use]
    pub fn first_index(&self) -> u64 {
        self.first_index
    }

    /// The index one past the last line kept.
    #[must_use]
    pub fn next_index(&self) -> u64 {
        self.first_index + self.lines.len() as u64
    }

    fn partial(&mut self, stream: Stream) -> &mut Partial {
        match stream {
            Stream::Stdout => &mut self.stdout,
            Stream::Stderr => &mut self.stderr,
        }
    }

    /// Store one finished line, dropping the oldest past [`KEEP_LINES`].
    fn keep(&mut self, raw: &[u8]) -> (u64, String) {
        let line = String::from_utf8_lossy(&strip_escapes(raw)).into_owned();
        let index = self.next_index();
        self.lines.push_back(line.clone());
        if self.lines.len() > KEEP_LINES {
            self.lines.pop_front();
            self.first_index += 1;
        }
        (index, line)
    }
}

/// `line` without its ANSI escape sequences: CSI (`ESC [` … final byte), OSC (`ESC ]` … BEL or
/// `ESC \`) and the two-byte `ESC` sequences. An unfinished sequence at the end is dropped too.
fn strip_escapes(line: &[u8]) -> Vec<u8> {
    const ESC: u8 = 0x1b;
    const BEL: u8 = 0x07;
    let mut out = Vec::with_capacity(line.len());
    let mut i = 0;
    while i < line.len() {
        if line[i] != ESC {
            out.push(line[i]);
            i += 1;
            continue;
        }
        i += 1;
        match line.get(i) {
            // CSI: parameter and intermediate bytes (0x20–0x3F), then one final byte (0x40–0x7E).
            Some(b'[') => {
                i += 1;
                while line.get(i).is_some_and(|b| (0x20..=0x3f).contains(b)) {
                    i += 1;
                }
                i += 1;
            }
            // OSC: up to BEL or ST (`ESC \`).
            Some(b']') => {
                i += 1;
                while i < line.len() {
                    if line[i] == BEL {
                        i += 1;
                        break;
                    }
                    if line[i] == ESC && line.get(i + 1) == Some(&b'\\') {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            // Any other escape: intermediate bytes (0x20–0x2F), then one final byte.
            Some(_) => {
                while line.get(i).is_some_and(|b| (0x20..=0x2f).contains(b)) {
                    i += 1;
                }
                i += 1;
            }
            None => {}
        }
    }
    out
}
