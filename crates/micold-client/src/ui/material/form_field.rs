//! `FormField` — the shared chrome around a form control (feature 018, T044b — FR-031a, FR-031b,
//! FR-031c; contract §7.7, component-api §2.1).
//!
//! On the model of Angular Material's form field, the precedent this library already mimics. It
//! **wraps whichever control it is given** rather than replacing it: a text input, a select, or
//! anything else a form grows later.
//!
//! # The division of labour is the requirement
//!
//! This owns the filled container, the bottom active indicator, the in-container label, the
//! supporting-text slot, the error presentation and the optional adornment slots. The wrapped
//! control owns its own input behaviour and **nothing else** — it draws no container, no indicator,
//! no label and no supporting text.
//!
//! That split is FR-031c, and it exists because the seven input call sites each assembled their own
//! version of it. A label above the field as muted text, a hint folded into the placeholder, an
//! error line pushed in beside the input: seven arrangements of the same four parts, none of them
//! what Material specifies, all of them a separate edit.
//!
//! # Why "active" is a parameter and not a question this asks
//!
//! The state that thickens the indicator and takes the accent differs by control: **focus** for a
//! text input, **open** for the select, which cannot report focus at all (FR-043a). The rendered
//! result is identical and only the trigger differs, so the wrapper is *told* which. A wrapper that
//! assumed focus would leave the select's indicator permanently at rest.
//!
//! # Every slot is always emitted, filled or not
//!
//! The label, the adornments and the supporting text are rendered whether or not they have
//! anything in them — an unfilled slot is a zero-sized element rather than an absent one.
//!
//! That is not tidiness. The rendering stack rebuilds a subtree whose tag changed, and a text
//! input's tag carries its own state, focus included. A field that gained a child the moment a
//! validation error appeared would rebuild the input *while the user was typing into it* and drop
//! the focus, so the next keystroke would go nowhere. Feature 021 hit exactly this with a search
//! field whose clear button appeared on the first keystroke, and its answer — one shape for both
//! cases, so the difference is unrepresentable — is the answer here too.
//!
//! A zero-sized slot still costs nothing visually: `an_empty_slot_takes_no_space` measures that,
//! and `the_shape_is_stable_whatever_the_slots_hold` measures the other half.
//!
//! # Accepted fidelity gap #4 (FR-044)
//!
//! The label is composed alongside the input and rendered **persistently in its floating position**,
//! not animated between resting and floating. The rendering stack's text input has no label concept
//! to transition. The result matches Material's *populated* field exactly; only the transition is
//! absent.

use iced::widget::{column, container, Space};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, spacing, Roles};

use super::filled_field::FilledField;
use super::style;
use super::{Text, TypeRole};

/// A form control wearing Material's filled-field chrome. Builder form (Principle VIII):
///
/// ```ignore
/// FormField::new(input, roles)
///     .label("Branch name")
///     .supporting("Lowercase only")
///     .error(Some("Already exists"))
///     .active(is_focused)
///     .into()
/// ```
pub struct FormField<'a, M> {
    control: Element<'a, M>,
    roles: Roles,
    label: Option<String>,
    supporting: Option<String>,
    error: Option<String>,
    active: bool,
    leading: Option<Element<'a, M>>,
    trailing: Option<Element<'a, M>>,
}

impl<'a, M: 'a> FormField<'a, M> {
    /// Wrap `control` in the shared chrome.
    pub fn new(control: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            control: control.into(),
            roles,
            label: None,
            supporting: None,
            error: None,
            active: false,
            leading: None,
            trailing: None,
        }
    }

    /// The field's name, rendered **inside** the container above the value.
    ///
    /// This is the field's name, never a hint or an example — those are [`Self::supporting`]. The
    /// placeholder is for a genuine example only (§7.7), which is the distinction today's call
    /// sites collapse by bundling both into one string.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Explanatory text beneath the container: a constraint, an example, a unit.
    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

    /// The validation failure, if any.
    ///
    /// Takes the place of the supporting text and switches the indicator and the label to the error
    /// role together, because a field showing a problem should not also be showing the hint the
    /// problem replaces.
    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    /// Whether the control is currently active — **focused** for a text input, **open** for the
    /// select. See the module docs for why this is supplied rather than inferred.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// An adornment before the control, inside the container.
    pub fn leading(mut self, element: impl Into<Element<'a, M>>) -> Self {
        self.leading = Some(element.into());
        self
    }

    /// An adornment after the control, inside the container.
    pub fn trailing(mut self, element: impl Into<Element<'a, M>>) -> Self {
        self.trailing = Some(element.into());
        self
    }
}

impl<'a, M: 'a> From<FormField<'a, M>> for Element<'a, M> {
    fn from(f: FormField<'a, M>) -> Self {
        let r = f.roles;
        let invalid = f.error.is_some();

        // Every slot is emitted whether or not it is filled — see the module docs. `Space::new()`
        // and not a zero-sized one: iced drops a child whose size hint `is_void()`, which is true
        // the moment either dimension is `Fixed(0)`, so an explicitly empty placeholder is deleted
        // and takes the stability it was added for with it.
        let slot = |content: Option<Element<'a, M>>| -> Element<'a, M> {
            content.unwrap_or_else(|| Space::new().into())
        };
        let label: Element<'a, M> = match f.label {
            Some(text) => Text::new(text, TypeRole::Caption, r)
                .tint(style::field_support(r, invalid))
                .into(),
            None => Space::new().into(),
        };

        // The box itself is a widget, not a stack of containers: §7.7's internal geometry is fixed
        // (8 + 16 + 24 + 8 = 56) and a column distributes leftover space it does not have. See
        // `filled_field.rs` for what composing it looked like and why it did not read as Material.
        let field: Element<'a, M> = FilledField::new(
            slot(f.leading),
            f.control,
            slot(f.trailing),
            label,
            r,
            f.active,
            invalid,
        )
        .into();

        // Supporting text sits *beneath* the box, outside it. The error message replaces it rather
        // than joining it: a field showing a problem should not also be showing the hint the
        // problem replaces.
        let beneath: Element<'a, M> = match f.error.or(f.supporting) {
            Some(message) => container(
                Text::new(message, TypeRole::Caption, r).tint(style::field_support(r, invalid)),
            )
            .padding(iced::Padding {
                top: spacing::XS,
                bottom: 0.0,
                left: anatomy::text_field::PADDING,
                right: anatomy::text_field::PADDING,
            })
            .into(),
            None => Space::new().into(),
        };

        column![field, beneath].width(Length::Fill).into()
    }
}
