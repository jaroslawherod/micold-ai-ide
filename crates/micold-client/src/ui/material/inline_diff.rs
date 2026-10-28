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
//! `large_message` counts and **Show in full** take the hunks' place (FR-019). With
//! [`InlineDiff::copy_path`] the path has the code block's copy control beside it (feature 1550,
//! FR-017).
//!
//! Everything shown is owned, so the element is `'static` and can sit inside iced `lazy`.

use std::ops::Range;
use std::rc::Rc;

use iced::widget::{column, container, row, text::Wrapping};
use iced::{Alignment, Element, Length};
use micold_core::review::diff::{LineKind, SchemeSpans, SideSpans};
use micold_core::structured::edit_diff::{EditDiff, EditHunk};
use micold_core::tokens::{spacing, Roles};

use crate::features::transcript_rich::CopyState;

use super::code_block::{
    code_shape, copy_control, drawn_range, expand_tabs, in_line, selectable_line, sideways,
    MakeSelectable, SHOW_IN_FULL,
};
use super::diff_view::{large_message, line_text, line_text_selected, marker, tinted};
use super::selectable_text::{Hit, Measure};
use super::{style, Button, Text, TypeRole};

pub use crate::features::transcript_selection::shown::{hunk_title, note_text, NO_NEWLINE};

/// An Edit, MultiEdit or Write call's change. Builder form (Principle VIII):
/// `InlineDiff::new(&edit_diff, roles).spans(&scheme_spans).large(on_show).into()`.
pub struct InlineDiff<'a, M> {
    diff: &'a EditDiff,
    roles: Roles,
    spans: Option<SideSpans>,
    large: Option<M>,
    select: Option<SelectPart<M>>,
    copy_path: Option<(Option<CopyState>, M)>,
}

/// A selectable piece of an inline diff, each its own unit (feature 1550, R2): the path, hunk
/// `h`'s caption (its name and note), and hunk `h`'s lines joined with `\n`, without the gutter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffPart {
    Path,
    Caption(usize),
    Lines(usize),
}

/// What makes one part selectable: its selected bytes, the wrapper of a line drawn for it, and
/// the step that makes a [`Text`] drawn for it selectable.
struct Part<M> {
    selected: Option<Range<usize>>,
    make: MakeSelectable<M>,
    text: Box<dyn FnOnce(Text<'static, M>) -> Text<'static, M>>,
}

type SelectPart<M> = Rc<dyn Fn(DiffPart) -> Part<M>>;

impl<'a, M: Clone + 'static> InlineDiff<'a, M> {
    pub fn new(diff: &'a EditDiff, roles: Roles) -> Self {
        Self {
            diff,
            roles,
            spans: None,
            large: None,
            select: None,
            copy_path: None,
        }
    }

    /// The path's copy control (feature 1550, FR-017): what the last copy came to (`None` is
    /// idle), and what a press, Enter or Space sends. It sits beside the path.
    pub fn copy_path(mut self, state: Option<CopyState>, on_copy: M) -> Self {
        self.copy_path = Some((state, on_copy));
        self
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

    /// Make the path, each hunk's caption and each hunk's lines selectable (feature 1550,
    /// contracts/selectable-text.md): `part` names each piece's slot and selected bytes, and hits
    /// are sent as `on_hit(slot, hit)`. The lines answer in bytes of their text joined with `\n`,
    /// without the marker gutter or the padding. With nothing selected it draws exactly as
    /// without this call.
    pub fn selectable<K: Clone + 'static>(
        mut self,
        part: impl Fn(DiffPart) -> (K, Option<Range<usize>>) + 'static,
        on_hit: impl Fn(K, Hit) -> M + 'static,
    ) -> Self {
        let on_hit: Rc<dyn Fn(K, Hit) -> M> = Rc::new(on_hit);
        self.select = Some(Rc::new(move |p| {
            let (slot, selected) = part(p);
            let hit = Rc::clone(&on_hit);
            let (key, range) = (slot.clone(), selected.clone());
            Part {
                selected,
                make: selectable_line(slot, Rc::clone(&on_hit)),
                text: Box::new(move |t: Text<'static, M>| {
                    t.selectable(key, range, move |k, h| hit(k, h))
                }),
            }
        }));
        self
    }
}

