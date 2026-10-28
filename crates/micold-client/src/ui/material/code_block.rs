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
use std::rc::Rc;

use iced::keyboard::key::Named;
use iced::widget::{column, container, row, Space};
use iced::{Alignment, Element, Length};
use micold_core::review::diff::SideSpans;
use micold_core::tokens::{shape, spacing, Rgb, Roles};

use super::diff_view::{line_text, line_text_selected, CODE_SIZE};
use super::keyboard_focus::{Indicator, TakesTheKeyboard};
use super::selectable_text::{self, Hit, Measure, Selectable, Shape};
use super::{style, Button, Ellipsized, IconButton, ScrollDirection, Scrollable, Text, TypeRole};
use crate::features::transcript_rich::CopyState;
use crate::icons::Icon;

/// What the copy control says once the read-back held the code (FR-012).
pub const COPIED: &str = "Copied";

/// What the copy control says when the clipboard did not take the code (FR-012).
pub const COPY_FAILED: &str = "Copy failed";

/// The button that lays out a part held back by the size limits (FR-019).
pub const SHOW_IN_FULL: &str = "Show in full";

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
    select: Option<(Option<Range<usize>>, MakeSelectable<M>)>,
}

/// Wraps a drawn line in a [`Selectable`] for one slot: the line's element, what it measures and
/// where the line starts in the slot's text (feature 1550).
pub(super) type MakeSelectable<M> =
    Rc<dyn Fn(Element<'static, M>, Measure, usize) -> Element<'static, M>>;

/// The [`MakeSelectable`] of `slot`, sending hits as `on_hit(slot, hit)`.
pub(super) fn selectable_line<K: Clone + 'static, M: 'static>(
    slot: K,
    on_hit: Rc<dyn Fn(K, Hit) -> M>,
) -> MakeSelectable<M> {
    Rc::new(move |child, measure, base| {
        let on_hit = Rc::clone(&on_hit);
        Selectable::new(child, measure, slot.clone(), move |k, hit| on_hit(k, hit))
            .base(base)
            .into()
    })
}

/// How a code line is measured: monospace at the code size, never wrapped, as [`code_line`]
/// draws it.
pub(super) fn code_shape() -> Shape {
    Shape {
        size: CODE_SIZE,
        line_height: iced::widget::text::LineHeight::default(),
        font: iced::Font::MONOSPACE,
        wrapping: iced::widget::text::Wrapping::None,
        shaping: iced::advanced::text::Shaping::default(),
    }
}

/// Each line of `code` as `.lines()` splits it, with where it starts in `code`.
pub(super) fn lines_with_starts(code: &str) -> impl Iterator<Item = (usize, &str)> {
    code.split_inclusive('\n').scan(0, |at, piece| {
        let start = *at;
        *at += piece.len();
        let line = piece.strip_suffix('\n').unwrap_or(piece);
        Some((start, line.strip_suffix('\r').unwrap_or(line)))
    })
}

/// The part of `selected` (bytes of a slot's text) that falls in the line at `start`..`start +
/// len`, in bytes of the line.
pub(super) fn in_line(
    selected: Option<&Range<usize>>,
    start: usize,
    len: usize,
) -> Option<Range<usize>> {
    let s = selected?;
    let (from, to) = (s.start.max(start), s.end.min(start + len));
    (from < to).then(|| from - start..to - start)
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
            select: None,
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

    /// Make the code selectable as one unit, `slot` (feature 1550, contracts/selectable-text.md):
    /// each line answers hits in bytes of the whole code — full lines, tabs as written, even
    /// while the block is scrolled sideways — and the bytes of `selected` are drawn on the
    /// selection's fill. With `selected = None` it draws exactly as without this call.
    pub fn selectable<K: Clone + 'static>(
        mut self,
        slot: K,
        selected: Option<Range<usize>>,
        on_hit: impl Fn(K, Hit) -> M + 'static,
    ) -> Self {
        self.select = Some((selected, selectable_line(slot, Rc::new(on_hit))));
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
    let (out, map) = selectable_text::expand_tabs(line);
    let moved = spans
        .iter()
        .map(|(range, colour)| {
            (
                drawn_at(&map, range.start)..drawn_at(&map, range.end),
                *colour,
            )
        })
        .collect();
    (out, moved)
}

/// Where byte `i` of a line lands once its tabs are drawn as spaces: a byte inside a character
/// lands after it.
fn drawn_at(map: &[(usize, usize)], i: usize) -> usize {
    let k = map
        .partition_point(|&(text, _)| text < i)
        .min(map.len() - 1);
    map[k].1
}

/// `selected` (bytes of `line`) in the line as drawn, tabs as spaces.
pub(super) fn drawn_range(line: &str, selected: &Range<usize>) -> Range<usize> {
    let (_, map) = selectable_text::expand_tabs(line);
    drawn_at(&map, selected.start)..drawn_at(&map, selected.end)
}

/// One code line, selectable: drawn as [`code_line`] (selected bytes on the selection's fill),
/// measured in bytes of `line`, starting at `start` in the slot's text.
fn selectable_code_line<M: 'static>(
    line: &str,
    start: usize,
    spans: &[(Range<usize>, Rgb)],
    selected: Option<&Range<usize>>,
    make: &MakeSelectable<M>,
    r: Roles,
) -> Element<'static, M> {
    let child = match in_line(selected, start, line.len()) {
        None => code_line(line, spans, r),
        Some(part) => {
            let (text, spans) = expand_tabs(line, spans);
            line_text_selected(text, &spans, drawn_range(line, &part), r)
        }
    };
    make(child, Measure::code_line(line, 0, code_shape()), start)
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
                let lines: Vec<Element<'static, M>> = match &b.select {
                    None => b
                        .code
                        .lines()
                        .enumerate()
                        .map(|(i, line)| code_line(line, spans.line(i as u32 + 1), r))
                        .collect(),
                    Some((selected, make)) => lines_with_starts(&b.code)
                        .enumerate()
                        .map(|(i, (start, line))| {
                            let spans = spans.line(i as u32 + 1);
                            selectable_code_line(line, start, spans, selected.as_ref(), make, r)
                        })
                        .collect(),
                };
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
    use super::super::test_support::{
        dark_roles, named_key, roles, rows_with, Focusables, Mounted, ShownTexts,
    };
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

    fn selectable(selected: Option<Range<usize>>) -> CodeBlock<String> {
        block(None, roles()).selectable("code", selected, |slot: &str, hit: Hit| {
            format!("hit {slot} {}", hit.offset)
        })
    }

    /// Feature 1550 T-4 (gate): selectable with nothing selected draws pixel-equal to the block.
    #[test]
    fn a_selectable_block_with_nothing_selected_draws_as_before() {
        let draw = |b: CodeBlock<String>| Mounted::new(b).drawn(iced::Color::WHITE);
        assert_eq!(draw(selectable(None)), draw(block(None, roles())));
    }

    /// Feature 1550 T-4: selected lines are drawn on the selection fill.
    #[test]
    fn selected_lines_are_drawn_on_the_selection_fill() {
        let r = roles();
        let (none, width) = Mounted::new(selectable(None)).drawn(iced::Color::WHITE);
        assert!(rows_with(&none, width, r.primary_container).is_empty());
        let (some, width) = Mounted::new(selectable(Some(3..CODE.len()))).drawn(iced::Color::WHITE);
        assert!(!rows_with(&some, width, r.primary_container).is_empty());
    }

    /// Feature 1550 T-1: a press on the second line answers in bytes of the whole code, the tab
    /// counted as the one byte it is.
    #[test]
    fn a_press_on_a_line_answers_in_bytes_of_the_whole_code() {
        let mut m = Mounted::new(selectable(None));
        let painted = m.painted_text();
        let line = painted
            .iter()
            .find(|p| p.content.contains("println!"))
            .expect("the second line is painted");
        let advance = line.bounds.width / line.content.chars().count() as f32;
        let at = Point::new(
            line.bounds.x + 5.3 * advance,
            line.bounds.y + line.bounds.height / 2.0,
        );
        // Drawn column 5 is `r`, byte 2 of the line after its tab; the line starts at byte 12.
        assert_eq!(m.press(at), vec!["hit code 14".to_owned()]);
    }

    /// Feature 1550 (US1 scenario 6): each line reports its full text and where it starts, even
    /// when it is wider than the block and scrolls sideways.
    #[test]
    fn each_line_reports_its_full_text_while_wider_than_the_block() {
        let wide = format!("let wide = \"{}\";\nend", "x".repeat(200));
        let b = CodeBlock::new(wide.clone(), roles())
            .selectable("code", None, |_, h: Hit| h.offset.to_string());
        let mut m = Mounted::new_in(b, iced::Size::new(200.0, 300.0));
        let shown = m.drive(ShownTexts::<&str>(Vec::new())).expect("reported");
        let first = wide.lines().next().expect("a line").to_owned();
        assert_eq!(
            shown,
            vec![
                ("code", 0, first.clone()),
                ("code", first.len() + 1, "end".to_owned())
            ]
        );
    }

    #[test]
    fn lines_with_starts_matches_lines() {
        let code = "a\r\nbb\n\nc\n";
        let lines: Vec<(usize, &str)> = lines_with_starts(code).collect();
        assert_eq!(lines, vec![(0, "a"), (3, "bb"), (6, ""), (7, "c")]);
        assert_eq!(lines.len(), code.lines().count());
    }
}
