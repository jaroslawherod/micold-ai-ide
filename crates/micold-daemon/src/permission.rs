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
//! [`serve_prompt`] is the waiting half: the prompt tool's `tools/call` handler.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

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
    let (id, receiver) = match state.open_prompt(caller, call, kind) {
        Opened::Answered(answer) => return prompt_result(&answer),
        Opened::Pending { id, receiver, .. } => (id, receiver),
    };
    // However this wait ends — the agent hung up, or the task was dropped — a request still
    // pending is withdrawn from every client; after a decision it finds nothing.
    let _withdraw = WithdrawOnDrop { state: &state, id };
    tokio::select! {
        answer = receiver => prompt_result(&answer.unwrap_or_else(|_| deny(WITHDRAWN))),
        () = hangup.cancelled() => prompt_result(&deny(WITHDRAWN)),
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
