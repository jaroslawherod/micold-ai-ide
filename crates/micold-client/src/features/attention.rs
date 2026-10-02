//! What this window last told the session service it has in view (feature 039, W1.1).
//!
//! The service counts an attention event only for a session no window has in view, so every window
//! reports the one it shows. Which session that is, is the core's rule
//! ([`micold_core::attention::in_view`]); what this feature adds is *when to say so*: once after
//! each `Welcome`, and afterwards only when the answer has changed (FR-019).
//!
//! # No vocabulary
//!
//! There is no `Msg` here and no `update`. Nothing the user does is addressed to this feature: the
//! report is a consequence of whatever else changed — a session selected, Settings opened, the
//! window losing focus — so the shell asks after every message, through the root
//! ([`crate::app::State::view_report`]), and sends what comes back.
//!
//! # The state this feature remembers
//!
//! One value in [`State`], reached as `state.attention`: `sent_view`, the last report sent on this
//! connection. `None` means this connection has been told nothing yet, which is what makes the
//! first report unconditional — also when it says that nothing is in view.

use micold_core::attention::ViewFacts;
use micold_core::protocol::messages::WindowView;

/// What this feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The last report sent on this connection; `None` until the first one after a `Welcome`.
    pub sent_view: Option<WindowView>,
}

/// The report to send for `facts`, when the service has not been told it yet (FR-019).
///
/// `Some` for the first report of a connection whatever it says, and afterwards only when the
/// derived value differs from the last one sent. A returned report is recorded as sent, so the
/// caller must send it.
pub fn view_report(state: &mut State, facts: ViewFacts) -> Option<WindowView> {
    let view = WindowView {
        focused: facts.window_focused,
        in_view: micold_core::attention::in_view(facts),
    };
    if state.sent_view == Some(view) {
        return None;
    }
    state.sent_view = Some(view);
    Some(view)
}

/// A new connection has been told nothing: forget what the previous one was sent (FR-006).
pub fn connection_started(state: &mut State) {
    state.sent_view = None;
}
