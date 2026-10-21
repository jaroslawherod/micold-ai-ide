//! `InlineDiff` — the change an Edit, MultiEdit or Write tool call asks for, as a unified diff
//! (feature 940, research R6, R7, R9; FR-013–FR-016, FR-019).
//!
//! The file path heads it, then each hunk: "Edit n of m" when the call holds several edits, the
//! hunk's note ("Replaces every occurrence", "No change", "Writes an empty file", "New content"),
//! and its lines. A line is the diff view's own row — the `+`/`−` marker cell and the line tint
//! from [`DiffView`](super::DiffView)'s shared helpers, so the two cannot drift — with no number
//! gutter: the tool call does not say where in the file its text is (FR-016). The lines of a hunk
//! scroll sideways inside a horizontal-only viewport, never wrapped. When only the final newline
//! changed, a marked "No newline at end" row says which side lacks it.
//!
//! Over the size limits the caller says so with [`InlineDiff::large`]: the diff view's
//! `large_message` counts and **Show in full** take the hunks' place (FR-019).
//!
//! Everything shown is owned, so the element is `'static` and can sit inside iced `lazy`.

use iced::widget::{column, container, row, text::Wrapping};
use iced::{Alignment, Element, Length};
use micold_core::review::diff::{LineKind, SchemeSpans, SideSpans};
use micold_core::structured::edit_diff::{EditDiff, EditHunk, HunkNote};
use micold_core::tokens::{spacing, Roles};

use super::code_block::{expand_tabs, sideways, SHOW_IN_FULL};
use super::diff_view::{large_message, line_text, marker, tinted};
use super::{style, Button, Text, TypeRole};

/// What a hunk's note says.
pub fn note_text(note: HunkNote) -> &'static str {
    match note {
        HunkNote::ReplaceAll => "Replaces every occurrence",
        HunkNote::NoChange => "No change",
        HunkNote::EmptyWrite => "Writes an empty file",
        HunkNote::NewContent => "New content",
    }
}

/// What the row under a hunk whose final newline changed says.
pub const NO_NEWLINE: &str = "No newline at end";

/// The name of hunk `ordinal` of `of`.
pub fn hunk_title(ordinal: u32, of: u32) -> String {
    format!("Edit {ordinal} of {of}")
}

/// An Edit, MultiEdit or Write call's change. Builder form (Principle VIII):
/// `InlineDiff::new(&edit_diff, roles).spans(&scheme_spans).large(on_show).into()`.
pub struct InlineDiff<'a, M> {
    diff: &'a EditDiff,
    roles: Roles,
    spans: Option<SideSpans>,
    large: Option<M>,
}

impl<'a, M: Clone + 'static> InlineDiff<'a, M> {
    pub fn new(diff: &'a EditDiff, roles: Roles) -> Self {
        Self {
            diff,
            roles,
            spans: None,
            large: None,
        }
    }

    /// The lines' syntax colours in the current scheme: every hunk's lines in order, numbered from
    /// 1 on the `new` side, as the diff's highlight job answers them (research R4).
    pub fn spans(mut self, spans: &SchemeSpans) -> Self {
        self.spans = Some(spans.new.clone());
        self
    }

    /// Hold the hunks back: the diff is over the size limits, and "Show in full" sends `on_show`.
    pub fn large(mut self, on_show: M) -> Self {
        self.large = Some(on_show);
        self
    }
}

/// The muted caption of a hunk: its name and note.
fn caption<M: 'static>(content: impl Into<String>, r: Roles) -> Element<'static, M> {
    Text::new(content.into(), TypeRole::Label, r)
        .muted()
        .wrapping(Wrapping::WordOrGlyph)
        .into()
}

/// One diff line: its marker cell and text on its tint, the text padded with spaces to `width`
/// characters. The lines are monospace, so padding every line of a hunk to the widest one's length
/// runs each tint to the same edge — inside the sideways viewport no row can fill a width, since
/// the viewport's content has none to give.
fn line_row<M: 'static>(
    kind: LineKind,
    text: &str,
    spans: &[(std::ops::Range<usize>, micold_core::tokens::Rgb)],
    width: usize,
    r: Roles,
) -> Element<'static, M> {
    let (mut text, spans) = expand_tabs(text, spans);
    let pad = width.saturating_sub(text.chars().count());
    text.extend(std::iter::repeat_n(' ', pad));
    let cells = row![marker(kind, r), line_text(text, &spans, r)]
        .padding([0.0, spacing::XS])
        .align_y(Alignment::Center);
    tinted(cells.into(), kind, r, Length::Shrink)
}

/// The width, in characters, of the widest of `lines` once its tabs are drawn as spaces.
fn widest<'t>(lines: impl IntoIterator<Item = &'t str>) -> usize {
    lines
        .into_iter()
        .map(|line| expand_tabs(line, &[]).0.chars().count())
        .max()
        .unwrap_or(0)
}

