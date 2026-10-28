//! `SelectableText` — text the pointer can select, and the hit-testing every selectable leaf shares
//! (feature 1550, research R5; contracts/selectable-text.md *T-1*–*T-6*; FR-001, FR-004, FR-005,
//! FR-020).
//!
//! [`Selectable`] wraps the element a leaf already draws (`Text`'s text, `LinkText`'s paragraph, a
//! code or diff line) and adds what the leaf cannot: it measures the same text as **one paragraph
//! per hard line** ([`Measure`]) and answers where the pointer is in it. iced's
//! `Paragraph::hit_test` gives a byte index *within one buffer line* and drops the line, so one
//! paragraph per hard line is what makes the index unambiguous; the widget adds the line's start.
//!
//! - **T-1**: a left press on it publishes `on_hit` with the grapheme boundary nearest the pointer,
//!   as an offset into the whole text. It never captures the event, so the selection area around
//!   it sees the press too. A press its child captured (a link) publishes nothing (T-5).
//! - **T-2**: while the left button is down, a cursor move or a redraw with the cursor over it
//!   publishes again when the offset changed — also for a unit the drag entered after the press,
//!   which is how a selection crosses units.
//! - **T-3**: `line` is the byte range of the visual row under the pointer.
//! - **T-6**: a widget operation reports [`Shown`]: the caller's slot and the text it measures.
//!
//! The highlight is drawn by the child: a leaf with a selected range splits its spans at the
//! range's ends and draws the selected ones on [`highlight`]'s fill in its ink ([`split_at`]).
//! With no range the child is exactly the leaf's own element, so nothing extra is drawn (T-4).
//!
//! [`SelectableText`] is the component built on it: a run of markdown spans as `LinkText` draws
//! them, selectable.

use std::any::Any;
use std::ops::Range;
use std::rc::Rc;

use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer;
use iced::advanced::text::{self as core_text, Paragraph as _, Shaping};
use iced::advanced::widget::{tree, Operation, Tree, Widget};
use iced::advanced::{mouse, Clipboard, Shell};
use iced::widget::span;
use iced::widget::text::{Alignment, LineHeight, Span as TextSpan, Wrapping};
use iced::{alignment, Element, Event, Font, Pixels, Point, Rectangle, Size, Vector};
use micold_core::markdown::Span;
use micold_core::tokens::{Rgb, Roles};
use unicode_segmentation::UnicodeSegmentation;

use super::link_text::LinkText;
use super::TypeRole;

type Paragraph = <iced::Renderer as core_text::Renderer>::Paragraph;

/// How many clicks a hit's press was: a word on the second, a line on the third (FR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Click {
    Single,
    Double,
    Triple,
}

impl Click {
    /// The count iced's click tracking gives a press.
    pub fn of(kind: mouse::click::Kind) -> Self {
        match kind {
            mouse::click::Kind::Single => Self::Single,
            mouse::click::Kind::Double => Self::Double,
            mouse::click::Kind::Triple => Self::Triple,
        }
    }

    /// The click as a count, 1 to 3.
    pub fn count(self) -> u8 {
        match self {
            Self::Single => 1,
            Self::Double => 2,
            Self::Triple => 3,
        }
    }
}

/// Where the pointer is in a selectable text: `offset` is a grapheme boundary of the whole text,
/// `line` the visual row under the pointer, both in bytes of the text the slot names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub offset: usize,
    pub click: Click,
    pub line: Range<usize>,
}

/// What a selectable leaf reports to a widget operation (T-6): the caller's slot, where its text
/// starts in the slot's text (a code line's start in its block), and the text.
pub struct Shown {
    pub slot: Box<dyn Any>,
    pub base: usize,
    pub text: String,
}

/// The selection's fill and the ink of selected text: `primary_container` and
/// `on_primary_container` on every surface, which `tokens_contrast` pins at 4.5:1 (FR-020).
pub fn highlight(r: Roles) -> (Rgb, Rgb) {
    (r.primary_container, r.on_primary_container)
}

