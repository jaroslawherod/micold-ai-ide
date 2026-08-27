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
use micold_core::git::{Git, GitCli};
use micold_core::naming::DerivedNames;
use micold_core::project::validate_rename;
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::messages::{
    ClientIdentity, ClientMsg, DaemonMsg, ErrorKind, LogSink, OperationResult, SessionProcess,
};
use micold_core::terminal::LaunchMode;
use micold_core::worktree::{
    branch_candidates, create_worktree, explain_directory_taken, parse_worktrees, preflight,
    remove_worktree, remove_worktree_dir, CreateError, CreateProgressEvent, Leftover,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::catalog::Catalog;
use crate::hooks;
use crate::logging;
use crate::progress::ProgressThrottle;
use crate::singleton::{self, Acquisition};
use crate::state::DaemonState;
use micold_core::endpoint;

/// A human-facing build string named in diagnostics and the handshake.
pub fn daemon_build() -> String {
    format!("micold-daemon {}", env!("CARGO_PKG_VERSION"))
}

/// The environment variable naming the address a containerised daemon listens on (feature 027).
///
/// Set by the image, not by the client: the *container* is what knows it is a container. A daemon
/// started on the host never sees it and takes the socket path it always did.
pub const LISTEN_ADDR_ENV: &str = "MICOLD_LISTEN_ADDR";

/// The address to listen on, when this daemon is containerised.
fn tcp_listen_addr() -> Option<String> {
    std::env::var(LISTEN_ADDR_ENV)
        .ok()
        .filter(|v| !v.is_empty())
}

/// Serve over loopback TCP (feature 027).
///
/// Binds `0.0.0.0` **inside the container**, which sounds alarming and is not: the container's
/// network is a user-defined bridge, and the only way in from outside is the port the runtime
/// publishes to `127.0.0.1` on the host. Binding the container's loopback instead would make the
/// published port unreachable, because the runtime forwards to the container's bridge address.
///
/// What actually guards this listener is the shared secret — see `protocol::auth`. That is not a
/// second line of defence; on this transport it is the only one.
async fn serve_tcp(state: Arc<DaemonState>, addr: &str) -> io::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await.inspect_err(|e| {
        tracing::error!(addr = %addr, error = %e, "failed to bind the sandbox listener");
    })?;
    tracing::info!(addr = %addr, "listening (sandboxed)");

    loop {
        let (conn, _peer) = listener.accept().await?;
        // Terminal traffic is small and latency-sensitive; Nagle would coalesce a keystroke with
        // whatever came next and show up to the user as input lag.
        let _ = conn.set_nodelay(true);
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            if let Err(e) = serve_connection(state, conn).await {
                tracing::warn!(error = %e, "connection ended with an error");
            }
        });
    }
}

/// Adopt the authentication token, if this daemon was started with one (feature 027, research R1).
///
/// The path comes from the environment because the container is what supplies it: the runtime
/// bind-mounts the host's `0600` token file at `MICOLD_TOKEN_PATH`, and the image sets that
/// variable. A daemon started without it is the host-process placement, which authenticates by the
/// `0700` directory guarding its socket and needs no token.
///
/// A token path that is set but unreadable is **fatal**. Falling back to accepting everyone would
/// turn a misconfigured mount into an open port, silently, inside the feature whose purpose is
/// containment.
fn adopt_auth_token(state: &DaemonState) -> io::Result<()> {
    let Some(path) = std::env::var_os(micold_core::protocol::auth::TOKEN_PATH_ENV) else {
        return Ok(());
    };
    let path = std::path::PathBuf::from(path);
    state.set_auth_token(&path).map_err(|e| {
        io::Error::new(
            e.kind(),
            format!(
                "{} names {} but it could not be read: {e}",
                micold_core::protocol::auth::TOKEN_PATH_ENV,
                path.display()
            ),
        )
    })?;
    tracing::info!("handshake authentication is enabled");
    Ok(())
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
    // Hand the diagnostics handle to the shared state so the `LogLocation`/`RecentErrors`/
    // `SetLogLevel` RPCs can serve it (FR-043–046).
    state.set_diagnostics(logging);

    // Feature 027: a sandboxed daemon requires the token its runtime mounted. Fatal if named and
    // unreadable — see `adopt_auth_token`.
    adopt_auth_token(&state)?;

    // FR-006a/b: sessions that were running when the service last stopped come back as
    // `InterruptedResumable` — never auto-relaunched, resumable by one explicit user action. This is
    // the ONLY lifecycle daemon startup may produce (data-model L4). Blocking (stats the provider
    // store), so it runs off the async runtime; it completes before the accept loop starts.
    {
        let startup = Arc::clone(&state);
        let marked =
            tokio::task::spawn_blocking(move || startup.present_interrupted_resumable_at_startup())
                .await
                .unwrap_or(0);
        if marked > 0 {
            tracing::info!(
                count = marked,
                "presented interrupted-resumable sessions after restart"
            );
        }
    }

    // Restart supervision runs on its own timer, independent of any client connection: a session
    // that crashes with no window open is restarted anyway (US4, FR-005).
    spawn_supervisor(Arc::clone(&state));

    // The loopback activity-hook receiver (US2, T045/T046): bind an ephemeral 127.0.0.1 port and
    // record it on the shared state so AI-CLI spawns point `claude`'s lifecycle hooks at it. A bind
    // failure is non-fatal — activity degrades to `Unknown` (H1), never to a wrong signal.
    match hooks::HookReceiver::bind(hooks::default_settings_dir()).await {
        Ok((receiver, listener)) => {
            let tokens = receiver.tokens();
            state.set_hooks(receiver);
            tokio::spawn(hooks::serve(listener, tokens, Arc::clone(&state)));
            tracing::info!("activity-hook receiver listening on loopback");
        }
        Err(e) => {
            tracing::warn!(error = %e, "could not bind the activity-hook receiver; activity will be Unknown");
        }
    }

    // Feature 027: inside a container there is no socket to bind and no host to share one with —
    // the client reaches us over loopback TCP, published from the container (research R1). Checked
    // before socket activation and before the endpoint, because in this placement neither exists:
    // `endpoint::resolve()` would try to create a directory under a home the container has no
    // business having, and fail with a permission error that says nothing about the real cause.
    if let Some(addr) = tcp_listen_addr() {
        return serve_tcp(state, &addr).await;
    }

    // systemd socket activation (Linux, opportunistic — MUST NOT be required; protocol.md §2).
    #[cfg(target_os = "linux")]
    if let Some(listener) = systemd_listener()? {
        tracing::info!("adopted systemd-activated socket");
        return serve_unix(state, listener).await;
    }

    let endpoint = endpoint::resolve().inspect_err(|e| {
        tracing::error!(error = %e, "could not resolve the endpoint to bind");
    })?;
    let acquisition = singleton::acquire(&endpoint).await.inspect_err(|e| {
        // Endpoint bind failure, logged with its reason before it propagates (FR-045).
        tracing::error!(endpoint = %endpoint.socket_path.display(), error = %e, "failed to bind the endpoint");
    })?;
    match acquisition {
        Acquisition::AlreadyRunning => {
            tracing::info!(
                endpoint = %endpoint.socket_path.display(),
                "another daemon already owns the endpoint; exiting"
            );
            Ok(())
        }
        Acquisition::Bound(bound) => {
            tracing::info!(endpoint = %bound.socket_path().display(), "listening");
            // Record our pid in the lock file so a version-mismatched client can stop us for its
            // "restart service" action (FR-022): a mismatched client can't handshake, so a control
            // message can't reach us — a recorded pid is the version-agnostic stop handle. Writing
            // through a separate fd does not disturb the daemon's held `flock` (advisory, per-OFD).
            if let Err(e) = std::fs::write(&endpoint.lock_path, std::process::id().to_string()) {
                tracing::warn!(error = %e, "could not record daemon pid in the lock file");
            }
            serve_interprocess(state, bound).await
        }
    }
}

