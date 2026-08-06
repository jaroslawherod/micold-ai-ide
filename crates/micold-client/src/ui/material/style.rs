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
use iced::{Background, Border, Color, Shadow, Theme};
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, elevation, shape, state, Rgb, Roles};

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

/// A divider: a hairline separating content *within* a surface.
///
/// Drawn in `outline_variant`, which is the role Material defines for exactly this and one of the
/// only three legitimate uses of a line at all (contract §1.5). It replaces feature 003's
/// `outline`-at-40%: that was a full-strength role dimmed by hand to stop it reading as a hard
/// rule, which is the job `outline_variant` already does at full alpha.
pub fn separator(r: Roles) -> Color {
    color(r.outline_variant)
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

/// A state layer over an opaque container (FR-020).
///
/// A state layer is the *content* colour composited over the container at the state's opacity.
/// This is the only place that composition happens, so "what does a hover look like" has one
/// answer for list rows, menu items, chips, tags and every button variant alike — the breadth is
/// the requirement, and it is what feature 003 lacked when only buttons responded.
///
/// Opacities come from `tokens::state`, never from a literal. Feature 003 used 0.12 for pressed,
/// which is the *selected* opacity: close enough to look reasonable, and wrong enough that a
/// selected row and a pressed one were indistinguishable.
pub fn state_layer(container: Color, content: Color, opacity: f32) -> Color {
    blend(container, content, opacity)
}

/// The same layer as a standalone fill, for a surface that paints nothing at rest.
///
/// Left semi-transparent rather than composited, so it works over whatever it happens to sit on —
/// a text button in a dialog and the same button on a card get a layer that suits each.
pub fn state_fill(content: Color, opacity: f32) -> Color {
    alpha(content, opacity)
}

fn radius(px: f32) -> Border {
    Border {
        radius: px.into(),
        ..Border::default()
    }
}

/// The drop shadow for an elevation level, in this scheme (contract §4, research R1).
///
/// **Material's key and ambient shadows folded into one.** The renderer exposes a single shadow per
/// widget, so the key shadow's offset is kept and its blur widened to stand in for the ambient
/// spread. Two overlapping shadows cannot be expressed here, and approximating them with one is
/// closer than dropping either.
///
/// Level 0 returns the default (fully transparent, zero-blur) shadow: a resting surface casts
/// nothing.
pub fn elevation_shadow(r: Roles, level: u8) -> Shadow {
    let Some(spec) = elevation::LEVELS[level as usize].shadow else {
        return Shadow::default();
    };
    let strength = match r.scheme() {
        ColorScheme::Light => spec.alpha_light,
        // Stronger in dark, and only so the shadow is not lost entirely — the tonal shift remains
        // the primary depth cue there (FR-016).
        ColorScheme::Dark => spec.alpha_dark,
    };
    Shadow {
        color: alpha(color(r.shadow), strength),
        offset: iced::Vector::new(0.0, spec.offset_y),
        blur_radius: spec.blur,
    }
}

/// A container at an elevation level, with the given corner size.
///
/// The single place a surface's depth is expressed, so "which containers are elevated, and how" is
/// answerable by reading one function rather than every style below. **No border**: depth comes from
/// the level's tone and its shadow, and adding an outline on top reads as a sticker rather than a
/// raised plane (contract §1.5, asserted by `style_outline_discipline`).
pub fn elevated(r: Roles, level: u8, corner: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.elevation_surface(level)))),
        text_color: Some(color(r.on_surface)),
        border: radius(corner),
        shadow: elevation_shadow(r, level),
        ..container::Style::default()
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

/// A raised Material surface (a card, or grouped content). Elevation 1: the `surface_container_low`
/// tone plus a shadow, and **no outline** — feature 003 drew a 1px border here to fake an edge, and
/// that is exactly what depth replaces (FR-002, FR-015).
pub fn surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    elevated(r, elevation::CARD, shape::MEDIUM)
}

/// A dialog surface. Elevation 3 and the extra-large 28dp corner — both a step up from feature
/// 003's 16dp and borrowed outline, and both what makes a dialog read as the frontmost thing on
/// screen (FR-018, FR-028).
pub fn dialog(r: Roles) -> impl Fn(&Theme) -> container::Style {
    elevated(r, elevation::DIALOG, shape::EXTRA_LARGE)
}

/// The sidebar panel. Elevation 1's tone, and **square corners**.
///
/// A deliberate departure from §3's `large` (16) assignment for "the sidebar panel": that size
/// suits an inset panel, and this sidebar is docked flush to the window edge and full height, where
/// a rounded corner would cut a notch out of the window rather than round a floating card. The tone
/// is the part that carries the hierarchy, and it is applied.
pub fn sidebar_surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    elevated(r, elevation::CARD, shape::NONE)
}

/// The app bar at rest — elevation 0, so the `surface` tone and no shadow (§4).
///
/// It gains elevation 2 when content scrolls under it (FR-025a); that transition is US4's work.
pub fn toolbar_surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    elevated(r, elevation::APP_BAR_REST, shape::NONE)
}

