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

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

use crate::activity::{ActivityEvent, HookKind};
use crate::state::DaemonState;
use micold_core::session::SessionId;

/// Maximum bytes read for the request head (request line + headers). A hook request's head is a few
/// hundred bytes; this is generous but bounds a hostile/looping sender.
const MAX_HEAD: usize = 8 * 1024;

/// Maximum request-body size accepted (contracts/hooks.md §5). Hook payloads are small JSON objects;
/// anything larger is rejected with `413` rather than buffered.
const MAX_BODY: usize = 64 * 1024;

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

/// A parsed HTTP request head (request line + the headers we care about).
#[derive(Debug, PartialEq, Eq)]
struct Head {
    method: String,
    path: String,
    content_length: usize,
    bearer: Option<String>,
}

/// The classification of a hook payload's `hook_event_name`.
#[derive(Debug, PartialEq, Eq)]
enum HookClass {
    /// A recognised turn-affecting hook — apply it to the FSM.
    Event(HookKind),
    /// A well-formed hook that carries no FSM transition (e.g. `SessionStart`) — accept, do nothing.
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
    let mut buf = Vec::with_capacity(1024);
    let head_end = loop {
        if let Some(pos) = find_head_end(&buf) {
            break pos;
        }
        if buf.len() >= MAX_HEAD {
            return respond(&mut stream, 431, "Request Header Fields Too Large").await;
        }
        let mut chunk = [0u8; 1024];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            // Connection closed before a complete head — nothing to answer.
            return Ok(());
        }
        buf.extend_from_slice(&chunk[..n]);
    };

    let Some(head) = parse_head(&buf[..head_end]) else {
        return respond(&mut stream, 400, "Bad Request").await;
    };

    // 2. Only POST /hook/<uuid> is a route; anything else is a flat 404 (no capability surface).
    if head.method != "POST" {
        return respond(&mut stream, 405, "Method Not Allowed").await;
    }
    let Some(session_uuid) = session_id_from_path(&head.path) else {
        return respond(&mut stream, 404, "Not Found").await;
    };

    if head.content_length > MAX_BODY {
        return respond(&mut stream, 413, "Payload Too Large").await;
    }

    // 3. Authenticate BEFORE reading the body. A missing/mismatched token is a bare 403 that never
    //    discloses whether the session exists (contract §3).
    let authorized = {
        let tokens = tokens.lock().expect("hook tokens lock poisoned");
        matches!((tokens.get(&session_uuid), &head.bearer), (Some(expected), Some(got)) if expected == got)
    };
    if !authorized {
        return respond(&mut stream, 403, "Forbidden").await;
    }

    // 4. Read the (bounded) body. The head read may have already pulled some of it in.
    let mut body = buf[head_end..].to_vec();
    while body.len() < head.content_length {
        let mut chunk = [0u8; 1024];
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > MAX_BODY {
            return respond(&mut stream, 413, "Payload Too Large").await;
        }
    }
    body.truncate(head.content_length);

    // 5. Map the hook to an activity transition. The body is NEVER logged (contract §6).
    let body_str = std::str::from_utf8(&body).unwrap_or("");
    match classify_hook(body_str) {
        HookClass::Event(kind) => {
            let session = SessionId::from_uuid(session_uuid);
            if state.note_activity(session, ActivityEvent::Hook(kind)) {
                state.broadcast_catalog();
            }
            respond(&mut stream, 200, "OK").await
        }
        HookClass::Ignored => respond(&mut stream, 200, "OK").await,
        HookClass::Invalid => respond(&mut stream, 400, "Bad Request").await,
    }
}

/// Write a minimal HTTP/1.1 response with an empty body and close-after semantics.
async fn respond(stream: &mut TcpStream, status: u16, reason: &str) -> io::Result<()> {
    let head =
        format!("HTTP/1.1 {status} {reason}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    stream.write_all(head.as_bytes()).await?;
    stream.flush().await
}

/// The index just past the `\r\n\r\n` that ends the request head, if present.
fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|p| p + 4)
}

/// Parse the request line + the `Content-Length` and `Authorization: Bearer` headers from a request
/// head. Header names are matched case-insensitively (HTTP allows any casing). Returns `None` if the
/// request line is malformed.
fn parse_head(head: &[u8]) -> Option<Head> {
    let text = std::str::from_utf8(head).ok()?;
    let mut lines = text.split("\r\n");
    let request_line = lines.next()?;
    let mut parts = request_line.split(' ');
    let method = parts.next()?.to_string();
    let path = parts.next()?.to_string();
    // A third field (HTTP version) must exist for a well-formed request line.
    parts.next()?;
    if method.is_empty() || path.is_empty() {
        return None;
    }

    let mut content_length = 0usize;
    let mut bearer = None;
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim();
        match name.as_str() {
            "content-length" => content_length = value.parse().unwrap_or(0),
            "authorization" => {
                bearer = value
                    .strip_prefix("Bearer ")
                    .or_else(|| value.strip_prefix("bearer "))
                    .map(|t| t.trim().to_string());
            }
            _ => {}
        }
    }
    Some(Head {
        method,
        path,
        content_length,
        bearer,
    })
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
        // SessionStart is well-formed but carries no FSM transition (the session starts Unknown).
        "SessionStart" => HookClass::Ignored,
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
    fn parses_a_well_formed_hook_request_head() {
        let head = b"POST /hook/abc HTTP/1.1\r\nHost: x\r\nContent-Length: 42\r\nAuthorization: Bearer secret-token\r\n\r\n";
        let end = find_head_end(head).unwrap();
        let parsed = parse_head(&head[..end]).unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.path, "/hook/abc");
        assert_eq!(parsed.content_length, 42);
        assert_eq!(parsed.bearer.as_deref(), Some("secret-token"));
    }

    #[test]
    fn header_names_are_case_insensitive() {
        let head =
            b"POST /hook/x HTTP/1.1\r\ncontent-length: 7\r\nAUTHORIZATION: bearer tok\r\n\r\n";
        let parsed = parse_head(head).unwrap();
        assert_eq!(parsed.content_length, 7);
        assert_eq!(parsed.bearer.as_deref(), Some("tok"));
    }

    #[test]
    fn a_malformed_request_line_is_rejected() {
        assert!(parse_head(b"GARBAGE\r\n\r\n").is_none());
        assert!(parse_head(b"POST\r\n\r\n").is_none());
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
        // SessionStart and unknown-but-valid hooks are accepted without a transition.
        assert_eq!(ev("SessionStart"), HookClass::Ignored);
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

    #[test]
    fn find_head_end_needs_the_blank_line() {
        assert_eq!(find_head_end(b"POST / HTTP/1.1\r\n"), None);
        assert_eq!(find_head_end(b"POST / HTTP/1.1\r\n\r\n"), Some(19));
    }
}
