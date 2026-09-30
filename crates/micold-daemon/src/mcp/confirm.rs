//! Pending confirmations (feature 034, FR-014; data-model TM5).
//!
//! A destructive request an agent makes waits here for the user: every connected window is shown
//! the prompt, the first answer decides, and every way out — an answer, the 60 s deadline, the
//! target or caller going away, the agent's connection closing — withdraws it from every window.
//!
//! [`Registry`] is the bookkeeping alone. It lives inside the daemon state's lock beside the
//! connected clients, so "is any window connected?", opening a prompt and broadcasting it, and a new
//! window's registration with its replay of the pending prompts are each one atomic step: a window
//! can neither miss a prompt nor be shown one twice. `DaemonState::confirm` is the waiting half.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

use micold_core::mcp::errors::{ErrorCategory, OpError};
use micold_core::protocol::messages::{ConfirmOperation, DaemonMsg};
use micold_core::session::SessionId;
use tokio::sync::oneshot;
use tokio::time::Instant;

/// How long a prompt waits for an answer before the request fails "needs confirmation" (FR-014).
pub const CONFIRM_TIMEOUT: Duration = Duration::from_secs(60);

/// What a confirmation is about: the target the operation would change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmTarget {
    /// A worktree of the request's project, by directory name.
    Worktree { dir_name: String },
    /// A session.
    Session(SessionId),
}

/// One request for the user's confirmation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmRequest {
    /// The project both the caller and the target belong to.
    pub project: PathBuf,
    /// The calling session.
    pub caller: SessionId,
    /// Its display label, named in the prompt.
    pub caller_label: String,
    /// What the agent asks to do.
    pub operation: ConfirmOperation,
    /// What it would change.
    pub target: ConfirmTarget,
    /// The target's display name, named in the prompt.
    pub target_label: String,
}

/// How a confirmation ended. The first transition wins; later ones are no-ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmOutcome {
    /// The user allowed it: perform the operation.
    Allowed,
    /// The user declined it.
    Declined,
    /// No answer within [`CONFIRM_TIMEOUT`].
    TimedOut,
    /// No window was connected to ask.
    NoWindow,
    /// The target or the calling session went away while the prompt was pending.
    TargetGone,
    /// The agent's connection closed while the prompt was pending; nothing is performed and no
    /// reply is sent.
    Abandoned,
}

impl ConfirmOutcome {
    /// `Ok` to perform the operation, or the failure the tool call answers with (FR-013).
    pub fn into_result(self) -> Result<(), OpError> {
        let (category, message) = match self {
            Self::Allowed => return Ok(()),
            Self::Declined => (ErrorCategory::RefusedByPolicy, "declined by the user"),
            Self::TimedOut => (
                ErrorCategory::NeedsConfirmation,
                "no answer in an application window within 60 seconds",
            ),
            Self::NoWindow => (
                ErrorCategory::NeedsConfirmation,
                "no application window is open to confirm it",
            ),
            Self::TargetGone => (
                ErrorCategory::NotFound,
                "the target or the calling session went away while waiting for confirmation",
            ),
            Self::Abandoned => (
                ErrorCategory::ServiceError,
                "the request was abandoned before it was confirmed",
            ),
        };
        Err(OpError::new(category, message))
    }
}

struct Pending {
    request: ConfirmRequest,
    deadline: Instant,
    answer: oneshot::Sender<ConfirmOutcome>,
}

/// A prompt just opened: what to broadcast, and what the requester waits on.
pub struct Opened {
    pub id: u64,
    pub prompt: DaemonMsg,
    pub deadline: Instant,
    pub answer: oneshot::Receiver<ConfirmOutcome>,
}

/// The pending prompts, by id. Ids are monotonic per service run.
#[derive(Default)]
pub struct Registry {
    next_id: u64,
    pending: BTreeMap<u64, Pending>,
}

impl Registry {
    /// Open a prompt for `request` at `now`.
    pub fn open(&mut self, request: ConfirmRequest, now: Instant) -> Opened {
        self.next_id += 1;
        let id = self.next_id;
        let deadline = now + CONFIRM_TIMEOUT;
        let (tx, rx) = oneshot::channel();
        let pending = Pending {
            request,
            deadline,
            answer: tx,
        };
        let prompt = prompt(id, &pending, now);
        self.pending.insert(id, pending);
        Opened {
            id,
            prompt,
            deadline,
            answer: rx,
        }
    }

    /// Resolve prompt `id` with `outcome` if it is still pending, returning the withdrawal to
    /// broadcast. `None` when it was already resolved or never existed: the first resolver wins.
    pub fn resolve(&mut self, id: u64, outcome: ConfirmOutcome) -> Option<DaemonMsg> {
        let pending = self.pending.remove(&id)?;
        // The requester may already be gone (abandoned); nothing to tell it then.
        let _ = pending.answer.send(outcome);
        Some(DaemonMsg::ConfirmationWithdrawn { id })
    }

    /// Resolve every pending prompt that `session` is the caller or the target of as
    /// [`ConfirmOutcome::TargetGone`], returning the withdrawals to broadcast.
    pub fn session_gone(&mut self, session: SessionId) -> Vec<DaemonMsg> {
        self.resolve_where(|r| r.caller == session || r.target == ConfirmTarget::Session(session))
    }

    /// Resolve every pending prompt whose target is `project`'s worktree `dir_name` as
    /// [`ConfirmOutcome::TargetGone`], returning the withdrawals to broadcast.
    pub fn worktree_gone(&mut self, project: &std::path::Path, dir_name: &str) -> Vec<DaemonMsg> {
        self.resolve_where(|r| {
            r.project == project
                && matches!(&r.target, ConfirmTarget::Worktree { dir_name: d } if d == dir_name)
        })
    }

    fn resolve_where(&mut self, matches: impl Fn(&ConfirmRequest) -> bool) -> Vec<DaemonMsg> {
        let ids: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, p)| matches(&p.request))
            .map(|(id, _)| *id)
            .collect();
        ids.into_iter()
            .filter_map(|id| self.resolve(id, ConfirmOutcome::TargetGone))
            .collect()
    }

    /// One prompt per pending confirmation, in the order they were opened, with the time each has
    /// left at `now` — for a window that has just connected.
    pub fn replay(&self, now: Instant) -> Vec<DaemonMsg> {
        self.pending
            .iter()
            .map(|(id, pending)| prompt(*id, pending, now))
            .collect()
    }
}

fn prompt(id: u64, pending: &Pending, now: Instant) -> DaemonMsg {
    let left = pending.deadline.saturating_duration_since(now);
    let request = &pending.request;
    DaemonMsg::ConfirmationRequested {
        id,
        project: request.project.clone(),
        caller: request.caller,
        caller_label: request.caller_label.clone(),
        operation: request.operation,
        target_label: request.target_label.clone(),
        expires_in_ms: u32::try_from(left.as_millis()).unwrap_or(u32::MAX),
    }
}
