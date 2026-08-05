//! `Typeahead` — a search field with an attached list of matched results (feature 021,
//! Constitution Principle VIII).
//!
//! The Material half: the field's treatment, the menu surface the results sit on, each row's
//! metrics, and the three things a row can say at once — which characters matched, whether the
//! keyboard is on it, and whether it is the current selection. All of it from the token set; none
//! of it chosen here.
//!
//! Positioning, capture and dismissal are not this module's job — [`cdk::typeahead`] does those and
//! is handed already-resolved values, the same arrangement `cdk::overlay` and `material::modal`
//! use.
//!
//! [`cdk::typeahead`]: crate::ui::cdk::typeahead
//!
//! Contract: `specs/021-branch-typeahead-search/contracts/typeahead-component.md` §1, §4.

use std::marker::PhantomData;
use std::ops::Range;

use super::style;
use super::text::TypeRole;
use crate::icons::Icon;
use crate::ui::cdk::typeahead::Typeahead as Behaviour;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Widget};
use iced::widget::{button, column, container, row, scrollable, text as text_widget, Space};
use iced::{alignment, Element, Length, Pixels, Rectangle, Size};
use micold_core::tokens::{spacing, Rgb, Roles};
use micold_core::typeahead::{fit_around, Direction};

/// One row of results: what it says, which of its characters matched, and whether it can be chosen.
///
/// A plain record the caller fills in, like [`MenuItem`](super::MenuItem) and
/// [`TreeItem`](super::TreeItem) — deliberately not a component. Whatever explains an unavailable
/// row must already be part of `label`; this module has no second text slot and no idea why any row
/// is disabled.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    /// The full text of the row.
    pub label: String,
    /// Byte ranges of `label` whose characters matched, ascending and non-overlapping.
    pub spans: Vec<Range<usize>>,
    /// Whether this row can be chosen. A row that cannot is still shown (contract §2).
    pub enabled: bool,
}

impl Row {
    /// A row that can be chosen.
    pub fn new(label: impl Into<String>, spans: Vec<Range<usize>>) -> Self {
        Self {
            label: label.into(),
            spans,
            enabled: true,
        }
    }

    /// Mark the row present but unchoosable.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// The role a result row's label is set in. A branch name is content, so it is body text — named
/// once here rather than at each of the three places the row measures, draws and spaces itself.
const ROW_ROLE: TypeRole = TypeRole::Body;

/// The distance between the field and its list.
const GAP: f32 = spacing::XS;
/// How tall the list may grow before it scrolls, in pixels — roughly eight rows.
const MAX_MENU_HEIGHT: f32 = 320.0;

/// A search field with a floating list of matched results.
///
/// Builder form (Principle VIII):
/// `Typeahead::new(query, &rows, Message::Typed, roles).placeholder("…").on_pick(…).into()`.
pub struct Typeahead<'a, M> {
    query: &'a str,
    rows: Vec<Row>,
    on_input: Box<dyn Fn(String) -> M + 'a>,
    roles: Roles,
    placeholder: String,
    open: bool,
    highlighted: Option<usize>,
    selected: Option<usize>,
    empty_message: Option<String>,
    on_pick: Option<Box<dyn Fn(usize) -> M + 'a>>,
    on_move: Option<Box<dyn Fn(Direction) -> M + 'a>>,
    on_focus: Option<M>,
    on_dismiss: Option<M>,
}

