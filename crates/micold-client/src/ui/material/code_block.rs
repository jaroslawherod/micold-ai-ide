//! `CodeBlock` — a block of code on its own surface, with a copy control (feature 940, research R4,
//! R5, R7, R9).
//!
//! The lines are monospace and never wrapped: a line wider than the block scrolls sideways inside
//! the block's own horizontal-only [`Scrollable`], so the block never widens what holds it (FR-010),
//! and a vertical wheel over it still scrolls the transcript (iced moves a horizontal-only
//! viewport by the wheel's horizontal part alone, research R12). Syntax colours, when the caller
//! has them, are painted per line as the diff view paints them; the bytes they leave are
//! `on_surface`.
//!
//! The header row holds the fence's tag, when there is one, and the copy control: an
//! [`IconButton`](super::IconButton) the keyboard can hold ([`TakesTheKeyboard`]), so a pointer
//! press, Enter and Space all send the caller's copy message (FR-011). What it shows follows the
//! copy state the caller passes: `Icon::Copy` while idle or pending, `Icon::CardDone` and "Copied"
//! once the read-back held the code, the error role and "Copy failed" when it did not (R5, FR-012).
//!
//! A block over the size limits shows "Large code block: N lines" and a **Show in full** button in
//! place of its lines (FR-019); the caller decides that, as it decides the copy state.
//!
//! Everything shown is owned, so the element is `'static` and can sit inside iced `lazy`.

use std::ops::Range;

use iced::keyboard::key::Named;
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::review::diff::SideSpans;
use micold_core::tokens::{shape, spacing, Rgb, Roles};

use super::diff_view::line_text;
use super::keyboard_focus::{Indicator, TakesTheKeyboard};
use super::{style, Button, Ellipsized, IconButton, ScrollDirection, Scrollable, Text, TypeRole};
use crate::features::transcript_rich::CopyState;
use crate::icons::Icon;

/// What the copy control says once the read-back held the code (FR-012).
pub const COPIED: &str = "Copied";

/// What the copy control says when the clipboard did not take the code (FR-012).
pub const COPY_FAILED: &str = "Copy failed";

/// The button that lays out a part held back by the size limits (FR-019).
pub const SHOW_IN_FULL: &str = "Show in full";

/// How many columns apart the tab stops are when a tab is drawn as spaces.
const TAB_STOP: usize = 4;

/// What a code block over the size limits says in place of its lines (research R7).
pub fn large_message(lines: u32) -> String {
    format!("Large code block: {lines} lines")
}

/// A code block. Builder form (Principle VIII):
/// `CodeBlock::new(code, roles).language(tag).spans(&scheme_spans).copy(state, on_copy)
/// .large(lines, on_show).into()`.
pub struct CodeBlock<M> {
    code: String,
    roles: Roles,
    tag: Option<String>,
    spans: Option<SideSpans>,
    copy: Option<(Option<CopyState>, M)>,
    large: Option<(u32, M)>,
}

impl<M: Clone + 'static> CodeBlock<M> {
    /// `code` exactly as it is copied: no fence, no tag.
    pub fn new(code: impl Into<String>, roles: Roles) -> Self {
        Self {
            code: code.into(),
            roles,
            tag: None,
            spans: None,
            copy: None,
            large: None,
        }
    }

    /// The fence's tag, as written, shown in the header row (FR-009). An empty tag is none.
    pub fn language(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into()).filter(|tag| !tag.is_empty());
        self
    }

    /// The code's syntax colours in the current scheme: line `n` of the code is line `n` of the
    /// `new` side, as a highlight job answers them (research R4).
    pub fn spans(mut self, spans: &micold_core::review::diff::SchemeSpans) -> Self {
        self.spans = Some(spans.new.clone());
        self
    }

    /// The copy control: what the last copy came to (`None` is idle), and what a press sends.
    pub fn copy(mut self, state: Option<CopyState>, on_copy: M) -> Self {
        self.copy = Some((state, on_copy));
        self
    }

    /// Hold the lines back: the block is over the size limits with `lines` lines, and "Show in
    /// full" sends `on_show` (FR-019).
    pub fn large(mut self, lines: u32, on_show: M) -> Self {
        self.large = Some((lines, on_show));
        self
    }
}

/// The copy control's look for `state`: its glyph, the glyph's tint, and the words beside it.
pub(super) fn copy_look(state: Option<CopyState>, r: Roles) -> (Icon, Rgb, Option<&'static str>) {
    match state {
        Some(CopyState::Copied(_)) => (Icon::CardDone, r.on_surface_variant, Some(COPIED)),
        Some(CopyState::Failed(_)) => (Icon::Copy, r.error, Some(COPY_FAILED)),
        Some(CopyState::Pending(_)) | None => (Icon::Copy, r.on_surface_variant, None),
    }
}

