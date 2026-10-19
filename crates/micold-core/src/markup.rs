//! Agent markup removed from prompt and todo text (feature 1120, research R7): the words of
//! `<agent-message from="…">Continue M2</agent-message>` on one line, or nothing when only markup
//! is left (FR-011, FR-013).
//!
//! Pure: no I/O and no logging. The text is a user's prompt, so nothing here may be logged
//! (FR-020).

/// The words of `text` with every tag removed and whitespace runs collapsed to one space; `None`
/// when nothing is left, or when removal left a tag behind (`<<b>a>`) (contract C1).
///
/// A tag is `<`, an optional `/`, an ASCII letter, then every character up to and including the
/// next `>`, provided no `<` comes first. `a < b` and `x<3` are not tags and stay as written.
pub fn readable(text: &str) -> Option<String> {
    let mut words = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('<') {
        words.push_str(&rest[..at]);
        match tag_len(&rest[at..]) {
            Some(len) => rest = &rest[at + len..],
            None => {
                words.push('<');
                rest = &rest[at + 1..];
            }
        }
    }
    words.push_str(rest);
    let words = words.split_whitespace().collect::<Vec<_>>().join(" ");
    let tag_left = words
        .match_indices('<')
        .any(|(at, _)| tag_len(&words[at..]).is_some());
    (!words.is_empty() && !tag_left).then_some(words)
}

/// The length in bytes of the tag `s` starts with, or `None` when `s` does not start with one.
fn tag_len(s: &str) -> Option<usize> {
    let body = s.strip_prefix('<')?;
    let name = body.strip_prefix('/').unwrap_or(body);
    if !name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let close = body.find(['<', '>'])?;
    (body.as_bytes()[close] == b'>').then_some(close + 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn some(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    #[test]
    fn a_wrapper_tag_is_removed_and_its_words_kept() {
        assert_eq!(
            readable(r#"<agent-message from="orchestrator">Continue M2</agent-message>"#),
            some("Continue M2")
        );
    }

    #[test]
    fn nested_and_unbalanced_tags_are_removed() {
        assert_eq!(
            readable(r#"<agent-message from="a"><agent-message from="b">Hi"#),
            some("Hi")
        );
    }

    #[test]
    fn a_multi_line_wrapper_and_multi_line_text_read_on_one_line() {
        assert_eq!(
            readable("<agent-message from=\"x\">\nContinue M2\n</agent-message>"),
            some("Continue M2")
        );
        assert_eq!(readable("line one\n  line two"), some("line one line two"));
    }

    #[test]
    fn a_less_than_sign_that_starts_no_tag_stays() {
        for text in ["a < b", "x<3", "if a<b"] {
            assert_eq!(readable(text), some(text), "{text}");
        }
    }

    #[test]
    fn nothing_readable_is_none() {
        for text in [r#"<agent-message from="x"></agent-message>"#, "   ", ""] {
            assert_eq!(readable(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_tag_left_after_removal_is_none() {
        assert_eq!(readable("<<b>a>"), None);
    }

    #[test]
    fn a_tag_runs_to_the_next_closing_bracket() {
        assert_eq!(readable("a<b and c>d"), some("ad"));
    }
}
