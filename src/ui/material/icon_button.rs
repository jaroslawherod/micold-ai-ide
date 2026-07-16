//! `IconButton` — a reusable icon-only button primitive (Constitution Principle VIII).
//!
//! Theme-aware (tinted via a design-system color role) with a disabled state. Exposed as a
//! chainable builder terminating in `.into()` (Principle VIII builder-API rule): construct with
//! the required glyph + roles, then set optional size/tint/press before converting to an
//! `Element`. Reused across the sidebar and any icon action rather than each feature forking one.

use crate::ui::{icon, style};
use iced::widget::button;
use iced::Element;
use micold_ai_ide::icons::Icon;
use micold_ai_ide::tokens::{spacing, type_scale, Rgb, Roles};
use std::marker::PhantomData;

/// A compact icon-only button. Without an `on_press` it renders disabled (greyed, inert).
/// Styled as a low-emphasis text button so it sits unobtrusively in dense rows.
pub struct IconButton<'a, M> {
    glyph: Icon,
    roles: Roles,
    size: u16,
    tint: Option<Rgb>,
    on_press: Option<M>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> IconButton<'a, M> {
    /// Build an icon button for `glyph` themed by `roles`. Defaults: body-size, `on_surface`
    /// tint, no press action (disabled).
    pub fn new(glyph: Icon, roles: Roles) -> Self {
        Self {
            glyph,
            roles,
            size: type_scale::BODY,
            tint: None,
            on_press: None,
            _marker: PhantomData,
        }
    }

    /// Override the glyph size.
    pub fn size(mut self, size: u16) -> Self {
        self.size = size;
        self
    }

    /// Override the icon tint (defaults to the roles' `on_surface`).
    pub fn tint(mut self, tint: Rgb) -> Self {
        self.tint = Some(tint);
        self
    }

    /// Set the message emitted on press (enables the button).
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// Set the press message from an `Option` (disabled when `None`).
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.on_press = message;
        self
    }
}

impl<'a, M: Clone + 'a> From<IconButton<'a, M>> for Element<'a, M> {
    fn from(b: IconButton<'a, M>) -> Self {
        let tint = b.tint.unwrap_or(b.roles.on_surface);
        let content = icon(b.glyph, b.size, tint);
        let mut btn = button(content)
            .padding(spacing::XS)
            .style(style::text_button(b.roles));
        if let Some(message) = b.on_press {
            btn = btn.on_press(message);
        }
        btn.into()
    }
}