/// The copy control: the icon button in a keyboard focus, then the state's words.
fn copy_control<M: Clone + 'static>(
    state: Option<CopyState>,
    on_copy: M,
    r: Roles,
) -> Element<'static, M> {
    let (glyph, tint, words) = copy_look(state, r);
    let button = IconButton::new(glyph, r)
        .compact()
        .size(TypeRole::Body)
        .tint(tint)
        .on_press(on_copy.clone());
    let focus = TakesTheKeyboard::new(button, true)
        .key(Named::Enter, on_copy.clone())
        .key(Named::Space, on_copy)
        .indicator(Indicator {
            outline: r.secondary,
            layer: tint,
            radius: shape::FULL,
        });
    let mut parts = row![].spacing(spacing::XS).align_y(Alignment::Center);
    if let Some(words) = words {
        parts = parts.push(
            Text::new(words, TypeRole::Label, r)
                .tint(tint)
                .wrapping(iced::widget::text::Wrapping::None),
        );
    }
    parts.push(focus).into()
}

/// `line` with each tab drawn as spaces up to the next tab stop (the text stack draws a tab as a
/// missing glyph), and `spans` moved with the bytes they cover. A line without a tab is unchanged.
pub(super) fn expand_tabs(
    line: &str,
    spans: &[(Range<usize>, Rgb)],
) -> (String, Vec<(Range<usize>, Rgb)>) {
    if !line.contains('\t') {
        return (line.to_owned(), spans.to_vec());
    }
    // `at[i]` is where byte `i` of `line` lands in the expanded text.
    let mut at = vec![0; line.len() + 1];
    let mut out = String::with_capacity(line.len() + TAB_STOP);
    let mut column = 0;
    for (i, c) in line.char_indices() {
        at[i] = out.len();
        if c == '\t' {
            let pad = TAB_STOP - column % TAB_STOP;
            out.extend(std::iter::repeat_n(' ', pad));
            column += pad;
        } else {
            out.push(c);
            column += 1;
        }
        at[i + 1..i + c.len_utf8()].fill(out.len());
    }
    at[line.len()] = out.len();
    let moved = spans
        .iter()
        .map(|(range, colour)| {
            let start = at[range.start.min(line.len())];
            let end = at[range.end.min(line.len())];
            (start..end, *colour)
        })
        .collect();
    (out, moved)
}

/// One code line in its colours, tabs drawn as spaces; monospace, never wrapped.
pub(super) fn code_line<M: 'static>(
    line: &str,
    spans: &[(Range<usize>, Rgb)],
    r: Roles,
) -> Element<'static, M> {
    let (text, spans) = expand_tabs(line, spans);
    line_text(text, &spans, r)
}

/// `content` in a horizontal-only viewport: wider than the block, it scrolls sideways inside it.
pub(super) fn sideways<M: Clone + 'static>(
    content: impl Into<Element<'static, M>>,
    r: Roles,
) -> Element<'static, M> {
    Scrollable::new(content, r)
        .direction(ScrollDirection::Horizontal)
        .width(Length::Fill)
        .into()
}

