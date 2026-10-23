//! The loopback HTTP hook receiver (contracts/hooks.md, T045).
//!
//! This is the one HTTP listener in an otherwise socket-only application, and it exists for one
//! reason: `claude` delivers its lifecycle hooks (`UserPromptSubmit`, `PreToolUse`, `Stop`, …) over
//! HTTP, and those hooks are the only reliable busy/idle signal (research R4 — PTY scraping was
//! measured and does not work). So it is constrained as tightly as the contract demands:
//!
//! 1. Binds **loopback only** (`127.0.0.1`), on an **ephemeral** port chosen at daemon start.
//! 2. Requires a **per-session bearer token**; a mismatch is a bare `403` that never reveals whether
//!    the session exists.
//! 3. Exposes **no capability** beyond reporting one session's activity, and (feature 490,
//!    `/status/<session-uuid>`) the plan usage that session's status line was given — it is not a
//!    route to the catalog, session input, or project state. A leaked token can lie about one
//!    activity signal, or about the plan's usage.
//! 4. **Bounds** the request head and body and rejects anything larger.
//! 5. **Never logs the body** (it carries transcript paths + prompt metadata — FR-047).
//!
//! The daemon writes a per-session `--settings` file (see [`settings_json`]) pointing `claude` at
//! `http://127.0.0.1:<port>/hook/<session-uuid>` with the token in an `Authorization: Bearer` header,
//! so **user configuration is never modified**.

use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

use crate::activity::{ActivityEvent, HookKind};
use crate::http::{self, respond, respond_json, Body, HeadRead};
use crate::plan_usage::Rejections;
use crate::state::{unix_secs, DaemonState};
use micold_core::agent_run::AgentSignal;
use micold_core::mcp::binding::ConfigLocations;
use micold_core::plan_usage::status_line;
use micold_core::plan_usage::user_status_line::{RelayConfig, UserStatusLine};
use micold_core::session::{AiCli, SessionId};
use micold_core::worktree_guard::paths::RealPathFacts;
use micold_core::worktree_guard::{
    self, decide, GuardInput, GuardVerdict, SessionBinding, ToolCall,
};

/// Longest `agent_id`, `agent_type` or transcript path recorded (feature 552): the runs ride on every
/// catalog frame, so an oversized field is treated as absent.
const MAX_AGENT_FIELD: usize = 1024;

/// Maximum request-body size accepted (contracts/hooks.md §5). Anything larger is rejected with
/// `413` rather than buffered.
///
/// **This bound is a memory guarantee, not a claim about payload size** (BUG-010). A hook payload is
/// *not* small: `PostToolUse` embeds the tool's entire input and response, so for `Edit`/`Write` it
/// carries the edited file's full contents (`tool_response.originalFile`, plus `tool_input`'s
/// old/new strings or `content`). Its size therefore scales with the user's source tree, not with the
/// hook envelope. Measured across 3,332 real payloads from this project's own transcripts: largest
/// 97,087 bytes, next two 68,227 and 67,933 — all of them `specs/**` markdown, which is exactly what
/// this repository edits most. The 64 KiB that shipped here was chosen from the assumption that hook
/// payloads are small JSON objects, and rejected every one of those edits with a `413` that surfaced
/// as an error inside the user's own `claude` session.
///
/// 4 MiB is ~43× the measured maximum, so it absorbs both a much larger file and upstream payload
/// growth, while still bounding what one connection can make the daemon buffer. Only
/// `hook_event_name` — a few dozen bytes — is ever read out of the body.
pub const MAX_BODY: usize = 4 * 1024 * 1024;

/// The bound on a `/status` body (feature 490, contracts/status-line-relay.md S3). The relay posts
/// only the status line's `rate_limits` object, a few hundred bytes.
pub const MAX_STATUS_BODY: usize = 16 * 1024;

/// The shared token registry: session uuid → its per-session bearer secret.
type Tokens = Arc<Mutex<HashMap<Uuid, String>>>;

/// Where a session's own status line is looked up (feature 490, research R4): its working directory
/// and the configuration locations it sees.
#[derive(Debug, Clone)]
pub struct StatusContext {
    /// The session's working directory (its `.claude/settings*.json` come first).
    pub cwd: PathBuf,
    /// Home and `CLAUDE_CONFIG_DIR` as the session sees them.
    pub locations: ConfigLocations,
}