/// `text`, made selectable as `part` when the diff is.
fn part_text<M: 'static>(
    text: Text<'static, M>,
    part: DiffPart,
    select: Option<&SelectPart<M>>,
) -> Element<'static, M> {
    match select {
        Some(select) => (select(part).text)(text).into(),
        None => text.into(),
    }
}

/// The muted caption of a hunk: its name and note.
fn caption<M: 'static>(content: impl Into<String>, r: Roles) -> Element<'static, M> {
    caption_text(content, r).into()
}

fn caption_text<M: 'static>(content: impl Into<String>, r: Roles) -> Text<'static, M> {
    Text::new(content.into(), TypeRole::Label, r)
        .muted()
        .wrapping(Wrapping::WordOrGlyph)
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
    select: Option<(&Part<M>, usize)>,
    r: Roles,
) -> Element<'static, M> {
    let line = text;
    let (mut text, spans) = expand_tabs(line, spans);
    let pad = width.saturating_sub(text.chars().count());
    text.extend(std::iter::repeat_n(' ', pad));
    let drawn = match select {
        None => line_text(text, &spans, r),
        Some((part, start)) => {
            let child = match in_line(part.selected.as_ref(), start, line.len()) {
                None => line_text(text, &spans, r),
                Some(sel) => line_text_selected(text, &spans, drawn_range(line, &sel), r),
            };
            (part.make)(child, Measure::code_line(line, width, code_shape()), start)
        }
    };
    let cells = row![marker(kind, r), drawn]
        .padding([0.0, spacing::XS])
        .align_y(Alignment::Center);
    tinted(cells.into(), kind, r, Length::Shrink)
}

