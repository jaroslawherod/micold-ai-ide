//! `FilterTrigger` — the sidebar tag-filter toggle button (feature 009, Constitution
//! Principle VIII).
//!
//! The filter panel itself is rendered as an inline accordion in `src/ui/sidebar.rs` (via the
//! shared [`super::expand`] primitive) rather than a floating overlay — it pushes the worktree
//! list down when open instead of floating over it, so this module only supplies the trigger
//! button. Reuses the existing `filter_bar()`/`filter_chip()` content (`src/ui/sidebar.rs`)
//! verbatim as the accordion's body.

use super::{IconButton, Tooltip};
use crate::icons::Icon;
use crate::tokens::Roles;
use iced::Element;

/// The filter trigger: an icon button (emitting `on_toggle`) placed in the sidebar header,
/// tinted to reflect whether any filter is currently active (feature 009 US2, research R4).
/// Builder form (Principle VIII): `FilterTrigger::new(on_toggle, roles).active(b).into()`.
pub struct FilterTrigger<M> {
    on_toggle: M,
    roles: Roles,
    active: bool,
}

impl<M> FilterTrigger<M> {
    /// A trigger emitting `on_toggle` when pressed, themed by `roles`. Inactive tint by default.
    pub fn new(on_toggle: M, roles: Roles) -> Self {
        Self {
            on_toggle,
            roles,
            active: false,
        }
    }

    /// Whether any tag filter is currently active — tints the icon with the `primary` role
    /// instead of the default muted `on_surface_variant` tint (research R4). Carries no visual
    /// state of its own; callers pass `!state.sidebar_filters.is_empty()`.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl<'a, M: Clone + 'a> From<FilterTrigger<M>> for Element<'a, M> {
    fn from(t: FilterTrigger<M>) -> Self {
        let tint = if t.active {
            t.roles.primary
        } else {
            t.roles.on_surface_variant
        };
        let button = IconButton::new(Icon::Filter, t.roles)
            .tint(tint)
            .on_press(t.on_toggle);
        Tooltip::new(button, "Filter worktrees", t.roles).into()
    }
}
