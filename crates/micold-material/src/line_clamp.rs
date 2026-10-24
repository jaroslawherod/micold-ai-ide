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
//!
//! [`LineClamped::when_cut`] hangs an element under the text that is laid out only when the measure
//! cut it: a review comment card's "Show all" under a reply of more than four lines (feature 1270,
//! FR-021, research R7). The decision stays where the cut is made, so the card never guesses it
//! from a character count.

use super::shaped_text::{draw_paragraph, State};
use super::text::TypeRole;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::advanced::widget::{tree, Operation, Tree};
use iced::advanced::{mouse, renderer, Clipboard, Shell, Widget};
use iced::{alignment, Element, Event, Font, Length, Pixels, Point, Rectangle, Size};
use micold_core::tokens::{spacing, Rgb};
use micold_core::tooltip::clamp_to_lines;

/// What a shaped paragraph was shaped from: the text, the width, the line limit, the role (its
/// size, font and line height) and the lead, so a tree slot reused at another role shapes again.
type Key = (String, f32, usize, TypeRole, Option<Lead>);

/// Words drawn before the text on its first line, at their own role and in their own colour: an
/// Overview card's "NOW" before its activity (BUG-1298 FR-025). They are never cut.
#[derive(Debug, Clone, PartialEq)]
struct Lead {
    words: String,
    role: TypeRole,
    tint: Rgb,
}

/// A wrapping label at a type role, cut to at most `max_lines` lines.
pub struct LineClamped<'a, M> {
    content: String,
    role: TypeRole,
    max_lines: usize,
    /// `None` draws in the surrounding text colour, as a tooltip's label does.
    tint: Option<Rgb>,
    /// `Shrink` is as wide as the text, as a tooltip's label is; `Fill` spans the row it is in.
    width: Length,
    lead: Option<Lead>,
    /// Laid out under the text only when the text was cut ([`Self::when_cut`]).
    when_cut: Option<Element<'a, M>>,
}

impl<'a, M> LineClamped<'a, M> {
    /// `content` at `role`, in the surrounding text colour, on at most `max_lines` lines.
    pub fn new(content: impl Into<String>, role: TypeRole, max_lines: usize) -> Self {
        Self {
            content: content.into(),
            role,
            max_lines,
            tint: None,
            width: Length::Shrink,
            lead: None,
            when_cut: None,
        }
    }

    /// Drawn in `color` rather than the surrounding text colour.
    pub fn tint(mut self, color: Rgb) -> Self {
        self.tint = Some(color);
        self
    }

    /// Laid out `width` wide: `Fill` spans the row, as the one-line label it replaces did.
    pub fn width(mut self, width: Length) -> Self {
        self.width = width;
        self
    }

    /// Lead the first line with `words` at `role` in `tint`, then two spaces, the text flowing on
    /// after them and wrapping back to the left edge (BUG-1298 FR-025, the mockup's `::before`).
    pub fn lead(mut self, words: impl Into<String>, role: TypeRole, tint: Rgb) -> Self {
        self.lead = Some(Lead {
            words: words.into(),
            role,
            tint,
        });
        self
    }

    /// Lay `element` out under the text, `spacing::XS` below it, only when the text was cut to fit
    /// its lines; a text shown whole gives it no room and it is not drawn (feature 1270, R7).
    pub fn when_cut(mut self, element: impl Into<Element<'a, M>>) -> Self {
        self.when_cut = Some(element.into());
        self
    }
}

/// Whether the element under the text was left out: a whole text gives it a zero-area node.
fn left_out(layout: Layout<'_>) -> bool {
    let b = layout.bounds();
    b.width <= 0.0 || b.height <= 0.0
}

