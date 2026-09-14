//! Recognising addresses in plain text (research R3; contract link-recognition §3).

use std::ops::Range;

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
        let end = body + chars[body..].iter().take_while(|c| !c.is_whitespace()).count();
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
}
