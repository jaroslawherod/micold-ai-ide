//! Daemon startup + accept loop + per-connection routing (protocol.md §2/§4, plan W2, T020/T022).
//!
//! Resolves the endpoint, runs the single-instance sequence (or adopts a systemd socket), then serves
//! each accepted connection through the shared [`DaemonState`]: strict handshake, then attach/detach,
//! viewed-session, keepalive, and settings routing, with catalog/settings changes pushed to every
//! connected client. Grid streaming and the mutating RPCs layer on in Phase 3 / T053.

use std::io;
use std::sync::Arc;

use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::messages::{ClientMsg, DaemonMsg};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::catalog::Catalog;
use crate::logging;
use crate::singleton::{self, Acquisition};
use crate::state::DaemonState;
use micold_core::endpoint;

/// A human-facing build string named in diagnostics and the handshake.
pub fn daemon_build() -> String {
    format!("micold-daemon {}", env!("CARGO_PKG_VERSION"))
}

/// Run the daemon: adopt a systemd socket if present, else acquire the endpoint, then accept.
pub async fn run() -> io::Result<()> {
    // Diagnostics first, so even a failed bind is recorded (FR-045).
    let logging = logging::init()?;
    tracing::info!(
        build = %daemon_build(),
        sink = ?logging.sink,
        log_path = ?logging.path,
        "micold-daemon starting"
    );

    let catalog = Catalog::load_default();
    // A recovered (corrupt) catalog is surfaced, not swallowed (data-model C4).
    tracing::info!(load_status = ?catalog.load_status(), "catalog adopted");
    let state = Arc::new(DaemonState::new(catalog));

    // systemd socket activation (Linux, opportunistic — MUST NOT be required; protocol.md §2).
    #[cfg(target_os = "linux")]
    if let Some(listener) = systemd_listener()? {
        tracing::info!("adopted systemd-activated socket");
        return serve_unix(state, listener).await;
    }

    let endpoint = endpoint::resolve()?;
    match singleton::acquire(&endpoint).await? {
        Acquisition::AlreadyRunning => {
            tracing::info!(
                endpoint = %endpoint.socket_path.display(),
                "another daemon already owns the endpoint; exiting"
            );
            Ok(())
        }
        Acquisition::Bound(bound) => {
            tracing::info!(endpoint = %bound.socket_path().display(), "listening");
            serve_interprocess(state, bound).await
        }
    }
}

/// Adopt an `LISTEN_FDS`-provided Unix socket, if this process is the intended recipient.
/// `set_nonblocking(true)` is mandatory — systemd does not guarantee it (protocol.md §2).
#[cfg(target_os = "linux")]
fn systemd_listener() -> io::Result<Option<tokio::net::UnixListener>> {
    let mut fds = listenfd::ListenFd::from_env();
    match fds.take_unix_listener(0) {
        Ok(Some(std_listener)) => {
            std_listener.set_nonblocking(true)?;
            Ok(Some(tokio::net::UnixListener::from_std(std_listener)?))
        }
        Ok(None) => Ok(None),
        Err(e) => Err(io::Error::other(e)),
    }
}

/// Accept loop over the single-instance interprocess listener.
async fn serve_interprocess(
    state: Arc<DaemonState>,
    bound: singleton::BoundListener,
) -> io::Result<()> {
    use interprocess::local_socket::traits::tokio::Listener as _;
    loop {
        let conn = bound.listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = serve_connection(state, conn).await {
                tracing::warn!(error = %e, "connection ended with an error");
            }
        });
    }
}

/// Accept loop over a systemd-activated Unix listener.
#[cfg(target_os = "linux")]
async fn serve_unix(state: Arc<DaemonState>, listener: tokio::net::UnixListener) -> io::Result<()> {
    loop {
        let (conn, _addr) = listener.accept().await?;
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = serve_connection(state, conn).await {
                tracing::warn!(error = %e, "connection ended with an error");
            }
        });
    }
}

