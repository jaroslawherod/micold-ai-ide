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
        let mut end = body + chars[body..].iter().take_while(|c| !c.is_whitespace()).count();
        while end > body && TRAILING_PUNCTUATION.contains(&chars[end - 1]) {
            end -= 1;
        }
        found.push(start..end);
        start = end;
    }
    found
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
}
