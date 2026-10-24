//! `Tag` — a reusable worktree tag/chip primitive (Constitution Principle VIII).
//!
//! A small pill carrying a short label on a solid fill, used for the color-coded type and
//! Jira/issue tags (and the missing/invalid status cue) beneath a worktree's name. Theme-aware
//! via the `(fill, on_fill)` color pair supplied by the caller (from `tokens::Roles`). Exposed
//! as a chainable builder terminating in `.into()` (Principle VIII builder-API rule), so it is
//! reused rather than each feature forking a bespoke chip.

use crate::style;
use crate::TypeRole;
use iced::widget::{container, row, text};
use iced::{Alignment, Element, Length, Padding};
use micold_core::tokens::{spacing, Rgb};
use std::marker::PhantomData;

/// A pill-shaped tag chip. Construct with a label + `accent` color (rendered as a dimmed tonal
/// chip — accent text on a faint accent tint); optionally set the type role (defaults to the
/// sidebar tag role).
pub struct Tag<'a, M> {
    label: String,
    accent: Rgb,
    on_accent: Option<Rgb>,
    role: TypeRole,
    truncate_to: Option<f32>,
    max_width: Option<f32>,
    leading: Option<Element<'a, M>>,
    _marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Tag<'a, M> {
    /// A tag showing `label` in the `accent` color.
    pub fn new(label: impl Into<String>, accent: Rgb) -> Self {
        Self {
            label: label.into(),
            accent,
            on_accent: None,
            role: TypeRole::SidebarTag,
            truncate_to: None,
            max_width: None,
            leading: None,
            _marker: PhantomData,
        }
    }

    /// Draw the accent as an opaque fill with `on_accent` as the label, instead of the default
    /// tint.
    ///
    /// For a tag whose background is not the caller's to know — one that can land on a plain
    /// surface or on a filled container depending on state. The tint reads as an accent only over
    /// the former; see [`style::chip_solid`].
    pub fn solid(mut self, on_accent: Rgb) -> Self {
        self.on_accent = Some(on_accent);
        self
    }

    /// Fix the chip at `width` dp and end its label in an ellipsis when it does not fit, so a long
    /// label cannot push what stands beside it. For a label of unknown length; a short one is better
    /// left to size itself.
    pub fn truncate_to(mut self, width: f32) -> Self {
        self.truncate_to = Some(width);
        self
    }

    /// Size the chip by its label up to `width` dp, and end the label in an ellipsis above it
    /// (feature 1126, R11). Unlike [`Self::truncate_to`], a label that fits keeps the chip its own
    /// size; only a long one is cut, and its full text belongs in a tooltip around the chip.
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Put `leading` (a glyph or an indicator) before the label, inside the chip (feature 1126,
    /// R11): the session area's state chip leads with its state's icon, its pull request chip with
    /// the sidebar's indicator.
    pub fn leading(mut self, leading: impl Into<Element<'a, M>>) -> Self {
        self.leading = Some(leading.into());
        self
    }

    /// Override the label's type role.
    ///
    /// A role rather than a size, so a tag outside the sidebar cannot end up at a size that is not
    /// in the scale — which is what a bare `f32` here allowed.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }
}

impl<'a, M: 'a> From<Tag<'a, M>> for Element<'a, M> {
    fn from(t: Tag<'a, M>) -> Self {
        // Taken apart rather than built with `Text`, because a tag carries an accent colour instead
        // of a `Roles` set and `Text` needs one. Still a role, so the weight and line height come
        // from the scale rather than from the renderer's defaults.
        let colour = t.on_accent.unwrap_or(t.accent);
        let label: Element<'a, M> = match (t.truncate_to, t.max_width) {
            (Some(_), _) => super::Ellipsized::at_role(t.label, t.role, colour).into(),
            (None, Some(_)) => super::Ellipsized::at_role(t.label, t.role, colour)
                .shrink()
                .into(),
            (None, None) => text(t.label)
                .size(t.role.size())
                .font(t.role.font())
                .line_height(t.role.line_height())
                .into(),
        };
        let content: Element<'a, M> = match t.leading {
            Some(leading) => row![leading, label]
                .spacing(spacing::XS)
                .align_y(Alignment::Center)
                .into(),
            None => label,
        };
        let mut chip = container(content)
            .width(t.truncate_to.map_or(Length::Shrink, Length::Fixed))
            .padding(Padding {
                top: 0.0,
                bottom: 0.0,
                left: spacing::XS,
                right: spacing::XS,
            });
        if let Some(width) = t.max_width {
            chip = chip.max_width(width);
        }
        // Styled in each arm rather than boxing one closure: the two style functions have different
        // types, and a `Box<dyn Fn>` here buys nothing the match does not.
        match t.on_accent {
            Some(on_accent) => chip.style(style::chip_solid(t.accent, on_accent)).into(),
            None => chip.style(style::chip(t.accent)).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{roles, size_of, Mounted};
    use iced::widget::Space;
    use iced::Size;

    const ROOM: Size = Size::new(600.0, 100.0);

    fn tag(label: &str) -> Tag<'static, String> {
        Tag::new(label, roles().primary)
    }

    /// Feature 1126 R11: under its ceiling a chip is as wide as its label, the same as without one.
    #[test]
    fn a_short_label_keeps_the_chip_its_own_size_under_a_ceiling() {
        let natural = size_of(tag("main").into(), ROOM);
        let capped = size_of(tag("main").max_width(200.0).into(), ROOM);
        assert!(natural.width < 200.0, "the sample is short: {natural:?}");
        assert!(
            (capped.width - natural.width).abs() < 0.5,
            "a label that fits sizes the chip: {capped:?} vs {natural:?}"
        );
    }

    /// Feature 1126 R11: a long label is cut at the ceiling and ends in an ellipsis.
    #[test]
    fn a_long_label_is_cut_at_the_ceiling_with_an_ellipsis() {
        let long = "feat/1126_worktree-context-header-and-session-tabs-above-the-terminal";
        let capped = size_of(tag(long).max_width(120.0).into(), ROOM);
        assert!(capped.width <= 120.0 + 0.5, "{capped:?}");
        let mut mounted = Mounted::new_in(tag(long).max_width(120.0), ROOM);
        let painted = mounted.painted_text();
        assert!(
            painted.iter().any(|p| p.content.ends_with('\u{2026}')),
            "the label ends in an ellipsis: {:?}",
            painted.iter().map(|p| &p.content).collect::<Vec<_>>()
        );
    }

    /// Feature 1126 R11: the leading element sits inside the chip, before the label.
    #[test]
    fn a_leading_element_widens_the_chip_by_itself_and_a_gap() {
        let plain = size_of(tag("Working").into(), ROOM);
        let led = size_of(
            tag("Working")
                .leading(Space::new().width(16.0).height(16.0))
                .into(),
            ROOM,
        );
        assert!(
            (led.width - (plain.width + 16.0 + spacing::XS)).abs() < 0.5,
            "{led:?} vs {plain:?}"
        );
    }
}
