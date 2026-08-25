//! `material` — the shared, reusable UI component library (Constitution Principle VIII).
//!
//! All custom widgets/components live here and mimic Angular Material: a flat toolbar, an
//! overflow menu, a segmented/tree navigation, icon buttons, tooltips, and the [`fade`] /
//! [`expand`] animation wrappers. Features MUST reuse or extend these rather than fork bespoke
//! one-off widgets. Every component is theme-aware (draws from the active [`Roles`]) and
//! cross-platform. This module is the living catalog Principle VIII refers to.
//!
//! [`Roles`]: micold_core::tokens::Roles

mod accordion;
mod activity_badge;
/// Whether a component lays out at the size its anatomy entry states, or grows into whatever room
/// it is offered. The constants gate reads the numbers, the snapshot records the geometry, and
/// `content_placement` reads the drawing — none of them asks whether the box is the stated size
/// (BUG-002).
#[cfg(test)]
mod anatomy_size;
mod animation;
mod button;
/// §7.3's horizontal paddings, read off the laid-out button. `anatomy_size` covers the height and
/// the touch target; a button is content-sized across, so no size check can see its inset.
#[cfg(test)]
mod button_anatomy;
mod checkbox;
mod connection_banner;
/// Where a component puts its content inside a height taller than that content. In-crate for the
/// same reason as `form_field_anatomy`, and the first check that reads what a component *drew*
/// rather than the constants it was built from (BUG-001).
#[cfg(test)]
mod content_placement;
pub(crate) mod dialog;
/// §7.4's spatial figures, read off a laid-out dialog. Every one of them is a *gap*, which no
/// size check can see.
#[cfg(test)]
mod dialog_anatomy;
mod divider;
mod ellipsized;
/// A field reporting and taking the keyboard, driven rather than posed — the gap the anatomy
/// modules structurally cannot see (BUG-003).
#[cfg(test)]
mod field_focus;
mod filled_field;
mod filter_panel;
mod form_field;
/// `FormField`'s composition and chrome, checked in-crate because `material` is `pub(crate)` and a
/// `FormField` cannot be constructed from `tests/` at all.
#[cfg(test)]
mod form_field_anatomy;
pub mod glyph;
mod icon_button;
mod icon_label;
mod menu;
/// §7.5's *spatial* figures — the item's inset, the panel's padding, the leading glyph, and what
/// sits between two items. `anatomy_size` reads sizes; these are positions, and nothing read them
/// (BUG-003).
#[cfg(test)]
mod menu_anatomy;
mod modal;
mod navigation_drawer;
mod picker;
/// The transition both lists arrive and leave by, asserted against **each other** and against
/// §6.3 rather than each against the contract. In-crate for the same reason as `picker_parity`
/// (feature 022, FR-021, SC-007).
#[cfg(test)]
mod picker_motion;
/// The two pickers' lists, compared against **each other** rather than each against the contract.
/// In-crate because neither control is reachable from `tests/`; see the module's own docs for why
/// the comparison is of geometry (feature 022, SC-001).
#[cfg(test)]
mod picker_parity;
/// Whether a press on an unavailable row stops at it — the half of "not pickable" that no
/// state-level test can see, and the one that closed the add-worktree form (016 BUG-002, FR-035).
#[cfg(test)]
mod picker_press;
mod progress;
mod resize_handle;
mod ripple;
mod scrollable;
mod section_list;
mod select;
/// The select's own anatomy, and the two behaviours nothing outside it can observe — its indicator
/// answering for itself, and its highlight seeded from the current choice. In-crate for the same
/// reason as `form_field_anatomy` (feature 022, FR-013).
#[cfg(test)]
mod select_anatomy;
mod snackbar;
/// The one place design tokens become rendering types. Internal by intent (FR-002): a feature
/// module that could reach it could render an off-spec variant of a shared component, which is
/// exactly the drift this feature removes. `pub(crate)` rather than private only because the
/// application's own theme function lives behind it — see [`crate::ui::theme`].
pub(crate) mod style;

