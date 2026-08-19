//! Joining a text field to [`State::focused_field`](crate::app::State::focused_field) — BUG-003.
//!
//! # Why this is one call and not two
//!
//! A focused field is two halves that must both be present: it reports focus
//! (`on_focus_change`) and it is *told* focus back (`active`). Either alone is a defect that
//! nothing catches. Reporting without being told is the whole of BUG-003 — the state was
//! observable and no view read it, so every field in the application drew permanently at rest for
//! two features running. Being told without reporting is the same field with a longer path to the
//! same nothing.
//!
//! Neither half looks wrong on its own, which is exactly why they were separable for so long. So
//! the two are one call here, and a field is either wired or visibly not (Principle V): there is no
//! spelling of half of it.
//!
//! # Why the library does not do this itself
//!
//! `FormField` takes `active` as a parameter on purpose — the state that thickens a filled field's
//! indicator is focus for a text input and *open* for a picker (§7.7), so the wrapper is told which
//! rather than assuming. That reasoning is sound and unchanged. What was missing was a way for a
//! caller to find out what to say, and where to keep it; this is the application's answer, and it
//! lives in the application because the choice of where the fact is held is the application's.

use crate::app::Message;
use crate::features::window::FieldId;
use crate::ui::material::{Checkbox, TextField};

/// Wire an input's focus to the application's [`FieldId`].
pub trait TrackFocus {
    /// Report this input's focus as `id`, and draw it from whatever the application currently
    /// holds in `focused`.
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self;
}

impl<'a> TrackFocus for TextField<'a, Message> {
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self {
        self.active(focused == Some(id))
            .on_focus_change(move |focused| Message::FieldFocusChanged(id, focused))
    }
}

/// The checkbox joins on the same terms, which is the point of it being a trait: one `FieldId`
/// space and one message for every input in the application, so "which control has the keyboard"
/// has a single answer rather than one per kind of control.
impl<'a> TrackFocus for Checkbox<'a, Message> {
    fn track_focus(self, id: FieldId, focused: Option<FieldId>) -> Self {
        self.focused(focused == Some(id))
            .on_focus_change(move |focused| Message::FieldFocusChanged(id, focused))
    }
}
