//! `TextField` — the library's wrapper around the rendering stack's text input (Principle VIII).
//!
//! Every text field in the application is built the same way today — same padding, same style
//! function, an input handler and usually a submit handler — repeated at seven call sites. Nothing
//! held them together beyond the fact that whoever added the seventh copied the sixth.
//!
//! # Feature 021: the two affordances beside the input
//!
//! A leading icon saying what the field is for, and a trailing icon that *does* something. The
//! branch search needed both, and the alternative was assembling them at that one call site.
//!
//! # Feature 018: the chrome moved out (T045, T046 — FR-031, FR-031a, FR-031c)
//!
//! The container, the active indicator, the label and the supporting text belong to
//! [`FormField`](super::FormField). This builds the *input* and hands it over; it assembles neither
//! part itself, which is what FR-031c asks for and what stops the seven call sites each inventing
//! their own arrangement of the same four pieces.
//!
//! The label is therefore a field on this builder rather than something a call site puts above the
//! widget, and the placeholder goes back to being what §7.7 says it is: a genuine example, never
//! the field's name.

use crate::icons::Icon;
use crate::ui::material::style;
use crate::ui::material::text::TypeRole;
use iced::widget::text_input;
use iced::{Element, Pixels};
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
    label: Option<String>,
    supporting: Option<String>,
    error: Option<String>,
    active: bool,
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
            label: None,
            supporting: None,
            error: None,
            active: false,
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

    /// The field's name, rendered inside the container above the value (FR-031a).
    ///
    /// A *name*, not a hint: "Ticket", not "Ticket (optional, e.g. ABC-123)". The example belongs in
    /// [`Self::supporting`], and the two were bundled into the placeholder at every call site —
    /// the migration §7.7 tabulates.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Explanatory text beneath the container: a constraint, an example, a unit.
    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

    /// The validation failure, if any — takes the supporting text's place and recolours the chrome.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Whether the input holds focus, which thickens the active indicator.
    ///
    /// Supplied rather than observed: the rendering stack reports focus only inside the input's own
    /// style closure, and the indicator is a sibling of the input rather than part of it. See
    /// [`FormField`](super::FormField) for why the wrapper takes this as a parameter.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
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
        // No padding and no style of its own: `FormField` supplies the container that would
        // otherwise be drawn twice, and pads it to §7.7's 16dp.
        // `.padding(0)` is load-bearing: the rendering stack insets a text input by default, and
        // `FormField` has already padded the container to §7.7's 16dp — so the default puts the
        // *value* a few pixels right of the label sitting directly above it, which reads as a
        // misalignment rather than as a field.
        // While the label rests it *occupies* the value's line, so a placeholder there would sit
        // on top of it. Material shows one or the other, never both: the resting label is the
        // placeholder, and the real one appears once the label has floated out of the way.
        let resting =
            f.label.is_some() && !super::form_field::label_floats(!f.value.is_empty(), f.active);
        let placeholder = if resting { "" } else { &f.placeholder };
        let mut widget = text_input(placeholder, f.value)
            .padding(0)
            .style(style::field_input(f.roles));

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

        // The trailing action goes into the wrapper's adornment slot rather than into a row built
        // here. `FormField` emits that slot whether or not it is filled, which is what keeps the
        // input's position in the widget tree stable — see its docs for why that matters more than
        // it looks.
        let mut field = super::FormField::new(widget, f.roles)
            .active(f.active)
            // Focus shades the container as well as thickening the indicator (FR-035, BUG-002).
            // For a text input `active` *is* focus — see `FormField`'s docs on why the wrapper is
            // told rather than asking. Not `.pressable`: clicking a text field places a caret, and
            // a ripple would announce an action that did not happen.
            .layer(if f.active {
                super::form_field::Layer::Focused
            } else {
                super::form_field::Layer::None
            })
            // A field with text in it floats its label; an empty one rests it where the text will
            // go. Only the control knows which it is.
            .populated(!f.value.is_empty());
        if let Some((icon, message)) = f.trailing_action {
            field = field.trailing(super::IconButton::new(icon, f.roles).on_press(message));
        }
        if let Some(label) = f.label {
            field = field.label(label);
        }
        if let Some(supporting) = f.supporting {
            field = field.supporting(supporting);
        }
        field.error(f.error).into()
    }
}
