//! `UnreadMark` — the mark of an unread session, and of a count of them (Constitution Principle
//! VIII; feature 039, contract `unread-mark.md` U1–U4).
//!
//! A filled circle in the `primary` role, optionally followed by a number and the word `unread`.
//! One component for the three places that show unread state — a session's row, a switcher row and
//! the switcher's button — so that none of them restyles it (research R9).
//!
//! It is not the [`ActivityBadge`](super::ActivityBadge): that one says what a session is doing
//! now and sits in a row's leading slot; this one says the user has not looked since the session
//! last finished a turn, and sits at the trailing edge (FR-018).
//!
//! Exposed as a chainable builder terminating in `.into()` (U4).

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use crate::ui::material::{style, TypeRole};
use iced::widget::{container, row, Space};
use iced::{Alignment, Background, Element, Length};
use micold_core::tokens::{shape, spacing, Rgb, Roles};
use std::marker::PhantomData;

/// The mark's diameter (U1). Also the status bar's dot (feature 1124), so the two marks read alike.
pub(super) const DIAMETER: f32 = 8.0;

/// The size of an icon standing in for the dot: the label role's, so it reads with the number.
const GLYPH_SIZE: f32 = 16.0;

/// The role the mark is filled in (U1). One function, so that the contrast gate
/// (`composition_contrast.rs`) measures the colour that is drawn.
pub(crate) fn fill(r: Roles) -> Rgb {
    r.primary
}

/// The mark itself: a box filled in [`fill`], with the `full` corner.
fn dot_style(r: Roles) -> container::Style {
    filled_circle(fill(r))
}

/// A box filled in `colour` with the `full` corner: the unread mark, and the status bar's dot.
pub(super) fn filled_circle(colour: Rgb) -> container::Style {
    container::Style {
        background: Some(Background::Color(style::color(colour))),
        border: iced::Border {
            radius: shape::FULL.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// The unread mark, alone or followed by a count. See the module documentation.
pub struct UnreadMark<'a, M> {
    roles: Roles,
    count: Option<usize>,
    glyph: Option<Icon>,
    worded: bool,
    role: TypeRole,
    tint: CaptionTint,
    _marker: PhantomData<&'a M>,
}

/// The colour the number and the word are drawn in (U1: the host's).
#[derive(Clone, Copy)]
enum CaptionTint {
    /// The theme's text colour.
    Text,
    /// The muted colour of supporting text.
    Muted,
    /// A colour the host states.
    Stated(Rgb),
}

impl<'a, M: 'a> UnreadMark<'a, M> {
    /// The mark alone, themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            count: None,
            glyph: None,
            worded: false,
            role: TypeRole::Label,
            tint: CaptionTint::Text,
            _marker: PhantomData,
        }
    }

    /// Draw `glyph` in place of the dot: the mark of a state with a shape of its own, as the
    /// Overview's badge says needs action or attention (feature 813, FR-012). The glyph takes the
    /// [`tint`](Self::tint) if one is stated, else the mark's `primary`; the count follows it.
    pub fn icon(mut self, glyph: Icon) -> Self {
        self.glyph = Some(glyph);
        self
    }

    /// The icon standing in for the dot, if any.
    fn glyph(&self) -> Option<Icon> {
        self.glyph
    }

    /// The colour the icon is drawn in: the stated tint, else the mark's own.
    fn glyph_tint(&self) -> Rgb {
        match self.tint {
            CaptionTint::Stated(color) => color,
            CaptionTint::Text | CaptionTint::Muted => fill(self.roles),
        }
    }

    /// Follow the mark with the number of unread sessions: `● 2`. A count of zero renders nothing
    /// at all (U2).
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// Follow the number with the word `unread`: `● 2 unread`. For a host that shows another
    /// number beside this one (FR-021).
    pub fn worded(mut self, worded: bool) -> Self {
        self.worded = worded;
        self
    }

    /// Draw the number and the word in `role`: the role of the host's own label (U1). `Label`
    /// unless stated.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Draw the number and the word muted, for a host whose trailing text is muted (a menu item's
    /// running count).
    pub fn muted(mut self) -> Self {
        self.tint = CaptionTint::Muted;
        self
    }

    /// Draw the number and the word in `color`, for a host that draws its label in a colour of
    /// its own (a button's content colour). The mark itself stays in the `primary` role.
    pub fn tint(mut self, color: Rgb) -> Self {
        self.tint = CaptionTint::Stated(color);
        self
    }

    /// The text after the mark, if any.
    fn caption(&self) -> Option<String> {
        match (self.count, self.worded) {
            (Some(count), true) => Some(format!("{count} unread")),
            (Some(count), false) => Some(count.to_string()),
            (None, true) => Some("unread".to_string()),
            (None, false) => None,
        }
    }
}

