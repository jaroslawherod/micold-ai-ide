//! `Ellipsized` — a one-line label that ends in an ellipsis when it will not fit.
//!
//! A sidebar row is a fixed width with a name in the middle and controls on either side. A name
//! longer than the space left over has to give way, and the rendering stack offers only two
//! built-in answers: wrap onto a second line, which breaks the row rhythm, or
//! `Wrapping::None`, which keeps one line but does *not* clip — the paragraph is measured at its
//! full natural width and drawn straight over whatever sits to its right. That is what put a
//! session name on top of its own close button.
//!
//! So this measures. At layout time the renderer can shape text, so the label is binary-searched
//! down to the longest prefix that fits alongside an ellipsis, and the result is clipped to the
//! widget's own bounds as a second line of defence — a name can never paint over a control again,
//! even if a measurement is off by a fraction.

use std::marker::PhantomData;

use super::text::TypeRole;
use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::advanced::widget::{tree, Tree};
use iced::advanced::{mouse, renderer, Widget};
use iced::{alignment, Element, Length, Pixels, Rectangle, Size};
use micold_core::tokens::Rgb;

/// The character standing in for what did not fit. One glyph rather than three dots: it is what
/// the typographic convention actually is, and it costs a third of the width.
const ELLIPSIS: char = '…';

/// A single-line label that truncates rather than overflowing. Builder form (Principle VIII):
/// `Ellipsized::new(label, size, tint).into()`.
pub struct Ellipsized<M> {
    content: String,
    size: f32,
    color: Rgb,
    marker: PhantomData<M>,
}

impl<M> Ellipsized<M> {
    /// A label drawn at `size` in `color`, shortened with an ellipsis if it does not fit.
    ///
    /// The size is a number because the sidebar's tree passes its own reduced scale through. A call
    /// site that just wants "body text" should use [`Self::at_role`] instead and let the role supply
    /// the number.
    pub fn new(content: impl Into<String>, size: f32, color: Rgb) -> Self {
        Self {
            content: content.into(),
            size,
            color,
            marker: PhantomData,
        }
    }

    /// A label at a type `role` in `color`, shortened with an ellipsis if it does not fit.
    ///
    /// Added by feature 020. The component showcase needs to pose this component, and could not: a
    /// call site outside the library may not name a text size (the boundary gate closes that at zero),
    /// but [`Self::new`]'s only constructor demanded one — so the one component whose entire job is
    /// text was the one component a feature module could not construct at an ordinary role. That is
    /// the kind of gap a gallery finds, and FR-021's answer is to fix the library rather than let the
    /// gallery work around it.
    ///
    /// Additive: no existing call site changes and no appearance moves, because a role's size is the
    /// same number the scale was already handing out.
    pub fn at_role(content: impl Into<String>, role: TypeRole, color: Rgb) -> Self {
        Self::new(content, role.size(), color)
    }
}

/// The shaped label, plus what it was shaped for.
///
/// Shaping is the expensive part and the search does it several times over, so the inputs that
/// decide the outcome are remembered: a re-render at the same width with the same name reuses the
/// paragraph rather than measuring the name again on every frame.
struct State<P> {
    paragraph: P,
    source: String,
    for_width: f32,
    for_size: f32,
}

impl<P: Default> Default for State<P> {
    fn default() -> Self {
        Self {
            paragraph: P::default(),
            source: String::new(),
            // NaN compares unequal to everything, including itself, so the first layout always
            // measures rather than trusting an uninitialised width.
            for_width: f32::NAN,
            for_size: f32::NAN,
        }
    }
}

impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for Ellipsized<Message>
where
    Renderer: text::Renderer,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State<Renderer::Paragraph>>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::<Renderer::Paragraph>::default())
    }

    fn size(&self) -> Size<Length> {
        // Fill, so the label claims whatever the row's controls leave it — the width the
        // truncation is measured against.
        Size::new(Length::Fill, Length::Shrink)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let state = tree.state.downcast_mut::<State<Renderer::Paragraph>>();
        let available = limits.max().width;

        // Shaping is unbounded here on purpose: the question is how wide the text *wants* to be,
        // which is exactly what a constrained measurement would hide.
        let template: CoreText<(), Renderer::Font> = CoreText {
            content: (),
            bounds: Size::INFINITE,
            size: Pixels(self.size),
            line_height: text::LineHeight::default(),
            font: renderer.default_font(),
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        };

        let stale = state.source != self.content
            || state.for_width != available
            || state.for_size != self.size;
        if stale {
            let measure = |candidate: &str| {
                Renderer::Paragraph::with_text(template.with_content(candidate))
                    .min_bounds()
                    .width
            };
            let fitted = fit(&self.content, available, measure);
            state.paragraph = Renderer::Paragraph::with_text(template.with_content(&fitted));
            state.source = self.content.clone();
            state.for_width = available;
            state.for_size = self.size;
        }

        layout::Node::new(limits.resolve(
            Length::Fill,
            Length::Shrink,
            state.paragraph.min_bounds(),
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
        let anchor = bounds.anchor(
            state.paragraph.min_bounds(),
            state.paragraph.align_x(),
            state.paragraph.align_y(),
        );
        // Clipped to the label's own bounds, not the viewport: truncation should already have made
        // this impossible, and a belt that only holds when the braces do is not a belt.
        let clip = bounds.intersection(viewport).unwrap_or(bounds);
        renderer.fill_paragraph(
            &state.paragraph,
            anchor,
            super::style::color(self.color),
            clip,
        );
    }
}

/// The longest prefix of `content` that fits in `available` once an ellipsis is added.
///
/// Binary search rather than a character-width estimate, because the font is proportional: an
/// estimate that runs long puts the text back over the controls, and one that runs short throws
/// away room that was there. `measure` shapes a candidate and reports its width.
fn fit(content: &str, available: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(content) <= available {
        return content.to_string();
    }

    // Byte offsets of every character boundary, plus the end — the only places a string may be cut.
    let mut cuts: Vec<usize> = content.char_indices().map(|(at, _)| at).collect();
    cuts.push(content.len());

    let shortened = |chars: usize| {
        let mut out = content[..cuts[chars]].trim_end().to_string();
        out.push(ELLIPSIS);
        out
    };

    // Largest `chars` that still fits. Monotone — adding characters never makes the label narrower
    // — so the search is sound; the full string is excluded because it is already known not to fit.
    let (mut low, mut high) = (0usize, cuts.len() - 1);
    while low < high {
        let mid = low + (high - low).div_ceil(2);
        if measure(&shortened(mid)) <= available {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    shortened(low)
}

impl<'a, Message, Theme, Renderer> From<Ellipsized<Message>>
    for Element<'a, Message, Theme, Renderer>
where
    Message: 'a,
    Theme: 'a,
    Renderer: text::Renderer + 'a,
{
    fn from(label: Ellipsized<Message>) -> Self {
        Element::new(label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in for the renderer's shaper: one unit per character, so the arithmetic is
    /// checkable by eye. Proportional fonts differ in the numbers, never in the rule.
    fn monospace(text: &str) -> f32 {
        text.chars().count() as f32
    }

    #[test]
    fn a_label_that_fits_is_left_alone() {
        assert_eq!(fit("session", 10.0, monospace), "session");
        // Exactly filling the space is fitting, not overflowing.
        assert_eq!(fit("session", 7.0, monospace), "session");
    }

    #[test]
    fn a_label_that_does_not_fit_ends_in_an_ellipsis() {
        let out = fit("understand check in circle", 10.0, monospace);
        assert!(out.ends_with(ELLIPSIS), "expected an ellipsis, got {out:?}");
        assert!(
            monospace(&out) <= 10.0,
            "{out:?} is {} wide, over the 10 available",
            monospace(&out)
        );
    }

    /// The point of the search: give back as much of the name as will actually fit. A result far
    /// short of the space available would be correct and useless.
    #[test]
    fn truncation_keeps_as_much_as_it_can() {
        let out = fit("understand check in circle", 10.0, monospace);
        assert_eq!(monospace(&out), 10.0, "{out:?} left room unused");
    }

    /// Cutting mid-character would panic on a byte index; cutting mid-word is merely ugly. Both
    /// are avoided by cutting only at character boundaries.
    #[test]
    fn multibyte_text_is_cut_at_character_boundaries() {
        // Each of these is multiple bytes but one character.
        let out = fit("日本語のセッション名", 5.0, monospace);
        assert!(out.ends_with(ELLIPSIS));
        assert!(out.chars().count() <= 5);
    }

    /// Trailing space before an ellipsis reads as a gap. Trimmed.
    #[test]
    fn a_cut_at_a_space_does_not_leave_one_dangling() {
        let out = fit("one two three", 5.0, monospace);
        assert!(!out.contains(" …"), "{out:?} kept a dangling space");
    }

    /// Degenerate widths must still terminate and still produce something drawable, rather than
    /// panicking on an empty range or looping.
    #[test]
    fn no_room_at_all_yields_just_the_ellipsis() {
        assert_eq!(fit("session", 0.0, monospace), "…");
        assert_eq!(fit("", 0.0, monospace), "");
    }

    /// Shapes text with the same engine the renderer uses, so the widths are the real ones — a
    /// proportional font where `i` and `W` differ by a factor of four, and where the ellipsis is
    /// not one character wide. The stand-in above proves the search logic; this proves the logic
    /// survives contact with the measurements it will actually be given.
    fn shaped(content: &str) -> f32 {
        use iced::advanced::graphics::text::Paragraph;
        use iced::advanced::text::Paragraph as _;

        Paragraph::with_text(CoreText {
            content,
            bounds: Size::INFINITE,
            size: Pixels(14.0),
            line_height: text::LineHeight::default(),
            font: iced::Font::DEFAULT,
            align_x: text::Alignment::Left,
            align_y: alignment::Vertical::Top,
            shaping: text::Shaping::Advanced,
            wrapping: text::Wrapping::None,
        })
        .min_bounds()
        .width
    }

    /// The reported bug, reproduced at the width that caused it: the name from the screenshot in a
    /// sidebar row. It must come back inside the space available, which the old
    /// `Wrapping::None` label did not.
    #[test]
    fn the_overflowing_session_name_is_brought_back_inside() {
        let name = "Understand check in circle session";
        let available = 180.0; // roughly a 260px sidebar less indent, icons and the close button

        assert!(
            shaped(name) > available,
            "the test is vacuous unless this name really does overflow"
        );

        let out = fit(name, available, shaped);
        assert!(out.ends_with(ELLIPSIS), "expected an ellipsis, got {out:?}");
        assert!(
            shaped(&out) <= available,
            "{out:?} shapes to {} wide, over the {available} available — still overlapping",
            shaped(&out)
        );
    }

    /// Real glyph widths vary, so "as much as fits" cannot be an exact equality as it is for the
    /// stand-in. Instead: adding back the next character must push it over. That pins the result
    /// to the true boundary without assuming anything about the font.
    #[test]
    fn real_shaping_stops_at_the_last_character_that_fits() {
        let name = "Understand check in circle session";
        let available = 180.0;
        let out = fit(name, available, shaped);

        let kept = out.trim_end_matches(ELLIPSIS).chars().count();
        let one_more: String = name
            .chars()
            .take(kept + 1)
            .chain(std::iter::once(ELLIPSIS))
            .collect();
        assert!(
            shaped(&one_more) > available,
            "{out:?} stopped early — {one_more:?} would still have fit"
        );
    }

    /// A name that fits must be left exactly as it is: no ellipsis, no trimming, nothing.
    #[test]
    fn real_shaping_leaves_a_short_name_untouched() {
        assert_eq!(fit("main", 180.0, shaped), "main");
    }
}
