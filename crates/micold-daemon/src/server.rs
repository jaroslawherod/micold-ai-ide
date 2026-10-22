//! Daemon startup + accept loop + per-connection routing (protocol.md §2/§4, plan W2, T020/T022).
//!
//! Resolves the endpoint and runs the single-instance sequence — the only way a listener is ever
//! bound, since feature 028 removed socket activation (lifecycle contract §1.3) — then serves
//! each accepted connection through the shared [`DaemonState`]: strict handshake, then attach/detach,
//! viewed-session, keepalive, and settings routing, with catalog/settings changes pushed to every
//! connected client. Grid streaming and the mutating RPCs layer on in Phase 3 / T053.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use micold_core::git::{containment, same_path, Git, GitCli};
use micold_core::naming::DerivedNames;
use micold_core::project::validate_rename;
use micold_core::protocol::codec::{DaemonCodec, Frame};
use micold_core::protocol::handshake;
use micold_core::protocol::messages::{
    BranchContainment, ClientIdentity, ClientMsg, DaemonMsg, ErrorKind, LogSink, MergedBranchQuery,
    OperationResult, RefusedRequest, SessionProcess, TerminalRef, WindowView,
};
use micold_core::terminal::LaunchMode;
use micold_core::worktree::{
    branch_candidates, explain_directory_taken, parse_worktrees, preflight, CreateError,
    CreateProgressEvent,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use crate::catalog::Catalog;
use crate::hooks;
use crate::idle::{self, StopReason};
use crate::logging;
use crate::ops;
use crate::progress::ProgressThrottle;
use crate::singleton::{self, Acquisition};
use crate::state::DaemonState;
use micold_core::endpoint;

/// A human-facing build string named in diagnostics and the handshake.
pub fn daemon_build() -> String {
    format!(
        "micold-daemon {}",
        micold_core::protocol::version::PACKAGE_VERSION
    )
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

    // The same rule as the host process, over a different transport (FR-018). A container whose
    // daemon has nobody connected is a container burning a machine's memory for nobody; because the
    // daemon is PID 1 there, returning from here is what makes the container exit.
    let idle = idle_watch(Arc::clone(&state));
    tokio::pin!(idle);
    let stop = crate::platform::stop_requested();
    tokio::pin!(stop);

    let reason = loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (conn, _peer) = accepted?;
                // Terminal traffic is small and latency-sensitive; Nagle would coalesce a keystroke
                // with whatever came next and show up to the user as input lag.
                let _ = conn.set_nodelay(true);
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = serve_connection(state, conn).await {
                        tracing::warn!(error = %e, "connection ended with an error");
                    }
                });
            }
            _ = &mut idle => break StopReason::Idle,
            _ = &mut stop => break StopReason::Requested,
        }
    };

    // Stop accepting first (we are out of the loop), then unwind (G5 steps 1, 3, 4). The listener
    // drops as this function returns.
    unwind(&state, reason).await;
    Ok(())
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

/// Run the daemon: resolve the endpoint, acquire it as the single instance, then accept.
///
/// One bind path, deliberately. Feature 028 removed the socket-activation branch that used to come
/// first (lifecycle contract §1.3): the application is the only thing that starts a service, so a
/// listener handed to us on fd 3 by a service manager has no defined behaviour and no way in.
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
    // A recovered (corrupt) catalog is surfaced, not swallowed (data-model C4). At `warn`, because
    // that is the floor of the diagnostics ring a client's "recent issues" request reads: at `info`
    // the only record of a reset project list was a line nobody is shown (002 BUG-007). A launch
    // normally meets the damaged file first and tells the user itself; this is the daemon starting
    // without one in front of it — a restart while the app stays open, or a kept-running sandbox
    // coming back after a reboot.
    if catalog.load_status() == micold_core::store::LoadStatus::Recovered {
        match catalog.recovered_backup() {
            Some(kept) => tracing::warn!(
                kept_as = %kept.display(),
                "the saved project list could not be read; starting with an empty one"
            ),
            None => tracing::warn!(
                "the saved project list could not be read and could not be moved aside; \
                 starting with an empty one"
            ),
        }
    } else {
        tracing::info!(load_status = ?catalog.load_status(), "catalog adopted");
    }
    let state = Arc::new(DaemonState::new(catalog));
    // Hand the diagnostics handle to the shared state so the `LogLocation`/`RecentErrors`/
    // `SetLogLevel` RPCs can serve it (FR-043–046).
    state.set_diagnostics(logging);

    // Feature 041: where terminal histories are saved. In a container the directory is the
    // launcher's to make, so a sandbox created before this feature saves nothing rather than
    // writing into the state mount (R15).
    if let Some(dir) = micold_core::terminal_history::history_dir() {
        let in_container = !crate::state::image_reference().is_empty();
        state.set_history_store(micold_core::terminal_history::HistoryStore::new(
            dir,
            !in_container,
        ));
        // Whatever belongs to no session that can still be shown is removed before any session
        // starts (FR-024).
        let sweeping = Arc::clone(&state);
        let _ = tokio::task::spawn_blocking(move || sweeping.sweep_saved_histories()).await;
    }

    // Feature 487: images pasted into sessions are removed with their session. Whatever a crash
    // left behind goes now, before any session starts (FR-014).
    if let Some(dir) = micold_core::paths::data_dir() {
        // Feature 806: what provisioning installed under the data directory is what is offered.
        state.set_ai_clis_dir(dir.join("ai-clis"));
        state.set_pasted_data_dir(dir);
    }
    let sweeping = Arc::clone(&state);
    let _ = tokio::task::spawn_blocking(move || sweeping.sweep_pasted_images()).await;

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
    spawn_history_saver(Arc::clone(&state));
    spawn_plan_usage_feed(Arc::clone(&state));
    spawn_mode_reports(Arc::clone(&state));
    spawn_model_answers(Arc::clone(&state));
    spawn_conversation_reports(Arc::clone(&state));
    spawn_input_driver(Arc::clone(&state));

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

    // The tool server (feature 034): a loopback MCP endpoint every bound AI session reaches with its
    // own credential. A bind failure is non-fatal — sessions start unbound, each logging why (FR-005).
    match crate::mcp::server::ToolServer::bind(crate::mcp::server::default_binding_dir()).await {
        Ok((tool_server, listener)) => {
            state.set_tool_server(tool_server);
            tokio::spawn(crate::mcp::server::serve(listener, Arc::clone(&state)));
            tracing::info!("tool server listening on loopback");
        }
        Err(e) => {
            tracing::warn!(error = %e, "could not bind the tool server; sessions start unbound");
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
            // On Windows the record is the only stop handle there is, so a failed write is an error:
            // "Restart service" cannot work without it (feature 030, R4).
            if let Err(e) = std::fs::write(&endpoint.lock_path, format!("{}\n", std::process::id()))
            {
                tracing::error!(error = %e, path = %endpoint.lock_path.display(), "could not record daemon pid");
            }
            crate::runtime_status::spawn(Arc::clone(&state), endpoint.clone());
            serve_interprocess(state, *bound).await
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

/// Spawn the loop that saves the history of running terminals (feature 041, FR-003): every
/// [`SAVER_TICK`], those with new output and a last save 30 s ago. The saving blocks on the disk
/// and on session gates, so it runs on the blocking pool, and the next tick waits for it.
fn spawn_history_saver(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(micold_core::terminal_history::schedule::SAVER_TICK);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let worker = Arc::clone(&state);
            let _ = tokio::task::spawn_blocking(move || {
                worker.save_due_at(std::time::Instant::now());
            })
            .await;
        }
    });
}

/// Spawn the task that hands structured sessions' plan-limit readings to the service (feature
/// 944, research R4). Once per service: a second call finds the feed taken and does nothing.
pub fn spawn_plan_usage_feed(state: Arc<DaemonState>) {
    if let Some(feed) = state.plan_usage_feed().take_receiver() {
        tokio::spawn(crate::plan_usage::drain(state, feed));
    }
}

/// `OperationOk` for `req`, or `OperationError` with the refusal's sentence.
fn ack_or_refused(req: u64, outcome: Result<(), String>) -> DaemonMsg {
    match outcome {
        Ok(()) => DaemonMsg::OperationOk {
            req,
            result: OperationResult::Ack,
        },
        Err(message) => DaemonMsg::OperationError {
            req,
            kind: ErrorKind::Refused,
            message,
            detail: None,
        },
    }
}

/// Spawn the task that applies the permission modes structured sessions' CLIs report (#942
/// FR-016). Once per service: a second call finds the feed taken and does nothing.
pub fn spawn_mode_reports(state: Arc<DaemonState>) {
    if let Some(mut reports) = state.mode_reports().take_receiver() {
        tokio::spawn(async move {
            while let Some((session, mode)) = reports.recv().await {
                state.follow_reported_mode(session, mode);
            }
        });
    }
}

/// Spawn the task that follows the CLIs' answers to deferred model changes (#943 FR-016). Once per
/// service: a second call finds the feed taken and does nothing.
pub fn spawn_model_answers(state: Arc<DaemonState>) {
    if let Some(mut answers) = state.model_answers().take_receiver() {
        tokio::spawn(async move {
            while let Some((session, answer)) = answers.recv().await {
                state.follow_model_answer(session, answer);
            }
        });
    }
}

/// Spawn the task that keeps the conversations structured sessions' CLIs report (#943 FR-005a).
/// Once per service: a second call finds the feed taken and does nothing.
pub fn spawn_conversation_reports(state: Arc<DaemonState>) {
    if let Some(mut reports) = state.conversation_reports().take_receiver() {
        tokio::spawn(async move {
            while let Some((session, handle)) = reports.recv().await {
                state.follow_reported_conversation(session, handle);
            }
        });
    }
}

/// Spawn the input driver (#941 research R2): the task that, after a structured session's turn
/// ends, delivers the next queued message and tells every window. Once per service: a second call
/// finds the feed taken and does nothing.
pub fn spawn_input_driver(state: Arc<DaemonState>) {
    if let Some(feed) = state.input_feed().take_receiver() {
        tokio::spawn(crate::input_driver::drive(state, feed));
    }
}

