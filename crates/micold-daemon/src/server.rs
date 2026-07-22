//! Daemon startup + accept loop (contracts/protocol.md §2/§4, plan W2, task T020).
//!
//! Resolves the endpoint, runs the single-instance sequence (or defers to socket activation), and
//! serves each accepted connection through the shared [`DaemonCodec`]: read `Hello`, evaluate the
//! strict handshake, reply `Welcome` or `Refused`. Catalog ownership, attach routing and grid
//! streaming layer on top in T021–T022; this is the connection spine.

use std::io;

use futures_util::{SinkExt, StreamExt};
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::messages::{CatalogSnapshot, ClientMsg, DaemonMsg, DaemonSettings};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::endpoint;
use crate::singleton::{self, Acquisition};

/// A human-facing build string named in diagnostics and the handshake.
pub fn daemon_build() -> String {
    format!("micold-daemon {}", env!("CARGO_PKG_VERSION"))
}

/// The current service settings (placeholder until the Catalog owns them — T021).
fn default_settings() -> DaemonSettings {
    DaemonSettings {
        scrollback_lines: 10_000,
    }
}

/// Run the daemon: adopt a systemd socket if present, else acquire the endpoint, then accept.
pub async fn run() -> io::Result<()> {
    // systemd socket activation (Linux, opportunistic — MUST NOT be required; protocol.md §2).
    #[cfg(target_os = "linux")]
    if let Some(listener) = systemd_listener()? {
        eprintln!("micold-daemon: adopted systemd-activated socket");
        return serve_unix(listener).await;
    }

    let endpoint = endpoint::resolve()?;
    match singleton::acquire(&endpoint).await? {
        Acquisition::AlreadyRunning => {
            eprintln!(
                "micold-daemon: another daemon owns {} — exiting",
                endpoint.socket_path.display()
            );
            Ok(())
        }
        Acquisition::Bound(bound) => {
            eprintln!(
                "micold-daemon: listening on {}",
                bound.socket_path().display()
            );
            serve_interprocess(bound).await
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
async fn serve_interprocess(bound: singleton::BoundListener) -> io::Result<()> {
    use interprocess::local_socket::traits::tokio::Listener as _;
    loop {
        let conn = bound.listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = serve_connection(conn).await {
                eprintln!("micold-daemon: connection ended: {e}");
            }
        });
    }
}

/// Accept loop over a systemd-activated Unix listener.
#[cfg(target_os = "linux")]
async fn serve_unix(listener: tokio::net::UnixListener) -> io::Result<()> {
    loop {
        let (conn, _addr) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = serve_connection(conn).await {
                eprintln!("micold-daemon: connection ended: {e}");
            }
        });
    }
}

/// Serve one connection: handshake, then the (currently minimal) message loop.
///
/// Generic over the stream so the interprocess path, the systemd path, and tests share one
/// implementation.
pub async fn serve_connection<S>(stream: S) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut framed = Framed::new(stream, DaemonCodec::new());

    // First frame must be a Hello.
    let hello = match framed.next().await {
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
        None => return Ok(()), // client hung up before saying hello
    };

    let (client_version, client_hash, _client_build) = hello;
    match handshake::evaluate(client_version, client_hash, daemon_build()) {
        Ok(()) => {
            let welcome = DaemonMsg::Welcome {
                daemon_build: daemon_build(),
                catalog: CatalogSnapshot::default(),
                settings: default_settings(),
            };
            framed
                .send(Frame::Control(welcome))
                .await
                .map_err(io::Error::other)?;
        }
        Err(reason) => {
            framed
                .send(Frame::Control(DaemonMsg::Refused { reason }))
                .await
                .map_err(io::Error::other)?;
            return Ok(()); // handshake refused; close.
        }
    }

    // Minimal post-handshake loop: answer Ping, ignore the rest until attach/catalog land (T022).
    while let Some(frame) = framed.next().await {
        match frame.map_err(io::Error::other)? {
            Frame::Control(ClientMsg::Ping { nonce }) => {
                framed
                    .send(Frame::Control(DaemonMsg::Pong { nonce }))
                    .await
                    .map_err(io::Error::other)?;
            }
            Frame::Control(ClientMsg::Goodbye) => break,
            _ => {}
        }
    }
    Ok(())
}
