//! An agent's destructive request, put to the user before it happens (feature 034, FR-014).
//!
//! When an AI session asks the daemon's tool server to delete a worktree or a session, stop or
//! interrupt another session, or type into one, the daemon does not act: it sends every connected
//! window a [`ConfirmationRequested`](micold_core::protocol::messages::DaemonMsg) prompt and waits.
//! The first window to answer decides; every resolution — an answer from any window, the 60 s
//! timeout, the target going away — arrives here as a withdrawal, and the dialog closes.
//!
//! # The vocabulary this feature declares
//!
//! Four transitions in [`Msg`], routed by [`update`], which is pure (shape A). Two come from the
//! daemon (`Requested`, `Withdrawn`), two from the dialog (`Answered`, `Dismissed`). Putting the
//! answer on the wire is the shell's job (`shell::daemon_sync::on_agent_confirm_answered`); this
//! reducer only forgets the prompt.
//!
//! # The state this feature remembers
//!
//! The prompts this window has been shown and not yet seen resolved, oldest first. The dialog shows
//! the oldest; answering it shows the next. A prompt the daemon sends twice (a replay on reconnect
//! racing a live broadcast) is kept once.
//!
//! # Dismissing is not denying
//!
//! Escape, a scrim click, or another dialog opening over this one ([`Msg::Dismissed`]) takes the
//! shown prompt off *this window* and sends nothing. The daemon keeps waiting for another window's
//! answer, and without one the request times out and the agent is told it needs confirmation. So a
//! dismissed prompt still changes nothing — the one outcome that performs the operation is an
//! explicit Allow — while a user who closed the dialog by reflex has not refused on behalf of a
//! colleague looking at another window.

use crate::app::Message;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;
use micold_core::protocol::messages::ConfirmOperation;
use std::path::PathBuf;

/// One agent request waiting for the user's answer: what the dialog needs to name it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    /// The daemon's id for it, echoed by the answer.
    pub id: u64,
    /// The project the caller and the target belong to.
    pub project: PathBuf,
    /// The calling session's display label.
    pub caller_label: String,
    /// What the agent asks to do.
    pub operation: ConfirmOperation,
    /// The target's display name: a worktree's display name or a session's label.
    pub target_label: String,
}

/// What this feature remembers: the prompts awaiting an answer here, in arrival order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Oldest first. The head is the one the dialog shows.
    pub pending: Vec<Prompt>,
}

impl State {
    /// The prompt the dialog shows: the oldest still pending.
    pub fn shown(&self) -> Option<&Prompt> {
        self.pending.first()
    }

    /// Whether `id` is still waiting for an answer in this window.
    pub fn is_pending(&self, id: u64) -> bool {
        self.pending.iter().any(|p| p.id == id)
    }
}

/// What can happen to an agent's pending request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The daemon asked this window to confirm a request.
    Requested(Prompt),
    /// The daemon resolved the request (answered anywhere, expired, or its target is gone).
    Withdrawn(u64),
    /// The user chose Allow (`true`) or Deny (`false`) on the shown prompt.
    Answered {
        /// The prompt answered.
        id: u64,
        /// Whether the operation may go ahead.
        allow: bool,
    },
    /// The dialog was dismissed without an answer (Escape, scrim, or another dialog opening).
    Dismissed,
}

/// The dialog that asks, as a floating surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmAgentRequestDialog;

impl FloatingSurface for ConfirmAgentRequestDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("confirm_agent_request")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::AgentConfirm(Msg::Dismissed))
    }
}

impl Registered for ConfirmAgentRequestDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state
            .agent_confirm
            .shown()
            .map(|_| ConfirmAgentRequestDialog)
    }
}

/// This feature's whole reducer surface (shape A). Pure: the answer's wire message is the shell's.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    #[allow(unreachable_code)]
    return Vec::new(); // red: not yet implemented
    match msg {
        Msg::Requested(prompt) => requested(state, prompt),
        Msg::Withdrawn(id) | Msg::Answered { id, .. } => forget(state, id),
        Msg::Dismissed => {
            if !state.agent_confirm.pending.is_empty() {
                state.agent_confirm.pending.remove(0);
            }
        }
    }
    Vec::new()
}

/// A prompt arrived. The first one opens the dialog, through `clear_for_dialog` like every other
/// dialog (one dialog at a time); later ones queue behind the shown one without disturbing it.
fn requested(state: &mut crate::app::State, prompt: Prompt) {
    if state.agent_confirm.is_pending(prompt.id) {
        return;
    }
    if state.agent_confirm.pending.is_empty() {
        state.clear_for_dialog();
    }
    state.agent_confirm.pending.push(prompt);
}

/// Drop `id`; an id this window does not hold changes nothing.
fn forget(state: &mut crate::app::State, id: u64) {
    state.agent_confirm.pending.retain(|p| p.id != id);
}

/// What the agent asks to do, in words, without its target (FR-014).
///
/// `SendInput` names the act only: the wire carries no text, and the prompt shows none.
pub fn operation_phrase(operation: &ConfirmOperation) -> String {
    #[allow(unreachable_code)]
    return String::new(); // red
    match operation {
        ConfirmOperation::DeleteWorktree {
            stop_sessions,
            delete_branch,
        } => {
            let mut phrase = "delete worktree".to_string();
            if *delete_branch {
                phrase.push_str(" and its branch");
            }
            if *stop_sessions {
                phrase.push_str(", stopping its sessions");
            }
            phrase
        }
        ConfirmOperation::DeleteSession => "delete session".to_string(),
        ConfirmOperation::StopSession => "stop session".to_string(),
        ConfirmOperation::InterruptSession => "interrupt session".to_string(),
        ConfirmOperation::SendInput => "type into session".to_string(),
    }
}

/// The dialog's headline: who asks, to do what, to which target.
///
/// `“planner” asks to delete worktree “feat-x” and its branch, stopping its sessions`.
pub fn headline(prompt: &Prompt) -> String {
    #[allow(unreachable_code)]
    return String::new(); // red
    let (verb, suffix) = match prompt.operation {
        ConfirmOperation::DeleteWorktree {
            stop_sessions,
            delete_branch,
        } => {
            let mut suffix = String::new();
            if delete_branch {
                suffix.push_str(" and its branch");
            }
            if stop_sessions {
                suffix.push_str(", stopping its sessions");
            }
            ("delete worktree".to_string(), suffix)
        }
        other => (operation_phrase(&other), String::new()),
    };
    format!(
        "“{}” asks to {verb} “{}”{suffix}",
        prompt.caller_label, prompt.target_label
    )
}
