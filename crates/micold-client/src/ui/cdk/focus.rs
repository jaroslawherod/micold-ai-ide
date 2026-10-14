//! One traversal for asking a control about focus and for handing it the keyboard.
//!
//! Focus lives in the control's own state, and an [`Operation`] is the way in that does not require
//! knowing what the control is. A control with nothing to focus never answers, so these are as safe
//! to run over a plain container as over a text input. Taking the keyboard back is the stock
//! [`focusable::unfocus`], which needs no copy here.

use iced::advanced::widget::operation::{focusable, Focusable};
use iced::advanced::widget::{Id, Operation};
use iced::Rectangle;

/// What to do to every focusable under the traversed widget.
pub enum FocusOp {
    /// Whether the control holds the keyboard — asked of the control, not guessed from the pointer.
    ///
    /// A field cannot infer this: a press in its padding lands on the container and not on the
    /// input, and focus also moves for reasons that never reach the field at all (Tab, a focus
    /// operation, the window losing and regaining focus). Any rule written there would be a second
    /// opinion about a fact the input already holds, so this asks, and the answer comes from the
    /// input's own state. A control that cannot be focused never answers, which leaves the flag
    /// `false`.
    Asking(bool),
    /// Hand the keyboard to each that does not hold it.
    Focusing,
}

impl FocusOp {
    /// An operation that asks, with the answer still `false`.
    pub fn asking() -> Self {
        Self::Asking(false)
    }

    /// Whether an [`Asking`](Self::Asking) operation found a focused control.
    pub fn answer(&self) -> bool {
        matches!(self, Self::Asking(true))
    }

    /// The stock operation that takes the keyboard back.
    pub fn unfocusing() -> impl Operation {
        focusable::unfocus::<()>()
    }
}

impl Operation for FocusOp {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
        match self {
            Self::Asking(found) => *found |= state.is_focused(),
            Self::Focusing => {
                if !state.is_focused() {
                    state.focus();
                }
            }
        }
    }
}
