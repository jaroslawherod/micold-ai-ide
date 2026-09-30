//! Shared support for the tool-server tests (feature 034): a `DaemonState` over a real git
//! repository with named worktrees and sessions, a bound tool server, and a raw-TCP `POST /mcp`.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::FakeProjectStore;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::mcp::server::{self, ToolServer};
use micold_daemon::state::DaemonState;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

/// Run `git -C repo args…`, asserting success.
pub fn git(repo: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A git repository with one empty commit.
pub fn init_repo(dir: &Path) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "t@t.test"]);
    git(dir, &["config", "user.name", "T"]);
    git(dir, &["commit", "-q", "--allow-empty", "-m", "root"]);
}

/// Add a worktree `.claude/worktrees/<name>` on a new branch `<name>`.
pub fn add_worktree(repo: &Path, name: &str) -> PathBuf {
    let path = repo.join(".claude/worktrees").join(name);
    git(
        repo,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            name,
            path.to_str().unwrap(),
        ],
    );
    path
}

/// A session id that is easy to read in a failure message.
pub fn sid(n: u128) -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(n))
}

/// An idle session record.
pub fn session(id: SessionId, worktree: Option<&str>, mode: TerminalMode, cli: AiCli) -> Session {
    let location = match worktree {
        Some(dir) => SessionLocation::Worktree(dir.to_string()),
        None => SessionLocation::Default,
    };
    Session::restored(id, location, SessionLabel::Pending, mode, cli)
}

/// A daemon state holding `projects`, each with its sessions, over an in-memory project store and
/// a settings file in `store_dir`.
pub fn state_over(
    projects: Vec<(PathBuf, bool, Vec<Session>)>,
    store_dir: &Path,
) -> Arc<DaemonState> {
    let mut sessions = BTreeMap::new();
    let mut list = Vec::new();
    for (path, is_git, project_sessions) in projects {
        list.push(Project::new(path.clone(), is_git, Availability::Available));
        sessions.insert(path, project_sessions);
    }
    let workspace = Workspace {
        active: list.first().map(|p| p.path.clone()),
        projects: list,
        sessions,
        ..Default::default()
    };
    let catalog = Catalog::load(
        Box::new(FakeProjectStore::loaded(workspace)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    );
    Arc::new(DaemonState::new(catalog))
}

/// Bind a tool server writing its binding files under `binding_dir`, record it on `state`, and
/// serve it; returns its address.
pub async fn serve_tool_server(state: &Arc<DaemonState>, binding_dir: PathBuf) -> SocketAddr {
    let (tool_server, listener) = ToolServer::bind(binding_dir).await.expect("bind");
    let addr = tool_server.addr();
    state.set_tool_server(tool_server);
    tokio::spawn(server::serve(listener, Arc::clone(state)));
    addr
}

/// The credential the tool server holds for `session` (issuing one if needed).
pub fn credential(state: &DaemonState, session: SessionId) -> String {
    state
        .tool_server()
        .expect("tool server bound")
        .credential_for(session)
}

/// Send raw request bytes and read the whole response: its status and body.
pub async fn raw_request(addr: SocketAddr, request: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    stream.write_all(request).await.expect("write");
    let mut response = Vec::new();
    let _ = stream.read_to_end(&mut response).await;
    let text = String::from_utf8_lossy(&response).into_owned();
    let status = text
        .split(' ')
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let body = text
        .split_once("\r\n\r\n")
        .map(|(_, b)| b.to_string())
        .unwrap_or_default();
    (status, body)
}

/// `POST /mcp` with `json` and, if given, `Authorization: Bearer <bearer>`.
pub async fn post_mcp(addr: SocketAddr, bearer: Option<&str>, json: &str) -> (u16, String) {
    let auth = bearer
        .map(|b| format!("Authorization: Bearer {b}\r\n"))
        .unwrap_or_default();
    let request = format!(
        "POST /mcp HTTP/1.1\r\nHost: {addr}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{auth}Content-Length: {}\r\n\r\n{json}",
        json.len()
    );
    raw_request(addr, request.as_bytes()).await
}

/// Call `tool` with `arguments` as `bearer`, returning the `CallToolResult`.
pub async fn call_tool(addr: SocketAddr, bearer: &str, tool: &str, arguments: Value) -> Value {
    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {"name": tool, "arguments": arguments}
    });
    let (status, body) = post_mcp(addr, Some(bearer), &request.to_string()).await;
    assert_eq!(status, 200, "tools/call {tool}: {body}");
    let response: Value = serde_json::from_str(&body).expect("JSON response");
    response["result"].clone()
}

/// Call a tool that must succeed, returning its structured output.
pub async fn call_ok(addr: SocketAddr, bearer: &str, tool: &str, arguments: Value) -> Value {
    let result = call_tool(addr, bearer, tool, arguments).await;
    assert_eq!(result["isError"], json!(false), "{tool} failed: {result}");
    result["structuredContent"].clone()
}

/// Call a tool that must fail, returning its `{category, message}`.
pub async fn call_err(addr: SocketAddr, bearer: &str, tool: &str, arguments: Value) -> Value {
    let result = call_tool(addr, bearer, tool, arguments).await;
    assert_eq!(result["isError"], json!(true), "{tool} succeeded: {result}");
    result["structuredContent"]["error"].clone()
}
