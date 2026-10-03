//! What this window last told the session service it has in view (feature 039, W1.1), which
//! attention events it claims, and the desktop notification it raises for the one it is granted
//! (W2.1).
//!
//! The service counts an attention event only for a session no window has in view, so every window
//! reports the one it shows. Which session that is, is the core's rule
//! ([`micold_core::attention::in_view`]); what this feature adds is *when to say so*: once after
//! each `Welcome`, and afterwards only when the answer has changed (FR-019).
//!
//! Every window sees the same change, so each one claims it and the service grants it to one
//! (research R3). Which changes are claims is the core's [`AttentionTracker`]; this feature owns
//! the tracker, and shows the granted one through a [`DesktopNotifier`] (contract N1–N4).
//!
//! # No vocabulary
//!
//! There is no `Msg` here and no `update`. Nothing the user does is addressed to this feature: the
//! report is a consequence of whatever else changed — a session selected, Settings opened, the
//! window losing focus — so the shell asks after every message, through the root
//! ([`crate::app::State::view_report`]), and sends what comes back. Snapshots and grants come from
//! the service, and reach this feature through the root in the same way.
//!
//! # The state this feature remembers
//!
//! Three values in [`State`], reached as `state.attention`: `sent_view`, the last report sent on
//! this connection (`None` means this connection has been told nothing yet, which is what makes the
//! first report unconditional — also when it says that nothing is in view); `tracker`, what this
//! process last saw of each session, kept across connections; and `failure_logged`, whether a
//! failure to show has been logged in this run (FR-010).

use micold_core::attention::{AttentionTracker, Phase, ViewFacts};
use micold_core::protocol::messages::{ClientMsg, SessionSummary, WindowView};
use micold_core::session::{Session, SessionId};
use std::path::PathBuf;

/// What this feature remembers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The last report sent on this connection; `None` until the first one after a `Welcome`.
    pub sent_view: Option<WindowView>,
    /// What this process last saw of each session (research R3). Kept across connections: the
    /// first snapshot after a reconnect is compared with it (FR-006).
    pub tracker: AttentionTracker,
    /// Whether a failure to show a notification has been logged in this run (FR-010).
    pub failure_logged: bool,
}

/// One desktop notification, as the backend is asked to show it (contract, "The seam").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesktopNotification {
    /// `<session label> is waiting for input`.
    pub title: String,
    /// `<project> — <worktree>`.
    pub body: String,
    /// The project the session belongs to.
    pub project: PathBuf,
    /// The session that is waiting.
    pub session: SessionId,
}

/// Why the system did not accept a notification (FR-010). Logged once per run, never shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyError {
    /// This build has no backend for the system it runs on.
    Unsupported,
    /// No notification service could be reached: no session bus, or nobody serves the
    /// notification interface on it.
    NoService(String),
    /// The notification service was reached and did not accept the notification.
    Refused(String),
}

impl std::fmt::Display for NotifyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotifyError::Unsupported => {
                f.write_str("desktop notifications are not supported on this system yet")
            }
            NotifyError::NoService(why) => write!(f, "no notification service: {why}"),
            NotifyError::Refused(why) => write!(f, "the notification service refused: {why}"),
        }
    }
}

impl std::error::Error for NotifyError {}

/// The operating system's notification facility (research R4). Tests use a recording one.
pub trait DesktopNotifier: Send + Sync {
    /// Show one notification. An error means the system did not accept it.
    fn show(&self, notification: DesktopNotification) -> Result<(), NotifyError>;
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

/// The session this window last reported in view, if any (FR-019).
pub fn in_view(state: &State) -> Option<SessionId> {
    state.sent_view.and_then(|view| view.in_view)
}

/// Whether `session`'s row carries the unread mark in a window that has `in_view` in view
/// (contract `unread-mark.md` U5).
///
/// The session in view is read at once: the window does not wait for the catalog in which the
/// service has cleared `unread` (FR-019, SC-006).
pub fn row_unread(session: &Session, in_view: Option<SessionId>) -> bool {
    session.unread && in_view != Some(session.id)
}

/// A new connection has been told nothing: forget what the previous one was sent (FR-006).
pub fn connection_started(state: &mut State) {
    state.sent_view = None;
}

/// The claims one full catalog snapshot gives rise to, as the messages to send (research R3).
pub fn snapshot_claims(
    state: &mut State,
    sessions: &[SessionSummary],
    phase: Phase,
    in_view: Option<SessionId>,
) -> Vec<ClientMsg> {
    state
        .tracker
        .observe(sessions, phase, in_view)
        .into_iter()
        .map(|claim| ClientMsg::AttentionClaim {
            session: claim.session,
            seq: claim.seq,
        })
        .collect()
}

/// Show the notification for a granted attention event (N1), on the calling thread, and take the
/// result as [`show_result`] does. The shell does not call this: showing can wait on the system,
/// so it shows on a blocking task and reports the result to [`show_result`].
pub fn show_granted(
    state: &mut State,
    notification: DesktopNotification,
    notifier: &dyn DesktopNotifier,
) -> Option<String> {
    show_result(state, notifier.show(notification))
}

/// What showing a notification came to. Returns the line to log when the system did not accept it
/// and nothing has been logged yet in this run (N4); the caller logs it, and nothing else happens.
pub fn show_result(state: &mut State, result: Result<(), NotifyError>) -> Option<String> {
    let error = result.err()?;
    if state.failure_logged {
        return None;
    }
    state.failure_logged = true;
    Some(format!(
        "desktop notification not shown ({error}); later failures in this run are not logged"
    ))
}
