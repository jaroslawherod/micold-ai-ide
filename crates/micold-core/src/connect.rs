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
#[cfg(not(windows))]
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

#[cfg(not(windows))]
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

/// Whether an error means the daemon went away *during* the handshake.
///
/// This is the other half of [`is_absent`], and it exists because of one race the daemon cannot
/// close from its side (contract §4.14/§4.15, research R5). A connect succeeds the moment the
/// kernel puts it on the listener's backlog — the daemon need never accept it — so a client that
/// dials at the instant an idle daemon stops gets an open stream and then a reset, an abort or a
/// clean EOF, depending on how far the handshake got. There is no ordering the daemon can adopt
/// that removes that instant; the remedy is here, where the client can simply look again.
///
/// From the caller's point of view a daemon that vanishes mid-handshake and a daemon that was
/// never there are the same thing, and are treated the same: nobody is listening, start one.
fn vanished_mid_handshake(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::BrokenPipe
            | io::ErrorKind::UnexpectedEof
    )
}

/// Refuses a pipe whose server process does not run as `own_sid` (U71, security review D1).
///
/// `Ok(true)` is this user's server; `Ok(false)` is a server that went away before it could be
/// checked, which the caller reads as nobody listening.
#[cfg(windows)]
fn refuse_foreign_server(stream: &Stream, own_sid: &str) -> io::Result<bool> {
    let pid = stream.peer_creds()?.pid();
    refuse_foreign_pid(pid, own_sid)
}

/// [`refuse_foreign_server`] for the server process `pid`, once the pipe has named it.
#[cfg(windows)]
fn refuse_foreign_pid(pid: Option<u32>, own_sid: &str) -> io::Result<bool> {
    let refuse = |server: &str| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "the daemon pipe is served as {server}, not as this user {own_sid}; refusing it"
            ),
        )
    };
    let pid = pid.ok_or_else(|| refuse("an unknown process"))?;
    match crate::endpoint::process_user_sid(pid) {
        Ok(server) if server == own_sid => Ok(true),
        Ok(server) => Err(refuse(&format!("{server} (pid {pid})"))),
        // A process this user may not even query is not this user's.
        Err(e) if e.kind() == io::ErrorKind::PermissionDenied => Err(refuse(&format!(
            "an account this user cannot query (pid {pid})"
        ))),
        Err(e) => Err(e),
    }
}

/// Connects to the endpoint's pipe at `SecurityIdentification` (U72, security review D2).
///
/// `interprocess` opens the pipe with no quality of service, which lets its server impersonate the
/// client fully. Whoever serves the pipe needs to learn who connected, and nothing more. Otherwise
/// this is `interprocess`'s own connect: a busy pipe is waited for, unbounded.
#[cfg(windows)]
async fn connect_pipe(endpoint: &Endpoint) -> io::Result<Stream> {
    use interprocess::os::windows::named_pipe::local_socket::tokio::Stream as PipeStream;
    use std::os::windows::ffi::OsStrExt;
    use std::os::windows::io::{FromRawHandle, OwnedHandle};
    use windows_sys::Win32::Foundation::{
        ERROR_PIPE_BUSY, GENERIC_READ, GENERIC_WRITE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
    };

    let path: Vec<u16> = endpoint
        .socket_path
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    loop {
        // SAFETY: `path` is NUL-terminated and outlives the call; every other argument is a plain
        // flag or null, as `CreateFileW` allows.
        let handle = unsafe {
            CreateFileW(
                path.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                std::ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            // SAFETY: a valid handle `CreateFileW` just returned, owned by nothing else.
            let handle = unsafe { OwnedHandle::from_raw_handle(handle) };
            let pipe = PipeStream::try_from(handle)?;
            return Ok(pipe.into());
        }
        let error = io::Error::last_os_error();
        if error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32) {
            return Err(error);
        }
        wait_for_pipe(path.clone()).await?;
    }
}