/// How often the restart supervisor polls live sessions for exits. Fast enough that a crash-restart
/// feels immediate, cheap enough to be negligible at idle with a handful of sessions (US4).
const SUPERVISION_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

/// Minimum gap between two live-output lines forwarded for the *same* create stage (BUG-009, T123).
/// A submodule fetch emits thousands of lines; the user needs to see it moving, not to read them.
/// Fast enough to read as motion, slow enough that the wire cost is nil.
const PROGRESS_DETAIL_MIN_GAP: std::time::Duration = std::time::Duration::from_millis(400);

/// Spawn the restart-supervision loop (US4, FR-005). Ticks on [`SUPERVISION_INTERVAL`], drives the
/// crash-loop policy for any session whose child exited, and broadcasts `CatalogChanged` when a
/// lifecycle moved. The supervision itself is blocking (PTY spawn / process teardown), so it runs on
/// a blocking thread, never on the async runtime (module invariant).
fn spawn_supervisor(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SUPERVISION_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let worker = Arc::clone(&state);
            let changed = tokio::task::spawn_blocking(move || worker.supervise_exited_sessions())
                .await
                .unwrap_or_default();
            // Drain out-of-band terminal signals (title + spinner-derived activity, US2 T046/T047)
            // on the same cadence. It is lock-only (no blocking I/O), so it runs on the async task.
            let signals_changed = state.drain_signals();
            if !changed.is_empty() || signals_changed {
                state.broadcast_catalog();
            }
        }
    });
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
    let intro = match framed.next().await {
        Some(Ok(Frame::Control(ClientMsg::Hello {
            protocol_version,
            schema_hash,
            client_build,
            client_instance,
            client_package_version,
            auth_token,
            client_fingerprint,
            require_fingerprint_match,
        }))) => handshake::Introduction {
            protocol_version,
            schema_hash,
            package_version: client_package_version,
            build: client_build,
            instance: client_instance,
            auth_token,
            fingerprint: client_fingerprint,
            require_fingerprint_match,
        },
        Some(Ok(_)) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "expected Hello as the first frame",
            ))
        }
        Some(Err(e)) => return Err(io::Error::other(e)),
        None => return Ok(()), // hung up before saying hello
    };
    let client_build = intro.build.clone();
    let client_identity = ClientIdentity::new(intro.build.clone(), intro.instance.clone());
    let client_version = intro.protocol_version;
    let client_package_version = intro.package_version.clone();

    if let Err(reason) = handshake::evaluate_introduction(&intro, &state.expectation()) {
        // Identity + versions only — never any session content (FR-047).
        tracing::warn!(
            client_build = %client_build,
            client_version,
            client_package_version = %client_package_version,
            daemon_version = micold_core::protocol::version::PROTOCOL_VERSION,
            daemon_package_version = micold_core::protocol::version::PACKAGE_VERSION,
            "refusing client: contract or build mismatch"
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
    // `client_window` is what makes two windows of one build distinguishable in the log — and the
    // reason it is here rather than only on the wire: the reconnect that BUG-022 is about is
    // invisible in a log that records only the build, because both connections print the same one.
    tracing::info!(
        client_build = %client_build,
        client_window = client_identity.instance.pid,
        "client attached to daemon"
    );
    let (id, mut rx) = state.register(client_identity);
    let (mut sink, mut incoming) = framed.split();

    // Writer task: drain this client's push channel to the wire. The channel already carries fully
    // formed frames — control messages (broadcasts, attach replies, pongs) and pushed grid frames —
    // in one ordered stream, so a grid delta never races ahead of the control it followed.
    // A failed push is the earliest proof the peer is gone, so it is also where the client's
    // attachments are released (FR-025a, BUG-009 T121). The ordinary release is `deregister` when
    // `route` exits below; this one does not wait for whatever `route` is doing on this client's
    // behalf to finish, which is what let a departed client keep refusing its own reconnect.
    let writer_state = Arc::clone(&state);
    let writer = tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if sink.send(frame).await.is_err() {
                writer_state.release_attachments(id);
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

/// An event from a connection's own spawned work back to its message loop.
///
/// The loop owns state no other task may touch — the view stream — so work that now finishes
/// elsewhere reports back rather than reaching in (BUG-009, T125).
enum Internal {
    /// A session start this connection asked for has concluded (successfully or not).
    SessionStarted(micold_core::session::SessionId),
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
    // The grid stream for the session this client is currently viewing (FR-016). At most one runs
    // at a time; changing the viewed session aborts the old stream and starts the new one, and the
    // loop exit below stops it. The task pushes `Frame::Grid` into this client's ordered channel.
    let mut view_stream: Option<tokio::task::JoinHandle<()>> = None;
    // Which session this client asked to view, whether or not it is live yet (BUG-009, T125).
    // `view_stream` alone cannot answer that: a client may ask to view a session whose start is
    // still running, and the stream can only be built once the session exists.
    let mut viewing: Option<micold_core::session::SessionId> = None;
    // Events from this connection's own spawned work back to its loop. The loop owns `view_stream`
    // and is the only thing that may touch it, so work that finishes elsewhere — a session start,
    // now that starts no longer block the loop — reports back here rather than reaching in.
    let (internal_tx, mut internal_rx) = tokio::sync::mpsc::unbounded_channel::<Internal>();

    loop {
        let msg = tokio::select! {
            // Both arms are cancel-safe: `Framed`'s decoder keeps its read buffer across polls, and
            // an unbounded receiver drops nothing on a cancelled `recv`.
            frame = incoming.next() => match frame {
                Some(Ok(Frame::Control(msg))) => msg,
                Some(Ok(Frame::Grid(_))) => continue, // clients never send grid frames
                Some(Err(e)) => return Err(io::Error::other(e)),
                None => break, // EOF
            },
            Some(event) = internal_rx.recv() => {
                match event {
                    // A start this connection asked for has finished. If the client is waiting to
                    // view that session, this is the moment its stream can exist.
                    Internal::SessionStarted(session) => {
                        if viewing == Some(session) {
                            if let Some((pty, framer)) =
                                state.live_session(session).zip(state.session_framer(session))
                            {
                                restart_view(state, id, &mut view_stream, pty, framer);
                            }
                        }
                    }
                }
                continue;
            }
        };

        match msg {
            ClientMsg::Ping { nonce } => state.send(id, DaemonMsg::Pong { nonce }),
            ClientMsg::Goodbye => break,
            ClientMsg::Attach { project, force } => {
                match state.attach(id, project.clone(), force) {
                    Ok(_sessions) => {
                        tracing::info!(client = id, project = %project.display(), force, "project attached");
                        // Prune this project's empty sessions now that it has an attached observer
                        // (FR-007a, T056) — before building the `Attached` list so a just-pruned
                        // phantom never flashes in the sidebar.
                        prune_empty_off_runtime(state, &project).await;
                        state.send(
                            id,
                            DaemonMsg::Attached {
                                project: project.clone(),
                                sessions: state.sessions_for(&project),
                            },
                        );
                        // Discover this project's worktrees from git now that a client is looking at
                        // it, then the sessions its CLIs recorded there that we have no record of
                        // (feature 026, FR-014 — research R15), and send the refreshed catalog to
                        // *this* client only (FR-018, T053). Attach is per-client and exclusive, so
                        // a broadcast would be both wrong (others aren't in this project) and
                        // disruptive to their message stream.
                        //
                        // The order is load-bearing: discovery reads the worktree cache the refresh
                        // just filled, and both must land before the snapshot is built or a
                        // discovered session appears only on the *next* open.
                        refresh_worktrees_and_send(state, id, project).await;
                    }
                    Err(reason) => {
                        tracing::info!(client = id, project = %project.display(), "attach refused: project busy");
                        state.send(id, DaemonMsg::Refused { reason })
                    }
                }
            }
            ClientMsg::Detach { project } => {
                tracing::info!(client = id, project = %project.display(), "project detached");
                state.detach(id, &project);
            }
            // --- AI CLIs (feature 027, FR-023c) ---
            //
            // Answered *here*, in the service, because here is where sessions run. Under sandboxed
            // placement this process is inside the container, so `available_here` reads the
            // image's `PATH` — which is the question FR-023c asks and the one the client cannot
            // ask for itself. Under host placement it reads the host's, and the same code path
            // gives the same right answer for the same reason.
            //
            // Recomputed per request rather than cached at boot: it is one `PATH` walk per
            // variant, the client only asks when a choice is offered, and research R11's rule is
            // that this answer is never stored.
            ClientMsg::AiCliAvailabilityRequest { req } => {
                let available = micold_core::provider::available_here();
                tracing::debug!(client = id, ?available, "AI CLI availability reported");
                state.send(id, DaemonMsg::AiCliAvailability { req, available });
            }

            // --- Diagnostics (US6/Phase 10, FR-043–046) ---
            ClientMsg::LogLocationRequest { req } => {
                let (path, sink) = state
                    .diagnostics()
                    .map(|d| (d.path.clone(), d.sink))
                    .unwrap_or((None, LogSink::Stderr));
                state.send(id, DaemonMsg::LogLocation { req, path, sink });
            }
            ClientMsg::RecentErrorsRequest { req, limit } => {
                let entries = state
                    .diagnostics()
                    .map(|d| d.recent_errors(limit as usize))
                    .unwrap_or_default();
                state.send(id, DaemonMsg::RecentErrors { req, entries });
            }
            ClientMsg::SetLogLevel { req, directives } => match state.diagnostics() {
                Some(d) => match d.set_directives(&directives) {
                    Ok(()) => {
                        // The directives are operator-supplied config, never terminal content (FR-047).
                        tracing::info!(%directives, "log level changed");
                        send_ack(state, id, req);
                    }
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::InvalidInput,
                            message: "invalid log directives".into(),
                            detail: Some(e),
                        },
                    ),
                },
                None => state.send(
                    id,
                    DaemonMsg::OperationError {
                        req,
                        kind: ErrorKind::Internal,
                        message: "diagnostics are not available".into(),
                        detail: None,
                    },
                ),
            },
            ClientMsg::SessionInput {
                session,
                serial,
                bytes,
            } => state.session_input(session, serial, &bytes),
            ClientMsg::SessionStart { session } => {
                // Bringing an existing durable session back is a resume.
                //
                // Spawned, not run here (BUG-009 T125, FR-026a): `start_session` resolves the
                // user's environment-include script, which is a subprocess with a timeout the user
                // can set to 60 s, and it used to run *directly on this loop* — so a version
                // manager waiting on the network silenced the connection well past the client's 9 s
                // liveness deadline. `begin_start` holds any input typed meanwhile so nothing is
                // lost to the gap it opens (protocol.md §7).
                state.begin_start(session);
                spawn_session_start(
                    state,
                    session,
                    LaunchMode::Resume,
                    None,
                    internal_tx.clone(),
                );
            }
            ClientMsg::ScrollbackRequest {
                session,
                req,
                ranges,
            } => {
                // Advisory, never an error (protocol.md §6): serve whatever the session's shared
                // framer can from its retained history, resolved against a per-response palette.
                if let (Some(pty), Some(framer)) =
                    (state.live_session(session), state.session_framer(session))
                {
                    let responses: Vec<DaemonMsg> = {
                        let fr = framer.lock().expect("framer poisoned");
                        let oldest_available = fr.oldest_available();
                        let newest = fr.newest(pty.term());
                        ranges
                            .into_iter()
                            .map(|range| {
                                // saturating_sub: never let an adversarial reversed/extreme range
                                // (e.g. start = i64::MIN) overflow the subtraction and panic.
                                let count =
                                    range.end.0.saturating_sub(range.start.0).max(0) as usize;
                                let (lines, styles, hyperlinks, more) =
                                    fr.scrollback_range(pty.term(), range.start, count);
                                DaemonMsg::ScrollbackResponse {
                                    session,
                                    req,
                                    oldest_available,
                                    newest,
                                    lines,
                                    styles,
                                    hyperlinks,
                                    more,
                                }
                            })
                            .collect()
                    };
                    for resp in responses {
                        state.send(id, resp);
                    }
                }
            }
            ClientMsg::SessionCreate {
                req,
                project,
                worktree_dir,
                provider,
            } => {
                // The catalog write stays on the loop: it is a small atomic file write, and it is
                // what mints the id every later message refers to. Only the spawn — the slow half
                // — is deferred (BUG-009, T125).
                match state.create_session(&project, &worktree_dir, provider) {
                    Ok(session) => {
                        // A brand-new session starts fresh (`claude --session-id`), never `--resume`
                        // against a conversation that does not exist yet.
                        //
                        // The reply rides with the start rather than preceding it, so the client
                        // still learns of the session exactly when it is usable — the ordering it
                        // had when this ran inline. `begin_start` is belt and braces here (the
                        // client cannot type into an id it has not been told yet), kept for one
                        // rule rather than two.
                        state.begin_start(session);
                        spawn_session_start(
                            state,
                            session,
                            LaunchMode::Fresh,
                            Some((id, req)),
                            internal_tx.clone(),
                        );
                    }
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::IoFailed,
                            message: "failed to create the session".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            ClientMsg::SetViewedSession { project, session } => {
                state.set_viewed(id, project, session);
                viewing = session;
                match session.and_then(|s| state.live_session(s).zip(state.session_framer(s))) {
                    Some((pty, framer)) => restart_view(state, id, &mut view_stream, pty, framer),
                    // Not live *yet* is the ordinary case now: the client sends `SessionStart` and
                    // `SetViewedSession` back to back, and the start no longer completes before this
                    // arrives (BUG-009, T125). `viewing` above records the intent, and the
                    // `Internal::SessionStarted` arm builds the stream when the session exists.
                    None => {
                        if let Some(prev) = view_stream.take() {
                            prev.abort();
                        }
                    }
                }
            }
            // --- Feature 011: shell instances + which process is attached ---
            ClientMsg::SessionAttachProcess { session, process } => {
                match state.attach_process(session, process) {
                    Some((pty, framer)) => restart_view(state, id, &mut view_stream, pty, framer),
                    // The client asked to display a process the daemon does not have, so the two
                    // now disagree about what is attached — the client will show its new mode while
                    // the pane keeps streaming whatever it streamed before (FR-007). Silently
                    // ignoring this is what let a Regular Terminal toggle do nothing at all with no
                    // trace anywhere (BUG-001, feature 010-regular-terminal-mode).
                    None => tracing::warn!(
                        session = %session.0,
                        ?process,
                        "attach requested for a process the session does not have"
                    ),
                }
            }
            ClientMsg::SessionOpenShell { session, instance } => {
                match state.open_shell(session, instance) {
                    // Fire-and-forget: the client gets no reply, so this log is the only place a
                    // failed shell open is visible at all.
                    Err(err) => {
                        tracing::warn!(session = %session.0, instance = instance.0, %err, "open shell failed")
                    }
                    Ok(()) => {
                        tracing::info!(session = %session.0, instance = instance.0, "shell instance opened");
                        // The set of live shell instances just changed, so say so (`012` FR-008,
                        // BUG-003). Publishing `live_shells` in the snapshot is not enough on its
                        // own: nothing else broadcasts on this path, so without this the client
                        // holds the instance at `Starting` until some unrelated change happens to
                        // push a snapshot — which is exactly what the visual pass found.
                        state.broadcast_catalog();
                    }
                }
            }
            ClientMsg::SessionCloseShell { session, instance } => {
                if let Some((pty, framer)) = state.close_shell(session, instance) {
                    restart_view(state, id, &mut view_stream, pty, framer);
                }
                // Closed or not (the id may name nothing), the live set may have changed.
                state.broadcast_catalog();
            }
            ClientMsg::SessionRestartShell { session, instance } => {
                // `close_shell` returns the primary it reattached to iff this instance was attached.
                let reattached_primary = state.close_shell(session, instance);
                match state.open_shell(session, instance) {
                    Ok(()) if reattached_primary.is_some() => {
                        // It was attached: re-attach the fresh instance so view + input follow it.
                        if let Some((pty, framer)) =
                            state.attach_process(session, SessionProcess::Shell(instance))
                        {
                            restart_view(state, id, &mut view_stream, pty, framer);
                        }
                    }
                    Ok(()) => {}
                    Err(err) => {
                        tracing::warn!(session = %session.0, instance = instance.0, %err, "restart shell failed");
                        // Respawn failed: fall back to the primary `close_shell` reattached to, so
                        // the view stream and input routing agree (both on Primary) instead of the
                        // view showing the dead shell while input goes to Primary.
                        if let Some((pty, framer)) = reattached_primary {
                            restart_view(state, id, &mut view_stream, pty, framer);
                        }
                    }
                }
                // A restart is a death and a birth; both change the live set (`012` BUG-003).
                state.broadcast_catalog();
            }
            ClientMsg::SessionResize {
                session,
                cols,
                rows,
            } => state.resize_session(session, cols, rows),
            ClientMsg::SessionKill { session } | ClientMsg::SessionStop { session } => {
                // Stop the session's processes and drop it from the live registry (kill happens
                // outside the state lock inside remove_session). TODO(T053): archive the durable
                // record so reconciliation can't resurrect it, and broadcast the catalog.
                for pty in state.remove_session(session) {
                    let _ = pty.kill();
                }
            }
            ClientMsg::SessionInterrupt { session } => {
                // Ctrl-C to the attached process — 0x03 to the PTY, never a real signal (§7).
                if let Some(pty) = state.live_session(session) {
                    if let Err(err) = pty.write_input(&[0x03]) {
                        tracing::warn!(session = %session.0, %err, "interrupt write failed");
                    }
                }
            }
            ClientMsg::SettingsSet {
                req,
                scrollback_lines,
                env_include_enabled,
                env_include_script_path,
                env_include_timeout_secs,
                default_ai_cli,
            } => {
                let result = match scrollback_lines {
                    Some(lines) => state.set_scrollback(lines),
                    None => Ok(()),
                }
                .and_then(|()| {
                    if env_include_enabled.is_none()
                        && env_include_script_path.is_none()
                        && env_include_timeout_secs.is_none()
                    {
                        Ok(())
                    } else {
                        state.set_env_include(
                            env_include_enabled,
                            env_include_script_path,
                            env_include_timeout_secs,
                        )
                    }
                })
                .and_then(|()| match default_ai_cli {
                    Some(which) => state.set_default_ai_cli(which),
                    None => Ok(()),
                });
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
            // --- US3: worktree management through the daemon (T053) ---
            ClientMsg::WorktreeCreate {
                req,
                project,
                branch,
                dir_name,
                mode,
            } => {
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                let names = DerivedNames {
                    dir_name: dir_name.clone(),
                    branch,
                };
                // git worktree add can take minutes (a submodule fetch), so it runs off the async
                // runtime; it never touches the state lock. When a create is rolled back the target
                // dir is removed (the fs half of the rollback) so no leftover directory survives
                // (FR-034, T050) — but ONLY then: the pre-flight refusals below reject before
                // anything is created, and the directory they name is the user's, not ours.
                // Feature 016 FR-024: stream the stage as the create advances, so the form can
                // name the step being performed ("Checking out existing branch" rather than
                // "Creating branch"). The client owns the wording; only the stage travels.
                //
                // Pushed through the client's own ordered frame channel — a clone of the sender
                // moves into the blocking task, so the git work never touches the state lock and
                // never blocks the runtime. A departed client simply drops the sends.
                //
                // BUG-009 (T120, FR-026a): the whole operation is *spawned*, not awaited here.
                // `spawn_blocking` frees the runtime; it does not free this loop, and this loop is
                // the only place this client's `Ping` is answered. Awaiting the join handle inline
                // therefore made a working daemon silent for the length of a submodule fetch, and
                // the client's 9 s liveness deadline reaped it — see `bugs/BUG-009.md`. Nothing in
                // this arm may block the loop again: every reply below goes through the client's
                // ordered frame channel, which is exactly what a departed client drops harmlessly.
                let progress_tx = state.frame_sender(id);
                // Read before the blocking task, like `repo`: the included set decides how a
                // blocked holder is described (016 BUG-002, FR-032), and the lock must not be held
                // across the git work.
                let included = state.included_worktrees(&project);
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let state = &task_state;
                    // Mutating worktree work is serialized per project. The inline `.await` used to
                    // provide this as a side effect of blocking the loop; spawning would otherwise
                    // let two creates interleave, and a second create's rollback removes `target`
                    // (above) — which for a same-named racing pair is the *first* create's freshly
                    // populated directory. Per project rather than globally: two projects have no
                    // shared git state to race over. Recorded per T120.
                    let gate = state.worktree_gate(&project);
                    let _serialized = gate.lock().await;
                    let result = tokio::task::spawn_blocking(move || {
                        let root = repo.join(".claude/worktrees");
                        let target = root.join(&names.dir_name);
                        let _ = std::fs::create_dir_all(&root);
                        let target_exists = target.exists()
                            && std::fs::read_dir(&target)
                                .map(|mut d| d.next().is_some())
                                .unwrap_or(false);
                        // A stage transition always gets a frame; within a stage the live output is
                        // forwarded at a fixed rate (BUG-009, T123 — the rule and its reasoning
                        // live in `ProgressThrottle`, with the clock injected so it is testable).
                        let mut throttle = ProgressThrottle::new(PROGRESS_DETAIL_MIN_GAP);
                        let mut on_progress = |event: CreateProgressEvent| {
                            let stage = event.stage;
                            let Some(detail) = throttle.admit(event, std::time::Instant::now())
                            else {
                                return;
                            };
                            if let Some(tx) = &progress_tx {
                                let _ = tx.send(Frame::Control(DaemonMsg::OperationProgress {
                                    req,
                                    stage,
                                    detail,
                                }));
                            }
                        };
                        let r = create_worktree(
                            &GitCli::new(),
                            &repo,
                            &target,
                            &names,
                            target_exists,
                            &mode,
                            &included,
                            &mut on_progress,
                        );
                        // `RolledBack` is the only outcome in which this attempt created anything
                        // at `target`. `DuplicateDir` in particular means the directory was already
                        // there — removing it would destroy the user's files (feature 016).
                        if matches!(r, Err(CreateError::RolledBack(_))) {
                            let _ = std::fs::remove_dir_all(&target);
                        }
                        r
                    })
                    .await;
                    match result {
                        Ok(Ok(_worktree)) => {
                            refresh_worktrees_and_broadcast(state, project).await;
                            state.send(
                                id,
                                DaemonMsg::OperationOk {
                                    req,
                                    result: OperationResult::WorktreeCreated { dir_name },
                                },
                            );
                        }
                        Ok(Err(e)) => {
                            let (kind, message, detail) = describe_create_error(e);
                            state.send(
                                id,
                                DaemonMsg::OperationError {
                                    req,
                                    kind,
                                    message,
                                    detail,
                                },
                            );
                        }
                        Err(join) => state.send(id, task_failed(req, "worktree create", &join)),
                    }
                });
            }
            // --- feature 016: read-only branch queries for the create form ---
            //
            // Both run on the blocking pool: they shell out to git (`worktree list --porcelain`,
            // `for-each-ref`) and must not stall the async runtime. Neither mutates anything, and
            // neither contacts a remote (FR-020).
            ClientMsg::BranchPreflight {
                req,
                project,
                branch,
                dir_name,
            } => {
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                let included = state.included_worktrees(&project);
                let situation = tokio::task::spawn_blocking(move || {
                    let target = repo.join(".claude/worktrees").join(&dir_name);
                    let target_exists = target.exists()
                        && std::fs::read_dir(&target)
                            .map(|mut d| d.next().is_some())
                            .unwrap_or(false);
                    preflight(
                        &GitCli::new(),
                        &repo,
                        &target,
                        &branch,
                        target_exists,
                        &included,
                    )
                })
                .await;
                match situation {
                    Ok(Ok(situation)) => state.send(
                        id,
                        DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::BranchPreflight { situation },
                        },
                    ),
                    Ok(Err(e)) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::GitFailed,
                            message: "could not check the branch".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::Internal,
                            message: "could not check the branch".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            // Feature 027 (research R2 part 2): the open-project gate, answered here because on
            // Windows — and for any remote daemon — the client's filesystem is not this one. It is
            // deliberately the *same* question `GitCli::is_repo_root` answers, on the same binary,
            // rather than a cheaper `.git` stat: a client that asks this and a client that answers
            // it locally must not disagree about what counts as a repository.
            ClientMsg::RepoRootQuery { req, path } => {
                let probed = path.clone();
                // `git rev-parse` on a cold or network-mounted directory is not instant, and this
                // task drives every other client's frames.
                let is_repo_root =
                    tokio::task::spawn_blocking(move || GitCli::new().is_repo_root(&probed)).await;
                match is_repo_root {
                    Ok(is_repo_root) => state.send(
                        id,
                        DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::RepoRoot { path, is_repo_root },
                        },
                    ),
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::Internal,
                            message: "could not check whether that folder is a repository".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            ClientMsg::BranchList { req, project } => {
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                let included = state.included_worktrees(&project);
                let listed = tokio::task::spawn_blocking(move || {
                    branch_candidates(&GitCli::new(), &repo, &included)
                })
                .await;
                match listed {
                    Ok(Ok(candidates)) => state.send(
                        id,
                        DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::BranchList { candidates },
                        },
                    ),
                    Ok(Err(e)) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::GitFailed,
                            message: "could not list branches".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::Internal,
                            message: "could not list branches".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            ClientMsg::WorktreeDelete {
                req,
                project,
                dir_name,
                stop_sessions,
                delete_branch,
            } => {
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                // Never orphan a live process: a delete with a live session and `stop_sessions:false`
                // fails specifically instead (W2, T052). No mutation has happened yet.
                // FR-045: a worktree delete is a destructive, frequently-failing operation whose
                // reason lives entirely in git's stderr. It used to log nothing at all — not the
                // attempt, not the refusal, not the failure — so a user watching a delete fail had
                // no way to find out why: the reason reached the client's transient notification
                // and nowhere else. Both outcomes are logged below. Worktree/branch names and git's
                // own message are identity and error text, never terminal content (FR-047).
                tracing::info!(
                    project = %project.display(),
                    worktree = %dir_name,
                    stop_sessions,
                    delete_branch,
                    "worktree delete requested"
                );
                let live = state.worktree_live_sessions(&project, &dir_name);
                if !live.is_empty() && !stop_sessions {
                    tracing::warn!(
                        project = %project.display(),
                        worktree = %dir_name,
                        live_sessions = live.len(),
                        "worktree delete refused: live sessions and stop_sessions not set"
                    );
                    state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::Busy,
                            message: "worktree has a live session — stop it first or retry with \
                                      stop_sessions"
                                .into(),
                            detail: None,
                        },
                    );
                    continue;
                }
                // Computed before `repo` moves into the closure below — the same path a session
                // located in this worktree resolves as its `cwd`, so the env-include cache entry for
                // it can be dropped once the delete succeeds (BUG-003: a worktree recreated for the
                // same branch reuses this exact path).
                let cache_path = repo.join(".claude/worktrees").join(&dir_name);
                // Feature 013 (FR-011/FR-012): the user's explicit keep/delete choice, resolved
                // against the worktree's actual bound branch (from the live git-discovery cache,
                // not guessed from `dir_name`) — `None` for either an unbound/orphan worktree or
                // when the user chose to keep the branch.
                let branch_to_delete = if delete_branch {
                    state.worktree_branch(&project, &dir_name)
                } else {
                    None
                };
                let dir2 = dir_name.clone();
                // Spawned rather than awaited here, for the same reason as `WorktreeCreate` above
                // (BUG-009, T120/T124, FR-026a): `remove_dir_all` over a populated worktree —
                // dependency trees, build output, initialized submodules — is unbounded work, and on
                // a network filesystem it is slow enough to cross the client's liveness deadline on
                // its own. It takes the same per-project gate, so a delete and a create on one
                // project still serialize.
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let state = &task_state;
                    let gate = state.worktree_gate(&project);
                    let _serialized = gate.lock().await;
                    let result = tokio::task::spawn_blocking(move || {
                        let target = repo.join(".claude/worktrees").join(&dir2);
                        let outcome = remove_worktree(
                            &GitCli::new(),
                            &repo,
                            &target,
                            branch_to_delete.as_deref(),
                        )?;
                        // Leftovers are NOT an error: git has already deregistered the worktree
                        // above, so the delete did partly succeed. Failing the whole operation here
                        // skipped the session cleanup and left the directory to come back as an
                        // unregistered orphan — the "I deleted it and it reappeared" report.
                        let leftovers = remove_worktree_dir(&target);
                        Ok::<(bool, Vec<Leftover>), std::io::Error>((
                            outcome.branch_delete_failed,
                            leftovers,
                        ))
                    })
                    .await;
                    match result {
                        Ok(Ok((branch_delete_failed, leftovers))) => {
                            // Gated on the git delete having succeeded (main `d88c7a1`): only now archive
                            // the worktree's sessions durably and kill their live procs (outside the lock).
                            match state.archive_and_remove_worktree_sessions(&project, &dir_name) {
                                Ok(ptys) => {
                                    for pty in ptys {
                                        let _ = pty.kill();
                                    }
                                }
                                Err(e) => {
                                    tracing::warn!(%e, "archiving deleted worktree's sessions failed")
                                }
                            }
                            state.invalidate_env_include(&cache_path);
                            if leftovers.is_empty() {
                                tracing::info!(
                                    project = %project.display(),
                                    worktree = %dir_name,
                                    branch_delete_failed,
                                    "worktree deleted"
                                );
                            } else {
                                // WARN, not ERROR: git released the worktree and the sessions are
                                // archived, so this is a partial success. Naming the blockers and their
                                // owner is the whole point — `remove_dir_all` reports only the first
                                // errno, which reached the user as a bare "Permission denied (os error
                                // 13)" for a tree they had no way to identify (FR-023).
                                tracing::warn!(
                                    project = %project.display(),
                                    worktree = %dir_name,
                                    branch_delete_failed,
                                    leftovers = %describe_leftovers(&leftovers),
                                    "worktree deregistered, but its directory could not be fully removed"
                                );
                            }
                            refresh_worktrees_and_broadcast(state, project).await;
                            state.send(
                                id,
                                DaemonMsg::OperationOk {
                                    req,
                                    result: OperationResult::WorktreeDeleted {
                                        branch_delete_failed,
                                        leftovers,
                                    },
                                },
                            );
                        }
                        // Failed delete: sessions are left untouched (not killed, not archived) so an
                        // FR-023-recoverable failure never becomes permanent loss. git's stderr rides along.
                        // Logged at ERROR so it also lands in the recent-errors ring (FR-046) — this is
                        // the only durable record of *why* a delete the user watched fail did fail.
                        Ok(Err(e)) => {
                            tracing::error!(
                                project = %project.display(),
                                worktree = %dir_name,
                                error = %e,
                                "worktree delete failed"
                            );
                            state.send(
                                id,
                                DaemonMsg::OperationError {
                                    req,
                                    kind: ErrorKind::GitFailed,
                                    message: "failed to remove the worktree".into(),
                                    detail: Some(e.to_string()),
                                },
                            )
                        }
                        Err(join) => {
                            tracing::error!(
                                project = %project.display(),
                                worktree = %dir_name,
                                error = %join,
                                "worktree delete task failed"
                            );
                            state.send(id, task_failed(req, "worktree delete", &join))
                        }
                    }
                });
            }
            ClientMsg::WorktreeRename {
                req,
                project,
                dir_name,
                display_name,
            } => {
                // A display-name override is durable catalog state — no git involved. Validation
                // (InvalidInput) and persistence (IoFailed) are separate, individually-mappable steps.
                match validate_rename(&display_name) {
                    Ok(name) => match state.set_worktree_display_name(&project, &dir_name, &name) {
                        Ok(()) => {
                            state.broadcast_catalog();
                            state.send(
                                id,
                                DaemonMsg::OperationOk {
                                    req,
                                    result: OperationResult::Ack,
                                },
                            );
                        }
                        Err(e) => state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::IoFailed,
                                message: "failed to persist the rename".into(),
                                detail: Some(e.to_string()),
                            },
                        ),
                    },
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::InvalidInput,
                            message: rename_error_message(e).into(),
                            detail: None,
                        },
                    ),
                }
            }
            // --- 016 BUG-002: showing a worktree the app does not manage (FR-027/FR-030) ---
            ClientMsg::WorktreeInclude { req, project, path } => {
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                // The repository has to actually know this worktree. Recording a location git does
                // not report would persist a wish that can never resolve into a row — and the whole
                // point of including one is that it already exists (contract `branch-rpc.md` §3a).
                let probe = path.clone();
                let known = tokio::task::spawn_blocking(move || {
                    let porcelain = GitCli::new()
                        .worktree_list_porcelain(&repo)
                        .unwrap_or_default();
                    parse_worktrees(&porcelain)
                        .into_iter()
                        .any(|rec| rec.path == probe)
                })
                .await;
                match known {
                    Ok(true) => {}
                    Ok(false) => {
                        state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::NotFound,
                                message: "that is not one of this repository's worktrees".into(),
                                detail: Some(path.display().to_string()),
                            },
                        );
                        continue;
                    }
                    Err(e) => {
                        state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::Internal,
                                message: "could not check the repository's worktrees".into(),
                                detail: Some(e.to_string()),
                            },
                        );
                        continue;
                    }
                }
                // Settings only — no git command runs, and nothing on disk moves (FR-028).
                match state.include_worktree(&project, &path) {
                    Ok(()) => {
                        refresh_worktrees_and_broadcast(state, project.clone()).await;
                        match state.worktree_snapshot_at(&project, &path) {
                            Some(worktree) => state.send(
                                id,
                                DaemonMsg::OperationOk {
                                    req,
                                    result: OperationResult::WorktreeIncluded { worktree },
                                },
                            ),
                            // Discovery ran and did not produce it — the worktree went away between
                            // the check above and the refresh. Say so rather than acknowledging a
                            // row the client would then wait for.
                            None => state.send(
                                id,
                                DaemonMsg::OperationError {
                                    req,
                                    kind: ErrorKind::NotFound,
                                    message: "the worktree is no longer there".into(),
                                    detail: Some(path.display().to_string()),
                                },
                            ),
                        }
                    }
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::IoFailed,
                            message: "failed to persist the included worktree".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            ClientMsg::WorktreeExclude { req, project, path } => {
                match state.exclude_worktree(&project, &path) {
                    Ok(()) => {
                        refresh_worktrees_and_broadcast(state, project).await;
                        state.send(
                            id,
                            DaemonMsg::OperationOk {
                                req,
                                result: OperationResult::WorktreeExcluded { path },
                            },
                        );
                    }
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::IoFailed,
                            message: "failed to stop showing the worktree".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            // --- US3: project management + session delete through the daemon (T053) ---
            ClientMsg::ProjectAdd { req, path } => match state.add_project(&path) {
                Ok(()) => {
                    refresh_worktrees_and_broadcast(state, path).await;
                    send_ack(state, id, req);
                }
                Err(e) => send_io_error(state, id, req, "failed to add the project", &e),
            },
            ClientMsg::ProjectRemove { req, path } => match state.forget_project(&path) {
                Ok(ptys) => {
                    for pty in ptys {
                        let _ = pty.kill();
                    }
                    state.broadcast_catalog();
                    send_ack(state, id, req);
                }
                Err(e) => send_io_error(state, id, req, "failed to remove the project", &e),
            },
            ClientMsg::ProjectRename {
                req,
                path,
                display_name,
            } => match validate_rename(&display_name) {
                Ok(name) => match state.rename_project(&path, &name) {
                    Ok(()) => {
                        state.broadcast_catalog();
                        send_ack(state, id, req);
                    }
                    Err(e) => send_io_error(state, id, req, "failed to persist the rename", &e),
                },
                Err(e) => state.send(
                    id,
                    DaemonMsg::OperationError {
                        req,
                        kind: ErrorKind::InvalidInput,
                        message: rename_error_message(e).into(),
                        detail: None,
                    },
                ),
            },
            ClientMsg::SessionDelete { req, session } => match state.delete_session(session) {
                Ok((owner, ptys)) => {
                    for pty in ptys {
                        let _ = pty.kill();
                    }
                    match owner {
                        Some(_) => {
                            state.broadcast_catalog();
                            send_ack(state, id, req);
                        }
                        None => state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::NotFound,
                                message: "unknown session".into(),
                                detail: None,
                            },
                        ),
                    }
                }
                Err(e) => send_io_error(state, id, req, "failed to delete the session", &e),
            },
            // Any remaining unhandled control message is ignored.
            _ => {}
        }
    }
    if let Some(stream) = view_stream.take() {
        stream.abort();
    }
    Ok(())
}

