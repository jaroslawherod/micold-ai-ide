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
//! 3. Exposes **no capability** beyond reporting one session's activity — it is not a route to the
//!    catalog, session input, or project state. A leaked token can lie about one activity signal.
//! 4. **Bounds** the request head and body and rejects anything larger.
//! 5. **Never logs the body** (it carries transcript paths + prompt metadata — FR-047).
//!
//! The daemon writes a per-session `--settings` file (see [`settings_json`]) pointing `claude` at
//! `http://127.0.0.1:<port>/hook/<session-uuid>` with the token in an `Authorization: Bearer` header,
//! so **user configuration is never modified**.

use std::collections::HashMap;
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

use crate::activity::{ActivityEvent, HookKind};
use crate::http::{self, respond, Body, HeadRead};
use crate::state::DaemonState;
use micold_core::session::SessionId;

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

/// The shared token registry: session uuid → its per-session bearer secret.
type Tokens = Arc<Mutex<HashMap<Uuid, String>>>;

/// The loopback hook receiver: the bound address, the per-session token registry, and the directory
/// the per-session `--settings` files are written to.
pub struct HookReceiver {
    addr: SocketAddr,
    tokens: Tokens,
    settings_dir: PathBuf,
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

    /// Drop a session's token (on session end) so a stale token can no longer report activity.
    pub fn forget(&self, session: SessionId) {
        self.tokens
            .lock()
            .expect("hook tokens lock poisoned")
            .remove(&session.0);
    }

    /// The hook URL for a session: `http://127.0.0.1:<port>/hook/<session-uuid>`.
    pub fn url(&self, session: SessionId) -> String {
        format!("http://{}/hook/{}", self.addr, session.0)
    }

    /// Prepare a session's `--settings` file: register its token, write the settings JSON pointing
    /// `claude` at this receiver, and return the file path to pass as `--settings`. Returns the path
    /// so the spawn path can hand it to `claude`. Best-effort: a write failure is surfaced to the
    /// caller, which spawns without hooks rather than failing the session (activity degrades to
    /// `Unknown`, never wrong — H1).
    pub fn prepare_settings(&self, session: SessionId) -> io::Result<PathBuf> {
        let token = self.token_for(session);
        let url = self.url(session);
        std::fs::create_dir_all(&self.settings_dir)?;
        let path = self
            .settings_dir
            .join(format!("{}.json", session.0.simple()));
        std::fs::write(&path, settings_json(&url, &token))?;
        Ok(path)
    }
}

/// The accept loop: one bounded, token-checked request handler per connection. Runs for the daemon's
/// lifetime. An accept error is transient (a peer that vanished mid-handshake) and is skipped rather
/// than tearing the loop down.
pub async fn serve(listener: TcpListener, tokens: Tokens, state: Arc<DaemonState>) {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _peer)) => stream,
            Err(_) => continue,
        };
        let tokens = Arc::clone(&tokens);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            // A handler error is a malformed/hostile request; it never propagates.
            let _ = handle_connection(stream, tokens, state).await;
        });
    }
}

/// The classification of a hook payload's `hook_event_name`.
#[derive(Debug, PartialEq, Eq)]
enum HookClass {
    /// A recognised turn-affecting hook — apply it to the FSM.
    Event(HookKind),
    /// `SessionStart`: no FSM transition (the session starts `Unknown`), but the session is now
    /// ready for its first prompt (feature 034, FR-017).
    SessionStart,
    /// A well-formed hook that carries no FSM transition — accept, do nothing.
    Ignored,
    /// The body was not valid JSON or lacked a usable `hook_event_name` — reject with `400`.
    Invalid,
}

/// Handle one connection: read a bounded head + body, authenticate the per-session token, map the
/// hook to an [`ActivityEvent`], and apply it. Writes exactly one HTTP response, then closes.
async fn handle_connection(
    mut stream: TcpStream,
    tokens: Tokens,
    state: Arc<DaemonState>,
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

    // 2. Only POST /hook/<uuid> is a route; anything else is a flat 404 (no capability surface).
    if head.method != "POST" {
        return respond(&mut stream, 405, "Method Not Allowed").await;
    }
    let Some(session_uuid) = session_id_from_path(&head.path) else {
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
        return respond(&mut stream, 403, "Forbidden").await;
    }

    // 4. Bound and read the body — after authentication, but still before a byte of an over-bound
    //    body is buffered (it is drained, never kept).
    let body = match http::read_body(&mut stream, rest, head.content_length, MAX_BODY).await? {
        Body::Complete(body) => body,
        Body::TooLarge => return respond(&mut stream, 413, "Payload Too Large").await,
    };

    // 6. Map the hook to an activity transition. The body is NEVER logged (contract §6).
    let body_str = std::str::from_utf8(&body).unwrap_or("");
    match classify_hook(body_str) {
        HookClass::Event(kind) => {
            let session = SessionId::from_uuid(session_uuid);
            if state.note_activity(session, ActivityEvent::Hook(kind)) {
                state.broadcast_catalog();
            }
            respond(&mut stream, 200, "OK").await
        }
        HookClass::SessionStart => {
            state.mark_ready_for_input(SessionId::from_uuid(session_uuid));
            respond(&mut stream, 200, "OK").await
        }
        HookClass::Ignored => respond(&mut stream, 200, "OK").await,
        HookClass::Invalid => respond(&mut stream, 400, "Bad Request").await,
    }
}

