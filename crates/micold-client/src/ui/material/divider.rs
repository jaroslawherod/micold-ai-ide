//! `Divider` — a one-pixel rule between two regions (Principle VIII).
//!
//! Small enough to look like it does not need a component, which is why there were two of them,
//! hand-rolled inline with a `container(Space)` and an anonymous style closure. They already
//! differed: one drew the separator colour flat, the other blended toward the accent on hover, and
//! both spelled out the same `Background::Color` boilerplate to do it.
//!
//! Material gives dividers a thickness and a colour role of their own, so this is a real component
//! rather than a convenience — and feature 018 changes both in one place instead of two.

use std::marker::PhantomData;

use crate::ui::material::style;
use iced::widget::{container, Space};
use iced::{Element, Length};
use micold_core::tokens::Roles;

/// Divider thickness. One device pixel, matching what the sidebar drew by hand.
const THICKNESS: f32 = 1.0;

/// A rule separating two regions. Builder form (Principle VIII):
/// `Divider::vertical(roles).accent(hover).into()`.
pub struct Divider<'a, M> {
    roles: Roles,
    vertical: bool,
    accent: f32,
    marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Divider<'a, M> {
    /// A full-height vertical rule — the edge between a panel and the content beside it.
    pub fn vertical(roles: Roles) -> Self {
        Self {
            roles,
            vertical: true,
            accent: 0.0,
            marker: PhantomData,
        }
    }

    /// A full-width horizontal rule.
    pub fn horizontal(roles: Roles) -> Self {
        Self {
            roles,
            vertical: false,
            accent: 0.0,
            marker: PhantomData,
        }
    }

    /// Blend the rule toward the primary colour, `0.0` (plain separator) to `1.0` (full accent).
    ///
    /// Used for a draggable edge, which brightens as the pointer approaches it. Takes a progress
    /// value only because the hover track still lives outside the component; T041 moves it inside
    /// the resize handle and this becomes a state the divider owns.
    pub fn accent(mut self, progress: f32) -> Self {
        self.accent = progress.clamp(0.0, 1.0);
        self
    }
}

impl<'a, M: 'a> From<Divider<'a, M>> for Element<'a, M> {
    fn from(d: Divider<'a, M>) -> Self {
        let from = style::separator(d.roles);
        let to = style::color(d.roles.primary);
        let t = d.accent;
        // Blended in the renderer's own colour space rather than in 8-bit token space: an 8-bit
        // blend rounds at every step and would not reproduce the hand-rolled version exactly.
        let color = iced::Color {
            r: from.r + (to.r - from.r) * t,
            g: from.g + (to.g - from.g) * t,
            b: from.b + (to.b - from.b) * t,
            a: from.a + (to.a - from.a) * t,
        };

        let (width, height) = if d.vertical {
            (Length::Fixed(THICKNESS), Length::Fill)
        } else {
            (Length::Fill, Length::Fixed(THICKNESS))
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