impl<Message> Widget<Message, iced::Theme, iced::Renderer> for LineClamped<'_, Message> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<<iced::Renderer as text::Renderer>::Paragraph, Key>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<<iced::Renderer as text::Renderer>::Paragraph, Key>::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.when_cut.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        match &self.when_cut {
            Some(element) => tree.diff_children(std::slice::from_ref(element)),
            None => tree.children.clear(),
        }
    }

    fn size(&self) -> Size<Length> {
        Size::new(self.width, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree
            .state
            .downcast_mut::<State<<iced::Renderer as text::Renderer>::Paragraph, Key>>();
        let available = limits.max().width;

        let key = (
            self.content.clone(),
            available,
            self.max_lines,
            self.role,
            self.lead.clone(),
        );
        if !state.holds(&key) {
            // Shaped as it will be drawn: at the width on offer, breaking on words and, where a
            // word is wider than the line (a path, an address), on glyphs.
            let template: CoreText<(), Font> = CoreText {
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
            let lead = self
                .lead
                .as_ref()
                .map(|lead| (format!("{}  ", lead.words), lead));
            let shape = |candidate: &str| match &lead {
                None => <iced::Renderer as text::Renderer>::Paragraph::with_text(
                    template.with_content(candidate),
                ),
                Some((words, lead)) => {
                    let spans: [text::Span<'_, (), Font>; 2] = [
                        text::Span::new(words.as_str())
                            .size(lead.role.size())
                            .font(lead.role.font())
                            .color(super::style::color(lead.tint)),
                        text::Span::new(candidate),
                    ];
                    <iced::Renderer as text::Renderer>::Paragraph::with_spans(
                        template.with_content(&spans[..]),
                    )
                }
            };
            let lines_of = |candidate: &str| {
                let height = shape(candidate).min_bounds().height;
                // A paragraph is a whole number of lines tall; rounding takes the float's dust off.
                (height / line).round() as usize
            };
            let fitted = clamp_to_lines(&self.content, self.max_lines, lines_of).into_owned();
            let paragraph = shape(&fitted);
            state.store(paragraph, fitted, key);
        }

        let text = state.paragraph.min_bounds();
        let cut = state.fitted != self.content;
        let Some(element) = self.when_cut.as_mut() else {
            return layout::Node::new(limits.resolve(self.width, Length::Shrink, text));
        };
        let below = if cut {
            let loose = layout::Limits::new(Size::ZERO, Size::new(available, f32::INFINITY));
            element
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, &loose)
                .move_to(Point::new(0.0, text.height + spacing::XS))
        } else {
            layout::Node::new(Size::ZERO)
        };
        let content = if cut {
            let b = below.bounds();
            Size::new(text.width.max(b.width), b.y + b.height)
        } else {
            text
        };
        layout::Node::with_children(
            limits.resolve(self.width, Length::Shrink, content),
            vec![below],
        )
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        if let (Some(element), Some(below)) = (self.when_cut.as_mut(), layout.children().next()) {
            if !left_out(below) {
                element
                    .as_widget_mut()
                    .operate(&mut tree.children[0], below, renderer, operation);
            }
        }
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        if let (Some(element), Some(below)) = (self.when_cut.as_mut(), layout.children().next()) {
            if !left_out(below) {
                element.as_widget_mut().update(
                    &mut tree.children[0],
                    event,
                    below,
                    cursor,
                    renderer,
                    clipboard,
                    shell,
                    viewport,
                );
            }
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
        match (self.when_cut.as_ref(), layout.children().next()) {
            (Some(element), Some(below)) if !left_out(below) => element
                .as_widget()
                .mouse_interaction(&tree.children[0], below, cursor, viewport, renderer),
            _ => mouse::Interaction::default(),
        }
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
        let state = tree
            .state
            .downcast_ref::<State<<iced::Renderer as text::Renderer>::Paragraph, Key>>();
        // A measurement off by a fraction must not paint a fourth line below the panel.
        draw_paragraph(
            renderer,
            &state.paragraph,
            layout,
            viewport,
            self.tint.map_or(style.text_color, super::style::color),
        );
        if let (Some(element), Some(below)) = (self.when_cut.as_ref(), layout.children().next()) {
            if !left_out(below) {
                element.as_widget().draw(
                    &tree.children[0],
                    renderer,
                    theme,
                    style,
                    below,
                    cursor,
                    viewport,
                );
            }
        }
    }
}

impl<'a, Message: 'a> From<LineClamped<'a, Message>> for Element<'a, Message> {
    fn from(label: LineClamped<'a, Message>) -> Self {
        Element::new(label)
    }
}

/// Shared with the client's card tests, which measure a title the same way.
/// `content` at `role` laid out `width` wide on at most `max_lines` lines: what it holds, and
/// its size.
#[cfg(any(test, feature = "test-support"))]
pub fn laid_out_at(
    content: &str,
    role: TypeRole,
    max_lines: usize,
    width: f32,
) -> (String, iced::Size) {
    let renderer = crate::test_support::renderer();
    let mut label = LineClamped::<()>::new(content, role, max_lines);
    let widget: &mut dyn iced::advanced::Widget<(), iced::Theme, iced::Renderer> = &mut label;
    let mut tree = iced::advanced::widget::Tree {
        tag: widget.tag(),
        state: widget.state(),
        children: Vec::new(),
    };
    let limits = iced::advanced::layout::Limits::new(
        iced::Size::ZERO,
        iced::Size::new(width, f32::INFINITY),
    );
    let node = widget.layout(&mut tree, &renderer, &limits);
    let state = tree
        .state
        .downcast_ref::<State<<iced::Renderer as iced::advanced::text::Renderer>::Paragraph, Key>>(
        );
    (state.fitted.clone(), node.size())
}

#[cfg(test)]
pub(super) mod tests {
    //! The line limit as the tooltip shows it (U68; contracts/rest-tooltip.md §5). The rule is
    //! `micold_core`'s and is held there with a fake measure; what is held here is the real one —
    //! the shipped face at the tooltip's width — and the panel that results.

    use super::*;
    use crate as material;
    use crate::{test_support, TOOLTIP_MAX_WIDTH};
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

    /// The room a line has inside the narrowest card: the card less its body's padding.
    const NARROWEST_LABEL: f32 = NARROWEST_CARD - 2.0 * spacing::MD;

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

    /// Feature 1121 review A M1 F1: a tree slot laid out at one role and then at another shapes
    /// the text again, at the second role's size and line height.
    #[test]
    fn a_slot_reused_at_another_role_is_shaped_again() {
        let renderer = test_support::renderer();
        let limits = layout::Limits::new(Size::ZERO, Size::new(NARROWEST_CARD, f32::INFINITY));
        let text = long();
        let mut title = LineClamped::<()>::new(&text, TypeRole::CardTitle, 3);
        let mut body = LineClamped::<()>::new(&text, TypeRole::Body, 3);
        let title: &mut dyn Widget<(), iced::Theme, iced::Renderer> = &mut title;
        let body: &mut dyn Widget<(), iced::Theme, iced::Renderer> = &mut body;
        let mut tree = Tree {
            tag: title.tag(),
            state: title.state(),
            children: Vec::new(),
        };
        title.layout(&mut tree, &renderer, &limits);
        let reused = body.layout(&mut tree, &renderer, &limits).size();
        let (_, fresh) = laid_out_at(&text, TypeRole::Body, 3, NARROWEST_CARD);
        assert_eq!(reused, fresh);
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

    /// Feature 1121 T001, US1 sc.2: a title needing five lines in a 280 px card takes three, the third
    /// ending in an ellipsis.
    #[test]
    fn a_five_line_title_at_280_takes_three_lines_ending_in_an_ellipsis() {
        let role = TypeRole::CardTitle;
        let five =
            "Overview cards show the full title and the activity without truncation ".repeat(3);
        let (whole, size) = laid_out_at(&five, role, 10, NARROWEST_LABEL);
        assert!(
            size.height >= 5.0 * role.line_height_dp() && whole == five,
            "precondition: the text needs five lines or more: {size:?}",
        );
        let (fitted, size) = laid_out_at(&five, role, 3, NARROWEST_LABEL);
        assert_eq!(size.height, 3.0 * role.line_height_dp(), "{fitted:?}");
        assert!(fitted.ends_with('…'), "{fitted:?}");
    }

    /// Feature 1121 T001, FR-007: an unbroken run of 120 characters stays within a 280 px card and ends
    /// in an ellipsis.
    #[test]
    fn an_unbroken_run_stays_within_the_width_and_ends_in_an_ellipsis() {
        let run = "m".repeat(120);
        for (role, lines) in [(TypeRole::CardTitle, 3), (TypeRole::Body, 2)] {
            let (fitted, size) = laid_out_at(&run, role, lines, NARROWEST_LABEL);
            assert!(size.width <= NARROWEST_LABEL, "{role:?}: {size:?}");
            assert_eq!(
                size.height,
                lines as f32 * role.line_height_dp(),
                "{role:?}"
            );
            assert!(fitted.ends_with('…'), "{role:?}: {fitted:?}");
        }
    }

    /// Feature 1270 T046 (FR-021, research R7): `when_cut`'s element is laid out under the text
    /// when the measure cut it, and takes no height when the text fits whole.
    #[test]
    fn the_when_cut_element_is_laid_out_under_a_cut_text_and_takes_no_height_otherwise() {
        let room = Size::new(NARROWEST_LABEL, 2_000.0);
        let line = TypeRole::Body.line_height_dp();
        let after = || iced::widget::Space::new().width(40.0).height(20.0);

        let cut = test_support::laid_out(
            LineClamped::<()>::new(long(), TypeRole::Body, 4)
                .when_cut(after())
                .into(),
            room,
        );
        let element = cut
            .children()
            .first()
            .expect("the element under the text")
            .bounds();
        assert_eq!(element.size(), Size::new(40.0, 20.0));
        assert!(element.y >= 4.0 * line, "under the four lines: {element:?}");
        assert_eq!(cut.size().height, element.y + element.height, "{cut:?}");

        let plain = test_support::size_of(
            LineClamped::<()>::new("Short", TypeRole::Body, 4).into(),
            room,
        );
        let whole = test_support::size_of(
            LineClamped::<()>::new("Short", TypeRole::Body, 4)
                .when_cut(after())
                .into(),
            room,
        );
        assert_eq!(whole, plain, "a whole text gives the element no room");
        assert_eq!(whole.height, line);
    }

    /// Feature 1270 T046: the element is drawn only under a cut text.
    #[test]
    fn the_when_cut_element_is_drawn_only_under_a_cut_text() {
        let roles = test_support::roles();
        let drawn = |content: String| {
            let mut mounted = test_support::Mounted::new(
                LineClamped::<String>::new(content, TypeRole::Body, 4)
                    .width(Length::Fixed(NARROWEST_LABEL))
                    .when_cut(material::Text::new("Show all", TypeRole::Label, roles)),
            );
            mounted
                .painted_text()
                .into_iter()
                .any(|p| p.content == "Show all")
        };
        assert!(drawn(long()), "a cut text offers the element");
        assert!(!drawn("Short".to_owned()), "a whole text does not");
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
