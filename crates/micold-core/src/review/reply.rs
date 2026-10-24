//! An agent's reply as it is shown (feature 946, data-model § Core: `review::reply`): control
//! characters made visible, and long replies shortened for cards and rows. Feature 1270 caps a
//! card's reply at [`CARD_LINES`] drawn lines ([`card_preview`], FR-021, research R7).

use std::borrow::Cow;

use crate::tooltip;

/// How many characters of a reply a card or a panel row shows before "Show all" (Edge Cases
/// "very long reply").
pub const CARD_CHARS: usize = 4_000;

/// How many drawn lines of a reply a comment card shows before "Show all" (feature 1270, FR-021).
pub const CARD_LINES: usize = 4;

/// `reply` as it is drawn: `\n` kept, every other C0 control character shown as its Control
/// Pictures glyph (U+2400 + code, so ESC → `␛`, TAB → `␉`) and DEL as `␡`, so a reply can never
/// act as a control code (Edge Cases "control characters").
pub fn display(reply: &str) -> String {
    reply.chars().map(shown_as).collect()
}

fn shown_as(c: char) -> char {
    match c {
        '\n' => c,
        '\u{0}'..='\u{1f}' => char::from_u32(0x2400 + u32::from(c)).unwrap_or(c),
        '\u{7f}' => '\u{2421}',
        _ => c,
    }
}

/// What a comment card shows of a reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview<'a> {
    /// The reply as drawn, shortened when `cut`.
    pub text: Cow<'a, str>,
    /// Whether it was shortened, so the card offers "Show all".
    pub cut: bool,
}

/// `reply` as a card shows it (FR-021, research R7): [`display`]ed, then cut to [`CARD_CHARS`]
/// characters, then to [`CARD_LINES`] lines by `lines_of`, the card's measure of how many lines a
/// text takes at its width (a long line that wraps counts the lines it takes).
pub fn card_preview(reply: &str, lines_of: impl Fn(&str) -> usize) -> Preview<'_> {
    let shown: Cow<'_, str> = if reply.chars().any(|c| shown_as(c) != c) {
        Cow::Owned(display(reply))
    } else {
        Cow::Borrowed(reply)
    };
    let (capped, capped_cut) = match shown.char_indices().nth(CARD_CHARS).map(|(at, _)| at) {
        Some(at) => match shown {
            Cow::Borrowed(text) => (Cow::Borrowed(&text[..at]), true),
            Cow::Owned(mut text) => {
                text.truncate(at);
                (Cow::Owned(text), true)
            }
        },
        None => (shown, false),
    };
    let clamped = match tooltip::clamp_to_lines(&capped, CARD_LINES, lines_of) {
        Cow::Borrowed(_) => None,
        Cow::Owned(text) => Some(text),
    };
    match clamped {
        Some(text) => Preview {
            text: Cow::Owned(text),
            cut: true,
        },
        None => Preview {
            text: capped,
            cut: capped_cut,
        },
    }
}

/// The first `max_chars` characters of `reply` and `true`, or `reply` whole and `false` when it
/// is no longer than that. Counts characters, so it never splits one.
pub fn shortened(reply: &str, max_chars: usize) -> (String, bool) {
    match reply.char_indices().nth(max_chars) {
        Some((cut, _)) => (reply[..cut].to_owned(), true),
        None => (reply.to_owned(), false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_keeps_line_breaks_and_shows_other_control_characters() {
        assert_eq!(display("a\nb"), "a\nb");
        assert_eq!(display("\u{1b}[31mred\tx"), "␛[31mred␉x");
        assert_eq!(display("\u{0}\r\u{1f}\u{7f}"), "␀␍␟␡");
        assert_eq!(display("plain é ✓ text"), "plain é ✓ text");
    }

    #[test]
    fn shortened_cuts_after_max_chars_and_never_splits_a_character() {
        let at = "é".repeat(CARD_CHARS);
        assert_eq!(shortened(&at, CARD_CHARS), (at.clone(), false));
        let over = format!("{at}✓");
        assert_eq!(shortened(&over, CARD_CHARS), (at, true));
        assert_eq!(shortened("", CARD_CHARS), (String::new(), false));
    }

    /// Lines `text` takes when drawn `width` characters wide: each `\n` line, wrapped.
    fn wrapped(width: usize) -> impl Fn(&str) -> usize {
        move |text: &str| {
            text.split('\n')
                .map(|line| line.chars().count().div_ceil(width).max(1))
                .sum()
        }
    }

    #[test]
    fn a_reply_of_at_most_four_lines_is_kept_whole() {
        for reply in ["one", "1\n2\n3\n4", "short\n\nlines"] {
            let preview = card_preview(reply, wrapped(40));
            assert_eq!(preview.text, reply);
            assert!(!preview.cut, "{reply:?}");
        }
    }

    #[test]
    fn a_long_line_counts_the_lines_it_wraps_to() {
        let four = "x".repeat(40);
        let preview = card_preview(&four, wrapped(10));
        assert_eq!((preview.text.as_ref(), preview.cut), (four.as_str(), false));

        let five = "word ".repeat(10);
        let preview = card_preview(&five, wrapped(10));
        assert!(preview.cut);
        assert!(preview.text.ends_with('…'), "{:?}", preview.text);
        assert!(wrapped(10)(&preview.text) <= CARD_LINES);
    }

    #[test]
    fn over_four_lines_is_cut() {
        let preview = card_preview("1\n2\n3\n4\n5", wrapped(40));
        assert!(preview.cut);
        assert_eq!(preview.text, "1\n2\n3\n4…");
    }

    #[test]
    fn the_character_cap_still_applies_and_sets_cut() {
        let long = "é".repeat(CARD_CHARS + 1);
        let preview = card_preview(&long, |_| 1);
        assert!(preview.cut);
        assert_eq!(preview.text.chars().count(), CARD_CHARS);
    }

    #[test]
    fn control_characters_are_shown_as_display_shows_them() {
        let preview = card_preview("\u{1b}[31mred\tx", wrapped(40));
        assert_eq!(preview.text, "␛[31mred␉x");
        assert!(!preview.cut);
    }
}