/// The pieces `ranges` (in order, over one text) cut at the ends of `selected`, each with whether
/// it is selected. Empty pieces are dropped, empty ranges included; with no selection the other
/// pieces are returned whole.
pub fn split_at(
    ranges: impl IntoIterator<Item = Range<usize>>,
    selected: Option<&Range<usize>>,
) -> Vec<(Range<usize>, bool)> {
    let mut out = Vec::new();
    for range in ranges.into_iter().filter(|r| !r.is_empty()) {
        let Some(sel) = selected.filter(|s| s.start < range.end && range.start < s.end) else {
            out.push((range, false));
            continue;
        };
        let (from, to) = (sel.start.max(range.start), sel.end.min(range.end));
        for (piece, on) in [
            (range.start..from, false),
            (from..to, true),
            (to..range.end, false),
        ] {
            if !piece.is_empty() {
                out.push((piece, on));
            }
        }
    }
    out
}

/// How many columns apart the tab stops are when a tab is drawn as spaces.
const TAB_STOP: usize = 4;

/// `line` with each tab drawn as spaces up to the next tab stop (the text stack draws a tab as a
/// missing glyph), and for each char boundary of `line` where it lands in the drawn text, the end
/// included: `(byte in line, byte in drawn)`.
pub fn expand_tabs(line: &str) -> (String, Vec<(usize, usize)>) {
    let mut out = String::with_capacity(line.len());
    let mut at = Vec::with_capacity(line.len() + 1);
    let mut column = 0;
    for (i, c) in line.char_indices() {
        at.push((i, out.len()));
        if c == '\t' {
            let pad = TAB_STOP - column % TAB_STOP;
            out.extend(std::iter::repeat_n(' ', pad));
            column += pad;
        } else {
            out.push(c);
            column += 1;
        }
    }
    at.push((line.len(), out.len()));
    (out, at)
}

/// How the measured text is shaped: the size, line height, face and wrapping its child draws it
/// in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shape {
    pub size: f32,
    pub line_height: LineHeight,
    pub font: Font,
    pub wrapping: Wrapping,
    /// The shaping the child uses: `Advanced` for rich text, the default (`Auto`) for plain text.
    pub shaping: Shaping,
}

impl Shape {
    /// A type role's text: its size, line height and face, wrapped as `wrapping`.
    pub fn role(role: TypeRole, monospace: bool, wrapping: Wrapping) -> Self {
        Self {
            size: role.size(),
            line_height: role.line_height(),
            font: if monospace {
                Font::MONOSPACE
            } else {
                role.font()
            },
            wrapping,
            shaping: Shaping::Advanced,
        }
    }
}

/// One hard line of a measure: where it starts in the text, its length there, the pieces it is
/// drawn as, and, when the drawn text differs from the text (tabs, padding), where each char
/// boundary of the line lands in the drawn text.
#[derive(Debug, Clone, PartialEq)]
struct Line {
    start: usize,
    len: usize,
    pieces: Vec<(String, Font)>,
    map: Option<Vec<(usize, usize)>>,
}

impl Line {
    /// The byte of the line nearest to drawn byte `d`.
    fn to_text(&self, d: usize) -> usize {
        let Some(map) = &self.map else {
            return d.min(self.len);
        };
        let after = map.partition_point(|&(_, drawn)| drawn <= d);
        let below = map[after.saturating_sub(1)];
        match map.get(after) {
            Some(&(text, drawn)) if drawn - d < d - below.1 => text,
            _ => below.0,
        }
    }
}

/// The text a [`Selectable`] measures, as its child draws it: one paragraph per hard line.
#[derive(Debug, Clone, PartialEq)]
pub struct Measure {
    shape: Shape,
    text: String,
    lines: Vec<Line>,
}