impl<'a, M: Clone + 'a> Typeahead<'a, M> {
    /// A field showing `query` over the already-matched, already-ranked `rows`, emitting
    /// `on_input` on every keystroke, themed by `roles`.
    ///
    /// The component does no matching, ranking or ordering of its own: it renders `rows` in the
    /// order given (contract §1.1).
    ///
    /// Rows arrive owned, as [`TreeView`](super::TreeView)'s items do: a caller builds them from
    /// whatever it matched this frame, so there is nowhere for them to live between frames.
    pub fn new(
        query: &'a str,
        rows: Vec<Row>,
        on_input: impl Fn(String) -> M + 'a,
        roles: Roles,
    ) -> Self {
        Self {
            query,
            rows,
            on_input: Box::new(on_input),
            roles,
            placeholder: "Search…".to_string(),
            open: false,
            highlighted: None,
            selected: None,
            empty_message: None,
            on_pick: None,
            on_move: None,
            on_focus: None,
            on_dismiss: None,
        }
    }

    /// Text shown when the field is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Whether the result list is showing. The caller owns this, so "open" can outlast an empty
    /// result set — which is what lets [`Self::empty_message`] ever be seen.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Which row the keyboard is on.
    pub fn highlighted(mut self, index: Option<usize>) -> Self {
        self.highlighted = index;
        self
    }

    /// Which row is the caller's current selection. Marked distinctly from the keyboard highlight,
    /// because both can sit on the same row at once (contract §4.7).
    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    /// What the list says when the query matches nothing. Without it, an empty list shows nothing.
    pub fn empty_message(mut self, message: impl Into<String>) -> Self {
        self.empty_message = Some(message.into());
        self
    }

    /// Emitted with the index of a chosen row. A disabled row never emits it.
    pub fn on_pick(mut self, f: impl Fn(usize) -> M + 'a) -> Self {
        self.on_pick = Some(Box::new(f));
        self
    }

    /// Emitted when the keyboard moves through the results.
    pub fn on_move(mut self, f: impl Fn(Direction) -> M + 'a) -> Self {
        self.on_move = Some(Box::new(f));
        self
    }

    /// Emitted when the field takes focus, so the caller can open the list.
    pub fn on_focus(mut self, message: M) -> Self {
        self.on_focus = Some(message);
        self
    }

    /// Emitted when the list should close without a pick.
    pub fn on_dismiss(mut self, message: M) -> Self {
        self.on_dismiss = Some(message);
        self
    }
}

/// One result row.
fn row_element<'a, M: Clone + 'a>(
    item: Row,
    highlighted: bool,
    selected: bool,
    press: Option<M>,
    r: Roles,
) -> Element<'a, M> {
    // Four channels, deliberately distinct (contract §4.3, §4.5, §4.7, FR-011, FR-012b):
    //   emphasis  → the text's own colour
    //   highlight → the row's state layer
    //   selection → the row's tonal fill, plus a leading marker
    //   disabled  → the whole label muted, and no emphasis accent to pick it back out
    let base = if item.enabled {
        r.on_surface
    } else {
        r.on_surface_variant
    };
    let accent = if item.enabled { r.primary } else { base };

    let label = EmphasisedLabel::<M>::new(item.label, item.spans, ROW_ROLE, base, accent);
    let marker = Text::marker(selected, r);

    let content = row![marker, label]
        .spacing(spacing::XS)
        .align_y(alignment::Vertical::Center);

    button(content)
        .padding([spacing::SM, spacing::SM])
        .width(Length::Fill)
        .style(style::menu_row(r, highlighted, selected))
        .on_press_maybe(press)
        .into()
}

/// The list: a menu surface anchored to the field, scrolling once it outgrows its height.
fn menu_element<'a, M: Clone + 'a>(
    rows: Vec<Row>,
    highlighted: Option<usize>,
    selected: Option<usize>,
    empty_message: Option<String>,
    on_pick: Option<&dyn Fn(usize) -> M>,
    r: Roles,
) -> Element<'a, M> {
    if rows.is_empty() {
        // An open list with nothing to say occupies nothing, so the caller can leave it open
        // through an empty query without a bare surface appearing under the field (C3.2).
        let Some(message) = empty_message else {
            return Space::new()
                .width(Length::Shrink)
                .height(Length::Fixed(0.0))
                .into();
        };
        return container(
            text_widget(message)
                .size(TypeRole::Caption.size())
                .style(style::muted(r)),
        )
        .padding(spacing::SM)
        .width(Length::Fill)
        .style(style::menu_surface(r))
        .into();
    }

    let mut list = column![].width(Length::Fill);
    for (index, item) in rows.into_iter().enumerate() {
        // A disabled row is present and readable but has nowhere to send a press, so it renders
        // unpressable rather than carrying a flag that could disagree with one (FR-012a).
        let press = item.enabled.then(|| on_pick.map(|f| f(index))).flatten();
        list = list.push(row_element(
            item,
            highlighted == Some(index),
            selected == Some(index),
            press,
            r,
        ));
    }

    container(
        scrollable(list)
            .style(style::scrollbar(r))
            .height(Length::Shrink),
    )
    .max_height(MAX_MENU_HEIGHT)
    .padding(spacing::XS)
    .width(Length::Fill)
    .style(style::menu_surface(r))
    .into()
}

