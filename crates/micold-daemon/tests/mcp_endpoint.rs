//! Feature 034 (contracts/binding.md §1–§3; U17–U29): the tool server's loopback endpoint.
//!
//! Drives the real listener over TCP with raw HTTP: bounds, routes, the bearer check whose refusal
//! is identical for every cause (SC-006), per-session credentials, and their revocation when a
//! session leaves the catalog or the service restarts (FR-006).

#[path = "support/mcp.rs"]
mod mcp_support;

use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use micold_core::session::{AiCli, TerminalMode};
use serde_json::{json, Value};
use mcp_support::*;
use tracing_subscriber::fmt::MakeWriter;

const INITIALIZED: &str = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
const PING: &str = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;

#[derive(Clone)]
struct BufWriter(Arc<Mutex<Vec<u8>>>);

impl io::Write for BufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for BufWriter {
    type Writer = BufWriter;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

/// Everything this test binary logs, at every level, from the first call on.
fn log() -> Arc<Mutex<Vec<u8>>> {
    static LOG: OnceLock<Arc<Mutex<Vec<u8>>>> = OnceLock::new();
    LOG.get_or_init(|| {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let subscriber = tracing_subscriber::fmt()
            .with_writer(BufWriter(buf.clone()))
            .with_ansi(false)
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("one global subscriber");
        buf
    })
    .clone()
}

/// A served tool server over a project root with sessions S1 (root) and S2 (worktree `a`).
struct Served {
    state: Arc<micold_daemon::state::DaemonState>,
    addr: std::net::SocketAddr,
    project: tempfile::TempDir,
    store: tempfile::TempDir,
}

async fn served() -> Served {
    log();
    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    init_repo(project.path());
    let state = state_over(
        vec![(
            project.path().to_path_buf(),
            true,
            vec![
                session(sid(1), None, TerminalMode::AiCli, AiCli::ClaudeCode),
                session(sid(2), Some("a"), TerminalMode::AiCli, AiCli::Copilot),
            ],
        )],
        store.path(),
    );
    let addr = serve_tool_server(&state, store.path().join("mcp")).await;
    Served {
        state,
        addr,
        project,
        store,
    }
}

fn binding_file(store: &Path, id: micold_core::session::SessionId) -> std::path::PathBuf {
    store.join("mcp").join(format!("{}.json", id.0))
}

#[tokio::test]
async fn the_listener_is_loopback_with_an_ephemeral_port() {
    let s = served().await;
    assert_eq!(s.addr.ip(), std::net::Ipv4Addr::LOCALHOST);
    assert_ne!(s.addr.port(), 0);
    let url = s.state.tool_server().unwrap().url();
    assert_eq!(url, format!("http://127.0.0.1:{}/mcp", s.addr.port()));
}

#[tokio::test]
async fn a_request_without_authorization_is_refused_with_an_empty_401() {
    let s = served().await;
    let (status, body) = post_mcp(s.addr, None, PING).await;
    assert_eq!(status, 401);
    assert_eq!(body, "");
}

#[tokio::test]
async fn a_malformed_authorization_is_refused_identically() {
    let s = served().await;
    let (_, reference) = post_mcp(s.addr, None, PING).await;
    let request = format!(
        "POST /mcp HTTP/1.1\r\nAuthorization: Basic dXNlcjpwYXNz\r\nContent-Length: {}\r\n\r\n{PING}",
        PING.len()
    );
    let full_reference = raw_request(
        s.addr,
        format!("POST /mcp HTTP/1.1\r\nContent-Length: {}\r\n\r\n{PING}", PING.len()).as_bytes(),
    )
    .await;
    let malformed = raw_request(s.addr, request.as_bytes()).await;
    assert_eq!(malformed, full_reference);
    assert_eq!(malformed, (401, reference));
}

#[tokio::test]
async fn an_unknown_bearer_is_refused_identically() {
    let s = served().await;
    let reference = post_mcp(s.addr, None, PING).await;
    let unknown = post_mcp(s.addr, Some("0000000000000000deadbeef00000000"), PING).await;
    assert_eq!(unknown, reference);
    assert_eq!(unknown.0, 401);
}

#[tokio::test]
async fn get_and_delete_on_mcp_are_method_not_allowed() {
    let s = served().await;
    for method in ["GET", "DELETE"] {
        let (status, _) = raw_request(
            s.addr,
            format!("{method} /mcp HTTP/1.1\r\nHost: x\r\n\r\n").as_bytes(),
        )
        .await;
        assert_eq!(status, 405, "{method} /mcp");
    }
}

#[tokio::test]
async fn any_other_path_is_not_found_with_an_empty_body() {
    let s = served().await;
    let cred = credential(&s.state, sid(1));
    for path in ["/", "/mcp/x", "/hook/1", "/sse"] {
        let request = format!(
            "POST {path} HTTP/1.1\r\nAuthorization: Bearer {cred}\r\nContent-Length: {}\r\n\r\n{PING}",
            PING.len()
        );
        let (status, body) = raw_request(s.addr, request.as_bytes()).await;
        assert_eq!((status, body.as_str()), (404, ""), "{path}");
    }
}

#[tokio::test]
async fn a_head_over_8_kib_is_refused_with_431() {
    let s = served().await;
    let padding = "x".repeat(9 * 1024);
    let request = format!("POST /mcp HTTP/1.1\r\nX-Padding: {padding}\r\n\r\n");
    let (status, _) = raw_request(s.addr, request.as_bytes()).await;
    assert_eq!(status, 431);
}

#[tokio::test]
async fn a_body_over_1_mib_is_refused_with_413() {
    let s = served().await;
    let cred = credential(&s.state, sid(1));
    let body = "x".repeat(1024 * 1024 + 1);
    let request = format!(
        "POST /mcp HTTP/1.1\r\nAuthorization: Bearer {cred}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw_request(s.addr, request.as_bytes()).await;
    assert_eq!(status, 413);

    // Exactly 1 MiB is within the bound: it is read, and answered as the malformed JSON it is.
    let body = "x".repeat(1024 * 1024);
    let request = format!(
        "POST /mcp HTTP/1.1\r\nAuthorization: Bearer {cred}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let (status, _) = raw_request(s.addr, request.as_bytes()).await;
    assert_ne!(status, 413);
}

#[tokio::test]
async fn the_initialized_notification_is_accepted_with_an_empty_202() {
    let s = served().await;
    let cred = credential(&s.state, sid(1));
    let (status, body) = post_mcp(s.addr, Some(&cred), INITIALIZED).await;
    assert_eq!((status, body.as_str()), (202, ""));
}

#[tokio::test]
async fn an_authorized_request_is_answered_with_json() {
    let s = served().await;
    let cred = credential(&s.state, sid(1));
    let (status, body) = post_mcp(s.addr, Some(&cred), PING).await;
    assert_eq!(status, 200);
    let response: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(response, json!({"jsonrpc":"2.0","id":1,"result":{}}));
}

#[tokio::test]
async fn each_session_gets_its_own_credential_and_is_named_by_it() {
    let s = served().await;
    let one = credential(&s.state, sid(1));
    let two = credential(&s.state, sid(2));
    assert_ne!(one, two);
    assert_eq!(one, credential(&s.state, sid(1)), "stable while the session exists");
    let who_one = call_ok(s.addr, &one, "whoami", json!({})).await;
    let who_two = call_ok(s.addr, &two, "whoami", json!({})).await;
    assert_eq!(who_one["session"], sid(1).0.to_string());
    assert_eq!(who_two["session"], sid(2).0.to_string());
}

#[tokio::test]
async fn a_deleted_sessions_credential_and_binding_file_are_gone() {
    let s = served().await;
    let cred = credential(&s.state, sid(1));
    let server = s.state.tool_server().unwrap();
    let file = server.write_binding(sid(1), b"{}").unwrap();
    assert_eq!(file, binding_file(s.store.path(), sid(1)));
    assert_eq!(post_mcp(s.addr, Some(&cred), PING).await.0, 200);

    s.state.delete_session(sid(1)).unwrap();

    assert_eq!(post_mcp(s.addr, Some(&cred), PING).await.0, 401);
    assert!(!file.exists(), "the binding file goes with the credential");
}

#[tokio::test]
async fn a_worktree_deletes_sessions_lose_their_credentials() {
    let s = served().await;
    let cred = credential(&s.state, sid(2));
    let file = s
        .state
        .tool_server()
        .unwrap()
        .write_binding(sid(2), b"{}")
        .unwrap();
    assert_eq!(post_mcp(s.addr, Some(&cred), PING).await.0, 200);

    s.state
        .archive_and_remove_worktree_sessions(s.project.path(), "a")
        .unwrap();

    assert_eq!(post_mcp(s.addr, Some(&cred), PING).await.0, 401);
    assert!(!file.exists());
    let other = credential(&s.state, sid(1));
    assert_eq!(
        post_mcp(s.addr, Some(&other), PING).await.0,
        200,
        "a session outside the worktree keeps its credential"
    );
}

#[tokio::test]
async fn a_restarted_service_accepts_no_earlier_credential() {
    let before = served().await;
    let old = credential(&before.state, sid(1));
    let after = served().await;
    // The same session exists in the fresh service; it has not been issued anything yet.
    assert_eq!(post_mcp(after.addr, Some(&old), PING).await.0, 401);
    let fresh = credential(&after.state, sid(1));
    assert_ne!(fresh, old);
}

#[tokio::test]
async fn neither_the_body_nor_the_credential_is_logged() {
    let s = served().await;
    let cred = credential(&s.state, sid(1));
    let marker = "body-marker-7f3e9a";
    let request = json!({"jsonrpc":"2.0","id":marker,"method":"tools/call",
        "params":{"name":"whoami","arguments":{}}});
    post_mcp(s.addr, Some(&cred), &request.to_string()).await;
    post_mcp(s.addr, Some(&cred), &format!("{{not json {marker}")).await;
    post_mcp(s.addr, Some(&format!("{cred}x")), PING).await;
    let logged = String::from_utf8_lossy(&log().lock().unwrap()).into_owned();
    assert!(!logged.contains(&cred), "a credential reached the log");
    assert!(!logged.contains(marker), "a request body reached the log");
}
