//! `LineClamped` — a wrapping label of at most so many lines, ending in an ellipsis when cut
//! (feature 038, FR-021).
//!
//! [`Ellipsized`](super::Ellipsized) answers "one line, as wide as there is room". This answers the
//! other question a tooltip asks: the width is fixed by the panel's ceiling, the text wraps, and
//! what is limited is how many lines it may take. An issue's description can be a page long; its
//! tooltip is three lines.
//!
//! How many lines a text takes is only known once it is shaped at the width it will be drawn at,
//! so this measures at layout time, as `Ellipsized` does. The rule for where to cut — the longest
//! prefix that fits with its ellipsis, backed up to a whole word — is
//! `micold_core::tooltip::clamp_to_lines`, tested there without a renderer; this hands it the
//! measure.
//!
//! Not `pub`: it is the label of `material::Tooltip::max_lines`, a review row's lines, and an
//! overview card's title (three lines) and activity (two), tinted (feature 1121, R2).

use std::marker::PhantomData;

use super::shaped_text::{draw_paragraph, State};
use super::text::TypeRole;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Widget};
use iced::{alignment, Element, Font, Length, Pixels, Rectangle, Size};
use micold_core::tokens::Rgb;
use micold_core::tooltip::clamp_to_lines;

/// A wrapping label at a type role, cut to at most `max_lines` lines.
pub(crate) struct LineClamped<M> {
    content: String,
    role: TypeRole,
    max_lines: usize,
    /// `None` draws in the surrounding text colour, as a tooltip's label does.
    tint: Option<Rgb>,
    /// `Shrink` is as wide as the text, as a tooltip's label is; `Fill` spans the row it is in.
    width: Length,
    marker: PhantomData<M>,
}

impl<M> LineClamped<M> {
    /// `content` at `role`, in the surrounding text colour, on at most `max_lines` lines.
    pub(crate) fn new(content: impl Into<String>, role: TypeRole, max_lines: usize) -> Self {
        Self {
            content: content.into(),
            role,
            max_lines,
            tint: None,
            width: Length::Shrink,
            marker: PhantomData,
        }
    }

    /// Drawn in `color` rather than the surrounding text colour.
    pub(crate) fn tint(mut self, color: Rgb) -> Self {
        self.tint = Some(color);
        self
    }

    /// Laid out `width` wide: `Fill` spans the row, as the one-line label it replaces did.
    pub(crate) fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for LineClamped<Message>
where
    Renderer: text::Renderer<Font = Font>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph, (String, f32, usize)>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph, (String, f32, usize)>::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        _renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree
            .state
            .downcast_mut::<State<Renderer::Paragraph, (String, f32, usize)>>();
        let available = limits.max().width;

        let key = (self.content.clone(), available, self.max_lines);
        if !state.holds(&key) {
            // Shaped as it will be drawn: at the width on offer, breaking on words and, where a
            // word is wider than the line (a path, an address), on glyphs.
            let template: CoreText<(), Renderer::Font> = CoreText {
                content: (),
                bounds: Size::new(available, f32::INFINITY),
                size: Pixels(self.role.size()),
                line_height: self.role.line_height(),
                font: self.role.font(),
                align_x: text::Alignment::Left,
                align_y: alignment::Vertical::Top,
                shaping: text::Shaping::Advanced,
                wrapping: text::Wrapping::WordOrGlyph,
            };
            let line = self.role.line_height_dp();
            let lines_of = |candidate: &str| {
                let height = Renderer::Paragraph::with_text(template.with_content(candidate))
                    .min_bounds()
                    .height;
                // A paragraph is a whole number of lines tall; rounding takes the float's dust off.
                (height / line).round() as usize
            };
            let fitted = clamp_to_lines(&self.content, self.max_lines, lines_of).into_owned();
            let paragraph = Renderer::Paragraph::with_text(template.with_content(&fitted));
            state.store(paragraph, fitted, key);
        }

        layout::Node::new(limits.resolve(self.width, Length::Shrink, state.paragraph.min_bounds()))
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        _theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let state = tree
            .state
            .downcast_ref::<State<Renderer::Paragraph, (String, f32, usize)>>();
        // A measurement off by a fraction must not paint a fourth line below the panel.
        draw_paragraph(
            renderer,
            &state.paragraph,
            layout,
            viewport,
            self.tint.map_or(style.text_color, super::style::color),
        );
    }
}

impl<'a, Message, Theme, Renderer> From<LineClamped<Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer<Font = Font> + 'a,
{
    fn from(label: LineClamped<Message>) -> Self {
        Element::new(label)
    }
}