impl<'a, M: Clone + 'a> From<Typeahead<'a, M>> for Element<'a, M> {
    fn from(t: Typeahead<'a, M>) -> Self {
        let Typeahead {
            query,
            rows,
            on_input,
            roles: r,
            placeholder,
            open,
            highlighted,
            selected,
            empty_message,
            on_pick,
            on_move,
            on_focus,
            on_dismiss,
        } = t;

        let highlighted_enabled = highlighted
            .and_then(|i| rows.get(i))
            .is_some_and(|row| row.enabled);
        let row_count = rows.len();

        let menu = menu_element(
            rows,
            highlighted,
            selected,
            empty_message,
            on_pick.as_deref(),
            r,
        );

        // Clearing is emptying the query, so it goes through the caller's own input handler
        // rather than a message of its own — resolved here, before the handler moves into the
        // input (FR-016).
        let cleared = on_input(String::new());

        // The field: the design system's text input, with the search affordance in the input's own
        // leading slot and the clear affordance beside it. The clear appears only when there is
        // something to clear, so an empty field carries no action that would do nothing.
        let input = iced::widget::text_input(&placeholder, query)
            .padding(spacing::SM)
            .size(TypeRole::Body.size())
            .on_input(on_input)
            .icon(iced::widget::text_input::Icon {
                font: super::glyph::MATERIAL_SYMBOLS,
                code_point: Icon::Search.glyph(),
                size: Some(Pixels(TypeRole::Body.size())),
                spacing: spacing::XS,
                side: iced::widget::text_input::Side::Left,
            })
            .style(style::input(r));

        // Always a row, even with nothing in the trailing slot. Swapping the field between a bare
        // input and a row would change the *type* of the widget at that position, and the rendering
        // stack rebuilds a subtree whose tag changed — throwing away the input's own state, focus
        // included. The field would lose focus on the first keystroke and again on the last
        // backspace, so nothing after the first character would ever reach it.
        let mut field_row = row![input]
            .spacing(spacing::XS)
            .align_y(alignment::Vertical::Center);
        if !query.is_empty() {
            field_row = field_row.push(super::IconButton::new(Icon::Close, r).on_press(cleared));
        }
        let field: Element<'a, M> = field_row.into();

        let mut behaviour = Behaviour::new(field, menu, open, GAP).keyboard(
            highlighted,
            row_count,
            highlighted_enabled,
        );

        // Enter takes the highlighted row, so the message is resolved here rather than in the
        // behaviour half — which knows an index but nothing about what sits at it.
        if let (Some(on_pick), Some(index)) = (&on_pick, highlighted) {
            if highlighted_enabled {
                behaviour = behaviour.on_pick(on_pick(index));
            }
        }
        if let Some(on_move) = on_move {
            behaviour = behaviour.on_move(on_move);
        }
        if let Some(on_dismiss) = on_dismiss {
            behaviour = behaviour.on_dismiss(on_dismiss);
        }
        if let Some(on_focus) = on_focus {
            behaviour = behaviour.on_focus(on_focus);
        }

        behaviour.into()
    }
}

/// A leading marker for the selected row, and nothing at all for the rest.
struct Text;

impl Text {
    fn marker<'a, M: 'a>(selected: bool, r: Roles) -> Element<'a, M> {
        if selected {
            text_widget("•")
                .size(TypeRole::Body.size())
                .style(move |_: &iced::Theme| iced::widget::text::Style {
                    color: Some(style::color(r.primary)),
                })
                .into()
        } else {
            Space::new()
                .width(Length::Fixed(TypeRole::Body.size() * 0.6))
                .into()
        }
    }
}