/// Render leftover paths for one log field: `path (uid N)`, comma-separated.
///
/// The owner is what makes the line actionable — a foreign uid means the daemon cannot unlink the
/// entry no matter how often the user retries, and points straight at the cause (typically a
/// container that wrote build output through a bind mount as root).
fn describe_leftovers(leftovers: &[Leftover]) -> String {
    leftovers
        .iter()
        .map(|l| match l.foreign_uid {
            Some(uid) => format!("{} (uid {uid})", l.path.display()),
            None => l.path.display().to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Re-discover a project's worktrees from git (off the async runtime, never under the state lock) and
/// push the refreshed catalog to every client, so a worktree mutation propagates to all windows
/// without further user action (FR-011, T053).
async fn refresh_worktrees_and_broadcast(state: &Arc<DaemonState>, project: std::path::PathBuf) {
    refresh_worktrees_off_runtime(state, &project).await;
    state.broadcast_catalog();
}

/// Re-discover a project's worktrees and send the refreshed catalog to a single client — the
/// per-client attach case, where a broadcast would reach clients not in this project.
async fn refresh_worktrees_and_send(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    project: std::path::PathBuf,
) {
    refresh_worktrees_off_runtime(state, &project).await;
    state.send(
        id,
        DaemonMsg::CatalogChanged {
            catalog: state.catalog_snapshot(),
        },
    );
}

/// Run the (blocking) git worktree discovery off the async runtime, updating the cache — and, in
/// the same hop, the FR-014 pass that finds sessions started outside this application.
///
/// One `spawn_blocking` rather than two: the second step reads the worktree cache the first just
/// filled, so they are one unit of blocking work and splitting them would add a runtime round trip
/// for nothing (research R15).
async fn refresh_worktrees_off_runtime(state: &Arc<DaemonState>, project: &std::path::Path) {
    let st = Arc::clone(state);
    let proj = project.to_path_buf();
    let discovered = tokio::task::spawn_blocking(move || {
        st.refresh_worktrees(&proj);
        st.discover_external_sessions(&proj)
    })
    .await;
    if let Ok(count) = discovered {
        if count > 0 {
            tracing::info!(
                project = %project.display(),
                count,
                "adopted sessions started outside this application"
            );
        }
    }
}

/// Run empty-session pruning (which stats the provider's conversation store) off the async runtime.
async fn prune_empty_off_runtime(state: &Arc<DaemonState>, project: &std::path::Path) {
    let st = Arc::clone(state);
    let proj = project.to_path_buf();
    match tokio::task::spawn_blocking(move || st.prune_empty_sessions(&proj)).await {
        Ok(Ok(pruned)) if !pruned.is_empty() => {
            tracing::info!(project = %project.display(), count = pruned.len(), "pruned empty sessions")
        }
        Ok(Err(e)) => tracing::warn!(%e, "empty-session prune failed"),
        _ => {}
    }
}

/// Reply to a worktree RPC for a path that is not a known git-repo project. A missing project is
/// `NotFound`; a known non-git project is `Refused` (worktrees need a repo).
fn reject_non_repo(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    req: u64,
    project: &std::path::Path,
) {
    let (kind, message) = match state.project_repo(project) {
        Some((_, false)) => (ErrorKind::Refused, "project is not a git repository"),
        _ => (ErrorKind::NotFound, "unknown project"),
    };
    state.send(
        id,
        DaemonMsg::OperationError {
            req,
            kind,
            message: message.into(),
            detail: None,
        },
    );
}

/// Map a [`CreateError`] to its wire error. A duplicate branch/dir is the specific, actionable
/// `AlreadyExists` (caught pre-flight, before any git mutation); a git-level failure is `GitFailed`
/// carrying git's stderr verbatim (T050, FR-034).
fn describe_create_error(err: CreateError) -> (ErrorKind, String, Option<String>) {
    match err {
        // Feature 016 (FR-021): name the holder rather than reporting a bare failure. The sentence
        // comes from core, so a block caught here at create time reads exactly as the same block
        // caught by the client's pre-flight — two hand-written wordings is how BUG-001's holder
        // taxonomy came to be wrong in one place and right in neither.
        CreateError::BranchInUse { branch, reason } => {
            (ErrorKind::Busy, reason.explain(&branch), None)
        }
        // Feature 016 (FR-009): the branch changed between the user's answer and the act.
        CreateError::SituationChanged => (
            ErrorKind::Refused,
            "the branch changed while you were deciding, so nothing was done".into(),
            None,
        ),
        // Feature 016 (FR-022), BUG-003 item 3: the arm left behind when `BranchInUse` was fixed
        // after BUG-001, and the same defect. The sentence comes from core, so this reads exactly
        // as the form's own pre-flight refusal — including the half that says what to do about it,
        // which the hand-written wording here did not have.
        CreateError::DuplicateDir { dir } => (
            ErrorKind::AlreadyExists,
            explain_directory_taken(&dir).to_string(),
            None,
        ),
        CreateError::RolledBack(stderr) => (
            ErrorKind::GitFailed,
            "git failed to create the worktree".into(),
            Some(stderr),
        ),
    }
}

/// Reply to a correlated RPC with a bare success (`OperationOk { Ack }`).
fn send_ack(state: &Arc<DaemonState>, id: crate::state::ClientId, req: u64) {
    state.send(
        id,
        DaemonMsg::OperationOk {
            req,
            result: OperationResult::Ack,
        },
    );
}

/// Reply to a correlated RPC with an `IoFailed` error carrying the underlying error as detail.
fn send_io_error(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    req: u64,
    message: &str,
    e: &io::Error,
) {
    state.send(
        id,
        DaemonMsg::OperationError {
            req,
            kind: ErrorKind::IoFailed,
            message: message.into(),
            detail: Some(e.to_string()),
        },
    );
}

/// A plain-language message for a rejected display name (`RenameError` has no `Display`).
fn rename_error_message(err: micold_core::project::RenameError) -> &'static str {
    use micold_core::project::RenameError;
    match err {
        RenameError::Empty => "the name cannot be empty",
        RenameError::Whitespace => "the name cannot be only whitespace",
    }
}

/// Start a session off the connection loop (BUG-009 T125, FR-026a).
///
/// `start_session` is blocking twice over — it sources the user's environment-include script
/// (a subprocess, timeout configurable to 60 s) and forks a PTY — and it used to run on the loop
/// that answers this client's `Ping`. Here it runs on the blocking pool inside a spawned task, so
/// the loop keeps serving the connection for the whole start.
///
/// Three obligations the inline version met for free, met explicitly now:
/// - **Serialization**: the per-session gate, so two rapid starts cannot both see "not live" and
///   fork a process each.
/// - **Input ordering**: `finish_start` replays whatever was typed while the start ran, in arrival
///   order, and it runs on *every* outcome — a failed start must not strand held keystrokes
///   (protocol.md §7).
/// - **Reply ordering**: `reply` (Some for `SessionCreate`, None for `SessionStart`) is sent after
///   the start concludes, exactly where it was sent before.
///
/// A *failure*, by contrast, is announced regardless of `reply` (T087). The absent reply is
/// deliberate for the success case — a resume has no `SessionCreated` to send — but the reason a
/// start failed belongs to every client whatever asked for it, and the catalog is where it lives.
fn spawn_session_start(
    state: &Arc<DaemonState>,
    session: micold_core::session::SessionId,
    launch: LaunchMode,
    reply: Option<(crate::state::ClientId, u64)>,
    done: tokio::sync::mpsc::UnboundedSender<Internal>,
) {
    let task_state = Arc::clone(state);
    tokio::spawn(async move {
        let gate = task_state.session_gate(session);
        let _serialized = gate.lock().await;
        let worker = Arc::clone(&task_state);
        let outcome = tokio::task::spawn_blocking(move || {
            worker.start_session(session, launch)?;
            // Watch this session's own event log, for a provider that reports one (feature 026,
            // T064). In the same blocking hop as the spawn, and **only** for a session the daemon
            // has just started — that is what keeps a merely discovered session unwatched.
            worker.open_event_log_tail(session);
            Ok::<(), std::io::Error>(())
        })
        .await;
        match outcome {
            Ok(Ok(())) => {}
            // A failed start moves the catalog and, unlike a successful one, nothing else says so
            // (feature 026, T087, FR-010). `start_session` records the reason — it is what fills
            // the wire's `Failed { reason, attempts: 0 }` — and broadcasts only *after* it has
            // marked the session running, which it returned before doing. Announced here rather
            // than beside the reply below, because a resume has no reply to carry it:
            // `ClientMsg::SessionStart` carries no `req`, so there is no `OperationError` to
            // address to it, and pressing restart on a session whose CLI is gone did nothing
            // visible at all. The catalog is the surface both launch modes share, and the one the
            // `SessionCreate` path already relies on for exactly this.
            Ok(Err(err)) => {
                tracing::warn!(session = %session.0, %err, "session start failed");
                task_state.broadcast_catalog();
            }
            Err(join) => {
                tracing::warn!(session = %session.0, error = %join, "session start task failed");
                task_state.broadcast_catalog();
            }
        }
        // Before the reply, so a client that acts on `SessionCreated` immediately finds the session
        // already caught up on anything held.
        task_state.finish_start(session);
        // Tell the connection loop, which owns the view stream and may have been waiting to build
        // one for this session. A closed channel just means the client has gone.
        let _ = done.send(Internal::SessionStarted(session));
        if let Some((client, req)) = reply {
            task_state.send(
                client,
                DaemonMsg::OperationOk {
                    req,
                    result: OperationResult::SessionCreated { session },
                },
            );
            task_state.broadcast_catalog();
        }
    });
}

/// The error reply for a `spawn_blocking` task that itself failed (panicked / was cancelled) — an
/// internal fault, distinct from a git failure the task reported normally.
fn task_failed(req: u64, what: &str, join: &tokio::task::JoinError) -> DaemonMsg {
    DaemonMsg::OperationError {
        req,
        kind: ErrorKind::Internal,
        message: format!("{what} task failed"),
        detail: Some(join.to_string()),
    }
}

/// How often the view stream wakes to check for new output. Between ticks, output is coalesced into
/// a single delta (the VT dirty flag collapses many wakeups into one), so a briefly-slow reader
/// never falls behind unbounded (spec SC — screen state is lossy and convergent, unlike input).
const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_millis(16);

/// Abort the current view stream (if any) and start streaming `(pty, framer)` to client `id`. Used
/// whenever the attached process changes — a new viewed session, a process attach, or a close/restart
/// that reattaches to the primary.
fn restart_view(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    current: &mut Option<tokio::task::JoinHandle<()>>,
    pty: std::sync::Arc<crate::supervisor::PtySession>,
    framer: std::sync::Arc<std::sync::Mutex<crate::framer::Framer>>,
) {
    if let Some(prev) = current.take() {
        prev.abort();
    }
    if let Some(tx) = state.frame_sender(id) {
        *current = Some(tokio::spawn(stream_view(pty, framer, tx)));
    }
}

/// Stream grid frames for one viewed session to one client. Sends a full snapshot first (attach /
/// reattach semantics, FR-014/FR-017 — the client gets the current screen, not a replay), then
/// coalesced deltas whenever the VT reports new output. Ends when the client's channel closes
/// (disconnect) or the task is aborted (view changed / connection ended).
async fn stream_view(
    pty: std::sync::Arc<crate::supervisor::PtySession>,
    framer: std::sync::Arc<std::sync::Mutex<crate::framer::Framer>>,
    tx: tokio::sync::mpsc::UnboundedSender<Frame<DaemonMsg>>,
) {
    // Full snapshot on first view — the whole current screen, however long the client was away.
    let snapshot = framer
        .lock()
        .expect("framer poisoned")
        .frame(pty.term(), true, None);
    if tx.send(Frame::Grid(snapshot)).is_err() {
        return; // client already gone
    }

    let mut ticker = tokio::time::interval(FRAME_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        // Only frame when there is new output; a clean tick sends nothing.
        if pty.signals().take_dirty() {
            let delta = framer
                .lock()
                .expect("framer poisoned")
                .frame(pty.term(), false, None);
            if tx.send(Frame::Grid(delta)).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod create_error_tests {
    //! BUG-003 item 3 — the drift gate.
    //!
    //! Both surfaces that report a directory clash build their text from `explain_directory_taken`,
    //! and this is what says so of *this* one. The form's half is structural — it renders the two
    //! fields — but nothing stopped an edit here from writing a sentence again, which is exactly
    //! what had happened: `BranchInUse` was moved into core after BUG-001 and the arm one lower was
    //! left saying "a worktree with that name already exists", naming neither the folder nor the
    //! remedy.

    use super::*;

    #[test]
    fn a_directory_clash_is_reported_in_cores_own_words() {
        let dir = std::path::PathBuf::from("/repo/.claude/worktrees/feat-login");
        let (kind, message, detail) =
            describe_create_error(CreateError::DuplicateDir { dir: dir.clone() });

        assert_eq!(kind, ErrorKind::AlreadyExists);
        assert_eq!(message, explain_directory_taken(&dir).to_string());
        assert_eq!(detail, None);
    }

    /// …which means it names the folder and says what to do — the two things the hand-written
    /// version did not. Asserted against the *content* as well as against the source, because an
    /// equality with core would still pass if core's sentence lost either half.
    #[test]
    fn a_directory_clash_names_the_folder_and_offers_a_remedy() {
        let (_, message, _) = describe_create_error(CreateError::DuplicateDir {
            dir: std::path::PathBuf::from("/repo/.claude/worktrees/feat-login"),
        });
        assert!(message.contains("feat-login"), "{message}");
        assert!(message.contains("Choose a different name"), "{message}");
    }
}
