//! `UserTurn` — what a user said, set apart in a transcript (#1663).
//!
//! A tinted surface with a 4dp `primary` edge on the left, so the start of each of the user's turns
//! is found at a glance among the agent's output. It wraps content it is given and adds nothing of
//! its own: no text, no focusable widget, no message, so selection and the content's own controls
//! behave as without it (contracts/user-turn.md). Colours come from [`Roles`] alone.

use crate::style;
use iced::widget::{container, row, Space};
use iced::{Element, Length};
use micold_core::tokens::{spacing, Roles};

/// The accent edge's width.
const EDGE: f32 = 4.0;

/// A user's turn. Builder form (Principle VIII): `UserTurn::new(content, roles).into()`.
pub struct UserTurn<'a, M> {
    content: Element<'a, M>,
    roles: Roles,
}

impl<'a, M: 'a> UserTurn<'a, M> {
    /// `content` on the user-turn surface, behind the accent edge.
    pub fn new(content: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            roles,
        }
    }
}

impl<'a, M: 'a> From<UserTurn<'a, M>> for Element<'a, M> {
    fn from(turn: UserTurn<'a, M>) -> Self {
        let edge = container(Space::new().width(Length::Fixed(EDGE)).height(Length::Fill))
            .width(Length::Fixed(EDGE))
            .height(Length::Fill)
            .style(style::user_turn_edge(turn.roles));
        let body = container(turn.content)
            .padding(spacing::SM)
            .width(Length::Fill);
        container(row![edge, body].height(Length::Shrink))
            .width(Length::Fill)
            .style(style::user_turn_surface(turn.roles))
            .into()
    }
}
