//! The tool server's loopback listener (feature 034, contracts/binding.md §1–§3).
//!
//! A stateless MCP Streamable HTTP endpoint: every request is one `POST /mcp` carrying one JSON-RPC
//! message and a bearer credential, answered with one JSON response and closed. There is no session
//! header, no server-sent stream and no `GET`, so there is nothing to resume and nothing to leak
//! between requests (research R2).
//!
//! Order of checks per connection, each before the next is attempted: a bounded head (431), the
//! route (404), the method (405), the credential (401, identical for every cause), a bounded body
//! (413), then the JSON-RPC message. Neither the body nor the credential is ever logged (SC-006).

use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use micold_core::mcp::errors::{tool_failure, tool_success};
use micold_core::mcp::jsonrpc::{self, Route};
use micold_core::session::SessionId;
use tokio::net::{TcpListener, TcpStream};

use super::credentials::Credentials;
use crate::http::{self, respond, respond_json, Body, HeadRead};
use crate::state::DaemonState;

/// The largest request body the endpoint reads (contracts/binding.md §3).
pub const MAX_BODY: usize = 1024 * 1024;

/// The one route.
const PATH: &str = "/mcp";

/// The bound tool server: its address, its credentials, and where it writes binding files.
pub struct ToolServer {
    addr: SocketAddr,
    credentials: Arc<Credentials>,
    binding_dir: PathBuf,
}

impl ToolServer {
    /// Bind an ephemeral loopback port. Binding files go under `binding_dir`, created owner-only on
    /// first write.
    pub async fn bind(binding_dir: PathBuf) -> io::Result<(Self, TcpListener)> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await?;
        let addr = listener.local_addr()?;
        let server = Self {
            addr,
            credentials: Arc::new(Credentials::default()),
            binding_dir,
        };
        Ok((server, listener))
    }

    /// The bound address.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// The URL a bound CLI is pointed at.
    pub fn url(&self) -> String {
        format!("http://{}{PATH}", self.addr)
    }

    /// `session`'s credential, issued on first use.
    pub fn credential_for(&self, session: SessionId) -> String {
        self.credentials.credential_for(session)
    }

    /// The session a presented credential names, if it is live.
    pub fn session_for(&self, credential: &str) -> Option<SessionId> {
        self.credentials.session_for(credential)
    }

    /// Where `session`'s binding file lives.
    pub fn binding_file(&self, session: SessionId) -> PathBuf {
        self.binding_dir.join(binding_file_name(session))
    }

    /// Write `session`'s binding file owner-only, replacing any earlier one; returns its path.
    pub fn write_binding(&self, session: SessionId, contents: &[u8]) -> io::Result<PathBuf> {
        crate::platform::write_owner_only(
            &self.binding_dir,
            &binding_file_name(session),
            contents,
        )
    }

    /// Withdraw `session`'s credential and delete its binding file. Blocking file I/O: call it
    /// outside the state lock.
    pub fn revoke(&self, session: SessionId) {
        self.credentials.revoke(session);
        remove_if_present(&self.binding_file(session));
    }
}

/// The binding-file directory under the service's data directory (`<data_dir>/mcp`), falling back
/// to the system temporary directory when no data directory resolves — the rule the hook settings
/// directory follows.
pub fn default_binding_dir() -> PathBuf {
    directories::ProjectDirs::from("", "", "micold-ai-ide")
        .map(|dirs| dirs.data_dir().join("mcp"))
        .unwrap_or_else(|| std::env::temp_dir().join("micold-daemon-mcp"))
}

fn binding_file_name(session: SessionId) -> String {
    format!("{}.json", session.0)
}

fn remove_if_present(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        if e.kind() != io::ErrorKind::NotFound {
            tracing::warn!(file = %path.display(), error = %e, "could not delete a binding file");
        }
    }
}

/// The accept loop, for the daemon's lifetime. An accept error is transient and skipped.
pub async fn serve(listener: TcpListener, state: Arc<DaemonState>) {
    loop {
        let stream = match listener.accept().await {
            Ok((stream, _peer)) => stream,
            Err(_) => continue,
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            // A handler error is a peer that went away mid-request; it never propagates.
            let _ = handle_connection(stream, state).await;
        });
    }
}

/// Handle one connection: exactly one response, then close.
async fn handle_connection(mut stream: TcpStream, state: Arc<DaemonState>) -> io::Result<()> {
    let (head, rest) = match http::read_head(&mut stream).await? {
        HeadRead::Complete { head, rest } => (head, rest),
        HeadRead::TooLarge => {
            return respond(&mut stream, 431, "Request Header Fields Too Large").await
        }
        HeadRead::Malformed => return respond(&mut stream, 400, "Bad Request").await,
        HeadRead::Closed => return Ok(()),
    };

    let path = head.path.split('?').next().unwrap_or(&head.path);
    if path != PATH {
        return respond(&mut stream, 404, "Not Found").await;
    }
    if head.method != "POST" {
        return respond(&mut stream, 405, "Method Not Allowed").await;
    }

    // Missing, malformed and unknown credentials are refused identically (SC-006).
    let caller = head
        .bearer
        .as_deref()
        .and_then(|bearer| state.tool_server()?.session_for(bearer));
    let Some(caller) = caller else {
        return respond(&mut stream, 401, "Unauthorized").await;
    };

    let body = match http::read_body(&mut stream, rest, head.content_length, MAX_BODY).await? {
        Body::Complete(body) => body,
        Body::TooLarge => return respond(&mut stream, 413, "Payload Too Large").await,
    };

    let message = match jsonrpc::parse(&body) {
        Ok(message) => message,
        Err(response) => return reply(&mut stream, 400, "Bad Request", &response).await,
    };
    match jsonrpc::route(message, env!("CARGO_PKG_VERSION")) {
        Route::Reply(response) => reply(&mut stream, 200, "OK", &response).await,
        Route::Accepted => respond(&mut stream, 202, "Accepted").await,
        Route::CallTool {
            id,
            name,
            arguments,
        } => {
            let outcome = tokio::task::spawn_blocking(move || {
                super::tools::call(&state, caller, &name, &arguments)
            })
            .await
            .unwrap_or_else(|_| {
                Err(micold_core::mcp::errors::OpError::service_error(
                    "the tool call failed unexpectedly",
                ))
            });
            let result = match outcome {
                Ok(output) => tool_success(&output),
                Err(error) => tool_failure(&error),
            };
            reply(&mut stream, 200, "OK", &jsonrpc::result(id, result)).await
        }
    }
}

async fn reply(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    response: &serde_json::Value,
) -> io::Result<()> {
    let body = serde_json::to_vec(response).expect("JSON always serialises");
    respond_json(stream, status, reason, &body).await
}