/// Spawn the restart-supervision loop (US4, FR-005). Ticks on `SUPERVISION_INTERVAL`, drives the
/// crash-loop policy for any session whose child exited, and broadcasts `CatalogChanged` when a
/// lifecycle moved. The supervision itself is blocking (PTY spawn / process teardown), so it runs on
/// a blocking thread, never on the async runtime (module invariant).
pub fn spawn_supervisor(state: Arc<DaemonState>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SUPERVISION_INTERVAL);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            supervisor_tick(&state).await;
        }
    });
}

/// One tick of the supervisor (US4, FR-005): supervise exited sessions, drain terminal signals,
/// broadcast what changed, and write what the tick owes the disk. Public so a test can run one
/// tick and observe what it wrote (feature 039, BUG-567).
pub async fn supervisor_tick(state: &Arc<DaemonState>) {
    let worker = Arc::clone(state);
    // Same blocking hop: the names of running sessions that gave a reason to look again
    // (feature 029, FR-011). The flag bounds it — an idle tick reads nothing.
    let (changed, named) = tokio::task::spawn_blocking(move || {
        (
            worker.supervise_exited_sessions(),
            worker.recover_live_session_names(),
        )
    })
    .await
    .unwrap_or_default();
    // Drain out-of-band terminal signals (title + spinner-derived activity, US2 T046/T047)
    // on the same cadence. It is lock-only (no blocking I/O), so it runs on the async task.
    let crate::state::DrainedSignals {
        changed: signals_changed,
        names,
    } = state.drain_signals();
    if !changed.is_empty() || named > 0 || signals_changed {
        state.broadcast_catalog();
    }
    // The screen first, the record second — then persist the names that changed (feature
    // 029, FR-003). Writing one means rewriting a project's state file, which is blocking
    // I/O and belongs on a blocking thread, not on this 250 ms tick (contract C11); the
    // drain's own debounce is what keeps this to once per re-title rather than once per
    // tick, so the hop is rare. Awaited rather than detached so a slow disk cannot stack
    // up writes behind a tick that keeps firing.
    if !names.is_empty() {
        let writer = Arc::clone(state);
        let _ = tokio::task::spawn_blocking(move || writer.record_observed_names(&names)).await;
    }
    // Attention events counted under the lock (feature 039, FR-008a) are written here for
    // the same reason: blocking I/O, off the async runtime.
    write_unsaved_attention(state).await;
}

/// The supervisor tick's write of attention events counted since the last write (feature 039,
/// FR-008a): one blocking hop, and none when nothing is unsaved.
pub async fn write_unsaved_attention(state: &Arc<DaemonState>) {
    if state.has_unsaved_attention() {
        let writer = Arc::clone(state);
        let _ = tokio::task::spawn_blocking(move || writer.persist_attention()).await;
    }
}

/// Accept loop over the single-instance interprocess listener, ending when the idle window expires.
///
/// `bound` is taken **by value** so that dropping it is the last thing that happens here: it owns
/// the socket file and the `flock`, and that lock is the liveness beacon `singleton::acquire` tests
/// (data-model G5 step 5). Releasing it before the sessions are down would let a client that starts
/// during the unwind bind the endpoint while this daemon still holds process trees — two daemons,
/// one endpoint, which is the failure the singleton exists to prevent.
async fn serve_interprocess(
    state: Arc<DaemonState>,
    bound: singleton::BoundListener,
) -> io::Result<()> {
    use interprocess::local_socket::traits::tokio::Listener as _;
    let idle = idle_watch(Arc::clone(&state));
    tokio::pin!(idle);
    // Created once, outside the loop: a signal that lands between two turns of it is not lost.
    let stop = crate::platform::stop_requested();
    tokio::pin!(stop);
    let reason = loop {
        tokio::select! {
            accepted = bound.listener.accept() => {
                let conn = accepted?;
                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    if let Err(e) = serve_connection(state, conn).await {
                        tracing::warn!(error = %e, "connection ended with an error");
                    }
                });
            }
            _ = &mut idle => break StopReason::Idle,
            _ = &mut stop => break StopReason::Requested,
        }
    };

    // Out of the loop: nothing is being accepted any more (G5 step 2). Steps 1, 3 and 4 follow;
    // step 5 is `bound` dropping as this returns, and step 6 is the return itself.
    unwind(&state, reason).await;
    Ok(())
}

/// How often the idle rule is evaluated, at most.
///
/// A ticker rather than one 30-minute sleep, and this is the reason: a timer armed for half an hour
/// across a laptop suspend fires whenever the runtime decides it should, and the rule is written
/// against a suspend-inclusive clock precisely so that suspended time counts (research R3). Waking
/// every 30 s and asking the question again makes the answer prompt after a resume and bounds the
/// overshoot at one tick, which is what SC-004 measures.
///
/// The cap is a maximum, not the interval: a shortened window (tests, [`idle::IDLE_STOP_ENV`]) ticks
/// on a fraction of itself instead, so a 300 ms window is still evaluated several times.
const IDLE_TICK_CAP: std::time::Duration = std::time::Duration::from_secs(30);

/// The idle watch: a future that completes once the window has expired, and never otherwise.
///
/// A future the accept loop `select!`s on rather than a task that signals one, because what has to
/// be atomic is *stopping accepting*: the loop leaves at the moment the rule fires, with no instant
/// in which a client could attach to a daemon that has already decided to go. Nothing is spawned and
/// nothing is signalled, so there is no channel to leak and no "the watcher died" case to reason
/// about — the rule is evaluated by the same task that would otherwise be accepting.
///
/// A `None` window (`MICOLD_IDLE_STOP=off`, the survive-logout sandbox) parks forever on
/// [`std::future::pending`], which is precisely "this daemon has no idle rule": the `select!` arm
/// exists and can never be taken.
fn idle_watch(state: Arc<DaemonState>) -> impl std::future::Future<Output = ()> + Send {
    let configured = idle::configured_window();
    async move {
        let Some(window) = configured else {
            tracing::info!("the idle stop is disabled; this daemon runs until it is asked to stop");
            return std::future::pending().await;
        };
        tracing::info!(window = ?window.window(), "idle stop armed");
        // A quarter of the window, capped at [`IDLE_TICK_CAP`]: 30 s for the real rule (which is
        // what bounds the overshoot at one tick), and a fraction of a shortened one so tests still
        // get several evaluations. The floor only ever applies to a window small enough that a
        // quarter of it would round to nothing.
        let interval = (window.window() / 4)
            .min(IDLE_TICK_CAP)
            .max(IDLE_TICK_FLOOR);
        let mut ticker = tokio::time::interval(interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if window.expired(&state.presence(), micold_core::clock::now()) {
                return;
            }
        }
    }
}

/// The shortest window the ticker will divide down from, so a misconfigured value cannot produce a
/// zero-length interval (which `tokio::time::interval` panics on).
const IDLE_TICK_FLOOR: std::time::Duration = std::time::Duration::from_millis(40);

/// The ordered unwind (data-model G5 steps 1, 3 and 4; lifecycle contract §3.11).
///
/// The order is the contract, and the reason is what an *interrupted* stop leaves behind. Marking
/// the sessions first means a daemon killed halfway through this leaves records that are true of a
/// daemon that is gone; killing first would leave the daemon briefly describing sessions as
/// `Running` with nothing behind them.
///
/// Dropping the session table is the teardown — `PtySession::Drop` terminates each process tree —
/// so it is blocking, and runs on a blocking thread rather than on the runtime.
pub async fn unwind(state: &Arc<DaemonState>, reason: StopReason) {
    // Step 1: say why, before doing anything. This line is the only record of *which* way out this
    // was; absent it, an idle stop and a crash look identical in the log (data-model G4).
    //
    // The reason is spelled out in words as well as carried in the field, because of who reads it:
    // someone who left work running, came back to a machine where the service is gone, and opened
    // the log to find out what happened (FR-024, contract §6.19). `reason=Idle` answers that only
    // for a reader who knows this enum.
    let why = match reason {
        StopReason::Idle => "stopping: nothing has been connected for the whole idle window",
        StopReason::Requested => "stopping: something asked this service to stop",
    };
    tracing::info!(reason = ?reason, "{why}");

    // 002 BUG-007: the last chance to put a list that only this process still holds back on disk
    // — a launch moved a damaged `projects.json` aside and never connected.
    state.restore_missing_catalog_file();

    // Feature 041, FR-002: every covered running terminal is saved first, while its `Term` is still
    // there and before the endpoint is released, so a service started after this one loads
    // whole files. Bounded, so a stuck disk cannot keep the service from stopping.
    crate::history::save_all_live(state, crate::history::STOP_SAVE_BOUND).await;

    let worker = Arc::clone(state);
    let (marked, dropped) = tokio::task::spawn_blocking(move || {
        // Feature 039, FR-008a: an `unread` set or cleared since the last supervisor tick is written
        // before the stop, so a session read just before it is not unread again after a restart.
        worker.persist_attention();
        let marked = worker.mark_live_sessions_interrupted(); // step 3
        let dropped = worker.take_live_sessions(); // step 4
        (marked, dropped)
    })
    .await
    .unwrap_or((0, 0));

    tracing::info!(
        sessions_marked = marked,
        sessions_stopped = dropped,
        "sessions handed off as interrupted-resumable"
    );
}

