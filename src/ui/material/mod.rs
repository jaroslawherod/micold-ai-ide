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
mod toolbar;
mod tree_view;

pub use animation::{fade, slide};
pub use icon_button::icon_button;
pub use menu::{menu_overlay, menu_trigger, MenuItem};
pub use toolbar::toolbar;
pub use tree_view::{tree_view, TreeItem};

use crate::ui::style;
use iced::widget::{container, text, tooltip};
use iced::Element;
use micold_ai_ide::tokens::{spacing, type_scale, Roles};

/// Wrap any element with a hover tooltip describing the action it triggers. Reusable
/// (Principle VIII); theme-aware surface styling. Shown below the element.
pub fn with_tooltip<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    label: impl Into<String>,
    r: Roles,
) -> Element<'a, M> {
    let tip = container(text(label.into()).size(type_scale::LABEL))
        .padding(spacing::XS)
        .style(style::surface(r));
    tooltip(content, tip, tooltip::Position::Bottom)
        .gap(spacing::XS)
        .into()
}
