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
use micold_core::required::FieldRequirement;
use micold_core::tokens::{anatomy, shape, spacing, Roles};

pub use super::filled_field::Layer;
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

/// The three strings every field carries: its name, its hint, its validation failure.
///
/// One struct so the builders exist once ([`field_text_builders!`]) instead of being pasted on
/// each field type, and so a type wrapping another passes the trio through whole.
#[derive(Clone, Default)]
pub struct FieldText {
    label: Option<String>,
    supporting: Option<String>,
    error: Option<String>,
    required: FieldRequirement,
}

impl FieldText {
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn supporting(mut self, text: impl Into<String>) -> Self {
        self.supporting = Some(text.into());
        self
    }

    pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
        self.error = error.map(Into::into);
        self
    }

    pub fn required(mut self, required: FieldRequirement) -> Self {
        self.required = required;
        self
    }

    pub fn has_label(&self) -> bool {
        self.label.is_some()
    }

    /// Hand the trio to a [`FormField`].
    pub fn apply<'a, M: 'a>(self, mut field: FormField<'a, M>) -> FormField<'a, M> {
        field.text = self;
        field
    }
}

/// The `label`/`supporting` builders on a type holding a `text: FieldText`.
#[macro_export]
macro_rules! field_text_builders {
    () => {
        $crate::form_field::field_text_builders!(label_supporting);

        /// The validation failure, if any: takes the supporting text's place and recolours the
        /// chrome.
        pub fn error(mut self, error: Option<impl Into<String>>) -> Self {
            self.text = self.text.error(error);
            self
        }
    };
    (label_supporting) => {
        /// The field's name, rendered inside its container above the value (FR-031a).
        ///
        /// A *name*, not a hint ("Ticket", not "Ticket (optional, e.g. ABC-123)"): the example
        /// belongs in [`Self::supporting`].
        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.text = self.text.label(label);
            self
        }

        /// Explanatory text beneath the container: a constraint, an example, a unit.
        pub fn supporting(mut self, text: impl Into<String>) -> Self {
            self.text = self.text.supporting(text);
            self
        }

        /// Whether the form refuses this field empty, and whether to say so now (feature 937,
        /// contract C1): `Required` and `Missing` append the marker to the label.
        pub fn required(mut self, required: micold_core::required::FieldRequirement) -> Self {
            self.text = self.text.required(required);
            self
        }
    };
}
pub use crate::field_text_builders;

/// The row under a field's container: `text` inset to the container's text column, or an empty
/// `Space` when there is none, so the widget tree has the same shape either way (see the module
/// docs). `top` is the gap above it.
pub fn beneath<'a, M: 'a>(text: Option<Element<'a, M>>, top: f32) -> Element<'a, M> {
    match text {
        Some(text) => container(text)
            .padding(iced::Padding {
                top,
                bottom: 0.0,
                left: anatomy::text_field::PADDING,
                right: anatomy::text_field::PADDING,
            })
            .into(),
        None => Space::new().into(),
    }
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
    text: FieldText,
    active: bool,
    populated: bool,
    leading: Option<Element<'a, M>>,
    trailing: Option<Element<'a, M>>,
    layer: Layer,
    pressable: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: 'a> FormField<'a, M> {
    /// Wrap `control` in the shared chrome.
    pub fn new(control: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            control: control.into(),
            roles,
            text: FieldText::default(),
            active: false,
            populated: false,
            leading: None,
            trailing: None,
            layer: Layer::None,
            pressable: false,
            on_focus_change: None,
        }
    }

    /// Report the wrapped control gaining or losing the keyboard (BUG-003).
    ///
    /// The other half of [`Self::active`], and the half that was missing: `active` is *supplied*
    /// for the reasons in the module docs, and until this existed there was no way for a caller to
    /// find out what to supply. A control that cannot take focus never fires it.
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }

    field_text_builders!();

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

    /// The state layer the *control* reports — focus for a text input, open for the select
    /// (FR-035, BUG-002).
    ///
    /// Hover is not among the choices and cannot be: the container reads it off the cursor when it
    /// draws, so there is nowhere for a caller to disagree with it. What is left here is the state
    /// only the control knows, and it is one value rather than a set of flags so that
    /// open-and-focused resolves to a single layer instead of two stacked ones.
    pub fn layer(mut self, layer: Layer) -> Self {
        self.layer = layer;
        self
    }

    /// Whether pressing the container is a *press* — true for the select, false for a text input.
    ///
    /// It buys the ripple every pressable surface in the app produces (FR-010), over the whole
    /// container rather than over whatever sits in its control slot. A text field is not pressable
    /// in this sense: clicking it places a caret, and rippling for that would announce an action
    /// that did not happen.
    pub fn pressable(mut self, pressable: bool) -> Self {
        self.pressable = pressable;
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
        // A required field left or submitted empty is drawn invalid with the required message in
        // the supporting slot, unless the field has an error of its own, which wins (feature 937,
        // FR-005, FR-010). Filling the field turns `Missing` back into `Required` in the same
        // update, and only these two things change (FR-007).
        let missing = f.text.required == FieldRequirement::Missing;
        let invalid = f.text.error.is_some() || missing;

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
        // The required marker is a character of the label itself, so it floats and rests with
        // it, in its role and tint, and survives being read as text (feature 937, FR-001, FR-003).
        let marked = f.text.required != FieldRequirement::Optional;
        let label: Element<'a, M> = match f.text.label {
            Some(text) => Text::new(
                if marked {
                    format!("{text}{}", micold_core::required::MARKER)
                } else {
                    text
                },
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
        let field = FilledField::new(
            slot(f.leading),
            f.control,
            slot(f.trailing),
            label,
            r,
            FilledFieldState {
                active: f.active,
                error: invalid,
                floating,
                layer: f.layer,
            },
        );
        let field: Element<'a, M> = match f.on_focus_change {
            Some(on_focus_change) => field.on_focus_change(on_focus_change).into(),
            None => field.into(),
        };
        // The ripple wraps the **container**, not the control inside it (FR-010, FR-034). Wrapping
        // the control is what BUG-002 found: a press in the 16dp padding opened the select and
        // rippled nothing, because the two were reading different rectangles. `Ripple` is
        // layout-transparent, so this adds no node and moves nothing.
        let field = if f.pressable {
            super::Ripple::new(field, r.on_surface, shape::EXTRA_SMALL).into()
        } else {
            field
        };

        // Supporting text sits *beneath* the box, outside it. The error message replaces it rather
        // than joining it: a field showing a problem should not also be showing the hint the
        // problem replaces.
        let beneath = beneath(
            f.text
                .error
                .or(missing.then(|| micold_core::required::MESSAGE.to_string()))
                .or(f.text.supporting)
                .map(|message| {
                    Text::new(message, TypeRole::Caption, r)
                        .tint(style::field_support(r, invalid))
                        .into()
                }),
            spacing::XS,
        );

        column![field, beneath].width(Length::Fill).into()
    }
}