/// A single-line label whose matched characters are drawn in the emphasis treatment, truncated so
/// that the emphasis stays visible (FR-009, FR-010, FR-011c, FR-011d).
///
/// A widget rather than a `rich_text` because truncation has to happen at layout time, when the
/// renderer can shape text and the available width is known — the same reason
/// [`Ellipsized`](super::Ellipsized) is a widget. It shares that module's technique and none of its
/// code: this one draws several paragraphs in two colours, and that one draws one in a single
/// colour.
struct EmphasisedLabel<M> {
    content: String,
    spans: Vec<Range<usize>>,
    role: TypeRole,
    base: Rgb,
    accent: Rgb,
    marker: PhantomData<M>,
}

impl<M> EmphasisedLabel<M> {
    fn new(
        content: String,
        spans: Vec<Range<usize>>,
        role: TypeRole,
        base: Rgb,
        accent: Rgb,
    ) -> Self {
        Self {
            content,
            spans,
            role,
            base,
            accent,
            marker: PhantomData,
        }
    }
}

/// One drawn piece of the label: its shaped paragraph, whether it is emphasised, and where it sits.
struct Segment<P> {
    paragraph: P,
    emphasised: bool,
    x: f32,
}

/// The shaped label, plus what it was shaped for — so a re-render at the same width with the same
/// text reuses the paragraphs rather than measuring again on every frame.
struct State<P> {
    segments: Vec<Segment<P>>,
    width: f32,
    height: f32,
    source: String,
    /// The spans the segments were split at. Part of the key, not a passenger: a row keeps its
    /// place in the list as the query grows, so the same label at the same width routinely arrives
    /// with *different* emphasis — and a cache that ignored the spans would keep showing the
    /// characters the previous query matched.
    source_spans: Vec<Range<usize>>,
    for_width: f32,
}

impl<P> Default for State<P> {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            width: 0.0,
            height: 0.0,
            source: String::new(),
            source_spans: Vec::new(),
            // NaN compares unequal to everything, so the first layout always measures.
            for_width: f32::NAN,
        }
    }
}

/// The emphasised weight: the base font, one step heavier.
///
/// Derived from whatever font the row is already drawn in rather than named outright, so a change
/// of typeface carries the emphasis with it and this never becomes a second place the font is
/// chosen.
fn emphasis_font(base: iced::Font) -> iced::Font {
    iced::Font {
        weight: iced::font::Weight::Bold,
        ..base
    }
}

/// Splits `text` at `spans` into `(piece, emphasised)` runs, in order and with no gaps.
fn segments(text: &str, spans: &[Range<usize>]) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for span in spans {
        // A malformed span degrades to no emphasis rather than a panic (contract §2). Reversed and
        // mid-character spans are checked too: `&text[span]` panics on either, and the promise here
        // is that no span the caller can hand over takes the dialog down with it.
        if span.start > span.end
            || span.end > text.len()
            || span.start < at
            || !text.is_char_boundary(span.start)
            || !text.is_char_boundary(span.end)
        {
            continue;
        }
        if span.start > at {
            out.push((text[at..span.start].to_string(), false));
        }
        out.push((text[span.clone()].to_string(), true));
        at = span.end;
    }
    if at < text.len() {
        out.push((text[at..].to_string(), false));
    }
    out
}