impl Measure {
    /// `pieces` drawn as they are, in their faces; the text is their concatenation and a `\n` in
    /// it starts a hard line.
    pub fn new<'p>(pieces: impl IntoIterator<Item = (&'p str, Font)>, shape: Shape) -> Self {
        let mut text = String::new();
        let mut lines = vec![Line {
            start: 0,
            len: 0,
            pieces: Vec::new(),
            map: None,
        }];
        for (piece, font) in pieces {
            for (k, part) in piece.split('\n').enumerate() {
                if k > 0 {
                    text.push('\n');
                    lines.push(Line {
                        start: text.len(),
                        len: 0,
                        pieces: Vec::new(),
                        map: None,
                    });
                }
                let line = lines.last_mut().expect("a measure has a line");
                if !part.is_empty() {
                    line.pieces.push((part.to_owned(), font));
                    line.len += part.len();
                }
                text.push_str(part);
            }
        }
        Self { shape, text, lines }
    }

    /// `text` in one face.
    pub fn plain(text: &str, shape: Shape) -> Self {
        Self::new([(text, shape.font)], shape)
    }

    /// One code line, drawn with its tabs as spaces and padded with spaces to `width` characters
    /// (a diff line runs its tint to the widest line's edge). A hit in the drawn text is answered
    /// in bytes of `line`.
    pub fn code_line(line: &str, width: usize, shape: Shape) -> Self {
        let (mut drawn, map) = expand_tabs(line);
        let pad = width.saturating_sub(drawn.chars().count());
        drawn.extend(std::iter::repeat_n(' ', pad));
        let unchanged = drawn.len() == line.len();
        Self {
            shape,
            text: line.to_owned(),
            lines: vec![Line {
                start: 0,
                len: line.len(),
                pieces: vec![(drawn, shape.font)],
                map: (!unchanged).then_some(map),
            }],
        }
    }

    /// The text the measure answers in.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// One paragraph per hard line, laid out `width` wide.
    fn paragraphs(&self, width: f32) -> Vec<Paragraph> {
        let s = self.shape;
        self.lines
            .iter()
            .map(|line| {
                let spans: Vec<TextSpan<'_, (), Font>> = line
                    .pieces
                    .iter()
                    .map(|(text, font)| span(text.as_str()).font(*font))
                    .collect();
                Paragraph::with_spans(core_text::Text {
                    content: spans.as_slice(),
                    bounds: Size::new(width, f32::INFINITY),
                    size: Pixels(s.size),
                    line_height: s.line_height,
                    font: s.font,
                    align_x: Alignment::Default,
                    align_y: alignment::Vertical::Top,
                    shaping: s.shaping,
                    wrapping: s.wrapping,
                })
            })
            .collect()
    }
}

/// What a hit sends, from the hit.
type OnHit<'a, M> = Rc<dyn Fn(Hit) -> M + 'a>;

/// A leaf's element, made selectable: see the module docs. Builder form (Principle VIII):
/// `Selectable::new(child, measure, slot, on_hit).base(start).into()`.
pub struct Selectable<'a, M> {
    child: Element<'a, M>,
    measure: Measure,
    base: usize,
    slot: Option<Box<dyn Any>>,
    on_hit: OnHit<'a, M>,
}

impl<'a, M: 'a> Selectable<'a, M> {
    /// `child`, drawing the text `measure` measures, answering hits as `on_hit(slot, hit)`.
    pub fn new<K: Clone + 'static>(
        child: impl Into<Element<'a, M>>,
        measure: Measure,
        slot: K,
        on_hit: impl Fn(K, Hit) -> M + 'a,
    ) -> Self {
        let key = slot.clone();
        Self {
            child: child.into(),
            measure,
            base: 0,
            slot: Some(Box::new(key)),
            on_hit: Rc::new(move |hit| on_hit(slot.clone(), hit)),
        }
    }

    /// Where this text starts in the slot's text: a code line's start in its block. Hits and the
    /// reported text are offset by it.
    pub fn base(mut self, base: usize) -> Self {
        self.base = base;
        self
    }
}

/// The paragraphs a [`Selectable`] measured, and the gesture it is following.
struct State {
    measured: Option<(Measure, f32)>,
    /// Each hard line's paragraph, top and height.
    paragraphs: Vec<(Paragraph, f32, f32)>,
    /// The left button is down (a press anywhere, not yet released).
    down: bool,
    /// The offset last published in this gesture.
    last: Option<usize>,
    /// The last press on this text, for double and triple clicks.
    previous: Option<mouse::Click>,
    /// The click the gesture's press on this text was (`Single` when it began elsewhere).
    click: Click,
    /// What the last operation reported, kept so the operation can borrow it.
    shown: Option<Shown>,
}

