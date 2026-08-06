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
use iced::widget::{button, column, row, Space};
use iced::{alignment, Element, Length, Pixels, Rectangle, Size};
use micold_core::tokens::{density, shape, spacing, Rgb, Roles};
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
/// How many rows the list shows before it scrolls.
///
/// Expressed in rows and multiplied by the density scale's menu-item height, rather than as a pixel
/// number that happens to be about eight rows today — a density step that changed the row height
/// would otherwise silently change how many rows fit.
const MAX_ROWS_BEFORE_SCROLL: f32 = 8.0;

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
    label: Option<String>,
    supporting: Option<String>,
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
            label: None,
            supporting: None,
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
    /// The control's name, rendered inside its container above the value (FR-031a).
    ///
    /// Forwarded to the search field's own `FormField`, so the branch picker's label sits where
    /// every other field's does. §7.7's migration table predates this control — it names the select
    /// the type-ahead replaced — and design-tokens §7.7 extends the same requirement here.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Explanatory text beneath the container.
    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

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

/// One result row — Material's menu item, in the library's own assembly of it.
///
/// The same three parts `material::menu`'s items are built from, in the same order: a leading slot,
/// a label, and a pressable container carrying the state layer. It differs in exactly two places,
/// both forced by what this row has to say. Its label is an [`EmphasisedLabel`] rather than a
/// [`Text`](super::Text), because part of it is emphasised; and it is set at `Body` rather than at
/// `Action`, because `Action` is already the medium weight and emphasis would then have nowhere to
/// step up to.
fn row_element<'a, M: Clone + 'a>(
    item: Row,
    highlighted: bool,
    selected: bool,
    press: Option<M>,
    r: Roles,
) -> Element<'a, M> {
    // Four channels, deliberately distinct (contract §4.3, §4.5, §4.7, FR-011, FR-012b):
    //   emphasis  → the label's own colour and weight
    //   highlight → the row's state layer
    //   selection → the row's tonal fill, plus a leading marker
    //   disabled  → the label muted, and no emphasis accent to pick it back out
    let base = if item.enabled {
        r.on_surface
    } else {
        r.on_surface_variant
    };
    let accent = if item.enabled { r.primary } else { base };

    let label = EmphasisedLabel::<M>::new(item.label, item.spans, ROW_ROLE, base, accent);

    let content = row![marker(selected, r), label]
        .spacing(spacing::SM)
        .align_y(alignment::Vertical::Center);

    let pressable = button(content)
        .width(Length::Fill)
        // Material's menu-item height, from the density scale rather than from whatever the padding
        // happened to add up to — so a row keeps its touch target when its label is short.
        .height(Length::Fixed(density::MENU_ITEM_BASE))
        .padding([0.0, spacing::SM])
        .style(style::menu_row(r, highlighted, selected))
        .on_press_maybe(press.clone());

    match press {
        // Every pressable surface ripples (feature 019, FR-024c), and a menu row is one — built
        // here rather than through `material::Button`, exactly as `material::menu`'s items are, so
        // the ripple is composed explicitly.
        Some(_) => super::Ripple::new(pressable, r.on_surface, shape::SMALL).into(),
        // A row with nothing to press must not ripple. The ripple's whole message is "that did
        // something", and pressing an unavailable branch does nothing at all (FR-012a) — so the
        // wrapper is absent rather than present and lying.
        None => pressable.into(),
    }
}

/// The leading slot of a result row: Material's selected-item check, or the space it would occupy.
///
/// The space is kept when nothing is selected so every label in the list starts at the same x —
/// a marker that shifted the text sideways would make the selection the loudest thing on the row
/// rather than the quietest.
fn marker<'a, M: 'a>(selected: bool, r: Roles) -> Element<'a, M> {
    let size = TypeRole::Action.size();
    if selected {
        super::Glyph::new(Icon::ActiveMarker, TypeRole::Action, r)
            .tint(r.primary)
            .into()
    } else {
        Space::new().width(Length::Fixed(size)).into()
    }
}

/// The list: the library's own menu panel, anchored to the field, scrolling once it outgrows its
/// height.
///
/// [`menu_panel`](super::menu_panel) is what every floating popover in the application sits on, so
/// the elevation, the corner and the padding are the menu surface's rather than this component's.
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
        // Prose about the search rather than a row of it, so it is `Caption` and muted — it must
        // not read as a result that can be picked.
        return super::menu_panel(
            super::Text::new(message, TypeRole::Caption, r).muted(),
            Length::Fill,
            r,
            true,
        );
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

    // The cap is a layout constraint rather than a treatment, so it is a plain container: the
    // overlay already refuses to grow past the room on screen, and this stops a repository with two
    // hundred branches from taking all of it.
    let capped = iced::widget::container(super::Scrollable::new(list, r).height(Length::Shrink))
        .max_height(density::MENU_ITEM_BASE * MAX_ROWS_BEFORE_SCROLL);

    super::menu_panel(capped, Length::Fill, r, true)
}

impl<'a, M: Clone + 'a> From<Typeahead<'a, M>> for Element<'a, M> {
    fn from(t: Typeahead<'a, M>) -> Self {
        let Typeahead {
            query,
            rows,
            on_input,
            roles: r,
            placeholder,
            label,
            supporting,
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
        // field (FR-016).
        let cleared = on_input(String::new());

        // The field is the library's own text field, with Material's two affordances in their
        // named slots: the search icon leading, the clear action trailing. Neither is assembled
        // here — `TextField` grew both, so the next searchable picker gets them by asking.
        //
        // The clear action appears only when there is something to clear, so an empty field
        // carries no action that would do nothing.
        // `.active(open)` is the half the select cannot manage. §7.7 wants the active indicator to
        // follow **open** rather than focus (FR-043a), and `pick_list` reports its open state to
        // its own style closure and to nobody else — so `Select::active` must be supplied and in
        // practice is not. This control's openness is already a caller-held value, so the
        // indicator follows it for real. The accepted gap stands for the select and closes here.
        let mut input = super::TextField::new(placeholder, query, r)
            .leading_icon(Icon::Search)
            .active(open)
            .on_input(on_input);
        // Nothing here has to say whether the label rests or floats: `TextField` derives that from
        // its own value, which for the type-ahead *is* the query — so a picker with an empty search
        // box rests its label exactly as an empty input does, and one being typed into floats it.
        if let Some(label) = label {
            input = input.label(label);
        }
        if let Some(supporting) = supporting {
            input = input.supporting(supporting);
        }
        if !query.is_empty() {
            input = input.trailing_action(Icon::Close, cleared);
        }
        let field: Element<'a, M> = input.into();

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