/// A session's worktree guard (feature 1210, data-model): fixed when the session is launched, so a
/// later change of the setting reaches only later launches (research R7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardState {
    Off,
    On {
        binding: SessionBinding,
        project: PathBuf,
    },
}

/// The deny body for `message` (`contracts/pretooluse-guard.md`).
fn deny_body(message: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": message,
        }
    }))
    .expect("a deny body is always serialisable")
}

/// What the guard says to a `PreToolUse` payload of `session`: the deny body, or `None` to answer
/// as usual. Failing open is logged by reason, never with the payload (rule 5).
fn guard_response(
    state: &DaemonState,
    session: SessionId,
    payload: &serde_json::Value,
) -> Option<Vec<u8>> {
    let GuardState::On { binding, project } = state.session_guard(session)? else {
        return None;
    };
    let tool = match ToolCall::from_payload(payload) {
        Ok(ToolCall::Other) => return None,
        Ok(tool) => tool,
        Err(reason) => {
            tracing::warn!(session = %session.0, %reason, "worktree guard allowed a call it could not read");
            return None;
        }
    };
    let input = GuardInput {
        worktrees: state.guard_worktrees(&project),
        session_cwd: worktree_guard::payload_cwd(payload).unwrap_or_else(|| match &binding {
            SessionBinding::Worktree(own) | SessionBinding::Default(own) => own.clone(),
            SessionBinding::Unknown => project.clone(),
        }),
        project_root: project,
        binding,
        tool,
    };
    match decide(&input, &RealPathFacts) {
        GuardVerdict::Allow => None,
        GuardVerdict::AllowUnknown { reason } => {
            tracing::warn!(session = %session.0, %reason, "worktree guard allowed a call it could not judge");
            None
        }
        GuardVerdict::Deny { message, target } => {
            tracing::info!(session = %session.0, target = %target.display(), "worktree guard denied a call");
            Some(deny_body(&message))
        }
    }
}

/// The loopback hook receiver: the bound address, the per-session token registry, and the directory
/// the per-session `--settings` files are written to.
pub struct HookReceiver {
    addr: SocketAddr,
    tokens: Tokens,
    settings_dir: PathBuf,
    /// The executable Claude Code runs as the status-line relay: this daemon (research R3).
    relay_exe: PathBuf,
    /// The plan-usage switch as last applied, and each prepared session's status context, so
    /// turning the switch can rewrite its settings file (research R9). One lock over both, held
    /// while files are written, so a switch, a prepare and a forget never interleave.
    prepared: Mutex<Prepared>,
}

/// See [`HookReceiver::prepared`].
#[derive(Default)]
struct Prepared {
    plan_usage: bool,
    sessions: HashMap<Uuid, StatusContext>,
}

