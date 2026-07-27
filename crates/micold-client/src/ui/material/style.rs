//! The iced styling layer for the Material design system (gui-only).
//!
//! Converts the pure [`tokens`] into an [`iced::Theme`] and exposes shared style helpers so
//! every surface draws from one place (SC-007). Nothing here holds decision logic; the
//! values come from `src/tokens.rs` (contracts/design-tokens.md).

use crate::app::NoticeLevel;
use iced::overlay::menu;
use iced::widget::{
    button, checkbox as checkbox_widget, container, pick_list, scrollable, text, text_input,
};
use iced::{Background, Border, Color, Theme};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, shape, Rgb, Roles};

/// Convert a token color into an iced color.
pub fn color(c: Rgb) -> Color {
    Color::from_rgb8(c.r, c.g, c.b)
}

/// The same color at a given alpha (for state layers and disabled states).
fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// The Material opacity for disabled content. The button style fns below apply it to their own
/// labels; widgets that draw their own content (an icon glyph, say) must apply it themselves —
/// an explicit `.color()` on the content overrides the button's inherited `text_color`, so a
/// disabled button cannot grey out content that colors itself.
pub const DISABLED_OPACITY: f32 = 0.38;

/// A token color at the disabled opacity, for content a widget colors itself.
pub fn disabled_color(c: Rgb) -> Color {
    alpha(color(c), DISABLED_OPACITY)
}

/// A low-contrast divider color (thin 1px separator lines, e.g. under a toolbar or beside the
/// sidebar): the `outline` role softened with reduced alpha so it reads as a subtle line, not
/// a hard rule.
pub fn separator(r: Roles) -> Color {
    alpha(color(r.outline), 0.4)
}

/// Linearly blend `over` on top of `base` by factor `t` (0 = base, 1 = over).
fn blend(base: Color, over: Color, t: f32) -> Color {
    Color {
        r: base.r + (over.r - base.r) * t,
        g: base.g + (over.g - base.g) * t,
        b: base.b + (over.b - base.b) * t,
        a: 1.0,
    }
}

fn radius(px: f32) -> Border {
    Border {
        radius: px.into(),
        ..Border::default()
    }
}

/// Build the iced theme for a resolved scheme from the token palette
/// (contracts/design-tokens.md palette mapping).
pub fn theme(scheme: ColorScheme) -> Theme {
    let r = tokens::roles(scheme);
    let name = match scheme {
        ColorScheme::Light => "Micold Light",
        ColorScheme::Dark => "Micold Dark",
    };
    Theme::custom(
        name.to_string(),
        iced::theme::Palette {
            background: color(r.background),
            text: color(r.on_surface),
            primary: color(r.primary),
            // No dedicated success role in this UI; reuse primary.
            success: color(r.primary),
            // Likewise no dedicated warning role (new in iced 0.14) — `error` is the closest
            // token, and nothing in this UI renders a palette-driven warning today.
            warning: color(r.error),
            danger: color(r.error),
        },
    )
}

/// The window background surface.
pub fn window_bg(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.background))),
        text_color: Some(color(r.on_background)),
        ..container::Style::default()
    }
}

/// A raised Material surface (card/dialog): `surface` fill, subtle outline, rounded.
pub fn surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface))),
        text_color: Some(color(r.on_surface)),
        border: Border {
            color: alpha(color(r.outline), 0.4),
            width: 1.0,
            radius: shape::MD.into(),
        },
        ..container::Style::default()
    }
}

/// A dialog surface: like [`surface`] but with the larger dialog radius.
pub fn dialog(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface))),
        text_color: Some(color(r.on_surface)),
        border: Border {
            color: alpha(color(r.outline), 0.4),
            width: 1.0,
            radius: shape::LG.into(),
        },
        ..container::Style::default()
    }
}

/// The sidebar surface — flat `surface` fill with **no border and no rounded corners**. The
/// sidebar's right edge is drawn by the resize handle's 1px boundary line instead.
pub fn sidebar_surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface))),
        text_color: Some(color(r.on_surface)),
        border: Border::default(),
        ..container::Style::default()
    }
}

/// The Material toolbar surface — flat `surface` fill, **no border** (the toolbar's bottom
/// border is a separate 1px line composed alongside this surface, same idiom as
/// [`sidebar_surface`]'s resize-handle boundary).
pub fn toolbar_surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface))),
        text_color: Some(color(r.on_surface)),
        border: Border::default(),
        ..container::Style::default()
    }
}

/// A raised menu/dropdown surface: `surface` fill, subtle outline, rounded (for popup menus).
pub fn menu_surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface))),
        text_color: Some(color(r.on_surface)),
        border: Border {
            color: alpha(color(r.outline), 0.4),
            width: 1.0,
            radius: shape::SM.into(),
        },
        ..container::Style::default()
    }
}