impl<'b, M: Clone + 'static> From<CodeBlock<M>> for Element<'b, M> {
    fn from(b: CodeBlock<M>) -> Self {
        let r = b.roles;
        let tag: Element<'static, M> = match b.tag {
            Some(tag) => container(Ellipsized::at_role(
                tag,
                TypeRole::Label,
                r.on_surface_variant,
            ))
            .width(Length::Fill)
            .into(),
            None => Space::new().width(Length::Fill).into(),
        };
        let mut header = row![tag].spacing(spacing::SM).align_y(Alignment::Center);
        if let Some((state, on_copy)) = b.copy {
            header = header.push(copy_control(state, on_copy, r));
        }
        let body: Element<'static, M> = match b.large {
            Some((lines, on_show)) => column![
                Text::new(large_message(lines), TypeRole::Body, r).muted(),
                Button::filled(SHOW_IN_FULL, r).on_press(on_show),
            ]
            .spacing(spacing::SM)
            .into(),
            None => {
                let none = SideSpans::default();
                let spans = b.spans.as_ref().unwrap_or(&none);
                let lines = b
                    .code
                    .lines()
                    .enumerate()
                    .map(|(i, line)| code_line(line, spans.line(i as u32 + 1), r));
                sideways(column(lines).width(Length::Shrink), r)
            }
        };
        let block: Element<'static, M> = container(column![header, body].spacing(spacing::XS))
            .padding([spacing::XS, spacing::SM])
            .width(Length::Fill)
            .style(style::code_surface(r))
            .into();
        block
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, named_key, roles, Focusables, Mounted};
    use super::*;
    use crate::features::transcript_rich::Ticket;
    use iced::advanced::mouse;
    use iced::{Event, Point};

    const CODE: &str = "fn main() {\n\tprintln!(\"hi\");\n}";

    fn block(state: Option<CopyState>, r: Roles) -> CodeBlock<String> {
        CodeBlock::new(CODE, r)
            .language("rust")
            .copy(state, "copy".to_string())
    }

    /// Where `glyph` was painted: the copy control's icon.
    fn glyph_at(m: &mut Mounted<'_>, glyph: Icon) -> Point {
        let painted = m.painted_text();
        let p = painted
            .iter()
            .find(|p| p.content == glyph.glyph().to_string())
            .unwrap_or_else(|| panic!("`{glyph:?}` is painted: {painted:?}"));
        Point::new(
            p.bounds.x + p.bounds.width / 2.0,
            p.bounds.y + p.bounds.height / 2.0,
        )
    }

    fn release(m: &mut Mounted<'_>, at: Point) -> Vec<String> {
        m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// FR-011: the copy control is the block's one keyboard focus stop.
    #[test]
    fn the_copy_control_is_one_focus_stop() {
        let mut m = Mounted::new(block(None, roles()));
        assert_eq!(m.drive(Focusables(0)), Some(1));
    }

    /// FR-011: a pointer press on the control, and Enter once it holds the keyboard, send the copy.
    #[test]
    fn a_press_and_enter_both_copy() {
        let mut m = Mounted::new(block(None, roles()));
        let at = glyph_at(&mut m, Icon::Copy);
        let mut out = m.press(at);
        out.extend(release(&mut m, at));
        assert_eq!(out, vec!["copy".to_string()], "the pointer copies");
        let mut m = Mounted::new(block(None, roles()));
        m.focus_next();
        let out = m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
        assert_eq!(out, vec!["copy".to_string()], "Enter copies");
    }

    /// FR-012: idle and pending show the copy glyph and no words; copied shows `CardDone` and
    /// "Copied"; failed shows "Copy failed" in the error role — both schemes.
    #[test]
    fn the_control_follows_the_copy_state() {
        for r in [roles(), dark_roles()] {
            let words = |state| {
                let mut m = Mounted::new(block(state, r));
                m.painted_text()
                    .into_iter()
                    .filter(|p| p.content == COPIED || p.content == COPY_FAILED)
                    .map(|p| (p.content, p.color))
                    .collect::<Vec<_>>()
            };
            assert!(words(None).is_empty(), "idle says nothing");
            assert!(words(Some(CopyState::Pending(Ticket::posed(1)))).is_empty());
            let copied = Some(CopyState::Copied(Ticket::posed(1)));
            assert_eq!(words(copied).len(), 1, "copied says so");
            assert_eq!(words(copied)[0].0, COPIED);
            glyph_at(&mut Mounted::new(block(copied, r)), Icon::CardDone);
            let failed = words(Some(CopyState::Failed(Ticket::posed(1))));
            assert_eq!(
                failed,
                vec![(COPY_FAILED.to_string(), style::color(r.error))],
                "failed says so in the error role"
            );
        }
    }

    /// FR-019: a block over the limits shows its line count and "Show in full", whose press sends
    /// `on_show`; its lines are not laid out.
    #[test]
    fn a_large_block_shows_its_count_and_show_in_full() {
        let large = || block(None, roles()).large(6_000, "show".to_string());
        let mut m = Mounted::new(large());
        let painted = m.painted_text();
        for content in [large_message(6_000).as_str(), SHOW_IN_FULL] {
            assert!(
                painted.iter().any(|p| p.content == content),
                "`{content}` is painted: {painted:?}"
            );
        }
        assert!(
            painted.iter().all(|p| !p.content.contains("main")),
            "the code is held back: {painted:?}"
        );
        let button = painted
            .iter()
            .find(|p| p.content == SHOW_IN_FULL)
            .expect("the button is painted")
            .bounds;
        let at = Point::new(button.x + 1.0, button.y + button.height / 2.0);
        let mut out = m.press(at);
        out.extend(release(&mut m, at));
        assert_eq!(out, vec!["show".to_string()]);
    }

    /// FR-010: the lines are monospace and unwrapped, the tag is shown, and a tab is drawn as
    /// spaces up to the next stop.
    #[test]
    fn lines_are_monospace_with_the_tag_and_tabs_as_spaces() {
        let mut m = Mounted::new(block(None, roles()));
        let painted = m.painted_text();
        assert!(painted.iter().any(|p| p.content == "rust"), "{painted:?}");
        let line = painted
            .iter()
            .find(|p| p.content.contains("println!"))
            .unwrap_or_else(|| panic!("the second line is painted: {painted:?}"));
        assert_eq!(line.content, "    println!(\"hi\");");
        assert_eq!(line.font, iced::Font::MONOSPACE);
    }

    /// Spans move with the bytes a tab expands.
    #[test]
    fn expanding_a_tab_moves_the_spans() {
        let red = Rgb::hex(0xff0000);
        let (text, spans) = expand_tabs("a\tb", &[(2..3, red)]);
        assert_eq!(text, "a   b");
        assert_eq!(spans, vec![(4..5, red)]);
    }
}
