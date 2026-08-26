//! `Checkbox` — the library's wrapper around the rendering stack's checkbox (Principle VIII).
//!
//! Two call sites, both identical, both naming the style function directly. Wrapped for the same
//! reason as the rest: a call site that can reach the style layer can render a checkbox that does
//! not match the other one, and the only thing preventing it is that nobody has yet.
//!
//! Parity: the style resolves to exactly what the call sites use today (FR-005).
//!
//! # Feature 022: the keyboard the stack's checkbox does not have (BUG-003)
//!
//! FR-035 asks every input to answer focus, and this was recorded as impossible for the checkbox:
//! its style is a function of a `Status` with three variants — active, hovered, disabled — so there
//! was no focused state to attach a layer to.
//!
//! That was the *symptom*. The cause is larger and simpler: **the rendering stack's checkbox cannot
//! be focused at all.** Its widget state is the label's shaped paragraph, it implements no focus
//! traversal, and it answers no key. There was no focus to report because there was never any focus
//! — the control was reachable by pointer only, which is an accessibility gap as much as a visual
//! one.
//!
//! So this gives it one, in the smallest thing that can hold it: [`TakesTheKeyboard`], a wrapper
//! widget that owns the focus, takes it on a press, offers it to the focus traversal, toggles on
//! Space (and only Space — Enter belongs to the dialog, see below), and reports changes so a screen
//! can supply the flag back. The stack's checkbox keeps drawing itself and keeps owning the
//! pointer; nothing about its appearance moved here.
//!
//! That wrapper is no longer this module's own: feature 027's FR-030 asked for the same capability
//! on the buttons of a settings surface, so it lives in
//! [`keyboard_focus`](super::keyboard_focus) and the checkbox is now one of its two callers.
//!
//! Deliberately **not** a reimplementation. `FilledField` owns the field's box because §7.7's
//! geometry could not be composed; nothing here is wrong with the checkbox's geometry, so what is
//! added is the one capability it lacks and no more.

use crate::ui::material::keyboard_focus::TakesTheKeyboard;
use crate::ui::material::style;
use iced::widget::checkbox;
use iced::{keyboard, Element};
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
    focused: bool,
    on_focus_change: Option<Box<dyn Fn(bool) -> M + 'a>>,
}

impl<'a, M: Clone + 'a> Checkbox<'a, M> {
    /// A checkbox reading `label`, currently `checked`, themed by `roles`.
    pub fn new(label: impl Into<String>, checked: bool, roles: Roles) -> Self {
        Self {
            label: label.into(),
            checked,
            roles,
            on_toggle: None,
            focused: false,
            on_focus_change: None,
        }
    }

    /// The message emitted when the box is toggled, given the new state.
    pub fn on_toggle(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_toggle = Some(Box::new(f));
        self
    }

    /// Whether the box holds the keyboard, which shades it with the focused state layer.
    ///
    /// Supplied rather than observed, exactly as [`FormField::active`](super::FormField::active)
    /// is, and for a sharper version of the same reason: the style is resolved when the widget is
    /// *built*, and the thing that knows about focus does not exist until afterwards. What a caller
    /// supplies here comes back from [`Self::on_focus_change`]; setting one without the other is
    /// BUG-003.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    /// The message emitted when the box takes or loses the keyboard (BUG-003).
    pub fn on_focus_change(mut self, f: impl Fn(bool) -> M + 'a) -> Self {
        self.on_focus_change = Some(Box::new(f));
        self
    }
}

impl<'a, M: Clone + 'a> From<Checkbox<'a, M>> for Element<'a, M> {
    fn from(c: Checkbox<'a, M>) -> Self {
        let mut widget = checkbox(c.checked)
            .label(c.label)
            .style(style::checkbox(c.roles, c.focused));
        // What Space will send, worked out now because the closure is about to be handed to the
        // inner widget. A checkbox has exactly one thing a key can do, so there is one message
        // rather than a second closure.
        let on_key = c.on_toggle.as_ref().map(|f| f(!c.checked));
        if let Some(on_toggle) = c.on_toggle {
            widget = widget.on_toggle(on_toggle);
        }

        // `on_key` is `Some` exactly when `on_toggle` was, and a checkbox without one renders
        // disabled — so it is also the disabled test, read off the one field that still remembers.
        let mut wrapper = TakesTheKeyboard::new(widget, on_key.is_some()).focused(c.focused);
        if let Some(message) = on_key {
            // Space, and **only** Space. That is the key a checkbox answers everywhere it exists —
            // the platform convention and WAI-ARIA's — and Enter is deliberately left alone,
            // because Enter belongs to the dialog. Today it reaches `TextField::on_submit`, which
            // is what saves the settings form and confirms both renames; a dialog-level default
            // action is the obvious next thing to add. Either way, toggling is what the box does
            // and committing the form is not its business, so it must not be the thing that
            // answers first.
            wrapper = wrapper.key(keyboard::key::Named::Space, message);
        }
        // No indicator of its own: the focused state layer is composited into the box's fill by
        // `style::checkbox`, from the flag the application supplies here. A ring drawn by the
        // wrapper as well would be one state said twice, in two tones.
        if let Some(on_focus_change) = c.on_focus_change {
            wrapper = wrapper.on_focus_change(on_focus_change);
        }
        wrapper.into()
    }
}