impl HookReceiver {
    /// Bind the receiver to an ephemeral loopback port and return it alongside the [`TcpListener`]
    /// the [`serve`] loop consumes. Binding is `127.0.0.1:0` — never `0.0.0.0` — so nothing leaves
    /// the device (contract §1/§2). `settings_dir` is where per-session settings files are written.
    pub async fn bind(settings_dir: PathBuf) -> io::Result<(Self, TcpListener)> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let addr = listener.local_addr()?;
        let receiver = Self {
            addr,
            tokens: Arc::new(Mutex::new(HashMap::new())),
            settings_dir,
            relay_exe: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("micold-daemon")),
            prepared: Mutex::new(Prepared {
                plan_usage: micold_core::plan_usage::DEFAULT_PLAN_USAGE_ENABLED,
                sessions: HashMap::new(),
            }),
        };
        Ok((receiver, listener))
    }

    /// A clone of the token registry, for the [`serve`] loop.
    pub fn tokens(&self) -> Tokens {
        Arc::clone(&self.tokens)
    }

    /// Register (idempotently) a per-session token and return it. The same session always gets the
    /// same token for its lifetime, so a respawn reuses the already-written settings file.
    pub fn token_for(&self, session: SessionId) -> String {
        self.tokens
            .lock()
            .expect("hook tokens lock poisoned")
            .entry(session.0)
            .or_insert_with(|| Uuid::new_v4().simple().to_string())
            .clone()
    }

    /// Drop a session's token (on session end) so a stale token can no longer report activity, and
    /// delete its status-line relay file, which holds that token (feature 490).
    pub fn forget(&self, session: SessionId) {
        self.tokens
            .lock()
            .expect("hook tokens lock poisoned")
            .remove(&session.0);
        let mut prepared = self.prepared.lock().expect("hook contexts lock poisoned");
        prepared.sessions.remove(&session.0);
        let _ = std::fs::remove_file(self.relay_file(session));
    }

    /// The hook URL for a session: `http://127.0.0.1:<port>/hook/<session-uuid>`.
    pub fn url(&self, session: SessionId) -> String {
        format!("http://{}/hook/{}", self.addr, session.0)
    }

    /// The status route for a session: `http://127.0.0.1:<port>/status/<session-uuid>` (feature 490).
    pub fn status_url(&self, session: SessionId) -> String {
        format!("http://{}/status/{}", self.addr, session.0)
    }

    /// A session's `--settings` file.
    pub fn settings_file(&self, session: SessionId) -> PathBuf {
        self.settings_dir
            .join(format!("{}.json", session.0.simple()))
    }

    /// A session's status-line relay file, beside its settings file (feature 490).
    pub fn relay_file(&self, session: SessionId) -> PathBuf {
        self.settings_dir
            .join(format!("{}.status.json", session.0.simple()))
    }

    /// Prepare a session's `--settings` file: register its token, write the settings JSON pointing
    /// `claude` at this receiver, and return the file path to pass as `--settings`. Returns the path
    /// so the spawn path can hand it to `claude`. Best-effort: a write failure is surfaced to the
    /// caller, which spawns without hooks rather than failing the session (activity degrades to
    /// `Unknown`, never wrong — H1).
    ///
    /// While the plan-usage switch is on (feature 490, [`Self::set_plan_usage`]), the file also
    /// names the status-line relay, and the relay file is written; off, neither is there
    /// (contract S1).
    pub fn prepare_settings(
        &self,
        session: SessionId,
        status: StatusContext,
    ) -> io::Result<PathBuf> {
        let mut prepared = self.prepared.lock().expect("hook contexts lock poisoned");
        let path = self.write_settings(session, &status, prepared.plan_usage)?;
        prepared.sessions.insert(session.0, status);
        Ok(path)
    }

    /// Apply the plan-usage switch: sessions prepared from now on follow it, and the settings file
    /// of every prepared session is rewritten for it, writing or deleting its relay file (feature
    /// 490, research R9). A session whose file cannot be written keeps its old one; Claude Code
    /// picks up the change when it reloads its settings.
    pub fn set_plan_usage(&self, on: bool) {
        let mut prepared = self.prepared.lock().expect("hook contexts lock poisoned");
        prepared.plan_usage = on;
        for (id, status) in &prepared.sessions {
            if let Err(e) = self.write_settings(SessionId(*id), status, on) {
                tracing::warn!(session = %id, error = %e, "could not rewrite hook settings for the plan-usage switch");
            }
        }
    }

    fn write_settings(
        &self,
        session: SessionId,
        status: &StatusContext,
        plan_usage: bool,
    ) -> io::Result<PathBuf> {
        let token = self.token_for(session);
        let url = self.url(session);
        std::fs::create_dir_all(&self.settings_dir)?;
        let relay_file = self.relay_file(session);
        let block = if plan_usage {
            let user = UserStatusLine::resolve(&status.locations, &status.cwd);
            let config = RelayConfig {
                url: self.status_url(session),
                token: token.clone(),
                user_command: user.as_ref().map(|line| line.command.clone()),
            };
            let name = relay_file
                .file_name()
                .and_then(|n| n.to_str())
                .expect("the relay file name is ASCII");
            micold_core::owner_only::write(
                &self.settings_dir,
                name,
                &serde_json::to_vec(&config).expect("relay config is always serialisable"),
            )?;
            Some(StatusLineBlock {
                kind: "command",
                command: format!(
                    "{} status-line {}",
                    shell_quoted(&self.relay_exe),
                    shell_quoted(&relay_file)
                ),
                padding: user.as_ref().and_then(|line| line.padding),
                refresh_interval: user.as_ref().and_then(|line| line.refresh_interval),
            })
        } else {
            match std::fs::remove_file(&relay_file) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
                _ => None,
            }
        };
        let path = self.settings_file(session);
        std::fs::write(&path, settings_json(&url, &token, block.as_ref()))?;
        Ok(path)
    }
}

/// A path for the status-line command: double-quoted, with `/` separators on every platform, as
/// Claude Code runs the command through Git Bash on Windows (contract S1.2).
fn shell_quoted(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('\\', "/"))
}