/// The ripple's clipping, checked in rasterised pixels rather than in geometry. In-crate for the
/// same reason as the snapshots above.
#[cfg(test)]
mod ripple_clipping;
/// The reference scene's ripple, and the rule it presses by. In-crate for the same reason.
#[cfg(test)]
mod ripple_pulse;
/// The style layer's parity snapshot. Lives inside the crate rather than in `tests/` because the
/// layer it asserts is no longer reachable from outside it — which is the point.
#[cfg(test)]
mod style_elevation;
#[cfg(test)]
mod style_outline_discipline;
#[cfg(test)]
mod style_shape;
#[cfg(test)]
mod style_snapshot;
#[cfg(test)]
mod style_states;
mod surface;
mod tag;
mod terminal_pane;
/// The headless renderer the in-crate component tests share.
#[cfg(test)]
mod test_support;
mod text;
mod text_field;
/// The filled field's anatomy, checked in-crate — `material` is `pub(crate)`.
#[cfg(test)]
mod text_field_anatomy;
mod toggle_chip;
mod toolbar;
mod tree_view;
/// The application's typographic vocabulary, pinned. In-crate for the same reason as the style
/// snapshots above: `TypeRole` is not reachable from `tests/`.
#[cfg(test)]
mod type_role_mapping;
mod typeahead;

pub use accordion::Accordion;
pub use activity_badge::ActivityBadge;
pub use animation::{expand, fade, scale, scrim, HoverReveal, ViewFade};
pub use button::{Button, Variant as ButtonVariant};
pub use checkbox::Checkbox;
pub use connection_banner::ConnectionBanner;
pub use divider::Divider;
pub use ellipsized::Ellipsized;
pub use filter_panel::FilterTrigger;
pub use form_field::{FormField, Layer as FieldLayer};
pub use glyph::Glyph;
pub use icon_button::IconButton;
pub use icon_label::IconLabel;
pub use menu::{menu_panel_size, ContextMenu, MenuItem, MenuOverlay, MenuTrigger};
pub use modal::Modal;
pub use navigation_drawer::NavigationDrawer;
pub use picker::Row as TypeaheadRow;
pub use progress::StageProgress;
pub use resize_handle::ResizeHandle;
pub use ripple::{pulse as ripple_pulse, Ripple};
pub use scrollable::Scrollable;
pub use section_list::{Section, SectionList};
pub use select::Select;
pub use snackbar::Snackbar;
pub use surface::{Kind as SurfaceKind, Surface};
pub use tag::Tag;
#[cfg(test)]
pub(crate) use terminal_pane::scrollbar_metrics;
pub use terminal_pane::target_offset_delta;
pub use terminal_pane::GridSizeReporter;
pub use terminal_pane::TerminalPane;
pub use text::{Text, TypeRole, ROBOTO, ROBOTO_MEDIUM_BYTES, ROBOTO_REGULAR_BYTES};
pub use text_field::TextField;
pub use toggle_chip::ToggleChip;
pub use toolbar::Toolbar;
pub use tree_view::{TreeItem, TreeView};
pub use typeahead::Typeahead;

/// The application's theme, derived from the active colour scheme.
///
/// The one part of the styling layer that reaches beyond the library. The window needs a theme to
/// hand the renderer, and that is application wiring rather than a call site styling a widget —
/// every other entry point is unreachable from outside, so a feature module is structurally unable
/// to render an off-spec variant of a shared component (FR-002).
pub fn theme(scheme: micold_core::theme::ColorScheme) -> iced::Theme {
    style::theme(scheme)
}

use iced::widget::{container, tooltip};
use iced::Element;
use micold_core::tokens::{spacing, Roles};

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
        // A tooltip explains, so it is prose at `Caption` — Material's `body_small`.
        let tip = container(Text::new(t.label, TypeRole::Caption, t.roles))
            .padding(spacing::XS)
            .style(style::surface(t.roles));
        tooltip(t.content, tip, t.position).gap(spacing::XS).into()
    }
}

/// The themed surface every floating popover's inner panel sits on (`MenuOverlay`,
/// `ContextMenu`, the sidebar's filter accordion): padded content on
/// the `menu_surface` background, at `width` (pass `Length::Shrink` for a natural-width panel
/// like the filter accordion, or `Length::Fixed(...)` for a fixed-width dropdown). Factors out
/// what was otherwise the identical `container(...).padding(...).style(...)` chain repeated at
/// every popover call site. `bordered` drops the outline for panels that already read as
/// distinct without one (the filter accordion sits inline in the sidebar rather than floating,
/// so its own outline would be redundant next to the sidebar's edge).
///
/// `padding` is stated by the caller rather than fixed here, because the panels this serves do not
/// agree about it and pretending they do is how §7.5's 8dp went unapplied. A **menu** panel pads 8dp
/// above its first item and below its last and nothing at either side — its items run edge to edge,
/// which is what makes a full-width state layer possible. The filter accordion holds arbitrary
/// content and pads it on all four sides. One number could satisfy either, not both.
pub fn menu_panel<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    width: impl Into<iced::Length>,
    roles: Roles,
    bordered: bool,
    padding: impl Into<iced::Padding>,
) -> Element<'a, M> {
    container(content)
        .padding(padding)
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
