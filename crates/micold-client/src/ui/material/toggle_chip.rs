//! `ToggleChip` — a reusable on/off pill control (Constitution Principle VIII).
//!
//! Promoted out of `src/ui/sidebar.rs`'s private `filter_chip()` when feature 014 needed the same
//! look for its "Show agent worktrees" control. Copying those thirty lines of button styling next
//! to their original is precisely the accretion the Component-reuse gate exists to prevent, so the
//! primitive moved here and both call sites consume it.
//!
//! Exposed as a chainable builder terminating in `.into()` (Principle VIII builder-API rule).

use crate::ui::material::style;
use crate::ui::material::{Text, TypeRole};
use iced::widget::button;
use iced::{Background, Border, Color, Element};
use micold_core::tokens::{anatomy, shape, state, Rgb, Roles};

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
        // The ripple takes a token role rather than a resolved colour, so keep the `Rgb` before
        // these are shadowed by their `Color` forms.
        let ripple_tint = if chip.active {
            on
        } else {
            r.on_surface_variant
        };
        let (fill, on) = (style::color(fill), style::color(on));
        let muted = style::color(r.on_surface_variant);
        let outline = style::color(r.outline);
        let active = chip.active;
        // §7.6 gives a chip `label_large`. It was `SidebarTag` (`label_small`, 11dp), which was
        // right while the chip had no height of its own — but T060 gave it §7.6's 32dp, and an 11dp
        // word in a 32dp pill leaves so much empty height that a short label renders as a circle
        // rather than a chip. The height and the label role are one decision, not two.
        //
        // `Tag` keeps `sidebar_tag`: the contract's own row for the worktree tag says `label_small`
        // *in the sidebar*, and those sit inside 36dp dense rows where 32dp chips would not fit.
        let chip_button = button(
            Text::new(chip.label, TypeRole::Action, r)
                // §7.6's label alignment (FR-030a), and the whole of BUG-001. A chip is 32dp and
                // its label is a 20dp line box, so 12dp of slack exists by construction and
                // *something* has to place it. Two defaults were placing it, neither chosen: the
                // button lays its content out under `limits.height(Fixed(32))`, which sets the
                // minimum with the maximum and stretches this node to the full height, and a text
                // widget draws its glyphs at the top of whatever node it is given. The label read
                // 6dp high of centre while every constant in §7.6 was correct.
                //
                // Both halves are stated rather than inherited: `Fill` says the node is the pill's
                // height on purpose, `Center` says where the line box sits inside it.
                .height(iced::Length::Fill)
                .align_y(iced::alignment::Vertical::Center),
        )
        // §7.6: a chip is 32dp tall with 12dp of horizontal padding. There is no vertical padding
        // because the height is set outright — which is precisely why the alignment above is not
        // optional. Zero vertical padding means "size me by my label" only while a component has
        // no height of its own.
        .padding(iced::Padding {
            top: 0.0,
            bottom: 0.0,
            left: anatomy::chip::PADDING,
            right: anatomy::chip::PADDING,
        })
        .height(iced::Length::Fixed(anatomy::chip::HEIGHT))
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
                    width: if active { 0.0 } else { anatomy::chip::OUTLINE },
                    radius: shape::FULL.into(),
                },
                ..Default::default()
            }
        });
        // A filter chip is pressed like anything else, so it ripples like anything else (FR-024c),
        // in its own text colour: the accent when on, the muted role when off.
        super::Ripple::new(chip_button, ripple_tint, shape::FULL).into()
    }
}
