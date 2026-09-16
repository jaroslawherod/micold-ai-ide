//! Recognising addresses in plain text (research R3; contract link-recognition §3).

use std::ops::Range;

/// Punctuation that ends a sentence rather than an address (research R3 rule 3).
pub(super) const TRAILING_PUNCTUATION: [char; 8] = ['.', ',', ';', ':', '!', '?', '\'', '*'];

/// The char-index ranges of every address in `text` (FR-001, FR-005).
pub fn detect(text: &str) -> Vec<Range<usize>> {
    let chars: Vec<char> = text.chars().collect();
    let scan = Scan::new(&chars);
    let mut found = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let Some(scheme) = scheme_at(&chars, start) else {
            start += 1;
            continue;
        };
        let body = start + scheme.chars().count();
        let quoted = start
            .checked_sub(1)
            .is_some_and(|before| chars[before] == '\'');
        let end = scan.end(body, quoted);
        if !scan.well_formed(scheme, body, end) {
            start += 1;
            continue;
        }
        found.push(start..end);
        start = end;
    }
    found
}

/// What every candidate asks of the text after it, worked out in one pass each way, so a line
/// packed with schemes that are not addresses costs no more than a plain one (contract L4 bounds
/// the line, not the candidates in it).
struct Scan<'a> {
    chars: &'a [char],
    /// Where an address whose characters start at `i` stops: at a character no address contains
    /// (research R3 rule 2), or at a closing bracket that no opener at or after `i` balances
    /// (rule 4).
    stop: Vec<usize>,
    /// The first `'` at or after `i`. A `"` needs none: it already ends an address.
    next_apostrophe: Vec<usize>,
    /// The first `@` at or after `i`.
    next_at: Vec<usize>,
    /// The first `/` at or after `i`.
    next_slash: Vec<usize>,
    /// How many of the characters right before `i` are trailing punctuation (rule 3).
    punctuation_before: Vec<usize>,
}

impl<'a> Scan<'a> {
    fn new(chars: &'a [char]) -> Self {
        let len = chars.len();
        let mut stop = vec![len; len + 1];
        // Starts whose stop is not yet known, in order, and the openers a scan from them has seen.
        // A stop ends every scan that reaches it, so both clear there.
        let mut pending: Vec<usize> = Vec::new();
        let mut open: Vec<(char, usize)> = Vec::new();
        for (index, &c) in chars.iter().enumerate() {
            pending.push(index);
            // The opener a scan must have seen to read past this character: starts at or before it
            // carry it on; every other pending start stops here.
            let carried = match c {
                c if ends_an_address(c) => None,
                '(' | '[' => {
                    open.push((c, index));
                    continue;
                }
                ')' | ']' => {
                    let opener = if c == ')' { '(' } else { '[' };
                    match open.last() {
                        Some(&(last, at)) if last == opener => {
                            open.pop();
                            Some(at)
                        }
                        _ => None,
                    }
                }
                _ => continue,
            };
            if carried.is_none() {
                open.clear();
            }
            while let Some(&from) = pending.last() {
                if carried.is_some_and(|at| from <= at) {
                    break;
                }
                stop[from] = index;
                pending.pop();
            }
        }
        let next = |wanted: char| {
            let mut next = vec![len; len + 1];
            for index in (0..len).rev() {
                next[index] = if chars[index] == wanted {
                    index
                } else {
                    next[index + 1]
                };
            }
            next
        };
        let mut punctuation_before = vec![0; len + 1];
        for index in 1..=len {
            if TRAILING_PUNCTUATION.contains(&chars[index - 1]) {
                punctuation_before[index] = punctuation_before[index - 1] + 1;
            }
        }
        Self {
            chars,
            stop,
            next_apostrophe: next('\''),
            next_at: next('@'),
            next_slash: next('/'),
            punctuation_before,
        }
    }

    /// Where the address whose characters start at `body` ends: where its scan stops, or at the `'`
    /// that opened right before it (rule 5), less the trailing punctuation before that (rule 3).
    fn end(&self, body: usize, quoted: bool) -> usize {
        let mut end = self.stop[body];
        if quoted {
            end = end.min(self.next_apostrophe[body]);
        }
        end.saturating_sub(self.punctuation_before[end]).max(body)
    }

