//! The iced styling layer for the Material design system (gui-only).
//!
//! Converts the pure [`tokens`] into an [`iced::Theme`] and exposes shared style helpers so
//! every surface draws from one place (SC-007). Nothing here holds decision logic; the
//! values come from `src/tokens.rs` (contracts/design-tokens.md).

use iced::widget::{button, container, text, text_input};
use iced::{Background, Border, Color, Theme};
use micold_ai_ide::theme::ColorScheme;
use micold_ai_ide::tokens::{self, shape, Rgb, Roles};

/// Convert a token color into an iced color.
pub fn color(c: Rgb) -> Color {
    Color::from_rgb8(c.r, c.g, c.b)
}

/// The same color at a given alpha (for state layers and disabled states).
fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
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

fn radius(px: u16) -> Border {
    Border {
        radius: (px as f32).into(),
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
            radius: (shape::MD as f32).into(),
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
            radius: (shape::LG as f32).into(),
        },
        ..container::Style::default()
    }
}

/// The top app bar surface (flat surface with a bottom hairline via a full border).
pub fn app_bar(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface))),
        text_color: Some(color(r.on_surface)),
        border: Border {
            color: alpha(color(r.outline), 0.3),
            width: 1.0,
            radius: 0.0.into(),
        },
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

/// The dimmed modal backdrop behind a dialog.
pub fn backdrop() -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(Color {
            a: 0.6,
            ..Color::BLACK
        })),
        ..container::Style::default()
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
                radius: (shape::SM as f32).into(),
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

/// Secondary/caption text color (paths, labels, badges) — `on_surface_variant`.
pub fn muted(r: Roles) -> impl Fn(&Theme) -> text::Style {
    move |_theme| text::Style {
        color: Some(color(r.on_surface_variant)),
    }
}

/// A text input styled to the design system.
pub fn input(r: Roles) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, status| {
        let border_color = match status {
            text_input::Status::Focused => color(r.primary),
            _ => color(r.outline),
        };
        text_input::Style {
            background: Background::Color(color(r.surface)),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: (shape::SM as f32).into(),
            },
            icon: color(r.on_surface_variant),
            placeholder: color(r.on_surface_variant),
            value: color(r.on_surface),
            selection: alpha(color(r.primary), 0.3),
        }
    }
}