/// The closed field of a `pick_list`-backed `Select` (feature 013): an outlined box matching
/// [`input`]'s look, with the outline switching to `primary` while hovered or open.
pub fn select_field(r: Roles) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_theme, status| {
        let border_color = match status {
            pick_list::Status::Hovered | pick_list::Status::Opened { .. } => color(r.primary),
            pick_list::Status::Active => color(r.outline),
        };
        pick_list::Style {
            text_color: color(r.on_surface),
            placeholder_color: color(r.on_surface_variant),
            handle_color: color(r.on_surface_variant),
            background: Background::Color(color(r.surface)),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: shape::SM.into(),
            },
        }
    }
}

/// The dropdown list of a `pick_list`-backed `Select` (feature 013): the same look as
/// [`menu_surface`], with the current selection tinted `primary` (pick_list highlights it on
/// open the same way it highlights hover, so this doubles as the "reopen shows the current
/// selection" treatment, FR-003).
pub fn select_menu(r: Roles) -> impl Fn(&Theme) -> menu::Style {
    move |_theme| menu::Style {
        background: Background::Color(color(r.surface)),
        border: Border {
            color: alpha(color(r.outline), 0.4),
            width: 1.0,
            radius: shape::SM.into(),
        },
        text_color: color(r.on_surface),
        selected_text_color: color(r.on_primary),
        selected_background: Background::Color(color(r.primary)),
        // New in 0.14; the default zero shadow keeps the flat 0.13 look.
        shadow: iced::Shadow::default(),
    }
}

/// A themed scrollbar for scrollable regions (e.g. the worktree sidebar): a subtle rail with a
/// rounded `outline`-colored thumb that darkens on hover/drag. Visible whenever content overflows.
pub fn scrollbar(r: Roles) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |_theme, status| {
        let thumb = match status {
            scrollable::Status::Hovered {
                is_vertical_scrollbar_hovered: true,
                ..
            }
            | scrollable::Status::Dragged {
                is_vertical_scrollbar_dragged: true,
                ..
            } => color(r.on_surface_variant),
            _ => alpha(color(r.outline), 0.8),
        };
        let rail = scrollable::Rail {
            background: Some(Background::Color(alpha(color(r.surface_variant), 0.6))),
            border: radius(shape::FULL),
            scroller: scrollable::Scroller {
                background: Background::Color(thumb),
                border: radius(shape::FULL),
            },
        };
        scrollable::Style {
            container: container::Style::default(),
            vertical_rail: rail,
            horizontal_rail: rail,
            gap: None,
            // The autoscroll overlay is new in 0.14; themed to match the rest of the design
            // system (surface pill, `outline` edge, flat) rather than iced's default.
            auto_scroll: scrollable::AutoScroll {
                background: Background::Color(alpha(color(r.surface), 0.9)),
                border: Border {
                    color: alpha(color(r.outline), 0.8),
                    width: 1.0,
                    radius: shape::FULL.into(),
                },
                shadow: iced::Shadow::default(),
                icon: color(r.on_surface_variant),
            },
        }
    }
}

/// A worktree tag chip: a dimmed **tonal** pill — a faint tint of `accent` behind `accent`-colored
/// text, fully rounded (feature 008, FR-005). Softer than a solid fill so the tags read as calm
/// metadata beneath the worktree name. The vivid `accent` as text on its own faint tint keeps
/// high contrast in both light and dark schemes.
pub fn chip(accent: Rgb) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(alpha(color(accent), 0.20))),
        text_color: Some(color(accent)),
        border: radius(shape::FULL),
        ..container::Style::default()
    }
}

/// A known-projects / selector list row.
pub fn list_item(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface_variant))),
        text_color: Some(color(r.on_surface)),
        border: radius(shape::MD),
        ..container::Style::default()
    }
}

/// A global notification banner. `Error` uses the error role so a failed action reads as one
/// at a glance; `Info` reuses the neutral list-row surface.
pub fn notification(r: Roles, level: NoticeLevel) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        let (bg, fg) = match level {
            NoticeLevel::Error => (r.error, r.on_error),
            NoticeLevel::Info => (r.surface_variant, r.on_surface),
        };
        container::Style {
            background: Some(Background::Color(color(bg))),
            text_color: Some(color(fg)),
            border: radius(shape::MD),
            ..container::Style::default()
        }
    }
}

/// Filled button: primary fill, `on_primary` label (the single primary action, FR-015).
pub fn filled(r: Roles) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let base = color(r.primary);
        let on = color(r.on_primary);
        let bg = match status {
            button::Status::Active => base,
            button::Status::Hovered => blend(base, on, 0.08),
            button::Status::Pressed => blend(base, on, 0.12),
            button::Status::Disabled => alpha(base, 0.38),
        };
        let text = if matches!(status, button::Status::Disabled) {
            alpha(on, 0.38)
        } else {
            on
        };
        button::Style {
            background: Some(Background::Color(bg)),
            text_color: text,
            border: radius(shape::SM),
            ..button::Style::default()
        }
    }
}