impl<M> Selectable<'_, M> {
    /// The offset and visual line under `p`, relative to the text's top-left.
    fn hit_at(&self, state: &State, p: Point) -> Option<(usize, Range<usize>)> {
        let last = state.paragraphs.len().checked_sub(1)?;
        let i = state
            .paragraphs
            .iter()
            .position(|(_, top, height)| p.y < top + height)
            .unwrap_or(last);
        let (paragraph, top, height) = &state.paragraphs[i];
        let y = (p.y - top).clamp(0.0, (height - 0.01).max(0.0));
        let at = |x: f32| match paragraph.hit_test(Point::new(x, y)) {
            Some(core_text::Hit::CharOffset(offset)) => offset,
            None => 0,
        };
        let line = &self.measure.lines[i];
        let text = |drawn: usize| {
            let offset = line.start + line.to_text(drawn);
            self.base + nearest_boundary(&self.measure.text, offset)
        };
        Some((text(at(p.x)), text(at(0.0))..text(at(1.0e7))))
    }

    fn publish(&self, state: &mut State, p: Point, shell: &mut Shell<'_, M>) {
        let Some((offset, line)) = self.hit_at(state, p) else {
            return;
        };
        if state.last == Some(offset) {
            return;
        }
        state.last = Some(offset);
        shell.publish((self.on_hit)(Hit {
            offset,
            click: state.click,
            line,
        }));
    }
}

/// The grapheme boundary of `text` nearest `offset`.
fn nearest_boundary(text: &str, offset: usize) -> usize {
    let offset = offset.min(text.len());
    let mut below = 0;
    for (at, _) in text.grapheme_indices(true) {
        if at >= offset {
            return if at - offset < offset - below {
                at
            } else {
                below
            };
        }
        below = at;
    }
    if text.len() - offset < offset - below {
        text.len()
    } else {
        below
    }
}

impl<'a, M: 'a> Widget<M, iced::Theme, iced::Renderer> for Selectable<'a, M> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State {
            measured: None,
            paragraphs: Vec::new(),
            down: false,
            last: None,
            previous: None,
            click: Click::Single,
            shown: None,
        })
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.child)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.child));
    }

    fn size(&self) -> Size<iced::Length> {
        self.child.as_widget().size()
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let child = self
            .child
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits);
        let width = match self.measure.shape.wrapping {
            Wrapping::None => f32::INFINITY,
            _ => limits.max().width,
        };
        let state = tree.state.downcast_mut::<State>();
        let stale = state
            .measured
            .as_ref()
            .is_none_or(|(measure, w)| *w != width || *measure != self.measure);
        if stale {
            let s = self.measure.shape;
            let line = s.line_height.to_absolute(Pixels(s.size)).0;
            let mut top = 0.0;
            state.paragraphs = self
                .measure
                .paragraphs(width)
                .into_iter()
                .map(|p| {
                    let height = p.min_bounds().height.max(line);
                    let at = top;
                    top += height;
                    (p, at, height)
                })
                .collect();
            state.measured = Some((self.measure.clone(), width));
        }
        // The child's own node, not a node around it: the wrapper adds no level to the layout
        // tree, so a leaf made selectable lays out exactly as before (T-4; layout_snapshot).
        child
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let child = layout;
        self.child.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            child,
            cursor,
            viewport,
        );
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        let state = tree.state.downcast_mut::<State>();
        if let Some(slot) = self.slot.take() {
            state.shown = Some(Shown {
                slot,
                base: self.base,
                text: self.measure.text.clone(),
            });
        }
        if let Some(shown) = state.shown.as_mut() {
            operation.custom(None, layout.bounds(), shown as &mut dyn Any);
        }
        let child = layout;
        self.child
            .as_widget_mut()
            .operate(&mut tree.children[0], child, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        let child = layout;
        self.child.as_widget_mut().update(
            &mut tree.children[0],
            event,
            child,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
        let bounds = layout.bounds();
        let origin = Vector::new(bounds.x, bounds.y);
        let state = tree.state.downcast_mut::<State>();
        match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                state.down = true;
                state.last = None;
                state.click = Click::Single;
                // A press the child took (a link) opens it and starts nothing (T-5, FR-004).
                if shell.is_event_captured() {
                    return;
                }
                let Some(p) = cursor.position_over(bounds) else {
                    return;
                };
                let click = mouse::Click::new(p, mouse::Button::Left, state.previous);
                state.previous = Some(click);
                state.click = Click::of(click.kind());
                self.publish(state, p - origin, shell);
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.down = false;
                state.last = None;
            }
            Event::Mouse(mouse::Event::CursorMoved { .. })
            | Event::Window(iced::window::Event::RedrawRequested(_))
                if state.down =>
            {
                if let Some(p) = cursor.position_over(bounds) {
                    self.publish(state, p - origin, shell);
                }
            }
            _ => {}
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let child = layout;
        match self.child.as_widget().mouse_interaction(
            &tree.children[0],
            child,
            cursor,
            viewport,
            renderer,
        ) {
            mouse::Interaction::None | mouse::Interaction::Idle
                if cursor.is_over(layout.bounds()) =>
            {
                mouse::Interaction::Text
            }
            other => other,
        }
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<iced::advanced::overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let child = layout;
        self.child.as_widget_mut().overlay(
            &mut tree.children[0],
            child,
            renderer,
            viewport,
            translation,
        )
    }
}

