//! Feature 034, U147 (FR-001, FR-007, SC-008, the *Sandboxed placement* edge case; research R8):
//! the tool-server binding works for a session that runs in the sandbox container.
//!
//! R8 decided the sandbox needs no change: the service runs inside the container, so its loopback
//! listener is the container's and its data directory is the container's. The fast suites cannot
//! check that against a real runtime. This probe does, in one sandbox:
//!
//! 1. an AI-CLI session is started, and its binding file appears in the container's data
//!    directory;
//! 2. `whoami` is called from a shell session **inside** the container, with the URL and the
//!    credential that file holds, and names the bound session;
//! 3. the project path it returns exists inside the container;
//! 4. the runtime publishes only the control port, and the tool server does not answer on the
//!    host's loopback.
//!
//! The call in step 2 is made with `node`, which the image ships for the AI CLIs, because the
//! image has no `curl`. The script is in the project directory, which is mounted, and writes the
//! reply there, so the reply is read from a file and not off a wrapped terminal line.
//!
//! The AI CLI itself is not driven: it has no sign-in here. What is probed is what the service
//! hands it, used from where it runs.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_mcp -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

#[path = "support/mcp.rs"]
mod mcp_support;
mod sandbox_real_support;

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::SinkExt;
use micold_core::project::{Availability, Project};
use micold_core::protocol::auth::Token;
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::ClientMsg;
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::store::ProjectStore;
use micold_core::workspace::Workspace;
use serde_json::{json, Value};

use sandbox_real_support::{
    cli_out, credentials, input_serial, open_session, start_sandbox, wait_for_accept, SandboxSpec,
    Terminal,
};

const CONTAINER: &str = "micold-mcp-probe";
const NETWORK: &str = "micold-mcp-probe-net";
/// Not 7727: a developer's own sandbox must neither be disturbed nor accidentally probed.
const PORT: u16 = 17752;
/// The control port inside the container, the only one `argv::create` publishes.
const CONTROL_PORT: &str = "7727";
/// The service's data directory inside the container: the image sets `XDG_DATA_HOME=/var/lib`.
const CONTAINER_DATA_DIR: &str = "/var/lib/micold-ai-ide";

/// Pinned for the reason the other probes pin it.
const SESSION_SHELL: &str = "/bin/sh";

/// Calls `whoami` with the binding file named by its first argument and writes
/// `{"status", "body"}` or `{"error"}` to the file named by its second.
const PROBE_SCRIPT: &str = r#"
const fs = require('fs');
const [, , binding, out] = process.argv;
const entry = JSON.parse(fs.readFileSync(binding, 'utf8')).mcpServers.micold;
const request = { jsonrpc: '2.0', id: 1, method: 'tools/call', params: { name: 'whoami', arguments: {} } };
fetch(entry.url, {
  method: 'POST',
  headers: { ...entry.headers, 'Content-Type': 'application/json', Accept: 'application/json, text/event-stream' },
  body: JSON.stringify(request),
})
  .then(async (reply) => {
    fs.writeFileSync(out, JSON.stringify({ status: reply.status, body: await reply.text() }));
  })
  .catch((error) => {
    fs.writeFileSync(out, JSON.stringify({ error: String(error), cause: String(error.cause) }));
    process.exit(1);
  });
"#;