/// Outlined button: transparent fill, `outline` border, `primary` label (secondary actions).
pub fn outlined(r: Roles) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let prim = color(r.primary);
        let fill = match status {
            button::Status::Hovered => Some(Background::Color(alpha(prim, 0.08))),
            button::Status::Pressed => Some(Background::Color(alpha(prim, 0.12))),
            _ => None,
        };
        let (text, border_color) = if matches!(status, button::Status::Disabled) {
            (alpha(prim, 0.38), alpha(color(r.outline), 0.38))
        } else {
            (prim, color(r.outline))
        };
        button::Style {
            background: fill,
            text_color: text,
            border: Border {
                color: border_color,
                width: 1.0,
                radius: shape::SM.into(),
            },
            ..button::Style::default()
        }
    }
}

/// Text button: transparent, no border, `primary` label (low-emphasis actions).
pub fn text_button(r: Roles) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let prim = color(r.primary);
        let fill = match status {
            button::Status::Hovered => Some(Background::Color(alpha(prim, 0.08))),
            button::Status::Pressed => Some(Background::Color(alpha(prim, 0.12))),
            _ => None,
        };
        let text = if matches!(status, button::Status::Disabled) {
            alpha(prim, 0.38)
        } else {
            prim
        };
        button::Style {
            background: fill,
            text_color: text,
            border: radius(shape::SM),
            ..button::Style::default()
        }
    }
}

/// A small icon-only button with a fully-rounded hit area (`IconButton::circular`) — same
/// hover/press fill and label color as [`text_button`], but a `shape::FULL` border radius so a
/// roughly-square button (small, uniform padding) reads as a circle around the glyph rather than
/// a rounded square.
pub fn circular_icon_button(r: Roles) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let prim = color(r.primary);
        let fill = match status {
            button::Status::Hovered => Some(Background::Color(alpha(prim, 0.08))),
            button::Status::Pressed => Some(Background::Color(alpha(prim, 0.12))),
            _ => None,
        };
        let text = if matches!(status, button::Status::Disabled) {
            alpha(prim, 0.38)
        } else {
            prim
        };
        button::Style {
            background: fill,
            text_color: text,
            border: radius(shape::FULL),
            ..button::Style::default()
        }
    }
}

/// Secondary/caption text color (paths, labels, badges) — `on_surface_variant`.
pub fn muted(r: Roles) -> impl Fn(&Theme) -> text::Style {
    move |_theme| text::Style {
        color: Some(color(r.on_surface_variant)),
    }
}

/// A checkbox styled to the design system (feature 011's "Enabled" toggle).
pub fn checkbox(r: Roles) -> impl Fn(&Theme, checkbox_widget::Status) -> checkbox_widget::Style {
    move |_theme, status| {
        let is_checked = matches!(
            status,
            checkbox_widget::Status::Active { is_checked: true }
                | checkbox_widget::Status::Hovered { is_checked: true }
        );
        let border_color = match status {
            checkbox_widget::Status::Hovered { .. } => color(r.primary),
            _ => color(r.outline),
        };
        checkbox_widget::Style {
            background: Background::Color(if is_checked {
                color(r.primary)
            } else {
                color(r.surface)
            }),
            icon_color: color(r.on_primary),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: shape::SM.into(),
            },
            text_color: Some(color(r.on_surface)),
        }
    }
}

/// A text input styled to the design system.
pub fn input(r: Roles) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, status| {
        let border_color = match status {
            text_input::Status::Focused { .. } => color(r.primary),
            _ => color(r.outline),
        };
        text_input::Style {
            background: Background::Color(color(r.surface)),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: shape::SM.into(),
            },
            icon: color(r.on_surface_variant),
            placeholder: color(r.on_surface_variant),
            value: color(r.on_surface),
            selection: alpha(color(r.primary), 0.3),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Bin unit tests — run with `cargo test --features gui`.
    use super::*;
    use iced::widget::button::Status;

    /// A glyph that colors itself does not inherit a disabled button's `text_color`, so
    /// `IconButton` greys it via `disabled_color`. That must match what the button style fn
    /// applies to its own label, or a disabled icon button and a disabled text button would
    /// disagree about how faded "disabled" looks.
    #[test]
    fn disabled_color_matches_the_button_styles_disabled_label() {
        let r = tokens::roles(ColorScheme::Dark);
        let style = text_button(r)(&iced::Theme::Dark, Status::Disabled);
        assert_eq!(disabled_color(r.primary), style.text_color);
    }

    /// The enabled path must stay fully opaque — greying is the disabled state alone.
    #[test]
    fn enabled_icon_tint_is_opaque() {
        let r = tokens::roles(ColorScheme::Dark);
        assert_eq!(color(r.on_surface).a, 1.0);
        assert_eq!(disabled_color(r.on_surface).a, DISABLED_OPACITY);
    }
}
