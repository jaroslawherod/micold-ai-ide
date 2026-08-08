//! The appearance both pickers share (feature 022, Constitution Principle VIII).
//!
//! A picker is a field with a list of choices anchored beneath it. The application has two — the
//! search picker (`material::typeahead`) and the select (`material::select`) — and everything below
//! is what they look like in common: the row, the panel it sits on, and the transition that brings
//! it in and takes it away. What differs stays with each control: one has a search field and
//! emphasised matches, the other a trigger and a chevron.
//!
//! **Extracted, not invented.** All of this was `material::typeahead`'s, written for one control and
//! correct for both. Moving it here is what stops the select from being a second, drifting copy of a
//! list that already exists — which is the whole of Principle VIII's argument.
//!
//! Positioning, capture and dismissal are not this module's job — [`cdk::picker`] does those and is
//! handed already-resolved values, the same arrangement `cdk::overlay` and `material::modal` use.
//!
//! [`cdk::picker`]: crate::ui::cdk::picker
//!
//! Contract: `specs/022-dedicated-select-component/contracts/picker-base.md` §2.

use std::marker::PhantomData;
use std::ops::Range;

use super::style;
use super::text::TypeRole;
use crate::icons::Icon;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Widget};
use iced::widget::{button, column, row, Space};
use iced::{alignment, Element, Length, Pixels, Rectangle, Size};
use micold_core::tokens::{density, motion::duration, shape, spacing, Rgb, Roles};
use micold_core::typeahead::fit_around;
use std::time::Duration;

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
pub(super) const ROW_ROLE: TypeRole = TypeRole::Body;

/// The distance between the field and its list.
pub(super) const GAP: f32 = spacing::XS;
/// How long the list takes to arrive, and how long it takes to leave.
///
/// §6.3's "menu fade in" and "menu fade out" rows, unchanged: a list is a menu, and this feature
/// introduces no animation of its own — it applies one the motion table already assigns to a surface
/// that was not drawing it. Feature 018's count of sanctioned new animations therefore does not move
/// (FR-020).
///
/// Leaving is quicker than arriving because leaving is acknowledgement and arriving is presentation.
pub(super) const ENTER: Duration = Duration::from_millis(duration::SHORT_3);
pub(super) const EXIT: Duration = Duration::from_millis(duration::SHORT_2);

/// How many rows the list shows before it scrolls.
///
/// Expressed in rows and multiplied by the density scale's menu-item height, rather than as a pixel
/// number that happens to be about eight rows today — a density step that changed the row height
/// would otherwise silently change how many rows fit.
const MAX_ROWS_BEFORE_SCROLL: f32 = 8.0;

/// One result row — Material's menu item, in the library's own assembly of it.
///
/// The same three parts `material::menu`'s items are built from, in the same order: a leading slot,
/// a label, and a pressable container carrying the state layer. It differs in exactly two places,
/// both forced by what this row has to say. Its label is an [`EmphasisedLabel`] rather than a
/// [`Text`](super::Text), because part of it is emphasised; and it is set at `Body` rather than at
/// `Action`, because `Action` is already the medium weight and emphasis would then have nowhere to
/// step up to.
/// `pub(super)` so `menu_anatomy` can measure a picker row's content against its 48dp. It was the
/// second half of BUG-004 and would otherwise be the fixed half nothing checks — which is how the
/// first half survived BUG-003. Both pickers' rows are this one function now, so the measurement
/// covers the select's as well without `menu_anatomy` gaining a second case.
pub(super) fn row_element<'a, M: Clone + 'a>(
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

    // `height(Fill)` for the reason `material::menu`'s item row states: a `Row`'s `align_y`
    // centres its children against each other inside the cross size the flex computed, and that
    // band lands at the top of the node `button` stretched to 48dp. This row is the same shape and
    // had the same defect — found while fixing the menu's (FR-030a).
    let content = row![marker(selected, r), label]
        .spacing(spacing::SM)
        .align_y(alignment::Vertical::Center)
        .height(Length::Fill);

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
pub(super) fn menu_element<'a, M: Clone + 'a>(
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
            spacing::XS,
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

    // `spacing::XS`, not §7.5's panel padding: the type-ahead's rows are the third copy of
    // the item row and T108 decides whether they become the shared one. Until then it keeps the
    // padding it had, rather than acquiring half of a change it is not part of.
    super::menu_panel(capped, Length::Fill, r, true, spacing::XS)
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

/// The list, wrapped in the transition that brings it in and takes it away (FR-018, FR-019).
///
/// Grow-and-fade: the panel arrives from `MIN_SCALE` at full transparency and settles at its own
/// size, and leaves the way it came but faster. Both wrappers are the library's existing ones, and
/// **neither curve is stated here** — `Motion`'s defaults are already `standard_decelerate` in and
/// `standard_accelerate` out, which is exactly what §6.3 gives a menu. Restating them would create a
/// second definition that can drift from the first.
///
/// `scale` transforms *drawing only* — it delegates layout, events and the overlay to its child — so
/// "nothing outside the list moves while it animates" (FR-023) holds by construction rather than by
/// care. The fade veils toward the menu surface's own tone, as `MenuOverlay` does, because veiling
/// toward the plain surface leaves a rectangle two tones too dark over an elevated panel.
pub(super) fn animated_menu<'a, M: Clone + 'a>(
    panel: Element<'a, M>,
    open: bool,
    r: Roles,
) -> Element<'a, M> {
    let faded = super::fade(panel, open, ENTER, super::SurfaceKind::Menu.tone(r))
        .exiting_over(EXIT)
        .rounded(super::SurfaceKind::Menu.shape())
        .animate_in();
    super::scale(faded, open, ENTER)
        .exiting_over(EXIT)
        .animate_in()
        .into()
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
