//! `StateIcon` — a location's state on its sidebar row (Constitution Principle VIII; feature 905,
//! contract `state-icon.md` S1).
//!
//! One glyph per state, each a different shape so the state reads without colour (FR-007): a
//! triangle for Running, an octagon for Blocked, two bars for Idle / waiting. An Unknown state
//! draws nothing in a slot of the same width, so the row's name never moves (FR-005, FR-010).
//!
//! `Copy`, so a [`TreeItem`](super::TreeItem) can hold it and size it to its own label role.

use crate::icons::Icon;
use crate::ui::material::glyph::icon;
use crate::ui::material::TypeRole;
use iced::widget::{container, Space};
use iced::{Alignment, Element, Length};
use micold_core::location_state::LocationState;
use micold_core::tokens::{Rgb, Roles};

/// The glyph a state draws, or `None` for the empty slot of an Unknown state (data-model *Glyph*).
fn glyph(state: LocationState) -> Option<Icon> {
    match state {
        LocationState::Running => Some(Icon::StateRunning),
        LocationState::Blocked(_) => Some(Icon::StateBlocked),
        LocationState::Idle => Some(Icon::StateIdle),
        LocationState::Unknown => None,
    }
}

/// The colour role a state's glyph is drawn in (data-model *Glyph*). One function, so the contrast
/// gate (`composition_contrast.rs`) measures the colour that is drawn.
pub(crate) fn role(state: LocationState, r: &Roles) -> Option<Rgb> {
    match state {
        LocationState::Running => Some(r.primary),
        LocationState::Blocked(_) => Some(r.error),
        LocationState::Idle => Some(r.on_surface_variant),
        LocationState::Unknown => None,
    }
}

/// A location's state icon. See the module documentation.
#[derive(Debug, Clone, Copy)]
pub struct StateIcon {
    state: LocationState,
    roles: Roles,
    size: f32,
}

impl StateIcon {
    /// The icon of `state`, themed by `roles`, at the sidebar label's size.
    pub fn new(state: LocationState, roles: Roles) -> Self {
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
        let content: Element<'a, M> = match (glyph(s.state), role(s.state, &s.roles)) {
            (Some(glyph), Some(color)) => icon(glyph, s.size, color),
            _ => Space::new().into(),
        };
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
        let drawn: Vec<char> = CardState::ALL.iter().map(|&s| glyph(s).glyph()).collect();
        assert_eq!(
            drawn,
            ['\u{e645}', '\u{e8f4}', '\u{e037}', '\u{e86c}', '\u{e034}']
        );
        assert_eq!(drawn.iter().collect::<HashSet<_>>().len(), 5);
        for state in CardState::ALL {
            assert_eq!(glyph(state), super::super::card_state_icon(state));
        }
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
