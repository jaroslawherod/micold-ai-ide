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
//!
//! Feature 021 adds the two affordances Material's text-field anatomy names beside the input — a
//! leading icon and a trailing action — because the branch search needed both and the alternative
//! was assembling them at that one call site, which is the drift this module exists to prevent.

use crate::icons::Icon;
use crate::ui::material::style;
use crate::ui::material::text::TypeRole;
use iced::widget::{row, text_input};
use iced::{alignment, Element, Pixels};
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
    leading_icon: Option<Icon>,
    trailing_action: Option<(Icon, M)>,
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
            leading_icon: None,
            trailing_action: None,
            on_input: None,
            on_submit: None,
        }
    }

    /// An icon inside the field's leading edge, saying what the field is for — Material's leading
    /// icon slot. Decorative: it is drawn in the input's own icon slot and cannot be pressed.
    pub fn leading_icon(mut self, icon: Icon) -> Self {
        self.leading_icon = Some(icon);
        self
    }

    /// A pressable icon at the field's trailing edge — Material's trailing icon slot, which unlike
    /// the leading one is an action (clearing the field, revealing a password).
    pub fn trailing_action(mut self, icon: Icon, message: M) -> Self {
        self.trailing_action = Some((icon, message));
        self
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

        if let Some(icon) = f.leading_icon {
            widget = widget.icon(text_input::Icon {
                font: super::glyph::MATERIAL_SYMBOLS,
                code_point: icon.glyph(),
                size: Some(Pixels(TypeRole::Action.size())),
                spacing: spacing::XS,
                side: text_input::Side::Left,
            });
        }
        if let Some(on_input) = f.on_input {
            widget = widget.on_input(on_input);
        }
        if let Some(on_submit) = f.on_submit {
            widget = widget.on_submit(on_submit);
        }

        // **Always a row**, even with nothing in the trailing slot.
        //
        // A field that returned a bare input without a trailing action and a row with one would
        // change the *type* of the widget at that position the moment a caller started or stopped
        // offering the action. The rendering stack rebuilds a subtree whose tag changed, and the
        // input's tag carries its own state — focus included. A search field whose clear button
        // appears on the first keystroke would lose focus on that keystroke, so the second one
        // would never arrive. One shape for both cases makes that unrepresentable rather than a
        // rule each caller has to know.
        let mut field = row![widget]
            .spacing(spacing::XS)
            .align_y(alignment::Vertical::Center);
        if let Some((icon, message)) = f.trailing_action {
            field = field.push(super::IconButton::new(icon, f.roles).on_press(message));
        }
        field.into()
    }
}
