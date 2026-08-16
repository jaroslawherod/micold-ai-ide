//! `Divider` — a one-pixel rule between two regions (Principle VIII).
//!
//! Small enough to look like it does not need a component, which is why there were two of them,
//! hand-rolled inline with a `container(Space)` and an anonymous style closure — both spelling out
//! the same `Background::Color` boilerplate to draw a single hairline.
//!
//! Material gives dividers a thickness and a colour role of their own, so this is a real component
//! rather than a convenience — and feature 018 changes both in one place instead of two.

use std::marker::PhantomData;

use crate::ui::material::style;
use iced::widget::{container, Space};
use iced::{Element, Length};
use micold_core::tokens::{Rgb, Roles};

/// Divider thickness. One device pixel, matching what the sidebar drew by hand.
const THICKNESS: f32 = 1.0;

/// A rule separating two regions. Builder form (Principle VIII):
/// `Divider::vertical(roles).into()`.
pub struct Divider<'a, M> {
    roles: Roles,
    vertical: bool,
    thickness: f32,
    tint: Option<Rgb>,
    marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Divider<'a, M> {
    /// A full-height vertical rule — the edge between a panel and the content beside it.
    pub fn vertical(roles: Roles) -> Self {
        Self {
            roles,
            vertical: true,
            thickness: THICKNESS,
            tint: None,
            marker: PhantomData,
        }
    }

    /// A full-width horizontal rule.
    pub fn horizontal(roles: Roles) -> Self {
        Self {
            roles,
            vertical: false,
            thickness: THICKNESS,
            tint: None,
            marker: PhantomData,
        }
    }

    /// Draw at a thickness other than the default hairline.
    ///
    /// A **tab's active indicator is a rule** — Material draws it as one, and so does this
    /// (feature 012 BUG-002, `anatomy::tab::INDICATOR`). Reusing this component rather than adding
    /// a `TabIndicator` beside it follows research R3's own conclusion in the other direction:
    /// that rejected `TreeView` for having the wrong *shape*, and a rule is exactly this shape.
    /// What a caller may not do is name a raw number here — pass the anatomy constant.
    pub fn thickness(mut self, dp: f32) -> Self {
        self.thickness = dp;
        self
    }

    /// Draw in a given colour rather than the separator role.
    ///
    /// A separator recedes; an indicator is meant to be seen. The caller supplies the colour
    /// because it also tints the label the indicator belongs to, and those two must be the same
    /// accent — two independent choices would drift, which is the class of bug FR-011a already
    /// records once for this row.
    pub fn tint(mut self, color: Rgb) -> Self {
        self.tint = Some(color);
        self
    }
}

impl<'a, M: 'a> From<Divider<'a, M>> for Element<'a, M> {
    fn from(d: Divider<'a, M>) -> Self {
        let color = match d.tint {
            Some(rgb) => iced::Color::from_rgb8(rgb.r, rgb.g, rgb.b),
            None => style::separator(d.roles),
        };

        let (width, height) = if d.vertical {
            (Length::Fixed(d.thickness), Length::Fill)
        } else {
            (Length::Fill, Length::Fixed(d.thickness))
        };
        container(Space::new().width(width).height(height))
            .width(width)
            .height(height)
            .style(move |_: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(color)),
                ..container::Style::default()
            })
            .into()
    }
}