/// The app bar, flat at rest and raised once content passes beneath it (§7.1, FR-025a).
///
/// Two levels rather than a shadow toggled on: at elevation 2 the bar takes a *tonal* shift as well
/// as a shadow, which is what keeps the raise readable in the dark scheme where a shadow alone
/// barely registers (FR-016).
pub fn app_bar_surface(r: Roles, scrolled: bool) -> impl Fn(&Theme) -> container::Style {
    let level = if scrolled {
        elevation::APP_BAR_SCROLLED
    } else {
        elevation::APP_BAR_REST
    };
    elevated(r, level, shape::NONE)
}

/// A floating menu, context menu or popover. Elevation 2 and the extra-small 4dp corner (§3).
///
/// Level 2 sits below a dialog's 3 on purpose: a menu opened *over* a dialog must still read as
/// above it, which it does because the menu is drawn later in the overlay order while keeping its
/// own shadow (FR-017) — elevation grades the resting hierarchy, not the stacking order.
pub fn menu_surface(r: Roles) -> impl Fn(&Theme) -> container::Style {
    elevated(r, elevation::MENU, shape::EXTRA_SMALL)
}

/// The closed field of a `pick_list`-backed `Select` (feature 013): an outlined box matching
/// [`input`]'s look, with the outline switching to `primary` while hovered or open.
pub fn select_field(r: Roles) -> impl Fn(&Theme, pick_list::Status) -> pick_list::Style {
    move |_theme, status| pick_list::Style {
        text_color: color(r.on_surface),
        placeholder_color: color(r.on_surface_variant),
        handle_color: color(r.on_surface_variant),
        // No chrome of its own, exactly as [`field_input`]: the container and the active indicator
        // belong to `FormField`, and a select that kept its own box would draw a second outline
        // inside the filled container (FR-031c).
        //
        // Its open state used to be a 3dp `secondary` border. That affordance is gone with the
        // border — §7.7 gives a filled field a bottom indicator instead, and `FormField` draws it.
        //
        // But the indicator cannot answer for this control: `pick_list` reports `Opened` here and
        // to no parent, so `Select::active` has to be supplied by a caller that tracks openness,
        // and none does (FR-043a, accepted gap #3). Deleting the border and stopping there left
        // opening the list with *no* feedback at all, which is worse than the affordance it
        // replaced.
        //
        // So the state layer answers instead, which is what §5 asks of every interactive surface
        // anyway (FR-021): the content colour over the container at the state's opacity. It needs
        // no parent to know anything, and it composes with the indicator rather than standing in
        // for it.
        background: Background::Color(match status {
            pick_list::Status::Opened { .. } => state_fill(color(r.on_surface), state::PRESSED),
            pick_list::Status::Hovered => state_fill(color(r.on_surface), state::HOVER),
            pick_list::Status::Active => Color::TRANSPARENT,
        }),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
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
            radius: shape::EXTRA_SMALL.into(),
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

/// A result row inside a type-ahead menu (feature 021, contracts/typeahead-component.md §4.7).
///
/// Three things can be true of one row at the same time, so each gets its own channel and none can
/// hide another: the current *selection* is a tonal fill — Material's `secondary_container`, the
/// same treatment a selected list item carries — the *keyboard's* row is a state layer at the focus
/// opacity over whatever fill it already has, and the *pointer's* row is the same layer at hover or
/// pressed strength. Which characters *matched* is not here: the label colours those itself, so
/// emphasis survives on a filled row.
///
/// Every opacity comes from `tokens::state`, never from a literal. That is not tidiness — the first
/// draft of this function hardcoded `0.12` for pressed, which is the *selected* opacity, so a
/// pressed row and a selected one were indistinguishable. Feature 019 had already fixed exactly that
/// bug everywhere else; this function was written before those tokens existed and reintroduced it.
///
/// A row with nothing to press arrives here as `Disabled`. It keeps its fill and mutes only its
/// label, so it still reads as a line of the list rather than disappearing from it (FR-012).
pub fn menu_row(
    r: Roles,
    highlighted: bool,
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let (fill, on) = if selected {
            (
                Some(color(r.secondary_container)),
                color(r.on_secondary_container),
            )
        } else {
            (None, color(r.on_surface))
        };

        let opacity = match status {
            button::Status::Pressed => state::PRESSED,
            button::Status::Hovered => state::HOVER,
            // The keyboard's row reads at the focus opacity even with the pointer elsewhere — it is
            // where the keyboard is, which is what focus means.
            _ if highlighted => state::FOCUS,
            _ => 0.0,
        };

        let background = match fill {
            // Over an opaque fill the layer is composited, so the result stays opaque.
            Some(fill) if opacity > 0.0 => Some(Background::Color(state_layer(fill, on, opacity))),
            Some(fill) => Some(Background::Color(fill)),
            // Over nothing it stays translucent, so it works on whatever the menu surface is.
            None if opacity > 0.0 => Some(Background::Color(state_fill(on, opacity))),
            None => None,
        };

        button::Style {
            background,
            text_color: if matches!(status, button::Status::Disabled) {
                alpha(on, state::DISABLED_CONTENT)
            } else {
                on
            },
            border: radius(shape::SMALL),
            ..button::Style::default()
        }
    }
}

/// A known-projects / selector list row.
pub fn list_item(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface_variant))),
        text_color: Some(color(r.on_surface)),
        border: radius(shape::MEDIUM),
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
            border: radius(shape::MEDIUM),
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
            button::Status::Hovered => state_layer(base, on, state::HOVER),
            button::Status::Pressed => state_layer(base, on, state::PRESSED),
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
            border: radius(shape::FULL),
            ..button::Style::default()
        }
    }
}

