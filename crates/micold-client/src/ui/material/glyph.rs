//! `Glyph` — an icon at a type role (Principle VIII).
//!
//! Icons are text: the same font machinery, the same sizing, the same colour inheritance. So they
//! have the same problem the rest of the text had — every call site picked a number, and the number
//! was `type_scale::BODY` or `type_scale::LABEL` depending on who wrote it.
//!
//! A call site names the role it wants the glyph to match, and the tint it should carry. The tint
//! is still supplied by the caller because *which* semantic role an icon takes (an app-bar action,
//! a badge, an unavailable marker) is a decision about meaning, resolved by
//! [`icon_role`](crate::icons::icon_role) — not something the glyph can infer from itself.

use std::marker::PhantomData;

use crate::icons::Icon;
use crate::ui::material::style;
use crate::ui::material::text::TypeRole;
use iced::{Element, Font};
use micold_core::tokens::{Rgb, Roles};

/// The embedded Material Symbols (Outlined) icon font. Registered once at startup in `main` so
/// every icon glyph resolves; see `assets/fonts/PROVENANCE.md`.
pub const MATERIAL_SYMBOLS_BYTES: &[u8] =
    include_bytes!("../../../../../assets/fonts/MaterialSymbolsOutlined.ttf");

/// The font family the embedded icon file advertises (asserted by `tests/icons_font.rs`).
pub const MATERIAL_SYMBOLS: Font = Font::with_name("Material Symbols Outlined");

/// Draw `icon` at `size`, tinted with a foreground colour role (FR-004). Goes through
/// [`style::color`] so the tint follows the active theme exactly like all other text, giving
/// light/dark for free (FR-007).
///
/// The low-level primitive [`Glyph`] is built on. Prefer `Glyph`, which names a type role instead
/// of a number; this stays public for the components that already resolve a size themselves.
pub fn icon<'a, M: 'a>(icon: Icon, size: f32, color: Rgb) -> Element<'a, M> {
    icon_colored(icon, size, style::color(color))
}

/// [`icon`] with an already-resolved colour, so callers can apply alpha — notably
/// [`style::disabled_color`], since a glyph that colours itself does not inherit a disabled
/// button's `text_color`.
pub fn icon_colored<'a, M: 'a>(icon: Icon, size: f32, color: iced::Color) -> Element<'a, M> {
    iced::widget::text(icon.glyph().to_string())
        .font(MATERIAL_SYMBOLS)
        .size(size)
        .color(color)
        .into()
}

/// An icon glyph sized to a type role. Builder form (Principle VIII):
/// `Glyph::new(Icon::Git, TypeRole::Label, roles).tint(badge).into()`.
pub struct Glyph<'a, M> {
    icon: Icon,
    role: TypeRole,
    roles: Roles,
    tint: Option<Rgb>,
    disabled: bool,
    marker: PhantomData<&'a M>,
}

impl<'a, M: 'a> Glyph<'a, M> {
    /// `icon` drawn at `role`'s size, themed by `roles`. Tinted `on_surface` unless told otherwise.
    pub fn new(icon: Icon, role: TypeRole, roles: Roles) -> Self {
        Self {
            icon,
            role,
            roles,
            tint: None,
            disabled: false,
            marker: PhantomData,
        }
    }

    /// Tint the glyph with a resolved colour role — normally the output of
    /// [`icon_role`](crate::icons::icon_role), which maps an icon's *purpose* to a colour.
    pub fn tint(mut self, tint: Rgb) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Draw the glyph at the disabled opacity.
    ///
    /// Needed as an explicit step rather than inherited: a glyph sets its own colour, which
    /// overrides whatever `text_color` a disabled parent would have handed down, so an unmarked
    /// glyph inside a disabled control would otherwise render at full strength.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl<'a, M: 'a> From<Glyph<'a, M>> for Element<'a, M> {
    fn from(g: Glyph<'a, M>) -> Self {
        let tint = g.tint.unwrap_or(g.roles.on_surface);
        let size = g.role.size();
        if g.disabled {
            icon_colored(g.icon, size, style::disabled_color(tint))
        } else {
            icon(g.icon, size, tint)
        }
    }
}