impl<'a, M: 'a> From<Selectable<'a, M>> for Element<'a, M> {
    fn from(s: Selectable<'a, M>) -> Self {
        Element::new(s)
    }
}

/// What a link press sends, from its address.
type OnLink<M> = Rc<dyn Fn(String) -> M>;

/// A run of formatted text the pointer can select (contracts/selectable-text.md). Builder form
/// (Principle VIII): `SelectableText::new(&spans, roles).role(TypeRole::Body).monospace()
/// .muted(false).wrapping(Wrapping::WordOrGlyph).slot(key).selected(range).on_hit(f).on_link(g)
/// .into()`.
///
/// The spans are drawn as [`LinkText`] draws them; without [`Self::on_hit`] it *is* that
/// `LinkText`.
pub struct SelectableText<'a, M, K> {
    spans: &'a [Span],
    roles: Roles,
    role: TypeRole,
    monospace: bool,
    muted: bool,
    wrapping: Wrapping,
    slot: Option<K>,
    selected: Option<Range<usize>>,
    on_hit: Option<Rc<dyn Fn(K, Hit) -> M>>,
    on_link: Option<OnLink<M>>,
}

impl<'a, M: Clone + 'static, K: Clone + Default + 'static> SelectableText<'a, M, K> {
    /// `spans` as body text, nothing selected, answering no hits until [`Self::on_hit`].
    pub fn new(spans: &'a [Span], roles: Roles) -> Self {
        Self {
            spans,
            roles,
            role: TypeRole::Body,
            monospace: false,
            muted: false,
            wrapping: Wrapping::WordOrGlyph,
            slot: None,
            selected: None,
            on_hit: None,
            on_link: None,
        }
    }

    /// The type role.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Set the text in `Font::MONOSPACE` at the role's size.
    pub fn monospace(mut self) -> Self {
        self.monospace = true;
        self
    }

    /// Draw the text in `on_surface_variant`.
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// How the text breaks when it does not fit.
    pub fn wrapping(mut self, wrapping: Wrapping) -> Self {
        self.wrapping = wrapping;
        self
    }

    /// The unit this text is, echoed in each hit (a `Default` key when unset).
    pub fn slot(mut self, slot: K) -> Self {
        self.slot = Some(slot);
        self
    }

    /// The selected bytes of the text, drawn highlighted.
    pub fn selected(mut self, range: Option<Range<usize>>) -> Self {
        self.selected = range;
        self
    }

    /// What a hit sends.
    pub fn on_hit(mut self, f: impl Fn(K, Hit) -> M + 'static) -> Self {
        self.on_hit = Some(Rc::new(f));
        self
    }

    /// What pressing an activatable link sends, from its address (as `LinkText`).
    pub fn on_link(mut self, f: impl Fn(String) -> M + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }
}