    /// Whether `body..end` is enough to be an address after `scheme` (research R3 rule 6): a mail
    /// address has text on both sides of its `@`, a file address has a path starting with `/`
    /// after an optional host, and a web address names a host.
    fn well_formed(&self, scheme: &str, body: usize, end: usize) -> bool {
        match scheme {
            "mailto:" => {
                let at = self.next_at[body];
                at > body && at + 1 < end
            }
            "file://" => self.next_slash[body] < end,
            _ => names_a_host(&self.chars[body..end]),
        }
    }
}

/// Whether the text after a web scheme names a host: `localhost`, a dotted name, a bracketed IPv6
/// literal, or any name followed by a port.
fn names_a_host(rest: &[char]) -> bool {
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
pub(super) fn ends_an_address(c: char) -> bool {
    c.is_whitespace()
        || c.is_control()
        || matches!(c, '<' | '>' | '"' | '`' | '{' | '}' | '|' | '\\' | '^')
}

/// The scheme prefix an address starts with at `start`, unless a letter or digit right before it
/// makes it the end of a word (research R3 rule 1).
fn scheme_at(chars: &[char], start: usize) -> Option<&'static str> {
    const SCHEMES: [&str; 4] = ["http://", "https://", "mailto:", "file://"];
    if start > 0 && chars[start - 1].is_ascii_alphanumeric() {
        return None;
    }
    SCHEMES
        .into_iter()
        .find(|scheme| starts_with(&chars[start..], scheme))
}

/// Whether `chars` starts with the ASCII `prefix`, ignoring ASCII case.
fn starts_with(chars: &[char], prefix: &str) -> bool {
    chars.len() >= prefix.len()
        && prefix
            .chars()
            .zip(chars)
            .all(|(p, c)| c.eq_ignore_ascii_case(&p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_packed_with_schemes_that_are_not_addresses_is_scanned_in_time() {
        // The longest logical line `link_at` reads: 129 rows of a 300-column pane.
        const CHARS: usize = 129 * 300;
        for unit in ["mailto:", "http://x/"] {
            let text = unit.repeat(CHARS / unit.len());
            let started = std::time::Instant::now();
            assert_eq!(detect(&text), Vec::<Range<usize>>::new(), "{unit} repeated");
            let elapsed = started.elapsed();
            assert!(
                elapsed < std::time::Duration::from_millis(500),
                "{unit} repeated over {CHARS} chars took {elapsed:?}: every candidate rescans the rest of the line, so a hover stalls"
            );
        }
    }

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

    #[test]
    fn finds_a_mail_address_only_with_text_on_both_sides_of_the_at_sign() {
        assert_eq!(
            found("Write to mailto:team@example.com."),
            ["mailto:team@example.com"],
            "a mail address is found like a web one, without the sentence's full stop"
        );
        assert_eq!(
            found("mailto:@example.com mailto:team@"),
            Vec::<String>::new(),
            "a mail address needs a mailbox before the at sign and a domain after it"
        );
    }

    #[test]
    fn finds_a_file_address_only_when_its_path_starts_with_a_slash() {
        assert_eq!(
            found("Saved to file:///tmp/x and file://localhost/tmp/y."),
            ["file:///tmp/x", "file://localhost/tmp/y"],
            "a file address is a path starting with a slash, after an optional host"
        );
        assert_eq!(
            found("file:// file://tmp"),
            Vec::<String>::new(),
            "a file address with no path, or a host with no path after it, is not an address"
        );
    }

    #[test]
    fn never_finds_an_address_without_a_scheme() {
        assert_eq!(
            found("See example.com/docs or www.example.com for details."),
            Vec::<String>::new(),
            "text that only looks like an address, with no scheme in front, is not one"
        );
    }

    #[test]
    fn never_finds_a_script_or_data_address() {
        assert_eq!(
            found(
                r#"javascript:alert(document.cookie) data:text/html;base64,PHNjcmlwdD4= vbscript:msgbox("x")"#
            ),
            Vec::<String>::new(),
            "an address that runs code or carries its own content is never offered as a link"
        );
    }

    #[test]
    fn matches_the_scheme_whatever_its_case() {
        assert_eq!(
            found("HTTPS://EXAMPLE.COM Mailto:Team@Example.com FILE:///TMP/X"),
            ["HTTPS://EXAMPLE.COM", "Mailto:Team@Example.com", "FILE:///TMP/X"],
            "a scheme is recognised in any mix of upper and lower case, and the address keeps its case"
        );
    }
}
