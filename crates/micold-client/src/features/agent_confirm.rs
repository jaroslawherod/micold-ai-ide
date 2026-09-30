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
//! # A prompt waits behind a dialog the user is already in
//!
//! A prompt that arrives while another dialog is open (an Add Worktree form half filled in, say)
//! does not close it: the prompt is **held** until no other dialog is open, and then shown
//! ([`release`], which the root runs after every message). Nothing the user typed is lost to an
//! agent's request, and the prompt still has the rest of its 60 s.
//!
//! # Dismissing is declining
//!
//! Escape, a scrim click, or another dialog opening over this one ([`Msg::Dismissed`]) declines the
//! shown prompt: its id goes to [`State::declined`], which the shell sends as a `ConfirmationAnswer
//! { allow: false }` after the message. It is the dialog's dismissive action, as Deny is, so the
//! agent is told at once rather than after 60 s, and a prompt never vanishes from a window while
//! the daemon still waits on it. Nothing changes either way: the one outcome that performs the
//! operation is an explicit Allow.

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
    /// The prompts wait behind another open dialog; the dialog opens once that one closes.
    pub held: bool,
    /// Prompts this window declined by dismissing the dialog, not yet sent. The shell drains it
    /// after every message.
    pub declined: Vec<u64>,
    /// How deep `app::State::update` is nested; held prompts are released only at depth 0.
    pub update_depth: u32,
}

impl State {
    /// The prompt the dialog shows: the oldest still pending, unless it waits behind another
    /// dialog.
    pub fn shown(&self) -> Option<&Prompt> {
        if self.held {
            None
        } else {
            self.pending.first()
        }
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
    /// The dialog was dismissed (Escape, scrim, or another dialog opening): the shown prompt is
    /// declined.
    Dismissed,
    /// The connection to the service was lost: every prompt it sent is void, and none is declined.
    Disconnected,
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
    match msg {
        Msg::Requested(prompt) => requested(state, prompt),
        Msg::Withdrawn(id) | Msg::Answered { id, .. } => forget(state, id),
        Msg::Disconnected => disconnected(state),
        Msg::Dismissed => {
            if let Some(id) = state.agent_confirm.shown().map(|p| p.id) {
                state.agent_confirm.declined.push(id);
                forget(state, id);
                // The next prompt waits for the root's `release`: another dialog opening over
                // this one closes every open dialog in a loop, and an unheld next prompt would
                // be shown and declined in turn without the user ever seeing it.
                if !state.agent_confirm.pending.is_empty() {
                    state.agent_confirm.held = true;
                }
            }
        }
    }
    Vec::new()
}

/// A prompt arrived. The first one opens the dialog, like every other dialog (one at a time) —
/// unless another dialog is open, which it then waits behind rather than closing. Later ones queue
/// behind the first without disturbing it.
fn requested(state: &mut crate::app::State, prompt: Prompt) {
    if state.agent_confirm.is_pending(prompt.id) {
        return;
    }
    if state.agent_confirm.pending.is_empty() {
        if crate::overlay::registry::open_dialog(state).is_some() {
            state.agent_confirm.held = true;
        } else {
            state.clear_for_dialog();
        }
    }
    state.agent_confirm.pending.push(prompt);
}

/// Show held prompts once no other dialog is open. The root runs this after every message, so
/// the prompt opens as the dialog it waited behind closes.
pub fn release(state: &mut crate::app::State) {
    if state.agent_confirm.held && crate::overlay::registry::open_dialog(state).is_none() {
        state.agent_confirm.held = false;
        // Opening a modal closes the lightweight popovers (FR-012), as `clear_for_dialog` does.
        crate::overlay::registry::close_popovers(state);
    }
}

/// The connection to the service was lost: every prompt it sent is void. The service withdraws
/// them on its side (the window is gone), and after a restart its ids start again, so a stale
/// prompt kept here could be answered for a different request. Nothing is declined.
fn disconnected(state: &mut crate::app::State) {
    state.agent_confirm.pending.clear();
    state.agent_confirm.declined.clear();
    state.agent_confirm.held = false;
}

/// Drop `id`; an id this window does not hold changes nothing.
fn forget(state: &mut crate::app::State, id: u64) {
    state.agent_confirm.pending.retain(|p| p.id != id);
    if state.agent_confirm.pending.is_empty() {
        state.agent_confirm.held = false;
    }
}

/// What the agent asks to do, in words, without its target (FR-014).
///
/// `SendInput` names the act only: the wire carries no text, and the prompt shows none.
pub fn operation_phrase(operation: &ConfirmOperation) -> String {
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