/// A hunk: its caption rows, then its lines in a sideways viewport. `first` is the number of the
/// hunk's first line among all the diff's lines (from 1), for the spans.
fn hunk<M: Clone + 'static>(
    h: &EditHunk,
    show_title: bool,
    first: u32,
    spans: &SideSpans,
    r: Roles,
) -> Element<'static, M> {
    let mut parts = column![].spacing(spacing::XS);
    let mut said: Vec<String> = Vec::new();
    if show_title {
        said.push(hunk_title(h.ordinal, h.of));
    }
    if let Some(note) = h.note {
        said.push(note_text(note).to_owned());
    }
    if !said.is_empty() {
        parts = parts.push(caption(said.join(" · "), r));
    }
    let newline = (h.old_no_newline != h.new_no_newline).then_some(if h.old_no_newline {
        LineKind::Removed
    } else {
        LineKind::Added
    });
    let width = widest(
        h.lines
            .iter()
            .map(|line| line.text.as_str())
            .chain(newline.map(|_| NO_NEWLINE)),
    );
    let mut lines: Vec<Element<'static, M>> = h
        .lines
        .iter()
        .zip(first..)
        .map(|(line, n)| line_row(line.kind, &line.text, spans.line(n), width, r))
        .collect();
    if let Some(kind) = newline {
        lines.push(line_row(kind, NO_NEWLINE, &[], width, r));
    }
    if !lines.is_empty() {
        parts = parts.push(sideways(column(lines), r));
    }
    parts.into()
}

impl<'a, 'b, M: Clone + 'static> From<InlineDiff<'a, M>> for Element<'b, M> {
    fn from(view: InlineDiff<'a, M>) -> Self {
        let r = view.roles;
        let diff = view.diff;
        let path = Text::new(diff.path.clone(), TypeRole::Label, r)
            .monospace()
            .wrapping(Wrapping::WordOrGlyph);
        let mut parts = column![path].spacing(spacing::SM);
        match view.large {
            Some(on_show) => {
                parts = parts.push(
                    column![
                        Text::new(large_message(diff.added, diff.removed), TypeRole::Body, r)
                            .muted(),
                        Button::filled(SHOW_IN_FULL, r).on_press(on_show),
                    ]
                    .spacing(spacing::SM),
                );
            }
            None => {
                let none = SideSpans::default();
                let spans = view.spans.as_ref().unwrap_or(&none);
                let several = diff.hunks.len() > 1;
                let mut first = 1;
                for h in &diff.hunks {
                    parts = parts.push(hunk(h, several, first, spans, r));
                    first += u32::try_from(h.lines.len()).unwrap_or(u32::MAX);
                }
            }
        }
        let element: Element<'static, M> = container(parts)
            .padding([spacing::XS, spacing::SM])
            .width(Length::Fill)
            .style(style::code_surface(r))
            .into();
        element
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{roles, Mounted};
    use super::*;
    use micold_core::structured::edit_diff::edit_diff;

    fn painted(diff: &EditDiff) -> Vec<String> {
        let mut m = Mounted::new(InlineDiff::<String>::new(diff, roles()));
        // Lines are padded to the hunk's widest: compare what they say.
        m.painted_text()
            .into_iter()
            .map(|p| p.content.trim_end().to_owned())
            .collect()
    }

    /// FR-013, FR-014, FR-016: the path, each hunk's name and note, the marked lines, and no line
    /// numbers.
    #[test]
    fn a_multi_edit_shows_its_path_hunks_notes_and_signs() {
        let input = r#"{"file_path":"src/a.rs","edits":[
            {"old_string":"let a = 1;","new_string":"let a = 2;","replace_all":true},
            {"old_string":"x","new_string":"x"}]}"#;
        let diff = edit_diff("MultiEdit", input).expect("a MultiEdit");
        let painted = painted(&diff);
        for content in [
            "src/a.rs",
            "Edit 1 of 2 · Replaces every occurrence",
            "Edit 2 of 2 · No change",
            "+",
            "\u{2212}",
            "let a = 1;",
            "let a = 2;",
        ] {
            assert!(
                painted.iter().any(|p| p == content),
                "`{content}` is painted: {painted:?}"
            );
        }
        assert!(
            painted
                .iter()
                .all(|p| !p.chars().all(|c| c.is_ascii_digit())),
            "no number gutter: {painted:?}"
        );
    }

    /// A Write is its new content; a missing final newline is marked on its side.
    #[test]
    fn a_write_is_new_content_and_says_when_it_ends_without_a_newline() {
        let diff =
            edit_diff("Write", r#"{"file_path":"b.txt","content":"one\ntwo"}"#).expect("a Write");
        let painted = painted(&diff);
        for content in ["New content", "one", "two", NO_NEWLINE] {
            assert!(
                painted.iter().any(|p| p == content),
                "`{content}` is painted: {painted:?}"
            );
        }
    }

    /// FR-019: over the limits, the counts and "Show in full" take the hunks' place.
    #[test]
    fn a_large_diff_shows_its_counts_and_show_in_full() {
        let diff =
            edit_diff("Write", r#"{"file_path":"b.txt","content":"one\n"}"#).expect("a Write");
        let mut m = Mounted::new(InlineDiff::new(&diff, roles()).large("show".to_string()));
        let painted: Vec<String> = m.painted_text().into_iter().map(|p| p.content).collect();
        assert!(painted.contains(&large_message(1, 0)), "{painted:?}");
        assert!(painted.contains(&SHOW_IN_FULL.to_string()), "{painted:?}");
        assert!(!painted.contains(&"one".to_string()), "{painted:?}");
    }
}
