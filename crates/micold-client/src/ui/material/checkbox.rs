//! `Checkbox` — the library's wrapper around the rendering stack's checkbox (Principle VIII).
//!
//! Two call sites, both identical, both naming the style function directly. Wrapped for the same
//! reason as the rest: a call site that can reach the style layer can render a checkbox that does
//! not match the other one, and the only thing preventing it is that nobody has yet.
//!
//! Parity: the style resolves to exactly what the call sites use today (FR-005).

use crate::ui::material::style;
use iced::widget::checkbox;
use iced::Element;
use micold_core::tokens::Roles;

/// A labelled checkbox. Builder form (Principle VIII):
/// `Checkbox::new("Enabled", draft.enabled, roles).on_toggle(Message::Toggled).into()`.
///
/// Without an `on_toggle` it renders disabled.
pub struct Checkbox<'a, M> {
    label: String,
    checked: bool,
    roles: Roles,
    on_toggle: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> Checkbox<'a, M> {
    /// A checkbox reading `label`, currently `checked`, themed by `roles`.
    pub fn new(label: impl Into<String>, checked: bool, roles: Roles) -> Self {
        Self {
            label: label.into(),
            checked,
            roles,
            on_toggle: None,
        }
    }

    /// The message emitted when the box is toggled, given the new state.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }
}

impl<'a, M: Clone + 'a> From<Checkbox<'a, M>> for Element<'a, M> {
    fn from(c: Checkbox<'a, M>) -> Self {
        let mut widget = checkbox(c.checked)
            .label(c.label)
            .style(style::checkbox(c.roles));
        if let Some(on_toggle) = c.on_toggle {
            widget = widget.on_toggle(on_toggle);
        }
        widget.into()
    }
}