#[cfg(test)]
/// `content` at `role` laid out `width` wide on at most `max_lines` lines: what it holds, and
/// its size.
pub(super) fn laid_out_at(
    content: &str,
    role: TypeRole,
    max_lines: usize,
    width: f32,
) -> (String, Size) {
    let renderer = super::test_support::renderer();
    let mut label = LineClamped::<()>::new(content, role, max_lines);
    let widget: &mut dyn Widget<(), iced::Theme, iced::Renderer> = &mut label;
    let mut tree = Tree {
        tag: widget.tag(),
        state: widget.state(),
        children: Vec::new(),
    };
    let limits = layout::Limits::new(Size::ZERO, Size::new(width, f32::INFINITY));
    let node = widget.layout(&mut tree, &renderer, &limits);
    let state = tree
        .state
        .downcast_ref::<State<<iced::Renderer as text::Renderer>::Paragraph, (String, f32, usize)>>(
        );
    (state.fitted.clone(), node.size())
}

#[cfg(test)]
mod tests {
    //! The line limit as the tooltip shows it (U68; contracts/rest-tooltip.md §5). The rule is
    //! `micold_core`'s and is held there with a fake measure; what is held here is the real one —
    //! the shipped face at the tooltip's width — and the panel that results.

    use super::*;
    use crate::ui::material::{self, test_support, TOOLTIP_MAX_WIDTH};
    use iced::advanced::Shell;
    use iced::{Event, Point, Vector};
    use micold_core::theme::ColorScheme;
    use micold_core::tokens::{self, spacing, Rgb};

    const WINDOW: Size = Size::new(800.0, 600.0);

    /// Far more than three lines at the tooltip's width, in words.
    fn long() -> String {
        "The list cuts long titles off and gives no way to tell two issues apart. ".repeat(12)
    }

    /// The width a tooltip's label is laid out at: the ceiling less the panel's padding.
    fn label_width() -> f32 {
        TOOLTIP_MAX_WIDTH - 2.0 * spacing::XS
    }

    /// A label laid out at the tooltip's width: what it holds, and how tall it is.
    fn laid_out(content: &str, max_lines: usize) -> (String, f32) {
        let (fitted, size) = laid_out_at(content, TypeRole::Caption, max_lines, label_width());
        (fitted, size.height)
    }

