//! Permission prompts of structured sessions (#942, research R4): the requests a session's CLI
//! asks through the prompt tool, held until the user decides.
//!
//! [`PromptRegistry`] is the bookkeeping alone. It lives inside the daemon state's lock beside the
//! connected clients, as the confirmations of [`crate::mcp::confirm`] do, so opening a request and
//! broadcasting it, and a client's registration with its replay of the pending requests, are each
//! one atomic step: no client misses a request or is shown one twice (FR-005). The first decision
//! removes the entry, so a later one finds nothing (FR-010, FR-012). Nothing here has a deadline
//! (FR-003, R3), and nothing is persisted.
//!
//! [`serve_prompt`] is the waiting half: the prompt tool's `tools/call` handler. It also withdraws
//! its session's requests when [`Endings`] says the session's turn or process ended (FR-011).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

use micold_core::protocol::messages::{
    DaemonMsg, PermissionRequest, PromptChoice, PromptId, Resolution, WithdrawReason,
};
use micold_core::session::{PermissionMode, SessionId};
use micold_core::structured::permission::{
    answer, classify, input_preview, parse_call, validate, AllowRule, DecisionRefusal, PromptCall,
    RequestKind,
};
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::state::DaemonState;

/// What Claude is told when the call cannot become a request (FR-004).
const MALFORMED: &str = "Micold could not read this permission request";

/// What Claude is told when its request ended without a decision.
const WITHDRAWN: &str = "The request was withdrawn before you answered.";

/// What Claude is told when its turn, process or session ended while it waited (FR-011).
const TURN_ENDED: &str = "The turn ended before you answered.";

/// One request waiting for the user.
struct Pending {
    request: PermissionRequest,
    call: PromptCall,
    /// The waiting tool call's half; dropped unanswered when the request is withdrawn.
    answer: oneshot::Sender<Value>,
}

/// The pending requests of every structured session, and each session's always-allow rules.
#[derive(Default)]
pub struct PromptRegistry {
    next_id: u64,
    pending: BTreeMap<PromptId, Pending>,
    rules: HashMap<SessionId, Vec<AllowRule>>,
}

/// What opening a request did.
pub enum Opened {
    /// A rule of the session allows it: the answer, and no request (R9).
    Answered(Value),
    /// A request is pending: broadcast `broadcast` and wait on `receiver`.
    Pending {
        id: PromptId,
        receiver: oneshot::Receiver<Value>,
        broadcast: Box<DaemonMsg>,
        /// No other request of the session was pending: it begins waiting for the user (R8).
        first_of_session: bool,
    },
}

/// What a decision did.
#[derive(Debug)]
pub struct Decided {
    /// The `PermissionResolved` every client is sent.
    pub resolved: DaemonMsg,
    /// No other request of the session is pending.
    pub last_of_session: bool,
    /// The mode an approved plan sets (R7).
    pub mode_change: Option<PermissionMode>,
}

impl PromptRegistry {
    /// Open a request of `session` for `call` of `kind`, unless one of the session's rules allows
    /// it.
    pub fn open(&mut self, session: SessionId, call: PromptCall, kind: RequestKind) -> Opened {
        let rules = self.rules.get(&session).map(Vec::as_slice).unwrap_or(&[]);
        if matches!(kind, RequestKind::Tool) && rules.iter().any(|rule| rule.matches(&call)) {
            return Opened::Answered(answer(&call, &kind, &PromptChoice::Allow));
        }
        let first_of_session = !self.has_pending(session);
        self.next_id += 1;
        let id = PromptId(self.next_id);
        let (input_preview, input_bytes) = input_preview(&call.input);
        let request = PermissionRequest {
            id,
            session,
            tool_use_id: call.tool_use_id.clone(),
            tool_name: call.tool_name.clone(),
            input_preview,
            input_bytes,
            always_allow: AllowRule::offered(&call, &kind).map(|rule| rule.describe()),
            kind,
        };
        let broadcast = Box::new(DaemonMsg::PermissionRequested {
            request: request.clone(),
        });
        let (answer, receiver) = oneshot::channel();
        self.pending.insert(
            id,
            Pending {
                request,
                call,
                answer,
            },
        );
        Opened::Pending {
            id,
            receiver,
            broadcast,
            first_of_session,
        }
    }

