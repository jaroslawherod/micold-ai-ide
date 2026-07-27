//! `TextField` — the library's wrapper around the rendering stack's text input (Principle VIII).
//!
//! Every text field in the application is built the same way today — same padding, same style
//! function, an input handler and usually a submit handler — repeated at seven call sites. Nothing
//! held them together beyond the fact that whoever added the seventh copied the sixth.
//!
//! Holding them together matters more than usual here: feature 018 gives text fields a container,
//! a floating label and a focus indicator, which is a change to the *anatomy* of the widget rather
//! than to its colours. That is one edit if there is one text field, and seven if there are seven.
//!
//! Parity: padding and style resolve to exactly what the call sites use today (FR-005).

use crate::ui::material::style;
use iced::widget::text_input;
use iced::Element;
use micold_core::tokens::{spacing, Roles};

/// A single-line text field. Builder form (Principle VIII):
/// `TextField::new("Project name", &draft.text, roles).on_input(Message::Changed).into()`.
///
/// Without an `on_input` the field renders disabled — the same rule the button follows, for the
/// same reason: unavailability is expressed by having nowhere to send the value.
pub struct TextField<'a, M> {
    placeholder: String,
    value: &'a str,
    roles: Roles,
    on_input: Option<Box<dyn Fn(String) -> M + 'a>>,
    on_submit: Option<M>,
}

impl<'a, M: Clone + 'a> TextField<'a, M> {
    /// A field showing `value`, prompting with `placeholder`, themed by `roles`.
    pub fn new(placeholder: impl Into<String>, value: &'a str, roles: Roles) -> Self {
        Self {
            placeholder: placeholder.into(),
            value,
            roles,
            on_input: None,
            on_submit: None,
        }
    }

    /// The message emitted as the user types. Omit it and the field is read-only.
    pub fn on_input(mut self, f: impl Fn(String) -> M + 'a) -> Self {
        self.on_input = Some(Box::new(f));
        self
    }

    /// The message emitted when the user presses Enter — for a field whose dialog has an obvious
    /// primary action.
    pub fn on_submit(mut self, message: M) -> Self {
        self.on_submit = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<TextField<'a, M>> for Element<'a, M> {
    fn from(f: TextField<'a, M>) -> Self {
        let mut widget = text_input(&f.placeholder, f.value)
            .padding(spacing::SM)
            .style(style::input(f.roles));
        if let Some(on_input) = f.on_input {
            widget = widget.on_input(on_input);
        }
        if let Some(on_submit) = f.on_submit {
            widget = widget.on_submit(on_submit);
        }
        widget.into()
    }
}
