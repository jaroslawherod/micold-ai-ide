//! `micold-material` — the generic Material Design building blocks (feature 1262).
//!
//! Buttons, fields, menus, tabs, dialogs, motion, the styling layer that turns design tokens into
//! rendering types, the CDK behaviour layer ([`cdk`]) and the icon vocabulary ([`icons`]). It names
//! no Micold domain type — no session, daemon, worktree, queue or diff: `tests/no_domain_types.rs`
//! holds it to the render-free `micold_core` modules it may read (tokens, theme and a few models).
//! The client's `ui::material` composes these into Micold's own widgets and re-exports this crate.
//!
//! All custom widgets/components live here and mimic Angular Material: a flat toolbar, an
//! overflow menu, a segmented/tree navigation, icon buttons, tooltips, and the [`fade`] /
//! [`expand`] animation wrappers. Every component is theme-aware (draws from the active
//! [`Roles`]) and cross-platform.
//!
//! [`Roles`]: micold_core::tokens::Roles

pub mod cdk;
pub mod icons;

pub mod accordion;
pub mod accordion_group;
pub mod accordion_section;
pub mod animation;
pub mod attachment_chip;
pub mod browse_bar;
pub mod busy_indicator;
pub mod button;
pub mod checkbox;
pub mod choice_list;
pub mod dialog;
pub mod disclosure_card;
pub mod divider;
pub mod edge_fade;
pub mod ellipsized;
pub mod field_group;
pub mod filled_field;
pub mod filter_panel;
pub mod form_field;
pub mod glyph;
pub mod icon_button;
pub mod icon_label;
pub mod key_value_list;
pub mod keyboard_focus;
/// A toggle that names what it toggles — a glyph and a short word in one control (feature 026,
/// T066a). The terminal bar's AI-CLI mode toggle carries the session's CLI with it.
pub mod labelled_toggle;
pub mod line_clamp;
pub mod link_text;
pub mod menu;
pub mod modal;
pub mod mono;
pub mod navigation_drawer;
pub mod progress;
pub mod required_legend;
pub mod resize_handle;
pub mod ripple;
/// A press draws one state layer with its ripple, read back from rendered frames (BUG-014).
#[cfg(test)]
pub mod ripple_layers;
pub mod scrollable;
pub mod scrolling_dialog;
pub mod section_list;
pub mod shaped_text;
pub mod snackbar;
/// The split action — a primary button with an adjacent "…or choose" chevron (feature 026,
/// T033). Used by the sidebar's start-a-session row action.
pub mod split_action;
pub mod status_bar;
/// The one place design tokens become rendering types. Internal by intent (FR-002): a feature
/// module that could reach it could render an off-spec variant of a shared component, which is
/// exactly the drift this feature removes. `pub` rather than private only because the
/// application's own theme function lives behind it — see the client's `ui::theme`.
pub mod style;
/// The style layer's parity snapshot. Lives inside the crate rather than in `tests/` because the
/// layer it asserts is no longer reachable from outside it — which is the point.
#[cfg(test)]
pub mod style_elevation;
#[cfg(test)]
pub mod style_outline_discipline;
#[cfg(test)]
pub mod style_states;
pub mod surface;
pub mod tab;
pub mod tab_strip;
pub mod tag;
/// The headless renderer the component tests share, in this crate and in the client's
/// (`features = ["test-support"]`).
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
pub mod text;
pub mod text_area;
pub mod text_field;
pub mod text_link;
pub mod toggle_chip;
pub mod toolbar;
/// The application's typographic vocabulary, pinned. In-crate for the same reason as the style
/// snapshots above: `TypeRole` is not reachable from `tests/`.
#[cfg(test)]
pub mod type_role_mapping;
pub mod unread_mark;
pub mod virtual_rows;
pub use accordion::Accordion;
pub use accordion_group::AccordionGroup;
pub use accordion_section::AccordionSection;
pub use animation::{expand, fade, scale, scrim, HoverReveal, ViewFade};
pub use attachment_chip::AttachmentChip;
pub use browse_bar::BrowseBar;
pub use busy_indicator::BusyIndicator;
pub use button::{Button, Variant as ButtonVariant};
pub use checkbox::Checkbox;
pub use choice_list::{ChoiceList, OTHER as CHOICE_OTHER, OTHER_PLACEHOLDER};
pub use disclosure_card::DisclosureCard;
pub use divider::Divider;
pub use edge_fade::EdgeFade;
pub use ellipsized::Ellipsized;
pub use field_group::FieldGroup;
pub use filter_panel::FilterTrigger;
pub use form_field::beneath;
pub use form_field::{FormField, Layer as FieldLayer};
pub use glyph::Glyph;
pub use icon_button::IconButton;
pub use icon_label::IconLabel;
pub use key_value_list::KeyValueList;
pub use labelled_toggle::LabelledToggle;
pub use link_text::LinkText;
pub use menu::{menu_panel_size, ContextMenu, MenuItem, MenuOverlay, MenuTrigger};
pub use modal::Modal;
pub use mono::{
    install_monospace, pin_monospace, FIRA_MONO_BOLD_BYTES, FIRA_MONO_REGULAR_BYTES, MONO_FAMILY,
};
pub use navigation_drawer::NavigationDrawer;
pub use progress::StageProgress;
pub use required_legend::RequiredLegend;
pub use resize_handle::{ResizeHandle, WIDTH as RESIZE_HANDLE_WIDTH};
pub use ripple::{pulse as ripple_pulse, Ripple};
pub use scrollable::{ScrollDirection, Scrollable};
pub use scrolling_dialog::ScrollingDialog;
pub use section_list::{Section, SectionList};
pub use snackbar::Snackbar;
pub use split_action::SplitAction;
pub use status_bar::{GiveWay, ItemTone, StatusBar, StatusItem};
pub use surface::{Kind as SurfaceKind, Surface};
pub use tab::{content_colour as tab_content_colour, IndicatorEdge, Tab, WIDTH as TAB_WIDTH};
pub use tab_strip::TabStrip;
pub use tag::Tag;
pub use text::{
    Text, TypeRole, ROBOTO, ROBOTO_MEDIUM_BYTES, ROBOTO_REGULAR_BYTES, ROBOTO_SEMIBOLD_BYTES,
};
pub use text_area::TextArea;
pub use text_field::TextField;
pub use text_link::TextLink;
pub use toggle_chip::{chip_label, chip_neutral_accent, ToggleChip};
pub use toolbar::Toolbar;
pub use unread_mark::UnreadMark;
pub use virtual_rows::VirtualRows;