    /// Decide request `id`, named as `session`'s, with `choice`: the answer goes to the waiting
    /// call, and the entry is gone. Refused, it changes nothing (FR-012).
    pub fn decide(
        &mut self,
        session: SessionId,
        id: PromptId,
        choice: &PromptChoice,
    ) -> Result<Decided, DecisionRefusal> {
        let entry = self.pending.get(&id).ok_or(DecisionRefusal::NotPending)?;
        if entry.request.session != session {
            return Err(DecisionRefusal::OtherSession);
        }
        let kind = &entry.request.kind;
        validate(kind, choice)?;
        let rule = match choice {
            PromptChoice::AlwaysAllow => {
                Some(AllowRule::offered(&entry.call, kind).ok_or(DecisionRefusal::WrongKind)?)
            }
            _ => None,
        };
        // A call that hung up is withdrawn by its own handler; until then it is not answerable.
        if entry.answer.is_closed() {
            return Err(DecisionRefusal::NotPending);
        }
        let entry = self.pending.remove(&id).expect("looked up above");
        let reply = answer(&entry.call, &entry.request.kind, choice);
        let _ = entry.answer.send(reply);
        if let Some(rule) = rule {
            self.rules.entry(session).or_default().push(rule);
        }
        let (resolution, mode_change) = resolution_of(choice);
        Ok(Decided {
            resolved: resolved(&entry.request, resolution),
            last_of_session: !self.has_pending(session),
            mode_change,
        })
    }

    /// Withdraw request `id` for `reason`, if it is still pending: its call is not answered (its
    /// sender is dropped). Returns the `PermissionResolved` to broadcast.
    pub fn withdraw(&mut self, id: PromptId, reason: WithdrawReason) -> Option<DaemonMsg> {
        let entry = self.pending.remove(&id)?;
        Some(resolved(&entry.request, Resolution::Withdrawn { reason }))
    }

    /// Withdraw every pending request of `session` for `reason`: each call still waiting is
    /// answered deny (FR-011). Its process ended unless only its turn did, and its always-allow
    /// rules end with it (R9). Returns the `PermissionResolved`s to broadcast, oldest first.
    pub fn withdraw_session(
        &mut self,
        session: SessionId,
        reason: WithdrawReason,
    ) -> Vec<DaemonMsg> {
        if reason != WithdrawReason::TurnEnded {
            self.forget_rules(session);
        }
        let ids: Vec<PromptId> = self
            .pending
            .iter()
            .filter(|(_, entry)| entry.request.session == session)
            .map(|(id, _)| *id)
            .collect();
        ids.into_iter()
            .filter_map(|id| self.pending.remove(&id))
            .map(|entry| {
                let _ = entry.answer.send(deny(TURN_ENDED));
                resolved(&entry.request, Resolution::Withdrawn { reason })
            })
            .collect()
    }

    /// Drop `session`'s always-allow rules: the process they were given to has ended (R9).
    pub fn forget_rules(&mut self, session: SessionId) {
        self.rules.remove(&session);
    }

    /// The whole input JSON of request `id`, named as `session`'s (research R10).
    pub fn input_text(&self, session: SessionId, id: PromptId) -> Result<String, DecisionRefusal> {
        let entry = self.pending.get(&id).ok_or(DecisionRefusal::NotPending)?;
        if entry.request.session != session {
            return Err(DecisionRefusal::OtherSession);
        }
        Ok(serde_json::to_string(&entry.call.input).unwrap_or_default())
    }

    /// One `PermissionRequested` per pending request, oldest first, for a client's handshake.
    pub fn replay(&self) -> Vec<DaemonMsg> {
        self.pending
            .values()
            .map(|entry| DaemonMsg::PermissionRequested {
                request: entry.request.clone(),
            })
            .collect()
    }

    /// Whether any request of `session` is pending.
    pub fn has_pending(&self, session: SessionId) -> bool {
        self.pending
            .values()
            .any(|entry| entry.request.session == session)
    }
}

fn resolved(request: &PermissionRequest, resolution: Resolution) -> DaemonMsg {
    DaemonMsg::PermissionResolved {
        session: request.session,
        id: request.id,
        tool_use_id: request.tool_use_id.clone(),
        resolution,
    }
}

/// How a request decided with `choice` ends, and the mode an approved plan sets.
fn resolution_of(choice: &PromptChoice) -> (Resolution, Option<PermissionMode>) {
    match choice {
        PromptChoice::Allow => (Resolution::Allowed, None),
        PromptChoice::AlwaysAllow => (Resolution::AllowedAlways, None),
        PromptChoice::Deny { .. } => (Resolution::Denied, None),
        PromptChoice::ApprovePlan { accept_edits } => {
            let mode = if *accept_edits {
                PermissionMode::AcceptEdits
            } else {
                PermissionMode::Default
            };
            (Resolution::PlanApproved { mode }, Some(mode))
        }
        PromptChoice::KeepPlanning { .. } => (Resolution::KeptPlanning, None),
        PromptChoice::Answer { .. } => (Resolution::Answered, None),
        PromptChoice::DeclineQuestion => (Resolution::Declined, None),
    }
}

