//! Client-side connect + handshake, with auto-spawn (protocol.md §2/§4, FR-003, T026/T026a).
//!
//! Lives in the core so the client and the daemon compile against **one** definition of where the
//! endpoint is and how the handshake goes — a client and daemon can never disagree about either.
//! (It is also what lets the headless suite test auto-spawn without pulling in iced.)
//!
//! The cold-start path (SC-003) is: try to connect → nothing listening → spawn a detached daemon →
//! poll the endpoint until it answers → `Hello` → `Welcome`. No install step, no supervisor.

use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use interprocess::local_socket::tokio::prelude::*;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::{GenericFilePath, Name, ToFsName};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio_util::codec::Framed;

use crate::endpoint::{DialAddress, Endpoint};
use crate::protocol::codec::{ClientCodec, Frame};
use crate::protocol::messages::{
    CatalogSnapshot, ClientInstance, ClientMsg, DaemonMsg, DaemonSettings, RefusalReason,
};
use crate::protocol::version::{BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};
use crate::sandbox::placement::Placement;

/// The byte stream underneath a daemon connection.
///
/// An enum rather than a generic parameter, because `DaemonConnection` is a concrete type the
/// client's subscription holds across awaits, and making it generic would push a type parameter
/// through every one of those call sites for no gain. Two variants is the whole space: there are
/// two transports, and the second exists because a bind-mounted Unix socket does not survive
/// Docker Desktop's file sharing (research R1).
#[derive(Debug)]
pub enum Transport {
    /// The Unix socket or Windows named pipe: the host-process placement, unchanged.
    Local(Stream),
    /// Loopback TCP: the sandbox placement.
    Loopback(tokio::net::TcpStream),
}

macro_rules! delegate {
    ($self:ident, $method:ident $(, $arg:expr)*) => {
        match $self.get_mut() {
            Transport::Local(s) => Pin::new(s).$method($($arg),*),
            Transport::Loopback(s) => Pin::new(s).$method($($arg),*),
        }
    };
}

impl AsyncRead for Transport {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        delegate!(self, poll_read, cx, buf)
    }
}

impl AsyncWrite for Transport {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        delegate!(self, poll_write, cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        delegate!(self, poll_flush, cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        delegate!(self, poll_shutdown, cx)
    }
}

/// A handshaked connection to the daemon.
pub type DaemonConnection = Framed<Transport, ClientCodec>;

/// What a client presents about itself beyond the three identity constants (feature 027).
///
/// Defaulted so the host-process path is unchanged: no token, and a fingerprint mismatch is not a
/// refusal. Both only become interesting for a sandbox, and the second only for a *locally built*
/// one — see `contracts/protocol-delta.md` for why that asymmetry is deliberate.
#[derive(Debug, Clone, Default)]
pub struct Credentials {
    /// The shared secret, when the daemon requires one.
    ///
    /// [`PresentedToken`] rather than `String` so this struct's derived `Debug` cannot carry it —
    /// the same reason the wire field is wrapped (T118, rule P-3).
    pub auth_token: Option<crate::protocol::messages::PresentedToken>,
    /// Whether a build-fingerprint mismatch refuses the connection.
    pub require_fingerprint_match: bool,
}

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
    /// Handshake accepted. The connection is boxed (clippy::large_enum_variant) — `DaemonSettings`
    /// gained a `String` field for the environment-include setting (FR-012b, BUG-003), which tipped
    /// this variant well past `Refused`'s size.
    Ready(Box<DaemonConnection>, Welcome),
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

/// Whether an error means "nothing is listening yet", which is a normal cold-start state rather
/// than a failure. Shared by both transports so they cannot disagree about it.
fn is_absent(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::NotFound
            | io::ErrorKind::ConnectionRefused
            | io::ErrorKind::AddrNotAvailable
            | io::ErrorKind::TimedOut
    )
}

/// Open a raw connection to the endpoint, or `None` if nothing is listening.
pub async fn dial(endpoint: &Endpoint) -> io::Result<Option<Transport>> {
    dial_address(&DialAddress::Local(endpoint.clone())).await
}

