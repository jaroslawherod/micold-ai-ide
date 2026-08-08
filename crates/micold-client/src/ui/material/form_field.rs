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
//! text input, **open** for the select and the search picker's list. §7.7 asks for exactly that, and
//! both controls now answer it — the select from its own state, the search picker from its caller's
//! (feature 022, FR-013). The rendered result is identical and only the trigger differs, so the
//! wrapper is *told* which. A wrapper that assumed focus would leave both indicators at rest.
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
//! The label takes both of Material's positions — resting in the middle of an empty box at full
//! size, floating at the top in the smaller role once there is a value or the field is active —
//! but **snaps** between them rather than animating. See [`label_floats`] for how the two halves of
//! that decision are kept in agreement.
//!
//! The gap is the transition alone. Both endpoints are correct, and this narrowed once the field
//! became a widget with its own layout: the original note here recorded the label as permanently
//! floating, which matched Material's *populated* field and left an empty one with a caption
//! hanging over nothing.

use iced::widget::{column, container, Space};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, spacing, Roles};

use super::filled_field::{FilledField, State as FilledFieldState};
use super::style;
use super::{Text, TypeRole};

/// Whether the label has floated clear of the value's line.
///
/// One rule in one place, because two parties depend on it and must agree: `FormField` uses it to
/// decide where to put the label, and each *control* uses it to decide whether to draw a
/// placeholder at all. While the label rests it occupies the value's line, so a control that also
/// drew a placeholder there would print two words on top of each other — which is what an unset
/// select looked like the first time round. Disagreement is the failure mode, so there is only one
/// place to disagree with.
pub fn label_floats(populated: bool, active: bool) -> bool {
    populated || active
}

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
    populated: bool,
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
            populated: false,
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

    /// Whether the control has a value in it.
    ///
    /// Decides where the label sits. Material's filled field rests its label in the middle of an
    /// empty, unfocused box at full size, and *floats* it to the top at the smaller role once there
    /// is something to label — otherwise a name hangs above an empty box with nothing under it,
    /// which is what the field looked like before this existed.
    ///
    /// Supplied rather than inferred: the wrapper is handed an opaque control and cannot see
    /// whether a text input has text or a select has a selection. Each knows its own.
    pub fn populated(mut self, populated: bool) -> Self {
        self.populated = populated;
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
        // Floating once there is a value or the field is being used; resting otherwise. The role
        // changes with the position — `body_small` floating, `body_large` at rest — which is what
        // makes a resting label read as the field's *content placeholder* rather than as a caption.
        let floating = label_floats(f.populated, f.active);
        let label: Element<'a, M> = match f.label {
            Some(text) => Text::new(
                text,
                if floating {
                    TypeRole::Caption
                } else {
                    TypeRole::Body
                },
                r,
            )
            .tint(style::field_label(r, f.active, invalid))
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
            FilledFieldState {
                active: f.active,
                error: invalid,
                floating,
            },
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