/// The `CallToolResult` carrying `answer` as the CLI reads it: `content[0].text` (contracts/
/// claude-permission.md).
pub fn prompt_result(answer: &Value) -> Value {
    json!({"content": [{"type": "text", "text": answer.to_string()}], "isError": false})
}

fn deny(message: &str) -> Value {
    json!({"behavior": "deny", "message": message})
}

/// The prompt tool's `tools/call` from structured session `caller` (R4): a malformed call is
/// answered deny at once (FR-004); else a request is opened, unless a rule answers it, and this
/// waits for its decision or for the agent to hang up (`hangup`), with no deadline (FR-003).
pub async fn serve_prompt(
    state: Arc<DaemonState>,
    caller: SessionId,
    arguments: Value,
    meta: Option<Value>,
    hangup: CancellationToken,
) -> Value {
    let call = match parse_call(&arguments, meta.as_ref()) {
        Ok(call) => call,
        Err(malformed) => {
            // The input is never logged; the tool name says which request it was.
            let tool = arguments.get("tool_name").and_then(Value::as_str);
            tracing::warn!(
                session = %caller.0,
                tool = tool.unwrap_or("<none>"),
                reason = malformed.reason,
                "a malformed permission prompt was denied"
            );
            return prompt_result(&deny(&format!("{MALFORMED}: {}.", malformed.reason)));
        }
    };
    let kind = classify(&call);
    // Watched before the request opens, so an end of the turn after that is not missed.
    let ending = state.prompt_endings().watch(caller);
    let (id, mut receiver) = match state.open_prompt(caller, call, kind) {
        Opened::Answered(answer) => return prompt_result(&answer),
        Opened::Pending { id, receiver, .. } => (id, receiver),
    };
    // However this wait ends — the agent hung up, or the task was dropped — a request still
    // pending is withdrawn from every client; after a decision it finds nothing.
    let _withdraw = WithdrawOnDrop { state: &state, id };
    tokio::select! {
        answer = &mut receiver => prompt_result(&answer.unwrap_or_else(|_| deny(WITHDRAWN))),
        () = hangup.cancelled() => prompt_result(&deny(WITHDRAWN)),
        reason = ending.ended() => {
            // Every request of the session goes, this one's answer among them (a decision that
            // came first stands).
            state.withdraw_session_prompts(caller, reason);
            prompt_result(&receiver.await.unwrap_or_else(|_| deny(TURN_ENDED)))
        }
    }
}

/// The ends of structured sessions' turns and processes, as their stdout readers see them (FR-011,
/// R4), for the prompt calls waiting on those sessions. A reader signals here and never takes the
/// state's lock, which a teardown joining it may hold; each waiting call then withdraws its
/// session's requests.
#[derive(Default)]
pub struct Endings(Mutex<HashMap<SessionId, Ending>>);

/// One coming end of a session's turn or process: fired once, with the reason the first signal
/// gave.
#[derive(Clone, Default)]
pub struct Ending {
    fired: CancellationToken,
    reason: Arc<OnceLock<WithdrawReason>>,
}

impl Endings {
    /// The next end of `session`'s turn or process.
    pub fn watch(&self, session: SessionId) -> Ending {
        self.lock().entry(session).or_default().clone()
    }

    /// `session`'s turn or process ended for `reason`: every [`Ending`] watched so far fires.
    pub fn end(&self, session: SessionId, reason: WithdrawReason) {
        if let Some(ending) = self.lock().remove(&session) {
            let _ = ending.reason.set(reason);
            ending.fired.cancel();
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<SessionId, Ending>> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Ending {
    /// Wait for the end, and say why.
    pub async fn ended(&self) -> WithdrawReason {
        self.fired.cancelled().await;
        *self.reason.get().expect("set before the end fires")
    }
}

/// Withdraws request `id` as `AgentHungUp` when dropped, if it is still pending.
struct WithdrawOnDrop<'a> {
    state: &'a DaemonState,
    id: PromptId,
}

impl Drop for WithdrawOnDrop<'_> {
    fn drop(&mut self) {
        self.state
            .withdraw_prompt(self.id, WithdrawReason::AgentHungUp);
    }
}
