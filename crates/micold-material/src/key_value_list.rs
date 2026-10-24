//! `KeyValueList` — a multi-line editor for `key=value` rows (feature 686, FR-005).
//!
//! One row per line, blank lines ignored. [`parse`] and [`format`] are the pure halves; the builder
//! wraps a [`TextArea`]: `KeyValueList::new(&content, roles).on_action(Msg::Edit).into()`.

use iced::widget::text_editor::{Action, Content};
use iced::Element;
use micold_core::tokens::Roles;

use super::TextArea;

/// A row that is not valid, by 1-based line number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowError {
    /// The 1-based line of the offending row.
    pub line: usize,
}

impl std::fmt::Display for RowError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Row {} is not valid.", self.line)
    }
}

impl std::error::Error for RowError {}

/// Parse `key=value` rows: the first `=` splits, `key` alone has an empty value, whitespace is
/// trimmed, blank lines are skipped, an empty key is an error.
pub fn parse(text: &str) -> Result<Vec<(String, String)>, RowError> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (key, value) = line.split_once('=').unwrap_or((line, ""));
        let key = key.trim();
        if key.is_empty() {
            return Err(RowError { line: index + 1 });
        }
        rows.push((key.to_string(), value.trim().to_string()));
    }
    Ok(rows)
}

/// `key=value` rows joined by newlines.
pub fn format(rows: &[(String, String)]) -> String {
    rows.iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A multi-line `key=value` editor over `content`.
pub struct KeyValueList<'a, M>(TextArea<'a, M>);

impl<'a, M: Clone + 'a> KeyValueList<'a, M> {
    /// An editor over `content`, themed by `roles`; read-only until [`Self::on_action`].
    pub fn new(content: &'a Content, roles: Roles) -> Self {
        Self(TextArea::new(content, roles).placeholder("key=value"))
    }

    /// The message an edit sends; the caller performs the action on its `Content`.
    pub fn on_action(self, f: impl Fn(Action) -> M + 'a) -> Self {
        Self(self.0.on_action(f))
    }
}

impl<'a, M: Clone + 'a> From<KeyValueList<'a, M>> for Element<'a, M> {
    fn from(list: KeyValueList<'a, M>) -> Self {
        list.0.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(k: &str, v: &str) -> (String, String) {
        (k.to_string(), v.to_string())
    }

    #[test]
    fn rows_round_trip() {
        let rows = vec![row("a", "b"), row("empty", ""), row("url", "x=y")];
        assert_eq!(parse(&format(&rows)), Ok(rows));
    }

    #[test]
    fn parse_trims_skips_blanks_and_splits_on_the_first_equals() {
        assert_eq!(
            parse("  a = b \n\n  \nk\nu=x=y\n"),
            Ok(vec![row("a", "b"), row("k", ""), row("u", "x=y")]),
        );
    }

    #[test]
    fn empty_key_names_its_line() {
        let err = parse("a=b\n\n=v").unwrap_err();
        assert_eq!(err, RowError { line: 3 });
        assert_eq!(err.to_string(), "Row 3 is not valid.");
    }

    #[test]
    fn empty_text_is_no_rows() {
        assert_eq!(parse(""), Ok(vec![]));
        assert_eq!(format(&[]), "");
    }
}
