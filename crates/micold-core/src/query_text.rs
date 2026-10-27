//! Query-text helpers both advanced selectors share (feature 1400, research R12): the cleaning a
//! typed query gets before it is checked, and the cut a provider's refusal message gets before it
//! is shown. Jira's advanced JQL (feature 1186, `crate::jira_query`) and GitHub's advanced search
//! (`crate::github_filter`, `specs/1400-github-selector-filters/contracts/search-rules.md`) use the
//! same rule, so it lives here once.

/// Characters of one refusal message kept before it is cut (1186 contracts/jql-rules.md §4,
/// 1400 research R7).
pub const MAX_MESSAGE_CHARS: usize = 300;

/// `\n`, `\r` and `\t` become a space, every other control character goes, and the ends are
/// trimmed (1186 research R3).
pub fn clean(raw: &str) -> String {
    raw.chars()
        .filter_map(|c| match c {
            '\n' | '\r' | '\t' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// `message` cut at [`MAX_MESSAGE_CHARS`] characters, with `…` when it was longer.
pub fn cut_message(message: String) -> String {
    match message.char_indices().nth(MAX_MESSAGE_CHARS) {
        Some((at, _)) => format!("{}…", &message[..at]),
        None => message,
    }
}