/// The "No newline at end" note: the marker of the side that lacks the newline, then the note in
/// the caption's type, so it never reads as a line of the file.
fn note_row<M: 'static>(kind: LineKind, r: Roles) -> Element<'static, M> {
    let cells = row![marker(kind, r), caption(NO_NEWLINE, r)]
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
    (index, show_title): (usize, bool),
    first: u32,
    spans: &SideSpans,
    select: Option<&SelectPart<M>>,
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
        let text = caption_text(said.join(" · "), r);
        parts = parts.push(part_text(text, DiffPart::Caption(index), select));
    }
    let newline = (h.old_no_newline != h.new_no_newline).then_some(if h.old_no_newline {
        LineKind::Removed
    } else {
        LineKind::Added
    });
    let width = widest(h.lines.iter().map(|line| line.text.as_str()));
    let part = select.map(|select| select(DiffPart::Lines(index)));
    let mut start = 0;
    let mut lines: Vec<Element<'static, M>> = h
        .lines
        .iter()
        .zip(first..)
        .map(|(line, n)| {
            let at = part.as_ref().map(|part| (part, start));
            start += line.text.len() + 1;
            line_row(line.kind, &line.text, spans.line(n), width, at, r)
        })
        .collect();
    if let Some(kind) = newline {
        lines.push(note_row(kind, r));
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
        let select = view.select.as_ref();
        let path = Text::new(diff.path.clone(), TypeRole::Label, r)
            .monospace()
            .wrapping(Wrapping::WordOrGlyph);
        let path: Element<'static, M> = match view.copy_path {
            Some((state, on_copy)) => row![
                part_text(path.width(Length::Fill), DiffPart::Path, select),
                copy_control(state, on_copy, r),
            ]
            .spacing(spacing::SM)
            .align_y(Alignment::Center)
            .into(),
            None => part_text(path, DiffPart::Path, select),
        };
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
                for (index, h) in diff.hunks.iter().enumerate() {
                    parts = parts.push(hunk(h, (index, several), first, spans, select, r));
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
    use super::super::code_block::tests::{glyph_at, release};
    use super::super::code_block::COPIED;
    use super::super::test_support::{named_key, roles, Focusables, Mounted, ShownTexts};
    use super::*;
    use crate::features::transcript_rich::Ticket;
    use crate::icons::Icon;
    use crate::ui::material::SelectionArea;
    use iced::advanced::mouse;
    use iced::keyboard::key::Named;
    use iced::Point;
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

    /// "No newline at end" is a note on its side's tint, not a line of the file: it is drawn in the
    /// caption's type, never in the lines' monospace.
    #[test]
    fn the_no_newline_note_is_not_drawn_as_a_line() {
        let diff = edit_diff("Write", r#"{"file_path":"b.txt","content":"one"}"#).expect("a Write");
        let mut m = Mounted::new(InlineDiff::<String>::new(&diff, roles()));
        let painted = m.painted_text();
        let note = painted
            .iter()
            .find(|p| p.content.trim_end() == NO_NEWLINE)
            .unwrap_or_else(|| panic!("the note is painted: {painted:?}"));
        assert_ne!(note.font, iced::Font::MONOSPACE, "{note:?}");
        assert_eq!(note.content, NO_NEWLINE, "the note is not padded as a line");
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

    fn selectable(diff: &EditDiff) -> InlineDiff<'_, String> {
        InlineDiff::new(diff, roles()).selectable(
            |part| (part, None),
            |part: DiffPart, hit: Hit| format!("{part:?} {}", hit.offset),
        )
    }

    fn short_and_long() -> EditDiff {
        edit_diff(
            "Edit",
            r#"{"file_path":"src/a.rs","old_string":"x","new_string":"longer"}"#,
        )
        .expect("an Edit")
    }

    /// Feature 1550 T-4 (gate): selectable with nothing selected draws pixel-equal to the diff.
    #[test]
    fn a_selectable_diff_with_nothing_selected_draws_as_before() {
        let diff = short_and_long();
        let draw = |d: InlineDiff<'_, String>| Mounted::new(d).drawn(iced::Color::WHITE);
        assert_eq!(
            draw(selectable(&diff)),
            draw(InlineDiff::new(&diff, roles()))
        );
    }

    /// Feature 1550: the path, the hunk captions and each hunk's lines are units, the lines
    /// without the marker gutter or the padding.
    #[test]
    fn the_path_captions_and_line_text_are_reported_without_the_gutter() {
        let input = r#"{"file_path":"src/a.rs","edits":[
            {"old_string":"x","new_string":"longer","replace_all":true},
            {"old_string":"y","new_string":"y"}]}"#;
        let diff = edit_diff("MultiEdit", input).expect("a MultiEdit");
        let mut m = Mounted::new(selectable(&diff));
        let shown = m
            .drive(ShownTexts::<DiffPart>(Vec::new()))
            .expect("reported");
        let expected = [
            (DiffPart::Path, 0, "src/a.rs".to_owned()),
            (
                DiffPart::Caption(0),
                0,
                "Edit 1 of 2 \u{b7} Replaces every occurrence".to_owned(),
            ),
            (DiffPart::Lines(0), 0, "x".to_owned()),
            (DiffPart::Lines(0), 2, "longer".to_owned()),
            (
                DiffPart::Caption(1),
                0,
                "Edit 2 of 2 \u{b7} No change".to_owned(),
            ),
        ];
        assert_eq!(shown[..expected.len()], expected[..], "{shown:?}");
    }

    /// Feature 1550 T-1: a press on a line answers in bytes of the hunk's joined lines.
    #[test]
    fn a_press_on_a_line_answers_in_bytes_of_the_joined_lines() {
        let diff = short_and_long();
        let mut m = Mounted::new(selectable(&diff));
        let painted = m.painted_text();
        let line = painted
            .iter()
            .find(|p| p.content.starts_with("longer"))
            .expect("the added line is painted");
        let advance = line.bounds.width / line.content.chars().count() as f32;
        let at = Point::new(
            line.bounds.x + 2.3 * advance,
            line.bounds.y + line.bounds.height / 2.0,
        );
        assert_eq!(m.press(at), vec!["Lines(0) 4".to_owned()]);
    }

    /// Two lines, `banana split` and a short `fig`, and where each is painted.
    fn banana_fig() -> EditDiff {
        edit_diff(
            "Write",
            r#"{"file_path":"f.txt","content":"banana split\nfig\n"}"#,
        )
        .expect("a Write")
    }

    /// The selectable diff in a selection area, which publishes `press` on a press.
    fn in_area(diff: &EditDiff) -> Mounted<'_> {
        Mounted::new(SelectionArea::new(selectable(diff)).on_press(|_| "press".to_owned()))
    }

    /// The painted bounds of the line starting with `start`.
    fn line_at(m: &mut Mounted<'_>, start: &str) -> iced::Rectangle {
        m.painted_text()
            .into_iter()
            .find(|p| p.content.starts_with(start))
            .expect("the line is painted")
            .bounds
    }

    /// #1550 T054 (FR-001): the runtime hands every event of a batch the cursor where the batch
    /// ended, so a fast drag's press, its move and the move after it all come with the pointer
    /// on `fig`; the selection still starts where the press was, on `banana`.
    #[test]
    fn a_fast_drag_on_diff_lines_anchors_at_the_press() {
        let diff = banana_fig();
        let mut m = in_area(&diff);
        let (banana, fig) = (line_at(&mut m, "banana"), line_at(&mut m, "fig"));
        let advance = banana.width / "banana split".len() as f32;
        let mid = |r: iced::Rectangle| r.y + r.height / 2.0;
        let press_at = Point::new(banana.x + 2.3 * advance, mid(banana));
        let end_at = Point::new(fig.x + 1.3 * advance, mid(fig));
        let last = mouse::Cursor::Available(end_at);
        let moved = |p| iced::Event::Mouse(mouse::Event::CursorMoved { position: p });
        let _ = m.send(moved(press_at), last);
        let pressed = m.send(
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            last,
        );
        assert_eq!(pressed, vec!["press".to_owned(), "Lines(0) 2".to_owned()]);
        assert_eq!(m.send(moved(end_at), last), vec!["Lines(0) 14".to_owned()]);
    }

    /// #1550 T054 (FR-001): a drag along a short line that ends right of the line's row reaches
    /// that line's end.
    #[test]
    fn a_drag_right_of_a_short_diff_line_selects_to_its_end() {
        let diff = banana_fig();
        let mut m = in_area(&diff);
        let fig = line_at(&mut m, "fig");
        let y = fig.y + fig.height / 2.0;
        let start = Point::new(fig.x + 1.0, y);
        let _ = m.hover(start);
        assert_eq!(
            m.press(start),
            vec!["press".to_owned(), "Lines(0) 13".to_owned()]
        );
        let past = Point::new(fig.x + fig.width + 40.0, y);
        assert_eq!(m.hover(past), vec!["Lines(0) 16".to_owned()]);
    }

    fn with_copy(diff: &EditDiff, state: Option<CopyState>) -> InlineDiff<'_, String> {
        InlineDiff::new(diff, roles()).copy_path(state, "copy path".to_string())
    }

    /// Feature 1550 (C-3, C-4, FR-017, FR-018): the path's copy control is the diff's one
    /// keyboard focus stop, and a press, Enter and Space each send its message.
    #[test]
    fn the_path_copy_control_copies_by_pointer_enter_and_space() {
        let diff = short_and_long();
        let mut m = Mounted::new(with_copy(&diff, None));
        assert_eq!(m.drive(Focusables(0)), Some(1), "one focus stop");
        let at = glyph_at(&mut m, Icon::Copy);
        let mut out = m.press(at);
        out.extend(release(&mut m, at));
        assert_eq!(out, vec!["copy path".to_string()], "the pointer copies");
        for key in [Named::Enter, Named::Space] {
            let mut m = Mounted::new(with_copy(&diff, None));
            m.focus_next();
            let out = m.send(named_key(key), mouse::Cursor::Unavailable);
            assert_eq!(out, vec!["copy path".to_string()], "{key:?} copies");
        }
    }

    /// Feature 1550 (C-3, FR-018): the control sits on the path's line, after it, and confirms as
    /// the code block's does; without one the diff has no focus stop.
    #[test]
    fn the_path_copy_control_sits_beside_the_path_and_confirms() {
        let diff = short_and_long();
        let copied = Some(CopyState::Copied(Ticket::posed(1)));
        let mut m = Mounted::new(with_copy(&diff, copied));
        let painted = m.painted_text();
        let find = |content: &str| {
            painted
                .iter()
                .find(|p| p.content == content)
                .unwrap_or_else(|| panic!("`{content}` is painted: {painted:?}"))
                .bounds
        };
        let (path, words) = (find("src/a.rs"), find(COPIED));
        let middle = words.y + words.height / 2.0;
        assert!(
            middle > path.y && middle < path.y + path.height,
            "on the path's line: {path:?} {words:?}"
        );
        assert!(words.x > path.x, "after the path: {path:?} {words:?}");
        let mut plain = Mounted::new(InlineDiff::<String>::new(&diff, roles()));
        assert_eq!(plain.drive(Focusables(0)), Some(0), "no control, no stop");
    }
}
