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
            .align_x(Alignment::Center)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::roles;
    use super::*;
    use iced::Size;
    use micold_core::location_state::BlockedReason;

    const STATES: [LocationState; 4] = [
        LocationState::Running,
        LocationState::Blocked(BlockedReason::NeedsPermission),
        LocationState::Idle,
        LocationState::Unknown,
    ];

    fn size_of(s: StateIcon) -> Size {
        super::super::test_support::size_of::<()>(s.into(), Size::new(400.0, 100.0))
    }

    /// S1, A2: every state, the empty one included, is a slot of the stated width, and no glyph is
    /// taller than the sidebar name's line, so no state changes a row's height.
    #[test]
    fn every_state_takes_the_same_width() {
        let name_line = super::super::test_support::size_of(
            super::super::Text::<()>::new("Ag", TypeRole::SidebarName, roles()).into(),
            Size::new(400.0, 100.0),
        )
        .height;
        for state in STATES {
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
                size_of(StateIcon::new(state, roles()).size(20.0)).width,
                20.0,
                "{state:?} at 20dp"
            );
        }
    }

    /// FR-007: three shapes, and the Unknown state draws none.
    #[test]
    fn three_states_draw_three_glyphs_and_unknown_draws_none() {
        let drawn: Vec<_> = STATES.iter().map(|&s| glyph(s)).collect();
        assert_eq!(
            drawn,
            [
                Some(Icon::StateRunning),
                Some(Icon::StateBlocked),
                Some(Icon::StateIdle),
                None
            ]
        );
        for reason in [
            BlockedReason::Resumable,
            BlockedReason::Failed,
            BlockedReason::NeedsPermission,
        ] {
            assert_eq!(
                glyph(LocationState::Blocked(reason)),
                Some(Icon::StateBlocked)
            );
        }
    }

    /// Data-model *Glyph*: the colour roles.
    #[test]
    fn each_glyph_is_drawn_in_its_role() {
        let r = roles();
        assert_eq!(role(LocationState::Running, &r), Some(r.primary));
        assert_eq!(
            role(LocationState::Blocked(BlockedReason::Failed), &r),
            Some(r.error)
        );
        assert_eq!(role(LocationState::Idle, &r), Some(r.on_surface_variant));
        assert_eq!(role(LocationState::Unknown, &r), None);
    }
}
