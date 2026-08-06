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

use iced::widget::{column, container, row, Space};
use iced::{Element, Length};
use micold_core::tokens::{anatomy, density, spacing, Roles};

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

        // Both adornment slots are always present, an empty one as a bare `Space`. See the module
        // docs for why the slot must exist even when empty — and note that the placeholder is
        // `Space::new()` rather than an explicitly zero-sized one: iced's `Column::push` and
        // `Row::push` *drop* any child whose size hint `is_void()`, which is true the moment either
        // dimension is `Fixed(0)`. A zero-sized placeholder is therefore deleted outright, which is
        // exactly the shape change it was added to prevent. A `Shrink` space lays out at zero and
        // survives. `spacing` is deliberately absent, so an empty slot adds no gap either.
        let slot = |content: Option<Element<'a, M>>| -> Element<'a, M> {
            content.unwrap_or_else(|| Space::new().into())
        };
        let line =
            row![slot(f.leading), f.control, slot(f.trailing)].align_y(iced::Alignment::Center);

        // The label sits above the value, inside the container, always in its floating position
        // (FR-044). `Caption` is `body_small`, the role §7.7 gives it. Emitted even when there is
        // no label, for the same tree-stability reason as the adornment slots.
        let label: Element<'a, M> = match f.label {
            Some(label) => Text::new(label, TypeRole::Caption, r)
                .tint(style::field_support(r, invalid))
                .into(),
            None => Space::new().into(),
        };
        let inner: Element<'a, M> = column![label, line].into();

        let filled = container(inner)
            .width(Length::Fill)
            .height(Length::Fixed(density::height(
                density::TEXT_FIELD_BASE,
                density::STANDARD,
            )))
            .padding(iced::Padding {
                top: spacing::SM,
                bottom: spacing::SM,
                left: anatomy::text_field::PADDING,
                right: anatomy::text_field::PADDING,
            })
            .style(style::field_container(r));

        // The active indicator: the bottom edge of the container, and the whole of the field's focus
        // affordance — there is no border here to recolour.
        let (indicator_color, indicator_thickness) = style::field_indicator(r, f.active, invalid);
        let indicator = container(Space::new().width(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fixed(indicator_thickness))
            .style(move |_theme: &iced::Theme| container::Style {
                background: Some(iced::Background::Color(indicator_color)),
                ..container::Style::default()
            });

        // The supporting slot is emitted unconditionally too — this is the one that would
        // otherwise appear *while the user types*, the moment a value became invalid.
        let beneath: Element<'a, M> = match f.error.or(f.supporting) {
            // The error message replaces the supporting text rather than joining it.
            Some(message) => (container(
                Text::new(message, TypeRole::Caption, r).tint(style::field_support(r, invalid)),
            )
            .padding(iced::Padding {
                top: spacing::XS,
                bottom: 0.0,
                left: anatomy::text_field::PADDING,
                right: anatomy::text_field::PADDING,
            }))
            .into(),
            None => Space::new().into(),
        };

        column![filled, indicator, beneath]
            .width(Length::Fill)
            .into()
    }
}