/// The accept loop: one bounded, token-checked request handler per connection. Runs for the daemon's
/// lifetime. An accept error is transient (a peer that vanished mid-handshake) and is skipped rather
/// than tearing the loop down.
pub async fn serve(listener: TcpListener, tokens: Tokens, state: Arc<DaemonState>) {
    let rejections = state.plan_usage_rejections().clone();
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _peer)) => stream,
            Err(_) => continue,
        };
        let tokens = Arc::clone(&tokens);
        let state = Arc::clone(&state);
        let rejections = rejections.clone();
        tokio::spawn(async move {
            // A handler error is a malformed/hostile request; it never propagates.
            let _ = handle_connection(stream, tokens, state, rejections).await;
        });
    }
}

/// The two routes: lifecycle hooks, and (feature 490) the plan usage of a session's status line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Route {
    /// `/hook/<session-uuid>`: an activity hook.
    Hook,
    /// `/status/<session-uuid>`: a status line's `rate_limits` (contracts/status-line-relay.md S3).
    Status,
}

/// The classification of a hook payload's `hook_event_name`.
#[derive(Debug, PartialEq, Eq)]
enum HookClass {
    /// A recognised turn-affecting hook — apply it to the FSM.
    Event(HookKind, serde_json::Value),
    /// A sub-agent run started or stopped (feature 552, contract C2): recorded against the origin
    /// session, never applied to its activity (FR-007).
    Agent(AgentSignal),
    /// A well-formed hook that carries no FSM transition (e.g. `SessionStart`) — accept, do nothing.
    Ignored,
    /// The body was not valid JSON or lacked a usable `hook_event_name` — reject with `400`.
    Invalid,
}

/// Handle one connection: read a bounded head + body, authenticate the per-session token, and
/// apply it — a hook to the session's activity, a status body to the plan's usage. Writes exactly
/// one HTTP response, then closes.
async fn handle_connection(
    mut stream: TcpStream,
    tokens: Tokens,
    state: Arc<DaemonState>,
    rejections: Rejections,
) -> io::Result<()> {
    // 1. Read the head, bounded, up to the blank line that terminates it.
    let (head, rest) = match http::read_head(&mut stream).await? {
        HeadRead::Complete { head, rest } => (head, rest),
        HeadRead::TooLarge => {
            return respond(&mut stream, 431, "Request Header Fields Too Large").await
        }
        HeadRead::Malformed => return respond(&mut stream, 400, "Bad Request").await,
        // Connection closed before a complete head — nothing to answer.
        HeadRead::Closed => return Ok(()),
    };

    // 2. Only POST /hook/<uuid> and POST /status/<uuid> are routes; anything else is a flat 404
    //    (no capability surface).
    if head.method != "POST" {
        return respond(&mut stream, 405, "Method Not Allowed").await;
    }
    let Some((route, session_uuid)) = route_from_path(&head.path) else {
        return respond(&mut stream, 404, "Not Found").await;
    };

    // 3. Authenticate BEFORE reading the body — and before the size check, so an unauthenticated
    //    caller cannot distinguish "too large" from "forbidden" (contract §7). A missing/mismatched
    //    token is a bare 403 that never discloses whether the session exists (contract §3).
    let authorized = {
        let tokens = tokens.lock().expect("hook tokens lock poisoned");
        matches!((tokens.get(&session_uuid), &head.bearer), (Some(expected), Some(got)) if expected == got)
    };
    if !authorized {
        // Logged only for a session this receiver serves: a made-up uuid must not grow the set.
        let known = tokens
            .lock()
            .expect("hook tokens lock poisoned")
            .contains_key(&session_uuid);
        if route == Route::Status && known {
            rejections.reject(session_uuid, "forbidden");
        }
        return respond(&mut stream, 403, "Forbidden").await;
    }

    // 4. Bound and read the body — after authentication, but still before a byte of an over-bound
    //    body is buffered (it is drained, never kept).
    let limit = match route {
        Route::Hook => MAX_BODY,
        Route::Status => MAX_STATUS_BODY,
    };
    let body = match http::read_body(&mut stream, rest, head.content_length, limit).await? {
        Body::Complete(body) => body,
        Body::TooLarge => {
            if route == Route::Status {
                rejections.reject(session_uuid, "too large");
            }
            return respond(&mut stream, 413, "Payload Too Large").await;
        }
    };
    let session = SessionId::from_uuid(session_uuid);
    if route == Route::Status {
        let (status, reason) = plan_usage(&state, &rejections, session, &body);
        return respond(&mut stream, status, reason).await;
    }

    // 6. Map the hook to an activity transition. The body is NEVER logged (contract §6).
    let body_str = std::str::from_utf8(&body).unwrap_or("");
    match classify_hook(body_str) {
        HookClass::Event(kind, value) => {
            // Both are called, never short-circuited: each folds this event in.
            let activity = state.note_activity(session, ActivityEvent::Hook(kind));
            let hook_activity = state.note_hook_activity(session, &value);
            if activity || hook_activity {
                state.broadcast_catalog();
            }
            if kind == HookKind::PreToolUse {
                // It reads the disk to resolve symlinks, so off the async workers.
                let guarded = {
                    let state = Arc::clone(&state);
                    tokio::task::spawn_blocking(move || guard_response(&state, session, &value))
                        .await
                        .ok()
                        .flatten()
                };
                if let Some(body) = guarded {
                    return respond_json(&mut stream, 200, "OK", &body).await;
                }
            }
            respond(&mut stream, 200, "OK").await
        }
        HookClass::Agent(signal) => {
            if state.note_agent_signal(session, signal) {
                state.broadcast_catalog();
            }
            respond(&mut stream, 200, "OK").await
        }
        HookClass::Ignored => respond(&mut stream, 200, "OK").await,
        HookClass::Invalid => respond(&mut stream, 400, "Bad Request").await,
    }
}