/// Outlined button: transparent fill, `outline` border, `primary` label (secondary actions).
pub fn outlined(r: Roles) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let prim = color(r.primary);
        let fill = match status {
            button::Status::Hovered => Some(Background::Color(state_fill(prim, state::HOVER))),
            button::Status::Pressed => Some(Background::Color(state_fill(prim, state::PRESSED))),
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
                radius: shape::FULL.into(),
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
            button::Status::Hovered => Some(Background::Color(state_fill(prim, state::HOVER))),
            button::Status::Pressed => Some(Background::Color(state_fill(prim, state::PRESSED))),
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

/// A small icon-only button with a fully-rounded hit area (`IconButton::circular`) — same
/// hover/press fill and label color as [`text_button`], but a `shape::FULL` border radius so a
/// roughly-square button (small, uniform padding) reads as a circle around the glyph rather than
/// a rounded square.
pub fn circular_icon_button(r: Roles) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let prim = color(r.primary);
        let fill = match status {
            button::Status::Hovered => Some(Background::Color(state_fill(prim, state::HOVER))),
            button::Status::Pressed => Some(Background::Color(state_fill(prim, state::PRESSED))),
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
                radius: shape::SMALL.into(),
            },
            text_color: Some(color(r.on_surface)),
        }
    }
}

// --- the filled form field (feature 018, T044b/T045 — FR-031, FR-031a, FR-031c; contract §7.7) --

/// The filled field container: `surface_container_highest`, rounded at the top and square at the
/// bottom, with **no border at all**.
///
/// The squared bottom is not a stylistic flourish — it is what makes the active indicator read as
/// part of the field rather than as a line underneath it. A uniform radius would leave the
/// indicator's ends floating past the container's curve.
///
/// Today's field is `surface`, the same tone as the dialog behind it, inside a uniform 1dp box.
/// That is the largest single departure in this feature (§7.7).
pub fn field_container(r: Roles) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color(r.surface_container_highest))),
        text_color: Some(color(r.on_surface)),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: iced::border::Radius {
                top_left: shape::EXTRA_SMALL,
                top_right: shape::EXTRA_SMALL,
                bottom_right: 0.0,
                bottom_left: 0.0,
            },
        },
        ..container::Style::default()
    }
}

/// The bottom active indicator's colour and thickness, as one decision.
///
/// Returned together because they move together: the indicator is 1dp `on_surface_variant` at rest
/// and 2dp in the accent when the field is active, and an implementation that took them from two
/// places could thicken without recolouring. `error` outranks `active` — a field that is both
/// focused and invalid is invalid, and showing it in the accent would say the opposite.
///
/// "Active" is deliberately not "focused": it is **focus** for a text input and **open** for the
/// select, which cannot report focus at all (FR-043a). The wrapper is told which; it never assumes.
pub fn field_indicator(r: Roles, active: bool, error: bool) -> (Color, f32) {
    let thickness = if active {
        tokens::anatomy::text_field::INDICATOR_ACTIVE
    } else {
        tokens::anatomy::text_field::INDICATOR
    };
    let tone = if error {
        r.error
    } else if active {
        r.primary
    } else {
        r.on_surface_variant
    };
    (color(tone), thickness)
}

/// The in-container label and the supporting text beneath it, which share a colour: the muted
/// foreground, or `error` when the field is invalid.
///
/// One function rather than two, because §7.7 moves them together — an error state that recoloured
/// the supporting text and left the label muted would read as two unrelated pieces of text.
pub fn field_support(r: Roles, error: bool) -> Rgb {
    if error {
        r.error
    } else {
        r.on_surface_variant
    }
}

/// The input *inside* a [`field_container`]: no background and no border of its own.
///
/// The container and the indicator belong to `FormField`, so the input draws neither (FR-031c).
/// Leaving its old box in place would put a 1dp outline inside the filled container — the exact
/// duplication the wrapper exists to remove.
pub fn field_input(r: Roles) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_theme, _status| text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius: 0.0.into(),
        },
        icon: color(r.on_surface_variant),
        placeholder: color(r.on_surface_variant),
        value: color(r.on_surface),
        selection: alpha(color(r.primary), 0.3),
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
