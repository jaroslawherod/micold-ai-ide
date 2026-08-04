//! `ToggleChip` — a reusable on/off pill control (Constitution Principle VIII).
//!
//! Promoted out of `src/ui/sidebar.rs`'s private `filter_chip()` when feature 014 needed the same
//! look for its "Show agent worktrees" control. Copying those thirty lines of button styling next
//! to their original is precisely the accretion the Component-reuse gate exists to prevent, so the
//! primitive moved here and both call sites consume it.
//!
//! Exposed as a chainable builder terminating in `.into()` (Principle VIII builder-API rule).

use crate::ui::material::style;
use iced::widget::{button, text};
use iced::{Background, Border, Color, Element};
use micold_core::tokens::{shape, sidebar, spacing, state, Rgb, Roles};

/// A pill-shaped on/off chip: filled in its accent while active, outlined while inactive.
/// Pressing it emits `on_press`.
///
/// Builder form: `ToggleChip::new(label, on_press, roles).active(b).accent(fill, on).into()`.
/// Carries no visual state of its own — `active` is supplied by the caller, never latched here.
pub struct ToggleChip<M> {
    label: String,
    on_press: M,
    roles: Roles,
    active: bool,
    accent: Option<(Rgb, Rgb)>,
}

impl<M> ToggleChip<M> {
    /// A chip showing `label`, emitting `on_press` when pressed, themed by `roles`. Inactive by
    /// default, with the neutral `surface_variant`/`on_surface_variant` accent.
    pub fn new(label: impl Into<String>, on_press: M, roles: Roles) -> Self {
        Self {
            label: label.into(),
            on_press,
            roles,
            active: false,
            accent: None,
        }
    }

    /// Whether the chip reads as on (filled) or off (outlined).
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The `(fill, on_fill)` pair used while active — e.g. a worktree type's tag color. Defaults
    /// to the neutral surface-variant pair.
    pub fn accent(mut self, fill: Rgb, on_fill: Rgb) -> Self {
        self.accent = Some((fill, on_fill));
        self
    }
}

impl<'a, M: Clone + 'a> From<ToggleChip<M>> for Element<'a, M> {
    fn from(chip: ToggleChip<M>) -> Self {
        let r = chip.roles;
        let (fill, on) = chip
            .accent
            .unwrap_or((r.surface_variant, r.on_surface_variant));
        let (fill, on) = (style::color(fill), style::color(on));
        let muted = style::color(r.on_surface_variant);
        let outline = style::color(r.outline);
        let active = chip.active;
        button(text(chip.label).size(sidebar::TAG))
            .padding(iced::Padding {
                top: 1.0,
                bottom: 1.0,
                left: spacing::SM,
                right: spacing::SM,
            })
            .on_press(chip.on_press)
            .style(move |_theme: &iced::Theme, status| {
                // The chip responds to the pointer. It used to ignore `status` entirely, so a
                // filter chip was the one interactive thing in the sidebar that gave no
                // feedback at all — FR-021 applies the state-layer set to *every* interactive
                // surface, not to buttons alone, and a chip that never reacts reads as
                // decoration rather than as a control.
                //
                // The layer is the content colour over the container, exactly as everywhere
                // else: over the fill when the chip is on, over what it sits on when it is off.
                let opacity = match status {
                    iced::widget::button::Status::Hovered => state::HOVER,
                    iced::widget::button::Status::Pressed => state::PRESSED,
                    _ => 0.0,
                };
                let background = if active {
                    Some(Background::Color(style::state_layer(fill, on, opacity)))
                } else if opacity > 0.0 {
                    Some(Background::Color(style::state_fill(muted, opacity)))
                } else {
                    Some(Background::Color(Color::TRANSPARENT))
                };
                iced::widget::button::Style {
                    background,
                    text_color: if active { on } else { muted },
                    border: Border {
                        color: if active { fill } else { outline },
                        width: if active { 0.0 } else { 1.0 },
                        radius: shape::FULL.into(),
                    },
                    ..Default::default()
                }
            })
            .into()
    }
}