impl<'a, 'b, M: Clone + 'static, K: Clone + Default + 'static> From<SelectableText<'a, M, K>>
    for Element<'b, M>
{
    fn from(t: SelectableText<'a, M, K>) -> Self {
        let mut text = LinkText::new(t.spans, t.roles)
            .role(t.role)
            .muted(t.muted)
            .wrapping(t.wrapping);
        if t.monospace {
            text = text.monospace();
        }
        if let Some(on_link) = t.on_link {
            text = text.on_link(move |url| on_link(url));
        }
        if let Some(on_hit) = t.on_hit {
            let slot = t.slot.unwrap_or_default();
            text = text.selectable(slot, t.selected, move |k, hit| on_hit(k, hit));
        }
        text.into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{roles, size_of, Mounted, ShownTexts};
    use super::super::Text;
    use super::*;
    use micold_core::markdown::{Link, Style};

    fn plain(text: &str) -> Span {
        Span {
            text: text.to_owned(),
            style: Style::default(),
            link: None,
        }
    }

    /// The advance of one monospace character at the body role, measured from a laid-out text
    /// rather than from the widget under test.
    fn advance() -> f32 {
        let ten = Text::<String>::new("0123456789", TypeRole::Body, roles()).monospace();
        size_of(ten.into(), Size::new(1000.0, 100.0)).width / 10.0
    }

    fn row() -> f32 {
        TypeRole::Body.line_height_dp()
    }

    fn hit_message(slot: &'static str, hit: Hit) -> String {
        format!(
            "{slot} {} {:?} {}..{}",
            hit.offset, hit.click, hit.line.start, hit.line.end
        )
    }

    fn mono(spans: &[Span]) -> SelectableText<'_, String, &'static str> {
        SelectableText::new(spans, roles())
            .monospace()
            .slot("u")
            .on_hit(hit_message)
    }

    fn offsets(messages: &[String]) -> Vec<usize> {
        messages
            .iter()
            .filter_map(|m| m.split(' ').nth(1)?.parse().ok())
            .collect()
    }

    fn release(m: &mut Mounted<'_>, at: Point) -> Vec<String> {
        m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        )
    }

    /// T-1: a press publishes the grapheme boundary nearest the pointer, in the whole text — line
    /// start plus in-line index, on the second and third hard line too — and is not captured.
    #[test]
    fn a_press_publishes_the_nearest_boundary_in_the_whole_text() {
        let spans = [plain("abcdef\nghijkl\nmnopqr")];
        let (a, h) = (advance(), row());
        let mut m = Mounted::new(mono(&spans));
        let mut press = |x: f32, y: f32| {
            let at = Point::new(x, y);
            let (out, captured) = m.send_captured(
                Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
                mouse::Cursor::Available(at),
            );
            assert!(!captured, "a press on text is never captured");
            release(&mut m, at);
            out
        };
        assert_eq!(offsets(&press(2.3 * a, 0.5 * h)), vec![2]);
        assert_eq!(offsets(&press(2.7 * a, 1.5 * h)), vec![7 + 3]);
        assert_eq!(offsets(&press(1.2 * a, 2.5 * h)), vec![14 + 1]);
        assert_eq!(
            offsets(&press(40.0 * a, 1.5 * h)),
            vec![13],
            "past a line's end is its end"
        );
    }

    /// T-2: with the button down, a move publishes again only when the offset changed, a redraw
    /// with the cursor over it too; after the release nothing is published.
    #[test]
    fn a_drag_publishes_each_new_offset_once() {
        let spans = [plain("abcdefgh")];
        let (a, h) = (advance(), row());
        let mut m = Mounted::new(mono(&spans));
        assert_eq!(offsets(&m.press(Point::new(1.2 * a, h / 2.0))), vec![1]);
        assert!(m.hover(Point::new(1.3 * a, h / 2.0)).is_empty());
        assert_eq!(offsets(&m.hover(Point::new(4.2 * a, h / 2.0))), vec![4]);
        let (out, _) = m.tick(
            std::time::Instant::now(),
            mouse::Cursor::Available(Point::new(6.2 * a, h / 2.0)),
        );
        assert_eq!(offsets(&out), vec![6], "a redraw under the pointer");
        release(&mut m, Point::new(6.2 * a, h / 2.0));
        assert!(m.hover(Point::new(2.2 * a, h / 2.0)).is_empty());
    }

    /// T-2: a drag that began outside the text publishes once the pointer is over it, so a
    /// selection can cross into it.
    #[test]
    fn a_drag_from_elsewhere_publishes_over_the_text() {
        let spans = [plain("abcdefgh")];
        let (a, h) = (advance(), row());
        let mut m = Mounted::new(mono(&spans));
        assert!(m.press(Point::new(2.0 * a, 10.0 * h)).is_empty());
        assert_eq!(offsets(&m.hover(Point::new(3.2 * a, h / 2.0))), vec![3]);
    }

    /// T-3: `line` is the visual row under the pointer in wrapped text.
    #[test]
    fn line_is_the_visual_row_under_the_pointer() {
        let spans = [plain("aaaa bbbb cccc dddd")];
        let (a, h) = (advance(), row());
        let mut m = Mounted::new_in(mono(&spans), Size::new(11.0 * a, 400.0));
        let out = m.press(Point::new(1.2 * a, 1.5 * h));
        let line = out[0].split(' ').nth(3).expect("a line").to_owned();
        assert_eq!(line, "10..19", "the second row: {out:?}");
    }

    /// T-5: a press on a link opens it and publishes no hit; a press beside it does.
    #[test]
    fn a_press_on_a_link_opens_it_and_selects_nothing() {
        let spans = [
            Span {
                link: Some(Link {
                    target: "https://example.com".to_owned(),
                    activatable: true,
                }),
                ..plain("link")
            },
            plain(" text"),
        ];
        let (a, h) = (advance(), row());
        let mut m = Mounted::new(mono(&spans).on_link(|url| format!("open {url}")));
        let on_link = Point::new(1.5 * a, h / 2.0);
        let mut out = m.press(on_link);
        out.extend(release(&mut m, on_link));
        assert_eq!(out, vec!["open https://example.com".to_owned()]);
        let beside = Point::new(6.2 * a, h / 2.0);
        assert_eq!(offsets(&m.press(beside)), vec![6]);
    }

    /// T-6: the operation reports the slot and the text, which is the text drawn.
    #[test]
    fn the_operation_reports_the_slot_and_the_drawn_text() {
        let spans = [plain("one "), plain("two\nthree")];
        let mut m = Mounted::new(mono(&spans));
        let reported = m.drive(ShownTexts::<&str>(Vec::new())).expect("reported");
        assert_eq!(reported, vec![("u", 0, "one two\nthree".to_owned())]);
        // The painted record joins a paragraph's lines with a space: compare the words.
        let painted: String = m.painted_text().into_iter().map(|p| p.content).collect();
        let words = |t: &str| t.split_whitespace().map(str::to_owned).collect::<Vec<_>>();
        assert_eq!(words(&painted), words("one two\nthree"));
    }

    /// FR-005: a second and third press in place carry `Double` and `Triple`.
    #[test]
    fn repeated_presses_count_as_double_and_triple_clicks() {
        let spans = [plain("abcdefgh")];
        let at = Point::new(2.2 * advance(), row() / 2.0);
        let mut m = Mounted::new(mono(&spans));
        let mut clicks = Vec::new();
        for _ in 0..3 {
            clicks.extend(m.press(at));
            release(&mut m, at);
        }
        let kinds: Vec<&str> = clicks.iter().filter_map(|c| c.split(' ').nth(2)).collect();
        assert_eq!(kinds, vec!["Single", "Double", "Triple"]);
    }

    #[test]
    fn split_at_cuts_pieces_at_the_selection_ends() {
        assert_eq!(
            split_at([0..4, 4..9], Some(&(2..6))),
            vec![(0..2, false), (2..4, true), (4..6, true), (6..9, false)]
        );
        assert_eq!(split_at(std::iter::once(0..4), None), vec![(0..4, false)]);
        assert_eq!(
            split_at([0..1, 1..1, 1..3, 3..3], Some(&(2..3))),
            vec![(0..1, false), (1..2, false), (2..3, true)],
            "an empty range is no piece"
        );
    }

    #[test]
    fn a_code_line_answers_in_bytes_of_the_line_with_tabs_drawn_as_spaces() {
        let shape = Shape::role(TypeRole::Body, true, Wrapping::None);
        let measure = Measure::code_line("\tab", 10, shape);
        let line = &measure.lines[0];
        assert_eq!(line.pieces[0].0, "    ab    ");
        assert_eq!(line.to_text(0), 0);
        assert_eq!(line.to_text(1), 0, "inside the tab, nearer its start");
        assert_eq!(line.to_text(3), 1, "inside the tab, nearer its end");
        assert_eq!(line.to_text(5), 2);
        assert_eq!(line.to_text(9), 3, "the padding is the line's end");
    }
}
