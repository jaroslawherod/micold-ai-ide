//! The rules of a tooltip that waits for the cursor to rest (feature 038).
//!
//! Two rules, both free of rendering: when such a tooltip is open ([`RestTimer`]), and how a text
//! is cut to a number of lines ([`clamp_to_lines`]). The client's tooltip widget feeds them the
//! cursor, the clock and a measure of its own.

use std::borrow::Cow;
use std::time::{Duration, Instant};

/// How far, in logical pixels, the cursor may move from where it came to rest and still count as
/// resting. A distance of exactly this much is within tolerance.
pub const REST_TOLERANCE: f32 = 4.0;

/// Where a rest-delay tooltip is in its wait.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RestTimer {
    /// The cursor is not over the trigger.
    #[default]
    Away,
    /// The cursor came to rest at `anchor` at `since` and has not left its tolerance.
    Waiting {
        /// Where the cursor came to rest.
        anchor: (f32, f32),
        /// When it did.
        since: Instant,
    },
    /// The cursor rested for the delay: the tooltip is open until the cursor leaves.
    Open,
    /// The trigger was pressed: closed until the cursor has left.
    Spent,
}

/// What the widget does after an observation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rest {
    /// Whether the tooltip is open.
    pub open: bool,
    /// When to look again though nothing else happens: the end of the wait, only while waiting.
    pub wake_at: Option<Instant>,
}

impl RestTimer {
    /// Take in where the cursor is — `Some` only when it is over the trigger — at `now`.
    ///
    /// The anchor is where the cursor *first* came to rest: movement inside the tolerance does not
    /// move it, so a cursor creeping a pixel at a time is seen leaving it.
    pub fn observe(&mut self, cursor: Option<(f32, f32)>, now: Instant, delay: Duration) -> Rest {
        *self = match (*self, cursor) {
            (_, None) => Self::Away,
            (Self::Away, Some(at)) => Self::Waiting {
                anchor: at,
                since: now,
            },
            (Self::Waiting { anchor, since }, Some(at)) => {
                if distance(anchor, at) > REST_TOLERANCE {
                    Self::Waiting {
                        anchor: at,
                        since: now,
                    }
                } else if now.saturating_duration_since(since) >= delay {
                    Self::Open
                } else {
                    Self::Waiting { anchor, since }
                }
            }
            (held @ (Self::Open | Self::Spent), Some(_)) => held,
        };
        Rest {
            open: matches!(self, Self::Open),
            wake_at: match *self {
                Self::Waiting { since, .. } => Some(since + delay),
                _ => None,
            },
        }
    }

    /// The trigger was pressed: closed until the cursor has left it.
    pub fn press(&mut self) {
        *self = Self::Spent;
    }

    /// The trigger now describes something else: closed, and the wait starts from nothing.
    pub fn reset(&mut self) {
        *self = Self::Away;
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// The mark a cut text ends in.
const ELLIPSIS: char = '…';

/// How far back from the cut a word boundary is looked for. A text with no space that near is cut
/// where it is: backing up further would throw away most of a line for the sake of one long word.
const WORD_BACKUP: usize = 24;

/// `text` cut to at most `max_lines` lines by the caller's measure, ending in `…` when cut.
///
/// `lines_of` says how many lines a text takes; it is assumed not to shrink as a text grows, so
/// the longest prefix that fits with its `…` is found by binary search over character boundaries.
/// A text that fits is returned as it came, borrowed.
pub fn clamp_to_lines(
    text: &str,
    max_lines: usize,
    lines_of: impl Fn(&str) -> usize,
) -> Cow<'_, str> {
    if lines_of(text) <= max_lines {
        return Cow::Borrowed(text);
    }
    let ends: Vec<usize> = text
        .char_indices()
        .map(|(at, c)| at + c.len_utf8())
        .collect();
    let with_ellipsis = |end: usize| format!("{}{ELLIPSIS}", &text[..end]);

    // `fits` characters are known to fit; `over` are known not to (the whole text does not).
    let (mut fits, mut over) = (0, ends.len());
    while over - fits > 1 {
        let mid = fits + (over - fits) / 2;
        if lines_of(&with_ellipsis(ends[mid - 1])) <= max_lines {
            fits = mid;
        } else {
            over = mid;
        }
    }
    let cut = if fits == 0 { 0 } else { ends[fits - 1] };

    // Mid-word: back up to the last space, when one is near.
    let mid_word = !text[cut..].starts_with(char::is_whitespace)
        && !text[..cut].ends_with(char::is_whitespace);
    let near = if fits > WORD_BACKUP {
        ends[fits - WORD_BACKUP - 1]
    } else {
        0
    };
    let cut = match text[near..cut].rfind(char::is_whitespace) {
        Some(space) if mid_word => near + space,
        _ => cut,
    };

    let kept = text[..cut].trim_end_matches(|c: char| c.is_whitespace() || c == ELLIPSIS);
    Cow::Owned(format!("{kept}{ELLIPSIS}"))
}
