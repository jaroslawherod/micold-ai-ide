//! `material` — the shared, reusable UI component library (Constitution Principle VIII).
//!
//! All custom widgets/components live here and mimic Angular Material: a flat toolbar, an
//! overflow menu, a segmented/tree navigation, icon buttons, tooltips, and the [`fade`] /
//! [`slide`] animation wrappers. Features MUST reuse or extend these rather than fork bespoke
//! one-off widgets. Every component is theme-aware (draws from the active [`Roles`]) and
//! cross-platform. This module is the living catalog Principle VIII refers to.
//!
//! [`Roles`]: micold_ai_ide::tokens::Roles

mod animation;
mod icon_button;
mod menu;
mod modal;
mod project_switcher;
mod tag;
mod terminal_pane;
mod toolbar;
mod tree_view;

pub use animation::{fade, scale, slide};
pub use icon_button::IconButton;
pub use menu::{ContextMenu, MenuItem, MenuOverlay, MenuTrigger};
pub use modal::Modal;
pub use project_switcher::{ProjectRow, ProjectSwitcherOverlay, ProjectSwitcherTrigger};
pub use tag::Tag;
pub(crate) use terminal_pane::target_offset_delta;
pub use terminal_pane::TerminalPane;
#[cfg(test)]
pub(crate) use terminal_pane::{scrollbar_metrics, viewport_row};
pub use toolbar::Toolbar;
pub use tree_view::{TreeItem, TreeView};

use crate::ui::style;
use iced::widget::{container, text, tooltip};
use iced::Element;
use micold_ai_ide::tokens::{spacing, type_scale, Roles};

/// Wrap any element with a hover tooltip describing the action it triggers (Principle VIII
/// builder-API rule: construct with the required content + label + roles, then `.into()`).
/// Theme-aware surface styling; shown below the element.
pub struct Tooltip<'a, M> {
    content: Element<'a, M>,
    label: String,
    roles: Roles,
}

impl<'a, M: 'a> Tooltip<'a, M> {
    /// Wrap `content` with a hover tooltip showing `label`, themed by `roles`.
    pub fn new(content: impl Into<Element<'a, M>>, label: impl Into<String>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            label: label.into(),
            roles,
        }
    }
}

impl<'a, M: 'a> From<Tooltip<'a, M>> for Element<'a, M> {
    fn from(t: Tooltip<'a, M>) -> Self {
        let tip = container(text(t.label).size(type_scale::LABEL))
            .padding(spacing::XS)
            .style(style::surface(t.roles));
        tooltip(t.content, tip, tooltip::Position::Bottom)
            .gap(spacing::XS)
            .into()
    }
}