/// A shell to type into and a Claude Code session to bind.
fn seed(state: &Path, project: &Path) -> (SessionId, SessionId) {
    std::fs::create_dir_all(state).expect("state dir");
    let shell = SessionId::new();
    let agent = SessionId::new();
    let sessions = vec![
        Session::restored(
            shell,
            SessionLocation::Default,
            SessionLabel::Named("shell".into()),
            TerminalMode::Regular,
            AiCli::default(),
        ),
        Session::restored(
            agent,
            SessionLocation::Default,
            SessionLabel::Named("agent".into()),
            TerminalMode::AiCli,
            AiCli::ClaudeCode,
        ),
    ];
    let workspace = Workspace {
        projects: vec![Project::new(
            project.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project.to_path_buf()),
        sessions: BTreeMap::from([(project.to_path_buf(), sessions)]),
        worktree_names: BTreeMap::new(),
        ..Default::default()
    };
    micold_core::store::JsonFileStore::at(state.join("projects.json"))
        .save(&workspace)
        .expect("seed projects.json");
    (shell, agent)
}

/// Wait for `file` to appear, quoting the daemon's log if it does not.
async fn wait_for_file(file: &Path, log: &Path) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !file.exists() {
        assert!(
            Instant::now() < deadline,
            "no binding file at {} within 30s.\n--- daemon log ---\n{}",
            file.display(),
            std::fs::read_to_string(log).unwrap_or_else(|e| format!("<unreadable: {e}>"))
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

#[tokio::test]
async fn sandbox_real_mcp_binding_answers_whoami_from_inside_the_container() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let state = data.join("micold-ai-ide");
    let (shell, agent) = seed(&state, &project);

    let script = project.join("whoami-probe.cjs");
    let reply_file = project.join("whoami-reply.json");
    std::fs::write(&script, PROBE_SCRIPT).unwrap();

    let token = Token::generate();
    let token_path = state.join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let log = state.join("micold-daemon.log");

    // SAFETY: set before any spawn in this single-test binary.
    std::env::set_var("SHELL", SESSION_SHELL);

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        survive_logout: false,
        extra: &[],
    });
    let (mut conn, catalog) = wait_for_accept(PORT, &credentials(&token)).await;

    // 1. Starting the AI-CLI session writes its binding file under the container's data
    // directory. The state mount is that directory, so the file is waited for on the host side
    // and then read at its container path, from inside.
    conn.send(Frame::Control(ClientMsg::SessionStart { session: agent }))
        .await
        .expect("start the AI-CLI session");
    let binding_name = format!("{}.json", agent.0);
    let binding_on_host = state.join("mcp").join(&binding_name);
    wait_for_file(&binding_on_host, &log).await;
    let binding_in_container = format!("{CONTAINER_DATA_DIR}/mcp/{binding_name}");

    let serial = input_serial(&catalog, shell);
    let screen = open_session(&mut conn, &project, shell, &log).await;
    let mut term = Terminal::new(&mut conn, shell, screen, CONTAINER, &log, serial);

    // 2. `whoami`, called from inside the container with what the binding file holds.
    let ran = term
        .run(&format!(
            "node '{}' '{binding_in_container}' '{}'; echo rc=$?",
            script.display(),
            reply_file.display()
        ))
        .await;
    let reply: Value = serde_json::from_str(
        &std::fs::read_to_string(&reply_file)
            .unwrap_or_else(|e| panic!("the probe wrote no reply ({e}); it printed:\n{ran}")),
    )
    .expect("the probe's reply is JSON");
    assert_eq!(
        reply["status"],
        json!(200),
        "the tool server did not answer from inside the container: {reply}\n{ran}"
    );
    let response: Value =
        serde_json::from_str(reply["body"].as_str().unwrap()).expect("a JSON-RPC response");
    let result = &response["result"];
    assert_eq!(result["isError"], json!(false), "whoami failed: {response}");
    assert_eq!(
        result["structuredContent"],
        json!({
            "session": agent.0.to_string(),
            "project": {"name": "project", "path": project.display().to_string()},
            "worktree": "default",
            "ai_cli": "claude_code",
        })
    );

    // 3. The path it returned is a directory the caller can see.
    let returned = result["structuredContent"]["project"]["path"]
        .as_str()
        .unwrap();
    let seen = term
        .run(&format!("test -d '{returned}' && echo present"))
        .await;
    assert!(
        seen.contains("present"),
        "whoami returned {returned}, which is not a directory inside the container:\n{seen}"
    );

    // 4. Only the control port is published, and the tool server's port is not reachable on the
    // host's loopback.
    let published = cli_out(&["port", CONTAINER]);
    for line in published.lines().filter(|l| !l.trim().is_empty()) {
        assert!(
            line.starts_with(&format!("{CONTROL_PORT}/")),
            "the sandbox publishes more than the control port: {published}"
        );
    }

    let binding: Value =
        serde_json::from_str(&std::fs::read_to_string(&binding_on_host).unwrap()).unwrap();
    let entry = &binding["mcpServers"]["micold"];
    let url = entry["url"].as_str().expect("the binding names a URL");
    let port: u16 = url
        .strip_prefix("http://127.0.0.1:")
        .and_then(|rest| rest.strip_suffix("/mcp"))
        .and_then(|port| port.parse().ok())
        .unwrap_or_else(|| panic!("the binding's URL is not the container's loopback: {url}"));
    let bearer = entry["headers"]["Authorization"]
        .as_str()
        .and_then(|value| value.strip_prefix("Bearer "))
        .expect("the binding carries a bearer credential");
    let on_host = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    // Refused is the expected answer. Another host process may hold the same ephemeral port, so a
    // connection is not a failure by itself: an answer to this credential is.
    if tokio::net::TcpStream::connect(on_host).await.is_ok() {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": "whoami", "arguments": {}}
        });
        let (status, body) =
            mcp_support::post_mcp(on_host, Some(bearer), &request.to_string()).await;
        assert!(
            status != 200 || !body.contains(&agent.0.to_string()),
            "the sandbox's tool server answered on the host's loopback at {on_host}: {body}"
        );
    }
}