/// Serve one connection: handshake, register, then route messages until the client hangs up.
///
/// Generic over the stream so the interprocess path, the loopback TCP path and tests share one
/// implementation.
pub async fn serve_connection<S>(state: Arc<DaemonState>, stream: S) -> io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut framed = Framed::new(stream, DaemonCodec::new());

    // A launch that met a damaged `projects.json` moved it aside before dialling; the list this
    // daemon still holds goes back on disk (002 BUG-007). Before the handshake, not at `Welcome`:
    // a client of another build is refused below and then replaces this daemon, and one that dies
    // before its `Hello` never gets that far — either way this memory is the list's only copy.
    state.restore_missing_catalog_file();

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
    let plan_usage = state.welcome_plan_usage();
    framed
        .send(Frame::Control(DaemonMsg::Welcome {
            daemon_build: daemon_build(),
            catalog,
            settings,
            plan_usage,
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
    //
    // Unconditional, and that is what makes the presence count honest (feature 028, T033, lifecycle
    // contract §2.6). `route` has exactly three ways out — `Goodbye`, EOF, and a codec error — and
    // the last two are what a crashed or `SIGKILL`ed client looks like from here. Because this runs
    // after all three rather than on the clean one, a client that vanishes is counted gone as soon
    // as its socket closes, with no keepalive to wait for (research R6).
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
    // Which session this client asked to view, whether or not it is live yet (BUG-009, T125).
    // `view_stream` alone cannot answer that: a client may ask to view a session whose start is
    // still running, and the stream can only be built once the session exists.
    let mut viewing: Option<micold_core::session::SessionId> = None;
    // Every finished session start, whoever asked for it (this window, another, or an agent's
    // `create_session`). The loop owns `view_stream` and is the only thing that may touch it, so a
    // start that finishes elsewhere reports here rather than reaching in (BUG-009, T125).
    let mut started_rx = state.subscribe_session_started();
    // Feature 484: the terminals this client asked for with `SetViewedTerminals`, and the running
    // stream of each live one. Independent of `view_stream`, which serves `SetViewedSession`.
    let mut viewed_terminals: Vec<TerminalRef> = Vec::new();
    let mut terminal_streams: TerminalStreams = HashMap::new();

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
            started = started_rx.recv() => {
                // A session start has concluded. If the client is waiting to view that session,
                // this is the moment its stream can exist. Missed announcements (the receiver
                // lagged) may include it, so a viewed session is rechecked then.
                let concerns_view = match started {
                    Ok(session) => viewing == Some(session),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => viewing.is_some(),
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => false,
                };
                if concerns_view && viewing.is_some() {
                    show_single_view(
                        state,
                        id,
                        &mut view_stream,
                        &mut terminal_streams,
                        &mut viewed_terminals,
                        viewing,
                    );
                } else if viewed_terminals.iter().any(|t| matches!(started, Ok(s) if s == t.session) || started.is_err()) {
                    sync_terminal_streams(state, id, &mut terminal_streams, &viewed_terminals);
                }
                continue;
            }
        };

        match msg {
            ClientMsg::Ping { nonce } => state.send(id, DaemonMsg::Pong { nonce }),
            // The first answer to an agent's destructive request decides (feature 034, FR-014).
            ClientMsg::ConfirmationAnswer { id: prompt, allow } => {
                state.answer_confirmation(prompt, allow)
            }
            // The first decision on a structured session's permission request wins (#942 FR-010);
            // any other is refused with the reason (FR-012). Its message, note and answers are
            // never logged.
            ClientMsg::PermissionDecision {
                req,
                session,
                id: prompt,
                choice,
            } => {
                match state.decide_prompt(session, prompt, &choice) {
                    // An approved plan sets the mode first, which waits on the CLI (R7): off the
                    // connection loop.
                    Ok(decided) if decided.plan.is_some() => {
                        let state = Arc::clone(state);
                        tokio::spawn(async move {
                            let set = state.finish_plan_approval(session, decided).await;
                            state.send(id, ack_or_refused(req, set));
                        });
                    }
                    Ok(_) => state.send(id, ack_or_refused(req, Ok(()))),
                    Err(refusal) => state.send(id, ack_or_refused(req, Err(refusal.to_string()))),
                }
            }
            // A running session's CLI is asked and may take up to 10 s to answer (FR-017): off
            // the connection loop.
            ClientMsg::SetPermissionMode { req, session, mode } => {
                let state = Arc::clone(state);
                tokio::spawn(async move {
                    let set = state.set_permission_mode(session, mode).await;
                    state.send(id, ack_or_refused(req, set));
                });
            }
            // An idle running session's CLI is asked here, so picks reach it in the order sent; its
            // answer (up to 10 s) is waited for off the connection loop (#943 FR-015).
            ClientMsg::SetSessionModel {
                req,
                session,
                model,
            } => match state.pick_model(session, model) {
                crate::state::ModelPick::Answered(result) => {
                    state.send(id, ack_or_refused(req, result))
                }
                crate::state::ModelPick::Asked(asked) => {
                    let state = Arc::clone(state);
                    tokio::spawn(async move {
                        let picked = state.finish_model_pick(asked).await;
                        state.send(id, ack_or_refused(req, picked));
                    });
                }
            },
            ClientMsg::PermissionInput {
                req,
                session,
                id: prompt,
            } => {
                let answer = match state.prompt_input(session, prompt) {
                    Ok(text) => DaemonMsg::PermissionInputText { req, text },
                    Err(refusal) => DaemonMsg::OperationError {
                        req,
                        kind: ErrorKind::Refused,
                        message: refusal.to_string(),
                        detail: None,
                    },
                };
                state.send(id, answer)
            }
            ClientMsg::Goodbye => break,
            // Every session reads it when a program asks for its colours (`006` FR-003a, BUG-007).
            ClientMsg::TerminalColorScheme { scheme } => state.terminal_colors().set(scheme),
            // Not an operation: no `req`, no reply, and no attachment is needed (feature 039,
            // W1.5, W1.6).
            ClientMsg::WindowView { focused, in_view } => {
                state.set_window_view(id, WindowView { focused, in_view })
            }
            // Not an operation either; it needs no attachment (W1.4–W1.6).
            ClientMsg::AttentionClaim { session, seq } => state.claim_attention(id, session, seq),
            // Forwarded to one window, unchecked; no reply and no attachment needed (W3).
            ClientMsg::SessionReveal {
                project,
                session,
                activation,
            } => state.reveal_session(id, project, session, activation),
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
                        // Feature 483: the project's run groups, once, right after `Attached`.
                        // The first read of the file may clean up after an interrupted create (git, disk),
                        // so it runs off the connection loop.
                        let groups = {
                            let (state, project) = (Arc::clone(state), project.clone());
                            tokio::task::spawn_blocking(move || {
                                state.run_groups_on_attach(&project)
                            })
                            .await
                        };
                        match groups {
                            Ok(groups) => state.send(id, groups),
                            Err(err) => {
                                tracing::warn!(%err, "the run groups were not read on attach");
                            }
                        }
                        // Feature 482: the project's stored comments, one push per entry.
                        for msg in state.review_pushes_on_attach(&project) {
                            state.send(id, msg);
                        }
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
                        announce_git_targets(state, Some(id)).await;
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
            // --- AI CLIs (feature 027, FR-023c; feature 029 BUG-001, FR-003b) ---
            //
            // Answered *here*, in the service, because here is where sessions run. Under sandboxed
            // placement this process is inside the container and reads the image's environment —
            // the question FR-023c asks and the one the client cannot ask for itself. Under host
            // placement it reads the host's, and the same code path gives the right answer for the
            // same reason.
            //
            // *Which* environment: the one a session in `cwd` would be spawned with (FR-003b), not
            // this process's own. A service started from the desktop has the login `PATH`, which
            // lacks a version manager's directories; its sessions get them from the
            // environment-include script. `None` (Settings) is answered for the home directory.
            //
            // Recomputed per request rather than cached at boot: the client only asks when a choice
            // is offered, and research R11's rule is that this answer is never stored. What *is*
            // cached is the resolved environment, in the per-directory cache spawns already use.
            // The answer carries that environment's state too (037, contract A2), read from the
            // same cache entry, so the reason for a missing CLI costs no run of the script.
            //
            // Resolving it can run the script for up to its timeout, so it runs on the blocking
            // pool and the whole answer is *spawned*, never awaited here: this loop is where the
            // client's `Ping` is answered (BUG-009, above), and a slow script must not silence it.
            ClientMsg::AiCliAvailabilityRequest { req, cwd } => {
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let resolver = Arc::clone(&task_state);
                    let (available, env) = tokio::task::spawn_blocking(move || {
                        let home =
                            || directories::UserDirs::new().map(|d| d.home_dir().to_path_buf());
                        resolver.availability_for(cwd.or_else(home).as_deref())
                    })
                    .await
                    .unwrap_or_else(|_| task_state.availability_for(None));
                    tracing::debug!(
                        client = id,
                        ?available,
                        ?env,
                        "AI CLI availability reported"
                    );
                    task_state.send(
                        id,
                        DaemonMsg::AiCliAvailability {
                            req,
                            available,
                            env,
                        },
                    );
                });
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
                process,
            } => {
                if !refuse_terminal_request(state, id, session, process, RefusedRequest::Input) {
                    state.session_input_to(session, serial, &bytes, process);
                }
            }
            ClientMsg::SessionStart {
                session,
                allow_additional_session,
            } => {
                // Bringing an existing durable session back is a resume.
                //
                // Spawned, not run here (BUG-009 T125, FR-026a): `start_session` resolves the
                // user's environment-include script, which is a subprocess with a timeout the user
                // can set to 60 s, and it used to run *directly on this loop* — so a version
                // manager waiting on the network silenced the connection well past the client's 9 s
                // liveness deadline. `begin_start` holds any input typed meanwhile so nothing is
                // lost to the gap it opens (protocol.md §7).
                let additional = allow_additional_session.into();
                if !refuse_blocked_start(state, session, additional) {
                    state.begin_start(session);
                    spawn_session_start(state, session, LaunchMode::Resume, additional, None);
                }
            }
            ClientMsg::SessionSetMain { id } => {
                // Fire-and-forget (FR-021): the catalog update is the answer. A no-op for an
                // unknown, archived or already-main id.
                state.set_main(id);
            }
            ClientMsg::SessionRestart {
                session,
                allow_additional_session,
            } => {
                // The user's manual restart of the AI CLI (011 FR-007(b), BUG-442): exactly the
                // start above, after dropping the session's directory from the environment-include
                // cache so the relaunch re-sources the script. A plain `SessionStart` stays cached.
                let additional = allow_additional_session.into();
                if !refuse_blocked_start(state, session, additional) {
                    state.forget_session_env(session);
                    state.begin_start(session);
                    spawn_session_start(state, session, LaunchMode::Resume, additional, None);
                }
            }
            // Feature 938 (FR-008), 941 (R2): the message is delivered or queued here, in arrival
            // order, so of two messages the first is delivered and the second queued (FR-032);
            // the write to the CLI's stdin may block on a full pipe, so it runs off the connection
            // loop. The text and pictures are never logged.
            ClientMsg::SessionUserMessage {
                req,
                session,
                text,
                images,
            } => {
                let submitted = micold_core::structured::attachment::check_images(&images)
                    .map_err(|why| why.to_string())
                    .and_then(|()| {
                        let message = micold_core::structured::input::Message { text, images };
                        state
                            .submit_user_message(id, session, message)
                            .map_err(|why| why.to_string())
                    });
                match submitted {
                    Err(why) => state.send(id, ack_or_refused(req, Err(why))),
                    Ok(None) => state.send(id, ack_or_refused(req, Ok(()))),
                    Ok(Some(delivery)) => {
                        let state = Arc::clone(state);
                        tokio::task::spawn_blocking(move || {
                            let answer = match state.deliver(delivery) {
                                Ok(()) => ack_or_refused(req, Ok(())),
                                Err(err) => DaemonMsg::OperationError {
                                    req,
                                    kind: ErrorKind::IoFailed,
                                    message:
                                        crate::structured::UserMessageUndelivered::WriteFailed(err)
                                            .to_string(),
                                    detail: None,
                                },
                            };
                            state.send(id, answer);
                        });
                    }
                }
            }
            // Feature 941 (FR-020, FR-022): queue edits name their entry; each is answered, and
            // every window is told the queue.
            ClientMsg::QueueEdit {
                req,
                session,
                entry,
                text,
                keep,
            } => {
                let changed = state.edit_queue(session, entry, text, &keep);
                answer_queue_change(
                    state,
                    id,
                    req,
                    changed.map(|((), d)| (OperationResult::Ack, d)),
                );
            }
            ClientMsg::QueueRemove {
                req,
                session,
                entry,
            } => {
                let changed = state.change_queue(session, |input| input.remove(entry));
                answer_queue_change(
                    state,
                    id,
                    req,
                    changed.map(|((), d)| (OperationResult::Ack, d)),
                );
            }
            ClientMsg::QueueTake {
                req,
                session,
                entry,
            } => {
                let changed = state.change_queue(session, |input| input.take(entry));
                let changed = changed.map(|(taken, d)| {
                    let result = OperationResult::Taken {
                        text: taken.text,
                        images: taken.images,
                    };
                    (result, d)
                });
                answer_queue_change(state, id, req, changed);
            }
            ClientMsg::QueueRelease { req, session } => {
                let delivery = state.release_queue(session);
                answer_queue_change(state, id, req, Ok((OperationResult::Ack, delivery)));
            }
            ClientMsg::TurnInterrupt { session, turn } => {
                // Asked in arrival order; written and timed off this connection's task.
                if let Some(pending) = state.ask_interrupt(session, turn) {
                    tokio::spawn(crate::input_driver::interrupt(Arc::clone(state), pending));
                }
            }
            ClientMsg::ScrollbackRequest {
                session,
                req,
                ranges,
            } => {
                // Advisory, never an error (protocol.md §6): serve whatever the session's shared
                // framer can from its retained history, resolved against a per-response palette.
                // A structured session's AI process has no terminal: refused (feature 938).
                if !refuse_terminal_request(state, id, session, None, RefusedRequest::Scrollback) {
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
            }
            ClientMsg::SessionCreate {
                req,
                project,
                worktree_dir,
                provider,
                allow_additional_session,
                kind,
                permission_mode,
            } => {
                // The catalog write stays on the loop: it is a small atomic file write, and it is
                // what mints the id every later message refers to. Only the spawn — the slow half
                // — is deferred (BUG-009, T125).
                match state.create_session_in_mode(
                    &project,
                    &worktree_dir,
                    provider,
                    kind,
                    permission_mode,
                    allow_additional_session.into(),
                ) {
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
                            // The creation's own check has passed or was waived (FR-002).
                            micold_core::occupancy::Additional::Yes,
                            Some((id, req)),
                        );
                    }
                    // Feature 776: a location that has a session answers with that session's
                    // name and id, and the catalog is unchanged.
                    Err(crate::catalog::CreateSessionError::Occupied(existing, location)) => state
                        .send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::Refused,
                                message: micold_core::occupancy::refusal_text(&existing, &location),
                                detail: Some(existing.id.0.to_string()),
                            },
                        ),
                    // Feature 938, FR-003: a kind the CLI cannot run is refused, nothing written;
                    // #942, FR-018: so is a terminal session with a permission mode.
                    Err(
                        refusal @ (crate::catalog::CreateSessionError::Kind(_)
                        | crate::catalog::CreateSessionError::Mode(_)),
                    ) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::Refused,
                            message: refusal.to_string(),
                            detail: None,
                        },
                    ),
                    Err(crate::catalog::CreateSessionError::Io(e)) => state.send(
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
                // The single view replaces a pane set (feature 484). Not live *yet* is the ordinary
                // case: the client sends `SessionStart` and `SetViewedSession` back to back, and the
                // start no longer completes before this arrives (BUG-009, T125). `viewing` records
                // the intent, and the session-started arm builds the stream when the session exists.
                show_single_view(
                    state,
                    id,
                    &mut view_stream,
                    &mut terminal_streams,
                    &mut viewed_terminals,
                    session,
                );
            }
            // --- Feature 011: shell instances + which process is attached ---
            ClientMsg::SessionAttachProcess { session, process } => {
                let terminal = state.attach_process(session, process);
                // `None` is also a structured primary's answer, which has no terminal to give.
                if terminal.is_none() && state.attached_process(session) != Some(process) {
                    // The client asked to display a process the daemon does not have, so the two
                    // now disagree about what is attached — the client will show its new mode while
                    // the pane keeps streaming whatever it streamed before (FR-007). Silently
                    // ignoring this is what let a Regular Terminal toggle do nothing at all with no
                    // trace anywhere (BUG-001, feature 010-regular-terminal-mode).
                    tracing::warn!(
                        session = %session.0,
                        ?process,
                        "attach requested for a process the session does not have"
                    );
                } else if viewing == Some(session) {
                    // A structured primary has no terminal: its events stream instead (BUG-1256).
                    show_single_view(
                        state,
                        id,
                        &mut view_stream,
                        &mut terminal_streams,
                        &mut viewed_terminals,
                        Some(session),
                    );
                } else if let Some((pty, framer)) = terminal {
                    restart_view(state, id, &mut view_stream, process, pty, framer);
                }
            }
            ClientMsg::SessionOpenShell { session, instance } => {
                match state.open_shell(session, instance) {
                    // The request is fire-and-forget, and the client has already switched to the
                    // instance and asked to attach it. Without this reply the refusal reached only
                    // the log and the Terminal toggle did nothing on screen (FR-006c, BUG-592).
                    Err(err) => {
                        tracing::warn!(session = %session.0, instance = instance.0, %err, "open shell failed");
                        state.send(
                            id,
                            DaemonMsg::ShellOpenFailed {
                                session,
                                instance,
                                reason: crate::supervisor::shell_open_failure(&err),
                            },
                        );
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
                let was_attached =
                    state.attached_process(session) == Some(SessionProcess::Shell(instance));
                let reattached = state.close_shell(session, instance);
                if was_attached && viewing == Some(session) {
                    // Back on the primary, which for a structured session is its events (BUG-1256).
                    show_single_view(
                        state,
                        id,
                        &mut view_stream,
                        &mut terminal_streams,
                        &mut viewed_terminals,
                        Some(session),
                    );
                } else if let Some((pty, framer)) = reattached {
                    restart_view(
                        state,
                        id,
                        &mut view_stream,
                        SessionProcess::Primary,
                        pty,
                        framer,
                    );
                }
                // Closed or not (the id may name nothing), the live set may have changed.
                state.broadcast_catalog();
            }
            ClientMsg::SessionRestartShell { session, instance } => {
                // A manual restart re-sources the session's directory (011 FR-007(b), BUG-442);
                // `SessionOpenShell` stays cached.
                state.forget_session_env(session);
                // Asked before `close_shell`, whose answer is `None` for a structured session's
                // primary even when it reattached to it (BUG-1256).
                let was_attached =
                    state.attached_process(session) == Some(SessionProcess::Shell(instance));
                let reattached_primary = state.close_shell(session, instance);
                match state.open_shell(session, instance) {
                    Ok(()) if was_attached => {
                        // It was attached: re-attach the fresh instance so view + input follow it.
                        let terminal =
                            state.attach_process(session, SessionProcess::Shell(instance));
                        if viewing == Some(session) {
                            show_single_view(
                                state,
                                id,
                                &mut view_stream,
                                &mut terminal_streams,
                                &mut viewed_terminals,
                                Some(session),
                            );
                        } else if let Some((pty, framer)) = terminal {
                            restart_view(
                                state,
                                id,
                                &mut view_stream,
                                SessionProcess::Shell(instance),
                                pty,
                                framer,
                            );
                        }
                    }
                    Ok(()) => {}
                    Err(err) => {
                        tracing::warn!(session = %session.0, instance = instance.0, %err, "restart shell failed");
                        // Tell the client, which still holds the instance (BUG-592).
                        state.send(
                            id,
                            DaemonMsg::ShellOpenFailed {
                                session,
                                instance,
                                reason: crate::supervisor::shell_open_failure(&err),
                            },
                        );
                        // Respawn failed: fall back to the primary `close_shell` reattached to, so
                        // the view stream and input routing agree (both on Primary) instead of the
                        // view showing the dead shell while input goes to Primary.
                        if was_attached && viewing == Some(session) {
                            show_single_view(
                                state,
                                id,
                                &mut view_stream,
                                &mut terminal_streams,
                                &mut viewed_terminals,
                                Some(session),
                            );
                        } else if let Some((pty, framer)) = reattached_primary {
                            restart_view(
                                state,
                                id,
                                &mut view_stream,
                                SessionProcess::Primary,
                                pty,
                                framer,
                            );
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
                process,
            } => {
                // `None` resizes every process (`resize_terminal`), the session's shell instances
                // included, so it is not addressed to the AI process: only a `Some` is refused.
                if process.is_none()
                    || !refuse_terminal_request(state, id, session, process, RefusedRequest::Resize)
                {
                    state.resize_terminal(session, process, cols, rows);
                }
            }
            // Feature 484: exactly the named terminals stream to this client, replacing the last
            // set. The first is the project's foreground session; it is also the attached process
            // of its session, so an input without a `process` keeps landing where it always did.
            ClientMsg::SetViewedTerminals { project, terminals } => {
                let wanted: Vec<TerminalRef> = terminals.into_iter().take(MAX_VIEWED).collect();
                state.set_viewed(id, project, wanted.first().map(|t| t.session));
                if let Some(first) = wanted.first() {
                    let _ = state.attach_process(first.session, first.process);
                }
                // A set replaces the single view of `SetViewedSession`.
                if let Some(prev) = view_stream.take() {
                    prev.abort();
                }
                viewing = None;
                viewed_terminals = wanted;
                sync_terminal_streams(state, id, &mut terminal_streams, &viewed_terminals);
            }
            // Feature 484 (FR-013): the daemon is the store's single writer.
            ClientMsg::SetPaneLayout { project, layout } => {
                state.set_pane_layout(&project, layout.as_deref());
            }
            // Feature 034 (FR-009): the stop an agent's `stop_session` performs, so the two agree:
            // processes end, the record is `Idle`, and every window is told.
            //
            // The stop runs on a task of its own, under the session's gate, which it queues for
            // here, in order with the window's next message (a start that follows waits for it).
            // Not on this loop: ending the processes and carrying the history can wait up to
            // `TEARDOWN_WAIT` for a process that holds its terminal open (041, R4), and this loop
            // answers the window's `Ping`.
            ClientMsg::SessionStop { session } => {
                drop(ops::stop_session(state, session));
            }
            ClientMsg::SessionKill { session } => {
                // Stop the session's processes and drop it from the live registry (kill happens
                // outside the state lock inside remove_session). TODO(T053): archive the durable
                // record so reconciliation can't resurrect it, and broadcast the catalog.
                for pty in state.remove_session(session) {
                    let _ = pty.kill();
                }
            }
            ClientMsg::SessionInterrupt { session } => {
                // Ctrl-C to the attached process — 0x03 to the PTY, never a real signal (§7).
                if !refuse_terminal_request(state, id, session, None, RefusedRequest::Interrupt) {
                    if let Some(pty) = state.live_session(session) {
                        if let Err(err) = pty.write_input(&[0x03]) {
                            tracing::warn!(session = %session.0, %err, "interrupt write failed");
                        }
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
                default_prompt_template,
                pi_activity_component,
                save_terminal_history,
                tool_server_enabled,
                cross_session_access,
                pr_status_enabled,
                jira_enabled,
                jira_connection,
                github_issues_enabled,
                jira_base_url,
                desktop_notifications,
                notification_kinds,
                long_task_threshold_secs,
                diff_layout,
                plan_usage_enabled,
                plan_usage_warn_percent,
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
                })
                .and_then(|()| match default_prompt_template {
                    Some(text) => state.set_default_prompt_template(text),
                    None => Ok(()),
                })
                .and_then(|()| match pi_activity_component {
                    Some(on) => state.set_pi_activity_component(on),
                    None => Ok(()),
                })
                .and_then(|()| match save_terminal_history {
                    Some(on) => state.set_save_terminal_history(on),
                    None => Ok(()),
                })
                .and_then(|()| match tool_server_enabled {
                    Some(on) => state.set_tool_server_enabled(on),
                    None => Ok(()),
                })
                .and_then(|()| match cross_session_access {
                    Some(access) => state.set_cross_session_access(access),
                    None => Ok(()),
                })
                .and_then(|()| match pr_status_enabled {
                    Some(on) => state.set_pr_status_enabled(on),
                    None => Ok(()),
                })
                .and_then(|()| match jira_enabled {
                    Some(on) => state.set_jira_enabled(on),
                    None => Ok(()),
                })
                .and_then(|()| match &jira_connection {
                    Some(change) => state.set_jira_connection(change),
                    None => Ok(()),
                })
                .and_then(|()| match github_issues_enabled {
                    Some(on) => state.set_github_issues_enabled(on),
                    None => Ok(()),
                })
                .and_then(|()| match &jira_base_url {
                    Some(change) => state.set_jira_base_url(change),
                    None => Ok(()),
                })
                .and_then(|()| match desktop_notifications {
                    Some(on) => state.set_desktop_notifications(on),
                    None => Ok(()),
                })
                .and_then(|()| match notification_kinds {
                    Some(kinds) => state.set_notification_kinds(kinds),
                    None => Ok(()),
                })
                .and_then(|()| match long_task_threshold_secs {
                    Some(secs) => state.set_long_task_threshold_secs(secs),
                    None => Ok(()),
                })
                .and_then(|()| match diff_layout {
                    Some(layout) => state.set_diff_layout(layout),
                    None => Ok(()),
                })
                .and_then(|()| match plan_usage_enabled {
                    Some(on) => state.set_plan_usage_enabled(on),
                    None => Ok(()),
                })
                .and_then(|()| match plan_usage_warn_percent {
                    Some(percent) => state.set_plan_usage_warn_percent(percent),
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
            // Feature 482: comments are edited and stored under the state lock, pushed to every
            // window, then answered (W5: on disk before `OperationOk`).
            ClientMsg::ReviewEdit {
                req,
                project,
                worktree_dir,
                edit,
            } => match state.review_edit(&project, &worktree_dir, edit) {
                Ok(()) => state.send(
                    id,
                    DaemonMsg::OperationOk {
                        req,
                        result: micold_core::protocol::messages::OperationResult::Ack,
                    },
                ),
                Err(refusal) => state.send(
                    id,
                    DaemonMsg::OperationError {
                        req,
                        kind: refusal.kind,
                        message: refusal.message,
                        detail: None,
                    },
                ),
            },
            // Feature 482 (W6–W9): the send waits on a terminal, so it runs off the connection
            // loop and answers when the prompt was typed or refused.
            ClientMsg::ReviewSend {
                req,
                project,
                worktree_dir,
                outdated,
                pr,
            } => {
                let state = Arc::clone(state);
                tokio::spawn(async move {
                    let answer = match state
                        .review_send(&project, &worktree_dir, &outdated, pr.as_ref())
                        .await
                    {
                        Ok(result) => DaemonMsg::OperationOk { req, result },
                        Err(refusal) => DaemonMsg::OperationError {
                            req,
                            kind: refusal.kind,
                            message: refusal.message,
                            detail: None,
                        },
                    };
                    state.send(id, answer);
                });
            }
            // Feature 483 (W1): checked and recorded off the connection loop (the base branch's
            // tip is read from git), answered, pushed, and only then are the runs started.
            ClientMsg::RunGroupCreate {
                req,
                project,
                naming,
                prompt,
                base_branch,
                providers,
            } => {
                let state = Arc::clone(state);
                tokio::spawn(async move {
                    let request = crate::runs::CreateRequest {
                        project: project.clone(),
                        naming,
                        prompt,
                        base_branch,
                        providers,
                    };
                    match crate::runs::create(&state, request).await {
                        Ok(group) => {
                            state.send(
                                id,
                                DaemonMsg::OperationOk {
                                    req,
                                    result: OperationResult::RunGroupCreated { group: group.id },
                                },
                            );
                            state.broadcast_run_groups(&project);
                            crate::runs::spawn_runs(&state, &project, &group);
                        }
                        Err(refusal) => state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: refusal.kind,
                                message: refusal.message,
                                detail: None,
                            },
                        ),
                    }
                });
            }
            // Feature 483 (W4): the grouping goes; worktrees, branches and sessions stay.
            ClientMsg::RunGroupDismiss {
                req,
                project,
                group,
            } => match state.dismiss_run_group(&project, group) {
                Ok(()) => {
                    state.send(
                        id,
                        DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::Ack,
                        },
                    );
                    state.broadcast_run_groups(&project);
                }
                Err(refusal) => state.send(
                    id,
                    DaemonMsg::OperationError {
                        req,
                        kind: refusal.kind,
                        message: refusal.message,
                        detail: None,
                    },
                ),
            },
            // Feature 483 (W3): integrate the run's branch into the base branch.
            ClientMsg::RunGroupPick {
                req,
                project,
                group,
                run,
            } => {
                let state = Arc::clone(state);
                tokio::spawn(async move {
                    match crate::ops::pick_run(&state, project.clone(), group, run).await {
                        Ok(integration) => {
                            state.send(
                                id,
                                DaemonMsg::OperationOk {
                                    req,
                                    result: OperationResult::RunPicked {
                                        group,
                                        run,
                                        integration,
                                    },
                                },
                            );
                            state.broadcast_run_groups(&project);
                        }
                        Err(refusal) => state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: refusal.kind,
                                message: refusal.message,
                                detail: None,
                            },
                        ),
                    }
                });
            }
            // --- US3: worktree management through the daemon (T053) ---
            ClientMsg::WorktreeCreate {
                req,
                project,
                branch,
                dir_name,
                mode,
                session,
            } => {
                if !matches!(state.project_repo(&project), Some((_, true))) {
                    reject_non_repo(state, id, req, &project);
                    continue;
                }
                let names = DerivedNames {
                    dir_name: dir_name.clone(),
                    branch,
                };
                // Feature 016 FR-024: stream the stage as the create advances, so the form can
                // name the step being performed. The client owns the wording; only the stage
                // travels. Pushed through the client's own ordered frame channel — a clone of the
                // sender moves into the blocking task, so the git work never touches the state lock
                // and never blocks the runtime. A departed client simply drops the sends. A stage
                // transition always gets a frame; within a stage the live output is forwarded at a
                // fixed rate (BUG-009, T123 — the rule lives in `ProgressThrottle`).
                let progress_tx = state.frame_sender(id);
                let mut throttle = ProgressThrottle::new(PROGRESS_DETAIL_MIN_GAP);
                let progress: ops::ProgressSink = Box::new(move |event: CreateProgressEvent| {
                    let stage = event.stage;
                    let Some(detail) = throttle.admit(event, std::time::Instant::now()) else {
                        return;
                    };
                    if let Some(tx) = &progress_tx {
                        let _ = tx.send(Frame::Control(DaemonMsg::OperationProgress {
                            req,
                            stage,
                            detail,
                        }));
                    }
                });
                // BUG-009 (T120, FR-026a): the whole operation is *spawned*, not awaited here.
                // `spawn_blocking` frees the runtime; it does not free this loop, and this loop is
                // the only place this client's `Ping` is answered. Awaiting inline made a working
                // daemon silent for the length of a submodule fetch, and the client's 9 s liveness
                // deadline reaped it — see `bugs/BUG-009.md`. Every reply below goes through the
                // client's ordered frame channel, which a departed client drops harmlessly.
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let state = &task_state;
                    // Feature 776: a definition makes the worktree and its one session one
                    // operation, so the reply names both, or neither for a missing CLI.
                    let asked = tokio::time::Instant::now();
                    let created = match &session {
                        None => ops::create_worktree(state, project, names, mode, Some(progress))
                            .await
                            .map(|()| None)
                            .map_err(ops::WorktreeStartFailure::Create),
                        Some(definition) => ops::create_worktree_with_session(
                            state,
                            project,
                            names,
                            mode,
                            Some(progress),
                            ops::StartDefinition {
                                cli: definition.ai_cli,
                                prompt: definition.prompt.as_deref().map(|text| ops::FirstPrompt {
                                    text,
                                    asked,
                                    require_bracketed: false,
                                }),
                            },
                        )
                        .await
                        .map(|(session, _prompt)| Some(session)),
                    };
                    let reply = match created {
                        Ok(session) => DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::WorktreeCreated { dir_name, session },
                        },
                        Err(ops::WorktreeStartFailure::CliUnavailable(why)) => {
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::Refused,
                                message: why,
                                detail: None,
                            }
                        }
                        Err(ops::WorktreeStartFailure::Session { dir_name, error }) => {
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::IoFailed,
                                message: format!(
                                    "the worktree was created but its session was not: {error}"
                                ),
                                detail: Some(dir_name),
                            }
                        }
                        Err(ops::WorktreeStartFailure::Create(failure)) => match failure {
                            ops::CreateFailure::Create(e) => {
                                let (kind, message, detail) = describe_create_error(e);
                                DaemonMsg::OperationError {
                                    req,
                                    kind,
                                    message,
                                    detail,
                                }
                            }
                            ops::CreateFailure::NotARepository => DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::NotFound,
                                message: "unknown project".into(),
                                detail: None,
                            },
                            ops::CreateFailure::Task(join) => {
                                task_failed(req, "worktree create", &join)
                            }
                        },
                    };
                    state.send(id, reply);
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
            // Feature 686 (FR-007): the pod's clone of the project's remote. Spawned, like
            // `WorktreeCreate`, so a long clone never silences this loop's `Ping` answers.
            ClientMsg::RepoClone { url, dest } => {
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let root = crate::repo_clone::root_from_env();
                    let asked = dest.clone();
                    let done = tokio::task::spawn_blocking(move || {
                        crate::repo_clone::ensure_clone(root.as_deref(), &url, &dest)
                    })
                    .await;
                    let reply = match done {
                        Ok(Ok(())) => DaemonMsg::RepoCloned { dest: asked },
                        Ok(Err(failure)) => DaemonMsg::RepoCloneFailed {
                            dest: asked,
                            failure,
                        },
                        Err(e) => DaemonMsg::RepoCloneFailed {
                            dest: asked,
                            failure: micold_core::protocol::messages::CloneFailureWire::Other(
                                format!("the clone task failed: {e}"),
                            ),
                        },
                    };
                    task_state.send(id, reply);
                });
            }
            // #1002: a scoped credential for one repository, written to the in-memory file the
            // helper reads. Never logged; refused unless a project here pushes to the repository.
            ClientMsg::GitCredentialGrant {
                repo,
                token,
                expires_at,
            } => {
                let st = Arc::clone(state);
                let asked = repo.clone();
                let done = tokio::task::spawn_blocking(move || {
                    crate::git_credential::apply_grant(
                        &st.project_paths(),
                        &repo,
                        &token,
                        expires_at,
                    )
                })
                .await;
                use crate::git_credential::GrantRefused;
                match done {
                    Ok(Ok(())) => {}
                    Ok(Err(GrantRefused::NotSandboxed)) => refuse_git_credential(
                        state,
                        id,
                        ErrorKind::Refused,
                        format!("this daemon holds no git credentials; refused one for {asked}"),
                    ),
                    Ok(Err(GrantRefused::GitTooOld(version))) => {
                        tracing::warn!(version = %version, repo = %asked, "git credential refused: git is too old");
                        refuse_git_credential(
                            state,
                            id,
                            ErrorKind::Refused,
                            format!("git {version} is older than 2.31; refused a credential for {asked}"),
                        );
                    }
                    Ok(Err(GrantRefused::Invalid)) => refuse_git_credential(
                        state,
                        id,
                        ErrorKind::InvalidInput,
                        format!("the credential for {asked} is empty, malformed or expired"),
                    ),
                    Ok(Err(GrantRefused::NotAProjectRepo)) => refuse_git_credential(
                        state,
                        id,
                        ErrorKind::Refused,
                        format!("no project here pushes to {asked}; refused its credential"),
                    ),
                    Ok(Err(GrantRefused::Write(e))) => {
                        tracing::warn!(repo = %asked, error = %e, "git credential not written");
                        refuse_git_credential(
                            state,
                            id,
                            ErrorKind::IoFailed,
                            format!("could not write the git credential for {asked}: {e}"),
                        );
                    }
                    Err(e) => tracing::warn!(error = %e, "the git credential task failed"),
                }
            }
            ClientMsg::GitCredentialRevoke { repo } => {
                if let Some(root) = crate::git_credential::credential_root() {
                    if let Err(e) = crate::git_credential::remove_grant(&root, &repo.lowercased()) {
                        tracing::warn!(repo = %repo, error = %e, "git credential not removed");
                        refuse_git_credential(
                            state,
                            id,
                            ErrorKind::IoFailed,
                            format!("could not remove the git credential for {repo}: {e}"),
                        );
                    }
                }
            }
            ClientMsg::RemoteList { req, project } => {
                // Read-only and local: `git config`, never a network call (034 FR-002).
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                let listed = tokio::task::spawn_blocking(move || {
                    GitCli::new()
                        .remote_list(&repo)
                        .map(|raw| micold_core::git::parse_remote_list(&raw))
                })
                .await;
                let reply = match listed {
                    Ok(Ok(remotes)) => DaemonMsg::OperationOk {
                        req,
                        result: OperationResult::RemoteList { remotes },
                    },
                    Ok(Err(e)) => DaemonMsg::OperationError {
                        req,
                        kind: ErrorKind::GitFailed,
                        message: "could not list remotes".into(),
                        detail: Some(e.to_string()),
                    },
                    Err(e) => DaemonMsg::OperationError {
                        req,
                        kind: ErrorKind::Internal,
                        message: "could not list remotes".into(),
                        detail: Some(e.to_string()),
                    },
                };
                state.send(id, reply);
            }
            ClientMsg::MergedBranchCheck {
                req,
                project,
                checks,
            } => {
                // Read-only and local: refs and the object store, never a fetch, and nothing is
                // written (040 FR-016, FR-018a).
                if checks.len() > MERGED_BRANCH_CHECK_LIMIT {
                    state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::InvalidInput,
                            message: format!(
                                "at most {MERGED_BRANCH_CHECK_LIMIT} branches can be checked at once"
                            ),
                            detail: Some(format!("{} were asked for", checks.len())),
                        },
                    );
                    continue;
                }
                let Some((repo, true)) = state.project_repo(&project) else {
                    reject_non_repo(state, id, req, &project);
                    continue;
                };
                // Spawned, not awaited (BUG-009): up to 100 git calls must not keep this loop from
                // answering the client's `Ping`.
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let answered = tokio::task::spawn_blocking(move || {
                        let git = GitCli::new();
                        checks
                            .iter()
                            .map(|check| merged_branch_answer(&git, &repo, check))
                            .collect::<Vec<_>>()
                    })
                    .await;
                    let reply = match answered {
                        Ok(answers) => DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::MergedBranchCheck { answers },
                        },
                        Err(e) => task_failed(req, "merged branch check", &e),
                    };
                    task_state.send(id, reply);
                });
            }
            ClientMsg::WorktreeDelete {
                req,
                project,
                dir_name,
                stop_sessions,
                delete_branch,
            } => {
                if !matches!(state.project_repo(&project), Some((_, true))) {
                    reject_non_repo(state, id, req, &project);
                    continue;
                }
                // Spawned rather than awaited here, for the same reason as `WorktreeCreate` above
                // (BUG-009, T120/T124, FR-026a): `remove_dir_all` over a populated worktree is
                // unbounded work, and on a network filesystem it is slow enough to cross the
                // client's liveness deadline on its own.
                let task_state = Arc::clone(state);
                tokio::spawn(async move {
                    let state = &task_state;
                    let reply = match ops::delete_worktree(
                        state,
                        project,
                        dir_name,
                        stop_sessions,
                        delete_branch,
                    )
                    .await
                    {
                        Ok(deleted) => DaemonMsg::OperationOk {
                            req,
                            result: OperationResult::WorktreeDeleted {
                                branch_delete_failed: deleted.branch_delete_failed,
                                leftovers: deleted.leftovers,
                            },
                        },
                        Err(ops::DeleteFailure::LiveSessions(_)) => DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::Busy,
                            message: "worktree has a live session — stop it first or retry with \
                                      stop_sessions"
                                .into(),
                            detail: None,
                        },
                        // Failed delete: git's stderr rides along.
                        Err(ops::DeleteFailure::Git(e)) => DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::GitFailed,
                            message: "failed to remove the worktree".into(),
                            detail: Some(e.to_string()),
                        },
                        Err(ops::DeleteFailure::NotARepository) => DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::NotFound,
                            message: "unknown project".into(),
                            detail: None,
                        },
                        Err(ops::DeleteFailure::Task(join)) => {
                            task_failed(req, "worktree delete", &join)
                        }
                    };
                    state.send(id, reply);
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
                match ops::rename_worktree(state, &project, &dir_name, &display_name) {
                    Ok(()) => send_ack(state, id, req),
                    Err(ops::RenameFailure::Io(e)) => {
                        send_io_error(state, id, req, "failed to persist the rename", &e)
                    }
                    Err(ops::RenameFailure::Invalid(e)) => state.send(
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
            // --- 582: what the project offers to attach, and attaching it ---
            ClientMsg::AttachDiscover { req, project } => {
                // Blocking git and filesystem reads, so off the runtime and off the lock, in the
                // style of `refresh_worktrees_off_runtime`.
                let st = Arc::clone(state);
                let report =
                    tokio::task::spawn_blocking(move || st.attach_discover(&project)).await;
                match report {
                    Ok(report) => state.send(id, DaemonMsg::AttachReport { req, report }),
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::IoFailed,
                            message: "could not read the project's worktrees".into(),
                            detail: Some(e.to_string()),
                        },
                    ),
                }
            }
            ClientMsg::AttachApply {
                req,
                project,
                targets,
            } => {
                // Validate against a live read, not whatever the cache held when the dialog
                // opened, so a stale list cannot attach a path that is no longer a worktree. A
                // session target reads the provider stores, so the whole apply is off the runtime.
                let st = Arc::clone(state);
                let applied = tokio::task::spawn_blocking(move || {
                    st.refresh_worktrees(&project);
                    st.attach_apply(&project, &targets)
                })
                .await;
                match applied {
                    Ok(Ok(results)) => {
                        // One broadcast for the whole batch, and only when something changed.
                        if results
                            .iter()
                            .any(|r| r.outcome == micold_core::attach::AttachOutcome::Attached)
                        {
                            state.broadcast_catalog();
                        }
                        state.send(
                            id,
                            DaemonMsg::OperationOk {
                                req,
                                result: OperationResult::AttachApplied { results },
                            },
                        );
                    }
                    Ok(Err(e)) => send_io_error(state, id, req, "failed to persist the attach", &e),
                    Err(e) => state.send(
                        id,
                        DaemonMsg::OperationError {
                            req,
                            kind: ErrorKind::IoFailed,
                            message: "failed to persist the attach".into(),
                            detail: Some(e.to_string()),
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
                // Matched by location, and recorded as git spells it: a path through a symlink is
                // the same worktree, and `reconcile()` compares the stored path with git's exactly
                // (BUG-004).
                let probe = path.clone();
                let known = tokio::task::spawn_blocking(move || {
                    let porcelain = GitCli::new()
                        .worktree_list_porcelain(&repo)
                        .unwrap_or_default();
                    parse_worktrees(&porcelain)
                        .into_iter()
                        .find(|rec| same_path(&rec.path, &probe))
                        .map(|rec| rec.path)
                })
                .await;
                let path = match known {
                    Ok(Some(recorded)) => recorded,
                    Ok(None) => {
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
                };
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
                // The stored entry naming the same location, whatever the spelling (BUG-004).
                // Resolving links touches the filesystem, so it happens off the state lock.
                let stored = state.included_worktrees(&project);
                let probe = path.clone();
                let matched = tokio::task::spawn_blocking(move || {
                    stored
                        .into_iter()
                        .filter(|p| same_path(p, &probe))
                        .collect::<Vec<_>>()
                })
                .await;
                let matched = match matched {
                    Ok(matched) => matched,
                    Err(e) => {
                        state.send(
                            id,
                            DaemonMsg::OperationError {
                                req,
                                kind: ErrorKind::Internal,
                                message: "could not check the included worktrees".into(),
                                detail: Some(e.to_string()),
                            },
                        );
                        continue;
                    }
                };
                match matched
                    .iter()
                    .try_for_each(|p| state.exclude_worktree(&project, p))
                {
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
            // The user asking for the re-read directly (feature 029, FR-003). Deliberately the
            // whole arm: the same helper every other trigger reaches for, then the ack. Nothing is
            // mutated and nothing is added — a second discovery path is exactly what FR-004 exists
            // to prevent, and the listing rides out on the broadcast above rather than in the
            // reply. The order matters: a client that has the ack has, by then, the listing.
            ClientMsg::WorktreeRefresh { req, project } => {
                refresh_worktrees_and_broadcast(state, project).await;
                send_ack(state, id, req);
            }
            // --- US3: project management + session delete through the daemon (T053) ---
            ClientMsg::ProjectAdd { req, path } => match state.add_project(&path) {
                Ok(()) => {
                    refresh_worktrees_and_broadcast(state, path).await;
                    send_ack(state, id, req);
                }
                Err(e) => send_io_error(state, id, req, "failed to add the project", &e),
            },
            // No broadcast: a client never takes its active project from the snapshot's
            // `last_active` — each keeps its own — so nothing any client renders has changed. What
            // changed is what the next launch restores (002 BUG-003).
            ClientMsg::ProjectActivate { req, path } => match state.activate_project(&path) {
                Ok(true) => send_ack(state, id, req),
                Ok(false) => state.send(
                    id,
                    DaemonMsg::OperationError {
                        req,
                        kind: ErrorKind::NotFound,
                        message: "that project is not known or its folder is unavailable".into(),
                        detail: Some(path.display().to_string()),
                    },
                ),
                Err(e) => send_io_error(state, id, req, "failed to record the active project", &e),
            },
            ClientMsg::ProjectRemove { req, path } => match state.forget_project(&path) {
                Ok(ptys) => {
                    for pty in ptys {
                        let _ = pty.kill();
                    }
                    state.broadcast_catalog();
                    announce_git_targets(state, None).await;
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
    viewed_terminals.clear();
    sync_terminal_streams(state, id, &mut terminal_streams, &viewed_terminals);
    if let Some(stream) = view_stream.take() {
        stream.abort();
    }
    Ok(())
}

/// Re-discover a project's worktrees from git (off the async runtime, never under the state lock) and
/// push the refreshed catalog to every client, so a worktree mutation propagates to all windows
/// without further user action (FR-011, T053).
pub(crate) async fn refresh_worktrees_and_broadcast(
    state: &Arc<DaemonState>,
    project: std::path::PathBuf,
) {
    refresh_worktrees_off_runtime(state, &project).await;
    state.broadcast_catalog();
    announce_git_targets(state, None).await;
}

/// Tell the clients where this sandboxed daemon's projects push (#1002, contracts/protocol.md):
/// to `only` always (after its `Attach`), otherwise to everyone when the list changed. A
/// host-process daemon sends nothing.
async fn announce_git_targets(state: &Arc<DaemonState>, only: Option<crate::state::ClientId>) {
    if crate::git_credential::credential_root().is_none() {
        return;
    }
    let st = Arc::clone(state);
    let computed = tokio::task::spawn_blocking(move || {
        (
            crate::git_credential::collect_targets(&st.project_paths()),
            crate::git_credential::git_too_old(),
        )
    })
    .await;
    let Ok((targets, git_too_old)) = computed else {
        return;
    };
    if let Some(version) = &git_too_old {
        tracing::warn!(version = %version, "git is older than 2.31: scoped git credentials are refused");
    }
    let changed = state.git_targets_changed(&targets, git_too_old.as_deref());
    let msg = DaemonMsg::GitCredentialTargets {
        targets,
        git_too_old,
    };
    match only {
        Some(id) => state.send(id, msg),
        None if changed => state.broadcast(msg),
        None => {}
    }
}

/// The `req` of an `OperationError` answering no request: a refused or failed credential message.
/// Clients number requests up from 0, so the top of the range cannot be one of theirs.
pub const GIT_CREDENTIAL_NOTICE_REQ: u64 = u64::MAX;

/// A refused or failed credential request, to the requester only. It names the repository, never
/// the token.
fn refuse_git_credential(
    state: &DaemonState,
    id: crate::state::ClientId,
    kind: ErrorKind,
    message: String,
) {
    state.send(
        id,
        DaemonMsg::OperationError {
            req: GIT_CREDENTIAL_NOTICE_REQ,
            kind,
            message,
            detail: None,
        },
    );
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
        let adopted = st.discover_external_sessions(&proj);
        // Same hop (feature 551, R3): what each provider reports about who started the sessions
        // now in the catalog, adopted a moment ago or earlier.
        let origins = st.discover_origins(&proj);
        // Third step, same hop and same worktree cache (feature 029, FR-006, contract C14): the
        // sessions this application already knows but has no name for. After discovery, not
        // before — a session adopted a moment ago already carries whatever name its records hold,
        // so it is `Named` and costs this pass nothing.
        let recovered = st.recover_session_names(&proj);
        (adopted, recovered, origins)
    })
    .await;
    if let Ok((adopted, recovered, origins)) = discovered {
        if adopted > 0 {
            tracing::info!(
                project = %project.display(),
                count = adopted,
                "adopted sessions started outside this application"
            );
        }
        if origins > 0 {
            tracing::info!(
                project = %project.display(),
                count = origins,
                "recorded the origin a provider reported for sessions"
            );
        }
        if recovered > 0 {
            tracing::info!(
                project = %project.display(),
                count = recovered,
                "recovered session names from the AI CLI's own records"
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

/// The most branches one `MergedBranchCheck` may ask about. A reading sends one query per merged
/// pull request it shows, which stays far below this; a longer list is refused, so that one
/// request cannot hold a blocking thread for an unbounded run of git calls.
const MERGED_BRANCH_CHECK_LIMIT: usize = 50;

/// One answer of `MergedBranchCheck`: whether `check.branch`, as the repository holds it now, has
/// commits beyond `check.head` (feature 040, contracts/reading-and-wire.md §3). Asks git for the
/// branch's tip and, unless the tip is the head itself, whether the tip is an ancestor of it.
fn merged_branch_answer(
    git: &impl Git,
    repo: &std::path::Path,
    check: &MergedBranchQuery,
) -> BranchContainment {
    // The head arrives from outside: only a full commit id is ever handed to git.
    let is_commit_id = matches!(check.head.len(), 40 | 64)
        && check.head.bytes().all(|byte| byte.is_ascii_hexdigit());
    if !is_commit_id {
        return BranchContainment::Unknown;
    }
    let tip = git.branch_tip(repo, &check.branch);
    let ancestor = match tip.as_deref() {
        Some(tip) if tip != check.head => git.is_ancestor(repo, tip, &check.head),
        _ => None,
    };
    containment(tip.as_deref(), &check.head, ancestor)
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
    additional: micold_core::occupancy::Additional,
    reply: Option<(crate::state::ClientId, u64)>,
) {
    let started = ops::start_session(state, session, launch, additional);
    let task_state = Arc::clone(state);
    tokio::spawn(async move {
        // `ops::start_session` announces the finished start to every connection loop.
        let _ = started.await;
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

/// Answer a queue edit (#941 contracts/wire.md *Queue edits*) to `client` alone, and write the
/// delivery its lock section made, if any, off the connection loop.
fn answer_queue_change(
    state: &Arc<DaemonState>,
    client: crate::state::ClientId,
    req: u64,
    changed: Result<
        (OperationResult, Option<crate::state::Delivery>),
        micold_core::structured::input::QueueRefusal,
    >,
) {
    match changed {
        Ok((result, delivery)) => {
            state.send(client, DaemonMsg::OperationOk { req, result });
            if let Some(delivery) = delivery {
                let state = Arc::clone(state);
                tokio::task::spawn_blocking(move || {
                    let _ = state.deliver(delivery);
                });
            }
        }
        Err(why) => state.send(client, ack_or_refused(req, Err(why.to_string()))),
    }
}

/// Whether a terminal-only `request` to `process` of `session` is refused because it addresses a
/// structured session's AI process, which has no terminal (feature 938, FR-019). When it is, the
/// connection `client` is told why and nothing else is.
fn refuse_terminal_request(
    state: &DaemonState,
    client: u64,
    session: micold_core::session::SessionId,
    process: Option<SessionProcess>,
    request: RefusedRequest,
) -> bool {
    let refused = state.refuses_terminal_request(session, process);
    if refused {
        state.send(
            client,
            DaemonMsg::SessionRequestRefused {
                session,
                request,
                reason: "a structured session has no terminal: this request applies to terminal \
                         sessions and shell instances only"
                    .into(),
            },
        );
    }
    refused
}

/// Whether a `SessionStart`/`SessionRestart` is dropped because another session of the location
/// is live and the request is not explicit (feature 776, FR-013). Nothing is spawned or written;
/// the client learns the rule from the catalog and asks again with the flag (FR-016).
fn refuse_blocked_start(
    state: &DaemonState,
    session: micold_core::session::SessionId,
    additional: micold_core::occupancy::Additional,
) -> bool {
    let Some(refusal) = state.start_refusal(session, additional) else {
        return false;
    };
    tracing::warn!(session = %session.0, %refusal, "start refused");
    true
}

/// The error reply for a `spawn_blocking` task that itself failed (panicked / was cancelled) — an
/// internal fault, distinct from a git failure the task reported normally.
fn task_failed(req: u64, what: &str, join: &dyn std::fmt::Display) -> DaemonMsg {
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
    process: SessionProcess,
    pty: std::sync::Arc<crate::supervisor::PtySession>,
    framer: std::sync::Arc<std::sync::Mutex<crate::framer::Framer>>,
) {
    if let Some(prev) = current.take() {
        prev.abort();
    }
    if let Some(tx) = state.frame_sender(id) {
        *current = Some(tokio::spawn(stream_view(process, pty, framer, tx)));
    }
}

/// Point a client's single view (`SetViewedSession`) at `session`'s attached process. A structured
/// session's primary has no screen, only events (feature 938, R6): those stream the way a pane
/// streams them, as a one-member terminal set, so a single-pane window shows its transcript too
/// (BUG-1256). Any other process streams its screen through `view_stream`; the other is stopped,
/// and both are for no session.
fn show_single_view(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    view_stream: &mut Option<tokio::task::JoinHandle<()>>,
    terminal_streams: &mut TerminalStreams,
    viewed_terminals: &mut Vec<TerminalRef>,
    session: Option<micold_core::session::SessionId>,
) {
    let process = session
        .and_then(|s| state.attached_process(s))
        .unwrap_or(SessionProcess::Primary);
    let structured = session.filter(|s| {
        process == SessionProcess::Primary
            && state.session_kind(*s) == Some(micold_core::session::SessionKind::Structured)
    });
    *viewed_terminals = structured
        .map(|session| vec![TerminalRef { session, process }])
        .unwrap_or_default();
    sync_terminal_streams(state, id, terminal_streams, viewed_terminals);
    match session
        .filter(|_| structured.is_none())
        .and_then(|s| state.live_session(s).zip(state.session_framer(s)))
    {
        Some((pty, framer)) => restart_view(state, id, view_stream, process, pty, framer),
        None => {
            if let Some(prev) = view_stream.take() {
                prev.abort();
            }
        }
    }
}

/// The most terminals one client may view at once (feature 484: the 6-pane cap).
const MAX_VIEWED: usize = 6;

/// A client's running terminal streams, each with the PTY it streams (to notice a restart), or
/// `None` for a structured session's event stream, which a restart continues (feature 938).
type TerminalStreams = HashMap<
    TerminalRef,
    (
        Option<std::sync::Arc<crate::supervisor::PtySession>>,
        tokio::task::JoinHandle<()>,
    ),
>;

/// Make `streams` exactly the live members of `wanted` (feature 484): stop the others, keep the
/// running ones (no new snapshot, so a pane does not flicker), start the missing ones. A terminal
/// that is not live yet is started when its session announces itself (see the `started_rx` arm).
fn sync_terminal_streams(
    state: &Arc<DaemonState>,
    id: crate::state::ClientId,
    streams: &mut TerminalStreams,
    wanted: &[TerminalRef],
) {
    streams.retain(|t, (pty, handle)| {
        // A restart replaces the terminal's PTY: the stream of the old one would stay silent.
        let current = pty.as_ref().is_none_or(|pty| {
            state
                .terminal(*t)
                .is_some_and(|(live, _)| std::sync::Arc::ptr_eq(&live, pty))
        });
        let keep = wanted.contains(t) && !handle.is_finished() && current;
        if !keep {
            handle.abort();
        }
        keep
    });
    let Some(tx) = state.frame_sender(id) else {
        return;
    };
    for t in wanted {
        if streams.contains_key(t) {
            continue;
        }
        // A structured session's primary is its events, not a screen (feature 938, R6).
        if let Some(events) = state
            .event_stream(t.session)
            .filter(|_| t.process == SessionProcess::Primary)
        {
            let handle = tokio::spawn(stream_events(t.session, events, tx.clone()));
            streams.insert(*t, (None, handle));
            continue;
        }
        // One with no stream in this run yet but a saved event history is restored from it first,
        // off the runtime's threads (research R14). One that never ran has nothing to show.
        if t.process == SessionProcess::Primary
            && state.session_kind(t.session) == Some(micold_core::session::SessionKind::Structured)
            && state.has_saved_events(t.session)
        {
            let (state, session, tx) = (Arc::clone(state), t.session, tx.clone());
            let handle = tokio::spawn(async move {
                let loading = Arc::clone(&state);
                let events = tokio::task::spawn_blocking(move || {
                    loading.event_stream_loading(session, false)
                })
                .await
                .ok()
                .flatten();
                if let Some(events) = events {
                    stream_events(session, events, tx).await;
                }
            });
            streams.insert(*t, (None, handle));
            continue;
        }
        if let Some((pty, framer)) = state.terminal(*t) {
            let handle = tokio::spawn(stream_view(
                t.process,
                std::sync::Arc::clone(&pty),
                framer,
                tx.clone(),
            ));
            streams.insert(*t, (Some(pty), handle));
        }
    }
}

/// Stream one structured session's events to one client (feature 938, R6): the history as
/// `live: false` messages under the size limit, the first carrying `omitted`, then each later event
/// as its own `live: true` message. The snapshot and the subscription are taken together, so no
/// event is sent twice or missed; an event already in the snapshot is skipped all the same. A
/// viewer that falls behind is sent a fresh snapshot, from which it starts its log again, never a
/// gap. Ends when the client's channel closes or the task is aborted.
async fn stream_events(
    session: micold_core::session::SessionId,
    events: std::sync::Arc<crate::structured::EventStream>,
    tx: tokio::sync::mpsc::UnboundedSender<Frame<DaemonMsg>>,
) {
    let send = |omitted, events, live| {
        tx.send(Frame::Control(DaemonMsg::StructuredEvents {
            session,
            omitted,
            events,
            live,
        }))
        .is_ok()
    };
    loop {
        let (omitted, history, mut live) = events.subscribe();
        let mut last = history.last().map(|e| e.seq);
        let messages = crate::structured::history_messages(session, history);
        for (i, events) in messages.into_iter().enumerate() {
            if !send(if i == 0 { omitted } else { 0 }, events, false) {
                return;
            }
        }
        loop {
            let event = match live.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(missed)) => {
                    tracing::debug!(session = %session.0, missed, "a structured viewer lagged; resending its snapshot");
                    break;
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            };
            if last.is_some_and(|seq| event.seq <= seq) {
                continue;
            }
            last = Some(event.seq);
            if !send(0, vec![crate::structured::on_wire(session, event)], true) {
                return;
            }
        }
    }
}

/// Stream grid frames for one viewed session to one client. Sends a full snapshot first (attach /
/// reattach semantics, FR-014/FR-017 — the client gets the current screen, not a replay), then
/// coalesced deltas whenever the VT reports new output. Ends when the client's channel closes
/// (disconnect) or the task is aborted (view changed / connection ended).
async fn stream_view(
    process: SessionProcess,
    pty: std::sync::Arc<crate::supervisor::PtySession>,
    framer: std::sync::Arc<std::sync::Mutex<crate::framer::Framer>>,
    tx: tokio::sync::mpsc::UnboundedSender<Frame<DaemonMsg>>,
) {
    // Full snapshot on first view — the whole current screen, however long the client was away.
    // Read the change count first: a change during the snapshot only costs one extra delta.
    let mut seen = pty.signals().changes();
    let mut viewer = crate::framer::Viewer::default();
    let mut snapshot =
        framer
            .lock()
            .expect("framer poisoned")
            .frame_for(&mut viewer, pty.term(), true, None);
    snapshot.process = process;
    if tx.send(Frame::Grid(snapshot)).is_err() {
        return; // client already gone
    }

    let mut ticker = tokio::time::interval(FRAME_INTERVAL);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        // Only frame when there is new output; a clean tick sends nothing. The count is per
        // stream: another client viewing this terminal must still see the change.
        let now = pty.signals().changes();
        if now != seen {
            seen = now;
            let mut delta = framer.lock().expect("framer poisoned").frame_for(
                &mut viewer,
                pty.term(),
                false,
                None,
            );
            delta.process = process;
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
