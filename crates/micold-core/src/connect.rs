//! Client-side connect + handshake, with auto-spawn (protocol.md §2/§4, FR-003, T026/T026a).
//!
//! Lives in the core so the client and the daemon compile against **one** definition of where the
//! endpoint is and how the handshake goes — a client and daemon can never disagree about either.
//! (It is also what lets the headless suite test auto-spawn without pulling in iced.)
//!
//! The cold-start path (SC-003) is: try to connect → nothing listening → spawn a detached daemon →
//! poll the endpoint until it answers → `Hello` → `Welcome`. No install step, no supervisor.

use std::io;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{GenericFilePath, Name, ToFsName};
use tokio_util::codec::Framed;

use crate::endpoint::Endpoint;
use crate::protocol::codec::{ClientCodec, Frame};
use crate::protocol::messages::{
    CatalogSnapshot, ClientMsg, DaemonMsg, DaemonSettings, RefusalReason,
};
use crate::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};

/// A handshaked connection to the daemon.
pub type DaemonConnection = Framed<Stream, ClientCodec>;

/// What the daemon said when we introduced ourselves.
pub struct Welcome {
    /// The daemon's build string (named in diagnostics).
    pub daemon_build: String,
    /// The catalog as of the handshake.
    pub catalog: CatalogSnapshot,
    /// The service-owned settings.
    pub settings: DaemonSettings,
}

/// The outcome of a connect attempt.
pub enum Connected {
    /// Handshake accepted.
    Ready(DaemonConnection, Welcome),
    /// Handshake refused — typically a version/schema mismatch needing the restart action (FR-022).
    Refused(RefusalReason),
}

/// How long to wait for a freshly-spawned daemon to start accepting.
pub const SPAWN_TIMEOUT: Duration = Duration::from_secs(10);

fn fs_name(endpoint: &Endpoint) -> io::Result<Name<'_>> {
    endpoint
        .socket_path
        .as_os_str()
        .to_fs_name::<GenericFilePath>()
}

/// Open a raw connection to the endpoint, or `None` if nothing is listening.
pub async fn dial(endpoint: &Endpoint) -> io::Result<Option<Stream>> {
    let name = fs_name(endpoint)?;
    match Stream::connect(name).await {
        Ok(stream) => Ok(Some(stream)),
        // "Nothing there" is a normal, expected state on a cold start — not an error.
        Err(e)
            if matches!(
                e.kind(),
                io::ErrorKind::NotFound
                    | io::ErrorKind::ConnectionRefused
                    | io::ErrorKind::AddrNotAvailable
            ) =>
        {
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

/// Perform the `Hello`/`Welcome` handshake over an open stream.
pub async fn handshake(stream: Stream, client_build: &str) -> io::Result<Connected> {
    let mut framed = Framed::new(stream, ClientCodec::new());
    framed
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: client_build.to_string(),
            client_package_version: PACKAGE_VERSION.to_string(),
        }))
        .await
        .map_err(io::Error::other)?;

    match framed.next().await {
        Some(Ok(Frame::Control(DaemonMsg::Welcome {
            daemon_build,
            catalog,
            settings,
        }))) => Ok(Connected::Ready(
            framed,
            Welcome {
                daemon_build,
                catalog,
                settings,
            },
        )),
        Some(Ok(Frame::Control(DaemonMsg::Refused { reason }))) => Ok(Connected::Refused(reason)),
        Some(Ok(other)) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected Welcome or Refused, got {other:?}"),
        )),
        Some(Err(e)) => Err(io::Error::other(e)),
        None => Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "daemon closed the connection during the handshake",
        )),
    }
}

/// Connect to a daemon if one is listening, else `None`. Does not spawn.
pub async fn connect(endpoint: &Endpoint, client_build: &str) -> io::Result<Option<Connected>> {
    match dial(endpoint).await? {
        Some(stream) => handshake(stream, client_build).await.map(Some),
        None => Ok(None),
    }
}

/// Connect, spawning a detached daemon first if none is listening, then polling until it accepts
/// (FR-003; closes the SC-003 cold-start path).
pub async fn connect_or_spawn(
    endpoint: &Endpoint,
    client_build: &str,
    timeout: Duration,
) -> io::Result<Connected> {
    if let Some(connected) = connect(endpoint, client_build).await? {
        return Ok(connected);
    }

    let pid = crate::spawn::spawn_detached_daemon()?;
    let _ = pid; // the daemon is intentionally not ours to wait on

    // The daemon is ready when it *accepts*, not when exec returns — poll until it answers.
    let deadline = std::time::Instant::now() + timeout;
    let mut backoff = Duration::from_millis(10);
    loop {
        if let Some(connected) = connect(endpoint, client_build).await? {
            return Ok(connected);
        }
        if std::time::Instant::now() >= deadline {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "spawned micold-daemon did not start accepting on {} within {:?}",
                    endpoint.socket_path.display(),
                    timeout
                ),
            ));
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(Duration::from_millis(250));
    }
}
