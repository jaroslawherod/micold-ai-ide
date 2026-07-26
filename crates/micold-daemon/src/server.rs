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
use micold_core::git::GitCli;
use micold_core::naming::DerivedNames;
use micold_core::project::validate_rename;
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::messages::{
    ClientMsg, DaemonMsg, ErrorKind, LogSink, OperationResult, SessionProcess,
};
use micold_core::terminal::LaunchMode;
use micold_core::worktree::{create_worktree, remove_worktree, remove_worktree_dir, CreateError};
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
    // Hand the diagnostics handle to the shared state so the `LogLocation`/`RecentErrors`/
    // `SetLogLevel` RPCs can serve it (FR-043–046).
    state.set_diagnostics(logging);

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

    // Writer task: drain this client's push channel to the wire. The channel already carries fully
    // formed frames — control messages (broadcasts, attach replies, pongs) and pushed grid frames —
    // in one ordered stream, so a grid delta never races ahead of the control it followed.
    let writer = tokio::spawn(async move {
        while let Some(frame) = rx.recv().await {
            if sink.send(frame).await.is_err() {
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
    // The grid stream for the session this client is currently viewing (FR-016). At most one runs
    // at a time; changing the viewed session aborts the old stream and starts the new one, and the
    // loop exit below stops it. The task pushes `Frame::Grid` into this client's ordered channel.
    let mut view_stream: Option<tokio::task::JoinHandle<()>> = None;

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
                        // it, and send the refreshed catalog to *this* client only (FR-018, T053).
                        // Attach is per-client and exclusive, so a broadcast would be both wrong
                        // (others aren't in this project) and disruptive to their message stream.
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
                if let Err(err) = state.start_session(session, LaunchMode::Resume) {
                    tracing::warn!(session = %session.0, %err, "session start failed");
                }
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
            } => {
                match state.create_session(&project, &worktree_dir) {
                    Ok(session) => {
                        // A brand-new session starts fresh (`claude --session-id`), never `--resume`
                        // against a conversation that does not exist yet.
                        if let Err(err) = state.start_session(session, LaunchMode::Fresh) {
                            tracing::warn!(session = %session.0, %err, "created session failed to start");
                        }
                        state.send(
                            id,
                            DaemonMsg::OperationOk {
                                req,
                                result: OperationResult::SessionCreated { session },
                            },
                        );
                        state.broadcast_catalog();
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
                match session.and_then(|s| state.live_session(s).zip(state.session_framer(s))) {
                    Some((pty, framer)) => restart_view(state, id, &mut view_stream, pty, framer),
                    None => {
                        if let Some(prev) = view_stream.take() {
                            prev.abort();
                        }
                    }
                }
            }
            // --- Feature 011: shell instances + which process is attached ---
            ClientMsg::SessionAttachProcess { session, process } => {
                if let Some((pty, framer)) = state.attach_process(session, process) {
                    restart_view(state, id, &mut view_stream, pty, framer);
                }
            }
            ClientMsg::SessionOpenShell { session, instance } => {
                if let Err(err) = state.open_shell(session, instance) {
                    tracing::warn!(session = %session.0, instance = instance.0, %err, "open shell failed");
                }
            }
            ClientMsg::SessionCloseShell { session, instance } => {
                if let Some((pty, framer)) = state.close_shell(session, instance) {
                    restart_view(state, id, &mut view_stream, pty, framer);
                }
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
            }
            ClientMsg::SessionResize {
                session,
                cols,
                rows,
            } => {
                // Resize every one of the session's processes so a later attach-switch shows a
                // correctly-sized grid, not the 100×30 spawn seed. PTY resize happens outside the
                // state lock (the handles are cloned Arcs).
                for pty in state.session_ptys(session) {
                    if let Err(err) = pty.resize(cols, rows) {
                        tracing::warn!(session = %session.0, %err, "resize failed");
                    }
                    // Force the stream to re-frame at the new size even for a process that doesn't
                    // redraw on SIGWINCH (the framer treats a size change as structural → full frame).
                    pty.signals().mark_dirty();
                }
            }
            ClientMsg::SessionKill { session } | ClientMsg::SessionStop { session } => {
                // Stop the session's processes and drop it from the live registry (kill happens
                // outside the state lock inside remove_session). TODO(T053): archive the durable
                // record so reconciliation can't resurrect it, and broadcast the catalog.
                if let Some(pty) = state.remove_session(session) {
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
            // --- US3: worktree management through the daemon (T053) ---
            ClientMsg::WorktreeCreate {
                req,
                project,
                branch,
                dir_name,
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
                // runtime; it never touches the state lock. On any failure the target dir is removed
                // (the fs half of the rollback) so no leftover directory survives (FR-034, T050).
                let result = tokio::task::spawn_blocking(move || {
                    let root = repo.join(".claude/worktrees");
                    let target = root.join(&names.dir_name);
                    let _ = std::fs::create_dir_all(&root);
                    let target_exists = target.exists()
                        && std::fs::read_dir(&target)
                            .map(|mut d| d.next().is_some())
                            .unwrap_or(false);
                    let r = create_worktree(
                        &GitCli::new(),
                        &repo,
                        &target,
                        &names,
                        target_exists,
                        &mut |_| {},
                    );
                    if r.is_err() {
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
            }
            ClientMsg::WorktreeDelete {
                req,
                project,
                dir_name,
                stop_sessions,
            } => {
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                // Never orphan a live process: a delete with a live session and `stop_sessions:false`
                // fails specifically instead (W2, T052). No mutation has happened yet.
                let live = state.worktree_live_sessions(&project, &dir_name);
                if !live.is_empty() && !stop_sessions {
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
                let dir2 = dir_name.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let target = repo.join(".claude/worktrees").join(&dir2);
                    // Keep the branch (`None`): branch deletion is irreversible and the wire carries
                    // no keep/delete choice, so losing only the worktree dir stays FR-023-recoverable.
                    remove_worktree(&GitCli::new(), &repo, &target, None)
                        .and_then(|_outcome| remove_worktree_dir(&target))
                })
                .await;
                match result {
                    Ok(Ok(())) => {
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
                        refresh_worktrees_and_broadcast(state, project).await;
                        state.send(
                            id,
                            DaemonMsg::OperationOk {
                                req,
                                result: OperationResult::Ack,
                            },
                        );
                    }
                    // Failed delete: sessions are left untouched (not killed, not archived) so an
                    // FR-023-recoverable failure never becomes permanent loss. git's stderr rides along.
                    Ok(Err(e)) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::GitFailed,
                            message: "failed to remove the worktree".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                    Err(join) => state.send(id, task_failed(req, "worktree delete", &join)),
                }
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
                Ok((owner, pty)) => {
                    if let Some(pty) = pty {
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

/// Run the (blocking) git worktree discovery off the async runtime, updating the cache.
async fn refresh_worktrees_off_runtime(state: &Arc<DaemonState>, project: &std::path::Path) {
    let st = Arc::clone(state);
    let proj = project.to_path_buf();
    let _ = tokio::task::spawn_blocking(move || st.refresh_worktrees(&proj)).await;
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
        CreateError::DuplicateBranch => (
            ErrorKind::AlreadyExists,
            "a branch with that name already exists".into(),
            None,
        ),
        CreateError::DuplicateDir => (
            ErrorKind::AlreadyExists,
            "a worktree with that name already exists".into(),
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
