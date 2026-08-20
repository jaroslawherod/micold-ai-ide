//! `LabelledToggle` — a toggle that says what it is toggling (Principle VIII, feature 026 T066a).
//!
//! A glyph and a short word in one pressable control. The terminal bar's AI-CLI mode toggle uses it
//! to carry the session's CLI (`claude`, `copilot`) beside the mode glyph, so an open session names
//! its CLI without a trip back to the sidebar (FR-016a).
//!
//! # Why this is not `IconButton::label(…)`
//!
//! That was the obvious edit and it is the wrong one. [`IconButton`](super::IconButton) is not
//! icon-only by habit — its module contract says so, and `anatomy_size.rs` holds it to it in two
//! independent places: a disabled one *is sized by its glyph*, and a compact one *is sized by its
//! glyph, not by the room it is given*. A `label` method would make both of those statements
//! conditional on a caller, which is how a contract stops being one.
//!
//! So the label lives in a type whose whole subject is having one, composed from
//! [`IconLabel`](super::IconLabel) — the existing "a picture and its words" primitive — inside a
//! button. Two named things rather than one thing with a mode.
//!
//! # The label is the caller's word, not a description
//!
//! It takes a `&'static str` because every use is a fixed vocabulary word — a command name from the
//! provider seam. That is deliberate: a control whose label is arbitrary runtime text is a button,
//! and this is a toggle whose label names *which* thing it is toggling between.

use crate::icons::Icon;
use crate::ui::material::{style, IconLabel, TypeRole};
use iced::widget::button;
use iced::Element;
use micold_core::tokens::{spacing, Rgb, Roles};

/// A pressable glyph-plus-label toggle.
///
/// Builder form: `LabelledToggle::new(icon, "copilot", roles).on_press(msg).into()`.
pub struct LabelledToggle<M> {
    icon: Icon,
    label: &'static str,
    roles: Roles,
    role: TypeRole,
    on_press: Option<M>,
    tint: Option<Rgb>,
    padding: f32,
}

impl<M> LabelledToggle<M> {
    /// A toggle showing `icon` labelled `label`, themed by `roles`.
    ///
    /// Unpressable until [`Self::on_press`] is given one, matching every other control here: a
    /// caller that forgets the message gets an inert control rather than one that looks live.
    pub fn new(icon: Icon, label: &'static str, roles: Roles) -> Self {
        Self {
            icon,
            label,
            roles,
            role: TypeRole::Caption,
            on_press: None,
            tint: None,
            padding: spacing::SM,
        }
    }

    /// What pressing it emits.
    pub fn on_press(mut self, message: M) -> Self {
        self.on_press = Some(message);
        self
    }

    /// The type role the glyph and the label are both sized at.
    ///
    /// One role for both, which is [`IconLabel`]'s rule and not a shortcut: a labelled icon is one
    /// piece of text with a picture in front of it, so the picture matches the words. A button's
    /// *leading* icon is a different thing at a different size, and conflating them is a defect
    /// this codebase has already corrected twice.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Tint the glyph.
    pub fn tint(mut self, tint: Rgb) -> Self {
        self.tint = Some(tint);
        self
    }

    /// The inset around the content.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl<'a, M: Clone + 'a> From<LabelledToggle<M>> for Element<'a, M> {
    fn from(toggle: LabelledToggle<M>) -> Self {
        let mut content = IconLabel::<M>::new(toggle.icon, toggle.label, toggle.role, toggle.roles);
        if let Some(tint) = toggle.tint {
            content = content.tint(tint);
        }
        let mut element = button(Element::<M>::from(content))
            .padding(toggle.padding)
            .style(style::text_button(toggle.roles));
        if let Some(message) = toggle.on_press {
            element = element.on_press(message);
        }
        element.into()
    }
}