/// Serve one connection: handshake, register, then route messages until the client hangs up.
///
/// Generic over the stream so the interprocess path, the systemd path, and tests share one
/// implementation.
pub async fn serve_connection<S>(state: Arc<DaemonState>, stream: S) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut framed = Framed::new(stream, DaemonCodec::new());

    // --- Handshake: the first frame must be a Hello, and it must match exactly. ---
    let (client_version, client_hash, client_build) = match framed.next().await {
        Some(Ok(Frame::Control(ClientMsg::Hello {
            protocol_version,
            schema_hash,
            client_build,
        }))) => (protocol_version, schema_hash, client_build),
        Some(Ok(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected Hello as the first frame",
            ))
        }
        Some(Err(e)) => return Err(io::Error::other(e)),
        None => return Ok(()), // hung up before saying hello
    };

    if let Err(reason) = handshake::evaluate(client_version, client_hash, daemon_build()) {
        // Identity + versions only — never any session content (FR-047).
        tracing::warn!(
            client_build = %client_build,
            client_version,
            daemon_version = micold_core::protocol::version::PROTOCOL_VERSION,
            "refusing client: contract mismatch"
        );
        framed
            .send(Frame::Control(DaemonMsg::Refused { reason }))
            .await
            .map_err(io::Error::other)?;
        return Ok(()); // refused; close without registering.
    }

    // --- Welcome (sent synchronously, so it is unambiguously the first frame the client sees). ---
    let (catalog, settings) = state.welcome_payload();
    framed
        .send(Frame::Control(DaemonMsg::Welcome {
            daemon_build: daemon_build(),
            catalog,
            settings,
        }))
        .await
        .map_err(io::Error::other)?;

    // --- Register, split, and run the reader/writer split. ---
    tracing::info!(client_build = %client_build, "client attached to daemon");
    let (id, mut rx) = state.register(client_build);
    let (mut sink, mut incoming) = framed.split();

    // Writer task: drain this client's push channel (broadcasts, attach replies, pongs) to the wire.
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Frame::Control(msg)).await.is_err() {
                break;
            }
        }
    });

    let result = route(&state, id, &mut incoming).await;

    // Cleanup: releasing the client drops its sender, which ends the writer task.
    tracing::info!(client = id, "client disconnected");
    state.deregister(id);
    let _ = writer.await;
    result
}

/// The per-connection message loop. Every push back to the client goes through the shared state's
/// per-client channel so other connections can reach this one too.
async fn route<St>(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    incoming: &mut St,
) -> io::Result<()>
where
    St: futures_util::Stream<
            Item = Result<Frame<ClientMsg>, micold_core::protocol::codec::CodecError>,
        > + Unpin,
{
    while let Some(frame) = incoming.next().await {
        let msg = match frame {
            Ok(Frame::Control(msg)) => msg,
            Ok(Frame::Grid(_)) => continue, // clients never send grid frames
            Err(e) => return Err(io::Error::other(e)),
        };

        match msg {
            ClientMsg::Ping { nonce } => state.send(id, DaemonMsg::Pong { nonce }),
            ClientMsg::Goodbye => break,
            ClientMsg::Attach { project, force } => {
                match state.attach(id, project.clone(), force) {
                    Ok(sessions) => {
                        tracing::info!(client = id, project = %project.display(), force, "project attached");
                        state.send(id, DaemonMsg::Attached { project, sessions })
                    }
                    Err(reason) => {
                        tracing::info!(client = id, project = %project.display(), "attach refused: project busy");
                        state.send(id, DaemonMsg::Refused { reason })
                    }
                }
            }
            ClientMsg::Detach { project } => state.detach(id, &project),
            ClientMsg::SetViewedSession { project, session } => {
                state.set_viewed(id, project, session)
            }
            ClientMsg::SettingsSet {
                req,
                scrollback_lines,
            } => {
                let result = match scrollback_lines {
                    Some(lines) => state.set_scrollback(lines),
                    None => Ok(()),
                };
                match result {
                    Ok(()) => state.send(
                        id,
                        DaemonMsg::OperationOk {
                            req,
                            result: micold_core::protocol::messages::OperationResult::Ack,
                        },
                    ),
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: micold_core::protocol::messages::ErrorKind::IoFailed,
                            message: "failed to persist settings".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            // Session commands, mutating RPCs and scrollback land in Phase 3 / T053.
            _ => {}
        }
    }
    Ok(())
}
