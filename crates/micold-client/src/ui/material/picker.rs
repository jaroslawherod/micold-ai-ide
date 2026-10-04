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
use iced::widget::{button, column, opaque, row, Space};
use iced::{alignment, Element, Length, Pixels, Rectangle, Size};
use micold_core::tokens::{density, motion::duration, shape, spacing, Rgb, Roles};
use micold_core::typeahead::fit_around;
use std::time::Duration;

/// One row of results: what it says, which of its characters matched, and whether it can be chosen.
///
/// A plain record the caller fills in, like [`MenuItem`](super::MenuItem) and
/// [`TreeItem`](super::TreeItem) — deliberately not a component. Whatever explains an unavailable
/// row must already be part of `label` or `details`; this module has no idea why any row is
/// disabled.
#[derive(Clone, PartialEq, Eq, Default)]
pub struct Row {
    /// The full text of the row.
    pub label: String,
    /// Byte ranges of `label` whose characters matched, ascending and non-overlapping.
    pub spans: Vec<Range<usize>>,
    /// Whether this row can be chosen. A row that cannot is still shown (contract §2).
    pub enabled: bool,
    /// A second line under the label, with the byte ranges of it that matched (feature 038).
    ///
    /// `None` is the single-line row every picker had before; `Some` makes the row two wrapping
    /// lines. See [`Row::details`].
    pub details: Option<(String, Vec<Range<usize>>)>,
    /// What the row's tooltip says once the cursor has rested on it; `None` is no tooltip. See
    /// [`Row::tooltip`].
    pub tooltip: Option<String>,
    /// What the row stands for, so a list that reuses the row for another choice starts its
    /// tooltip's wait again. See [`Row::key`].
    pub key: Option<u64>,
}

impl Row {
    /// A row that can be chosen.
    pub fn new(label: impl Into<String>, spans: Vec<Range<usize>>) -> Self {
        Self {
            label: label.into(),
            spans,
            enabled: true,
            details: None,
            tooltip: None,
            key: None,
        }
    }

    /// Give the row a second line: `text` under the label, smaller and lower in emphasis, with
    /// `spans` the byte ranges of `text` that matched.
    ///
    /// A row with details wraps both lines instead of truncating the label, and is as tall as its
    /// text needs (038 contracts/picker-row.md §1). A row without them is unchanged.
    pub fn details(mut self, text: impl Into<String>, spans: Vec<Range<usize>>) -> Self {
        self.details = Some((text.into(), spans));
        self
    }

    /// Give the row a tooltip: `text`, shown once the cursor has rested on the row for
    /// [`ROW_TOOLTIP_REST`], at most [`ROW_TOOLTIP_LINES`] lines of it (038 contracts/picker-row.md
    /// §4).
    ///
    /// The text is shown as given, and nothing is added to it. An empty text is no tooltip.
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        let text = text.into();
        self.tooltip = (!text.is_empty()).then_some(text);
        self
    }

    /// Say what the row stands for.
    ///
    /// A list reuses its rows: the row at one place stands for one choice now and for another once
    /// the list narrows. With a key, a tooltip that was open or waiting for the choice that left
    /// closes and waits again for the one that arrived (038 FR-017).
    pub fn key(mut self, key: u64) -> Self {
        self.key = Some(key);
        self
    }

    /// Mark the row present but unchoosable.
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// `{:?}` of a row prints its label and neither its second line nor its tooltip text: what a caller
/// puts there may be something no log should hold (038 FR-025: a reporter, a description).
impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        struct Redacted;
        impl std::fmt::Debug for Redacted {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("<redacted>")
            }
        }
        f.debug_struct("Row")
            .field("label", &self.label)
            .field("spans", &self.spans)
            .field("enabled", &self.enabled)
            .field("details", &self.details.as_ref().map(|_| Redacted))
            .field("tooltip", &self.tooltip.as_ref().map(|_| Redacted))
            .field("key", &self.key)
            .finish()
    }
}

/// How long the cursor rests on a row before the row's tooltip opens (038 FR-015).
pub const ROW_TOOLTIP_REST: Duration = Duration::from_secs(3);

/// The most lines a row's tooltip shows (038 FR-021).
pub const ROW_TOOLTIP_LINES: usize = 3;