/// Apply a `/status` body to the plan's usage (feature 490, S3) and return the answer: `400` for a
/// body that is not a JSON object, else `200`, with the reading kept only when it has a usable
/// window and plan usage is on (S3.2). Every window is told of a reading the service keeps (W2).
/// The body is never logged (S3.1).
fn plan_usage(
    state: &DaemonState,
    rejections: &Rejections,
    session: SessionId,
    body: &[u8],
) -> (u16, &'static str) {
    const OK: (u16, &str) = (200, "OK");
    let body: serde_json::Value = match serde_json::from_slice(body) {
        Ok(object @ serde_json::Value::Object(_)) => object,
        _ => {
            rejections.reject(session.0, "malformed");
            return (400, "Bad Request");
        }
    };
    let Some(reading) = status_line::parse(&body, unix_secs()) else {
        rejections.reject(session.0, "no usable window");
        return OK;
    };
    if !state.plan_usage_enabled() {
        rejections.reject(session.0, "plan usage is off");
        return OK;
    }
    state.offer_plan_usage(session, reading);
    OK
}

/// Extract the route and session uuid from a `/hook/<uuid>` or `/status/<uuid>` path (ignoring any
/// query string). `None` if the path is neither or the segment is not a uuid.
fn route_from_path(path: &str) -> Option<(Route, Uuid)> {
    let path = path.split('?').next().unwrap_or(path);
    let (route, rest) = if let Some(rest) = path.strip_prefix("/hook/") {
        (Route::Hook, rest)
    } else {
        (Route::Status, path.strip_prefix("/status/")?)
    };
    // Exactly one path segment — no deeper routes exist.
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Some((route, Uuid::parse_str(rest).ok()?))
}

/// Classify a hook payload by its `hook_event_name` (contracts/hooks.md state table). Uses a tolerant
/// substring scan for the field so it does not depend on a full JSON model of claude's evolving hook
/// schema — only the event name matters, and the body is otherwise ignored.
fn classify_hook(body: &str) -> HookClass {
    let value: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return HookClass::Invalid,
    };
    let Some(name) = value.get("hook_event_name").and_then(|v| v.as_str()) else {
        return HookClass::Invalid;
    };
    match name {
        "UserPromptSubmit" => HookClass::Event(HookKind::UserPromptSubmit, value),
        "PreToolUse" => HookClass::Event(HookKind::PreToolUse, value),
        "PostToolUse" => HookClass::Event(HookKind::PostToolUse, value),
        "Stop" => HookClass::Event(HookKind::Stop, value),
        "Notification" => HookClass::Event(HookKind::Notification, value),
        // SessionStart is well-formed but carries no FSM transition (the session starts Unknown).
        "SessionStart" => HookClass::Ignored,
        // A helper agent starting or finishing inside a turn neither ends nor pauses it (feature
        // 613, FR-024, C17; feature 552, FR-007): it is recorded as an agent run, and the origin's
        // activity is not touched. A payload with no usable `agent_id` records nothing (FR-008).
        "SubagentStart" | "SubagentStop" => {
            let text = |key: &str| {
                value
                    .get(key)
                    .and_then(|v| v.as_str())
                    .filter(|v| !v.is_empty() && v.len() <= MAX_AGENT_FIELD)
                    .map(str::to_owned)
            };
            let Some(agent_id) = text("agent_id") else {
                return HookClass::Ignored;
            };
            let provider = AiCli::ClaudeCode;
            let agent_type = text("agent_type");
            HookClass::Agent(if name == "SubagentStart" {
                AgentSignal::Started {
                    provider,
                    agent_id,
                    agent_type,
                }
            } else {
                AgentSignal::Stopped {
                    provider,
                    agent_id,
                    agent_type,
                    transcript: text("agent_transcript_path"),
                }
            })
        }
        // An unrecognised but structurally valid hook: accept it without inventing a transition.
        _ => HookClass::Ignored,
    }
}

