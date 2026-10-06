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
//! A click on a notification comes back from the backend as a [`NotifierEvent`]. The window sends
//! it to the service ([`notifier_event`]), which picks the window that shows the session (research
//! R6); that window is raised ([`raise_plan`], then [`after_send`] and [`ActivationWatch`] for a
//! Wayland activation) and does what [`reveal_steps`] says (N5, N6).
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

use micold_core::attention::{AttentionTracker, NotificationKind, Phase, Reveal, ViewFacts};
use micold_core::protocol::messages::{ClientMsg, SessionSummary, WindowView};
use micold_core::session::{Session, SessionId};
use std::path::{Path, PathBuf};
use std::time::Duration;

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
    /// What happened to the session (feature 613): it names the title, and the backend's icon.
    pub kind: NotificationKind,
    /// `<session label> <what happened>`, by kind (feature 613, contract T1).
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

/// What a backend reports back, over the channel it was built with (contract, "The seam").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifierEvent {
    /// The notification for this session was clicked.
    Activated {
        /// The project the notification named.
        project: PathBuf,
        /// The session the notification named.
        session: SessionId,
        /// The Wayland activation token of the click, when the notification service sent one.
        activation: Option<String>,
    },
}

/// One step of bringing the window to the front (contract, "Raising the window").
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaiseStep {
    /// Restore the window when it is minimised.
    Unminimize,
    /// Take keyboard focus.
    Focus,
    /// Wayland: ask the compositor for keyboard focus with this activation token.
    Activate(String),
    /// Ask for the user's attention: the window cannot take focus itself.
    RequestAttention,
}

/// The steps that bring the window to the front (N6): restore it, then take keyboard focus. On
/// Wayland a window cannot take focus itself: with the `activation` token of the click it asks the
/// compositor for the focus, and with none it asks for the user's attention. Off Wayland the token
/// is not needed and changes nothing.
pub fn raise_plan(wayland: bool, activation: Option<String>) -> Vec<RaiseStep> {
    let front = match (wayland, activation) {
        (true, Some(token)) => RaiseStep::Activate(token),
        (true, None) => RaiseStep::RequestAttention,
        (false, _) => RaiseStep::Focus,
    };
    vec![RaiseStep::Unminimize, front]
}

/// What follows a [`RaiseStep::Activate`], given whether it was `done`: one that was not done
/// falls back to asking for the user's attention (FR-015).
///
/// A compositor tells the window nothing about a token it declines (research R7). So an
/// activation is done only when the request went out ([`after_send`]) **and**, [`ACTIVATION_SETTLE`]
/// later, [`activation_done`] says so of the window's keyboard focus.
pub fn after_activation(done: bool) -> Option<RaiseStep> {
    (!done).then_some(RaiseStep::RequestAttention)
}

/// Whether an activation request that went out was honoured (research R7): the window gained
/// keyboard focus since the request was sent, or the last focus event it has seen since launch
/// (`last_focus`, `None` when there was none) is a gain — it had the focus when the click came, and
/// then no event follows. A window that has seen no focus event is not known to be focused.
pub fn activation_done(gained_since_send: bool, last_focus: Option<bool>) -> bool {
    gained_since_send || last_focus == Some(true)
}

/// What the window has seen of its keyboard focus, and the one activation request it is waiting
/// to judge. The shell records the facts here and takes the step [`settled`](Self::settled) gives.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ActivationWatch {
    /// The last focus event seen since launch: `None` until the first one.
    last_focus: Option<bool>,
    /// The request whose wait is running, if any.
    pending: Option<PendingCheck>,
}

/// An activation request that went out and is not judged yet.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PendingCheck {
    /// The number the shell gave the request.
    check: u64,
    /// Whether the window gained keyboard focus since the request was sent.
    gained: bool,
}

impl ActivationWatch {
    /// The window gained or lost keyboard focus.
    pub fn focus_changed(&mut self, focused: bool) {
        self.last_focus = Some(focused);
        if let (true, Some(pending)) = (focused, self.pending.as_mut()) {
            pending.gained = true;
        }
    }

    /// The activation request numbered `check` went out. It replaces a request that is still
    /// waiting: there is one pending check, the latest.
    pub fn sent(&mut self, check: u64) {
        self.pending = Some(PendingCheck {
            check,
            gained: false,
        });
    }

    /// The wait for the request numbered `check` is over: the step [`after_activation`] gives
    /// for it, if any. A request that was replaced, or judged already, gives none.
    pub fn settled(&mut self, check: u64) -> Option<RaiseStep> {
        let pending = self.pending.take_if(|pending| pending.check == check)?;
        after_activation(activation_done(pending.gained, self.last_focus))
    }
}

/// How long the compositor is given to move the keyboard focus after an activation request. It
/// took 12 ms in the probe (research R7).
pub const ACTIVATION_SETTLE: Duration = Duration::from_millis(400);

/// What the window does once it has tried to send an activation request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AfterSend {
    /// The request went out: after this long, take the step [`ActivationWatch::settled`] gives
    /// for it. The wait holds up nothing else the window does.
    CheckFocusAfter(Duration),
    /// The request did not go out: take this step at once.
    Now(RaiseStep),
}

/// What follows the attempt to send an activation request, given whether it was `sent`. One that
/// was not — no Wayland surface, no `xdg_activation_v1`, a failed call — is not done.
pub fn after_send(sent: bool) -> AfterSend {
    if sent {
        AfterSend::CheckFocusAfter(ACTIVATION_SETTLE)
    } else {
        AfterSend::Now(RaiseStep::RequestAttention)
    }
}

/// What this window sends the service for `event` (research R6): the click is not handled here,
/// because the window that holds the session's project may be another one (FR-012). The click's
/// activation token goes with it, for the window that is raised (W3.4).
pub fn notifier_event(event: NotifierEvent) -> ClientMsg {
    match event {
        NotifierEvent::Activated {
            project,
            session,
            activation,
        } => ClientMsg::SessionReveal {
            project,
            session,
            activation,
        },
    }
}

/// The notice for a click on a notification whose session cannot be shown (FR-013).
pub const SESSION_UNAVAILABLE: &str = "That session is no longer available.";

/// One thing a window does to show a session it was asked to reveal (N5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevealStep {
    /// Make this project the active one, by the ordinary switch.
    Reopen(PathBuf),
    /// Select this session, as a press on its row does.
    Select(SessionId),
    /// Say this at `Level::Info`, and change no selection.
    Notice(&'static str),
}

/// What a window whose active project is `active` does for `reveal` (N5, FR-011, FR-013, FR-014):
/// the project is reopened only when it is not the active one, then the session is selected; a
/// session that cannot be shown yields the notice alone.
pub fn reveal_steps(reveal: Reveal, active: Option<&Path>) -> Vec<RevealStep> {
    match reveal {
        Reveal::Show { project, session } => {
            let mut steps = Vec::with_capacity(2);
            if active != Some(project.as_path()) {
                steps.push(RevealStep::Reopen(project));
            }
            steps.push(RevealStep::Select(session));
            steps
        }
        Reveal::Unavailable => vec![RevealStep::Notice(SESSION_UNAVAILABLE)],
    }
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
///
/// The same rule a location row counts by (feature 575), so a collapsed row's number and its
/// session rows' marks cannot disagree; a closed session has no row to mark.
pub fn row_unread(session: &Session, in_view: Option<SessionId>) -> bool {
    micold_core::attention::counts_as_unread(session, in_view)
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