impl<M, Theme, Renderer> Widget<M, Theme, Renderer> for EmphasisedLabel<M>
where
    // Bound to the concrete font so emphasis can name a weight, and so the row can be set in its
    // type role's own face. The library already draws every glyph and every label through
    // `iced::Font` (see `glyph.rs` and `text.rs`), so this rules out no renderer the application
    // can actually have — it only makes the existing assumption checkable.
    Renderer: text::Renderer<Font = iced::Font>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let available = limits.max().width;

        let template: CoreText<(), Renderer::Font> = CoreText {
            content: (),
            bounds: Size::INFINITE,
            size: Pixels(self.role.size()),
            line_height: text::LineHeight::default(),
            font: self.role.font(),
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        };

        if state.source != self.content
            || state.source_spans != self.spans
            || state.for_width != available
        {
            let measure = |candidate: &str| {
                Renderer::Paragraph::with_text(template.with_content(candidate))
                    .min_bounds()
                    .width
            };
            // The window follows the emphasis, so a match near the end of a long name is never the
            // part that gets cut off (FR-011d).
            let (fitted, spans) = fit_around(&self.content, &self.spans, available, measure);

            let mut x = 0.0;
            let mut height: f32 = 0.0;
            state.segments = segments(&fitted, &spans)
                .into_iter()
                .map(|(piece, emphasised)| {
                    // Colour *and* weight (contract §4.3). Two channels rather than one, because a
                    // colour alone is the channel a developer with a colour-vision deficiency is
                    // least likely to have — and it is also the channel the selected row's tonal
                    // fill sits closest to. Weight survives both.
                    let mut text = template.with_content(piece.as_str());
                    if emphasised {
                        text.font = emphasis_font(text.font);
                    }
                    let paragraph = Renderer::Paragraph::with_text(text);
                    let bounds = paragraph.min_bounds();
                    let segment = Segment {
                        paragraph,
                        emphasised,
                        x,
                    };
                    x += bounds.width;
                    height = height.max(bounds.height);
                    segment
                })
                .collect();

            state.width = x;
            state.height = height;
            state.source = self.content.clone();
            state.source_spans = self.spans.clone();
            state.for_width = available;
        }

        layout::Node::new(limits.resolve(
            Length::Fill,
            Length::Shrink,
            Size::new(state.width, state.height),
        ))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree.state.downcast_ref::<State<Renderer::Paragraph>>();
        let bounds = layout.bounds();
        let clip = bounds.intersection(viewport).unwrap_or(bounds);

        for segment in &state.segments {
            let colour = if segment.emphasised {
                self.accent
            } else {
                self.base
            };
            let at = iced::Point::new(bounds.x + segment.x, bounds.y);
            renderer.fill_paragraph(&segment.paragraph, at, style::color(colour), clip);
        }
    }
}

impl<'a, M, Theme, Renderer> From<EmphasisedLabel<M>> for Element<'a, M, Theme, Renderer>
where
    M: 'a,
    Theme: 'a,
    Renderer: text::Renderer<Font = iced::Font> + 'a,
{
    fn from(label: EmphasisedLabel<M>) -> Self {
        Element::new(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One span, as a slice.
    ///
    /// Spelled out rather than written `&[5..8]`, which reads ambiguously enough that the linter
    /// asks whether a one-element array of ranges or the range's own contents was meant.
    fn one(span: Range<usize>) -> [Range<usize>; 1] {
        [span]
    }

    /// The split has to cover the whole label with no gaps and no overlaps, or the row silently
    /// loses characters.
    #[test]
    fn segments_cover_the_whole_label_in_order() {
        let out = segments("feat/login", &one(5..8));
        assert_eq!(
            out,
            vec![
                ("feat/".to_string(), false),
                ("log".to_string(), true),
                ("in".to_string(), false),
            ]
        );
        let rejoined: String = out.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(rejoined, "feat/login");
    }

    /// Scattered spans — an abbreviation match — alternate correctly.
    #[test]
    fn scattered_spans_alternate() {
        let out = segments("feat/reporting", &[0..1, 5..8]);
        assert_eq!(out[0], ("f".to_string(), true));
        assert_eq!(out[1], ("eat/".to_string(), false));
        assert_eq!(out[2], ("rep".to_string(), true));
        assert_eq!(out[3], ("orting".to_string(), false));
    }

    /// No spans at all is the empty-query case: one unemphasised run.
    #[test]
    fn no_spans_yields_one_plain_run() {
        assert_eq!(
            segments("feat/login", &[]),
            vec![("feat/login".to_string(), false)]
        );
    }

    /// A span pointing outside the label degrades to no emphasis rather than panicking.
    #[test]
    fn a_malformed_span_is_ignored() {
        let out = segments("main", &one(2..99));
        let rejoined: String = out.iter().map(|(s, _)| s.as_str()).collect();
        assert_eq!(rejoined, "main");
    }
}