/// Open a raw connection to a dial address, or `None` if nothing is listening.
pub async fn dial_address(address: &DialAddress) -> io::Result<Option<Transport>> {
    match address {
        DialAddress::Local(endpoint) => {
            let name = fs_name(endpoint)?;
            match Stream::connect(name).await {
                Ok(stream) => Ok(Some(Transport::Local(stream))),
                Err(e) if is_absent(&e) => Ok(None),
                Err(e) => Err(e),
            }
        }
        DialAddress::Loopback { .. } => {
            let addr = address.socket_addr().expect("loopback has an address");
            match tokio::net::TcpStream::connect(addr).await {
                Ok(stream) => {
                    // Terminal traffic is small and latency-sensitive; Nagle would coalesce a
                    // keystroke with whatever came next and show up as input lag.
                    let _ = stream.set_nodelay(true);
                    Ok(Some(Transport::Loopback(stream)))
                }
                Err(e) if is_absent(&e) => Ok(None),
                Err(e) => Err(e),
            }
        }
    }
}

/// Perform the `Hello`/`Welcome` handshake over an open stream, presenting no credentials.
pub async fn handshake(stream: Transport, client_build: &str) -> io::Result<Connected> {
    handshake_with(stream, client_build, &Credentials::default()).await
}

/// Perform the handshake, presenting the token and fingerprint policy in `credentials`.
pub async fn handshake_with(
    stream: Transport,
    client_build: &str,
    credentials: &Credentials,
) -> io::Result<Connected> {
    let mut framed = Framed::new(stream, ClientCodec::new());
    framed
        .send(Frame::Control(ClientMsg::Hello {
            protocol_version: PROTOCOL_VERSION,
            schema_hash: SCHEMA_HASH,
            client_build: client_build.to_string(),
            // This process's own instance, not a parameter: an instance identifies the *process*,
            // so letting a caller pass one in would let two connections of one process disagree
            // about which window they are — the exact confusion BUG-022 is about.
            client_instance: ClientInstance::current(),
            client_package_version: PACKAGE_VERSION.to_string(),
            auth_token: credentials.auth_token.clone(),
            client_fingerprint: BUILD_FINGERPRINT.to_string(),
            require_fingerprint_match: credentials.require_fingerprint_match,
        }))
        .await
        .map_err(io::Error::other)?;

    match framed.next().await {
        Some(Ok(Frame::Control(DaemonMsg::Welcome {
            daemon_build,
            catalog,
            settings,
        }))) => Ok(Connected::Ready(
            Box::new(framed),
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
    connect_at(
        &DialAddress::Local(endpoint.clone()),
        client_build,
        &Credentials::default(),
    )
    .await
}

/// Connect to whatever is listening at `address`, presenting `credentials`. Does not start one.
pub async fn connect_at(
    address: &DialAddress,
    client_build: &str,
    credentials: &Credentials,
) -> io::Result<Option<Connected>> {
    match dial_address(address).await? {
        Some(stream) => handshake_with(stream, client_build, credentials)
            .await
            .map(Some),
        None => Ok(None),
    }
}

/// Connect for a resolved [`Placement`], starting the daemon if none is listening.
///
/// Replaces the old `connect_or_spawn`, whose name encoded the assumption this feature removes:
/// that starting a daemon means spawning a host process. Only the host placement is startable from
/// here — a sandbox has to be created, image-acquired and started, which is a multi-stage operation
/// with progress the user watches, so it belongs to the client's sandbox lifecycle rather than to a
/// one-shot connect.
///
/// **It never substitutes one placement for another** (rule P-2). A sandbox placement with nothing
/// listening returns [`io::ErrorKind::NotConnected`], not a host-process daemon. That is FR-035's
/// guarantee, and it is enforced here because here is where it would be easiest to break.
pub async fn connect_or_start(
    placement: &Placement,
    address: &DialAddress,
    client_build: &str,
    credentials: &Credentials,
    timeout: Duration,
) -> io::Result<Connected> {
    if let Some(connected) = connect_at(address, client_build, credentials).await? {
        return Ok(connected);
    }

    match placement {
        Placement::HostProcess => {
            let endpoint = match address {
                DialAddress::Local(e) => e.clone(),
                DialAddress::Loopback { .. } => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "the host placement is not reached over loopback TCP",
                    ))
                }
            };
            connect_or_spawn(&endpoint, client_build, timeout).await
        }
        Placement::LocalSandbox(_) => Err(io::Error::new(
            io::ErrorKind::NotConnected,
            format!(
                "no sandboxed daemon is listening on {} — the sandbox must be started first",
                address.describe()
            ),
        )),
        Placement::Remote(_) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "a remote daemon cannot be started from here",
        )),
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
