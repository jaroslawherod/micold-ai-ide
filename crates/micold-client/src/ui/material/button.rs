//! `Button` — the library's wrapper around the rendering stack's button (Principle VIII).
//!
//! Three variants are in use today, and every call site picked one by naming a style function
//! directly. That is the leak this feature closes: a call site that can name `style::filled` can
//! also name `style::outlined` for a confirm action, or forget the style entirely, and nothing
//! stops it. A call site now names the *variant* and cannot reach the style layer at all.
//!
//! Icon-only buttons keep their own component ([`IconButton`](super::IconButton)) — they carry a
//! glyph rather than a label, and a disabled glyph needs colouring the label path does not.
//!
//! Parity: each variant resolves to exactly the style its call sites use today (FR-005).

use crate::ui::material::style;
use crate::ui::material::text::{Text, TypeRole};
use iced::widget::button;
use iced::{Element, Length, Padding};
use micold_core::tokens::Roles;

/// Each `impl Fn` returned by the style layer is a distinct opaque type, so the variants are boxed
/// behind one signature to be chosen at runtime.
type ButtonStyleFn = Box<dyn Fn(&iced::Theme, button::Status) -> button::Style>;

/// How much emphasis the button carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// The primary action of a screen or dialog. One per group.
    Filled,
    /// A secondary action of equal standing — cancel beside confirm.
    Outlined,
    /// A low-emphasis action that sits inside other content: a menu entry, a list row, a tab.
    Text,
}

impl Variant {
    /// The content colour this variant draws its label in — and therefore the colour its ripple
    /// takes, since a state layer is the content colour over the container (contract §5).
    fn content(self, roles: Roles) -> micold_core::tokens::Rgb {
        match self {
            Variant::Filled => roles.on_primary,
            Variant::Outlined | Variant::Text => roles.primary,
        }
    }

    fn style(self, roles: Roles) -> ButtonStyleFn {
        match self {
            Variant::Filled => Box::new(style::filled(roles)),
            Variant::Outlined => Box::new(style::outlined(roles)),
            Variant::Text => Box::new(style::text_button(roles)),
        }
    }
}

/// A labelled button. Builder form (Principle VIII):
/// `Button::filled("Create", roles).on_press(Message::Create).into()`.
///
/// Without an `on_press` the button renders disabled, matching the rendering stack's own rule —
/// so "this action is unavailable" is expressed by having no message to send, not by a flag that
/// could disagree with one.
pub struct Button<'a, M> {
    content: Element<'a, M>,
    variant: Variant,
    roles: Roles,
    on_press: Option<M>,
    padding: Option<Padding>,
    width: Option<Length>,
}

impl<'a, M: Clone + 'a> Button<'a, M> {
    /// The primary action, carrying `label` at the body role.
    pub fn filled(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Filled, roles)
    }

    /// A secondary action of equal standing, carrying `label` at the body role.
    pub fn outlined(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Outlined, roles)
    }

    /// A low-emphasis action, carrying `label` at the body role.
    pub fn text(label: impl Into<String>, roles: Roles) -> Self {
        Self::labelled(label, Variant::Text, roles)
    }

    /// A button wrapping arbitrary `content` — a row of icon plus label, a tag chip, a tree row.
    /// The variant still decides the appearance; only what sits inside differs.
    pub fn with_content(
        content: impl Into<Element<'a, M>>,
        variant: Variant,
        roles: Roles,
    ) -> Self {
        Self {
            content: content.into(),
            variant,
            roles,
            on_press: None,
            padding: None,
            width: None,
        }
    }

    fn labelled(label: impl Into<String>, variant: Variant, roles: Roles) -> Self {
        let label: Element<'a, M> = Text::new(label.into(), TypeRole::Action, roles).into();
        Self::with_content(label, variant, roles)
    }

    /// The message emitted on press. Omit it and the button is disabled.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The press message from an `Option` — disabled when `None`. For an action that is available
    /// only in some states, so the call site expresses the condition once.
    pub fn on_press_maybe(mut self, message: Option<M>) -> Self {
        self.on_press = message;
        self
    }

    /// Override the button's padding.
    ///
    /// **A parity affordance, not a design decision.** Today's call sites use four different
    /// paddings for the same variant, and reproducing that exactly is what makes this feature
    /// reviewable. Feature 018 assigns each variant a height from the density scale and this step
    /// goes away with the last caller.
    pub fn padding(mut self, padding: impl Into<Padding>) -> Self {
        self.padding = Some(padding.into());
        self
    }

    /// Lay the button out at a given width — `Length::Fill` for a full-width list row.
    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = Some(width.into());
        self
    }
}

impl<'a, M: Clone + 'a> From<Button<'a, M>> for Element<'a, M> {
    fn from(b: Button<'a, M>) -> Self {
        let mut widget = button(b.content).style(b.variant.style(b.roles));
        if let Some(padding) = b.padding {
            widget = widget.padding(padding);
        }
        if let Some(width) = b.width {
            widget = widget.width(width);
        }
        let pressable = b.on_press.is_some();
        if let Some(message) = b.on_press {
            widget = widget.on_press(message);
        }
        // Wrapping is the opt-in: every `Button` ripples without any call site asking (FR-024c).
        //
        // Except a disabled one. A button with no `on_press` cannot be pressed, and a ripple on it
        // would report a press that will never happen — worse than no feedback, because it says the
        // opposite of what the disabled styling says.
        if pressable {
            super::Ripple::new(widget, b.variant.content(b.roles)).into()
        } else {
            widget.into()
        }
    }
}