impl<'a, M: 'a> From<UnreadMark<'a, M>> for Element<'a, M> {
    fn from(mark: UnreadMark<'a, M>) -> Self {
        if mark.count == Some(0) {
            return Space::new().into();
        }
        let r = mark.roles;
        let dot: Element<'a, M> = match mark.glyph() {
            Some(glyph) => icon(glyph, GLYPH_SIZE, mark.glyph_tint()),
            None => container(Space::new())
                .width(Length::Fixed(DIAMETER))
                .height(Length::Fixed(DIAMETER))
                .style(move |_theme: &iced::Theme| dot_style(r))
                .into(),
        };
        match mark.caption() {
            Some(caption) => {
                let text = super::Text::new(caption, mark.role, r);
                let text = match mark.tint {
                    CaptionTint::Text => text,
                    CaptionTint::Muted => text.muted(),
                    CaptionTint::Stated(color) => text.tint(color),
                };
                row![dot, text]
                    .spacing(spacing::XS)
                    .align_y(Alignment::Center)
                    .into()
            }
            None => dot,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::roles;
    use super::*;
    use crate::icons::Icon;
    use iced::{Background, Size};
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;

    fn size_of(element: Element<'_, ()>) -> Size {
        super::super::test_support::size_of(element, Size::new(400.0, 100.0))
    }

    /// U145 (U1, FR-018).
    #[test]
    fn the_mark_is_an_8dp_filled_circle_in_the_primary_role() {
        for scheme in [ColorScheme::Light, ColorScheme::Dark] {
            let r = tokens::roles(scheme);

            assert_eq!(
                size_of(UnreadMark::new(r).into()),
                Size::new(8.0, 8.0),
                "the mark alone is 8dp wide and 8dp tall ({scheme:?})"
            );
            let dot = dot_style(r);
            assert_eq!(
                dot.background,
                Some(Background::Color(style::color(r.primary))),
                "the mark is filled in the primary role ({scheme:?})"
            );
            assert!(
                dot.border.radius.top_left >= DIAMETER / 2.0
                    && dot.border.radius.bottom_right >= DIAMETER / 2.0,
                "the mark's corners are rounded to a circle ({scheme:?})"
            );
        }
    }

    /// U147 (U2, FR-021, FR-023).
    #[test]
    fn a_count_of_zero_renders_nothing() {
        assert_eq!(
            size_of(UnreadMark::new(roles()).count(0).into()),
            Size::ZERO,
            "there is nothing unread to mark"
        );
        assert_eq!(
            size_of(UnreadMark::new(roles()).count(0).worded(true).into()),
            Size::ZERO,
            "the word does not stand alone for a count of zero"
        );
    }

    /// U148 (FR-021): the number and the word follow the mark.
    #[test]
    fn the_count_and_the_word_follow_the_mark() {
        let alone = UnreadMark::<()>::new(roles());
        let counted = UnreadMark::<()>::new(roles()).count(2);
        let worded = UnreadMark::<()>::new(roles()).count(2).worded(true);

        assert_eq!(alone.caption(), None, "the mark alone has no text");
        assert_eq!(
            counted.caption().as_deref(),
            Some("2"),
            "a count is the number after the mark"
        );
        assert_eq!(
            worded.caption().as_deref(),
            Some("2 unread"),
            "`.worded(true)` adds the word `unread`"
        );

        let (alone, counted, worded) = (
            size_of(alone.into()).width,
            size_of(counted.into()).width,
            size_of(worded.into()).width,
        );
        assert!(
            alone < counted && counted < worded,
            "the text is drawn beside the mark: {alone} < {counted} < {worded}"
        );
    }

    /// Contract U1: the number and the word are in the host's label role.
    #[test]
    fn the_caption_is_drawn_in_the_role_its_host_states() {
        let label = size_of(UnreadMark::new(roles()).count(2).into());
        let stated = size_of(
            UnreadMark::new(roles())
                .count(2)
                .role(TypeRole::Label)
                .into(),
        );
        let action = size_of(
            UnreadMark::new(roles())
                .count(2)
                .role(TypeRole::Action)
                .into(),
        );

        assert_eq!(
            label, stated,
            "`Label` is the role a host that states none gets"
        );
        assert!(
            action.width > label.width,
            "`Action` is the larger role, so the same number is wider in it: {} against {}",
            action.width,
            label.width
        );
    }

    /// T033 (FR-012): an icon mark draws the glyph and the count, tinted by the given role.
    #[test]
    fn an_icon_mark_draws_the_glyph_and_the_count_in_the_given_tint() {
        let r = roles();
        let counted = size_of(UnreadMark::new(r).count(2).into());
        let with_icon = UnreadMark::<()>::new(r)
            .icon(Icon::CardNeedsAction)
            .count(2)
            .tint(r.error);
        let untinted = UnreadMark::<()>::new(r).icon(Icon::CardNeedsAction);

        assert_eq!(with_icon.glyph(), Some(Icon::CardNeedsAction));
        assert_eq!(
            with_icon.glyph_tint(),
            r.error,
            "the glyph takes the stated role"
        );
        assert_eq!(
            untinted.glyph_tint(),
            r.primary,
            "the mark's own role otherwise"
        );
        assert_eq!(
            with_icon.caption().as_deref(),
            Some("2"),
            "the count follows"
        );
        let icon_width = size_of(with_icon.into()).width;
        assert!(
            icon_width > counted.width + 4.0,
            "the glyph is wider than the dot it replaces: {icon_width} against {}",
            counted.width
        );
        assert_eq!(
            size_of(
                UnreadMark::new(r)
                    .icon(Icon::CardNeedsAction)
                    .count(0)
                    .into()
            ),
            Size::ZERO,
            "a count of zero still renders nothing"
        );
    }

    /// Contract U1: a host's colour for the number changes no size.
    #[test]
    fn a_muted_or_tinted_caption_keeps_its_size() {
        let plain = size_of(UnreadMark::new(roles()).count(2).worded(true).into());

        assert_eq!(
            size_of(
                UnreadMark::new(roles())
                    .count(2)
                    .worded(true)
                    .muted()
                    .into()
            ),
            plain
        );
        assert_eq!(
            size_of(
                UnreadMark::new(roles())
                    .count(2)
                    .worded(true)
                    .tint(roles().primary)
                    .into()
            ),
            plain
        );
    }
}