    /// The height of the panel a tooltip floats once hovered, for `label` as `build` shapes it.
    fn panel_height(
        label: &str,
        build: impl Fn(material::Tooltip<'static, ()>) -> material::Tooltip<'static, ()>,
    ) -> f32 {
        let renderer = test_support::renderer();
        let roles = tokens::roles(ColorScheme::Light);
        let trigger = iced::widget::Space::new().width(40.0).height(20.0);
        let mut element: Element<'static, ()> =
            build(material::Tooltip::new(trigger, label, roles)).into();
        let mut tree = Tree::new(&element);
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, WINDOW),
        );
        let node = node.move_to(Point::new(300.0, 100.0));
        let over = Point::new(310.0, 110.0);

        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Mouse(mouse::Event::CursorMoved { position: over }),
            Layout::new(&node),
            mouse::Cursor::Available(over),
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );

        let mut panel = element
            .as_widget_mut()
            .overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &Rectangle::with_size(WINDOW),
                Vector::ZERO,
            )
            .expect("a hovered tooltip floats its panel");
        // The tooltip floats a group, as wide as the window: the panel is the one thing in it.
        let group = panel.as_overlay_mut().layout(&renderer, WINDOW);
        let panel = group.children().first().expect("the panel in its group");
        panel.size().height
    }

    /// The card widths feature 1121 names (FR-007).
    const NARROWEST_CARD: f32 = 280.0;

    /// Feature 1121 T001: a tinted label draws in its tint, not the surrounding text colour.
    #[test]
    fn a_tinted_label_draws_in_its_tint() {
        let tint = Rgb::hex(0x128034);
        let mut mounted = test_support::Mounted::new(
            LineClamped::<String>::new("Overview cards", TypeRole::CardTitle, 3).tint(tint),
        );
        let painted = mounted.painted_text();
        assert_eq!(painted.len(), 1, "{painted:?}");
        assert_eq!(painted[0].color, material::style::color(tint));
    }

    /// Feature 1121 T001, R2: a line is as tall as `Ellipsized::at_role_spaced` makes it at the
    /// same role, so a clamped title sits in the card's column as the ellipsized one did.
    #[test]
    fn a_line_is_as_tall_as_a_spaced_ellipsized_line_at_the_same_role() {
        for role in [TypeRole::CardTitle, TypeRole::Body] {
            let (_, clamped) = laid_out_at("Overview", role, 3, NARROWEST_CARD);
            let spaced = test_support::size_of(
                material::Ellipsized::<()>::at_role_spaced("Overview", role, Rgb::hex(0)).into(),
                Size::new(NARROWEST_CARD, 400.0),
            );
            assert_eq!(clamped.height, spaced.height, "{role:?}");
        }
    }

    /// Feature 1121 T006: a `Fill` label spans its row as the ellipsized line did, so the card's
    /// title and activity keep their width in the layout; the default is as wide as its text.
    #[test]
    fn a_filling_label_spans_the_row_and_a_default_one_its_text() {
        let room = Size::new(NARROWEST_CARD, 400.0);
        let fill = test_support::size_of(
            LineClamped::<()>::new("Overview", TypeRole::CardTitle, 3)
                .width(Length::Fill)
                .into(),
            room,
        );
        let shrink = test_support::size_of(
            LineClamped::<()>::new("Overview", TypeRole::CardTitle, 3).into(),
            room,
        );
        assert_eq!(fill.width, NARROWEST_CARD);
        assert!(shrink.width < NARROWEST_CARD, "{shrink:?}");
    }

    /// Feature 1121 T001, US1 sc.2: a title needing five lines at 280 px takes three, the third
    /// ending in an ellipsis.
    #[test]
    fn a_five_line_title_at_280_takes_three_lines_ending_in_an_ellipsis() {
        let role = TypeRole::CardTitle;
        let five =
            "Overview cards show the full title and the activity without truncation ".repeat(3);
        let (whole, size) = laid_out_at(&five, role, 10, NARROWEST_CARD);
        assert!(
            size.height >= 5.0 * role.line_height_dp() && whole == five,
            "precondition: the text needs five lines or more: {size:?}",
        );
        let (fitted, size) = laid_out_at(&five, role, 3, NARROWEST_CARD);
        assert_eq!(size.height, 3.0 * role.line_height_dp(), "{fitted:?}");
        assert!(fitted.ends_with('…'), "{fitted:?}");
    }

    /// Feature 1121 T001, FR-007: an unbroken run of 120 characters stays within 280 px and ends
    /// in an ellipsis.
    #[test]
    fn an_unbroken_run_stays_within_the_width_and_ends_in_an_ellipsis() {
        let run = "m".repeat(120);
        for (role, lines) in [(TypeRole::CardTitle, 3), (TypeRole::Body, 2)] {
            let (fitted, size) = laid_out_at(&run, role, lines, NARROWEST_CARD);
            assert!(size.width <= NARROWEST_CARD, "{role:?}: {size:?}");
            assert_eq!(
                size.height,
                lines as f32 * role.line_height_dp(),
                "{role:?}"
            );
            assert!(fitted.ends_with('…'), "{role:?}: {fitted:?}");
        }
    }

    #[test]
    fn a_long_text_is_cut_to_three_lines_and_ends_in_an_ellipsis() {
        let (fitted, height) = laid_out(&long(), 3);

        assert!(fitted.ends_with('…'), "cut, so marked: {fitted:?}");
        assert_eq!(
            height,
            3.0 * TypeRole::Caption.line_height_dp(),
            "three `Caption` lines, and all three used",
        );
        let kept = fitted.trim_end_matches('…');
        assert!(
            long().starts_with(kept) && long()[kept.len()..].starts_with(' '),
            "the cut falls after a whole word: {fitted:?}",
        );
    }

    #[test]
    fn a_text_that_fits_is_shown_whole() {
        let short = "The list cuts long titles off.";
        let (fitted, height) = laid_out(short, 3);

        assert_eq!(fitted, short);
        assert_eq!(height, TypeRole::Caption.line_height_dp(), "one line");
    }

    /// A word wider than the line breaks on glyphs, so a text without a space is cut too.
    #[test]
    fn a_text_without_spaces_is_cut_as_well() {
        let path = "feat-abc-123_a-long-branch-name/".repeat(20);
        let (fitted, height) = laid_out(&path, 3);

        assert!(fitted.ends_with('…'));
        assert!(height <= 3.0 * TypeRole::Caption.line_height_dp());
    }

    #[test]
    fn the_opened_panel_is_at_most_three_lines_and_its_padding_tall() {
        let line = TypeRole::Caption.line_height_dp();
        let one_line = panel_height("Short", |tip| tip.max_lines(3));
        let clamped = panel_height(&long(), |tip| tip.max_lines(3));
        let unclamped = panel_height(&long(), |tip| tip);

        assert_eq!(
            clamped - one_line,
            2.0 * line,
            "the same padding around three lines as around one",
        );
        assert!(
            unclamped > clamped,
            "precondition: the text takes more than three lines when nothing limits it",
        );
    }

    #[test]
    fn the_line_limit_leaves_a_short_tooltip_as_it_was() {
        assert_eq!(
            panel_height("Settings", |tip| tip.max_lines(3)),
            panel_height("Settings", |tip| tip),
        );
    }
}