/// An `http`-type hook entry (contracts/hooks.md §Configuration): posts the event JSON to `url`
/// with a bearer token, rather than running a local command.
#[derive(serde::Serialize)]
struct HttpHook<'a> {
    #[serde(rename = "type")]
    kind: &'static str,
    url: &'a str,
    headers: HttpHookHeaders,
}

#[derive(serde::Serialize)]
struct HttpHookHeaders {
    #[serde(rename = "Authorization")]
    authorization: String,
}

/// The matcher-group wrapper every Claude Code hook type requires around its entries — a bare hook
/// object placed directly in an event's array is rejected by the settings loader (BUG-001).
#[derive(serde::Serialize)]
struct MatcherGroup<'a> {
    matcher: &'static str,
    hooks: [HttpHook<'a>; 1],
}

/// The `hooks` map of the per-session `--settings` file: one matcher-group array per lifecycle
/// event this daemon's activity FSM (`classify_hook`) understands — omitting an event here means
/// `claude` never POSTs it at all, regardless of what `classify_hook` is prepared to handle.
/// `SubagentStart` and `SubagentStop` are registered (feature 552, C1) to record agent runs; they
/// never reach the FSM, as a helper agent finishing is not the end of a turn (feature 613, FR-024).
#[derive(serde::Serialize)]
struct HooksMap<'a> {
    #[serde(rename = "SessionStart")]
    session_start: [MatcherGroup<'a>; 1],
    #[serde(rename = "UserPromptSubmit")]
    user_prompt_submit: [MatcherGroup<'a>; 1],
    #[serde(rename = "PreToolUse")]
    pre_tool_use: [MatcherGroup<'a>; 1],
    #[serde(rename = "PostToolUse")]
    post_tool_use: [MatcherGroup<'a>; 1],
    #[serde(rename = "Stop")]
    stop: [MatcherGroup<'a>; 1],
    #[serde(rename = "Notification")]
    notification: [MatcherGroup<'a>; 1],
    #[serde(rename = "SubagentStart")]
    subagent_start: [MatcherGroup<'a>; 1],
    #[serde(rename = "SubagentStop")]
    subagent_stop: [MatcherGroup<'a>; 1],
}

/// The `statusLine` block of the per-session settings (feature 490, contract S1).
#[derive(Debug, serde::Serialize)]
pub struct StatusLineBlock {
    #[serde(rename = "type")]
    kind: &'static str,
    command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    padding: Option<u32>,
    #[serde(rename = "refreshInterval", skip_serializing_if = "Option::is_none")]
    refresh_interval: Option<u32>,
}

/// `permissions`: only `deny` rules, so the project's own rules stay in effect (FR-013).
#[derive(serde::Serialize)]
struct PermissionsBlock {
    deny: &'static [&'static str],
}

#[derive(serde::Serialize)]
struct SettingsDoc<'a> {
    hooks: HooksMap<'a>,
    permissions: PermissionsBlock,
    #[serde(rename = "statusLine", skip_serializing_if = "Option::is_none")]
    status_line: Option<&'a StatusLineBlock>,
}

/// The per-session `--settings` JSON that points `claude` at this receiver (contracts/hooks.md
/// §Configuration). Every lifecycle hook posts to the same per-session URL carrying the bearer token
/// in a header, so **user configuration is never modified**. Built from typed structs rather than
/// untyped `serde_json::json!` so a key typo is a compile error, not a silent shape mismatch caught
/// only against a live `claude` binary (BUG-001).
pub fn settings_json(url: &str, token: &str, status_line: Option<&StatusLineBlock>) -> String {
    let group = || MatcherGroup {
        matcher: "",
        hooks: [HttpHook {
            kind: "http",
            url,
            headers: HttpHookHeaders {
                authorization: format!("Bearer {token}"),
            },
        }],
    };
    let doc = SettingsDoc {
        hooks: HooksMap {
            session_start: [group()],
            user_prompt_submit: [group()],
            pre_tool_use: [group()],
            post_tool_use: [group()],
            stop: [group()],
            notification: [group()],
            subagent_start: [group()],
            subagent_stop: [group()],
        },
        permissions: PermissionsBlock {
            deny: &micold_core::worktree_guard::DENY_RULES,
        },
        status_line,
    };
    serde_json::to_string_pretty(&doc).expect("settings json is always serialisable")
}

/// The default per-session settings directory under the daemon's data dir. Falls back to the system
/// temp dir if no data dir resolves (e.g. a headless CI environment).
pub fn default_settings_dir() -> PathBuf {
    micold_core::paths::data_subdir("hooks")
        .unwrap_or_else(|| std::env::temp_dir().join("micold-daemon-hooks"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session_id_from_path(path: &str) -> Option<Uuid> {
        route_from_path(path).map(|(_, uuid)| uuid)
    }

    #[test]
    fn extracts_the_session_uuid_from_the_path() {
        let uuid = Uuid::from_u128(0x1234);
        let path = format!("/hook/{uuid}");
        assert_eq!(session_id_from_path(&path), Some(uuid));
        // A query string is ignored.
        let with_query = format!("/hook/{uuid}?x=1");
        assert_eq!(session_id_from_path(&with_query), Some(uuid));
    }

    #[test]
    fn names_the_route_of_the_path() {
        let uuid = Uuid::from_u128(0x490);
        assert_eq!(
            route_from_path(&format!("/hook/{uuid}")),
            Some((Route::Hook, uuid))
        );
        assert_eq!(
            route_from_path(&format!("/status/{uuid}")),
            Some((Route::Status, uuid))
        );
        assert_eq!(route_from_path("/status/"), None);
        assert_eq!(route_from_path(&format!("/status/{uuid}/extra")), None);
        assert_eq!(route_from_path(&format!("/usage/{uuid}")), None);
    }

    #[test]
    fn rejects_non_hook_and_deep_paths() {
        assert_eq!(session_id_from_path("/catalog"), None);
        assert_eq!(session_id_from_path("/hook/"), None);
        assert_eq!(session_id_from_path("/hook/not-a-uuid"), None);
        // No deeper routes: a token-in-path style is not accepted here.
        let uuid = Uuid::from_u128(1);
        assert_eq!(session_id_from_path(&format!("/hook/{uuid}/extra")), None);
    }

    #[test]
    fn classifies_hook_event_names() {
        // The body rides along with the kind (feature 813); compare the kind alone.
        let ev = |n: &str| match classify_hook(&format!(r#"{{"hook_event_name":"{n}"}}"#)) {
            HookClass::Event(kind, _) => HookClass::Event(kind, serde_json::Value::Null),
            other => other,
        };
        let event = |kind| HookClass::Event(kind, serde_json::Value::Null);
        assert_eq!(ev("UserPromptSubmit"), event(HookKind::UserPromptSubmit));
        assert_eq!(ev("PreToolUse"), event(HookKind::PreToolUse));
        assert_eq!(ev("PostToolUse"), event(HookKind::PostToolUse));
        assert_eq!(ev("Stop"), event(HookKind::Stop));
        assert_eq!(ev("Notification"), event(HookKind::Notification));
        // A helper agent starting or finishing is not the end of the turn (feature 613, FR-024);
        // with no `agent_id` there is nothing to record (feature 552, FR-008).
        assert_eq!(ev("SubagentStop"), HookClass::Ignored);
        assert_eq!(ev("SubagentStart"), HookClass::Ignored);
        // SessionStart and unknown-but-valid hooks are accepted without a transition.
        assert_eq!(ev("SessionStart"), HookClass::Ignored);
        assert_eq!(ev("SomethingNew"), HookClass::Ignored);
    }

    #[test]
    fn classifies_sub_agent_reports_into_agent_signals() {
        let start = classify_hook(
            r#"{"hook_event_name":"SubagentStart","agent_id":"a1","agent_type":"Explore"}"#,
        );
        assert_eq!(
            start,
            HookClass::Agent(AgentSignal::Started {
                provider: AiCli::ClaudeCode,
                agent_id: "a1".into(),
                agent_type: Some("Explore".into()),
            })
        );
        let stop = classify_hook(
            r#"{"hook_event_name":"SubagentStop","agent_id":"a1","agent_transcript_path":"/t/a1.jsonl","last_assistant_message":"x"}"#,
        );
        assert_eq!(
            stop,
            HookClass::Agent(AgentSignal::Stopped {
                provider: AiCli::ClaudeCode,
                agent_id: "a1".into(),
                agent_type: None,
                transcript: Some("/t/a1.jsonl".into()),
            })
        );
    }

    #[test]
    fn a_sub_agent_report_without_a_usable_agent_id_is_ignored() {
        for body in [
            r#"{"hook_event_name":"SubagentStart"}"#,
            r#"{"hook_event_name":"SubagentStart","agent_id":""}"#,
            r#"{"hook_event_name":"SubagentStop","agent_id":7}"#,
            r#"{"hook_event_name":"SubagentStop","agent_id":null}"#,
        ] {
            assert_eq!(classify_hook(body), HookClass::Ignored, "{body}");
        }
    }

    #[test]
    fn rejects_non_json_and_fieldless_bodies() {
        assert_eq!(classify_hook("not json"), HookClass::Invalid);
        assert_eq!(classify_hook("{}"), HookClass::Invalid);
    }

    #[test]
    fn a_guarded_sessions_settings_keep_the_pre_tool_use_hook_and_override_nothing() {
        let json = settings_json("http://127.0.0.1:5000/hook/abc", "tok", None);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["hooks"]["PreToolUse"][0]["hooks"][0]["type"], "http");
        // FR-013: only the hooks the session already had; no `allow`, no override key.
        let keys: Vec<&str> = parsed
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(keys, ["hooks", "permissions"]);
        assert!(!json.contains("\"allow\""));
    }

    #[test]
    fn settings_json_denies_worktree_entry_and_creation_but_not_create_worktree() {
        let json = settings_json("http://127.0.0.1:5000/hook/abc", "tok", None);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(
            parsed["permissions"]["deny"],
            serde_json::json!([
                "EnterWorktree",
                "Bash(git worktree add:*)",
                "Bash(git * worktree add*)"
            ])
        );
        assert!(!json.contains("mcp__micold__create_worktree"));
        assert!(!json.contains("\"allow\""));
    }

    #[test]
    fn the_deny_body_follows_the_contract() {
        let body: serde_json::Value = serde_json::from_slice(&deny_body("Blocked: x")).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": "Blocked: x",
            }})
        );
    }

    #[test]
    fn settings_json_embeds_the_url_and_bearer_token() {
        let json = settings_json("http://127.0.0.1:5000/hook/abc", "tok-123", None);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // The event's array entry is a matcher-group wrapper (BUG-001) — the `http` hook itself is
        // nested under `hooks`, not placed directly in the top-level array.
        let group = &parsed["hooks"]["UserPromptSubmit"][0];
        let entry = &group["hooks"][0];
        assert_eq!(entry["type"], "http");
        assert_eq!(entry["url"], "http://127.0.0.1:5000/hook/abc");
        assert_eq!(entry["headers"]["Authorization"], "Bearer tok-123");
        // Every lifecycle hook the FSM cares about is wired, and every one of them uses the
        // matcher-group shape Claude Code's settings loader requires (BUG-001): each top-level
        // array entry has a `matcher` field and a non-empty `hooks` array.
        for hook in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "Stop",
            "Notification",
            "SubagentStart",
            "SubagentStop",
        ] {
            assert!(
                parsed["hooks"][hook].is_array(),
                "{hook} must be configured"
            );
            let group = &parsed["hooks"][hook][0];
            assert!(
                group.get("matcher").is_some(),
                "{hook}'s array entry must be a matcher-group object, not a bare hook"
            );
            assert!(
                group["hooks"].as_array().is_some_and(|h| !h.is_empty()),
                "{hook}'s matcher-group must have a non-empty nested hooks array"
            );
        }
    }
}