/// The application's theme, derived from the active colour scheme.
///
/// The one part of the styling layer that reaches beyond the library. The window needs a theme to
/// hand the renderer, and that is application wiring rather than a call site styling a widget —
/// every other entry point is unreachable from outside, so a feature module is structurally unable
/// to render an off-spec variant of a shared component (FR-002).
pub fn theme(scheme: micold_core::theme::ColorScheme) -> iced::Theme {
    style::theme(scheme)
}

use crate::cdk::tooltip;
use iced::widget::container;
use iced::Element;
use micold_core::tokens::{spacing, Roles};

/// Re-exported so call sites can pick a `Tooltip::position(...)` without reaching into `iced`
/// directly.
pub use crate::cdk::tooltip::Position as TooltipPosition;

/// The widest a tooltip panel may grow before its label wraps (feature 029, FR-009).
///
/// Material 3's ceiling for a rich tooltip, and component-owned rather than an `anatomy::` token:
/// `anatomy::ALL` is a fixed array asserted against feature 018's contract, which names no tooltip
/// figure at all, so adding one there would edit that feature's gate to state something it never
/// claimed. `tab.rs`'s `LABEL_MAX_WIDTH` is the precedent for a component holding its own ceiling.
///
/// It matters here because a tooltip's label used to be one short phrase and is now a block of
/// lines, one of which is a filesystem path. Unbounded, the panel measures at that path's natural
/// width and can end up wider than the sidebar it is describing.
pub const TOOLTIP_MAX_WIDTH: f32 = 320.0;

