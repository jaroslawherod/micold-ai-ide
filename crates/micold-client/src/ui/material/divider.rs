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
use micold_core::tokens::Roles;

/// Divider thickness. One device pixel, matching what the sidebar drew by hand.
const THICKNESS: f32 = 1.0;

/// A rule separating two regions. Builder form (Principle VIII):
/// `Divider::vertical(roles).into()`.
pub struct Divider<'a, M> {
    roles: Roles,
    vertical: bool,
    marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Divider<'a, M> {
    /// A full-height vertical rule — the edge between a panel and the content beside it.
    pub fn vertical(roles: Roles) -> Self {
        Self {
            roles,
            vertical: true,
            marker: PhantomData,
        }
    }

    /// A full-width horizontal rule.
    pub fn horizontal(roles: Roles) -> Self {
        Self {
            roles,
            vertical: false,
            marker: PhantomData,
        }
    }
}

impl<'a, M: 'a> From<Divider<'a, M>> for Element<'a, M> {
    fn from(d: Divider<'a, M>) -> Self {
        let color = style::separator(d.roles);

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