/// The role a result row's label is set in. A branch name is content, so it is body text — named
/// once here rather than at each of the three places the row measures, draws and spaces itself.
pub(super) const ROW_ROLE: TypeRole = TypeRole::Body;

/// The role a row's second line is set in: supporting text under the label, so it is one step
/// smaller and drawn in the lower-emphasis colour (038 contracts/picker-row.md §1).
const DETAILS_ROLE: TypeRole = TypeRole::Caption;

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

/// The widget `Id` of the highlighted row, and of no other (feature 038, FR-007).
///
/// It is how `ui::picker_scroll` finds the row the keyboard is on: a row's height is whatever its
/// wrapping lines come to, so where the highlighted one lies cannot be computed from its index.
/// One `Id` for every list is enough, because one list has the keyboard at a time.
pub static PICKER_HIGHLIGHT: std::sync::LazyLock<iced::advanced::widget::Id> =
    std::sync::LazyLock::new(|| iced::advanced::widget::Id::new("picker-highlight"));

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

    let pressable = match item.details {
        Some((details, detail_spans)) => {
            // Two wrapping lines (feature 038). Nothing is truncated: each line takes as many
            // lines as the row's width needs, and the row is as tall as they come to.
            let details_base = r.on_surface_variant;
            let details_accent = if item.enabled {
                r.primary
            } else {
                details_base
            };
            let label = EmphasisedLabel::<M>::new(item.label, item.spans, ROW_ROLE, base, accent)
                .wrapping();
            let details = EmphasisedLabel::<M>::new(
                details,
                detail_spans,
                DETAILS_ROLE,
                details_base,
                details_accent,
            )
            .wrapping();

            // The marker is centred on the label's first line, not on the row: a row five lines
            // tall with its check half-way down would point at no line in particular.
            let marker = iced::widget::container(marker(selected, r))
                .height(Length::Fixed(ROW_ROLE.line_height_dp()))
                .align_y(alignment::Vertical::Center);
            let lines = row![marker, column![label, details].width(Length::Fill)]
                .spacing(spacing::SM)
                .align_y(alignment::Vertical::Top)
                .width(Length::Fill);

            // The menu item's height is the least a row is, as its touch target; `button` has no
            // minimum height, so a strut as tall as that height less the padding holds it, and
            // lines shorter than it are centred beside it. Lines taller than it set the height.
            // Its width is `Shrink`, which comes to nothing: a `Row` drops a child whose width is
            // stated as zero, and the strut would go with it.
            let strut = Space::new()
                .width(Length::Shrink)
                .height(Length::Fixed(density::MENU_ITEM_BASE - 2.0 * spacing::XS));
            let content = row![strut, lines].align_y(alignment::Vertical::Center);

            button(content)
                .width(Length::Fill)
                .height(Length::Shrink)
                .padding([spacing::XS, spacing::SM])
        }
        None => {
            let label = EmphasisedLabel::<M>::new(item.label, item.spans, ROW_ROLE, base, accent);

            // `height(Fill)` for the reason `material::menu`'s item row states: a `Row`'s `align_y`
            // centres its children against each other inside the cross size the flex computed, and
            // that band lands at the top of the node `button` stretched to 48dp. This row is the
            // same shape and had the same defect — found while fixing the menu's (FR-030a).
            let content = row![marker(selected, r), label]
                .spacing(spacing::SM)
                .align_y(alignment::Vertical::Center)
                .height(Length::Fill);

            button(content)
                .width(Length::Fill)
                // Material's menu-item height, from the density scale rather than from whatever
                // the padding happened to add up to — so a row keeps its touch target when its
                // label is short.
                .height(Length::Fixed(density::MENU_ITEM_BASE))
                .padding([0.0, spacing::SM])
        }
    }
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
        //
        // `opaque`, though, because "does nothing" has to mean the press **stops here**. Withholding
        // `on_press` is what makes the row unpressable, and it also makes the library's button
        // decline the event — so the press carried on past the row, past the floating list (which
        // claims only presses outside itself) and onto whatever was behind. Behind the branch list
        // is the add-worktree dialog's scrim, whose press message is its cancellation: reaching for
        // an in-use branch closed the form and discarded every input in it (016 BUG-002, FR-035).
        // `opaque` draws the content unchanged and swallows presses over its own bounds, so the row
        // stays exactly as unpressable and exactly as quiet as it looks.
        None => opaque(pressable),
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
    for (index, mut item) in rows.into_iter().enumerate() {
        // A disabled row is present and readable but has nowhere to send a press, so it renders
        // unpressable rather than carrying a flag that could disagree with one (FR-012a).
        let press = item.enabled.then(|| on_pick.map(|f| f(index))).flatten();
        let is_highlighted = highlighted == Some(index);
        let (tip, key) = (item.tooltip.take(), item.key);
        let row = row_element(item, is_highlighted, selected == Some(index), press, r);
        let row = if is_highlighted {
            // A container for its `Id` alone: it is the one widget that reports an `Id` with its
            // bounds to an operation, and it sizes itself as its content does, so the row lies
            // where it would without it.
            iced::widget::container(row)
                .id(PICKER_HIGHLIGHT.clone())
                .into()
        } else {
            row
        };
        list = list.push(match tip {
            // The tooltip is the outermost widget of the row, outside the highlight's container:
            // its wait and its open panel are state kept at the row's place in the list, and a
            // highlight arriving on the row or leaving it must not replace that place's widget.
            // It lays out its content alone, so the row lies where it would without it.
            Some(text) => {
                let tooltip = super::Tooltip::new(row, text, r)
                    .after_rest(ROW_TOOLTIP_REST)
                    .max_lines(ROW_TOOLTIP_LINES)
                    .position(super::TooltipPosition::Bottom);
                match key {
                    Some(key) => tooltip.subject(key),
                    None => tooltip,
                }
                .into()
            }
            None => row,
        });
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
/// In its [wrapping](EmphasisedLabel::wrapping) mode it is not truncated: label and emphasis are
/// shaped as one paragraph that breaks onto as many lines as the width needs, and the label is as
/// tall as that paragraph (feature 038).
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
    /// Wrap onto further lines instead of truncating to one. See [`EmphasisedLabel::wrapping`].
    wrap: bool,
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
            wrap: false,
            marker: PhantomData,
        }
    }

    /// Show the whole text, on as many lines as the width needs, instead of one truncated line.
    ///
    /// A word wider than the label breaks inside the word, so nothing is cut or drawn outside the
    /// label (038 FR-004).
    fn wrapping(mut self) -> Self {
        self.wrap = true;
        self
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
    /// The wrapping mode's one paragraph; `None` in the single-line mode.
    wrapped: Option<P>,
    /// The accent the wrapped paragraph's emphasised spans were shaped with. The single-line mode
    /// picks its colours when it draws; a paragraph of spans carries them, so a change of theme
    /// has to shape it again.
    for_accent: Option<Rgb>,
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
            wrapped: None,
            for_accent: None,
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

        if self.wrap {
            if state.wrapped.is_none()
                || state.source != self.content
                || state.source_spans != self.spans
                || state.for_width != available
                || state.for_accent != Some(self.accent)
            {
                let font = self.role.font();
                let accent = style::color(self.accent);
                // The same split, and the same two channels, as the single-line mode below.
                let runs = segments(&self.content, &self.spans);
                let spans: Vec<text::Span<'_, (), Renderer::Font>> = runs
                    .iter()
                    .map(|(piece, emphasised)| {
                        let span = text::Span::new(piece.as_str());
                        if *emphasised {
                            span.font(emphasis_font(font)).color(accent)
                        } else {
                            span
                        }
                    })
                    .collect();
                let paragraph = Renderer::Paragraph::with_spans(CoreText {
                    content: spans.as_slice(),
                    // Bounded by the width alone: the height is what is being asked for.
                    bounds: Size::new(available, f32::INFINITY),
                    size: Pixels(self.role.size()),
                    line_height: self.role.line_height(),
                    font,
                    align_x: text::Alignment::Left,
                    align_y: alignment::Vertical::Top,
                    shaping: text::Shaping::Advanced,
                    // A word wider than the label breaks inside the word (FR-004).
                    wrapping: text::Wrapping::WordOrGlyph,
                });
                let bounds = paragraph.min_bounds();
                state.segments.clear();
                state.wrapped = Some(paragraph);
                state.width = bounds.width;
                state.height = bounds.height;
                state.source = self.content.clone();
                state.source_spans = self.spans.clone();
                state.for_width = available;
                state.for_accent = Some(self.accent);
            }

            return layout::Node::new(limits.resolve(
                Length::Fill,
                Length::Shrink,
                Size::new(state.width, state.height),
            ));
        }

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

        if state.wrapped.is_some()
            || state.source != self.content
            || state.source_spans != self.spans
            || state.for_width != available
        {
            state.wrapped = None;
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

        if let Some(paragraph) = &state.wrapped {
            // The emphasised spans carry their own colour; this one is every other character's.
            renderer.fill_paragraph(paragraph, bounds.position(), style::color(self.base), clip);
            return;
        }

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

    // --- Feature 038: the wrapping label and the two-line row (contracts/picker-row.md §1–2) ---

    use iced::advanced::layout::Limits;
    use micold_core::theme::ColorScheme;

    /// Taller than any row here grows, so the limit never decides a height.
    const TALL: f32 = 4000.0;
    /// Layout arithmetic accumulates over a nested tree; far below a line of text.
    const TOLERANCE: f32 = 0.5;

    fn roles() -> Roles {
        micold_core::tokens::roles(ColorScheme::Light)
    }

    /// The absolute bounds of the node at `path` with `element` laid out `width` wide.
    fn bounds_at(element: Element<'_, ()>, width: f32, path: &[usize]) -> Rectangle {
        let mut element = element;
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &Limits::new(Size::ZERO, Size::new(width, TALL)),
        );
        let mut layout = Layout::new(&node);
        for (depth, &index) in path.iter().enumerate() {
            layout = layout.children().nth(index).unwrap_or_else(|| {
                panic!(
                    "no child {index} at depth {depth} of {path:?}: the row's tree changed shape"
                )
            });
        }
        layout.bounds()
    }

    /// The node of a wrapping label laid out `width` wide, and the width its shaped text needs.
    fn wrapped(content: &str, spans: Vec<Range<usize>>, width: f32) -> (Size, f32) {
        let r = roles();
        let mut element: Element<'_, ()> = EmphasisedLabel::<()>::new(
            content.to_string(),
            spans,
            ROW_ROLE,
            r.on_surface,
            r.primary,
        )
        .wrapping()
        .into();
        let renderer = super::super::test_support::renderer();
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &Limits::new(Size::ZERO, Size::new(width, TALL)),
        );
        let state = tree
            .state
            .downcast_ref::<State<<iced::Renderer as text::Renderer>::Paragraph>>();
        (node.size(), state.width)
    }

    fn two_line_row<'a>(label: &str, details: &str, selected: bool) -> Element<'a, ()> {
        row_element(
            Row::new(label, Vec::new()).details(details, Vec::new()),
            false,
            selected,
            Some(()),
            roles(),
        )
    }

    /// The path of the label, the details and the marker inside a row with details.
    const LABEL: &[usize] = &[0, 1, 1, 0];
    const DETAILS: &[usize] = &[0, 1, 1, 1];
    const MARKER: &[usize] = &[0, 1, 0];

    const LONG_TITLE: &str =
        "#1234 The issue list cuts long titles off so nobody can tell two issues apart";

    /// U31: narrower than its text, the wrapping label is several lines high and no wider than its
    /// bound.
    #[test]
    fn the_wrapping_label_takes_more_lines_inside_its_bound() {
        let line = ROW_ROLE.line_height_dp();
        let (node, text_width) = wrapped(LONG_TITLE, one(0..5).to_vec(), 160.0);
        assert!(
            node.height >= 2.0 * line - TOLERANCE,
            "a label of {} characters is {}dp high in 160dp: one line is {line}dp, so it did not wrap",
            LONG_TITLE.len(),
            node.height,
        );
        assert!(
            node.width <= 160.0 + TOLERANCE && text_width <= 160.0 + TOLERANCE,
            "the label's node is {}dp wide and its text {text_width}dp, in a bound of 160dp",
            node.width,
        );
    }

    /// U32: a word wider than the label breaks inside the word (FR-004).
    #[test]
    fn a_word_wider_than_the_label_breaks_inside_the_word() {
        let line = ROW_ROLE.line_height_dp();
        let title = "a".repeat(256);
        let (node, text_width) = wrapped(&title, Vec::new(), 200.0);
        assert!(
            text_width <= 200.0 + TOLERANCE,
            "256 characters without a space are shaped {text_width}dp wide in a bound of 200dp",
        );
        assert!(
            node.height >= 2.0 * line - TOLERANCE,
            "256 characters without a space are {}dp high in 200dp: they did not break",
            node.height,
        );
    }

    /// U33: the runs the wrapping label shapes are the input, in order, with nothing lost.
    #[test]
    fn the_wrapping_labels_runs_concatenate_to_the_input() {
        let text = "#7 Zażółć gęślą jaźń — naprawić";
        let start = text.find("gęślą").expect("the word is in the text");
        let spans = vec![0..2, start..start + "gęślą".len()];
        let runs = segments(text, &spans);
        let rejoined: String = runs.iter().map(|(piece, _)| piece.as_str()).collect();
        assert_eq!(rejoined, text);
        let emphasised: Vec<&str> = runs
            .iter()
            .filter(|(_, emphasised)| *emphasised)
            .map(|(piece, _)| piece.as_str())
            .collect();
        assert_eq!(emphasised, vec!["#7", "gęślą"]);
    }

    /// U34: a row with details is never shorter than a menu item, and grows when a line wraps.
    #[test]
    fn a_row_with_details_is_at_least_a_menu_item_high_and_grows_when_it_wraps() {
        let short = bounds_at(two_line_row("#7 Short", "octocat", false), 400.0, &[]);
        assert!(
            (short.height - density::MENU_ITEM_BASE).abs() < TOLERANCE,
            "a row of two short lines is {}dp high, not the menu item's {}dp",
            short.height,
            density::MENU_ITEM_BASE,
        );

        let long_title = bounds_at(two_line_row(LONG_TITLE, "octocat", false), 200.0, &[]);
        assert!(
            long_title.height > density::MENU_ITEM_BASE + TOLERANCE,
            "a row whose title wraps is {}dp high: it did not grow",
            long_title.height,
        );

        let labels = "octocat  ·  bug, good first issue, needs triage, area: worktrees, regression";
        let many_labels = bounds_at(two_line_row("#7 Short", labels, false), 200.0, &[]);
        assert!(
            many_labels.height > density::MENU_ITEM_BASE + TOLERANCE,
            "a row whose details wrap is {}dp high: it did not grow",
            many_labels.height,
        );

        // Both lines are inside the row, whatever its height.
        for path in [LABEL, DETAILS] {
            let row = bounds_at(two_line_row(LONG_TITLE, labels, false), 200.0, &[]);
            let line = bounds_at(two_line_row(LONG_TITLE, labels, false), 200.0, path);
            assert!(
                line.y >= row.y - TOLERANCE
                    && line.y + line.height <= row.y + row.height + TOLERANCE
                    && line.x >= row.x - TOLERANCE
                    && line.x + line.width <= row.x + row.width + TOLERANCE,
                "the line at {path:?} is {line:?}, outside its row {row:?}",
            );
        }
    }

    /// U35 (FR-029): a row without details is the fixed-height, single-line row it was.
    #[test]
    fn a_row_without_details_keeps_its_fixed_height_and_single_line() {
        let row = || {
            row_element(
                Row::new(LONG_TITLE, Vec::new()),
                false,
                false,
                Some(()),
                roles(),
            )
        };
        let outer = bounds_at(row(), 200.0, &[]);
        assert!(
            (outer.height - density::MENU_ITEM_BASE).abs() < TOLERANCE,
            "a single-line row is {}dp high, not {}dp",
            outer.height,
            density::MENU_ITEM_BASE,
        );
        // The label is the second child of the content row, as `menu_anatomy` reads it.
        let label = bounds_at(row(), 200.0, &[0, 1]);
        let one_line = ROW_ROLE.size() * 1.3;
        assert!(
            label.height <= one_line + TOLERANCE,
            "a single-line row's label is {}dp high: more than one line of {one_line}dp",
            label.height,
        );
    }

    /// A7 (US1 scenario 7): the picked-row marker sits beside the first line, whatever the row's
    /// height.
    #[test]
    fn a_picked_rows_marker_is_beside_the_first_line_whatever_the_height() {
        let line = ROW_ROLE.line_height_dp();
        let labels = "octocat  ·  bug, good first issue, needs triage, area: worktrees, regression";
        for (label, details, width) in [("#7 Short", "octocat", 400.0), (LONG_TITLE, labels, 200.0)]
        {
            let marker = bounds_at(two_line_row(label, details, true), width, MARKER);
            let first = bounds_at(two_line_row(label, details, true), width, LABEL);
            let marker_centre = marker.y + marker.height / 2.0;
            let first_line_centre = first.y + line / 2.0;
            assert!(
                (marker_centre - first_line_centre).abs() < 1.0,
                "the marker is centred on {marker_centre}dp and the first line of {label:?} on \
                 {first_line_centre}dp",
            );
        }
    }

    // --- Feature 038: the row's tooltip (contracts/picker-row.md §4) ---

    use iced::advanced::Shell;
    use iced::{window, Event, Point, Vector};
    use std::time::Instant;

    /// The window the list and its tooltip are laid out in.
    const WINDOW: Size = Size::new(800.0, 600.0);
    /// The list's width.
    const LIST_WIDTH: f32 = 320.0;
    const MS: Duration = Duration::from_millis(1);

    /// Far more than three lines at the tooltip's width.
    fn long_text() -> String {
        "The list cuts long titles off and gives no way to tell two issues apart. ".repeat(8)
    }

    /// A list of single-line rows, laid out, that takes redraws at chosen instants.
    struct Listed {
        element: Element<'static, ()>,
        tree: Tree,
        node: layout::Node,
        renderer: iced::Renderer,
    }

    impl Listed {
        fn new(rows: Vec<Row>, highlighted: Option<usize>) -> Self {
            let renderer = super::super::test_support::renderer();
            let mut element = menu_element::<()>(rows, highlighted, None, None, None, roles());
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &Limits::new(Size::ZERO, Size::new(LIST_WIDTH, WINDOW.height)),
            );
            Self {
                element,
                tree,
                node,
                renderer,
            }
        }

        /// The same list position by position, holding `rows` now: what a narrowed list is.
        fn rebuilt(mut self, rows: Vec<Row>) -> Self {
            self.element = menu_element::<()>(rows, None, None, None, None, roles());
            self.tree.diff(self.element.as_widget());
            self.node = self.element.as_widget_mut().layout(
                &mut self.tree,
                &self.renderer,
                &Limits::new(Size::ZERO, Size::new(LIST_WIDTH, WINDOW.height)),
            );
            self
        }

        /// A point inside row `index`: every row here is one line, a menu item high.
        fn over_row(index: usize) -> mouse::Cursor {
            mouse::Cursor::Available(Point::new(
                LIST_WIDTH / 2.0,
                spacing::XS + density::MENU_ITEM_BASE * (index as f32 + 0.5),
            ))
        }

        /// A frame at `at` with the cursor where `cursor` says. A redraw carries its instant, so
        /// it is the event whose clock a test controls.
        fn redraw(&mut self, at: Instant, cursor: mouse::Cursor) {
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            self.element.as_widget_mut().update(
                &mut self.tree,
                &Event::Window(window::Event::RedrawRequested(at)),
                Layout::new(&self.node),
                cursor,
                &self.renderer,
                &mut iced::advanced::clipboard::Null,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
        }

        /// The sizes of the panels floating above the list: none while no tooltip is open.
        fn panels(&mut self) -> Vec<Size> {
            let Some(mut floated) = self.element.as_widget_mut().overlay(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &Rectangle::with_size(WINDOW),
                Vector::ZERO,
            ) else {
                return Vec::new();
            };
            let node = floated.as_overlay_mut().layout(&self.renderer, WINDOW);
            let mut found = Vec::new();
            panels_of(&node, &mut found);
            found
        }
    }

    /// A panel is the first node on a path down that is narrower than the window: the groups that
    /// carry it are each as large as the window. Its one child is the surface a person sees; the
    /// node around it adds the margin the panel keeps from the window's edge, drawn as nothing.
    fn panels_of(node: &layout::Node, found: &mut Vec<Size>) {
        if node.size().width < WINDOW.width {
            found.push(node.children().first().unwrap_or(node).size());
            return;
        }
        for child in node.children() {
            panels_of(child, found);
        }
    }

    /// U73 — the row holds exactly the text it was given; an empty text is no tooltip.
    #[test]
    fn a_row_holds_exactly_the_tooltip_text_it_was_given() {
        let row = Row::new("#7 Fix it", Vec::new())
            .details("ana  ·  bug", Vec::new())
            .tooltip("  Two  spaces, kept. ")
            .key(7);
        assert_eq!(
            row.tooltip.as_deref(),
            Some("  Two  spaces, kept. "),
            "the text is the caller's, unchanged"
        );
        assert_eq!(row.key, Some(7));
        assert_eq!(
            Row::new("a", Vec::new()).tooltip("").tooltip,
            None,
            "an empty text is no tooltip"
        );
        let plain = Row::new("a", Vec::new());
        assert_eq!((plain.tooltip, plain.key), (None, None));
    }

    /// U74 — the constants the contract names.
    #[test]
    fn a_rows_tooltip_waits_three_seconds_and_shows_three_lines() {
        assert_eq!(ROW_TOOLTIP_REST, Duration::from_secs(3));
        assert_eq!(ROW_TOOLTIP_LINES, 3);
    }

    /// U74 — a row with a tooltip text is wrapped in a rest-delay tooltip: nothing floats before
    /// `ROW_TOOLTIP_REST` has passed at rest, and one panel floats once it has.
    #[test]
    fn a_row_with_a_tooltip_opens_its_panel_after_the_rest_delay() {
        let mut list = Listed::new(
            vec![
                Row::new("#1 One", Vec::new()).tooltip("About one.").key(1),
                Row::new("#2 Two", Vec::new()).tooltip("About two.").key(2),
            ],
            None,
        );
        let start = Instant::now();
        list.redraw(start, Listed::over_row(0));
        assert!(list.panels().is_empty(), "nothing opens on arrival");
        list.redraw(start + ROW_TOOLTIP_REST - MS, Listed::over_row(0));
        assert!(
            list.panels().is_empty(),
            "nothing opens a millisecond before the delay"
        );
        list.redraw(start + ROW_TOOLTIP_REST, Listed::over_row(0));
        assert_eq!(
            list.panels().len(),
            1,
            "the rested row's panel, and only it, opens at the delay"
        );
    }

    /// U74 — a row without a tooltip text is not wrapped: resting on it floats nothing.
    #[test]
    fn a_row_without_a_tooltip_floats_nothing() {
        let mut list = Listed::new(
            vec![
                Row::new("#1 One", Vec::new()).key(1),
                Row::new("#2 Two", Vec::new()).tooltip("").key(2),
                Row::new("#3 Three", Vec::new())
                    .tooltip("About three.")
                    .key(3),
            ],
            None,
        );
        let start = Instant::now();
        for row in [0, 1] {
            list.redraw(start, Listed::over_row(row));
            list.redraw(start + ROW_TOOLTIP_REST * 2, Listed::over_row(row));
            assert!(
                list.panels().is_empty(),
                "row {row} has no tooltip text, so nothing floats"
            );
        }
    }

    /// U74 — the panel is at most `ROW_TOOLTIP_LINES` lines of `Caption` and its padding tall,
    /// however long the text.
    #[test]
    fn a_rows_panel_is_at_most_three_lines_tall() {
        let mut list = Listed::new(
            vec![Row::new("#1 One", Vec::new()).tooltip(long_text()).key(1)],
            None,
        );
        let start = Instant::now();
        list.redraw(start, Listed::over_row(0));
        list.redraw(start + ROW_TOOLTIP_REST, Listed::over_row(0));
        let panels = list.panels();
        assert_eq!(panels.len(), 1, "the panel opens");
        let limit =
            ROW_TOOLTIP_LINES as f32 * TypeRole::Caption.line_height_dp() + 2.0 * spacing::XS;
        assert!(
            (panels[0].height - limit).abs() <= TOLERANCE,
            "a long text fills exactly three lines and the padding: {} against {limit}",
            panels[0].height
        );
    }

    /// U74 — the key is the tooltip's subject: another row arriving at the same place under a still
    /// cursor closes the panel and waits the whole delay again.
    #[test]
    fn another_key_at_the_same_place_closes_the_panel_and_waits_again() {
        let mut list = Listed::new(
            vec![Row::new("#1 One", Vec::new()).tooltip("About one.").key(1)],
            None,
        );
        let start = Instant::now();
        list.redraw(start, Listed::over_row(0));
        list.redraw(start + ROW_TOOLTIP_REST, Listed::over_row(0));
        assert_eq!(list.panels().len(), 1, "precondition: the panel is open");

        let mut list = list.rebuilt(vec![Row::new("#2 Two", Vec::new())
            .tooltip("About two.")
            .key(2)]);
        assert!(
            list.panels().is_empty(),
            "the panel described the row that left"
        );
        let arrived = start + ROW_TOOLTIP_REST + MS;
        list.redraw(arrived, Listed::over_row(0));
        list.redraw(arrived + ROW_TOOLTIP_REST - MS, Listed::over_row(0));
        assert!(
            list.panels().is_empty(),
            "the new row waits the whole delay"
        );
        list.redraw(arrived + ROW_TOOLTIP_REST, Listed::over_row(0));
        assert_eq!(list.panels().len(), 1, "and then opens its own panel");
    }

    /// U74 — the same key at the same place keeps its panel across a rebuild: a view rebuilt for
    /// another reason does not close what the user is reading.
    #[test]
    fn the_same_key_keeps_its_panel_across_a_rebuild() {
        let row = || Row::new("#1 One", Vec::new()).tooltip("About one.").key(1);
        let mut list = Listed::new(vec![row()], None);
        let start = Instant::now();
        list.redraw(start, Listed::over_row(0));
        list.redraw(start + ROW_TOOLTIP_REST, Listed::over_row(0));
        assert_eq!(list.panels().len(), 1, "precondition: the panel is open");
        let mut list = list.rebuilt(vec![row()]);
        assert_eq!(list.panels().len(), 1, "the panel stays");
    }

    /// U83 — the highlight is not a hover: a highlighted row the cursor is not over floats nothing,
    /// however long it stays highlighted, and neither does the row the cursor left.
    #[test]
    fn a_highlighted_row_without_the_cursor_floats_nothing() {
        let rows = || {
            vec![
                Row::new("#1 One", Vec::new()).tooltip("About one.").key(1),
                Row::new("#2 Two", Vec::new()).tooltip("About two.").key(2),
            ]
        };
        let start = Instant::now();
        let long_after = start + ROW_TOOLTIP_REST * 4;

        let mut list = Listed::new(rows(), Some(1));
        list.redraw(start, mouse::Cursor::Unavailable);
        list.redraw(long_after, mouse::Cursor::Unavailable);
        assert!(
            list.panels().is_empty(),
            "no cursor, so no panel on the highlighted row"
        );

        // The cursor rests on the first row while the second is highlighted: the first row's
        // panel opens, and it is the only one.
        let mut list = Listed::new(rows(), Some(1));
        list.redraw(start, Listed::over_row(0));
        list.redraw(long_after, Listed::over_row(0));
        assert_eq!(
            list.panels().len(),
            1,
            "one panel: the rested row's, none for the highlighted row"
        );
    }

    /// U86 — `{:?}` of a row prints neither its second line nor its tooltip text: for the issue
    /// list those are a reporter and a description, which no log may hold (FR-025).
    #[test]
    fn debug_output_of_a_row_redacts_its_details_and_its_tooltip() {
        let row = Row::new("#7 Fix it", Vec::new())
            .details("a-private-login  ·  bug", Vec::new())
            .tooltip("a-private-sentence")
            .key(7);
        for debug in [format!("{row:?}"), format!("{row:#?}")] {
            assert!(debug.contains("#7 Fix it"), "the label is printed: {debug}");
            assert!(
                !debug.contains("a-private-login") && !debug.contains("a-private-sentence"),
                "neither the details nor the tooltip text is printed: {debug}"
            );
        }
    }
}
