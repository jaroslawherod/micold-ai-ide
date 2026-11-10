//! `StateIcon` — a location's card state on its sidebar row (Constitution Principle VIII; feature
//! 905, contract `state-icon.md` S1, re-keyed on `CardState` by feature 1718, research R1).
//!
//! One glyph per card state, the Overview card's own (`overview_card::state_icon`), so the five
//! states read without colour (FR-004, SC-002): Needs action, Needs attention, Working, Done and
//! Idle each draw a different shape in a slot of one width, so the row's name never moves. A
//! location with no session is Idle, so the slot is never blank.
//!
//! [`row_tint`] is the row's colour-only fill by the same state (feature 1718, FR-010 to FR-012).
//!
//! `Copy`, so a [`TreeItem`](super::TreeItem) can hold it and size it to its own label role.

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use crate::ui::material::TypeRole;
use iced::widget::container;
use iced::{Alignment, Element, Length};
use micold_core::overview::CardState;
use micold_core::tokens::{Rgb, Roles};

/// The glyph a card state draws: the Overview card's.
fn glyph(state: CardState) -> Icon {
    super::card_state_icon(state)
}

/// The colour role a state's glyph is drawn in (research R1, R7): the ink of its row's tint for the
/// tinted states, so the glyph sits on its tint as the text does. One function, so the contrast
/// gate (`composition_contrast.rs`) measures the colour that is drawn.
pub(crate) fn role(state: CardState, r: &Roles) -> Rgb {
    tint_ink(state, r).unwrap_or(match state {
        CardState::Working => r.primary,
        _ => r.on_surface_variant,
    })
}

/// A row's resting fill by card state (feature 1718, FR-010, FR-011, research R7): the louder
/// container pair for Needs action and Needs attention, a quieter tonal one for Done (the cleanup
/// candidate), none for Working and Idle. Colour only: the position, the glyph and the words stay.
pub fn row_tint(state: CardState, r: &Roles) -> Option<Rgb> {
    match state {
        CardState::NeedsAction => Some(r.error_container),
        CardState::NeedsAttention => Some(r.tertiary_container),
        CardState::Done => Some(r.surface_container_high),
        CardState::Working | CardState::Idle => None,
    }
}

/// The colour of a tinted row's text and glyph (FR-012): the container's `on_*` role.
pub fn tint_ink(state: CardState, r: &Roles) -> Option<Rgb> {
    match state {
        CardState::NeedsAction => Some(r.on_error_container),
        CardState::NeedsAttention => Some(r.on_tertiary_container),
        CardState::Done => Some(r.on_surface),
        CardState::Working | CardState::Idle => None,
    }
}

/// A location's state icon. See the module documentation.
#[derive(Debug, Clone, Copy)]
pub struct StateIcon {
    pub(in crate::ui) state: CardState,
    roles: Roles,
    size: f32,
}

impl StateIcon {
    /// The icon of `state`, themed by `roles`, at the sidebar label's size.
    pub fn new(state: CardState, roles: Roles) -> Self {
        Self {
            state,
            roles,
            size: TypeRole::SidebarName.size(),
        }
    }

    /// Draw the glyph, and size the slot, at `px`: a host passes its own label role's size.
    pub fn size(mut self, px: f32) -> Self {
        self.size = px;
        self
    }
}

impl<'a, M: 'a> From<StateIcon> for Element<'a, M> {
    fn from(s: StateIcon) -> Self {
        let content: Element<'a, M> = icon(glyph(s.state), s.size, role(s.state, &s.roles));
        container(content)
            .width(Length::Fixed(s.size))
            .height(Length::Fixed(s.size))
            .align_x(Alignment::Center)
            .align_y(Alignment::Center)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::roles;
    use super::*;
    use iced::Size;
    use micold_core::overview::CardState;
    use std::collections::HashSet;

    fn size_of(s: StateIcon) -> Size {
        super::super::test_support::size_of::<()>(s.into(), Size::new(400.0, 100.0))
    }

    /// T015 (SC-002, G5): every card state is a slot of the stated width, and no glyph is taller
    /// than the sidebar name's line, so no state changes a row's height.
    #[test]
    fn every_state_takes_the_same_width() {
        let name_line = super::super::test_support::size_of(
            super::super::Text::<()>::new("Ag", TypeRole::SidebarName, roles()).into(),
            Size::new(400.0, 100.0),
        )
        .height;
        for state in CardState::ALL {
            let default = size_of(StateIcon::new(state, roles()));
            assert_eq!(
                default.width,
                TypeRole::SidebarName.size(),
                "{state:?} at the default size"
            );
            assert!(
                default.height <= name_line + 0.5,
                "{state:?} is {}dp tall, the name's line {name_line}dp",
                default.height
            );
            assert_eq!(
                default.height, default.width,
                "{state:?}: the slot is square"
            );
            let px = TypeRole::Title.size();
            assert_eq!(
                size_of(StateIcon::new(state, roles()).size(px)).width,
                px,
                "{state:?} at {px}dp"
            );
        }
    }

    /// T015 (SC-002, FR-004): five card states, five different glyphs, and the Overview card's.
    #[test]
    fn five_card_states_draw_five_different_glyphs() {
        let drawn: Vec<Icon> = CardState::ALL.iter().map(|&s| glyph(s)).collect();
        assert_eq!(
            drawn,
            [
                Icon::CardNeedsAction,
                Icon::CardNeedsAttention,
                Icon::StateRunning,
                Icon::CardDone,
                Icon::StateIdle
            ]
        );
        assert_eq!(
            drawn
                .iter()
                .map(|i| i.glyph())
                .collect::<HashSet<_>>()
                .len(),
            5
        );
    }

    /// T015: a location with no session is Idle, which draws a glyph: the slot is never blank.
    #[test]
    fn idle_is_drawn_not_blank() {
        assert_eq!(glyph(CardState::Idle), Icon::StateIdle);
    }

    /// Research R1, R7: the glyph's colour is the one its row's tint pairs it with.
    #[test]
    fn each_glyph_is_drawn_in_its_role() {
        let r = roles();
        assert_eq!(role(CardState::NeedsAction, &r), r.on_error_container);
        assert_eq!(role(CardState::NeedsAttention, &r), r.on_tertiary_container);
        assert_eq!(role(CardState::Working, &r), r.primary);
        assert_eq!(role(CardState::Done, &r), r.on_surface);
        assert_eq!(role(CardState::Idle, &r), r.on_surface_variant);
    }

    /// T023 (G7, FR-010, FR-011): the tint by card state.
    #[test]
    fn row_tint_by_card_state() {
        let r = roles();
        assert_eq!(
            row_tint(CardState::NeedsAction, &r),
            Some(r.error_container)
        );
        assert_eq!(
            row_tint(CardState::NeedsAttention, &r),
            Some(r.tertiary_container)
        );
        assert_eq!(
            row_tint(CardState::Done, &r),
            Some(r.surface_container_high)
        );
        assert_eq!(row_tint(CardState::Working, &r), None);
        assert_eq!(row_tint(CardState::Idle, &r), None);
    }
}
