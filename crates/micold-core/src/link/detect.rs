//! Recognising addresses in plain text (research R3; contract link-recognition §3).

use std::ops::Range;

/// Punctuation that ends a sentence rather than an address (research R3 rule 3).
const TRAILING_PUNCTUATION: [char; 8] = ['.', ',', ';', ':', '!', '?', '\'', '*'];

/// The char-index ranges of every address in `text` (FR-001, FR-005).
pub fn detect(text: &str) -> Vec<Range<usize>> {
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let Some(scheme) = scheme_at(&chars, start) else {
            start += 1;
            continue;
        };
        let body = start + scheme.chars().count();
        let quote = start
            .checked_sub(1)
            .map(|before| chars[before])
            .filter(|c| matches!(c, '"' | '\''));
        let mut end = scan_end(&chars, body, quote);
        while end > body && TRAILING_PUNCTUATION.contains(&chars[end - 1]) {
            end -= 1;
        }
        if !well_formed(&chars[body..end]) {
            start += 1;
            continue;
        }
        found.push(start..end);
        start = end;
    }
    found
}

/// Where the characters of an address starting at `from` end: at a character no address contains
/// (research R3 rule 2), at a closing bracket that no opener inside the address balances (rule 4),
/// or at the `quote` that opened right before the address (rule 5).
fn scan_end(chars: &[char], from: usize, quote: Option<char>) -> usize {
    let mut open = Vec::new();
    for (index, &c) in chars.iter().enumerate().skip(from) {
        match c {
            c if ends_an_address(c) || Some(c) == quote => return index,
            '(' | '[' => open.push(c),
            ')' | ']' => {
                let opener = if c == ')' { '(' } else { '[' };
                if open.last() != Some(&opener) {
                    return index;
                }
                open.pop();
            }
            _ => {}
        }
    }
    chars.len()
}

/// Whether the text after a web scheme names a host: `localhost`, a dotted name, a bracketed IPv6
/// literal, or any name followed by a port (research R3 rule 6).
fn well_formed(rest: &[char]) -> bool {
    let authority: String = rest
        .iter()
        .take_while(|c| !matches!(c, '/' | '?' | '#'))
        .collect();
    let host_and_port = authority.rsplit('@').next().unwrap_or_default();
    if host_and_port.starts_with('[') {
        return host_and_port.contains(']');
    }
    let (host, port) = match host_and_port.split_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (host_and_port, None),
    };
    let has_port =
        port.is_some_and(|port| !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()));
    !host.is_empty() && (host.eq_ignore_ascii_case("localhost") || host.contains('.') || has_port)
}

/// A character no address contains (research R3 rule 2).
fn ends_an_address(c: char) -> bool {
    c.is_whitespace()
        || c.is_control()
        || matches!(c, '<' | '>' | '"' | '`' | '{' | '}' | '|' | '\\' | '^')
}

/// The scheme prefix an address starts with at `start`, unless a letter or digit right before it
/// makes it the end of a word (research R3 rule 1).
fn scheme_at(chars: &[char], start: usize) -> Option<&'static str> {
    const SCHEMES: [&str; 2] = ["http://", "https://"];
    if start > 0 && chars[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    SCHEMES
        .into_iter()
        .find(|scheme| starts_with(&chars[start..], scheme))
}