/// Extract the session uuid from a `/hook/<uuid>` path (ignoring any query string). `None` if the
/// path is not a hook route or the segment is not a uuid.
fn session_id_from_path(path: &str) -> Option<Uuid> {
    let path = path.split('?').next().unwrap_or(path);
    let rest = path.strip_prefix("/hook/")?;
    // Exactly one path segment — no deeper routes exist.
    if rest.is_empty() || rest.contains('/') {
        return None;
    }
    Uuid::parse_str(rest).ok()
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
        "UserPromptSubmit" => HookClass::Event(HookKind::UserPromptSubmit),
        "PreToolUse" => HookClass::Event(HookKind::PreToolUse),
        "PostToolUse" => HookClass::Event(HookKind::PostToolUse),
        "Stop" | "SubagentStop" => HookClass::Event(HookKind::Stop),
        "Notification" => HookClass::Event(HookKind::Notification),
        // No FSM transition (the session starts Unknown); it marks the session ready for input.
        "SessionStart" => HookClass::SessionStart,
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
/// event this daemon's activity FSM (`activity.rs::classify_hook`) understands, including
/// `SubagentStop` (grouped with `Stop` there) — omitting an event here means `claude` never POSTs
/// it at all, regardless of what `classify_hook` is prepared to handle.
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
    #[serde(rename = "SubagentStop")]
    subagent_stop: [MatcherGroup<'a>; 1],
    #[serde(rename = "Notification")]
    notification: [MatcherGroup<'a>; 1],
}

#[derive(serde::Serialize)]
struct SettingsDoc<'a> {
    hooks: HooksMap<'a>,
}

/// The per-session `--settings` JSON that points `claude` at this receiver (contracts/hooks.md
/// §Configuration). Every lifecycle hook posts to the same per-session URL carrying the bearer token
/// in a header, so **user configuration is never modified**. Built from typed structs rather than
/// untyped `serde_json::json!` so a key typo is a compile error, not a silent shape mismatch caught
/// only against a live `claude` binary (BUG-001).
pub fn settings_json(url: &str, token: &str) -> String {
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
            subagent_stop: [group()],
            notification: [group()],
        },
    };
    serde_json::to_string_pretty(&doc).expect("settings json is always serialisable")
}

/// The default per-session settings directory under the daemon's data dir. Falls back to the system
/// temp dir if no data dir resolves (e.g. a headless CI environment).
pub fn default_settings_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|dirs| dirs.data_dir().join("hooks"))
        .unwrap_or_else(|| std::env::temp_dir().join("micold-daemon-hooks"))
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let ev = |n: &str| classify_hook(&format!(r#"{{"hook_event_name":"{n}"}}"#));
        assert_eq!(
            ev("UserPromptSubmit"),
            HookClass::Event(HookKind::UserPromptSubmit)
        );
        assert_eq!(ev("PreToolUse"), HookClass::Event(HookKind::PreToolUse));
        assert_eq!(ev("PostToolUse"), HookClass::Event(HookKind::PostToolUse));
        assert_eq!(ev("Stop"), HookClass::Event(HookKind::Stop));
        assert_eq!(ev("Notification"), HookClass::Event(HookKind::Notification));
        // SessionStart marks readiness (feature 034), not a transition; unknown-but-valid hooks are
        // accepted without either.
        assert_eq!(ev("SessionStart"), HookClass::SessionStart);
        assert_eq!(ev("SomethingNew"), HookClass::Ignored);
    }

    #[test]
    fn rejects_non_json_and_fieldless_bodies() {
        assert_eq!(classify_hook("not json"), HookClass::Invalid);
        assert_eq!(classify_hook("{}"), HookClass::Invalid);
    }

    #[test]
    fn settings_json_embeds_the_url_and_bearer_token() {
        let json = settings_json("http://127.0.0.1:5000/hook/abc", "tok-123");
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
            "SubagentStop",
            "Notification",
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