/// Wrap any element with a hover tooltip describing the action it triggers (Principle VIII
/// builder-API rule: construct with the required content + label + roles, then optionally
/// `.position(...)`, then `.into()`). Theme-aware surface styling; shown below the element by
/// default, and on the opposite side of it when the asked-for side has no room, so it does not
/// cover the element it describes (029 BUG-001, FR-013). Only a panel too large for either side
/// is slid onto it, from the side with more room.
///
/// The label may be multi-line: it is bounded at [`TOOLTIP_MAX_WIDTH`] and wraps on glyphs where a
/// word boundary is unavailable, so a long path breaks rather than widening the panel.
pub struct Tooltip<'a, M> {
    content: Element<'a, M>,
    label: String,
    roles: Roles,
    position: tooltip::Position,
    wait: tooltip::Wait,
    max_lines: Option<usize>,
    subject: Option<u64>,
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
            wait: tooltip::Wait::Hover,
            max_lines: None,
            subject: None,
        }
    }

    /// Open only after the cursor has rested on the content for `delay`, and never while it moves
    /// (feature 038, FR-015, FR-016). A press on the content closes the tooltip until the cursor
    /// has left. Without this the tooltip opens on hover. An alternative to
    /// [`Tooltip::show_delay`]: the one called last wins.
    pub fn after_rest(mut self, delay: std::time::Duration) -> Self {
        self.wait = tooltip::Wait::Rest(delay);
        self
    }

    /// Open only once the pointer has been over the content for `delay`, counted from entering and
    /// not restarted by movement (feature 430, FR-003). Works with
    /// [`TooltipPosition::FollowCursor`]. An alternative to [`Tooltip::after_rest`]: the one called
    /// last wins.
    pub fn show_delay(mut self, delay: std::time::Duration) -> Self {
        self.wait = tooltip::Wait::Delay(delay);
        self
    }

    /// Show at most `lines` lines of the label; a label that takes more is cut after a whole word
    /// and ends in an ellipsis, and one that fits is shown whole (FR-021).
    pub fn max_lines(mut self, lines: usize) -> Self {
        self.max_lines = Some(lines);
        self
    }

    /// What the content describes, for a tooltip on a row a list reuses: when `key` changes, an
    /// open tooltip closes and the rest delay starts again (FR-017).
    pub fn subject(mut self, key: u64) -> Self {
        self.subject = Some(key);
        self
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
        //
        // `WordOrGlyph` rather than the stack's default `Word`: the labels this carries include
        // filesystem paths, which contain no spaces and so cannot be word-wrapped at all. Without
        // the glyph fallback the ceiling below would be a limit the text simply ignores.
        let label: Element<'a, M> = match t.max_lines {
            // Measured where it is laid out: inside the ceiling below, less the panel's padding.
            Some(lines) => line_clamp::LineClamped::new(t.label, TypeRole::Caption, lines).into(),
            None => Text::new(t.label, TypeRole::Caption, t.roles)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .into(),
        };
        let tip = container(label)
            .max_width(TOOLTIP_MAX_WIDTH)
            .padding(spacing::XS)
            .style(style::surface(t.roles));
        // Built on `cdk::tooltip` rather than the stack's own: that one slides a panel with no
        // room on its side back over the trigger, and this one opens it on the other side
        // (029 BUG-001, FR-013).
        let mut floated = tooltip::Tooltip::new(t.content, tip, t.position).gap(spacing::XS);
        match t.wait {
            tooltip::Wait::Hover => {}
            tooltip::Wait::Delay(delay) => floated = floated.show_delay(delay),
            tooltip::Wait::Rest(delay) => floated = floated.after_rest(delay),
        }
        if let Some(key) = t.subject {
            floated = floated.subject(key);
        }
        floated.into()
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

#[cfg(test)]
mod tooltip_wait_tests {
    use super::*;
    use micold_core::theme::ColorScheme;
    use micold_core::tokens;
    use std::time::Duration;

    fn tip() -> Tooltip<'static, ()> {
        Tooltip::new(
            Text::new("a", TypeRole::Label, tokens::roles(ColorScheme::Dark)),
            "label",
            tokens::roles(ColorScheme::Dark),
        )
    }

    /// 430 research R1: `after_rest` and `show_delay` are alternatives; the one called last wins.
    #[test]
    fn the_wait_set_last_wins() {
        let d = Duration::from_secs(1);
        assert_eq!(
            tip().after_rest(d).show_delay(d).wait,
            tooltip::Wait::Delay(d)
        );
        assert_eq!(
            tip().show_delay(d).after_rest(d).wait,
            tooltip::Wait::Rest(d)
        );
    }
}
