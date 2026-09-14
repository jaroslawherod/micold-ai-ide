//! Recognising addresses in plain text (research R3; contract link-recognition §3).

use std::ops::Range;

/// Punctuation that ends a sentence rather than an address (research R3 rule 3).
const TRAILING_PUNCTUATION: [char; 8] = ['.', ',', ';', ':', '!', '?', '\'', '*'];

/// The char-index ranges of every address in `text` (FR-001, FR-005).
pub fn detect(text: &str) -> Vec<Range<usize>> {
    const SCHEMES: [&str; 2] = ["http://", "https://"];
    let chars: Vec<char> = text.chars().collect();
    let mut found = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let Some(scheme) = SCHEMES.iter().find(|s| starts_with(&chars[start..], s)) else {
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
        found.push(start..end);
        start = end;
    }
    found
}

/// Where the characters of an address starting at `from` end: at whitespace, at a closing
/// bracket that no opener inside the address balances (research R3 rule 4), or at the `quote`
/// that opened right before the address (rule 5).
fn scan_end(chars: &[char], from: usize, quote: Option<char>) -> usize {
    let mut open = Vec::new();
    for (index, &c) in chars.iter().enumerate().skip(from) {
        match c {
            c if c.is_whitespace() || Some(c) == quote => return index,
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
}
