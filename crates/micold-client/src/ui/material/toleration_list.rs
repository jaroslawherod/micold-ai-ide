//! `TolerationList` — a multi-line editor for tolerations (feature 686, FR-005).
//!
//! One row per line (`key[=value]:Effect` or `key:Exists:Effect`), blank lines ignored.
//! [`parse`] and [`format`] are the pure halves; the builder wraps a [`TextArea`].

use iced::widget::text_editor::{Action, Content};
use iced::Element;
use micold_core::kube::settings::Toleration;
use micold_core::tokens::Roles;

pub use super::key_value_list::RowError;
use super::TextArea;

/// Parse one toleration per non-blank line; a bad row names its 1-based line.
pub fn parse(text: &str) -> Result<Vec<Toleration>, RowError> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(index, line)| {
            Toleration::parse(line.trim()).map_err(|_| RowError { line: index + 1 })
        })
        .collect()
}

/// Tolerations in their row form, joined by newlines.
pub fn format(rows: &[Toleration]) -> String {
    rows.iter()
        .map(Toleration::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// A multi-line toleration editor over `content`.
pub struct TolerationList<'a, M>(TextArea<'a, M>);

impl<'a, M: Clone + 'a> TolerationList<'a, M> {
    /// An editor over `content`, themed by `roles`; read-only until [`Self::on_action`].
    pub fn new(content: &'a Content, roles: Roles) -> Self {
        Self(TextArea::new(content, roles).placeholder("key=value:NoSchedule"))
    }

    /// The message an edit sends; the caller performs the action on its `Content`.
    pub fn on_action(self, f: impl Fn(Action) -> M + 'a) -> Self {
        Self(self.0.on_action(f))
    }
}

impl<'a, M: Clone + 'a> From<TolerationList<'a, M>> for Element<'a, M> {
    fn from(list: TolerationList<'a, M>) -> Self {
        list.0.into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_accepted_form_round_trips() {
        let text = "gpu=true:NoSchedule\ngpu:PreferNoSchedule\ngpu:Exists:NoExecute";
        let rows = parse(text).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(format(&rows), text);
        assert_eq!(parse(&format(&rows)), Ok(rows));
    }

    #[test]
    fn blank_lines_are_ignored() {
        assert_eq!(parse("\n gpu:NoSchedule \n\n").unwrap().len(), 1);
        assert_eq!(parse(""), Ok(vec![]));
    }

    #[test]
    fn a_bad_row_names_its_line() {
        let err = parse("gpu:NoSchedule\n\nnope").unwrap_err();
        assert_eq!(err, RowError { line: 3 });
        assert_eq!(err.to_string(), "Row 3 is not valid.");
    }
}
