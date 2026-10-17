//! An agent's reply as it is shown (feature 946, data-model § Core: `review::reply`): control
//! characters made visible, and long replies shortened for cards and rows.

/// How many characters of a reply a card or a panel row shows before "Show all" (Edge Cases
/// "very long reply").
pub const CARD_CHARS: usize = 4_000;

/// `reply` as it is drawn: `\n` kept, every other C0 control character shown as its Control
/// Pictures glyph (U+2400 + code, so ESC → `␛`, TAB → `␉`) and DEL as `␡`, so a reply can never
/// act as a control code (Edge Cases "control characters").
pub fn display(reply: &str) -> String {
    reply
        .chars()
        .map(|c| match c {
            '\n' => c,
            '\u{0}'..='\u{1f}' => char::from_u32(0x2400 + u32::from(c)).unwrap_or(c),
            '\u{7f}' => '\u{2421}',
            _ => c,
        })
        .collect()
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
}