/// Waits for an instance of the NUL-terminated pipe `path` to become free, unbounded.
///
/// RED: reads the wait's error back on the awaiting thread.
#[cfg(windows)]
async fn wait_for_pipe(path: Vec<u16>) -> io::Result<()> {
    use windows_sys::Win32::System::Pipes::{WaitNamedPipeW, NMPWAIT_WAIT_FOREVER};

    // SAFETY: `path` is NUL-terminated and owned by the closure for the whole call.
    let waited = tokio::task::spawn_blocking(move || unsafe {
        WaitNamedPipeW(path.as_ptr(), NMPWAIT_WAIT_FOREVER)
    })
    .await
    .map_err(io::Error::other)?;
    if waited == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Open a raw connection to the endpoint, or `None` if nothing is listening.
pub async fn dial(endpoint: &Endpoint) -> io::Result<Option<Transport>> {
    dial_address(&DialAddress::Local(endpoint.clone())).await
}

/// Open a raw connection to a dial address, or `None` if nothing is listening.
pub async fn dial_address(address: &DialAddress) -> io::Result<Option<Transport>> {
    match address {
        DialAddress::Local(endpoint) => {
            #[cfg(windows)]
            let connected = connect_pipe(endpoint).await;
            #[cfg(not(windows))]
            let connected = Stream::connect(fs_name(endpoint)?).await;
            match connected {
                Ok(stream) => {
                    // Anyone may create a pipe by this name first; only this user's daemon is ours
                    // to talk to (FR-021).
                    #[cfg(windows)]
                    if !refuse_foreign_server(&stream, &crate::endpoint::user_sid()?)? {
                        return Ok(None);
                    }
                    Ok(Some(Transport::Local(stream)))
                }
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
        .map_err(io::Error::from)?;

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
        // `io::Error::from`, not `io::Error::other`: a reset arriving here is how a departing
        // daemon most often ends a handshake, and [`vanished_mid_handshake`] can only recognise it
        // if the kind survives the trip through the codec's error type.
        Some(Err(e)) => Err(io::Error::from(e)),
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
        Some(stream) => match handshake_with(stream, client_build, credentials).await {
            Ok(connected) => Ok(Some(connected)),
            // Not a failure to report: see [`vanished_mid_handshake`]. Reported as absence so
            // every caller absorbs it the way it already absorbs a cold endpoint.
            Err(e) if vanished_mid_handshake(&e) => Ok(None),
            Err(e) => Err(e),
        },
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

/// How long a spawned daemon is given to take the endpoint before another is spawned.
///
/// One spawn is not always enough, and the reason is the idle stop (US3): a daemon spawned while
/// the previous one is still unwinding finds the endpoint owned, says so, and exits — correctly,
/// since two daemons on one endpoint is the failure the singleton exists to prevent. By the time
/// the old one has released it there is nobody left to answer, and waiting longer does not change
/// that. So the poll spawns again, rather than waiting out the timeout for a daemon that already
/// gave up. Long enough that a healthy cold start is never spawned over.
const RESPAWN_AFTER: Duration = Duration::from_secs(1);

/// Connect, spawning a detached daemon if none is listening, then polling until it accepts
/// (FR-003; closes the SC-003 cold-start path).
///
/// Also closes the connect-as-the-window-expires race (FR-016, contract §4.15): between
/// [`vanished_mid_handshake`] reading a departing daemon as absence and [`RESPAWN_AFTER`] spawning
/// over one that lost the endpoint race, a connect issued at the moment an idle daemon stops ends
/// attached to a fresh one rather than reaching the user as an error.
pub async fn connect_or_spawn(
    endpoint: &Endpoint,
    client_build: &str,
    timeout: Duration,
) -> io::Result<Connected> {
    // The daemon is ready when it *accepts*, not when exec returns — poll until it answers.
    let deadline = std::time::Instant::now() + timeout;
    let mut backoff = Duration::from_millis(10);
    let mut spawned_at: Option<std::time::Instant> = None;
    loop {
        if let Some(connected) = connect(endpoint, client_build).await? {
            return Ok(connected);
        }
        if spawned_at.is_none_or(|at| at.elapsed() >= RESPAWN_AFTER) {
            let pid = crate::spawn::spawn_detached_daemon()?;
            let _ = pid; // the daemon is intentionally not ours to wait on
            spawned_at = Some(std::time::Instant::now());
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::codec::CodecError;
    #[cfg(windows)]
    use interprocess::local_socket::{GenericFilePath, ToFsName};

    /// FR-016, §4.14: a reset that arrives through the codec is still a reset.
    ///
    /// The race this guards is not reproducible on demand — it needs a connect to land inside the
    /// instant an idle daemon is unwinding, which `tests/idle_race.rs` reaches roughly one run in
    /// five — so the classification it depends on is pinned here, where it is deterministic.
    ///
    /// The failure this prevents is invisible at the call site: `io::Error::other` compiles, reads
    /// fine, and turns "the daemon went away, look again" into "show the user an error", because
    /// the only thing that distinguishes the two is a kind that conversion throws away.
    #[test]
    fn a_reset_arriving_through_the_codec_is_still_read_as_the_daemon_going_away() {
        for kind in [
            io::ErrorKind::ConnectionReset,
            io::ErrorKind::ConnectionAborted,
            io::ErrorKind::BrokenPipe,
            io::ErrorKind::UnexpectedEof,
        ] {
            let wrapped: io::Error =
                CodecError::Io(io::Error::new(kind, "the other end went")).into();
            assert_eq!(wrapped.kind(), kind, "the kind must survive the codec");
            assert!(
                vanished_mid_handshake(&wrapped),
                "a {kind:?} reaching the client through the codec must read as absence, not as a \
                 failure to put in front of the user"
            );
        }
    }

    /// The other direction, so the conversion above cannot be "call everything a disconnect".
    ///
    /// A frame the daemon should never have sent is this layer's own failure and has no kind to
    /// preserve; reading it as absence would make the client spawn a second daemon over a healthy
    /// one every time the two disagreed about the protocol.
    #[test]
    fn a_protocol_failure_is_not_mistaken_for_a_departing_daemon() {
        let wrapped: io::Error =
            CodecError::ControlNotJson(crate::protocol::envelope::Encoding::Postcard).into();
        assert_eq!(wrapped.kind(), io::ErrorKind::Other);
        assert!(!vanished_mid_handshake(&wrapped));
    }

    /// Review A F1 on #332: `GetLastError` is per thread. The wait runs on a blocking-pool thread,
    /// so its failure has to be read there. Read back on the awaiting thread it is whatever that
    /// thread last left, which in `connect_pipe` is the `ERROR_PIPE_BUSY` that started the wait:
    /// a daemon that exits during the wait then reads as busy, not absent, and nobody respawns it.
    #[cfg(windows)]
    #[tokio::test]
    async fn a_pipe_that_vanishes_during_the_wait_reads_as_absent_not_busy() {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Foundation::{SetLastError, ERROR_PIPE_BUSY};

        let path: Vec<u16> = std::ffi::OsStr::new(&format!(
            r"\\.\pipe\Micold.Test.Connect.Gone.{}",
            std::process::id()
        ))
        .encode_wide()
        .chain(Some(0))
        .collect();
        // What `connect_pipe`'s `CreateFileW` leaves on this thread before it waits.
        // SAFETY: sets this thread's last-error value only.
        unsafe { SetLastError(ERROR_PIPE_BUSY) };

        let waited = wait_for_pipe(path)
            .await
            .expect_err("no pipe by that name exists to wait for");
        assert!(
            is_absent(&waited),
            "waiting on a pipe that is not there must read as absence, got {waited:?}"
        );
    }

    /// U71 (security review D1): the pipe name is predictable, so another account can create it
    /// first. A client that checks nothing hands that account its session traffic. No second
    /// account exists on CI, so the server here runs as this user and the client is told to expect
    /// SYSTEM instead; the same client expecting this user still connects.
    #[cfg(windows)]
    #[tokio::test]
    async fn a_pipe_served_as_another_account_is_refused_naming_both_sids() {
        use interprocess::local_socket::ListenerOptions;

        const SYSTEM: &str = "S-1-5-18";
        let path = format!(
            r"\\.\pipe\Micold.Test.Connect.Foreign.{}",
            std::process::id()
        );
        let name = || path.as_str().to_fs_name::<GenericFilePath>().unwrap();
        let _listener = ListenerOptions::new().name(name()).create_tokio().unwrap();
        let stream = Stream::connect(name()).await.unwrap();
        let own = crate::endpoint::user_sid().unwrap();

        let refused = refuse_foreign_server(&stream, SYSTEM)
            .expect_err("a pipe served as this user was accepted as SYSTEM's");
        assert_eq!(refused.kind(), io::ErrorKind::PermissionDenied);
        let message = refused.to_string();
        assert!(
            message.contains(SYSTEM) && message.contains(&own),
            "the refusal must name the expected SID {SYSTEM} and the server's {own}, got: {message}"
        );
        assert!(
            refuse_foreign_server(&stream, &own).expect("this user's own server must be accepted"),
            "this user's own, live server must read as ours"
        );
    }

    /// Review A F2 on #332: a daemon that exits between the pipe opening and the account check
    /// (idle stop, "Restart service") has no process left to query. That is nobody listening, as
    /// it is at every other point of a connect, so the client spawns a daemon instead of failing.
    #[cfg(windows)]
    #[test]
    fn a_server_that_exited_before_its_account_was_checked_reads_as_gone() {
        let mut child = std::process::Command::new("cmd.exe")
            .args(["/c", "exit"])
            .spawn()
            .unwrap();
        let pid = child.id();
        child.wait().unwrap();
        // The process object lives while any handle to it does; only then is the pid really gone.
        drop(child);
        let own = crate::endpoint::user_sid().unwrap();

        let checked = refuse_foreign_pid(Some(pid), &own);
        assert!(
            matches!(checked, Ok(false)),
            "a server process that no longer exists must read as gone, got {checked:?}"
        );
    }

    /// U72, security review D2: whoever serves the pipe learns who connected, and nothing more. A
    /// pipe opened without a quality of service lets its server act as the client
    /// (`SecurityImpersonation`), so a pipe squatter could reach whatever this user can.
    #[cfg(windows)]
    #[tokio::test]
    async fn the_daemon_pipe_lets_its_server_identify_the_client_but_not_impersonate_it() {
        use interprocess::local_socket::traits::tokio::Listener as _;
        use interprocess::local_socket::ListenerOptions;
        use std::os::windows::io::{AsHandle, AsRawHandle};
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use windows_sys::Win32::Security::{
            GetTokenInformation, RevertToSelf, SecurityIdentification, TokenImpersonationLevel,
            SECURITY_IMPERSONATION_LEVEL, TOKEN_QUERY,
        };
        use windows_sys::Win32::System::Pipes::ImpersonateNamedPipeClient;
        use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

        let path = format!(r"\\.\pipe\Micold.Test.Connect.Sqos.{}", std::process::id());
        let name = path.as_str().to_fs_name::<GenericFilePath>().unwrap();
        let listener = ListenerOptions::new().name(name).create_tokio().unwrap();
        let endpoint = Endpoint {
            socket_path: path.clone().into(),
            lock_path: Default::default(),
        };
        let (served, dialed) = tokio::join!(listener.accept(), dial(&endpoint));
        let mut served = served.unwrap();
        let mut client = dialed.unwrap().expect("the test pipe is listening");
        // A server may impersonate only once it has read from the client.
        client.write_all(b"?").await.unwrap();
        served.read_exact(&mut [0u8]).await.unwrap();
        let Stream::NamedPipe(pipe) = &served;
        let server_handle = pipe.as_handle().as_raw_handle();

        // SAFETY: the handle is the accepted pipe, alive for this whole block. The thread token is
        // read into a correctly sized local and closed before `RevertToSelf` ends the impersonation.
        let level = unsafe {
            assert_ne!(
                ImpersonateNamedPipeClient(server_handle),
                0,
                "ImpersonateNamedPipeClient: {}",
                io::Error::last_os_error()
            );
            let mut token = std::ptr::null_mut();
            let opened = OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token);
            let opened_error = io::Error::last_os_error();
            let mut level: SECURITY_IMPERSONATION_LEVEL = -1;
            let mut written = 0u32;
            let read = opened != 0
                && GetTokenInformation(
                    token,
                    TokenImpersonationLevel,
                    (&mut level as *mut SECURITY_IMPERSONATION_LEVEL).cast(),
                    std::mem::size_of::<SECURITY_IMPERSONATION_LEVEL>() as u32,
                    &mut written,
                ) != 0;
            if opened != 0 {
                windows_sys::Win32::Foundation::CloseHandle(token);
            }
            RevertToSelf();
            assert!(opened != 0, "OpenThreadToken: {opened_error}");
            assert!(read, "GetTokenInformation(TokenImpersonationLevel) failed");
            level
        };
        assert_eq!(
            level, SecurityIdentification,
            "the client must open the pipe at SecurityIdentification (1); 2 is SecurityImpersonation"
        );
    }
}
