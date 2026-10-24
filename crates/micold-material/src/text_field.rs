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

use crate::form_field::{field_text_builders, FieldText};
use crate::icons::Icon;
use crate::style;
use crate::text::TypeRole;
use iced::widget::text_input;
use iced::Element;
use micold_core::tokens::Roles;

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
    text: FieldText,
    active: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
    secure: bool,
    id: Option<iced::widget::Id>,
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
            text: FieldText::default(),
            active: false,
            on_focus_change: None,
            secure: false,
            id: None,
        }
    }

    /// Name the input, so an operation can focus it by id.
    pub fn id(mut self, id: iced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }

    /// An icon inside the field's leading edge, saying what the field is for — Material's leading
    /// icon slot. Decorative: it cannot be pressed.
    ///
    /// It goes into [`FormField`](super::FormField)'s own leading slot rather than into the input's
    /// icon slot (BUG-003 item 1). An icon the input draws is one the *field* cannot see, and the
    /// field is what positions the label — so the label stayed at the container's padding while the
    /// value moved past the icon, and the two collided in every empty, unfocused search box. The
    /// module docs of `filled_field` state the rule this restores: the input is the editing engine
    /// and decides where nothing sits.
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

    field_text_builders!();

    /// Take the name, hint and failure from `text` whole (what a wrapping control passes through).
    pub fn text(mut self, text: FieldText) -> Self {
        self.text = text;
        self
    }

    /// Whether the input holds focus, which floats the label, thickens the active indicator and
    /// shades the container.
    ///
    /// Supplied rather than observed: the rendering stack reports focus only inside the input's own
    /// style closure, and the indicator is a sibling of the input rather than part of it. See
    /// [`FormField`](super::FormField) for why the wrapper takes this as a parameter.
    ///
    /// What a caller supplies here comes back from [`Self::on_focus_change`]. Setting one without
    /// the other is BUG-003 — a field permanently at rest, or one that reports a fact nobody keeps.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The message emitted when the input takes or loses the keyboard (BUG-003).
    ///
    /// The source of the flag [`Self::active`] wants back. Two calls rather than one because the
    /// application decides *where* the fact lives — one field per form, or one for the whole app —
    /// and this library has no business knowing which.
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }

    /// The message emitted as the user types. Omit it and the field is read-only.
    pub fn on_input(mut self, f: impl Fn(String) -> M + 'a) -> Self {
        self.on_input = Some(Box::new(f));
        self
    }

    /// Mask the value, for a secret such as an API token (feature 533, FR-009).
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
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
            f.text.has_label() && !super::form_field::label_floats(!f.value.is_empty(), f.active);
        let placeholder = if resting { "" } else { &f.placeholder };
        let mut widget = text_input(placeholder, f.value)
            .padding(0)
            .secure(f.secure)
            .style(style::field_input(f.roles));

        if let Some(id) = f.id {
            widget = widget.id(id);
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
        if let Some(on_focus_change) = f.on_focus_change {
            field = field.on_focus_change(on_focus_change);
        }
        // Tinted to match what the input's own icon slot used to draw it as (`style::field_input`),
        // so moving it out of that slot changed where it sits and nothing else.
        if let Some(icon) = f.leading_icon {
            field = field.leading(
                super::Glyph::new(icon, TypeRole::Action, f.roles).tint(f.roles.on_surface_variant),
            );
        }
        if let Some((icon, message)) = f.trailing_action {
            field = field.trailing(super::IconButton::new(icon, f.roles).on_press(message));
        }
        f.text.apply(field).into()
    }
}
