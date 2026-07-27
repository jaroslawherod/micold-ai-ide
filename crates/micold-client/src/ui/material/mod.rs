//! `material` — the shared, reusable UI component library (Constitution Principle VIII).
//!
//! All custom widgets/components live here and mimic Angular Material: a flat toolbar, an
//! overflow menu, a segmented/tree navigation, icon buttons, tooltips, and the [`fade`] /
//! [`slide`] animation wrappers. Features MUST reuse or extend these rather than fork bespoke
//! one-off widgets. Every component is theme-aware (draws from the active [`Roles`]) and
//! cross-platform. This module is the living catalog Principle VIII refers to.
//!
//! [`Roles`]: micold_core::tokens::Roles

mod activity_badge;
mod animation;
mod button;
mod checkbox;
mod connection_banner;
mod divider;
mod filter_panel;
pub mod glyph;
mod icon_button;
mod menu;
mod modal;
mod progress;
mod project_switcher;
mod scrollable;
mod select;
/// The one place design tokens become rendering types. Internal to the library by
/// intent (FR-002): a feature module that could reach it could render an off-spec variant of a
/// shared component, which is exactly the drift this feature removes.
pub mod style;
mod surface;
mod tag;
mod terminal_pane;
mod text;
mod text_field;
mod toggle_chip;
mod toolbar;
mod tree_view;

pub use activity_badge::ActivityBadge;
pub use animation::{expand, fade, scale, slide};
pub use button::{Button, Variant as ButtonVariant};
pub use checkbox::Checkbox;
pub use connection_banner::ConnectionBanner;
pub use divider::Divider;
pub use filter_panel::FilterTrigger;
pub use glyph::Glyph;
pub use icon_button::IconButton;
pub use menu::{menu_panel_size, ContextMenu, MenuItem, MenuOverlay, MenuTrigger};
pub use modal::Modal;
pub use progress::StageProgress;
pub use project_switcher::{ProjectRow, ProjectSwitcherOverlay, ProjectSwitcherTrigger};
pub use scrollable::Scrollable;
pub use select::Select;
pub use surface::{Kind as SurfaceKind, Surface};
pub use tag::Tag;
#[cfg(test)]
pub(crate) use terminal_pane::scrollbar_metrics;
pub use terminal_pane::target_offset_delta;
pub use terminal_pane::TerminalPane;
pub use text::{Text, TypeRole};
pub use text_field::TextField;
pub use toggle_chip::ToggleChip;
pub use toolbar::Toolbar;
pub use tree_view::{TreeItem, TreeView};

use micold_core::tokens::{spacing, type_scale, Roles};
use iced::widget::{container, text as text_widget, tooltip};
use iced::Element;

/// Re-exported so call sites can pick a `Tooltip::position(...)` without reaching into `iced`
/// directly.
pub use iced::widget::tooltip::Position as TooltipPosition;

/// Wrap any element with a hover tooltip describing the action it triggers (Principle VIII
/// builder-API rule: construct with the required content + label + roles, then optionally
/// `.position(...)`, then `.into()`). Theme-aware surface styling; shown below the element by
/// default.
pub struct Tooltip<'a, M> {
    content: Element<'a, M>,
    label: String,
    roles: Roles,
    position: tooltip::Position,
}

impl<'a, M: 'a> Tooltip<'a, M> {
    /// Wrap `content` with a hover tooltip showing `label`, themed by `roles`. Defaults to
    /// showing below the content — override with `.position(...)` for content near an edge
    /// (e.g. `tooltip::Position::Left` for controls pinned to the right edge of their
    /// container, so the tooltip opens inward instead of overflowing off-screen).
    pub fn new(content: impl Into<Element<'a, M>>, label: impl Into<String>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            label: label.into(),
            roles,
            position: tooltip::Position::Bottom,
        }
    }

    /// Override where the tooltip opens relative to its content.
    pub fn position(mut self, position: tooltip::Position) -> Self {
        self.position = position;
        self
    }
}

impl<'a, M: 'a> From<Tooltip<'a, M>> for Element<'a, M> {
    fn from(t: Tooltip<'a, M>) -> Self {
        let tip = container(text_widget(t.label).size(type_scale::LABEL))
            .padding(spacing::XS)
            .style(style::surface(t.roles));
        tooltip(t.content, tip, t.position).gap(spacing::XS).into()
    }
}

/// The themed surface every floating popover's inner panel sits on (`MenuOverlay`,
/// `ContextMenu`, `ProjectSwitcherOverlay`, the sidebar's filter accordion): padded content on
/// the `menu_surface` background, at `width` (pass `Length::Shrink` for a natural-width panel
/// like the filter accordion, or `Length::Fixed(...)` for a fixed-width dropdown). Factors out
/// what was otherwise the identical `container(...).padding(...).style(...)` chain repeated at
/// every popover call site. `bordered` drops the outline for panels that already read as
/// distinct without one (the filter accordion sits inline in the sidebar rather than floating,
/// so its own outline would be redundant next to the sidebar's edge).
pub fn menu_panel<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    width: impl Into<iced::Length>,
    roles: Roles,
    bordered: bool,
) -> Element<'a, M> {
    container(content)
        .padding(spacing::XS)
        .width(width)
        .style(move |theme: &iced::Theme| {
            let mut panel_style = style::menu_surface(roles)(theme);
            if !bordered {
                panel_style.border.width = 0.0;
            }
            panel_style
        })
        .into()
}