fn starts_with(chars: &[char], prefix: &str) -> bool {
    let prefix: Vec<char> = prefix.chars().collect();
    chars.len() >= prefix.len() && chars[..prefix.len()] == prefix[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text of each range `detect` finds, so a failure reads as the addresses themselves.
    fn found(text: &str) -> Vec<String> {
        detect(text)
            .into_iter()
            .map(|range| text.chars().skip(range.start).take(range.len()).collect())
            .collect()
    }

    #[test]
    fn finds_an_address_inside_a_sentence() {
        assert_eq!(
            found("See https://example.com/docs/page.html for details."),
            ["https://example.com/docs/page.html"],
            "the address is found alone: not the words around it, nor the sentence's full stop"
        );
    }

    #[test]
    fn trims_trailing_punctuation_repeatedly() {
        assert_eq!(
            found("Is it https://example.com/docs/page.html?!."),
            ["https://example.com/docs/page.html"],
            "sentence punctuation after an address is not part of it, however much of it there is"
        );
        assert_eq!(
            found("https://example.com/a.,;:!?'*"),
            ["https://example.com/a"],
            "each of . , ; : ! ? ' * is trimmed from the end"
        );
    }

    #[test]
    fn keeps_a_closing_bracket_balanced_inside_the_address() {
        assert_eq!(
            found("Read https://example.com/a_(b)."),
            ["https://example.com/a_(b)"],
            "a closing bracket that closes one opened inside the address is part of it"
        );
    }

    #[test]
    fn stops_before_an_unbalanced_closing_bracket() {
        assert_eq!(
            found("(https://example.com/a)"),
            ["https://example.com/a"],
            "a closing bracket with no opener inside the address belongs to the text around it"
        );
        assert_eq!(
            found("(https://example.com/a_(b))"),
            ["https://example.com/a_(b)"],
            "only the unbalanced closer is excluded, not the balanced one before it"
        );
    }

    #[test]
    fn excludes_an_enclosing_quote() {
        assert_eq!(
            found(r#"url = "https://example.com""#),
            ["https://example.com"],
            "an address opened right after a double quote ends before the closing quote"
        );
        assert_eq!(
            found("'https://example.com/a'b'"),
            ["https://example.com/a"],
            "an address opened right after a single quote ends at the next one, not the last"
        );
    }

    #[test]
    fn finds_only_the_address_of_a_markdown_link() {
        assert_eq!(
            found("See [text](https://x.example) here"),
            ["https://x.example"],
            "neither the link text nor the brackets around the address are part of it"
        );
    }

    #[test]
    fn finds_only_the_address_inside_angle_brackets() {
        assert_eq!(
            found("Homepage: <https://x.example>"),
            ["https://x.example"],
            "the angle brackets that mark an address in mail and Markdown are not part of it"
        );
    }

    #[test]
    fn rejects_a_scheme_preceded_by_a_letter_or_digit() {
        assert_eq!(
            found("xhttps://a.example 2http://a.example"),
            Vec::<String>::new(),
            "a scheme glued to the end of a word is part of that word, not the start of an address"
        );
    }

    #[test]
    fn stops_at_whitespace_control_characters_and_characters_no_address_contains() {
        for stop in [
            ' ', '\t', '\u{7}', '\u{1b}', '<', '>', '"', '`', '{', '}', '|', '\\', '^',
        ] {
            assert_eq!(
                found(&format!("https://a.example/x{stop}y")),
                ["https://a.example/x"],
                "{stop:?} cannot appear in an address, so the address ends before it"
            );
        }
    }

    #[test]
    fn accepts_a_web_host_only_when_it_is_localhost_dotted_bracketed_or_has_a_port() {
        assert_eq!(
            found("http://localhost/ https://a.example http://[::1]/x http://intranet:8080/x"),
            [
                "http://localhost/",
                "https://a.example",
                "http://[::1]/x",
                "http://intranet:8080/x"
            ],
            "localhost, a dotted name, a bracketed IPv6 literal and a name with a port are hosts"
        );
        assert_eq!(
            found("http://intranet/x"),
            Vec::<String>::new(),
            "a dotless name with no port is too likely to be prose to be an address"
        );
    }

    #[test]
    fn rejects_a_scheme_alone_and_a_host_broken_by_a_space() {
        assert_eq!(
            found("https:// and http://exa mple"),
            Vec::<String>::new(),
            "a scheme with no host, and a host cut short by a space, are not addresses"
        );
    }
}
