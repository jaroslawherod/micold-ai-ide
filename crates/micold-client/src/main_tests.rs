use super::*;
// The catalog fixtures moved to `shell/daemon_sync.rs` with the reconcile tests that are
// mostly what they were for; the stamper-seeding tests below still build a snapshot, so they
// import them back rather than keep a second copy.
use crate::shell::daemon_sync::tests::{snapshot_with, summary, summary_at};
use micold_client::features::connection::Msg as ConnectionMsg;
use micold_client::features::sandbox::Msg as SandboxMsg;
use micold_client::features::settings::Msg as SettingsMsg;
use micold_client::features::settings::{EnvironmentDraft, SettingsDraft, TerminalDraft};
use micold_core::sandbox::placement::PlacementKind;
use micold_core::session::AiCli;
// These tests drive whole messages through `update_inner`, which is this file's dispatcher, so
// they stay here even though what they assert about is the daemon's: they are tests of the
// routing reaching the right arm as much as of the arm itself.
use micold_core::protocol::messages::{DaemonMsg, WireLifecycle};

// --- T111 / FR-028a: seeding the stamper from the daemon's authoritative position (BUG-006) ---

#[test]
fn seeding_adopts_the_daemons_position_for_a_session_this_client_never_drove() {
    // The restarted-UI case: the stamper is empty because the process is new, but the session
    // has been taking input for a while. Its first keystroke must be stamped where the daemon
    // expects, not at 0 — which is what made every pre-existing session read-only.
    let id = SessionId::new();
    let snapshot = snapshot_with("/p", vec![summary_at(id, "s", WireLifecycle::Running, 40)]);
    let mut stamper = SessionInputStamper::new();

    stamper.seed_from_catalog(&snapshot);

    let ClientMsg::SessionInput { serial, .. } = stamper.stamp(id, b"x".to_vec()) else {
        panic!("stamp must produce SessionInput");
    };
    assert_eq!(
        serial, 40,
        "the first keystroke resumes at the daemon's mark"
    );
}

#[test]
fn seeding_never_rewinds_a_counter_this_client_is_already_driving() {
    // A snapshot is a moment behind: input this client has already stamped may still be in
    // flight, so its counter is legitimately *ahead*. Adopting the older number would re-mint
    // serials the daemon has applied — the duplicate `Stale` exists to reject.
    let id = SessionId::new();
    let mut stamper = SessionInputStamper::new();
    for _ in 0..3 {
        stamper.stamp(id, b"x".to_vec());
    }

    let snapshot = snapshot_with("/p", vec![summary_at(id, "s", WireLifecycle::Running, 1)]);
    stamper.seed_from_catalog(&snapshot);

    let ClientMsg::SessionInput { serial, .. } = stamper.stamp(id, b"x".to_vec()) else {
        panic!("stamp must produce SessionInput");
    };
    assert_eq!(
        serial, 3,
        "the live counter continues; the stale snapshot is ignored"
    );
}

#[test]
fn a_session_the_daemon_is_not_hosting_seeds_at_zero() {
    // No live entry means no `InputReceiver`, so the catalog's default stands. The client and
    // the daemon are both at 0, which is exactly in step.
    let id = SessionId::new();
    let snapshot = snapshot_with(
        "/p",
        vec![summary_at(id, "s", WireLifecycle::InterruptedResumable, 0)],
    );
    let mut stamper = SessionInputStamper::new();

    stamper.seed_from_catalog(&snapshot);

    let ClientMsg::SessionInput { serial, .. } = stamper.stamp(id, b"x".to_vec()) else {
        panic!("stamp must produce SessionInput");
    };
    assert_eq!(serial, 0);
}

#[test]
fn update_inner_applies_window_focus_changed() {
    let mut app = App {
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
        activation: Default::default(),
        last_grid: None,
        env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
        env_include_script_path: String::new(),
        env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
        env_include_cache: HashMap::new(),
        env_include_last_outcome: EnvIncludeOutcome::Disabled,
        daemon: None,
        daemon_catalog: None,
        displaced: HashMap::new(),
        disconnected: false,
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: None,
        build_mismatch: None,
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        probe: None,
        scene_ready: false,
        scene_frames: 0,
        ripples_animating: Arc::new(AtomicUsize::new(0)),
        scene_ripple_frames: std::cell::Cell::new(0),
    };

    let _ = update_inner(&mut app, Message::WindowFocusChanged(false));
    assert!(!app.window_focused);

    let _ = update_inner(&mut app, Message::WindowFocusChanged(true));
    assert!(app.window_focused);
}

#[test]
fn terminal_resized_remembers_the_pane_size_for_future_spawns() {
    // Reproduces the reported bug: a freshly spawned session used to always start at the
    // hardcoded INIT_ROWS x INIT_COLS default, filling only that fixed area until the next
    // window resize reconciled it. `TerminalResized` (published by the pane widget whenever
    // its live size changes) must now be remembered on `App` so `spawn_pty` call sites can
    // seed new sessions at the pane's actual current size instead.
    let mut app = App {
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
        activation: Default::default(),
        last_grid: None,
        env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
        env_include_script_path: String::new(),
        env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
        env_include_cache: HashMap::new(),
        env_include_last_outcome: EnvIncludeOutcome::Disabled,
        daemon: None,
        daemon_catalog: None,
        displaced: HashMap::new(),
        disconnected: false,
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: None,
        build_mismatch: None,
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        probe: None,
        scene_ready: false,
        scene_frames: 0,
        ripples_animating: Arc::new(AtomicUsize::new(0)),
        scene_ripple_frames: std::cell::Cell::new(0),
    };
    assert_eq!(app.last_grid, None);

    let _ = update_inner(
        &mut app,
        Message::Session(SessionMsg::TerminalResized {
            cols: 220,
            rows: 60,
        }),
    );
    assert_eq!(app.last_grid, Some((220, 60)));

    let _ = update_inner(
        &mut app,
        Message::Session(SessionMsg::TerminalResized {
            cols: 180,
            rows: 45,
        }),
    );
    assert_eq!(app.last_grid, Some((180, 45)));
}

/// T061 (BUG-003, `006-real-terminal-emulator` FR-014a): displaying a session must state the
/// pane's size *before* starting it, so the daemon spawns the process at that size instead of
/// its 100×30 seed.
///
/// The pane widget publishes `TerminalResized` only when its own size changes, so a session
/// started into a window nobody is resizing was never told anything at all. The neighbouring
/// `terminal_resized_remembers_the_pane_size_for_future_spawns` pins that `App::last_grid` is
/// *stored*; nothing pinned that anything reads it, and nothing did — this closes that gap by
/// asserting the message order on the wire.
#[test]
fn displaying_a_session_states_the_pane_size_before_starting_it() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
    let id = SessionId::new();
    app.last_grid = Some((220, 60));

    let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

    match rx.try_recv() {
        Ok(ClientMsg::SessionResize {
            session,
            cols,
            rows,
        }) => {
            assert_eq!(session, id);
            assert_eq!((cols, rows), (220, 60));
        }
        other => panic!("expected the size first, got {other:?}"),
    }
    assert!(
        matches!(
            rx.try_recv(),
            Ok(ClientMsg::SessionStart { session }) if session == id
        ),
        "the start must follow the size, not precede it"
    );
}

/// The pane has not been laid out yet (nothing has published a size): the start goes out alone
/// and the daemon's own default applies. A zero-size guess here would be worse than no guess.
#[test]
fn displaying_a_session_before_the_pane_has_a_size_sends_only_the_start() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
    let id = SessionId::new();
    assert_eq!(app.last_grid, None);

    let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

    assert!(
        matches!(
            rx.try_recv(),
            Ok(ClientMsg::SessionStart { session }) if session == id
        ),
        "the first message is the start itself"
    );
}

/// BUG-442 (feature 011, FR-007(b)): the AI CLI tab's restart control tells the service it is a
/// restart, so the service re-sources the session's directory before relaunching. The service
/// cannot tell a `SessionStart` sent by the restart from one sent by selecting a session.
#[test]
fn the_ai_cli_restart_control_sends_a_restart_and_selecting_sends_a_start() {
    let project = std::path::PathBuf::from("/tmp/project");
    let id = SessionId::new();
    let mut app = base_app();
    app.core.workspace.active = Some(project.clone());
    let _ = connect(
        &mut app,
        snapshot_with(
            project.to_str().unwrap(),
            vec![summary(id, "s", WireLifecycle::Idle)],
        ),
    );
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    app.core.session.active = Some(id);

    let _ = update_inner(
        &mut app,
        Message::Session(SessionMsg::TerminalRestartRequested),
    );
    let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|m| matches!(m, ClientMsg::SessionRestart { session } if *session == id)),
        "the restart control must send `SessionRestart`, so the service re-sources the \
         session's directory (FR-007(b)). Sent: {sent:?}"
    );
    assert!(
        !sent
            .iter()
            .any(|m| matches!(m, ClientMsg::SessionStart { .. })),
        "and not a plain `SessionStart` beside it, which would start the session from the cached \
         environment first. Sent: {sent:?}"
    );

    let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));
    let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|m| matches!(m, ClientMsg::SessionStart { session } if *session == id))
            && !sent
                .iter()
                .any(|m| matches!(m, ClientMsg::SessionRestart { .. })),
        "selecting a session is not a restart: it sends `SessionStart`. Sent: {sent:?}"
    );
}

// --- BUG-002 (feature 025): the restored session is started, not only viewed ----------------
//
// Deciding which session to display and asking the daemon to run it are two halves of one act,
// and only `view_and_start` performed the second. `restore_after_activation` is client state
// only — it resolves the memory, reveals the row, takes the keyboard, and says nothing to the
// daemon. So the launch made a session current that the daemon was not hosting, and
// `SetViewedSession` had no stream to open. BUG-001 made that screen say so; these make it not
// happen.
//
// The seam had no tests at all: the plan reasoned `boot()` was glue because the decision
// (*which* session) lives in the tested reducer. What the client then sends is a second
// decision, and this is where it is now pinned.

/// Settings that change nothing and source no environment script — these tests are about which
/// session messages go out, and env-include would reach for the filesystem on the way.
pub(crate) fn quiet_settings() -> micold_core::protocol::messages::DaemonSettings {
    micold_core::protocol::messages::DaemonSettings {
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        env_include_enabled: false,
        env_include_script_path: String::new(),
        env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
        default_ai_cli: AiCli::ClaudeCode,
        pi_activity_component: true,
        tool_server_enabled: true,
        desktop_notifications: true,
        diff_layout: Default::default(),
        cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
        pr_status_enabled: false,
    }
}

/// Drive a daemon connection for `app` and return every `ClientMsg` it sent, in order.
fn connect(
    app: &mut App,
    catalog: micold_core::protocol::messages::CatalogSnapshot,
) -> Vec<ClientMsg> {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let _ = update_inner(
        app,
        Message::Connection(ConnectionMsg::Connected {
            outbox: micold_client::daemon::Outbox::new(tx),
            catalog,
            settings: quiet_settings(),
        }),
    );
    let mut sent = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        sent.push(msg);
    }
    sent
}

/// The container's log is an *answer*, and an empty one is an answer too (FR-038).
///
/// A user asks for diagnostics because something is wrong. Two outcomes are useful — "here is
/// what it said" and "it is running and has said nothing" — and neither is silence, which is
/// what the arm did before it existed.
#[test]
fn the_containers_log_is_reported_whether_or_not_it_has_anything_in_it() {
    let mut app = base_app();
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Diagnostics(vec![
            "starting session service".into(),
            "bind /work/demo: permission denied".into(),
        ])),
    );
    let said = app
        .core
        .notifications
        .queue
        .visible()
        .expect("a notice was raised");
    assert!(
        said.message.contains("permission denied"),
        "the most recent line is the one worth showing: {}",
        said.message
    );

    let mut empty = base_app();
    let _ = update_inner(
        &mut empty,
        Message::Sandbox(SandboxMsg::Diagnostics(Vec::new())),
    );
    let said = empty
        .core
        .notifications
        .queue
        .visible()
        .expect("an empty log still gets an answer");
    assert!(
        said.message.contains("logged nothing"),
        "an empty log should say so rather than say nothing: {}",
        said.message
    );
}

/// Connecting must **start** the restored session, not only view it (FR-004a, contract §3.3a).
///
/// `InterruptedResumable` is deliberate rather than incidental: it is what every durable session
/// is after a restart, which makes this the ordinary launch and not an edge case.
#[test]
fn connecting_starts_the_restored_session_rather_than_only_viewing_it() {
    let project = PathBuf::from("/repo/demo");
    let id = SessionId::new();
    let mut app = base_app();
    app.core.workspace.active = Some(project.clone());
    // What `boot()`'s `restore_after_activation` leaves behind: the session is already current
    // when the connection arrives, chosen from the memory loaded off disk.
    app.core.session.active = Some(id);

    let sent = connect(
        &mut app,
        snapshot_with(
            "/repo/demo",
            vec![summary(
                id,
                "left off here",
                WireLifecycle::InterruptedResumable,
            )],
        ),
    );

    let attach = sent
        .iter()
        .position(|m| matches!(m, ClientMsg::Attach { project: p, .. } if *p == project))
        .expect("the project is attached first");
    let start = sent
        .iter()
        .position(|m| matches!(m, ClientMsg::SessionStart { session } if *session == id))
        .expect("the restored session must be started, or no frame will ever arrive for it");
    let view = sent
        .iter()
        .position(|m| matches!(m, ClientMsg::SetViewedSession { session: Some(s), .. } if *s == id))
        .expect("and it must still be the session the daemon streams");
    assert!(
        attach < start,
        "nothing about a session precedes the attach"
    );
    assert!(
        start < view,
        "the start precedes the view, the order `view_and_start` already establishes"
    );
}

/// Exactly one start, naming the restored session (SC-005a, contract §3.3b).
///
/// This is the bound that replaces FR-004's prohibition. Resuming the one session the user is
/// being shown is the feature; resuming every session the application happens to remember would
/// be a launch that spawns a process per project, which is what "restoring starts nothing" was
/// really protecting against.
#[test]
fn connecting_starts_only_the_session_it_restores() {
    let project = PathBuf::from("/repo/demo");
    let restored = SessionId::new();
    let sibling = SessionId::new();
    let elsewhere = SessionId::new();
    let mut app = base_app();
    app.core.workspace.active = Some(project.clone());
    app.core.session.active = Some(restored);

    let mut catalog = snapshot_with(
        "/repo/demo",
        vec![
            summary(restored, "restored", WireLifecycle::InterruptedResumable),
            summary(sibling, "same project", WireLifecycle::Idle),
        ],
    );
    let mut other = snapshot_with(
        "/repo/other",
        vec![summary(
            elsewhere,
            "another project entirely",
            WireLifecycle::InterruptedResumable,
        )],
    );
    catalog.projects.append(&mut other.projects);

    let sent = connect(&mut app, catalog);

    let started: Vec<SessionId> = sent
        .iter()
        .filter_map(|m| match m {
            ClientMsg::SessionStart { session } => Some(*session),
            _ => None,
        })
        .collect();
    assert_eq!(
        started,
        vec![restored],
        "one start, naming the restored session — not its neighbours, not another project's"
    );
}

/// A service restart resumes at most the one session being restored (`010` FR-006b — BUG-016).
///
/// This is the case a daemon restart actually produces, and it is the one
/// [`connecting_starts_only_the_session_it_restores`] above does not cover: after the service
/// comes back, *every* session it recovered is `InterruptedResumable`, not just the remembered
/// one. `010` FR-006b used to promise that no agent takes action after a restart at all, which
/// the client has not done since `025` FR-004a made a restore resume its session; what survives
/// the amendment is the scope — one start, for the session the user is being shown, and nothing
/// for the sessions sitting beside it waiting to be asked.
///
/// An agent that resumes reads the conversation, can call tools, and costs tokens, so the count
/// here is the point: two interrupted-resumable sessions in the reopened project, one in another
/// project, and exactly one `SessionStart`.
#[test]
fn a_service_restart_resumes_only_the_session_being_restored() {
    let project = PathBuf::from("/repo/demo");
    let restored = SessionId::new();
    let interrupted_neighbour = SessionId::new();
    let elsewhere = SessionId::new();
    let mut app = base_app();
    app.core.workspace.active = Some(project.clone());
    app.core.session.active = Some(restored);

    // What the daemon reports after a restart: it relaunched nothing, so both of this project's
    // live-before-the-restart sessions come back interrupted-resumable (`server.rs`, the only
    // lifecycle daemon startup may produce), and so does the other project's.
    let mut catalog = snapshot_with(
        "/repo/demo",
        vec![
            summary(
                restored,
                "was displayed",
                WireLifecycle::InterruptedResumable,
            ),
            summary(
                interrupted_neighbour,
                "was running, not displayed",
                WireLifecycle::InterruptedResumable,
            ),
        ],
    );
    let mut other = snapshot_with(
        "/repo/other",
        vec![summary(
            elsewhere,
            "another project entirely",
            WireLifecycle::InterruptedResumable,
        )],
    );
    catalog.projects.append(&mut other.projects);

    let sent = connect(&mut app, catalog);

    let started: Vec<SessionId> = sent
        .iter()
        .filter_map(|m| match m {
            ClientMsg::SessionStart { session } => Some(*session),
            _ => None,
        })
        .collect();
    assert_eq!(
        started,
        vec![restored],
        "a restart resumes the restored session and no other: its interrupted neighbour and \
             the other project's session must still wait for an explicit resume"
    );
}

/// BUG-002: connecting is one of the moments the client asks which AI CLIs the service can run
/// (027 FR-023c, T144; today contract C1's A1), and it is one the user does not trigger by hand.
///
/// It asked into a handle it had not stored yet. `ask_cli_availability` returns early when
/// `app.daemon` is `None`, which it is on a first connect and — since `on_disconnected` clears
/// it — on every reconnect after, so the request was never sent. The single answer the client
/// held then (feature 033 later made it one per directory) stayed `None` for the whole run, and
/// `None` is read as the empty set by everything that decides what to *offer*: no override
/// chevron on the sidebar row (026 FR-004, FR-006), and a default that is not installed started
/// instead of offering the CLIs that are (026 FR-002).
/// Opening Settings was the only thing that could repair it: in 027 the only other places that
/// asked were that view and the menu the missing chevron opens.
///
/// Two assertions, because either alone passes while wrong: the send is what broke, the
/// chevron is why anyone noticed. Neither is visible to
/// `tests/cli_availability_comes_from_the_service.rs`, which scans source text — a dead call
/// site spells identically to a live one, which is how this shipped through Phase 13.
#[test]
fn connecting_asks_which_clis_the_service_can_run() {
    let mut app = base_app();
    // An active project, so the connection sends *something* either way. Without it the
    // assertion below is also satisfied by an outbox nothing can reach, which would pass
    // against a broken `Outbox` as readily as against the bug.
    app.core.workspace.active = Some(PathBuf::from("/repo/demo"));
    let sent = connect(&mut app, snapshot_with("/repo/demo", Vec::new()));
    assert!(
        sent.iter().any(|m| matches!(m, ClientMsg::Attach { .. })),
        "fixture check: the connection's outbox must be reaching this test at all"
    );

    assert!(
        sent.iter()
            .any(|m| matches!(m, ClientMsg::AiCliAvailabilityRequest { .. })),
        "connecting must ask the service which AI CLIs it can run (FR-023c): a reconnect may \
             be to a different sandbox, and the previous connection's answer describes a container \
             that is gone. Sent instead: {sent:?}"
    );

    let home = sent
        .iter()
        .find_map(|m| match m {
            ClientMsg::AiCliAvailabilityRequest { req, cwd: None } => Some(*req),
            _ => None,
        })
        .expect("connecting asks about the home directory");
    let _ = update_inner(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
            req: home,
            available: vec![AiCli::ClaudeCode, AiCli::Copilot],
            env: None,
        })),
    );
    assert_eq!(
        app.core
            .session
            .availability
            .home()
            .map(|answer| answer.available.clone()),
        Some(vec![AiCli::ClaudeCode, AiCli::Copilot]),
        "the answer is filed as the home directory's"
    );
    assert!(
        app.core
            .session
            .start_affordance_offers_a_choice(Path::new("/repo/demo")),
        "with two CLIs reported, the sidebar row's override chevron must exist (026 FR-006) — \
             the answer arriving is the whole point of asking for it"
    );
}

/// With nothing remembered, a launch starts nothing at all (FR-007, SC-005a).
///
/// The overview is a legitimate place to land, and landing there must stay free of side effects:
/// FR-007 forbids choosing a session on the user's behalf, and starting one would be that choice
/// made twice over.
#[test]
fn connecting_with_no_remembered_session_starts_nothing() {
    let project = PathBuf::from("/repo/demo");
    let unchosen = SessionId::new();
    let mut app = base_app();
    app.core.workspace.active = Some(project.clone());
    assert_eq!(
        app.core.session.active, None,
        "the memory resolved to nothing"
    );

    let sent = connect(
        &mut app,
        snapshot_with(
            "/repo/demo",
            vec![summary(unchosen, "not chosen", WireLifecycle::Idle)],
        ),
    );

    assert!(
        !sent
            .iter()
            .any(|m| matches!(m, ClientMsg::SessionStart { .. })),
        "landing on the project overview must not run anything (FR-007)"
    );
    assert!(
        sent.iter()
            .any(|m| matches!(m, ClientMsg::SetViewedSession { session: None, .. })),
        "the daemon is still told that no session is viewed"
    );
}

/// BUG-002 (024): a drained reveal must record where it sent the list, because the list will
/// not always tell us.
///
/// `scroll_offset` is a mirror of the scrollable's position, and its only writer is
/// `Message::Sidebar(SidebarMsg::Scrolled)` — which iced publishes from `notify_viewport`, and
/// `notify_viewport` returns without publishing when the content fits the viewport
/// (`iced_widget/src/scrollable.rs`). So a reveal that runs in a project whose sidebar fits —
/// the short project you pass through on the way somewhere — moves the list and is never told,
/// and the mirror keeps the *previous* project's offset.
///
/// What that costs is the whole feature: on the next arrival the drain measures the row against
/// an offset the list is nowhere near, decides it is already visible, and consumes the arm
/// under FR-009. Reproduced on screen 2026-08-20 — the panel sat at the top with the marked row
/// 1,968px below the fold, and the trace read `scroll_offset=734 -> no scroll, the row is
/// already visible` immediately followed by `the scrollable reports offset 0`.
#[test]
fn a_drained_reveal_records_where_it_sent_the_list() {
    use micold_core::project::{Availability, Project};
    use micold_core::session::{AiCli, Session};
    use micold_core::worktree::{Worktree, WorktreeStatus};

    let mut app = base_app();
    let path = PathBuf::from("/repo");
    app.core.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    app.core.workspace.active = Some(path.clone());
    app.core.worktree.worktrees = vec![Worktree {
        dir_name: "only".to_string(),
        path: PathBuf::from("/repo/.claude/worktrees/only"),
        branch: Some("feat/only".to_string()),
        status: WorktreeStatus::Valid,
        included: false,
    }];
    let session = Session::start_new(
        SessionLocation::Worktree("only".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    app.core.workspace.sessions.insert(path, vec![session]);
    app.core.session.active = Some(id);
    // Everything here fits: one location in a tall panel. This is the project that scrolls
    // without reporting.
    app.core.sidebar.viewport_height = 400;
    // Left behind by the project we just came from, whose list was long enough to scroll.
    app.core.sidebar.scroll_offset = 734;
    app.core.sidebar.pending_reveal_scroll = true;

    assert!(
        reveal_scroll(&mut app).is_some(),
        "precondition: the reveal drains and asks for a scroll — 734 is not where this \
             one-location list can be"
    );

    assert_eq!(
        app.core.sidebar.scroll_offset, 0,
        "the reveal knows where it sent the list, so it must not wait to be told: leaving 734 \
             here is BUG-002, and the next arrival measures its row against a position the panel \
             left long ago"
    );
}

/// Builds an `App` with every field at a neutral default, so each test only spells out the
/// fields it actually varies (mirrors the literal-construction pattern the other tests in this
/// module already use, factored out because T100's tests need several variants of it).
///
/// No settings store: a test that saves must never write the developer's own `settings.json`
/// (#368). A test that needs to read a save back gives itself a store in a temp dir.
pub(crate) fn base_app() -> App {
    App {
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            // Spec 035: no test examines the developer's own files. A trigger test swaps in a
            // fake whose calls it reads.
            .with_script_path_probe(Arc::new(
                micold_core::script_path_check::FakeScriptPathProbe::answering(
                    micold_core::script_path_check::ProbeAnswer::File,
                ),
            ))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
        activation: Default::default(),
        last_grid: None,
        env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
        env_include_script_path: String::new(),
        env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
        env_include_cache: HashMap::new(),
        env_include_last_outcome: EnvIncludeOutcome::Disabled,
        daemon: None,
        daemon_catalog: None,
        displaced: HashMap::new(),
        disconnected: false,
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: None,
        build_mismatch: None,
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        probe: None,
        scene_ready: false,
        scene_frames: 0,
        ripples_animating: Arc::new(AtomicUsize::new(0)),
        scene_ripple_frames: std::cell::Cell::new(0),
    }
}

// --- FR-035a: an accepted fallback has to reach the connection, not only the banner --------

/// The failure [`app_with_a_failed_sandbox`] starts from: the runtime is not installed.
fn failed_sandbox_state() -> micold_core::sandbox::lifecycle::SandboxState {
    use micold_core::sandbox::lifecycle::{Failure, Stage};
    micold_core::sandbox::lifecycle::SandboxState::Failed(Failure {
        stage: Stage::Probing,
        error: micold_core::sandbox::runtime::RuntimeError::NotInstalled {
            kind: micold_core::sandbox::runtime::RuntimeKind::Docker,
        },
    })
}

/// An `App` configured for the sandbox, with the sandbox failed and a boot plan to restart.
fn app_with_a_failed_sandbox() -> App {
    let mut app = base_app();
    app.placement.kind = micold_core::sandbox::placement::PlacementKind::LocalSandbox;
    app.sandbox = micold_client::features::sandbox::Sandbox {
        state: failed_sandbox_state(),
        ..micold_client::features::sandbox::Sandbox::default()
    };
    app.sandbox_boot = Some(shell::sandbox::BootPlan {
        profile: micold_core::sandbox::SandboxProfile::default(),
        state_dir: PathBuf::from("/tmp/micold-test-state"),
        projects: Vec::new(),
    });
    app
}

#[test]
fn accepting_the_fallback_moves_the_connection_to_a_host_process() {
    // The defect this was written for: `SandboxFallbackAccepted` used to record consent and
    // return `Task::none()`, and nothing else changed. But `daemon::connection` dials from
    // `app.placement`, and its `LocalSandbox` arm never falls back to a host process by design
    // (FR-035) — so the user pressed "Run without it for now", the banner said they were
    // running unsandboxed, and the client kept dialling a port nothing was listening on. The
    // offer worked as a statement and not as a service.
    let mut app = app_with_a_failed_sandbox();

    let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::FallbackAccepted));

    assert!(
        app.sandbox.fallback.is_some(),
        "the consent must be recorded, or the persistent notice has nothing to show"
    );
    assert_eq!(
            app.placement.kind,
            micold_core::sandbox::placement::PlacementKind::HostProcess,
            "accepting the fallback must move the connection to a host process — the subscription              is keyed on this value, and only changing it makes the client dial somewhere a daemon              can actually be"
        );
}

#[test]
fn a_fallback_that_was_not_on_offer_leaves_the_connection_where_it_is() {
    // The other half: consent is not a thing the application may take on its own behalf
    // (FR-035). A running sandbox offers no fallback, so nothing here may move the placement —
    // otherwise a stray message would quietly unsandbox a working session.
    let mut app = base_app();
    app.placement.kind = micold_core::sandbox::placement::PlacementKind::LocalSandbox;
    app.sandbox = micold_client::features::sandbox::Sandbox {
        state: micold_core::sandbox::lifecycle::SandboxState::Running(
            micold_core::sandbox::runtime::ContainerId("x".into()),
        ),
        ..micold_client::features::sandbox::Sandbox::default()
    };

    let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::FallbackAccepted));

    assert!(app.sandbox.fallback.is_none());
    assert_eq!(
        app.placement.kind,
        micold_core::sandbox::placement::PlacementKind::LocalSandbox,
        "a fallback nobody offered must not move a running sandbox's connection"
    );
}

#[test]
fn trying_the_sandbox_again_brings_the_connection_back_with_it() {
    // The return leg. Without it the restart succeeds, the container comes up, and every
    // session keeps running on the host process the fallback moved us to — a banner claiming
    // containment over an unconfined shell, which is precisely what FR-035b forbids.
    let mut app = app_with_a_failed_sandbox();
    let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::FallbackAccepted));
    assert_eq!(
        app.placement.kind,
        micold_core::sandbox::placement::PlacementKind::HostProcess,
        "precondition: the fallback moved us off the sandbox"
    );

    let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::RestartRequested));

    assert_eq!(
        app.placement.kind,
        micold_core::sandbox::placement::PlacementKind::LocalSandbox,
        "asking for the sandbox back must point the connection back at it"
    );
}

// --- T117 / FR-024a / SC-021: the read-only state must end when the daemon says we hold the
// project (BUG-007) -----------------------------------------------------------------------

/// A *different* window: same build as this one, its own instance. Every gate below that
/// speaks of "another window" means this — a second process, which is what the daemon reports
/// when the takeover is real. `010` BUG-022 is the case where it is not.
fn other_window() -> micold_core::protocol::messages::ClientIdentity {
    micold_core::protocol::messages::ClientIdentity::new(
        "other-window",
        micold_core::protocol::messages::ClientInstance {
            pid: 4242,
            nonce: "a-second-process".into(),
        },
    )
}

/// This window, as the daemon would name it back to us.
fn this_window() -> micold_core::protocol::messages::ClientIdentity {
    micold_core::protocol::messages::ClientIdentity::new(
        "micold-ai-ide/test",
        micold_core::protocol::messages::ClientInstance::current(),
    )
}

/// An `App` holding `project` as its active project, with nothing else varied.
fn app_on_project(project: &Path) -> App {
    let mut app = base_app();
    app.core.workspace.active = Some(project.to_path_buf());
    app
}

fn feed(app: &mut App, msg: DaemonMsg) {
    let _ = update_inner(app, Message::Connection(ConnectionMsg::Event(msg)));
}

// --- `010` BUG-021: what a scroll gesture costs -------------------------------------------

/// A grid sitting at the live tail: `rows` visible lines at `viewport_top`, no history cached.
fn at_the_tail(session: SessionId, viewport_top: i64, rows: u16) -> GridCache {
    use micold_core::protocol::grid::{
        GridFrame, WireCursor, WireCursorShape, WireLine, WireStyle,
    };
    let mut cache = GridCache::new();
    cache.apply(&GridFrame {
        session,
        seq: 1,
        generation: 1,
        full: true,
        viewport_top: LineId(viewport_top),
        oldest_available: LineId(0),
        cols: 80,
        rows,
        cursor: WireCursor {
            line: LineId(viewport_top),
            col: 0,
            shape: WireCursorShape::Block,
            visible: true,
            blinking: false,
        },
        styles: vec![WireStyle {
            fg: micold_core::protocol::grid::WireColor::Named(7),
            bg: micold_core::protocol::grid::WireColor::Named(0),
            flags: 0,
            underline_color: None,
        }],
        hyperlinks: Vec::new(),
        lines: (0..rows as i64)
            .map(|i| WireLine {
                id: LineId(viewport_top + i),
                text: "x".into(),
                runs: vec![micold_core::protocol::grid::StyleRun { len: 1, style: 0 }],
                extras: Vec::new(),
                wrapped: false,
            })
            .collect(),
        mode: 0,
        input_serial: None,
    });
    cache
}

/// An `App` scrolled-ready: one session, its grid at the tail, and a socket to read.
fn app_at_the_tail() -> (
    App,
    iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) {
    let (tx, rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    let id = SessionId::new();
    app.core.session.active = Some(id);
    app.grids.insert(id, at_the_tail(id, 4000, 69));
    (app, rx)
}

fn scrollback_ranges(
    rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) -> Vec<Range<LineId>> {
    let mut out = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let ClientMsg::ScrollbackRequest { ranges, .. } = msg {
            out.extend(ranges);
        }
    }
    out
}

/// `010` BUG-021, measured the way the report measured it: not one request, but a gesture.
///
/// The unit gates in `grid.rs` pin the shape of a single range. This one drives 150 wheel
/// notches of two lines each through the real dispatcher against a real `Outbox` and reads what
/// went on the wire. Before the fix that was 150 requests — one per notch, because no answer
/// had arrived to cache — each running from the revealed line to the live tail, summing to tens
/// of thousands of lines for a gesture that revealed three hundred. The daemon served every one
/// of them serially under the session's terminal lock, which is why a three-second scroll left
/// it saturated for fifteen seconds afterwards and the pane blank throughout.
///
/// Both numbers below are ceilings with slack in them, deliberately: what is load-bearing is
/// that the cost of a gesture tracks the lines it *reveals*, not the notches it takes or the
/// depth it reaches. An exact count would pin the prefetch size, which is a tuning decision.
#[test]
fn a_scroll_gesture_asks_once_per_viewport_not_once_per_notch() {
    let (mut app, mut rx) = app_at_the_tail();

    for _ in 0..150 {
        let _ = update_inner(&mut app, Message::Session(SessionMsg::TerminalScrolled(2)));
    }

    let asked = scrollback_ranges(&mut rx);
    assert_eq!(app.display_offset, 300, "the gesture went 300 lines back");
    assert!(
        asked.len() <= 6,
        "150 notches over 300 lines sent {} requests; one per viewport of travel is ~5",
        asked.len()
    );
    let lines: i64 = asked.iter().map(|r| r.end.0 - r.start.0).sum();
    assert!(
        lines <= 4 * 300,
        "a gesture revealing 300 lines asked the daemon for {lines}"
    );
    assert!(
        asked.iter().all(|r| r.end.0 - r.start.0 <= 2 * 69),
        "every request is bounded by the viewport, however deep the gesture went: {asked:?}"
    );
}

/// The gap probe D found, and the reason it is worth writing down: an in-flight record is
/// released by its *answer*, and removing that release broke nothing.
///
/// Every other gate here stops at the request. None of them let an answer arrive and then
/// scrolled again, so nothing said the record was ever cleared — and a record that is never
/// cleared is a range this view will never ask for twice. That is harmless while the answer
/// carries the lines (the cache then holds them, and `held` is true either way) and permanent
/// when it does not: the daemon trimmed them, or the session went away, and those rows stay
/// blank for as long as the window stays there. A storm traded for a hole.
#[test]
fn a_range_whose_answer_brought_nothing_is_asked_for_again() {
    let (mut app, mut rx) = app_at_the_tail();
    let session = app
        .core
        .session
        .active
        .expect("the fixture views a session");

    let _ = update_inner(&mut app, Message::Session(SessionMsg::TerminalScrolled(2)));
    let asked = scrollback_ranges(&mut rx);
    assert_eq!(
        asked.len(),
        1,
        "one notch into un-cached history, one request"
    );
    let req = *app
        .scrollback_inflight
        .keys()
        .next()
        .expect("and it is on the record as in flight");

    feed(
        &mut app,
        DaemonMsg::ScrollbackResponse {
            session,
            req,
            oldest_available: LineId(0),
            newest: LineId(4068),
            lines: Vec::new(),
            styles: Vec::new(),
            hyperlinks: Vec::new(),
            more: false,
        },
    );

    let _ = update_inner(&mut app, Message::Session(SessionMsg::TerminalScrolled(0)));
    assert_eq!(
        scrollback_ranges(&mut rx),
        asked,
        "the answer brought none of that range, so it is not on its way and must be asked \
             for again"
    );
}

/// The in-flight record is what stops the storm, so it must never outlive its answer.
///
/// `req`s are per-connection: a request outstanding when the socket drops will never be
/// answered on the next one. Keeping the record would suppress exactly the request that would
/// have filled those rows, and the pane would stay blank for as long as the view stayed there —
/// trading a storm for a permanent hole.
#[test]
fn a_disconnect_releases_the_ranges_that_will_never_be_answered() {
    let (mut app, _rx) = app_at_the_tail();
    let _ = update_inner(&mut app, Message::Session(SessionMsg::TerminalScrolled(2)));
    assert!(
        !app.scrollback_inflight.is_empty(),
        "the gesture must have left a request outstanding for this to be about anything"
    );

    let _ = shell::daemon_sync::on_disconnected(&mut app);

    assert!(
        app.scrollback_inflight.is_empty(),
        "a range asked for on a dead connection is not on its way"
    );
}

// --- `010` BUG-020: a mutation whose daemon dies mid-flight ------------------------------

/// An app with a connected daemon, the add-worktree form open, and a create in flight — the
/// state the report's reproduction was in when the daemon was `kill -9`ed.
fn app_creating_a_worktree() -> (
    App,
    iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) {
    let (tx, rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    let project = PathBuf::from("/repo/demo");
    app.core.workspace.active = Some(project.clone());
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    shell::daemon_sync::send_worktree_create(
        &mut app,
        project,
        micold_core::naming::DerivedNames {
            dir_name: "feat-probe-two".into(),
            branch: "feat/probe-two".into(),
        },
        micold_core::worktree::CreateMode::NewBranch,
    );
    (app, rx)
}

fn form_status(app: &App) -> Option<micold_client::features::worktree_form::WorktreeFormStatus> {
    app.core.worktree_form.form.as_ref().map(|f| f.status)
}

fn form_notice(app: &App) -> Option<String> {
    app.core.worktree_form.form.as_ref()?.interrupted.clone()
}

/// The dialog must not keep claiming an operation is running on a connection that is gone.
///
/// `on_disconnected` already drains `pending_ops` and raises a notification per op. That is
/// invisible here: the add-worktree form is a modal, its scrim covers the surface notifications
/// are raised into, and the form's own pending state — `Creating`, with an indeterminate bar
/// that has no target to arrive at — is not touched by the drain. The reported symptom is that
/// bar animating at +90 s over a sidebar that had already reconciled correctly.
#[test]
fn a_create_whose_connection_drops_stops_claiming_to_be_running() {
    let (mut app, _rx) = app_creating_a_worktree();
    assert_eq!(
        form_status(&app),
        Some(micold_client::features::worktree_form::WorktreeFormStatus::Creating),
        "the fixture must have a create in flight for this to be about anything"
    );

    let _ = shell::daemon_sync::on_disconnected(&mut app);

    assert_eq!(
        form_status(&app),
        Some(micold_client::features::worktree_form::WorktreeFormStatus::Editing),
        "the create can never be answered on this connection, so the form must stop showing \
             it as in flight — an indeterminate bar with nothing in flight is a defect that \
             outlives the operation for ever (`010` BUG-020, FR-039d)"
    );
    let notice = form_notice(&app).unwrap_or_default();
    assert!(
        notice.contains("feat-probe-two"),
        "the notice must name the operation whose outcome is unknown, got {notice:?}"
    );
}

/// ...and the notice must outlive the refresh it points at.
///
/// The message's whole content is "the list is the authority now". The list arriving is what
/// makes it true, so a notice cleared by that arrival is a notice nobody can read: reconnect
/// follows the drop within a second or two, and `set_worktrees` reports `WorktreesReplaced`
/// unconditionally, which clears `worktree_error` (T067a-4). That clear is right for a create
/// *failure* shown against the old list and wrong for this, which is why the two are separate
/// fields rather than one.
#[test]
fn the_unknown_outcome_survives_the_list_refresh_it_points_at() {
    let (mut app, _rx) = app_creating_a_worktree();
    let _ = shell::daemon_sync::on_disconnected(&mut app);
    assert!(
        form_notice(&app).is_some(),
        "precondition: the notice is up"
    );

    // What reconnecting does: a fresh catalog replaces the worktree list.
    let outcomes = app.core.set_worktrees(vec![]);
    micold_client::app::drain(outcomes, |o| {
        micold_client::app::interpret(&mut app.core, o)
    });

    assert!(
        form_notice(&app).is_some(),
        "the refresh the notice points at cleared the notice — the user is left with a form \
             that silently stopped and no statement that the service went away mid-flight"
    );
}

/// The over-correction half: a disconnect with nothing in flight must not invent a notice.
///
/// An assertion that the form leaves `Creating` is satisfied just as well by a form that is
/// reset on every disconnect, which would throw away a half-filled form for a drop that had
/// nothing to do with it (FR-007 keeps the inputs even across a cancel).
#[test]
fn a_disconnect_with_no_create_in_flight_leaves_the_form_alone() {
    let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    app.core
        .update(Message::WorktreeForm(FormMsg::NameChanged("probe".into())));

    let _ = shell::daemon_sync::on_disconnected(&mut app);

    assert_eq!(
        form_status(&app),
        Some(micold_client::features::worktree_form::WorktreeFormStatus::Editing)
    );
    assert_eq!(
        form_notice(&app),
        None,
        "nothing was in flight, so there is no unknown outcome to report"
    );
    assert_eq!(
        app.core.worktree_form.form.as_ref().map(|f| f.name.clone()),
        Some("probe".to_string()),
        "a drop that interrupted nothing must not discard what the user had typed"
    );
}

/// With no form on screen there is no scrim, so the notification is the right place — and it
/// must still be raised. Routing every create through the form would lose the message for a
/// create whose form the user had already cancelled.
#[test]
fn a_create_with_no_form_open_is_still_reported() {
    let (mut app, _rx) = app_creating_a_worktree();
    app.core.worktree_form.form = None;
    assert!(
        app.core.notifications.queue.visible().is_none(),
        "nothing said yet"
    );

    let _ = shell::daemon_sync::on_disconnected(&mut app);

    let said = app
        .core
        .notifications
        .queue
        .visible()
        .map(|n| n.message.clone())
        .unwrap_or_default();
    assert!(
        said.contains("feat-probe-two"),
        "with no modal covering it, the unknown outcome must reach the user as a \
             notification naming the operation, got {said:?}"
    );
}

/// The correlation id of the create `app_creating_a_worktree` sent.
fn pending_create_req(app: &App) -> u64 {
    *app.pending_ops
        .iter()
        .find(|(_, op)| matches!(op, PendingOp::WorktreeCreate { .. }))
        .expect("the fixture must have a create pending")
        .0
}

/// `013` BUG-001, A1: with a create in flight, neither Escape nor the scrim closes the form.
///
/// The reported case, end to end through the window's own paths: `Message::EscapePressed` for
/// the key, and whatever `on_escape` hands the scrim for a click on it. Thirty seconds into a
/// submodule fetch, either used to take the progress display away and leave the outcome nowhere
/// to land (FR-010a).
#[test]
fn a_create_in_flight_holds_its_dialog_through_escape_and_the_scrim() {
    use micold_client::features::worktree_form::WorktreeFormStatus;
    let (mut app, _rx) = app_creating_a_worktree();

    let _ = update_inner(&mut app, Message::EscapePressed);
    assert_eq!(
        form_status(&app),
        Some(WorktreeFormStatus::Creating),
        "Escape closed the add-worktree dialog while its create was still running (FR-010a)"
    );

    let scrim = micold_client::app::on_escape(&app.core);
    assert_eq!(
        scrim, None,
        "the scrim must have nothing to send while a create runs (FR-010a)"
    );
    if let Some(message) = scrim {
        let _ = update_inner(&mut app, message);
    }
    assert_eq!(
        form_status(&app),
        Some(WorktreeFormStatus::Creating),
        "a click on the scrim closed the add-worktree dialog while its create was still running"
    );
}

/// `013` BUG-001, A2: Cancel closes the dialog mid-create, and the create's later failure still
/// reaches the user, as a notification carrying the daemon's message (FR-010b).
#[test]
fn a_create_failing_after_cancel_reaches_the_user_as_a_notification() {
    let (mut app, _rx) = app_creating_a_worktree();
    let req = pending_create_req(&app);

    let _ = update_inner(&mut app, Message::WorktreeForm(FormMsg::Cancelled));
    assert!(
        app.core.worktree_form.form.is_none(),
        "Cancel must close the dialog even while its create runs (FR-010a)"
    );

    let _ = shell::daemon_sync::on_daemon_event(
        &mut app,
        DaemonMsg::OperationError {
            req,
            kind: micold_core::protocol::messages::ErrorKind::GitFailed,
            message: "git failed to create the worktree".into(),
            detail: None,
        },
    );

    let said = app
        .core
        .notifications
        .queue
        .visible()
        .map(|n| n.message.clone())
        .unwrap_or_default();
    assert!(
        said.contains("git failed to create the worktree"),
        "a create that failed after its dialog was cancelled must still be reported, carrying \
         the daemon's message (FR-010b), got {said:?}"
    );
}

/// `013` BUG-001 follow-up: a success after Cancel is announced even with no project open.
///
/// The success arm derived the new worktree's path from `workspace.active`, so a create whose
/// project was closed while it ran produced no `Created` message at all: the announcement FR-010b
/// owes a cancelled create was dropped, and the record of it owed stayed behind for ever. The
/// pending op knows which project it was sent for, so nothing about the answer needs the open one.
#[test]
fn a_success_after_cancel_is_announced_even_with_no_project_open() {
    let (mut app, _rx) = app_creating_a_worktree();
    let req = pending_create_req(&app);
    let _ = update_inner(&mut app, Message::WorktreeForm(FormMsg::Cancelled));
    app.core.workspace.active = None;

    let _ = shell::daemon_sync::on_daemon_event(
        &mut app,
        DaemonMsg::OperationOk {
            req,
            result: micold_core::protocol::messages::OperationResult::WorktreeCreated {
                dir_name: "feat-probe-two".into(),
            },
        },
    );

    let said = app
        .core
        .notifications
        .queue
        .visible()
        .map(|n| n.message.clone())
        .unwrap_or_default();
    assert!(
        said.contains("feat-probe-two"),
        "a create that succeeded after its dialog was cancelled must be announced whether or not \
         a project is open when the answer lands (FR-010b), got {said:?}"
    );
}

/// A new attempt starts with a clean slate.
///
/// The stale error from the previous attempt stood through the whole of the next one's pending
/// state: an error line and an animating progress bar on screen together, describing different
/// attempts, with nothing to say which (`010` BUG-020, second note).
#[test]
fn a_new_attempt_does_not_show_the_previous_attempts_error() {
    let (mut app, _rx) = app_creating_a_worktree();
    app.core.update(Message::WorktreeForm(FormMsg::CreateFailed(
        "a worktree folder named 'feat-probe-two' already exists".into(),
    )));
    assert!(
        app.core.worktree_form.worktree_error.is_some(),
        "precondition: the first attempt left an error on screen"
    );

    shell::daemon_sync::send_worktree_create(
        &mut app,
        PathBuf::from("/repo/demo"),
        micold_core::naming::DerivedNames {
            dir_name: "feat-probe-three".into(),
            branch: "feat/probe-three".into(),
        },
        micold_core::worktree::CreateMode::NewBranch,
    );

    assert_eq!(
        app.core.worktree_form.worktree_error, None,
        "the error describes an attempt that is over; leaving it up beside the new attempt's \
             progress bar says two contradictory things about one form"
    );
}

#[test]
fn an_accepted_attach_ends_the_read_only_state_a_refusal_started() {
    // The reported sequence: this window is refused a project another window holds, the holder
    // later releases it, and the window reaches the project again by an ordinary switch — which
    // sends a *non-forced* `Attach`. The daemon accepts. Before the fix, `Attached` fell into
    // the catch-all arm, so the window rendered a project it owned while refusing to type into
    // it, above a takeover banner naming a window that may have exited.
    let project = PathBuf::from("/repo/demo");
    let mut app = app_on_project(&project);

    feed(
        &mut app,
        DaemonMsg::Refused {
            reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                project: project.clone(),
                holder: other_window(),
                since_secs: 12,
            },
        },
    );
    assert!(
        active_project_displaced(&app),
        "a ProjectBusy refusal must make the window read-only (FR-023)"
    );

    feed(
        &mut app,
        DaemonMsg::Attached {
            project: project.clone(),
            sessions: vec![],
        },
    );

    assert!(
        !active_project_displaced(&app),
        "an accepted attach means the daemon says we hold it — input must flow again (FR-024a)"
    );
    assert_eq!(
        connection_status(&app),
        micold_client::features::connection::ConnectionStatus::Connected,
        "and no takeover affordance may be offered for a project we hold (SC-021)"
    );
}

/// `010` BUG-023: the refusal and the takeover are one state and must not be one sentence.
///
/// Both leave the window read-only with the same take-over button, which is why the refusal was
/// folded onto `displaced` in the first place. What the fold also did was tell a window that had
/// merely been turned away that another window "took over this project" — the opposite event,
/// described in the past tense, about something that never happened to it. The distinction has
/// to survive as far as the banner, so the banner is chosen from the status and the status is
/// what this pins.
#[test]
fn a_refused_attach_reaches_the_banner_as_a_refusal_not_a_takeover() {
    use micold_client::features::connection::ConnectionStatus;

    let project = PathBuf::from("/repo/demo");

    let mut refused = app_on_project(&project);
    feed(
        &mut refused,
        DaemonMsg::Refused {
            reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                project: project.clone(),
                holder: other_window(),
                since_secs: 12,
            },
        },
    );

    let mut taken = app_on_project(&project);
    feed(
        &mut taken,
        DaemonMsg::Displaced {
            project: project.clone(),
            by: other_window(),
        },
    );

    assert_eq!(
        connection_status(&refused),
        ConnectionStatus::ProjectBusy {
            holder: other_window().to_string()
        },
        "nobody took anything from this window — it asked and was told no"
    );
    assert_eq!(
        connection_status(&taken),
        ConnectionStatus::Displaced {
            by: other_window().to_string()
        },
        "and the window that really did lose the project keeps the sentence that fits it"
    );
    assert!(
        active_project_displaced(&refused) && active_project_displaced(&taken),
        "the difference is in what the user is told, not in what they may do: both are \
             read-only and both are resolved by the same take-over"
    );
}

// --- `010` BUG-022: a window must not displace itself ---------------------------------------

/// The reported defect, end to end: one window, one project, no second window anywhere, and
/// the window goes read-only above a banner naming its own build.
///
/// The sequence is a reconnect. The keepalive declares the link dead, the outer loop dials
/// again, the new connection attaches — and the daemon, which has not yet noticed the old
/// socket is gone, does exactly what FR-024 says: it displaces the holder and tells it so.
/// Both connections feed one `App`, so that `Displaced` lands after the new connection's
/// `Attached`, which is the frame that clears the flag. The stale frame wins, and re-latches
/// read-only on a window that just successfully attached.
///
/// The pair is the point. A window that really did lose the project must still be told; what
/// must not happen is a window being told it lost the project to itself.
#[test]
fn a_window_is_not_displaced_by_its_own_reconnect_but_is_by_another_window() {
    use micold_client::features::connection::ConnectionStatus;

    let project = PathBuf::from("/repo/demo");

    let mut myself = app_on_project(&project);
    feed(
        &mut myself,
        DaemonMsg::Displaced {
            project: project.clone(),
            by: this_window(),
        },
    );

    let mut other = app_on_project(&project);
    feed(
        &mut other,
        DaemonMsg::Displaced {
            project: project.clone(),
            by: other_window(),
        },
    );

    assert_eq!(
        connection_status(&myself),
        ConnectionStatus::Connected,
        "the only window the user has must keep its project when its own reconnect \
             supersedes its own dead connection (BUG-022)"
    );
    assert!(
        !active_project_displaced(&myself),
        "and it must keep typing into it — input suppression is the harm, the banner is only \
             how the user finds out about it"
    );
    assert_eq!(
        connection_status(&other),
        ConnectionStatus::Displaced {
            by: other_window().to_string()
        },
        "two genuinely different windows of one build must still displace each other, which \
             is why the identity is compared and not the build string"
    );
}

/// The same collision arriving as a refusal rather than a displacement — and the reason the
/// fix is an identity and not a connection generation.
///
/// When the reconnect wins the race the other way round, the daemon still has the old
/// attachment and refuses the new connection's attach as `ProjectBusy`, naming the holder.
/// That frame is *timely*: it arrives on the current connection, in answer to a request this
/// window just made, so no staleness rule can discard it. Only the holder's identity says what
/// it is — this window's own corpse — and the window can then reclaim the project instead of
/// asking the user to take it over from themselves.
#[test]
fn a_refusal_by_this_windows_own_dead_connection_is_reclaimed_not_offered() {
    use micold_client::features::connection::ConnectionStatus;

    let project = PathBuf::from("/repo/demo");
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = app_on_project(&project);
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));

    feed(
        &mut app,
        DaemonMsg::Refused {
            reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                project: project.clone(),
                holder: this_window(),
                since_secs: 3,
            },
        },
    );

    match rx.try_recv() {
        Ok(ClientMsg::Attach { project: p, force }) => {
            assert_eq!(p, project);
            assert!(
                force,
                "a non-forced retry would be refused by the same dead connection forever"
            );
        }
        other => panic!("expected a forced re-attach, got {other:?}"),
    }
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::Connected,
        "and the user is never shown a take-over offer against themselves"
    );
}

/// The other half of that pair: a refusal naming a *different* window is still the user's
/// decision, and forcing it silently would be a takeover nobody confirmed (FR-023).
#[test]
fn a_refusal_by_another_window_is_still_offered_never_forced() {
    use micold_client::features::connection::ConnectionStatus;

    let project = PathBuf::from("/repo/demo");
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = app_on_project(&project);
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));

    feed(
        &mut app,
        DaemonMsg::Refused {
            reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                project: project.clone(),
                holder: other_window(),
                since_secs: 3,
            },
        },
    );

    assert!(
        rx.try_recv().is_err(),
        "nothing may be sent: taking a project from another window is a confirmed action"
    );
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::ProjectBusy {
            holder: other_window().to_string()
        },
        "the window is told, and offered the take-over it must ask for"
    );
}

#[test]
fn an_accepted_attach_ends_the_read_only_state_a_takeover_started() {
    // The same fix from the other direction: this window *was* displaced by a real takeover
    // rather than refused up front. Once the taker releases the project and this window's own
    // attach is accepted, it is writable again — via the ordinary attach path, with no need to
    // press "Take over" and force a displacement of nobody.
    let project = PathBuf::from("/repo/demo");
    let mut app = app_on_project(&project);

    feed(
        &mut app,
        DaemonMsg::Displaced {
            project: project.clone(),
            by: other_window(),
        },
    );
    assert!(
        active_project_displaced(&app),
        "a takeover makes us read-only (FR-024)"
    );

    feed(
        &mut app,
        DaemonMsg::Attached {
            project: project.clone(),
            sessions: vec![],
        },
    );
    assert!(!active_project_displaced(&app));
}

#[test]
fn an_accepted_attach_clears_only_that_project() {
    // The map is per-project and must stay that way: being handed back one project says nothing
    // about another that a different window still holds.
    let mine = PathBuf::from("/repo/mine");
    let theirs = PathBuf::from("/repo/theirs");
    let mut app = app_on_project(&mine);

    for p in [&mine, &theirs] {
        feed(
            &mut app,
            DaemonMsg::Displaced {
                project: p.clone(),
                by: other_window(),
            },
        );
    }

    feed(
        &mut app,
        DaemonMsg::Attached {
            project: mine.clone(),
            sessions: vec![],
        },
    );

    assert!(
        !app.displaced.contains_key(&mine),
        "the attached project is cleared"
    );
    assert!(
        app.displaced.contains_key(&theirs),
        "a project we did not attach to is untouched"
    );
}

#[test]
fn a_refusal_after_an_attach_makes_the_window_read_only_again() {
    // The flag must move in both directions, not just the new one. Clearing on `Attached` must
    // not make the state sticky the other way: if we are later refused — because another window
    // took the project while we were away — the banner comes back.
    let project = PathBuf::from("/repo/demo");
    let mut app = app_on_project(&project);

    feed(
        &mut app,
        DaemonMsg::Attached {
            project: project.clone(),
            sessions: vec![],
        },
    );
    assert!(!active_project_displaced(&app));

    feed(
        &mut app,
        DaemonMsg::Displaced {
            project: project.clone(),
            by: other_window(),
        },
    );
    assert!(
        active_project_displaced(&app),
        "the flag is a cache of daemon-reported ownership, not a latch in either direction"
    );
}

/// #368: the app every test starts from has no settings store, so no test that saves can write
/// the developer's own `settings.json`. The two save tests below did, on every client test run.
#[test]
fn the_test_app_cannot_reach_the_real_settings_file() {
    assert!(
        base_app().caps.settings().is_none(),
        "`base_app()` hands tests the real settings store; a save test would write the \
         developer's own settings.json (#368)"
    );
    // 029 T069: #368 dropped only the settings store, so the same `App` still carried a store over
    // the developer's own `projects.json`. Nothing wrote through it, which is why it went unnoticed
    // for as long as the settings one did — the file the reporter lost was the one a test did write.
    assert!(
        base_app().caps.projects().is_none(),
        "`base_app()` hands tests the real project catalog; a test that saves a project would \
         write the developer's own projects.json (029 T069)"
    );
}

/// T100 (BUG-003 follow-up, FR-012a/FR-012b): saving Settings while connected to a daemon must
/// ask it to apply the service-owned fields too — not just write `settings.json` locally — so
/// the change takes effect for that daemon's already-running sessions immediately, rather than
/// only after its next restart.
#[test]
fn settings_saved_sends_settings_set_to_a_connected_daemon() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    app.core.settings.settings_draft = Some(SettingsDraft {
        terminal: TerminalDraft {
            scrollback_lines: "20000".into(),
        },
        environment: EnvironmentDraft {
            enabled: false,
            script_path: "/tmp/does-not-exist.sh".into(),
            timeout_secs: "15".into(),
            default_ai_cli: AiCli::Copilot,
            pi_activity_component: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
        },
        ..SettingsDraft::default()
    });

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    match rx.try_recv() {
        Ok(ClientMsg::SettingsSet {
            scrollback_lines,
            env_include_enabled,
            env_include_script_path,
            env_include_timeout_secs,
            ..
        }) => {
            assert_eq!(scrollback_lines, Some(20_000));
            assert_eq!(env_include_enabled, Some(false));
            assert_eq!(
                env_include_script_path,
                Some("/tmp/does-not-exist.sh".to_string())
            );
            assert_eq!(env_include_timeout_secs, Some(15));
        }
        other => panic!("expected a queued SettingsSet, got {other:?}"),
    }
    // Exactly one — a save must not double-send.
    assert!(rx.try_recv().is_err(), "no second message queued");
}

/// U215 (feature 034, FR-004): turning "Let AI sessions manage worktrees and sessions" off and
/// saving tells the connected service, which is what reads it at the next spawn.
#[test]
fn turning_the_binding_toggle_off_and_saving_tells_the_service() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::ToolServerToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            tool_server_enabled,
            ..
        } => Some(*tool_server_enabled),
        _ => None,
    });
    assert_eq!(
        told,
        Some(Some(false)),
        "the service must be told the binding is off: {sent:?}"
    );
}

/// U214 (feature 034, FR-004): the page shows what the service says is in force, so a toggle
/// another window turned off opens off here.
#[test]
fn the_binding_toggle_opens_with_the_value_the_service_reported() {
    let mut app = base_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                tool_server_enabled: false,
                ..quiet_settings()
            },
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));

    assert!(
        !app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("the page is open")
            .environment
            .tool_server_enabled,
        "the service said the binding is off; the page must not show it on"
    );
}

/// U142 (feature 039, FR-026, US4 scenario 1): the page opens with **Desktop notifications** on,
/// and the switch's message changes the draft.
#[test]
fn the_desktop_notifications_message_changes_the_draft() {
    let mut app = base_app();
    // What a connected service reports on default settings (`settings_wire` of the defaults).
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let shown = |app: &App| {
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("the page is open")
            .environment
            .desktop_notifications
    };
    assert!(
        shown(&app),
        "on default settings the switch is shown and it is on"
    );

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::DesktopNotificationsToggled(false)),
    );
    assert!(!shown(&app), "the switch was turned off");

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::DesktopNotificationsToggled(true)),
    );
    assert!(shown(&app), "and on again");
}

/// U143 (feature 039, FR-027): turning **Desktop notifications** off and saving tells the
/// connected service, which is what refuses the claims of every window.
#[test]
fn turning_desktop_notifications_off_and_saving_tells_the_service() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::DesktopNotificationsToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            desktop_notifications,
            ..
        } => Some(*desktop_notifications),
        _ => None,
    });
    assert_eq!(
        told,
        Some(Some(false)),
        "the service must be told desktop notifications are off: {sent:?}"
    );
}

/// U143: a save that leaves the switch on names it too, as `Some(true)`: the form holds the
/// field, so every save sends its value.
#[test]
fn saving_with_desktop_notifications_on_tells_the_service_they_are_on() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            desktop_notifications,
            ..
        } => Some(*desktop_notifications),
        _ => None,
    });
    assert_eq!(
        told,
        Some(Some(true)),
        "the service must be told the switch's position: {sent:?}"
    );
}

/// U144 (feature 039, FR-027, US4 scenario 3): the page shows what the service says is in force,
/// so a switch another window turned off opens off here, and on again after it is turned on.
#[test]
fn the_desktop_notifications_switch_opens_with_the_value_the_service_reported() {
    let mut app = base_app();
    for reported in [false, true] {
        feed(
            &mut app,
            DaemonMsg::SettingsChanged {
                settings: DaemonSettings {
                    desktop_notifications: reported,
                    ..quiet_settings()
                },
            },
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));

        assert_eq!(
            app.core
                .settings
                .settings_draft
                .as_ref()
                .expect("the page is open")
                .environment
                .desktop_notifications,
            reported,
            "the page must show the switch as the service reported it"
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Cancelled));
    }
}

/// U217 (feature 034, FR-016): choosing a value for "Let agents read and type into other
/// sessions" and saving tells the connected service, which reads it on every tool request.
#[test]
fn choosing_a_cross_session_value_and_saving_tells_the_service() {
    use micold_core::mcp::policy::CrossSessionAccess;

    for chosen in [
        CrossSessionAccess::ConfirmEachSend,
        CrossSessionAccess::Off,
        CrossSessionAccess::Auto,
    ] {
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        let mut app = base_app();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
        let _ = update_inner(
            &mut app,
            Message::Settings(SettingsMsg::CrossSessionAccessChanged(chosen)),
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

        let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        let told = sent.iter().find_map(|msg| match msg {
            ClientMsg::SettingsSet {
                cross_session_access,
                ..
            } => Some(*cross_session_access),
            _ => None,
        });
        assert_eq!(
            told,
            Some(Some(chosen)),
            "the service must be told the chosen value: {sent:?}"
        );
        assert_eq!(
            app.core.session.cross_session_access, chosen,
            "and this window's own copy follows the save"
        );
    }
}

/// U216 (feature 034, FR-016): the page shows what the service says is in force, so a value
/// another window chose opens here.
#[test]
fn the_cross_session_select_opens_with_the_value_the_service_reported() {
    use micold_core::mcp::policy::CrossSessionAccess;

    let mut app = base_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                cross_session_access: CrossSessionAccess::Off,
                ..quiet_settings()
            },
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));

    assert_eq!(
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("the page is open")
            .environment
            .cross_session_access,
        CrossSessionAccess::Off,
        "the service said Off; the page must not show Auto"
    );
}

/// The disconnected case is not an error: settings-saving already has a fully working local-only
/// path (the direct `settings.json` write above the daemon-send in
/// `Message::Settings(SettingsMsg::Saved)`), so
/// there is nothing to notify the user about — unlike every other `send_op`-routed mutation, which
/// has no such standalone path.
#[test]
fn settings_saved_is_a_silent_no_op_toward_the_daemon_when_disconnected() {
    let mut app = base_app();
    assert!(app.daemon.is_none());
    app.core.settings.settings_draft = Some(SettingsDraft {
        terminal: TerminalDraft {
            scrollback_lines: "20000".into(),
        },
        environment: EnvironmentDraft {
            enabled: true,
            script_path: String::new(),
            timeout_secs: "15".into(),
            default_ai_cli: AiCli::ClaudeCode,
            pi_activity_component: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
        },
        ..SettingsDraft::default()
    });

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert_eq!(
        app.scrollback_lines, 20_000,
        "the local field still updates"
    );
    assert!(app.pending_ops.is_empty(), "nothing was queued to send");
}

// --- BUG-003 (T162, FR-032a/FR-032b/FR-033a): a saved placement has to move the service -----

/// A `App` with the Settings view open on a valid draft that chooses `chosen`, the service
/// running where `in_force` says, and **no settings store** — the last so that "the save wrote
/// nothing" is a claim about this process rather than about the developer's own
/// `settings.json`. `Capabilities::settings` is documented as absent when no data directory
/// resolves, so this is a configuration the shell already has to handle.
fn app_saving_a_placement(in_force: PlacementKind, chosen: PlacementKind) -> App {
    let mut app = base_app();
    app.placement.kind = in_force;
    app.core.settings.placement_in_force = in_force;
    app.core.settings.settings_draft = Some(SettingsDraft {
        terminal: TerminalDraft {
            scrollback_lines: "20000".into(),
        },
        // A default `EnvironmentDraft` holds an *empty* timeout, which `validate` rejects —
        // and a save that never validates would pass the assertions below for the wrong
        // reason, reporting "nothing was applied" about a form that was simply never saved.
        environment: EnvironmentDraft {
            enabled: false,
            script_path: String::new(),
            timeout_secs: "5".into(),
            default_ai_cli: AiCli::ClaudeCode,
            pi_activity_component: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
        },
        daemon: micold_client::features::settings::DaemonDraft {
            placement: chosen,
            ..Default::default()
        },
        ..SettingsDraft::default()
    });
    app
}

/// FR-032a, FR-032b. The confirmation gates the *whole* save: until it is answered nothing is
/// applied — not the placement, not the scrollback the user changed in another section, and
/// nothing is sent to the daemon. The bug this replaces applied all of it and asked nothing.
#[test]
fn saving_a_changed_placement_applies_nothing_until_it_is_confirmed() {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = app_saving_a_placement(PlacementKind::HostProcess, PlacementKind::LocalSandbox);
    app.daemon = Some(micold_client::daemon::Outbox::new(tx));

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert_eq!(
        app.core.settings.pending_placement,
        Some(micold_client::features::settings::PendingPlacementChange {
            from: PlacementKind::HostProcess,
            to: PlacementKind::LocalSandbox,
        }),
        "the save has to raise the confirmation FR-032 asks for"
    );
    assert_eq!(
        app.placement.kind,
        PlacementKind::HostProcess,
        "the connection was re-dialled in the new placement before the user answered"
    );
    assert_eq!(
        app.scrollback_lines,
        micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        "the rest of the save was applied while the question was still open (FR-032b)"
    );
    assert!(
        rx.try_recv().is_err(),
        "the daemon was told about a save the user has not agreed to"
    );
    assert!(
        app.core.settings.settings_draft.is_some(),
        "the view closed behind the dialog, taking the draft with it"
    );
}

/// FR-033a. Confirming applies the change to the *running* application: the placement the
/// connection subscription dials from moves, the sandbox goes back to the start of its
/// lifecycle, and the bring-up plan a container needs is built from the settings just saved.
/// Without the plan there is nothing to restart — which is why re-sending `RestartRequested`
/// could never have fixed this on its own.
#[test]
fn confirming_a_placement_change_moves_the_running_service() {
    let mut app = app_saving_a_placement(PlacementKind::HostProcess, PlacementKind::LocalSandbox);
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PlacementChangeConfirmed),
    );

    assert_eq!(
        app.placement.kind,
        PlacementKind::LocalSandbox,
        "`daemon::connection` dials from `app.placement`, and its identity is what tears the \
             old connection down — leaving it is the whole of BUG-003 (FR-033a)"
    );
    assert_eq!(
        app.core.settings.placement_in_force,
        PlacementKind::LocalSandbox,
        "the note under the select reports this, so it has to follow the move (FR-035b)"
    );
    assert!(
        app.sandbox_boot.is_some(),
        "a container placement with no boot plan cannot be brought up or restarted (R9)"
    );
    assert_eq!(
        app.scrollback_lines, 20_000,
        "confirming performs the save that was deferred, not just the move"
    );
    assert!(
        app.core.settings.settings_draft.is_none(),
        "the save went through, so the view closes as it does for any other save"
    );
}

/// The way back out. Moving to the host has no bring-up of its own — the connection actor
/// spawns a host process when it cannot reach one — so the plan is dropped rather than kept:
/// a stale plan is what `check_alive` would go on polling a container nobody asked for.
#[test]
fn confirming_the_move_back_to_the_host_drops_the_container_plan() {
    let mut app = app_saving_a_placement(PlacementKind::LocalSandbox, PlacementKind::HostProcess);
    app.sandbox_boot = Some(crate::shell::sandbox::BootPlan {
        profile: Default::default(),
        state_dir: PathBuf::from("/tmp/state"),
        projects: Vec::new(),
    });
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PlacementChangeConfirmed),
    );

    assert_eq!(app.placement.kind, PlacementKind::HostProcess);
    assert!(
        app.sandbox_boot.is_none(),
        "the plan for a container this app is no longer running outlived the move"
    );
    assert_eq!(
        app.sandbox.state,
        micold_core::sandbox::lifecycle::SandboxState::Disabled,
        "the sandbox banner still describes a container that is not where sessions run"
    );
}

/// FR-032b at the shell: declining leaves the process exactly as it was, with the draft intact
/// so the user can change their mind about the one field they were asked about.
#[test]
fn declining_the_confirmation_leaves_the_process_untouched() {
    let mut app = app_saving_a_placement(PlacementKind::HostProcess, PlacementKind::LocalSandbox);
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PlacementChangeCancelled),
    );

    assert_eq!(app.placement.kind, PlacementKind::HostProcess);
    assert_eq!(
        app.scrollback_lines,
        micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        "a declined save applied part of itself anyway (FR-032b)"
    );
    assert!(app.core.settings.settings_draft.is_some());
}

/// FR-032a. A save that leaves the placement where it is must not ask, and must not restart
/// anything — it is the ordinary save every other setting goes through.
#[test]
fn a_save_that_keeps_the_placement_neither_asks_nor_restarts() {
    let mut app = app_saving_a_placement(PlacementKind::HostProcess, PlacementKind::HostProcess);

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert!(
        app.core.settings.pending_placement.is_none(),
        "saving a scrollback size put a dialog about the session service on screen"
    );
    assert_eq!(
        app.scrollback_lines, 20_000,
        "the save went straight through"
    );
    assert!(
        app.core.settings.settings_draft.is_none(),
        "the view closes, as it does for any save that needs nothing confirmed"
    );
}

/// T100: a fresh connect (or reconnect) must adopt the daemon's authoritative env-include
/// settings too, not just scrollback — the daemon is the single source of truth for both
/// (FR-012a/FR-012b), and this client's own boot-time local read may already be stale relative
/// to it (e.g. another window changed a setting first).
#[test]
fn daemon_connected_adopts_the_authoritative_env_include_settings() {
    let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.env_include_enabled = true;
    app.env_include_script_path = "/tmp/stale-local-path.sh".into();
    app.env_include_timeout_secs = 10;

    let _ = update_inner(
        &mut app,
        Message::Connection(ConnectionMsg::Connected {
            outbox: micold_client::daemon::Outbox::new(tx),
            catalog: snapshot_with("/repo/demo", Vec::new()),
            settings: micold_core::protocol::messages::DaemonSettings {
                default_ai_cli: AiCli::ClaudeCode,
                scrollback_lines: 12_345,
                env_include_enabled: false,
                env_include_script_path: "/authoritative/from-daemon.sh".into(),
                env_include_timeout_secs: 30,
                pi_activity_component: true,
                tool_server_enabled: true,
                desktop_notifications: true,
                diff_layout: Default::default(),
                cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                pr_status_enabled: false,
            },
        }),
    );

    assert_eq!(app.scrollback_lines, 12_345);
    assert!(!app.env_include_enabled);
    assert_eq!(app.env_include_script_path, "/authoritative/from-daemon.sh");
    assert_eq!(app.env_include_timeout_secs, 30);
}

/// T100: a `SettingsChanged` push (this client's own `SettingsSet` echoed back, or another
/// window's) must sync every service-owned field, not just scrollback — the whole point of
/// sending `SettingsSet` at all is that the change takes effect without a restart.
#[test]
fn settings_changed_event_syncs_env_include_fields() {
    let mut app = base_app();
    app.env_include_enabled = true;
    app.env_include_script_path = "/tmp/before.sh".into();
    app.env_include_timeout_secs = 10;

    let _ = update_inner(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::SettingsChanged {
            settings: micold_core::protocol::messages::DaemonSettings {
                default_ai_cli: AiCli::ClaudeCode,
                scrollback_lines: 5_000,
                env_include_enabled: false,
                env_include_script_path: "/tmp/after.sh".into(),
                env_include_timeout_secs: 45,
                pi_activity_component: true,
                tool_server_enabled: true,
                desktop_notifications: true,
                diff_layout: Default::default(),
                cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                pr_status_enabled: false,
            },
        })),
    );

    assert_eq!(app.scrollback_lines, 5_000);
    assert!(!app.env_include_enabled);
    assert_eq!(app.env_include_script_path, "/tmp/after.sh");
    assert_eq!(app.env_include_timeout_secs, 45);
}

#[test]
fn connection_status_orders_mismatch_over_displaced_over_disconnected() {
    // `connection_status` is decision/branching logic (Constitution I) picking which of five
    // mutually-possible states wins — pins the precedence directly rather than relying on it
    // only being exercised incidentally elsewhere (convergence finding F1, BUG-002).
    use micold_client::features::connection::ConnectionStatus;

    let mut app = App {
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
        activation: Default::default(),
        last_grid: None,
        env_include_enabled: micold_core::settings::DEFAULT_ENV_INCLUDE_ENABLED,
        env_include_script_path: String::new(),
        env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
        env_include_cache: HashMap::new(),
        env_include_last_outcome: EnvIncludeOutcome::Disabled,
        daemon: None,
        daemon_catalog: None,
        displaced: HashMap::new(),
        disconnected: false,
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: None,
        build_mismatch: None,
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        probe: None,
        scene_ready: false,
        scene_frames: 0,
        ripples_animating: Arc::new(AtomicUsize::new(0)),
        scene_ripple_frames: std::cell::Cell::new(0),
    };

    assert_eq!(connection_status(&app), ConnectionStatus::Connected);

    app.disconnected = true;
    assert_eq!(connection_status(&app), ConnectionStatus::Disconnected);

    let project = PathBuf::from("/repo/demo");
    app.core.workspace.active = Some(project.clone());
    app.displaced.insert(
        project.clone(),
        micold_client::features::connection::Hold::taken_over(other_window().to_string()),
    );
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::Displaced {
            by: other_window().to_string()
        },
        "a takeover must win over a plain disconnect"
    );

    app.build_mismatch = Some(("client-1".into(), "daemon-0".into()));
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::BuildMismatch {
            client_build: "client-1".into(),
            daemon_build: "daemon-0".into(),
        },
        "a same-contract build mismatch must win over a takeover"
    );

    app.version_mismatch = Some((2, 1, "daemon-0".into()));
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::VersionMismatch {
            client: 2,
            daemon: 1,
            daemon_build: "daemon-0".into(),
        },
        "a wire-contract mismatch must win over a same-contract build mismatch"
    );
}

// --- BUG-005 (T168, FR-002a/FR-036b): a sandbox the client finds absent is brought up --------

/// A refused dial, and the work the application scheduled in answer to it.
fn connection_failed(app: &mut App) -> Task<Message> {
    update_inner(
        app,
        Message::Connection(ConnectionMsg::ConnectFailed(
            "Connection refused (os error 111)".into(),
        )),
    )
}

/// Whether the refused dial reached the user as an error. By level, not wording: the sentence
/// is the handler's to rephrase.
fn a_connection_failure_was_reported(app: &App) -> bool {
    app.core
        .notifications
        .queue
        .visible()
        .is_some_and(|n| n.level == micold_core::notify::Level::Error)
}

/// Whether nothing at all was raised, on screen or waiting. The negative of "reported" is not
/// "not visible": a report queued behind another notice would pass for silence.
fn nothing_was_reported(app: &App) -> bool {
    let queue = &app.core.notifications.queue;
    queue.visible().is_none() && queue.pending() == 0
}

/// Every message the work an update returned produces, running it to its end. A bring-up run
/// here reaches a recording runner (`BringUp::task`), never the host's container runtime.
fn messages(work: Task<Message>) -> Vec<Message> {
    use iced::futures::StreamExt;
    let Some(stream) = iced_runtime::task::into_stream(work) else {
        return Vec::new();
    };
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    runtime.block_on(async {
        let outputs = stream
            .filter_map(|action| {
                std::future::ready(match action {
                    iced_runtime::Action::Output(message) => Some(message),
                    _ => None,
                })
            })
            .collect::<Vec<_>>();
        tokio::time::timeout(std::time::Duration::from_secs(10), outputs)
            .await
            .expect("the work an update returned has to finish")
    })
}

/// Whether `messages` is a bring-up reported whole: the first stage it enters, then how it ended.
fn reports_the_bring_up(messages: &[Message]) -> bool {
    matches!(
        messages.first(),
        Some(Message::Sandbox(SandboxMsg::Progress(stage)))
            if **stage == micold_core::sandbox::lifecycle::SandboxState::Probing
    ) && matches!(
        messages.last(),
        Some(Message::Sandbox(
            SandboxMsg::Started(_) | SandboxMsg::Failed(_)
        ))
    )
}

/// The failed sandbox, with its boot plan's state under `state_dir`: running a bring-up writes a
/// token there.
fn app_with_a_failed_sandbox_in(state_dir: &std::path::Path) -> App {
    let mut app = app_with_a_failed_sandbox();
    app.sandbox_boot
        .as_mut()
        .expect("setup: the failed sandbox has a boot plan")
        .state_dir = state_dir.to_path_buf();
    app
}

/// The report itself: switched to the sandbox, the bring-up failed once (the runtime was busy,
/// the port was a moment late), and from then on every dial was refused and nothing but a
/// restart the user did not know they needed could bring the daemon up.
#[test]
fn a_failed_sandbox_the_client_cannot_reach_is_brought_up_again() {
    let state_dir = tempfile::tempdir().expect("tempdir");
    let mut app = app_with_a_failed_sandbox_in(state_dir.path());

    let work = connection_failed(&mut app);

    assert_eq!(
        shell::sandbox::BringUp::scheduled().len(),
        1,
        "the state says a bring-up started, so one has to be scheduled — `Probing` with nothing \
             running is BUG-005 with a different banner"
    );
    assert_eq!(
        work.units(),
        1,
        "the bring-up has to be handed back to run, not built and dropped"
    );
    let reported = messages(work);
    assert!(
        reports_the_bring_up(&reported),
        "the work handed back has to report the bring-up's first stage and how it ended — one \
             whose messages are discarded starts a sandbox the view never hears of, and one that \
             drops its ending never records a failure or counts the attempt, which is BUG-005: \
             {reported:?}"
    );
    assert_eq!(
        app.sandbox.state,
        micold_core::sandbox::lifecycle::SandboxState::Probing,
        "a refused dial to a sandbox that is not running has to start one — waiting for the \
             user to find Restart is BUG-005"
    );
    assert!(
        nothing_was_reported(&app),
        "a bring-up the application has started is progress, not a failure to report (FR-036b)"
    );
}

/// S-6's spacing where it is decided: the first unattended bring-up starts at once, and the one
/// after a failed attempt waits the delay the budget gave it — not zero, which puts every attempt
/// on the next connection retry and spends the bound in seconds.
#[test]
fn the_bring_up_after_a_failed_attempt_waits_the_budgets_delay() {
    use micold_core::sandbox::lifecycle::UNATTENDED_BRING_UP_DELAYS;
    let mut app = app_with_a_failed_sandbox();
    let reason = "Connection refused (os error 111)";

    let first = shell::daemon_sync::refused_dial(&mut app, reason)
        .expect("a failed sandbox the client cannot reach is brought up");
    let micold_core::sandbox::lifecycle::SandboxState::Failed(failure) = failed_sandbox_state()
    else {
        unreachable!("the fixture's sandbox is failed")
    };
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Failed(Box::new(failure))),
    );
    let second = shell::daemon_sync::refused_dial(&mut app, reason)
        .expect("a failed attempt with attempts left is brought up again");

    assert_eq!(
        [first.after, second.after],
        [UNATTENDED_BRING_UP_DELAYS[0], UNATTENDED_BRING_UP_DELAYS[1]],
        "each unattended bring-up waits the delay the budget gave it (S-6)"
    );
}

/// One state from each stage of a bring-up in flight: the states `is_coming_up` is true for.
fn bring_up_stages() -> [micold_core::sandbox::lifecycle::SandboxState; 3] {
    use micold_core::sandbox::lifecycle::SandboxState;
    [
        SandboxState::Probing,
        SandboxState::Acquiring(micold_core::sandbox::runtime::Progress {
            stage: "Downloading".into(),
            detail: None,
            percent: Some(40),
        }),
        SandboxState::Starting,
    ]
}

#[test]
fn a_refused_dial_during_a_bring_up_is_not_reported_as_a_failure() {
    let in_flight = bring_up_stages();

    for stage in in_flight {
        let mut app = app_with_a_failed_sandbox();
        app.sandbox.state = stage.clone();

        let work = connection_failed(&mut app);

        assert_eq!(
            work.units(),
            0,
            "{stage:?}: the bring-up in flight is the only one there may be (S-6)"
        );
        assert_eq!(
            app.sandbox.state, stage,
            "a bring-up already under way must not be started a second time"
        );
        assert!(
            nothing_was_reported(&app),
            "{stage:?}: the daemon is not listening *yet*; saying it could not be reached reads \
                 a working bring-up as a broken one (FR-036b)"
        );
    }
}

/// FR-036b at the banner: a service that is not listening because the application is bringing
/// it up is not a lost connection. "Not connected to the session service … Reconnecting…" above a
/// sandbox view showing the stage reads a working bring-up as a broken one — the toast this
/// replaced said the same thing more quietly.
#[test]
fn a_bring_up_in_flight_is_not_shown_as_a_lost_connection() {
    use micold_client::features::connection::ConnectionStatus;
    use micold_core::sandbox::lifecycle::SandboxState;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    assert_eq!(
        app.sandbox.state,
        SandboxState::Probing,
        "setup: the refused dial has to start a bring-up"
    );
    let in_flight = bring_up_stages();

    for stage in in_flight {
        app.sandbox.state = stage.clone();
        let _ = connection_failed(&mut app);

        assert_ne!(
            connection_status(&app),
            ConnectionStatus::Disconnected,
            "{stage:?}: the service is being brought up, and the sandbox view says so (FR-036b)"
        );
    }
}

/// FR-036b and FR-035a together: while the application is bringing the sandbox up there is no
/// failure to stand behind, so neither "The sandbox did not start" nor an offer to run without it
/// may be on screen. Offering the host fallback over a bring-up that is about to succeed asks the
/// user to give up isolation for nothing.
#[test]
fn a_bring_up_in_flight_offers_no_failure_card_and_no_fallback() {
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let in_flight = bring_up_stages();

    for stage in in_flight {
        app.sandbox.state = stage.clone();
        let _ = connection_failed(&mut app);

        assert_eq!(
            app.sandbox.persistent_notice(),
            None,
            "{stage:?}: the standing sandbox card is for a sandbox that did not start (FR-036b)"
        );
        assert_eq!(
            app.sandbox.fallback_offer(),
            None,
            "{stage:?}: the host fallback is offered for a failure, not a bring-up (FR-035a)"
        );
    }
}

/// FR-036b where T178's visual pass caught it (finding 1): the liveness check finds the container
/// stopped, and for the two seconds until a refused dial started the bring-up the screen said "The
/// sandbox did not start" and offered to run without it. The application is about to bring the
/// sandbox back on its own, so there is no failure to stand behind yet.
#[test]
fn a_container_found_stopped_is_brought_up_without_showing_a_failure() {
    use micold_client::features::connection::ConnectionStatus;
    use micold_core::sandbox::runtime::ContainerId;
    let state_dir = tempfile::tempdir().expect("tempdir");
    let mut app = app_with_a_failed_sandbox_in(state_dir.path());
    app.sandbox.state =
        micold_core::sandbox::lifecycle::SandboxState::Running(ContainerId("abc".into()));
    let _ = update_inner(&mut app, Message::Connection(ConnectionMsg::Disconnected));

    let work = update_inner(&mut app, Message::Sandbox(SandboxMsg::Lost));

    assert_eq!(
        shell::sandbox::BringUp::scheduled().len(),
        1,
        "a stopped container with attempts left is brought up at once (FR-036a)"
    );
    assert_eq!(
        work.units(),
        1,
        "the bring-up has to be handed back to run — one built and dropped leaves `Probing` with \
             nothing running, which is BUG-005"
    );
    let reported = messages(work);
    assert!(
        reports_the_bring_up(&reported),
        "the work handed back has to report the bring-up's first stage and how it ended — one \
             that stops short leaves the view on `Probing` and a failed restart unrecorded, which \
             is BUG-005: {reported:?}"
    );
    assert_eq!(
        app.sandbox.persistent_notice(),
        None,
        "the standing sandbox card is for a sandbox that did not start (FR-036b)"
    );
    assert_eq!(
        app.sandbox.fallback_offer(),
        None,
        "the host fallback is offered for a failure, not a bring-up (FR-035a)"
    );
    assert_ne!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "the service is being brought up, and the sandbox view says so (FR-036b)"
    );
}

/// What a bring-up reports when it has started the container.
fn a_started_sandbox() -> micold_core::sandbox::lifecycle::Started {
    use micold_core::sandbox::runtime::{
        ContainerId, IdentityMapping, LimitSupport, RuntimeCapabilities, RuntimeKind,
    };
    micold_core::sandbox::lifecycle::Started {
        id: ContainerId("abc".into()),
        capabilities: RuntimeCapabilities {
            kind: RuntimeKind::Docker,
            version: "27.0".into(),
            cpus: LimitSupport::Supported,
            memory: LimitSupport::Supported,
            pids: LimitSupport::Supported,
            storage: LimitSupport::Supported,
            identity_mapping: IdentityMapping::ExplicitUidGid,
        },
        unsatisfiable: Vec::new(),
        mounted: None,
    }
}

/// The message that bring-up sends when it succeeds: the container, and what it shares.
fn a_started_sandbox_message() -> SandboxMsg {
    SandboxMsg::Started(Box::new((
        a_started_sandbox(),
        micold_client::features::sandbox::SandboxLocations::default(),
    )))
}

/// FR-036b where T178's visual pass caught it (finding 2): `Started` means the container is up,
/// not that the service inside it is listening yet. For the half-second between the two the
/// banner said "Not connected to the session service" about a service the application had just
/// started — at first enable and on every recovery.
#[test]
fn a_started_sandbox_whose_service_has_not_answered_yet_is_not_a_lost_connection() {
    use micold_client::features::connection::ConnectionStatus;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);

    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));

    assert_ne!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "the service was started a moment ago and has not been dialled since (FR-036b)"
    );
}

/// The daemon answered: the connection is up, with an empty catalog and default settings.
/// Empty, like the boot plan's project set: a catalog sharing other projects marks the sandbox
/// `Stale` on connect (see `the_service_answers_with`).
fn the_service_answers(app: &mut App) {
    the_service_answers_with(
        app,
        micold_core::protocol::messages::CatalogSnapshot::default(),
    );
}

/// The daemon answered with `catalog`, and default settings.
fn the_service_answers_with(
    app: &mut App,
    catalog: micold_core::protocol::messages::CatalogSnapshot,
) {
    let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
    let _ = update_inner(
        app,
        Message::Connection(ConnectionMsg::Connected {
            outbox: micold_client::daemon::Outbox::new(tx),
            catalog,
            settings: micold_core::protocol::messages::DaemonSettings {
                default_ai_cli: AiCli::ClaudeCode,
                scrollback_lines: 10_000,
                env_include_enabled: false,
                env_include_script_path: String::new(),
                env_include_timeout_secs: 30,
                pi_activity_component: false,
                tool_server_enabled: true,
                desktop_notifications: true,
                diff_layout: Default::default(),
                cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                pr_status_enabled: false,
            },
        }),
    );
}

/// The other side of the grace above: once the started service has answered, the bring-up is
/// over, and losing that connection is a lost connection like any other (FR-027) until the
/// liveness check says the container went with it.
#[test]
fn a_started_service_that_answered_and_went_away_is_a_lost_connection() {
    use micold_client::features::connection::ConnectionStatus;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));
    the_service_answers(&mut app);
    assert!(
        matches!(
            app.sandbox.state,
            micold_core::sandbox::lifecycle::SandboxState::Running(_)
        ),
        "setup: the sandbox that answered is running, not marked stale by a catalog that differs \
             from the plan"
    );

    let _ = update_inner(&mut app, Message::Connection(ConnectionMsg::Disconnected));

    assert_eq!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "a service that answered is no longer coming up; its disconnect is reported (FR-027)"
    );
}

/// `Stale` reached the other way: the service answered with a catalog whose projects differ from
/// the ones the container was started with, so the mount set is out of date. That answer still
/// ends the grace `Started` opened — being out of date is not being on the way up.
#[test]
fn a_service_that_answered_with_a_changed_mount_set_and_went_away_is_a_lost_connection() {
    use micold_client::features::connection::ConnectionStatus;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));
    the_service_answers_with(&mut app, snapshot_with("/repo/demo", Vec::new()));
    assert!(
        matches!(
            app.sandbox.state,
            micold_core::sandbox::lifecycle::SandboxState::Stale(_)
        ),
        "setup: a catalog sharing a project the container was not started with marks it stale"
    );

    let _ = update_inner(&mut app, Message::Connection(ConnectionMsg::Disconnected));

    assert_eq!(
            connection_status(&app),
            ConnectionStatus::Disconnected,
            "an out-of-date service that answered is not coming up; its disconnect is reported (FR-027)"
        );
}

/// The grace is bounded: a container that is up with a service inside it that never listens is
/// a failure, and hiding the banner for as long as the dials are refused would hide it for ever.
/// A second refused dial after `Started` is past the one reconnect a daemon takes to listen.
#[test]
fn a_started_service_that_keeps_refusing_is_reported() {
    use micold_client::features::connection::ConnectionStatus;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));

    let _ = connection_failed(&mut app);
    let _ = connection_failed(&mut app);

    assert_eq!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "a service still refusing after its grace is not coming up any more (FR-027)"
    );
    assert!(
        a_connection_failure_was_reported(&app),
        "a started service that never listens is reported, not waited on for ever"
    );
}

/// Where the bound sits: the container is up before the daemon inside it listens, and on a slow
/// machine the reconnect after `Started` lands in that gap. One refused dial there is the
/// bring-up still finishing, and is neither reported nor shown as a lost connection (FR-036b).
#[test]
fn the_first_refused_dial_after_start_is_the_service_still_starting() {
    use micold_client::features::connection::ConnectionStatus;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));

    let _ = connection_failed(&mut app);

    assert_ne!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "one reconnect is how long a started daemon may take to listen (FR-036b)"
    );
    assert!(
        nothing_was_reported(&app),
        "a service still starting is progress, not a failure to report (FR-036b)"
    );
}

/// The grace is for a service that is not listening yet. A daemon that answered and refused — a
/// stale `:dev` image, say — is listening, and what it said is the one thing the user has to act
/// on: it is reported on that dial, not one backoff later (#370).
#[test]
fn a_refusal_right_after_start_is_reported_at_once() {
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));

    let _ = update_inner(
        &mut app,
        Message::Connection(ConnectionMsg::Refused(
            "the sandbox is running `micold-daemon:dev`, built from a different working tree"
                .into(),
        )),
    );

    assert!(
        a_connection_failure_was_reported(&app),
        "a daemon that refused is not a service still starting (#370)"
    );
}

/// The grace belongs to a started sandbox. One whose container stopped before its service
/// answered is brought up again, and when that attempt fails the failure has to show — not stay
/// hidden behind a grace left over from a container that is no longer there.
#[test]
fn a_failure_after_start_is_not_still_waiting_for_the_service() {
    use micold_client::features::connection::ConnectionStatus;
    use micold_core::sandbox::lifecycle::SandboxState;
    let mut app = app_with_a_failed_sandbox();
    let SandboxState::Failed(failure) = failed_sandbox_state() else {
        unreachable!("the fixture's sandbox is failed")
    };
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));
    let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::Lost));

    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Failed(Box::new(failure))),
    );

    assert!(
        matches!(app.sandbox.state, SandboxState::Failed(_)),
        "setup: the attempt after the lost container ended in a failure"
    );
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "a failed sandbox is not coming up, whatever it was waiting for before (FR-036b)"
    );
}

/// `Stale` is a container that is still up, only created under settings that have since changed
/// (the keep-running opt-in saved while it started, feature 028 FR-022a). Its service is still on
/// its way to listening, so the grace `Started` opened holds: a refused dial in that gap is the
/// bring-up finishing, not a lost connection (FR-036b).
#[test]
fn a_started_sandbox_marked_stale_before_its_service_answered_is_still_coming_up() {
    use micold_client::features::connection::ConnectionStatus;
    use micold_core::sandbox::lifecycle::SandboxState;
    // The save below goes through the real route; `base_app()` has no settings store, so it
    // cannot reach the developer's own `settings.json` (#368).
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::SurviveLogoutToggled(true)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    assert!(
        matches!(app.sandbox.state, SandboxState::Stale(_)),
        "setup: the settings saved during the start marked the running sandbox out of date"
    );

    let _ = connection_failed(&mut app);

    assert_ne!(
        connection_status(&app),
        ConnectionStatus::Disconnected,
        "an out-of-date container is still up, and its service still starting (FR-036b)"
    );
    assert!(
        nothing_was_reported(&app),
        "a service still starting is progress, not a failure to report (FR-036b)"
    );
}

/// FR-036a / US6 scenario 9, "each attempt MUST report why it failed": the refused dial that
/// starts the next attempt moves `Failed(reason)` to `Probing`, and the reason has to survive
/// the move. Otherwise the user watches "Checking the container runtime" come round again with
/// no word of what went wrong last time, and a sandbox that fails three times reads as one that
/// is merely slow.
#[test]
fn the_previous_attempts_reason_stays_visible_while_the_next_one_runs() {
    use micold_core::sandbox::lifecycle::SandboxState;
    let mut app = app_with_a_failed_sandbox();
    let SandboxState::Failed(failure) = failed_sandbox_state() else {
        unreachable!("the fixture's sandbox is failed")
    };
    let _ = connection_failed(&mut app);

    for stage in [SandboxState::Probing, SandboxState::Starting] {
        let _ = update_inner(
            &mut app,
            Message::Sandbox(SandboxMsg::Progress(Box::new(stage.clone()))),
        );

        let line = micold_client::ui::attempt_line(&app.sandbox)
            .expect("setup: a bring-up in flight has a stage line");
        assert!(
            line.detail
                .as_deref()
                .is_some_and(|detail| detail.contains(&failure.reason())),
            "{stage:?}: the attempt that failed has to say why while the next one runs \
                 (FR-036a): {line:?}"
        );
    }
}

/// T207: a restart the user pressed is not "trying again" after an unattended attempt. The reason
/// recorded before it is an older failure than the one the user just read, so carrying it into
/// the restart's stage line names the wrong cause.
#[test]
fn a_restart_does_not_carry_an_unattended_attempts_reason() {
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Failed(Box::new(
            micold_core::sandbox::lifecycle::Failure {
                stage: micold_core::sandbox::lifecycle::Stage::Starting,
                error: micold_core::sandbox::runtime::RuntimeError::NotInstalled {
                    kind: micold_core::sandbox::runtime::RuntimeKind::Podman,
                },
            },
        ))),
    );
    assert!(
        app.sandbox.previous_attempt.is_some(),
        "setup: the unattended attempt has to leave a reason behind"
    );

    let _ = update_inner(&mut app, Message::Sandbox(SandboxMsg::RestartRequested));

    let line =
        micold_client::ui::attempt_line(&app.sandbox).expect("setup: the restart has a stage line");
    assert!(
        !line
            .detail
            .as_deref()
            .is_some_and(|detail| detail.contains("Trying again")),
        "a restart the user pressed showed an older attempt's reason: {line:?}"
    );
}

/// A refused dial while the state still says `Running` is reported, not brought up: nothing
/// has shown the sandbox is gone yet. The liveness check `on_disconnected` starts is what
/// decides that and moves the state on (FR-036, US6 scenario 3); a bring-up from here would
/// start a second container beside one that may be running. Pinned as it stands (T180).
#[test]
fn a_refused_dial_while_the_sandbox_reads_running_is_reported() {
    use micold_core::sandbox::lifecycle::SandboxState;
    use micold_core::sandbox::runtime::ContainerId;
    let mut app = app_with_a_failed_sandbox();
    let running = SandboxState::Running(ContainerId("abc".into()));
    app.sandbox.state = running.clone();

    let work = connection_failed(&mut app);

    assert_eq!(
        work.units(),
        0,
        "only the liveness check may conclude a running sandbox is gone (FR-036)"
    );
    assert_eq!(
        app.sandbox.state, running,
        "a refused dial is no evidence the running container went away (FR-036)"
    );
    assert!(
        a_connection_failure_was_reported(&app),
        "a running sandbox the client cannot reach is a connection failure, until shown otherwise"
    );
}

/// S-6's bound, driven the way the application meets it: a refused dial, the attempt failing,
/// and again — with nothing written into the budget by the test, so a budget the handler forgets
/// to spend is an unbounded loop here rather than a green run.
#[test]
fn once_the_unattended_bring_ups_are_spent_the_failure_is_reported() {
    use micold_core::sandbox::lifecycle::{SandboxState, UNATTENDED_BRING_UP_DELAYS};
    let mut app = app_with_a_failed_sandbox();
    let SandboxState::Failed(failure) = failed_sandbox_state() else {
        unreachable!("the fixture's sandbox is failed")
    };

    let mut bring_ups = 0;
    while connection_failed(&mut app).units() > 0 {
        bring_ups += 1;
        assert!(
            bring_ups <= UNATTENDED_BRING_UP_DELAYS.len(),
            "a bound that keeps bringing the sandbox up is no bound (S-6)"
        );
        let _ = update_inner(
            &mut app,
            Message::Sandbox(SandboxMsg::Failed(Box::new(failure.clone()))),
        );
    }

    assert_eq!(
        bring_ups,
        UNATTENDED_BRING_UP_DELAYS.len(),
        "every unattended bring-up the budget allows is made before the failure stands (S-6)"
    );
    assert_eq!(
        app.sandbox.state,
        failed_sandbox_state(),
        "with the attempts spent, the failure stands rather than a bring-up nothing runs"
    );
    assert!(
        a_connection_failure_was_reported(&app),
        "with nothing left to try, the user has to be told"
    );
}

#[test]
fn without_a_boot_plan_nothing_is_brought_up() {
    let mut app = app_with_a_failed_sandbox();
    app.sandbox_boot = None;
    let failed = app.sandbox.state.clone();

    let work = connection_failed(&mut app);

    assert_eq!(
        work.units(),
        0,
        "with no plan there is nothing to bring up, so nothing may be scheduled"
    );

    assert_eq!(
        app.sandbox.state, failed,
        "a sandbox with no plan to run stays failed rather than claiming a bring-up"
    );
    assert!(
        a_connection_failure_was_reported(&app),
        "with nothing to bring up, the refused dial is the user's to know about"
    );
}

#[test]
fn a_host_process_placement_never_brings_a_sandbox_up() {
    // FR-035 read the other way round: consent to run on the host is not consent to be put
    // back in a container behind the user's back either.
    let mut app = app_with_a_failed_sandbox();
    app.placement.kind = micold_core::sandbox::placement::PlacementKind::HostProcess;
    let failed = app.sandbox.state.clone();

    let work = connection_failed(&mut app);

    assert_eq!(
        work.units(),
        0,
        "the host placement's own connection starts its service; a sandbox task here would \
             undo the consent (FR-035)"
    );

    assert_eq!(
        app.sandbox.state, failed,
        "the host placement leaves the sandbox state alone (FR-035)"
    );
    assert!(
        a_connection_failure_was_reported(&app),
        "on the host placement a refused dial is an ordinary connection failure"
    );
}

#[test]
fn a_reported_stage_becomes_the_sandbox_state() {
    let mut app = app_with_a_failed_sandbox();

    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Progress(Box::new(
            micold_core::sandbox::lifecycle::SandboxState::Starting,
        ))),
    );

    assert_eq!(
        app.sandbox.state,
        micold_core::sandbox::lifecycle::SandboxState::Starting,
        "a stage that never reaches the state is the progress SC-004c says was dropped"
    );
}

/// The bound is earned back by a service that answered, not by a container that started: a
/// daemon that crashes after every start would otherwise refill its own budget on each loss and
/// restart the container forever, with no spacing (S-6).
#[test]
fn a_sandbox_whose_service_answered_earns_its_unattended_bring_ups_back() {
    use micold_core::sandbox::lifecycle::UnattendedBringUps;
    let mut app = app_with_a_failed_sandbox();
    let _ = connection_failed(&mut app);
    let spent = app.sandbox.unattended;
    assert_ne!(
        spent,
        UnattendedBringUps::default(),
        "setup: the refused dial has to spend an attempt, or there is nothing to earn back"
    );
    let _ = update_inner(&mut app, Message::Sandbox(a_started_sandbox_message()));
    assert_eq!(
        app.sandbox.unattended, spent,
        "a container that started has not recovered until its service answers — refilling here \
             lets a crashing service restart the container without bound (S-6)"
    );

    the_service_answers(&mut app);

    assert_eq!(
        app.sandbox.unattended,
        UnattendedBringUps::default(),
        "a bound that is never restored makes the second outage of a long session unrecoverable"
    );
}

/// T205: a stage or outcome that arrives once the placement has moved to the host belongs to a
/// sandbox nobody is running. Adopted, it draws a bring-up under the host placement and hides the
/// disconnected banner behind `is_coming_up` (FR-036b).
#[test]
fn a_bring_up_reported_after_moving_to_the_host_is_ignored() {
    use micold_core::sandbox::lifecycle::SandboxState;
    for report in [
        SandboxMsg::Progress(Box::new(SandboxState::Starting)),
        a_started_sandbox_message(),
        SandboxMsg::Failed(Box::new(match failed_sandbox_state() {
            SandboxState::Failed(failure) => failure,
            other => panic!("setup: expected a failure, got {other:?}"),
        })),
    ] {
        let mut app = base_app();
        app.placement.kind = PlacementKind::HostProcess;
        let label = format!("{report:?}");

        let _ = update_inner(&mut app, Message::Sandbox(report));

        assert_eq!(
            app.sandbox.state,
            SandboxState::Disabled,
            "{label} arrived under the host placement and was adopted"
        );
    }
}

/// T204: a bring-up still waiting its delay is cancelled by moving to the host. Left to run, it
/// starts `micold-sandbox` after the move and reports it `Running` under the host placement.
#[test]
fn moving_to_the_host_cancels_a_bring_up_that_has_not_run() {
    let mut app = app_saving_a_placement(PlacementKind::LocalSandbox, PlacementKind::HostProcess);
    let failed = app_with_a_failed_sandbox();
    app.sandbox = failed.sandbox.clone();
    app.sandbox_boot = failed.sandbox_boot.clone();
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    let waiting = connection_failed(&mut app);
    assert_eq!(
        shell::sandbox::BringUp::scheduled().len(),
        1,
        "setup: the refused dial has to schedule a bring-up, or there is nothing to cancel"
    );

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PlacementChangeConfirmed),
    );
    let reported = messages(waiting);
    for message in reported.clone() {
        let _ = update_inner(&mut app, message);
    }

    assert!(
        reported.is_empty(),
        "the bring-up ran after the move to the host: {reported:?}"
    );
    assert_eq!(
        app.sandbox.state,
        micold_core::sandbox::lifecycle::SandboxState::Disabled
    );
}

// ---------------------------------------------------------------------------------------------
// 029 BUG-001 (FR-003b): the availability question names the directory it is about
// ---------------------------------------------------------------------------------------------

/// Connect `app` on a channel this test keeps, and drop what the connection itself sent.
fn connected_with_outbox(
    app: &mut App,
) -> iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg> {
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let _ = update_inner(
        app,
        Message::Connection(ConnectionMsg::Connected {
            outbox: micold_client::daemon::Outbox::new(tx),
            catalog: snapshot_with("/repo/demo", Vec::new()),
            settings: quiet_settings(),
        }),
    );
    while rx.try_recv().is_ok() {}
    rx
}

/// Every directory an availability request sent since the last drain named.
fn availability_asked_for(
    rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) -> Vec<Option<PathBuf>> {
    let mut asked = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let ClientMsg::AiCliAvailabilityRequest { cwd, .. } = msg {
            asked.push(cwd);
        }
    }
    asked
}

/// U11: the per-session start menu for a worktree asks which CLIs a session *there* would find —
/// the environment-include script can give each directory a different `PATH`.
#[test]
fn opening_the_start_menu_asks_about_its_locations_directory() {
    let mut app = base_app();
    app.core.workspace.active = Some(PathBuf::from("/repo/demo"));
    let mut rx = connected_with_outbox(&mut app);

    let _ = update_inner(
        &mut app,
        Message::Session(SessionMsg::StartMenuOpened {
            location: SessionLocation::Worktree("feature-x".into()),
            unavailable_default: None,
        }),
    );

    assert_eq!(
        availability_asked_for(&mut rx),
        vec![Some(PathBuf::from(
            "/repo/demo/.claude/worktrees/feature-x"
        ))],
        "the menu offers CLIs for a session in this worktree, so the service must be asked about \
         the environment a session there is spawned with (FR-003b)"
    );
}

/// U12: Settings chooses the default for every directory, so no directory is in play and the
/// request names none — the service answers it for the home directory.
#[test]
fn opening_settings_asks_about_no_directory() {
    let mut app = base_app();
    app.core.workspace.active = Some(PathBuf::from("/repo/demo"));
    let mut rx = connected_with_outbox(&mut app);

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));

    assert_eq!(
        availability_asked_for(&mut rx),
        vec![None],
        "Settings is not about the active project's directory, so it must not ask about it: the \
         default applies everywhere, and the service answers a request with no directory for the \
         home directory (FR-003b)"
    );
}

/// U13, reversed by feature 033 (FR-002, FR-009): answers are resolved off the service's
/// connection loop, so they can arrive out of order — and each is about the directory its request
/// named. The home answer Settings asked for and a row's answer are both kept, whichever lands
/// last. (A later answer replacing an earlier one is now only a same-directory rule, pinned in
/// `tests/directory_availability.rs`.)
#[test]
fn an_answer_to_an_earlier_question_does_not_replace_a_later_one() {
    let mut app = base_app();
    app.core.workspace.active = Some(PathBuf::from("/repo/demo"));
    let mut rx = connected_with_outbox(&mut app);
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Session(SessionMsg::StartMenuOpened {
            location: SessionLocation::Worktree("feature-x".into()),
            unavailable_default: None,
        }),
    );
    let mut asked = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let ClientMsg::AiCliAvailabilityRequest { req, .. } = msg {
            asked.push(req);
        }
    }
    let [earlier, later] = asked[..] else {
        panic!("fixture check: Settings and the menu each ask once, got {asked:?}");
    };

    for (req, available) in [(later, vec![AiCli::Pi]), (earlier, vec![AiCli::ClaudeCode])] {
        let _ = update_inner(
            &mut app,
            Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
                req,
                available,
                env: None,
            })),
        );
    }

    let worktree = Path::new("/repo/demo/.claude/worktrees/feature-x");
    assert_eq!(
        app.core
            .session
            .availability
            .for_dir(worktree)
            .map(|answer| answer.available.clone()),
        Some(vec![AiCli::Pi]),
        "the menu's answer is the worktree's, and the Settings answer arriving after it describes \
         the home directory, not this one (FR-002)"
    );
    assert_eq!(
        app.core
            .session
            .availability
            .home()
            .map(|answer| answer.available.clone()),
        Some(vec![AiCli::ClaudeCode]),
        "and the home answer Settings reads is the one asked for it (FR-008)"
    );
}

// ---------------------------------------------------------------------------------------------
// Feature 033: each sidebar row is answered for its own directory
// ---------------------------------------------------------------------------------------------

use micold_client::features::session::{PressTarget, StartIntent};
use micold_core::protocol::messages::{ProjectSnapshot, WorktreeSnapshot};

const DEMO: &str = "/repo/demo";
const FEAT_A: &str = "/repo/demo/.claude/worktrees/feat-a";
const FEAT_B: &str = "/repo/demo/.claude/worktrees/feat-b";

/// `/repo/demo` open with the given worktrees, each `(dir, status, created by this app)`. A
/// worktree the app did not create is an agent's, hidden until revealed (029 FR-004).
fn app_on_demo(worktrees: &[(&str, micold_core::worktree::WorktreeStatus, bool)]) -> App {
    let mut app = base_app();
    let root = PathBuf::from(DEMO);
    app.core
        .workspace
        .projects
        .push(micold_core::project::Project::new(
            root.clone(),
            true,
            micold_core::project::Availability::Available,
        ));
    app.core.workspace.active = Some(root.clone());
    for (dir, _, created) in worktrees {
        if *created {
            app.core.workspace.record_user_created(&root, dir);
        }
    }
    app.core.worktree.worktrees = worktrees
        .iter()
        .map(|(dir, status, _)| micold_core::worktree::Worktree {
            dir_name: dir.to_string(),
            path: root.join(".claude/worktrees").join(dir),
            branch: Some(format!("feat/{dir}")),
            status: *status,
            included: false,
        })
        .collect();
    app
}

/// `/repo/demo` with one valid worktree of the user's, one hidden agent worktree and one whose
/// directory is gone: D = 2 rows to answer for (the root and `feat-a`).
fn app_with_mixed_rows() -> App {
    use micold_core::worktree::WorktreeStatus::{Missing, Valid};
    app_on_demo(&[
        ("feat-a", Valid, true),
        ("agent-1f", Valid, false),
        ("feat-gone", Missing, true),
    ])
}

/// Connect `app` with `catalog` on a channel this test keeps, **without** draining what the
/// connection sent — the connect's own asks are what these tests are about.
fn connect_with_catalog_keeping_outbox(
    app: &mut App,
    catalog: micold_core::protocol::messages::CatalogSnapshot,
) -> iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg> {
    connect_with_settings_keeping_outbox(app, catalog, quiet_settings())
}

/// [`connect_with_catalog_keeping_outbox`] with the service's settings given.
fn connect_with_settings_keeping_outbox(
    app: &mut App,
    catalog: micold_core::protocol::messages::CatalogSnapshot,
    settings: micold_core::protocol::messages::DaemonSettings,
) -> iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg> {
    let (tx, rx) = iced::futures::channel::mpsc::unbounded();
    let _ = update_inner(
        app,
        Message::Connection(ConnectionMsg::Connected {
            outbox: micold_client::daemon::Outbox::new(tx),
            catalog,
            settings,
        }),
    );
    rx
}

/// Every availability request sent since the last drain, as `(req, cwd)`.
fn availability_requests(
    rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) -> Vec<(u64, Option<PathBuf>)> {
    let mut asked = Vec::new();
    while let Ok(msg) = rx.try_recv() {
        if let ClientMsg::AiCliAvailabilityRequest { req, cwd } = msg {
            asked.push((req, cwd));
        }
    }
    asked
}

/// The service's reply to `req`.
fn availability_answer(app: &mut App, req: u64, available: &[AiCli]) {
    let _ = update_inner(
        app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
            req,
            available: available.to_vec(),
            env: None,
        })),
    );
}

/// Answer each request in `asked` with what its directory has: `home` for `cwd: None`, else the
/// entry of `dirs` naming its directory.
fn answer_each(
    app: &mut App,
    asked: &[(u64, Option<PathBuf>)],
    home: &[AiCli],
    dirs: &[(&str, &[AiCli])],
) {
    for (req, cwd) in asked {
        let available = match cwd {
            None => home,
            Some(dir) => {
                dirs.iter()
                    .find(|(d, _)| Path::new(d) == dir)
                    .unwrap_or_else(|| panic!("fixture: no answer given for {}", dir.display()))
                    .1
            }
        };
        availability_answer(app, *req, available);
    }
}

fn offers_a_choice(app: &App, dir: &str) -> bool {
    app.core
        .session
        .start_affordance_offers_a_choice(Path::new(dir))
}

/// What the primary half of `location`'s start affordance would do — the same reader
/// `ui/sidebar.rs`'s `start_press` calls.
fn primary_press(app: &App, location: &SessionLocation) -> StartIntent {
    let dir = app
        .core
        .location_dir(location)
        .expect("a row needs an active project");
    app.core.session.start_intent(PressTarget::Primary, &dir)
}

const CLAUDE: &[AiCli] = &[AiCli::ClaudeCode];
const CLAUDE_AND_PI: &[AiCli] = &[AiCli::ClaudeCode, AiCli::Pi];

fn open_start_list(app: &mut App, location: SessionLocation) {
    let _ = update_inner(
        app,
        Message::Session(SessionMsg::StartMenuOpened {
            location,
            unavailable_default: None,
        }),
    );
}

/// U22 (FR-006, FR-007, SC-004): the connect asks home once and each row's directory once — none
/// for a hidden agent worktree or one whose directory is gone.
#[test]
fn availability_connect_asks_home_and_each_row_directory_once() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));

    let asked: Vec<Option<PathBuf>> = availability_requests(&mut rx)
        .into_iter()
        .map(|(_, cwd)| cwd)
        .collect();
    assert_eq!(
        asked,
        vec![None, Some(PathBuf::from(DEMO)), Some(PathBuf::from(FEAT_A))],
        "home first, then one request per row on screen (D = 2)"
    );
}

/// U23 (FR-011): a reconnect forgets every answer, asks again, and a reply to a request the
/// previous connection sent is dropped.
#[test]
fn availability_reconnect_drops_answers_and_reasks() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let before = availability_requests(&mut rx);
    answer_each(
        &mut app,
        &before,
        CLAUDE,
        &[(DEMO, CLAUDE_AND_PI), (FEAT_A, CLAUDE_AND_PI)],
    );

    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let after = availability_requests(&mut rx);

    assert_eq!(
        after.iter().map(|(_, cwd)| cwd.clone()).collect::<Vec<_>>(),
        vec![None, Some(PathBuf::from(DEMO)), Some(PathBuf::from(FEAT_A))],
        "the reconnect asks the same set again"
    );
    assert_eq!(app.core.session.availability.home(), None);
    assert_eq!(
        app.core.session.availability.for_dir(Path::new(FEAT_A)),
        None
    );

    let (stale, _) = before[2].clone();
    availability_answer(&mut app, stale, CLAUDE_AND_PI);
    assert_eq!(
        app.core.session.availability.for_dir(Path::new(FEAT_A)),
        None,
        "an answer to the previous connection's request describes a service that may be gone"
    );
}

/// A1 (US1-1, FR-001, FR-006, SC-001): no Settings, no list opened — once connected and answered,
/// the project's rows offer the CLI its script provides.
#[test]
fn availability_a_project_whose_script_adds_a_cli_offers_it_on_its_rows() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let asked = availability_requests(&mut rx);
    // Rows first, home last: the arrival order a slow home resolution produces.
    let mut reversed = asked.clone();
    reversed.reverse();
    answer_each(
        &mut app,
        &reversed,
        CLAUDE,
        &[(DEMO, CLAUDE_AND_PI), (FEAT_A, CLAUDE_AND_PI)],
    );

    assert!(
        offers_a_choice(&app, DEMO),
        "the Default row offers the choice"
    );
    assert!(
        offers_a_choice(&app, FEAT_A),
        "and so does the worktree row"
    );
    assert!(
        app.core
            .session
            .offered_providers(Some(Path::new(DEMO)))
            .contains(&AiCli::Pi),
        "and the list it opens includes Pi"
    );
}

/// A2 (US1-2, FR-001): a second project without the script offers no choice, and its primary
/// press starts Claude.
#[test]
fn availability_a_project_without_the_script_offers_no_choice() {
    let p = tempfile::tempdir().unwrap();
    let q = tempfile::tempdir().unwrap();
    let mut app = base_app();
    let scanner = micold_core::fs_scan::FakeFolderScanner::new();
    app.core
        .workspace
        .open_or_activate(q.path().to_path_buf(), &scanner);
    app.core
        .workspace
        .open_or_activate(p.path().to_path_buf(), &scanner);
    let p_dir = p.path().to_str().unwrap().to_owned();
    let q_dir = q.path().to_str().unwrap().to_owned();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(&p_dir, Vec::new()));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(&p_dir, CLAUDE_AND_PI)]);
    assert!(offers_a_choice(&app, &p_dir), "fixture check: P offers Pi");

    let _ = update_inner(
        &mut app,
        Message::Project(ProjectMsg::Reopened(q.path().to_path_buf())),
    );
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(&q_dir, CLAUDE)]);

    assert!(!offers_a_choice(&app, &q_dir), "Q's rows offer no choice");
    assert_eq!(
        primary_press(&app, &SessionLocation::Default),
        StartIntent::Start(AiCli::ClaudeCode),
        "and Q's primary press starts Claude"
    );
}

/// A3 (US1-3, FR-002, FR-008): Settings asks for home, and home's answer never replaces a row's.
#[test]
fn availability_opening_settings_never_replaces_a_rows_answer() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let asked = availability_requests(&mut rx);
    answer_each(
        &mut app,
        &asked,
        CLAUDE,
        &[(DEMO, CLAUDE_AND_PI), (FEAT_A, CLAUDE_AND_PI)],
    );

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[]);

    assert!(
        offers_a_choice(&app, DEMO),
        "P's Default row still offers Pi"
    );
    assert!(
        offers_a_choice(&app, FEAT_A),
        "and so does its worktree row"
    );
}

/// U24 (FR-008, C1 A2): Settings asks about the home directory and nothing else.
#[test]
fn availability_settings_asks_for_home_only() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let _ = availability_requests(&mut rx);

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));

    assert_eq!(
        availability_requests(&mut rx)
            .into_iter()
            .map(|(_, cwd)| cwd)
            .collect::<Vec<_>>(),
        vec![None]
    );
}

/// U25 (FR-004, C1 A3): a row's list asks for its own directory even with an answer held, and the
/// answer lands under that directory.
#[test]
fn availability_a_start_list_asks_for_its_own_directory() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let asked = availability_requests(&mut rx);
    answer_each(
        &mut app,
        &asked,
        CLAUDE,
        &[(DEMO, CLAUDE), (FEAT_A, CLAUDE)],
    );

    open_start_list(&mut app, SessionLocation::Worktree("feat-a".into()));
    let asked = availability_requests(&mut rx);
    assert_eq!(
        asked.iter().map(|(_, cwd)| cwd.clone()).collect::<Vec<_>>(),
        vec![Some(PathBuf::from(FEAT_A))],
        "the list refreshes its own row's answer"
    );
    answer_each(&mut app, &asked, CLAUDE, &[(FEAT_A, CLAUDE_AND_PI)]);
    assert!(offers_a_choice(&app, FEAT_A), "filed under the worktree");
    assert_eq!(
        app.core
            .session
            .availability
            .home()
            .map(|a| a.available.clone()),
        Some(CLAUDE.to_vec()),
        "and not as the home answer"
    );
}

/// A4 (US1-4, FR-011): after a reconnect the rows are re-asked with no user action and offer Pi
/// again once the answers arrive.
#[test]
fn availability_a_reconnect_reasks_every_row_and_restores_its_answer() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let asked = availability_requests(&mut rx);
    let dirs: &[(&str, &[AiCli])] = &[(DEMO, CLAUDE_AND_PI), (FEAT_A, CLAUDE_AND_PI)];
    answer_each(&mut app, &asked, CLAUDE, dirs);

    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, dirs);

    assert!(offers_a_choice(&app, DEMO));
    assert!(offers_a_choice(&app, FEAT_A));
}

/// A5 (US1-5, FR-010): with default Pi and only this project resolving it, one press starts Pi.
#[test]
fn availability_the_primary_press_starts_a_default_only_the_row_provides() {
    let mut app = app_with_mixed_rows();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    app.core.session.default_ai_cli = AiCli::Pi;
    let mut asked = availability_requests(&mut rx);
    asked.reverse();
    answer_each(
        &mut app,
        &asked,
        CLAUDE,
        &[(DEMO, CLAUDE_AND_PI), (FEAT_A, CLAUDE_AND_PI)],
    );

    assert_eq!(
        primary_press(&app, &SessionLocation::Default),
        StartIntent::Start(AiCli::Pi)
    );
    assert_eq!(
        primary_press(&app, &SessionLocation::Worktree("feat-a".into())),
        StartIntent::Start(AiCli::Pi)
    );
}

/// U26 (FR-006, C1 A4): opening a project asks for each of its rows once; opening it again asks
/// nothing new.
#[test]
fn availability_opening_a_project_asks_once_per_directory() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    let mut app = base_app();
    let scanner = micold_core::fs_scan::FakeFolderScanner::new();
    app.core
        .workspace
        .open_or_activate(a.path().to_path_buf(), &scanner);
    app.core
        .workspace
        .open_or_activate(b.path().to_path_buf(), &scanner);
    let b_dir = b.path().to_str().unwrap().to_owned();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(&b_dir, Vec::new()));
    let _ = availability_requests(&mut rx);

    let _ = update_inner(
        &mut app,
        Message::Project(ProjectMsg::Reopened(a.path().to_path_buf())),
    );
    assert_eq!(
        availability_requests(&mut rx)
            .into_iter()
            .map(|(_, cwd)| cwd)
            .collect::<Vec<_>>(),
        vec![Some(a.path().to_path_buf())],
        "opening A asks about A's root, once"
    );

    let _ = update_inner(
        &mut app,
        Message::Project(ProjectMsg::Reopened(a.path().to_path_buf())),
    );
    assert_eq!(
        availability_requests(&mut rx),
        Vec::new(),
        "A's root is already asked, so opening it again asks nothing"
    );
}

fn catalog_with_worktree(dir: &str) -> micold_core::protocol::messages::CatalogSnapshot {
    catalog_with_worktree_in(dir, micold_core::protocol::messages::WorktreeStatus::Clean)
}

/// `/repo/demo` with one user-created worktree `dir` in `status`.
fn catalog_with_worktree_in(
    dir: &str,
    status: micold_core::protocol::messages::WorktreeStatus,
) -> micold_core::protocol::messages::CatalogSnapshot {
    let mut catalog = snapshot_with(DEMO, Vec::new());
    let project: &mut ProjectSnapshot = &mut catalog.projects[0];
    project.worktrees.push(WorktreeSnapshot {
        dir_name: dir.to_string(),
        branch: Some(format!("feat/{dir}")),
        display_name: dir.to_string(),
        status,
        path: PathBuf::from(DEMO).join(".claude/worktrees").join(dir),
        included: false,
        user_created: true,
    });
    catalog
}

/// U27 (FR-004, C1 A5): a worktree the catalog push adds is asked about exactly once.
#[test]
fn availability_a_new_worktree_is_asked_about_once() {
    let mut app = app_on_demo(&[]);
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let _ = availability_requests(&mut rx);

    feed(
        &mut app,
        DaemonMsg::CatalogChanged {
            catalog: catalog_with_worktree("feat-new"),
        },
    );
    assert_eq!(
        availability_requests(&mut rx)
            .into_iter()
            .map(|(_, cwd)| cwd)
            .collect::<Vec<_>>(),
        vec![Some(PathBuf::from(DEMO).join(".claude/worktrees/feat-new"))]
    );

    feed(
        &mut app,
        DaemonMsg::CatalogChanged {
            catalog: catalog_with_worktree("feat-new"),
        },
    );
    assert_eq!(
        availability_requests(&mut rx),
        Vec::new(),
        "the same push again changes no row, so it asks nothing"
    );
}

/// Worktrees A (`feat-a`, whose directory provides Pi) and B (`feat-b`, which does not), answered.
fn app_with_two_answered_worktrees() -> (
    App,
    iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) {
    use micold_core::worktree::WorktreeStatus::Valid;
    let mut app = app_on_demo(&[("feat-a", Valid, true), ("feat-b", Valid, true)]);
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let asked = availability_requests(&mut rx);
    answer_each(
        &mut app,
        &asked,
        CLAUDE,
        &[(DEMO, CLAUDE), (FEAT_A, CLAUDE_AND_PI), (FEAT_B, CLAUDE)],
    );
    (app, rx)
}

/// A6 (US2-1, FR-002): opening, answering and closing A's list leaves B as it was.
#[test]
fn availability_opening_one_rows_list_does_not_change_another_row() {
    let (mut app, mut rx) = app_with_two_answered_worktrees();

    open_start_list(&mut app, SessionLocation::Worktree("feat-a".into()));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(FEAT_A, CLAUDE_AND_PI)]);
    let _ = update_inner(&mut app, Message::Session(SessionMsg::StartMenuDismissed));

    assert!(!offers_a_choice(&app, FEAT_B), "B offers no choice");
    assert_eq!(
        primary_press(&app, &SessionLocation::Worktree("feat-b".into())),
        StartIntent::Start(AiCli::ClaudeCode),
        "and B's primary press starts Claude"
    );
}

/// A7 (US2-2, FR-002): opening B's list leaves A offering the choice.
#[test]
fn availability_opening_the_other_rows_list_leaves_the_first_alone() {
    let (mut app, mut rx) = app_with_two_answered_worktrees();

    open_start_list(&mut app, SessionLocation::Worktree("feat-b".into()));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(FEAT_B, CLAUDE)]);

    assert!(offers_a_choice(&app, FEAT_A), "A still offers the choice");
}

// ---------------------------------------------------------------------------------------------
// Feature 033 M2: the answer follows the events that change it (US3)
// ---------------------------------------------------------------------------------------------

use micold_core::protocol::messages::DaemonSettings;

/// The service's settings with environment-include switched `enabled`. The script path is blank,
/// which runs nothing either way (`resolve_env_include`), so no test here spawns a shell.
fn settings_with_env_include(enabled: bool) -> DaemonSettings {
    DaemonSettings {
        env_include_enabled: enabled,
        env_include_script_path: "   ".into(),
        ..quiet_settings()
    }
}

/// `/repo/demo`'s rows (the root and `feat-a`) connected under `settings`, with home answering
/// `[claude]` and both rows `[claude, pi]` — the project whose script provides Pi.
fn app_offering_pi_under(
    settings: DaemonSettings,
) -> (
    App,
    iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) {
    let mut app = app_with_mixed_rows();
    let mut rx =
        connect_with_settings_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()), settings);
    let asked = availability_requests(&mut rx);
    answer_each(
        &mut app,
        &asked,
        CLAUDE,
        &[(DEMO, CLAUDE_AND_PI), (FEAT_A, CLAUDE_AND_PI)],
    );
    assert!(offers_a_choice(&app, DEMO), "fixture check: P offers Pi");
    (app, rx)
}

/// This window's Settings save of `settings`' environment-include fields, then the service's
/// echo of it — the order a real save arrives in.
fn save_env_include_and_echo(app: &mut App, settings: DaemonSettings) {
    app.core.settings.settings_draft = Some(SettingsDraft {
        terminal: TerminalDraft {
            scrollback_lines: settings.scrollback_lines.to_string(),
        },
        environment: EnvironmentDraft {
            enabled: settings.env_include_enabled,
            script_path: settings.env_include_script_path.clone(),
            timeout_secs: settings.env_include_timeout_secs.to_string(),
            default_ai_cli: settings.default_ai_cli,
            pi_activity_component: settings.pi_activity_component,
            tool_server_enabled: settings.tool_server_enabled,
            desktop_notifications: settings.desktop_notifications,
            cross_session_access: settings.cross_session_access,
        },
        ..SettingsDraft::default()
    });
    let _ = update_inner(app, Message::Settings(SettingsMsg::Saved));
    feed(app, DaemonMsg::SettingsChanged { settings });
}

fn cwds(asked: &[(u64, Option<PathBuf>)]) -> Vec<Option<PathBuf>> {
    asked.iter().map(|(_, cwd)| cwd.clone()).collect()
}

/// Home and every row on screen, in the order a refresh asks them.
fn every_key() -> Vec<Option<PathBuf>> {
    vec![None, Some(PathBuf::from(DEMO)), Some(PathBuf::from(FEAT_A))]
}

/// A8 (US3-1, FR-004, SC-005): saving env-include off in this window re-asks every row, and the
/// project's rows stop offering Pi once the answers arrive — no restart, no list opened.
#[test]
fn availability_saving_env_include_refreshes_every_row() {
    let (mut app, mut rx) = app_offering_pi_under(settings_with_env_include(true));

    save_env_include_and_echo(&mut app, settings_with_env_include(false));
    let asked = availability_requests(&mut rx);
    answer_each(
        &mut app,
        &asked,
        CLAUDE,
        &[(DEMO, CLAUDE), (FEAT_A, CLAUDE)],
    );

    assert!(
        !offers_a_choice(&app, DEMO),
        "with the script off, P's Default row no longer offers Pi"
    );
    assert!(
        !offers_a_choice(&app, FEAT_A),
        "and neither does its worktree row"
    );
}

/// U32 (FR-004, SC-005, R6): the echo of this window's own save re-asks home and every row, even
/// though the save already overwrote the window's copy of the settings — and the held answers stay
/// readable until the new ones replace them.
#[test]
fn availability_this_windows_env_include_save_refreshes_every_row() {
    let (mut app, mut rx) = app_offering_pi_under(settings_with_env_include(true));

    save_env_include_and_echo(&mut app, settings_with_env_include(false));

    assert_eq!(
        cwds(&availability_requests(&mut rx)),
        every_key(),
        "home and each row are asked again, once each"
    );
    assert!(
        offers_a_choice(&app, DEMO),
        "until its new answer lands, a row keeps offering what it was last told"
    );
}

/// U33 (FR-004, 029 FR-011): another window's env-include change reaches this one only as the
/// service's `SettingsChanged`, and re-asks the same set.
#[test]
fn availability_another_windows_env_include_change_refreshes() {
    let (mut app, mut rx) = app_offering_pi_under(settings_with_env_include(false));

    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: settings_with_env_include(true),
        },
    );

    assert_eq!(cwds(&availability_requests(&mut rx)), every_key());
}

/// U34 (FR-004, "only these events"): a settings change that leaves env-include alone changes no
/// directory's `PATH`, so it asks nothing.
#[test]
fn availability_an_unrelated_settings_change_asks_nothing() {
    let (mut app, mut rx) = app_offering_pi_under(settings_with_env_include(true));

    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                scrollback_lines: 42_000,
                ..settings_with_env_include(true)
            },
        },
    );

    assert_eq!(availability_requests(&mut rx), Vec::new());
}

/// U28 (FR-012, FR-003): switching to another project drops the previous project's answers, and
/// switching back asks for them again.
#[test]
fn availability_switching_projects_drops_and_reasks() {
    let p = tempfile::tempdir().unwrap();
    let q = tempfile::tempdir().unwrap();
    let mut app = base_app();
    let scanner = micold_core::fs_scan::FakeFolderScanner::new();
    app.core
        .workspace
        .open_or_activate(q.path().to_path_buf(), &scanner);
    app.core
        .workspace
        .open_or_activate(p.path().to_path_buf(), &scanner);
    let p_dir = p.path().to_str().unwrap().to_owned();
    let q_dir = q.path().to_str().unwrap().to_owned();
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(&p_dir, Vec::new()));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(&p_dir, CLAUDE_AND_PI)]);

    let _ = update_inner(
        &mut app,
        Message::Project(ProjectMsg::Reopened(q.path().to_path_buf())),
    );
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(&q_dir, CLAUDE)]);
    assert!(
        !offers_a_choice(&app, &p_dir),
        "P has no row on screen any more, so its answer is not kept (FR-012)"
    );

    let _ = update_inner(
        &mut app,
        Message::Project(ProjectMsg::Reopened(p.path().to_path_buf())),
    );
    assert_eq!(
        cwds(&availability_requests(&mut rx)),
        vec![Some(p.path().to_path_buf())],
        "back on P, its root is asked again"
    );
}

/// U29 (FR-012, C1 A6): forgetting a project drops its answers.
#[test]
fn availability_forgetting_a_project_drops_its_answers() {
    let (mut app, _rx) = app_offering_pi_under(quiet_settings());

    app.core.project.forget_target = Some(PathBuf::from(DEMO));
    let _ = update_inner(&mut app, Message::Project(ProjectMsg::ForgetConfirmed));

    assert!(
        !offers_a_choice(&app, DEMO),
        "the forgotten project's root answer is dropped"
    );
    assert!(!offers_a_choice(&app, FEAT_A), "and so is its worktree's");
}

/// U30 (FR-004, deleted then recreated): a worktree whose directory goes missing drops its answer,
/// and one that comes back is asked about again.
#[test]
fn availability_a_deleted_then_recreated_worktree_is_asked_again() {
    use micold_core::protocol::messages::WorktreeStatus::{Clean, Missing};
    let mut app = app_on_demo(&[]);
    let mut rx = connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
    let _ = availability_requests(&mut rx);
    feed(
        &mut app,
        DaemonMsg::CatalogChanged {
            catalog: catalog_with_worktree_in("feat-a", Clean),
        },
    );
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(FEAT_A, CLAUDE_AND_PI)]);
    assert!(
        offers_a_choice(&app, FEAT_A),
        "fixture check: feat-a offers Pi"
    );

    feed(
        &mut app,
        DaemonMsg::CatalogChanged {
            catalog: catalog_with_worktree_in("feat-a", Missing),
        },
    );
    assert!(
        !offers_a_choice(&app, FEAT_A),
        "a worktree whose directory is gone keeps no answer"
    );

    feed(
        &mut app,
        DaemonMsg::CatalogChanged {
            catalog: catalog_with_worktree_in("feat-a", Clean),
        },
    );
    assert_eq!(
        cwds(&availability_requests(&mut rx)),
        vec![Some(PathBuf::from(FEAT_A))],
        "recreated, it is asked about again"
    );
}

/// U31 (FR-003, C1 A5b): revealing agent worktrees asks about them, and hiding them drops their
/// answers.
#[test]
fn availability_revealing_agent_worktrees_asks_hiding_drops() {
    const AGENT: &str = "/repo/demo/.claude/worktrees/agent-1f";
    let (mut app, mut rx) = app_offering_pi_under(quiet_settings());

    let _ = update_inner(
        &mut app,
        Message::Sidebar(SidebarMsg::ShowAgentWorktreesToggled),
    );
    let asked = availability_requests(&mut rx);
    assert_eq!(
        cwds(&asked),
        vec![Some(PathBuf::from(AGENT))],
        "the revealed agent worktree is asked about, once"
    );
    answer_each(&mut app, &asked, CLAUDE, &[(AGENT, CLAUDE_AND_PI)]);

    let _ = update_inner(
        &mut app,
        Message::Sidebar(SidebarMsg::ShowAgentWorktreesToggled),
    );
    assert!(
        !offers_a_choice(&app, AGENT),
        "hidden again, its answer is dropped"
    );
}

/// A9 (US3-2, FR-006, SC-003): with every row answered, hovering, scrolling and drawing ask
/// nothing. (That no *other* source line asks is `tests/availability_is_asked_only_on_named_events.rs`.)
#[test]
fn availability_nothing_is_asked_while_nothing_changes() {
    let (mut app, mut rx) = app_offering_pi_under(quiet_settings());

    let _ = update_inner(
        &mut app,
        Message::Worktree(WorktreeMsg::Hovered("feat-a".into())),
    );
    let _ = update_inner(&mut app, Message::Sidebar(SidebarMsg::Scrolled(120)));
    let _ = view(&app);

    assert_eq!(availability_requests(&mut rx), Vec::new());
}

/// A10 (US3-3, FR-004): a row whose answer is held is still re-asked when its list opens, and a
/// CLI installed since shows up there.
#[test]
fn availability_opening_a_rows_list_refreshes_its_answer() {
    let (mut app, mut rx) = app_with_two_answered_worktrees();
    assert!(
        !offers_a_choice(&app, FEAT_B),
        "fixture check: B offers no choice"
    );

    open_start_list(&mut app, SessionLocation::Worktree("feat-b".into()));
    let asked = availability_requests(&mut rx);
    answer_each(&mut app, &asked, CLAUDE, &[(FEAT_B, CLAUDE_AND_PI)]);

    assert!(
        offers_a_choice(&app, FEAT_B),
        "Pi, installed since B was answered, is offered on B's row"
    );
}

/// FR-004g (BUG-008): opening Settings carries the running sandbox's unshared sign-in into the
/// form, the way it carries the runtime's capabilities, and carries nothing once that sandbox has
/// stopped. `Sandbox::locations` answers only for the container that is running.
#[test]
fn opening_settings_seeds_the_unshared_sign_in_from_the_running_sandbox_only() {
    const LOOKED_FOR: &str = "/Users/u/.claude/.credentials.json";
    let mut app = app_with_a_failed_sandbox();
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new((
            a_started_sandbox(),
            micold_client::features::sandbox::SandboxLocations {
                unshared_sign_in: Some(LOOKED_FOR.to_string()),
                ..Default::default()
            },
        )))),
    );

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    assert_eq!(
        app.core
            .settings
            .settings_draft
            .as_ref()
            .and_then(|d| d.daemon.unshared_sign_in.as_deref()),
        Some(LOOKED_FOR),
        "the page was not told the running sandbox has no sign-in"
    );

    app.core.settings.settings_draft = None;
    app.sandbox
        .observe(micold_core::sandbox::lifecycle::SandboxState::Disabled);
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    assert_eq!(
        app.core
            .settings
            .settings_draft
            .as_ref()
            .and_then(|d| d.daemon.unshared_sign_in.as_deref()),
        None,
        "a stopped sandbox's report outlived it"
    );
}

// --- Spec 035: a missing environment-include script path is reported (outer loop, A1–A4) ------
//
// The highest level a cargo test reaches: the real shell handler, the check job it prepared run
// synchronously against a fake probe, its answer through the reducer, and the page's lines read
// from `script_path_notice`. What is actually drawn is quickstart §B's visual pass.

mod script_path_report {
    use super::*;
    use micold_client::features::settings::{
        script_path_notice, CheckOrigin, NoticeLine, ScriptCheck,
    };
    use micold_core::env_include::FakeEnvIncludeResolver;
    use micold_core::script_path_check::{
        CheckedScriptPath, FakeScriptPathProbe, ProbeAnswer, ScriptPathState,
    };

    /// The OFF note (contracts/settings-indication.md §2).
    const OFF: &str = "Environment include is off, so no script is sourced. Turning it on will \
                       not source one until this path names a readable file.";

    /// An absolute path on every OS, which the fake probe answers for.
    fn stored_path() -> String {
        std::env::temp_dir()
            .join("does-not-exist.sh")
            .to_str()
            .expect("utf-8 temp dir")
            .to_string()
    }

    /// Environment include off, `path` stored, and a probe that answers `answer`.
    fn app_with(path: &str, answer: ProbeAnswer) -> (App, Arc<FakeScriptPathProbe>) {
        let probe = Arc::new(FakeScriptPathProbe::answering(answer));
        let mut app = base_app();
        app.caps = app.caps.clone().with_script_path_probe(probe.clone());
        app.env_include_enabled = false;
        app.env_include_script_path = path.to_string();
        (app, probe)
    }

    /// Open Settings through the shell and let its check land.
    fn open_and_check(app: &mut App) -> Vec<NoticeLine> {
        let job = crate::shell::persist::open_settings(app);
        let landed = job.run();
        app.core.update(landed);
        script_path_notice(
            &app.core.settings.script_check,
            &app.env_include_last_outcome,
        )
    }

    #[test]
    fn off_with_a_missing_stored_path_the_page_says_it_was_not_found_and_that_the_feature_is_off() {
        let path = stored_path();
        let (mut app, _probe) = app_with(&path, ProbeAnswer::Missing);

        assert_eq!(
            open_and_check(&mut app),
            vec![
                NoticeLine::Caution(format!("Script not found: {path}")),
                NoticeLine::Note(OFF.to_string()),
            ],
            "issue #435: with the feature off a missing path was never reported (US1 scenario 1)"
        );
    }

    #[test]
    fn off_with_an_existing_stored_file_the_page_says_nothing() {
        let (mut app, _probe) = app_with(&stored_path(), ProbeAnswer::File);

        assert_eq!(
            open_and_check(&mut app),
            Vec::<NoticeLine>::new(),
            "a readable file is not a problem to report (US1 scenario 2, SC-002)"
        );
    }

    #[test]
    fn off_with_a_blank_stored_path_the_page_says_nothing_and_nothing_is_examined() {
        let (mut app, probe) = app_with("", ProbeAnswer::Missing);

        assert_eq!(
            open_and_check(&mut app),
            Vec::<NoticeLine>::new(),
            "a blank path is not a missing script (US1 scenario 3, FR-011)"
        );
        assert!(probe.calls().is_empty(), "a blank path must not be probed");
    }

    #[test]
    fn off_with_a_missing_stored_path_a_session_launch_examines_and_sources_nothing() {
        let (mut app, probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        let resolver = Arc::new(FakeEnvIncludeResolver::default());
        app.caps = app.caps.clone().with_env_include(resolver.clone());
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(std::env::temp_dir());
        let id = SessionId::new();

        let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

        assert!(
            matches!(rx.try_recv(), Ok(ClientMsg::SessionStart { session }) if session == id),
            "the launch proceeds as before (US1 scenario 4)"
        );
        assert!(
            probe.calls().is_empty(),
            "a launch must not check the path (FR-006, SC-004)"
        );
        assert!(
            resolver.calls().is_empty(),
            "nor source the script with the feature off (FR-003)"
        );
    }

    // --- T014: the triggers (contracts/settings-indication.md §3, T1) ---

    #[test]
    fn opening_settings_seeds_the_draft_at_once_and_checks_the_stored_path_as_it_is_stored() {
        let path = stored_path();
        let (mut app, probe) = app_with(&path, ProbeAnswer::Missing);
        app.env_include_enabled = true;

        let job = crate::shell::persist::open_settings(&mut app);

        assert_eq!(
            app.core
                .settings
                .settings_draft
                .as_ref()
                .map(|d| d.environment.script_path.as_str()),
            Some(path.as_str()),
            "the page opens without waiting for the check (FR-006)"
        );
        assert!(
            matches!(
                app.core.settings.script_check,
                micold_client::features::settings::ScriptCheck::Pending { .. }
            ),
            "the check is under way, got {:?}",
            app.core.settings.script_check
        );
        assert!(
            probe.calls().is_empty(),
            "nothing is examined on the UI thread"
        );

        let Message::Settings(SettingsMsg::ScriptPathChecked { origin, result, .. }) = job.run()
        else {
            panic!("a check reports through the settings reducer");
        };
        assert_eq!(
            origin,
            micold_client::features::settings::CheckOrigin::Opened
        );
        assert_eq!(
            probe.calls(),
            vec![PathBuf::from(&path)],
            "the stored path, exactly as stored"
        );
        assert_eq!(
            result.map(|c| (c.path, c.enabled)),
            Some((path, true)),
            "the answer describes the stored path and the stored enabled flag (research R8)"
        );
    }

    #[test]
    fn a_terminal_restart_does_not_check_the_path() {
        let project = std::env::temp_dir();
        let project_text = project.to_str().expect("utf-8 temp dir").to_string();
        let (mut app, probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));
        app.core.workspace.active = Some(project);
        let id = SessionId::new();
        let _ = connect(
            &mut app,
            snapshot_with(
                &project_text,
                vec![summary(id, "s", WireLifecycle::Running)],
            ),
        );
        app.core.session.active = Some(id);
        app.env_include_cache.clear();

        let _ = update_inner(
            &mut app,
            Message::Session(SessionMsg::TerminalRestartRequested),
        );

        assert!(
            !app.env_include_cache.is_empty(),
            "the restart took its path: it re-resolved the session's directory"
        );
        assert!(
            probe.calls().is_empty(),
            "a restart must not check the path (FR-006, SC-004)"
        );
    }

    // --- M2: the save-time notification (contracts/settings-indication.md §1 S5–S7, §3 T2) ---

    /// Environment include off, `path` stored, a probe that finds nothing there, a recording
    /// settings store, and a resolver that sources nothing: a save with the feature on must not
    /// run a real shell over the shared temp directory. Settings is open and its check has landed.
    fn saving_app(
        path: &str,
        store: micold_core::settings::FakeSettingsStore,
    ) -> (
        App,
        Arc<FakeScriptPathProbe>,
        Arc<micold_core::settings::FakeSettingsStore>,
    ) {
        let (mut app, probe) = app_with(path, ProbeAnswer::Missing);
        let store = Arc::new(store);
        app.caps = app
            .caps
            .clone()
            .with_settings(store.clone())
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));
        let _ = open_and_check(&mut app);
        (app, probe, store)
    }

    /// The open draft as validated settings, the way Save sees it.
    fn valid_draft(app: &App) -> micold_client::features::settings::ValidSettings {
        app.core
            .settings
            .settings_draft
            .clone()
            .expect("Settings is open")
            .validate()
            .expect("the draft is valid")
    }

    /// Save the open draft through the shell and let the save's check land.
    fn save_and_check(app: &mut App) {
        let valid = valid_draft(app);
        let (_survival, job) = crate::shell::persist::save_and_prepare_check(app, valid);
        if let Some(job) = job {
            app.core.update(job.run());
        }
    }

    /// Every notification raised so far, oldest first, taking each off the queue.
    fn drain_notifications(app: &mut App) -> Vec<micold_core::notify::Notification> {
        let mut raised = Vec::new();
        while let Some(visible) = app.core.notifications.queue.visible().cloned() {
            raised.push(visible);
            app.core.notifications.queue.dismiss();
        }
        raised
    }

    #[test]
    fn a_save_with_a_missing_path_saves_and_posts_one_notice_naming_it_with_the_feature_off_or_on()
    {
        let path = stored_path();
        let (mut app, _probe, store) =
            saving_app(&path, micold_core::settings::FakeSettingsStore::new());
        let notice = micold_core::notify::Notification::new(
            micold_core::notify::Level::Info,
            format!("The environment-include script was not found: {path}"),
        );

        for enabled in [false, true] {
            let _ = open_and_check(&mut app);
            let _ = drain_notifications(&mut app);
            app.core
                .settings
                .settings_draft
                .as_mut()
                .expect("Settings is open")
                .environment
                .enabled = enabled;

            save_and_check(&mut app);

            assert_eq!(
                store
                    .saves()
                    .last()
                    .map(|s| (s.env_include_enabled, s.env_include_script_path.clone())),
                Some((enabled, path.clone())),
                "the save goes through with the path missing (FR-004), enabled = {enabled}"
            );
            assert_eq!(
                drain_notifications(&mut app),
                vec![notice.clone()],
                "US1 scenario 5: one Info notice per save, naming the path, enabled = {enabled}"
            );
        }
    }

    /// U48 (review B). The form holds no pull request switch yet (040 M4), so a Settings save keeps
    /// the stored `pr_status_enabled` rather than writing it off (FR-030).
    #[test]
    fn a_settings_save_keeps_the_stored_pr_status_switch() {
        let path = stored_path();
        let stored = micold_core::settings::Settings {
            pr_status_enabled: true,
            ..Default::default()
        };
        let (mut app, _probe, store) = saving_app(
            &path,
            micold_core::settings::FakeSettingsStore::loaded(stored),
        );

        save_and_check(&mut app);

        assert_eq!(
            store.saves().last().map(|s| s.pr_status_enabled),
            Some(true),
            "a save from the form leaves the switch as stored"
        );
    }

    #[test]
    fn a_save_checks_the_saved_path_even_when_the_path_did_not_change() {
        let path = stored_path();
        let (mut app, probe, _store) =
            saving_app(&path, micold_core::settings::FakeSettingsStore::new());
        let before = probe.calls().len();

        let valid = valid_draft(&app);
        let (_survival, job) = crate::shell::persist::save_and_prepare_check(&mut app, valid);
        let job = job.expect("a save that was written checks the path");

        let Message::Settings(SettingsMsg::ScriptPathChecked {
            seq,
            origin,
            result,
        }) = job.run()
        else {
            panic!("a check reports through the settings reducer");
        };
        assert_eq!(
            origin,
            micold_client::features::settings::CheckOrigin::Saved
        );
        assert_eq!(
            app.core.settings.script_check_save_seq,
            Some(seq),
            "the save's own check is the one to report on"
        );
        assert_eq!(
            probe.calls()[before..],
            [PathBuf::from(&path)],
            "every save checks the stored path, changed or not (FR-004, FR-009)"
        );
        assert_eq!(result.map(|c| c.path), Some(path));
    }

    #[test]
    fn a_save_with_a_missing_path_writes_exactly_the_drafted_settings_and_nothing_of_the_check() {
        let path = stored_path();
        let (mut app, _probe, store) =
            saving_app(&path, micold_core::settings::FakeSettingsStore::new());
        let expected = valid_draft(&app).into_settings();

        save_and_check(&mut app);

        assert_eq!(
            store.saves(),
            vec![expected],
            "one write, holding exactly what was drafted; the check's answer is never persisted \
             (FR-010, SC-005)"
        );
    }

    #[test]
    fn a_save_whose_write_failed_posts_no_notice_about_the_path() {
        let path = stored_path();
        let (mut app, _probe, _store) = saving_app(
            &path,
            micold_core::settings::FakeSettingsStore::new()
                .failing_save(std::io::ErrorKind::PermissionDenied),
        );

        save_and_check(&mut app);

        let raised = drain_notifications(&mut app);
        assert!(
            raised
                .iter()
                .all(|n| n.level == micold_core::notify::Level::Error),
            "the path was never stored, so only the failed write is reported, got {raised:?}"
        );
        assert_eq!(
            raised.len(),
            1,
            "the failed write is reported, got {raised:?}"
        );
    }

    // --- M3: the same report with the feature on or off (A6–A8, US2) ---

    /// The ON note (contracts/settings-indication.md §2).
    const ON: &str = "Environment include is on, but the script cannot be sourced until this path \
                      names a readable file.";

    /// Environment include on with `path` stored and missing, 011's last attempt `MissingScript`
    /// (a resolver that keeps answering so), a recording store, and Settings not yet opened.
    fn on_and_missing(path: &str) -> App {
        let (mut app, _probe) = app_with(path, ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_settings(Arc::new(micold_core::settings::FakeSettingsStore::new()))
            .with_env_include(Arc::new(FakeEnvIncludeResolver::answering(
                Vec::new(),
                micold_core::env_include::EnvIncludeOutcome::MissingScript,
            )));
        app.env_include_enabled = true;
        app.env_include_last_outcome = micold_core::env_include::EnvIncludeOutcome::MissingScript;
        app
    }

    /// The file at the stored path has been created since the last check.
    fn create_the_file(app: &mut App) {
        app.caps = app
            .caps
            .clone()
            .with_script_path_probe(Arc::new(FakeScriptPathProbe::answering(ProbeAnswer::File)));
    }

    /// What another window saved: environment include on, `path` stored (spec 035 T3).
    fn settings_saved_elsewhere(path: &str) -> micold_core::protocol::messages::DaemonSettings {
        micold_core::protocol::messages::DaemonSettings {
            default_ai_cli: AiCli::ClaudeCode,
            scrollback_lines: 5_000,
            env_include_enabled: true,
            env_include_script_path: path.to_string(),
            env_include_timeout_secs: 10,
            pi_activity_component: false,
            tool_server_enabled: true,
            desktop_notifications: true,
            diff_layout: Default::default(),
            cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
            pr_status_enabled: false,
        }
    }

    #[test]
    fn another_windows_save_rechecks_the_new_path_while_settings_is_open() {
        let (mut app, probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));
        let _ = open_and_check(&mut app);
        let new_path = std::env::temp_dir()
            .join("saved-elsewhere.sh")
            .to_str()
            .expect("utf-8 temp dir")
            .to_string();
        let before = probe.calls().len();

        let job = crate::shell::daemon_sync::on_settings_changed(
            &mut app,
            settings_saved_elsewhere(&new_path),
        )
        .expect("an open Settings page is re-checked (T3)");

        assert!(
            matches!(
                app.core.settings.script_check,
                micold_client::features::settings::ScriptCheck::Pending { .. }
            ),
            "the check is under way, got {:?}",
            app.core.settings.script_check
        );
        let Message::Settings(SettingsMsg::ScriptPathChecked { origin, result, .. }) = job.run()
        else {
            panic!("a check reports through the settings reducer");
        };
        assert_eq!(
            origin,
            micold_client::features::settings::CheckOrigin::Opened,
            "only this window's own save notifies (FR-007)"
        );
        assert_eq!(
            probe.calls()[before..],
            [PathBuf::from(&new_path)],
            "every window showing Settings shows the stored path's answer (Edge Cases)"
        );
        assert_eq!(result.map(|c| (c.path, c.enabled)), Some((new_path, true)));
    }

    #[test]
    fn another_windows_save_checks_nothing_while_settings_is_closed() {
        let (mut app, probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));

        let job = crate::shell::daemon_sync::on_settings_changed(
            &mut app,
            settings_saved_elsewhere(&stored_path()),
        );

        assert!(job.is_none(), "no page, no check (FR-006)");
        assert_eq!(
            app.core.settings.script_check,
            micold_client::features::settings::ScriptCheck::Idle
        );
        assert!(probe.calls().is_empty());
    }

    #[test]
    fn showing_settings_sources_nothing() {
        let (mut app, _probe) = app_with(&stored_path(), ProbeAnswer::File);
        let resolver = Arc::new(FakeEnvIncludeResolver::default());
        app.caps = app.caps.clone().with_env_include(resolver.clone());
        app.env_include_enabled = true;
        app.env_include_last_outcome = micold_core::env_include::EnvIncludeOutcome::MissingScript;

        let _ = crate::shell::persist::on_settings_opened(&mut app);

        assert!(
            resolver.calls().is_empty(),
            "showing Settings must not re-source the script (FR-014)"
        );
    }

    #[test]
    fn on_with_a_missing_stored_path_the_page_says_it_was_not_found_once_and_that_the_feature_is_on(
    ) {
        let path = stored_path();
        let mut app = on_and_missing(&path);

        assert_eq!(
            open_and_check(&mut app),
            vec![
                NoticeLine::Caution(format!("Script not found: {path}")),
                NoticeLine::Note(ON.to_string()),
            ],
            "US2 scenario 1: 011's note and the path check say it once, by path (FR-005)"
        );
    }

    #[test]
    fn switching_the_feature_off_and_saving_keeps_the_same_not_found_report() {
        let path = stored_path();
        let mut app = on_and_missing(&path);
        let _ = open_and_check(&mut app);
        app.core
            .settings
            .settings_draft
            .as_mut()
            .expect("Settings is open")
            .environment
            .enabled = false;
        save_and_check(&mut app);

        assert_eq!(
            open_and_check(&mut app),
            vec![
                NoticeLine::Caution(format!("Script not found: {path}")),
                NoticeLine::Note(OFF.to_string()),
            ],
            "US2 scenario 2: switching off does not hide the report (SC-003)"
        );
    }

    #[test]
    fn creating_the_missing_file_clears_the_report_and_with_the_feature_on_says_how_to_source_it() {
        let path = stored_path();
        let mut on = on_and_missing(&path);
        let _ = open_and_check(&mut on);
        create_the_file(&mut on);

        assert_eq!(
            open_and_check(&mut on),
            vec![
                NoticeLine::Caution("The last attempt could not find the script".to_string()),
                NoticeLine::Note(format!(
                    "{path} exists now. Save Settings or restart a session to source it."
                )),
            ],
            "US2 scenario 3, feature on: the stale attempt is explained, not re-sourced (FR-014)"
        );

        let (mut off, _probe) = app_with(&path, ProbeAnswer::Missing);
        let _ = open_and_check(&mut off);
        create_the_file(&mut off);

        assert_eq!(
            open_and_check(&mut off),
            Vec::<NoticeLine>::new(),
            "US2 scenario 3, feature off: a path that became valid is no longer reported (FR-009)"
        );
    }

    // --- M4: recovery is the user's own edit (A9, A10, U62, US3) ---

    /// Settings open on a stored missing path whose check has landed `NotFound`, the feature off:
    /// the not-found indication is showing, and nothing has been posted yet.
    fn reporting_a_missing_path(
        path: &str,
    ) -> (
        App,
        Arc<FakeScriptPathProbe>,
        Arc<micold_core::settings::FakeSettingsStore>,
    ) {
        let (mut app, probe, store) =
            saving_app(path, micold_core::settings::FakeSettingsStore::new());
        assert_eq!(
            app.core.settings.script_check,
            ScriptCheck::Done(CheckedScriptPath {
                path: path.to_string(),
                enabled: false,
                state: ScriptPathState::NotFound { tilde: false },
            }),
            "the not-found indication is showing"
        );
        assert_eq!(
            drain_notifications(&mut app),
            Vec::new(),
            "opening posts nothing"
        );
        (app, probe, store)
    }

    /// The user types `path` into the open draft's Script path field.
    fn type_path(app: &mut App, path: &str) {
        app.core
            .settings
            .settings_draft
            .as_mut()
            .expect("Settings is open")
            .environment
            .script_path = path.to_string();
    }

    #[test]
    fn typing_an_existing_path_and_saving_clears_the_report_without_a_notice() {
        let (mut app, _missing, _store) = reporting_a_missing_path(&stored_path());
        let existing = std::env::temp_dir()
            .join("exists.sh")
            .to_str()
            .expect("utf-8 temp dir")
            .to_string();
        // The path the user types names a readable file.
        let probe = Arc::new(FakeScriptPathProbe::answering(ProbeAnswer::File));
        app.caps = app.caps.clone().with_script_path_probe(probe.clone());
        type_path(&mut app, &existing);

        save_and_check(&mut app);

        assert_eq!(
            probe.calls(),
            vec![PathBuf::from(&existing)],
            "the save checks the path the user typed, once"
        );
        assert_eq!(
            app.core.settings.script_check,
            ScriptCheck::Done(CheckedScriptPath {
                path: existing,
                enabled: false,
                state: ScriptPathState::Present,
            }),
            "the save's check describes the new path (FR-009)"
        );
        assert_eq!(
            drain_notifications(&mut app),
            Vec::new(),
            "a save leaving no missing path posts nothing (SC-006)"
        );
        assert_eq!(
            open_and_check(&mut app),
            Vec::<NoticeLine>::new(),
            "US3 scenario 1: reopened, the page no longer reports the path"
        );
    }

    #[test]
    fn clearing_the_path_and_saving_clears_the_report_without_a_notice() {
        let (mut app, probe, store) = reporting_a_missing_path(&stored_path());
        let probed_before = probe.calls().len();
        type_path(&mut app, "");

        save_and_check(&mut app);

        assert_eq!(
            store
                .saves()
                .last()
                .map(|s| s.env_include_script_path.clone()),
            Some(String::new()),
            "the blank path is what was saved"
        );
        assert_eq!(
            app.core.settings.script_check,
            ScriptCheck::Idle,
            "a blank path is not checked (FR-011)"
        );
        assert_eq!(
            drain_notifications(&mut app),
            Vec::new(),
            "a blank path is not a missing script, so nothing is posted (FR-011)"
        );
        assert_eq!(
            open_and_check(&mut app),
            Vec::<NoticeLine>::new(),
            "US3 scenario 2: reopened, the page no longer reports the path"
        );
        assert_eq!(probe.calls().len(), probed_before, "nothing is examined");
    }

    #[test]
    fn nothing_recovers_on_its_own_only_the_users_draft_is_saved() {
        let path = stored_path();
        let mut app = on_and_missing(&path);
        let store = Arc::new(micold_core::settings::FakeSettingsStore::new());
        app.caps = app.caps.clone().with_settings(store.clone());
        app.env_include_timeout_secs = 7;
        let stored = |app: &App| {
            (
                app.env_include_enabled,
                app.env_include_script_path.clone(),
                app.env_include_timeout_secs,
            )
        };
        let before = stored(&app);

        let _ = open_and_check(&mut app);
        assert_eq!(
            stored(&app),
            before,
            "opening and checking changes no environment-include setting (FR-008)"
        );
        assert!(store.saves().is_empty(), "and writes nothing");

        // The user's own edit: a new timeout, the missing path and the flag left as they are.
        app.core
            .settings
            .settings_draft
            .as_mut()
            .expect("Settings is open")
            .environment
            .timeout_secs = "9".to_string();
        let draft = valid_draft(&app).into_settings();
        save_and_check(&mut app);
        let _ = open_and_check(&mut app);

        assert_eq!(
            store.saves(),
            vec![draft.clone()],
            "only the user's own save writes, and it writes the draft (FR-008)"
        );
        assert_eq!(
            stored(&app),
            (true, path, 9),
            "the drafted timeout is applied; no default path, no blanking, no switching off"
        );
    }

    // --- Close (T043, T045): the real handlers start the check, bounded by the constant ---

    /// Every `ScriptPathChecked` the work a handler returned reports, run to completion.
    fn checks_reported(work: Task<Message>) -> Vec<(CheckOrigin, Option<CheckedScriptPath>)> {
        messages(work)
            .into_iter()
            .filter_map(|m| match m {
                Message::Settings(SettingsMsg::ScriptPathChecked { origin, result, .. }) => {
                    Some((origin, result))
                }
                _ => None,
            })
            .collect()
    }

    /// A check that reached the probe and found nothing at `path`.
    fn not_found_at(path: &str, enabled: bool) -> Option<CheckedScriptPath> {
        Some(CheckedScriptPath {
            path: path.to_string(),
            enabled,
            state: ScriptPathState::NotFound { tilde: false },
        })
    }

    #[test]
    fn opening_settings_hands_back_work_that_runs_the_check() {
        let path = stored_path();
        let (mut app, probe) = app_with(&path, ProbeAnswer::Missing);

        let work = crate::shell::persist::on_settings_opened(&mut app);

        assert_eq!(
            checks_reported(work),
            [(CheckOrigin::Opened, not_found_at(&path, false))],
            "T1: the open's check has to be handed back to run, not prepared and dropped"
        );
        assert_eq!(probe.calls(), [PathBuf::from(&path)]);
    }

    #[test]
    fn saving_settings_hands_back_work_that_runs_the_saves_check() {
        let path = stored_path();
        let (mut app, probe, _store) =
            saving_app(&path, micold_core::settings::FakeSettingsStore::new());
        let before = probe.calls().len();

        let work = crate::shell::persist::on_settings_saved(&mut app);

        assert_eq!(
            checks_reported(work),
            [(CheckOrigin::Saved, not_found_at(&path, false))],
            "T2: the save's check has to be handed back to run (FR-004, FR-009)"
        );
        assert_eq!(probe.calls()[before..], [PathBuf::from(&path)]);
    }

    #[test]
    fn another_windows_save_hands_back_work_that_runs_the_recheck() {
        let (mut app, probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));
        let _ = open_and_check(&mut app);
        let new_path = std::env::temp_dir()
            .join("saved-elsewhere.sh")
            .to_str()
            .expect("utf-8 temp dir")
            .to_string();
        let before = probe.calls().len();

        let work = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::SettingsChanged {
                settings: settings_saved_elsewhere(&new_path),
            },
        );

        assert_eq!(
            checks_reported(work),
            [(CheckOrigin::Opened, not_found_at(&new_path, true))],
            "T3: another window's save has to hand its re-check back to run"
        );
        assert_eq!(probe.calls()[before..], [PathBuf::from(&new_path)]);
    }

    #[test]
    fn a_check_with_no_answer_gives_up_at_the_bound_and_reports_unchecked() {
        let path = stored_path();
        let mut app = base_app();
        app.caps = app
            .caps
            .clone()
            .with_script_path_probe(Arc::new(FakeScriptPathProbe::blocking()));
        app.env_include_enabled = false;
        app.env_include_script_path = path.clone();
        let job =
            crate::shell::env_include::prepare_script_path_check(&mut app, CheckOrigin::Opened);

        let started = std::time::Instant::now();
        let Message::Settings(SettingsMsg::ScriptPathChecked { result, .. }) = job.run() else {
            panic!("a check reports through the settings reducer");
        };
        let waited = started.elapsed();

        assert_eq!(
            result.map(|c| c.state),
            Some(ScriptPathState::Unchecked),
            "a probe that never answers is reported as not checked (FR-006)"
        );
        let bound = micold_core::script_path_check::SCRIPT_PATH_CHECK_BOUND;
        assert!(
            waited >= bound && waited < bound + std::time::Duration::from_secs(3),
            "the job waits for SCRIPT_PATH_CHECK_BOUND ({bound:?}) and no longer, waited {waited:?}"
        );
    }
}

// --- feature 034: the GitHub issue source, through the shell (T022, T066) --------------------
//
// The reducer half is `tests/issue_source_state.rs`. These drive what only the shell does: asking
// the daemon for the remotes, locating `gh` and running the load off the update thread, caching
// the environment-include snapshot the load resolved. `gh` itself is never run: every test gets
// its tooling from `issue_rig`, which locates a fake path (or none) and hands out a
// `FakeIssueSource`.

mod issue_source {
    use super::*;
    use crate::shell::capabilities::IssueTooling;
    use micold_client::features::worktree_form::{BranchSource, IssueList, WorktreeForm};
    use micold_core::git::GitRemote;
    use micold_core::github::{
        DescriptionPage, FakeIssueSource, Issue, IssueLoadError, IssuePage, IssueSource,
    };
    use micold_core::naming::ConventionalType;
    use micold_core::protocol::messages::{ErrorKind, OperationResult};
    use micold_core::typeahead::Direction;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    const PROJECT: &str = "/repo/demo";
    const FAKE_GH: &str = "/fake/bin/gh";

    /// An app wired to fake issue tooling, and what that tooling saw.
    pub(super) struct IssueRig {
        pub app: App,
        pub rx: iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
        /// The `PATH` contributed by the environment include, per `gh` lookup.
        pub located_with: Arc<Mutex<Vec<Option<String>>>>,
        /// The `gh` path each constructed source was given.
        pub constructed: Arc<Mutex<Vec<PathBuf>>>,
        pub source: Arc<FakeIssueSource>,
    }

    fn issue(number: u64, title: &str, labels: &[&str]) -> Issue {
        Issue::new(
            number,
            title.to_string(),
            labels.iter().map(|l| l.to_string()).collect(),
            "2026-09-29T00:00:00Z".to_string(),
        )
    }

    /// Three open issues, most recently updated first.
    fn issues() -> Vec<Issue> {
        vec![
            issue(7, "Sidebar flickers on resize", &["ui"]),
            issue(42, "Crash when opening empty project", &["bug"]),
            issue(108, "Document the sandbox placement", &["documentation"]),
        ]
    }

    fn page(issues: Vec<Issue>) -> IssuePage {
        IssuePage {
            total_open: issues.len() as u64,
            issues,
            next_cursor: None,
        }
    }

    /// Connected, `PROJECT` active, and tooling that finds `gh` at `gh` (or nowhere).
    fn issue_rig(gh: Option<&str>, source: FakeIssueSource) -> IssueRig {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let located_with: Arc<Mutex<Vec<Option<String>>>> = Arc::default();
        let constructed: Arc<Mutex<Vec<PathBuf>>> = Arc::default();
        let source = Arc::new(source);
        let found = gh.map(PathBuf::from);
        let tooling = IssueTooling {
            locate_gh: {
                let located_with = Arc::clone(&located_with);
                Arc::new(move |path: Option<&str>| {
                    located_with.lock().unwrap().push(path.map(str::to_string));
                    found.clone()
                })
            },
            source: {
                let constructed = Arc::clone(&constructed);
                let source = Arc::clone(&source);
                Arc::new(move |gh: PathBuf| {
                    constructed.lock().unwrap().push(gh);
                    Arc::clone(&source) as Arc<dyn IssueSource + Send + Sync>
                })
            },
            pull_requests: IssueTooling::none().pull_requests,
        };
        let mut app = base_app();
        app.caps = app.caps.clone().with_issue_tooling(tooling);
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(PathBuf::from(PROJECT));
        IssueRig {
            app,
            rx,
            located_with,
            constructed,
            source,
        }
    }

    /// Run `work` and every message it produces back through the shell until nothing is left.
    fn settle(app: &mut App, work: Task<Message>) {
        let mut queue: VecDeque<Message> = messages(work).into();
        while let Some(message) = queue.pop_front() {
            queue.extend(messages(update_inner(app, message)));
        }
    }

    fn send(app: &mut App, msg: FormMsg) {
        let work = update_inner(app, Message::WorktreeForm(msg));
        settle(app, work);
    }

    fn form(app: &App) -> &WorktreeForm {
        app.core
            .worktree_form
            .form
            .as_ref()
            .expect("the form is open")
    }

    /// Every `RemoteList` sent so far, drained from the outbox.
    fn remote_lists_sent(
        rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) -> Vec<(u64, PathBuf)> {
        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let ClientMsg::RemoteList { req, project } = msg {
                sent.push((req, project));
            }
        }
        sent
    }

    fn github_remote() -> Vec<GitRemote> {
        vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }]
    }

    fn answer_remotes(app: &mut App, req: u64, remotes: Vec<GitRemote>) {
        let work = shell::daemon_sync::on_daemon_event(
            app,
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RemoteList { remotes },
            },
        );
        settle(app, work);
    }

    /// Open the form and answer its `RemoteList` with `remotes`.
    fn open_with(rig: &mut IssueRig, remotes: Vec<GitRemote>) {
        send(&mut rig.app, FormMsg::Opened);
        let sent = remote_lists_sent(&mut rig.rx);
        let (req, _) = *sent.first().expect("opening the form asks for the remotes");
        answer_remotes(&mut rig.app, req, remotes);
    }

    /// A rig on the issue source with `issues()` loaded.
    fn loaded_rig() -> IssueRig {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues())),
        );
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        rig
    }

    fn offered(app: &App) -> Vec<u64> {
        (0..form(app).issue_matches.len())
            .map(|i| app.core.worktree_form.issue_number_at(i).expect("a row"))
            .collect()
    }

    fn pick(app: &mut App, number: u64) {
        send(
            app,
            FormMsg::IssuePicked {
                number,
                mapping: vec![],
            },
        );
    }

    // --- T022: the shell's half -----------------------------------------------------------------

    /// U58 — opening the form sends one `RemoteList` for the active project; its answer decides the
    /// source, and an answer for a project that is no longer active is dropped (FR-002).
    #[test]
    fn issue_opening_the_form_asks_for_remotes() {
        use micold_client::features::worktree_form::GithubAvailability;
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        send(&mut rig.app, FormMsg::Opened);
        let sent = remote_lists_sent(&mut rig.rx);
        assert_eq!(sent.len(), 1, "exactly one RemoteList, got {sent:?}");
        assert_eq!(sent[0].1, PathBuf::from(PROJECT));

        answer_remotes(&mut rig.app, sent[0].0, github_remote());
        assert!(matches!(
            form(&rig.app).github,
            GithubAvailability::Available(_)
        ));

        // An error reply reads as the reason, on the form rather than as a toast.
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        send(&mut rig.app, FormMsg::Opened);
        let (req, _) = remote_lists_sent(&mut rig.rx)[0];
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            DaemonMsg::OperationError {
                req,
                kind: ErrorKind::GitFailed,
                message: "git remote failed".into(),
                detail: None,
            },
        );
        settle(&mut rig.app, work);
        assert_eq!(
            form(&rig.app).github,
            GithubAvailability::Unavailable(
                "Couldn't read this repository's remotes: git remote failed".into()
            )
        );
        assert!(nothing_was_reported(&rig.app), "the form says it; no toast");

        // The user switched project while the question was out.
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        send(&mut rig.app, FormMsg::Opened);
        let (req, _) = remote_lists_sent(&mut rig.rx)[0];
        rig.app.core.workspace.active = Some(PathBuf::from("/repo/other"));
        answer_remotes(&mut rig.app, req, github_remote());
        assert_eq!(
            form(&rig.app).github,
            GithubAvailability::Checking,
            "another project's remotes must not decide this form's source"
        );
    }

    /// U59 — with no connection the remotes cannot be read: the form says so, with no toast; and a
    /// connection that drops with the question out resolves it the same way (FR-002, FR-024).
    #[test]
    fn issue_remotes_without_a_connection() {
        use micold_client::features::worktree_form::GithubAvailability;
        let unreachable = GithubAvailability::Unavailable(
            "Couldn't read this repository's remotes: not connected to the session service".into(),
        );
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        rig.app.daemon = None;
        send(&mut rig.app, FormMsg::Opened);
        assert_eq!(form(&rig.app).github, unreachable);
        assert!(
            nothing_was_reported(&rig.app),
            "the caption says it; no toast"
        );

        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        send(&mut rig.app, FormMsg::Opened);
        let work = shell::daemon_sync::on_disconnected(&mut rig.app);
        settle(&mut rig.app, work);
        assert_eq!(form(&rig.app).github, unreachable);
        assert!(
            nothing_was_reported(&rig.app),
            "a read-only question needs no 'may or may not have taken effect' notice"
        );
    }

    /// U60 — `gh` is looked for on the `PATH` the environment include contributes; a cache miss
    /// resolves inside the load and lands in the cache, and a hit is reused (FR-026, R3).
    #[test]
    fn issue_the_load_uses_the_env_include_path() {
        let resolver = Arc::new(micold_core::env_include::FakeEnvIncludeResolver::answering(
            vec![("PATH".to_string(), "/opt/gh/bin".to_string())],
            EnvIncludeOutcome::Success,
        ));
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new()
                .with_page(page(issues()))
                .with_page(page(issues())),
        );
        rig.app.caps = rig.app.caps.clone().with_env_include(resolver.clone());
        rig.app.env_include_enabled = true;
        rig.app.env_include_script_path = "/home/u/.include.sh".into();
        open_with(&mut rig, github_remote());

        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        let asked: Vec<_> = resolver
            .calls()
            .into_iter()
            .map(|(script, cwd, _)| (script, cwd))
            .collect();
        assert_eq!(
            asked,
            vec![(PathBuf::from("/home/u/.include.sh"), PathBuf::from(PROJECT))],
            "a miss resolves the include once, for the project"
        );
        assert!(
            rig.app.env_include_cache.contains_key(Path::new(PROJECT)),
            "the snapshot the load resolved is kept"
        );

        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::New));
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert_eq!(resolver.calls().len(), 1, "a hit is reused");
        assert_eq!(
            *rig.located_with.lock().unwrap(),
            vec![Some("/opt/gh/bin".to_string()); 2],
            "both lookups searched the include's PATH"
        );
    }

    /// U61 — no `gh` found: the load fails as tooling missing and no source is built (Edge "tooling
    /// not installed").
    #[test]
    fn issue_missing_gh_is_reported_without_running() {
        let mut rig = issue_rig(None, FakeIssueSource::new());
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert_eq!(
            form(&rig.app).issues,
            IssueList::Failed {
                error: IssueLoadError::ToolMissing
            }
        );
        assert!(rig.constructed.lock().unwrap().is_empty());
        assert!(rig.source.calls().is_empty());
    }

    // --- T066: US1 acceptance ---------------------------------------------------------------------

    /// A1 — three sources; the issue one is enabled only with a GitHub remote (US1-1).
    #[test]
    fn issue_the_form_offers_a_github_issue_source() {
        use micold_client::features::worktree_form::GithubAvailability;
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        open_with(&mut rig, github_remote());
        assert_eq!(
            form(&rig.app).github,
            GithubAvailability::Available(
                micold_core::github::GithubRepo::from_remote_url("https://github.com/o/r")
                    .expect("a github.com URL names a repository")
            ),
            "origin on github.com makes the issue source available for o/r"
        );
        assert_eq!(
            form(&rig.app).source_caption().as_deref(),
            Some("GitHub issue reads open issues of o/r from GitHub."),
            "the enabled chip's caption names the repository it reads"
        );
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert_eq!(
            form(&rig.app).source,
            BranchSource::Issue,
            "with a GitHub remote the issue source can be chosen"
        );

        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        open_with(&mut rig, vec![]);
        assert_eq!(
            form(&rig.app).source_caption().as_deref(),
            Some("This repository has no GitHub remote."),
            "without a GitHub remote the caption says why the chip is disabled"
        );
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert_eq!(
            form(&rig.app).source,
            BranchSource::New,
            "without a GitHub remote the issue source cannot be chosen"
        );
    }

    /// A2 — choosing the source loads through the source and lists its issues in its order (US1-2).
    #[test]
    fn issue_choosing_the_source_lists_open_issues() {
        let rig = loaded_rig();
        assert_eq!(
            rig.source.calls(),
            vec![("o/r".to_string(), None)],
            "one load of o/r's first page"
        );
        assert_eq!(
            *rig.constructed.lock().unwrap(),
            vec![PathBuf::from(FAKE_GH); 2],
            "the source runs the gh that was located, for the list and for its descriptions"
        );
        let f = form(&rig.app);
        let rows: Vec<String> = f
            .issue_matches
            .iter()
            .map(|(held, _)| f.issues.held()[*held].row_text().to_string())
            .collect();
        assert_eq!(
            rows,
            vec![
                "#7 Sidebar flickers on resize  ·  ghost  ·  ui",
                "#42 Crash when opening empty project  ·  ghost  ·  bug",
                "#108 Document the sandbox placement  ·  ghost  ·  documentation",
            ],
            "each row's match text is number, title, reporter and labels, in the source's order"
        );
        assert!(f.issue_list_open, "the list opens on arrival");
    }

    /// Each loaded issue's number and description, in the list's order.
    fn described(app: &App) -> Vec<(u64, String)> {
        let IssueList::Loaded { listing, .. } = &form(app).issues else {
            panic!("the list is not loaded");
        };
        listing
            .issues
            .iter()
            .map(|issue| (issue.number(), issue.description().to_string()))
            .collect()
    }

    fn descriptions(pairs: &[(u64, &str)], next: Option<&str>) -> DescriptionPage {
        DescriptionPage {
            descriptions: pairs
                .iter()
                .map(|(n, text)| (*n, text.to_string()))
                .collect(),
            next_cursor: next.map(str::to_string),
        }
    }

    /// 038 U100 — choosing the source reads the list, then its descriptions a page at a time in
    /// order, each with the `gh` that was located, and the rows hold them (FR-024, US3-1).
    #[test]
    fn issue_choosing_the_source_reads_the_descriptions_after_the_list() {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new()
                .with_page(page(issues()))
                .with_descriptions(Ok(descriptions(
                    &[(7, "The sidebar"), (42, "It crashes")],
                    Some("P2"),
                )))
                .with_descriptions(Ok(descriptions(&[(108, "The docs")], None))),
        );
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));

        assert_eq!(
            rig.source.calls(),
            vec![("o/r".to_string(), None)],
            "the list is read once"
        );
        assert_eq!(
            rig.source.description_calls(),
            vec![
                ("o/r".to_string(), None),
                ("o/r".to_string(), Some("P2".to_string())),
            ],
            "then the description pages, in order, and no page after the last"
        );
        assert_eq!(
            described(&rig.app),
            vec![
                (7, "The sidebar".to_string()),
                (42, "It crashes".to_string()),
                (108, "The docs".to_string()),
            ]
        );
        assert_eq!(
            *rig.constructed.lock().unwrap(),
            vec![PathBuf::from(FAKE_GH); 3],
            "every request runs the gh the load located"
        );
        assert_eq!(
            rig.located_with.lock().unwrap().len(),
            1,
            "the pass never looks for gh again"
        );
        assert_eq!(offered(&rig.app), vec![7, 42, 108]);
    }

    /// 038 U100 — a failed description page ends the pass: the list stays shown, with no error and
    /// no further request (FR-024).
    #[test]
    fn issue_a_failed_description_page_leaves_the_list_shown() {
        // Nothing scripted: the fake answers the first page with an error.
        let rig = loaded_rig();
        assert_eq!(
            rig.source.description_calls(),
            vec![("o/r".to_string(), None)],
            "one page was asked for, and none after it failed"
        );
        assert!(matches!(form(&rig.app).issues, IssueList::Loaded { .. }));
        assert_eq!(offered(&rig.app), vec![7, 42, 108], "the list is shown");
        assert_eq!(form(&rig.app).error, None);
        assert!(described(&rig.app).iter().all(|(_, text)| text.is_empty()));
        assert_eq!(rig.source.calls().len(), 1, "the list is not read again");
    }

    /// A3 — typing narrows locally with no source call; Down then Enter picks (US1-3).
    #[test]
    fn issue_typing_narrows_the_list_locally() {
        let mut rig = loaded_rig();
        let calls = rig.source.calls().len();
        send(&mut rig.app, FormMsg::IssueQueryChanged("flicker".into()));
        assert_eq!(offered(&rig.app), vec![7], "a title fragment narrows");
        send(
            &mut rig.app,
            FormMsg::IssueQueryChanged("documentation".into()),
        );
        assert_eq!(offered(&rig.app), vec![108], "a label narrows");
        send(&mut rig.app, FormMsg::IssueQueryChanged("42".into()));
        assert_eq!(
            offered(&rig.app).first(),
            Some(&42),
            "a number finds its issue first"
        );
        assert_eq!(rig.source.calls().len(), calls, "search is local (FR-005)");

        send(&mut rig.app, FormMsg::IssueFocused);
        send(&mut rig.app, FormMsg::IssueHighlightMoved(Direction::Next));
        assert_eq!(
            form(&rig.app).issue_highlight,
            Some(0),
            "Down from the field lands on the first match"
        );
        assert_eq!(
            rig.app.core.worktree_form.issue_number_at(0),
            Some(42),
            "the first match for `42` is #42"
        );
        // Enter picks the highlighted row, as the list reports it: by index.
        send(&mut rig.app, FormMsg::IssueRowPicked(0));
        assert_eq!(form(&rig.app).ticket, "42", "Enter picked #42");
        assert_eq!(
            form(&rig.app).name,
            "Crash when opening empty project",
            "Enter filled #42's title"
        );
    }

    /// U105 — a row pick resolves its index to that row's issue, and an index past the rows picks
    /// nothing (Review B F1).
    #[test]
    fn issue_a_row_pick_resolves_to_its_issue() {
        let mut rig = loaded_rig();
        send(
            &mut rig.app,
            FormMsg::IssueQueryChanged("documentation".into()),
        );
        send(&mut rig.app, FormMsg::IssueRowPicked(1));
        assert_eq!(
            form(&rig.app).ticket,
            "",
            "one past the last row picks nothing"
        );
        send(&mut rig.app, FormMsg::IssueRowPicked(0));
        assert_eq!(form(&rig.app).ticket, "108");
        assert_eq!(form(&rig.app).picked_issue, Some(108));
    }

    /// U106 — with no project open there are no remotes to read, and the caption says so rather
    /// than checking for ever (Review A).
    #[test]
    fn issue_no_project_is_not_left_checking() {
        use micold_client::features::worktree_form::GithubAvailability;
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        rig.app.core.workspace.active = None;
        send(&mut rig.app, FormMsg::Opened);
        assert_eq!(
            form(&rig.app).github,
            GithubAvailability::Unavailable(
                "Couldn't read this repository's remotes: no project is open".into()
            )
        );
        assert!(remote_lists_sent(&mut rig.rx).is_empty());
    }

    /// A4 — a pick fills ticket and name; the preview is the new-branch one (US1-4).
    #[test]
    fn issue_a_pick_fills_ticket_and_name() {
        let mut rig = loaded_rig();
        pick(&mut rig.app, 42);
        // Chosen after the pick: with no mapping the pick clears the type (FR-014), and a preview
        // with no type is an error on both sources, which would compare equal whatever it said.
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Fix));
        let as_issue = form(&rig.app).clone();
        assert_eq!(as_issue.ticket, "42", "the pick fills the number, no `#`");
        assert_eq!(
            as_issue.name, "Crash when opening empty project",
            "the pick fills the title"
        );
        let preview = as_issue.preview();
        assert!(
            preview.is_ok(),
            "a picked issue with a type previews a branch: {preview:?}"
        );
        assert_eq!(
            preview.as_ref().map(|d| d.branch.as_str()),
            Ok("fix/42_crash-when-opening-empty-project"),
            "the preview is the branch the picked values name"
        );
        let mut as_new = as_issue.clone();
        as_new.source = BranchSource::New;
        assert_eq!(
            preview,
            as_new.preview(),
            "the issue source previews exactly as the new-branch source (FR-012)"
        );
    }

    /// A5 — picked values stay editable, and the preview follows the edit (US1-5).
    #[test]
    fn issue_picked_values_stay_editable() {
        let mut rig = loaded_rig();
        pick(&mut rig.app, 42);
        // Chosen after the pick: with no mapping the pick clears the type (FR-014).
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Fix));
        send(
            &mut rig.app,
            FormMsg::NameChanged("empty project crash".into()),
        );
        send(&mut rig.app, FormMsg::TicketChanged("43".into()));
        assert_eq!(
            form(&rig.app).preview().map(|d| d.branch),
            Ok("fix/43_empty-project-crash".to_string())
        );
    }

    /// The branch each `BranchPreflight` in the outbox asked about.
    fn preflights_sent(
        rx: &mut iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) -> Vec<String> {
        let mut sent = Vec::new();
        while let Ok(msg) = rx.try_recv() {
            if let ClientMsg::BranchPreflight { branch, .. } = msg {
                sent.push(branch);
            }
        }
        sent
    }

    /// A6 — submitting after a pick sends what a new-branch submit sends, and a taken name raises
    /// the existing prompt (US1-6).
    #[test]
    fn issue_submit_creates_like_a_new_branch() {
        let mut rig = loaded_rig();
        pick(&mut rig.app, 42);
        // Chosen after the pick: with no mapping the pick clears the type (FR-014).
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Fix));
        let _ = remote_lists_sent(&mut rig.rx);
        send(&mut rig.app, FormMsg::Submitted);
        assert_eq!(
            preflights_sent(&mut rig.rx),
            vec!["fix/42_crash-when-opening-empty-project".to_string()]
        );

        let req = *rig
            .app
            .pending_ops
            .iter()
            .find(|(_, op)| matches!(op, PendingOp::BranchPreflight { picked: false, .. }))
            .expect("the issue source pre-flights as a new branch, not a picked one")
            .0;
        let branch = "fix/42_crash-when-opening-empty-project".to_string();
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::BranchPreflight {
                    situation: micold_core::worktree::BranchSituation::LocalAvailable { branch },
                },
            },
        );
        settle(&mut rig.app, work);
        assert!(
            form(&rig.app).resolution.is_prompting(),
            "a taken branch raises the conflict prompt"
        );
    }

    /// A7 — the form stays usable while the load runs; the late result then changes nothing
    /// (US1-7).
    #[test]
    fn issue_the_form_stays_usable_while_loading() {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues())),
        );
        open_with(&mut rig, github_remote());
        // Take the load's work without running it: the result is "in flight".
        let in_flight = update_inner(
            &mut rig.app,
            Message::WorktreeForm(FormMsg::SourceChanged(BranchSource::Issue)),
        );
        assert!(matches!(form(&rig.app).issues, IssueList::Loading { .. }));

        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::New));
        send(&mut rig.app, FormMsg::NameChanged("typed".into()));
        assert_eq!(form(&rig.app).source, BranchSource::New);

        settle(&mut rig.app, in_flight);
        assert_eq!(form(&rig.app).issues, IssueList::NotRequested);
        assert_eq!(form(&rig.app).name, "typed");
    }

    /// A8 — a pick replaces a typed ticket and name, and an earlier pick (US1-8).
    #[test]
    fn issue_a_pick_replaces_ticket_and_name() {
        let mut rig = loaded_rig();
        send(&mut rig.app, FormMsg::TicketChanged("1".into()));
        send(&mut rig.app, FormMsg::NameChanged("typed".into()));
        pick(&mut rig.app, 7);
        assert_eq!(form(&rig.app).ticket, "7");
        assert_eq!(form(&rig.app).name, "Sidebar flickers on resize");
        pick(&mut rig.app, 108);
        assert_eq!(form(&rig.app).ticket, "108");
        assert_eq!(form(&rig.app).name, "Document the sandbox placement");
    }

    /// A9 — before the choice the caption names the repository and nothing was fetched; after it,
    /// the body notice names it (US1-9, FR-025, FR-003).
    #[test]
    fn issue_the_source_says_it_contacts_github_before_it_does() {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues())),
        );
        open_with(&mut rig, github_remote());
        assert_eq!(
            form(&rig.app).source_caption().as_deref(),
            Some("GitHub issue reads open issues of o/r from GitHub.")
        );
        assert!(
            rig.source.calls().is_empty(),
            "nothing is sent before the choice"
        );
        assert!(rig.located_with.lock().unwrap().is_empty());

        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert_eq!(
            form(&rig.app).issue_notice().as_deref(),
            Some("Reads open issues of o/r from GitHub.")
        );
        assert_eq!(rig.source.calls().len(), 1);
    }

    /// U122 — opening the form, rendering it and moving between the other sources contact GitHub
    /// never: no `gh` is located, no source built, no load or search run (FR-003). The behaviour
    /// half of U62's caller scan.
    #[test]
    fn issue_opening_the_form_loads_no_issues() {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues())),
        );
        open_with(&mut rig, github_remote());
        let _ = view(&rig.app);
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Existing));
        let _ = view(&rig.app);
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::New));
        send(&mut rig.app, FormMsg::NameChanged("typed".into()));
        let _ = view(&rig.app);

        assert!(
            rig.located_with.lock().unwrap().is_empty(),
            "no gh lookup before the issue source is chosen"
        );
        assert!(
            rig.constructed.lock().unwrap().is_empty(),
            "no issue source built before it is chosen"
        );
        assert!(
            rig.source.calls().is_empty(),
            "no issue load before the issue source is chosen: {:?}",
            rig.source.calls()
        );
        assert!(
            rig.source.search_calls().is_empty(),
            "no issue search before the issue source is chosen"
        );
        assert_eq!(
            form(&rig.app).issues,
            IssueList::NotRequested,
            "the listing was never requested"
        );
    }

    // --- T036, T076: the search beyond the loaded issues (FR-005a) ------------------------------

    /// 1,000 loaded of 1,200 open: #1000 down to #1, and #17 mentions #1100 in its title.
    fn capped_page() -> IssuePage {
        IssuePage {
            issues: (1..=1000)
                .rev()
                .map(|n| {
                    if n == 17 {
                        issue(17, "Follow-up to 1100", &["bug"])
                    } else {
                        issue(n, "Loaded issue", &[])
                    }
                })
                .collect(),
            total_open: 1_200,
            next_cursor: Some("more".into()),
        }
    }

    /// A rig on the issue source with `capped_page()` loaded and `source`'s searches scripted.
    fn capped_rig(source: FakeIssueSource) -> IssueRig {
        let mut rig = issue_rig(Some(FAKE_GH), source.with_page(capped_page()));
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert!(
            matches!(
                &form(&rig.app).issues,
                IssueList::Loaded { listing, .. } if !listing.complete
            ),
            "the capped page loaded as an incomplete listing: {:?}",
            std::mem::discriminant(&form(&rig.app).issues)
        );
        rig
    }

    /// Type `text` without running what the keystroke scheduled.
    fn keystroke(app: &mut App, text: &str) -> Task<Message> {
        update_inner(
            app,
            Message::WorktreeForm(FormMsg::IssueQueryChanged(text.into())),
        )
    }

    /// U74 — a keystroke on a capped list schedules one debounce; its end runs the search with the
    /// `gh` the list was read with, and an older keystroke's end runs nothing (R9, R3).
    #[test]
    fn issue_search_is_debounced() {
        let mut rig = capped_rig(FakeIssueSource::new().with_search(Ok(vec![issue(
            1100,
            "Beyond the cap",
            &[],
        )])));
        let started = std::time::Instant::now();
        let first = messages(keystroke(&mut rig.app, "beyond"));
        assert!(
            started.elapsed() >= shell::issues::ISSUE_SEARCH_DEBOUNCE,
            "the search waits out the debounce"
        );
        let [Message::WorktreeForm(FormMsg::IssueSearchDue { seq: stale })] = first[..] else {
            panic!("one keystroke schedules exactly one IssueSearchDue");
        };
        assert!(rig.source.search_calls().is_empty(), "not before it is due");

        let newer = keystroke(&mut rig.app, "beyond the");
        let work = update_inner(
            &mut rig.app,
            Message::WorktreeForm(FormMsg::IssueSearchDue { seq: stale }),
        );
        settle(&mut rig.app, work);
        assert!(
            rig.source.search_calls().is_empty(),
            "an older keystroke's debounce searches nothing"
        );

        settle(&mut rig.app, newer);
        assert_eq!(
            rig.source.search_calls(),
            vec![("o/r".to_string(), "beyond the".to_string())]
        );
        assert_eq!(
            *rig.constructed.lock().unwrap(),
            vec![PathBuf::from(FAKE_GH); 3],
            "the description pass and the search run the gh the list was read with"
        );
        assert_eq!(
            rig.located_with.lock().unwrap().len(),
            1,
            "gh is located once, for the load"
        );
    }

    /// U120 — typing on a listing that holds every open issue never asks GitHub: no debounce is
    /// scheduled and the source's search is never called (FR-005a; mutant M13 at the shell).
    #[test]
    fn issue_typing_on_a_complete_list_never_searches() {
        let mut rig = loaded_rig();
        let scheduled = messages(keystroke(&mut rig.app, "crash"));
        assert!(
            scheduled.is_empty(),
            "a complete listing schedules no search debounce, got {} message(s)",
            scheduled.len()
        );
        send(
            &mut rig.app,
            FormMsg::IssueQueryChanged("crash when".into()),
        );
        assert!(
            rig.source.search_calls().is_empty(),
            "every open issue is held, so GitHub is not searched: {:?}",
            rig.source.search_calls()
        );
        assert_eq!(
            offered(&rig.app),
            vec![42],
            "the loaded matches are all there is"
        );
    }

    /// A10 — with 1,000 loaded of 1,200 open, typing an unloaded issue's number shows the loaded
    /// matches at once, then the searched issue joins them once and can be picked (US1-10,
    /// FR-005a).
    #[test]
    fn issue_search_finds_an_issue_beyond_the_cap() {
        let mut rig = capped_rig(FakeIssueSource::new().with_search(Ok(vec![
            issue(1100, "Beyond the cap", &["enhancement"]),
            issue(17, "Follow-up to 1100", &["bug"]),
            issue(5555, "Unrelated work", &[]),
        ])));

        let scheduled = keystroke(&mut rig.app, "1100");
        let at_once = offered(&rig.app);
        assert!(
            at_once.contains(&17) && !at_once.contains(&1100),
            "the loaded matches are shown at once, before GitHub answers: {at_once:?}"
        );
        assert!(rig.source.search_calls().is_empty());

        settle(&mut rig.app, scheduled);
        assert_eq!(
            rig.source.search_calls(),
            vec![("o/r".to_string(), "1100".to_string())]
        );
        let shown = offered(&rig.app);
        assert_eq!(
            shown.iter().filter(|n| **n == 1100).count(),
            1,
            "the issue beyond the cap is found: {shown:?}"
        );
        assert_eq!(shown.iter().filter(|n| **n == 17).count(), 1, "#17 once");
        assert!(
            !shown.contains(&5555),
            "#5555 matched only in its body: held to FR-005's rule"
        );

        let row = shown.iter().position(|n| *n == 1100).unwrap();
        send(&mut rig.app, FormMsg::IssueRowPicked(row));
        assert_eq!(form(&rig.app).ticket, "1100");
        assert_eq!(form(&rig.app).name, "Beyond the cap");
    }

    // --- US2: the issue's labels choose the type (feature 034 M4) ------------------------------

    use micold_core::issue_types::LabelTypeEntry;
    use micold_core::settings::{FakeSettingsStore, SettingsStore};

    fn entry(label: &str, type_: ConventionalType) -> LabelTypeEntry {
        LabelTypeEntry {
            label: label.to_string(),
            type_,
        }
    }

    /// Store `mapping` as the application's label-to-type mapping and hand the store to the app.
    fn with_mapping(rig: &mut IssueRig, mapping: Vec<LabelTypeEntry>) -> Arc<FakeSettingsStore> {
        let store = Arc::new(FakeSettingsStore::loaded(micold_core::settings::Settings {
            issue_label_types: mapping,
            ..Default::default()
        }));
        rig.app.caps = rig.app.caps.clone().with_settings(store.clone());
        store
    }

    /// A rig with issues `issues` loaded and `mapping` in the settings store.
    fn mapped_rig(issues: Vec<Issue>, mapping: Vec<LabelTypeEntry>) -> IssueRig {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues)),
        );
        with_mapping(&mut rig, mapping);
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        rig
    }

    /// Pick issue `number` the way the list does: by its row, through the shell.
    fn pick_row(app: &mut App, number: u64) {
        send(app, FormMsg::IssueQueryChanged(String::new()));
        let row = offered(app)
            .iter()
            .position(|n| *n == number)
            .expect("the issue is offered");
        send(app, FormMsg::IssueRowPicked(row));
        assert_eq!(form(app).ticket, number.to_string(), "the row was picked");
    }

    fn default_rig() -> IssueRig {
        mapped_rig(issues(), micold_core::issue_types::default_mapping())
    }

    /// A11 — with the default mapping, an issue labelled `bug` selects `fix` (US2-1).
    #[test]
    fn issue_a_bug_label_selects_fix() {
        let mut rig = default_rig();
        pick_row(&mut rig.app, 42);
        assert_eq!(form(&rig.app).type_, Some(ConventionalType::Fix));
    }

    /// A12 — the mapping entry listed first wins, whatever order the issue lists its labels in
    /// (US2-2).
    #[test]
    fn issue_the_first_mapping_entry_wins() {
        let mut rig = mapped_rig(
            vec![issue(5, "Both kinds", &["enhancement", "bug"])],
            vec![
                entry("bug", ConventionalType::Fix),
                entry("enhancement", ConventionalType::Feat),
            ],
        );
        pick_row(&mut rig.app, 5);
        assert_eq!(form(&rig.app).type_, Some(ConventionalType::Fix));
    }

    /// A13 — picking an issue with no mapped label clears a selected type, and the form asks for
    /// one as it does today (US2-3).
    #[test]
    fn issue_an_unmapped_issue_clears_the_type() {
        let mut rig = default_rig();
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Chore));
        pick_row(&mut rig.app, 7);
        let f = form(&rig.app);
        assert_eq!(f.type_, None, "no type carries over from earlier input");
        assert!(!f.can_submit(), "submitting needs a type again");
        assert!(f.preview().is_err(), "the preview reports the missing type");
    }

    /// A14 — a type chosen after a label-selected one is the one the create request uses (US2-4).
    #[test]
    fn issue_a_label_type_can_be_overridden() {
        let mut rig = default_rig();
        pick_row(&mut rig.app, 42);
        assert_eq!(
            form(&rig.app).type_,
            Some(ConventionalType::Fix),
            "the label pre-selected fix"
        );
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Feat));
        let _ = remote_lists_sent(&mut rig.rx);
        send(&mut rig.app, FormMsg::Submitted);
        assert_eq!(
            preflights_sent(&mut rig.rx),
            vec!["feat/42_crash-when-opening-empty-project".to_string()]
        );
    }

    /// A15 — a mapping entry `Bug` matches an issue label `bug` (US2-5).
    #[test]
    fn issue_label_matching_ignores_case() {
        let mut rig = mapped_rig(issues(), vec![entry("Bug", ConventionalType::Perf)]);
        pick_row(&mut rig.app, 42);
        assert_eq!(form(&rig.app).type_, Some(ConventionalType::Perf));
    }

    /// A16 — a selected type is replaced by the picked issue's mapped one (US2-6).
    #[test]
    fn issue_a_mapped_label_replaces_the_selected_type() {
        let mut rig = default_rig();
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Chore));
        pick_row(&mut rig.app, 108);
        assert_eq!(form(&rig.app).type_, Some(ConventionalType::Docs));
    }

    /// A17 — an issue row's text shows its labels, so the type it will get is visible before the
    /// pick (US2-7). A regression pin: slice B's `row_text` already carries them.
    #[test]
    fn issue_rows_show_labels() {
        let rig = default_rig();
        let f = form(&rig.app);
        let row = f
            .issues
            .held()
            .into_iter()
            .find(|i| i.number() == 42)
            .expect("issue 42 is loaded")
            .row_text()
            .to_string();
        assert!(
            row.contains("bug"),
            "the row shows the `bug` label: {row:?}"
        );
    }

    /// U85 — the shell reads the mapping from the settings store at the pick: a later change to the
    /// stored mapping reaches the next pick and not the one already made; with no store the
    /// default applies (FR-014a).
    #[test]
    fn issue_the_pick_reads_the_stored_mapping() {
        let mut rig = loaded_rig();
        let store = with_mapping(&mut rig, vec![entry("bug", ConventionalType::Chore)]);
        pick_row(&mut rig.app, 42);
        assert_eq!(form(&rig.app).type_, Some(ConventionalType::Chore));

        store
            .save(&micold_core::settings::Settings {
                issue_label_types: vec![entry("bug", ConventionalType::Perf)],
                ..Default::default()
            })
            .expect("the fake saves");
        // Messages through the shell after the save: any that re-read the store and re-typed the
        // picked issue would show here, before the next pick.
        send(&mut rig.app, FormMsg::IssueQueryChanged("crash".into()));
        send(&mut rig.app, FormMsg::IssueFocused);
        send(&mut rig.app, FormMsg::IssueQueryChanged(String::new()));
        assert_eq!(
            form(&rig.app).picked_issue,
            Some(42),
            "the pick is still the one made"
        );
        assert_eq!(
            form(&rig.app).type_,
            Some(ConventionalType::Chore),
            "editing the mapping does not change the issue already picked"
        );
        pick_row(&mut rig.app, 42);
        assert_eq!(
            form(&rig.app).type_,
            Some(ConventionalType::Perf),
            "the next pick reads the mapping as it is now"
        );

        let mut bare = loaded_rig();
        pick_row(&mut bare.app, 42);
        assert_eq!(
            form(&bare.app).type_,
            Some(ConventionalType::Fix),
            "with no settings store the default mapping applies"
        );
    }
    // --- US3: the mapping is edited in Settings → GitHub issues (feature 034 M5) ----------------

    use micold_client::features::settings::SettingsSection;

    /// A rig whose settings store holds `mapping`, with the form not yet opened. The store is the
    /// fake: only the shell may name the real one (`no_concrete_implementations.rs`), and the file
    /// round trip is `micold-core`'s `settings_issue_mapping.rs`.
    fn file_rig(
        issues: Vec<Issue>,
        mapping: Vec<LabelTypeEntry>,
    ) -> (IssueRig, Arc<FakeSettingsStore>) {
        let store = Arc::new(FakeSettingsStore::loaded(micold_core::settings::Settings {
            issue_label_types: mapping,
            ..Default::default()
        }));
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues)),
        );
        rig.app.caps = rig
            .app
            .caps
            .clone()
            .with_settings(store.clone())
            .with_env_include(Arc::new(
                micold_core::env_include::FakeEnvIncludeResolver::default(),
            ));
        (rig, store)
    }

    fn settings(app: &mut App, msg: SettingsMsg) {
        let work = update_inner(app, Message::Settings(msg));
        settle(app, work);
    }

    /// Open Settings on the GitHub issues section.
    fn open_github_section(app: &mut App) {
        settings(app, SettingsMsg::Opened);
        settings(
            app,
            SettingsMsg::SectionShown(SettingsSection::GithubIssues),
        );
    }

    fn shown(app: &App) -> Vec<(String, ConventionalType)> {
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("Settings is open")
            .github
            .entries
            .iter()
            .map(|e| (e.label.clone(), e.type_))
            .collect()
    }

    fn stored(store: &FakeSettingsStore) -> Vec<LabelTypeEntry> {
        store.load().settings.issue_label_types
    }

    /// Open the form, choose the issue source, and pick issue `number` by its row.
    fn open_and_pick(rig: &mut IssueRig, number: u64) -> Option<ConventionalType> {
        open_with(rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        pick_row(&mut rig.app, number);
        form(&rig.app).type_
    }

    fn labelled() -> Vec<Issue> {
        vec![
            issue(11, "Button misaligned", &["defect"]),
            issue(12, "Both kinds", &["bug", "question"]),
        ]
    }

    /// A18 — opening Settings → GitHub issues shows the stored mapping as ordered label → type
    /// entries (US3-1).
    #[test]
    fn issue_settings_shows_the_mapping() {
        let (mut rig, _store) = file_rig(
            labelled(),
            vec![
                entry("defect", ConventionalType::Fix),
                entry("question", ConventionalType::Chore),
            ],
        );
        open_github_section(&mut rig.app);
        let draft = rig.app.core.settings.settings_draft.as_ref().unwrap();
        assert_eq!(draft.section, SettingsSection::GithubIssues);
        assert_eq!(
            shown(&rig.app),
            vec![
                ("defect".to_string(), ConventionalType::Fix),
                ("question".to_string(), ConventionalType::Chore),
            ],
            "the section lists the stored entries in order"
        );
        let _ = view(&rig.app);
    }

    /// A19 — adding `defect → fix` and saving types the next pick of a `defect` issue, with no
    /// restart (US3-2, SC-005).
    #[test]
    fn issue_an_added_entry_types_the_next_pick() {
        let (mut rig, store) = file_rig(labelled(), micold_core::issue_types::default_mapping());
        open_github_section(&mut rig.app);
        settings(&mut rig.app, SettingsMsg::IssueMappingAdded);
        settings(
            &mut rig.app,
            SettingsMsg::IssueMappingLabelChanged(3, "defect".into()),
        );
        settings(
            &mut rig.app,
            SettingsMsg::IssueMappingTypeChanged(3, ConventionalType::Fix),
        );
        settings(&mut rig.app, SettingsMsg::Saved);
        assert!(
            rig.app.core.settings.settings_draft.is_none(),
            "the save went through"
        );
        assert_eq!(
            stored(&store).last(),
            Some(&entry("defect", ConventionalType::Fix))
        );
        assert_eq!(open_and_pick(&mut rig, 11), Some(ConventionalType::Fix));
    }

    /// A20 — changing, removing and reordering entries, then saving, types the next pick by the new
    /// mapping (US3-3).
    #[test]
    fn issue_edited_mapping_types_the_next_pick() {
        let (mut rig, store) = file_rig(
            labelled(),
            vec![
                entry("bug", ConventionalType::Fix),
                entry("defect", ConventionalType::Fix),
                entry("question", ConventionalType::Docs),
            ],
        );
        open_github_section(&mut rig.app);
        settings(&mut rig.app, SettingsMsg::IssueMappingRemoved(1));
        settings(
            &mut rig.app,
            SettingsMsg::IssueMappingTypeChanged(1, ConventionalType::Chore),
        );
        settings(
            &mut rig.app,
            SettingsMsg::IssueMappingMoved(1, Direction::Prev),
        );
        settings(&mut rig.app, SettingsMsg::Saved);
        assert_eq!(
            stored(&store),
            vec![
                entry("question", ConventionalType::Chore),
                entry("bug", ConventionalType::Fix),
            ]
        );
        assert_eq!(
            open_and_pick(&mut rig, 12),
            Some(ConventionalType::Chore),
            "`question` now comes first, so it wins over `bug`"
        );
        pick_row(&mut rig.app, 11);
        assert_eq!(
            form(&rig.app).type_,
            None,
            "the removed `defect` entry no longer types anything"
        );
    }

    /// A21 — a save hands the edited mapping to the settings store (US3-4, FR-020). That the store
    /// writes it to `settings.json` and a fresh store reads it back is `micold-core`'s
    /// `settings_issue_mapping.rs::round_trip_and_default`; only the shell's half is pinned here.
    #[test]
    fn issue_the_mapping_survives_a_restart() {
        let (mut rig, store) = file_rig(labelled(), micold_core::issue_types::default_mapping());
        open_github_section(&mut rig.app);
        settings(&mut rig.app, SettingsMsg::IssueMappingAdded);
        settings(
            &mut rig.app,
            SettingsMsg::IssueMappingLabelChanged(3, "defect".into()),
        );
        settings(&mut rig.app, SettingsMsg::Saved);

        let mut expected = micold_core::issue_types::default_mapping();
        expected.push(entry("defect", ConventionalType::Feat));
        assert_eq!(
            store.saves().last().map(|s| s.issue_label_types.clone()),
            Some(expected),
            "the save hands the store the edited mapping"
        );
    }

    /// Open Settings → GitHub issues, add an entry labelled `label`, and save.
    fn save_with_added_label(rig: &mut IssueRig, label: &str) {
        open_github_section(&mut rig.app);
        settings(&mut rig.app, SettingsMsg::IssueMappingAdded);
        if !label.is_empty() {
            settings(
                &mut rig.app,
                SettingsMsg::IssueMappingLabelChanged(3, label.into()),
            );
        }
        settings(&mut rig.app, SettingsMsg::Saved);
    }

    /// The field and section a refused save marked.
    fn refusal(app: &App) -> (micold_client::features::window::FieldId, SettingsSection) {
        let draft = app
            .core
            .settings
            .settings_draft
            .as_ref()
            .expect("a refused save leaves Settings open");
        let error = draft.error.as_ref().expect("the refusal is reported");
        (error.field, error.section)
    }

    /// A22 (blank) — a blank label refuses the save with an error on that entry, and nothing is
    /// written (US3-5, FR-019).
    #[test]
    fn issue_an_invalid_mapping_blank_label_is_not_saved() {
        use micold_client::features::window::FieldId;
        let (mut rig, store) = file_rig(labelled(), micold_core::issue_types::default_mapping());
        save_with_added_label(&mut rig, "");
        assert_eq!(
            refusal(&rig.app),
            (FieldId::IssueMappingLabel(3), SettingsSection::GithubIssues),
            "the blank entry is the one marked"
        );
        assert!(
            store.saves().is_empty(),
            "a blank label writes nothing: {:?}",
            store.saves()
        );
    }

    /// A22 (duplicate) — `Bug` beside `bug` refuses the save with an error on that entry, and
    /// nothing is written (US3-5, FR-019).
    #[test]
    fn issue_an_invalid_mapping_duplicate_label_is_not_saved() {
        use micold_client::features::window::FieldId;
        let (mut rig, store) = file_rig(labelled(), micold_core::issue_types::default_mapping());
        save_with_added_label(&mut rig, "Bug");
        assert_eq!(
            refusal(&rig.app),
            (FieldId::IssueMappingLabel(3), SettingsSection::GithubIssues),
            "`Bug` duplicates `bug` ignoring case"
        );
        assert!(
            store.saves().is_empty(),
            "a duplicate label writes nothing: {:?}",
            store.saves()
        );
    }

    /// A23 — a never-edited mapping shows the default three entries; after edits, Restore defaults
    /// returns them (US3-6).
    #[test]
    fn issue_restore_defaults_returns_the_default_mapping() {
        let store = Arc::new(FakeSettingsStore::new());
        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        rig.app.caps = rig.app.caps.clone().with_settings(store.clone());
        open_github_section(&mut rig.app);
        let defaults: Vec<_> = micold_core::issue_types::default_mapping()
            .into_iter()
            .map(|e| (e.label, e.type_))
            .collect();
        assert_eq!(shown(&rig.app), defaults, "never edited shows the defaults");

        settings(&mut rig.app, SettingsMsg::IssueMappingRemoved(0));
        settings(&mut rig.app, SettingsMsg::IssueMappingAdded);
        assert_ne!(shown(&rig.app), defaults);
        settings(&mut rig.app, SettingsMsg::IssueMappingDefaultsRestored);
        assert_eq!(shown(&rig.app), defaults, "Restore defaults returns them");
    }
}

// ---------------------------------------------------------------------------------------------
// Feature 037, Story 1: the Settings note gives the reason and the action
// ---------------------------------------------------------------------------------------------

mod the_settings_note_explains_a_missing_cli {
    use super::*;
    use micold_client::features::session::{AvailabilityKey, AvailabilitySource, CliAvailability};
    use micold_client::features::settings::missing_cli_notice;
    use micold_core::cli_reason::SpawnEnv;

    type Sent = iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>;

    const WITHOUT_PI: [AiCli; 2] = [AiCli::ClaudeCode, AiCli::Copilot];

    /// Open Settings, which asks about the home directory, and answer that request.
    fn settings_opened_and_answered(
        app: &mut App,
        sent: &mut Sent,
        available: &[AiCli],
        env: Option<SpawnEnv>,
    ) {
        let _ = update_inner(app, Message::Settings(SettingsMsg::Opened));
        let mut asked = Vec::new();
        while let Ok(msg) = sent.try_recv() {
            if let ClientMsg::AiCliAvailabilityRequest { req, cwd: None } = msg {
                asked.push(req);
            }
        }
        let [req] = asked[..] else {
            panic!(
                "fixture check: opening Settings asks about the home directory once, got {asked:?}"
            );
        };
        let _ = update_inner(
            app,
            Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
                req,
                available: available.to_vec(),
                env,
            })),
        );
    }

    /// The note under **Default AI CLI**, read as the view reads it.
    fn note(app: &App) -> Option<String> {
        missing_cli_notice(app.core.session.availability.home())
    }

    fn note_for(available: &[AiCli], env: Option<SpawnEnv>) -> Option<String> {
        let mut app = base_app();
        let mut sent = connected_with_outbox(&mut app);
        settings_opened_and_answered(&mut app, &mut sent, available, env);
        note(&app)
    }

    /// U45 (contract A4, C1): the state on the wire is the state the client holds.
    #[test]
    fn the_shell_files_the_answers_environment_state_with_the_set() {
        let mut app = base_app();
        let mut sent = connected_with_outbox(&mut app);
        settings_opened_and_answered(
            &mut app,
            &mut sent,
            &[AiCli::ClaudeCode],
            Some(SpawnEnv::IncludeOff),
        );

        assert_eq!(
            app.core.session.availability.home(),
            Some(&CliAvailability {
                available: vec![AiCli::ClaudeCode],
                source: AvailabilitySource::ThisComputer,
                env: Some(SpawnEnv::IncludeOff),
                asked_for: AvailabilityKey::Home,
            }),
            "the answer is held as it was sent: the set, the state it was walked in, where the \
             boot plan says sessions run, and the directory it is for"
        );
    }

    /// A1 (Story 1 scenario 1): the reporter's situation.
    #[test]
    fn with_env_include_off_the_note_says_so_and_names_the_switch() {
        assert_eq!(
            note_for(&WITHOUT_PI, Some(SpawnEnv::IncludeOff)).as_deref(),
            Some(
                "A session would not find Pi Coding Agent: sessions get only the login PATH, \
                 because \"Source a script before each session\" is off. Turn it on if your \
                 startup file puts it on the PATH, or install it on the login PATH."
            ),
        );
    }

    /// A3 (Story 1 scenario 3): complete without the environment-include group's outcome line,
    /// which reports the directory most recently resolved and may be another one (FR-007).
    #[test]
    fn a_timed_out_script_is_named_for_the_home_directory_with_the_fields_to_change() {
        let mut app = base_app();
        let mut sent = connected_with_outbox(&mut app);
        settings_opened_and_answered(
            &mut app,
            &mut sent,
            &WITHOUT_PI,
            Some(SpawnEnv::ScriptTimedOut),
        );
        let said = note(&app).expect("Pi is missing, so there is a note");

        for part in [
            "Pi Coding Agent",
            "the startup script timed out for your home directory, so its PATH additions are not \
             applied.",
            "\"Script path\"",
            "\"Timeout\"",
        ] {
            assert!(said.contains(part), "the note lacks {part:?}: {said}");
        }

        for last_outcome in [
            micold_core::env_include::EnvIncludeOutcome::Success,
            micold_core::env_include::EnvIncludeOutcome::MissingScript,
        ] {
            app.env_include_last_outcome = last_outcome;
            assert_eq!(
                note(&app).as_deref(),
                Some(said.as_str()),
                "the note is about the home directory's attempt, whatever the outcome line of \
                 the environment-include group last reported for some other directory"
            );
        }
    }

    /// A4 (Story 1 scenario 4).
    #[test]
    fn a_failed_script_and_a_script_that_is_not_there_are_each_named() {
        let mut app = base_app();
        let mut sent = connected_with_outbox(&mut app);

        settings_opened_and_answered(
            &mut app,
            &mut sent,
            &WITHOUT_PI,
            Some(SpawnEnv::ScriptFailed),
        );
        let failed = note(&app).expect("Pi is missing, so there is a note");
        assert!(
            failed.contains("the startup script exited with an error for your home directory"),
            "{failed}"
        );

        settings_opened_and_answered(
            &mut app,
            &mut sent,
            &WITHOUT_PI,
            Some(SpawnEnv::ScriptNotFound),
        );
        let not_found = note(&app).expect("Pi is still missing");
        assert!(
            not_found.contains("the startup script was not found for your home directory"),
            "the note follows the newer answer: {not_found}"
        );
    }

    /// A5 (Story 1 scenario 4a).
    #[test]
    fn a_blank_script_path_is_the_reason_and_setting_it_is_the_action() {
        let said = note_for(&WITHOUT_PI, Some(SpawnEnv::NoScriptPath))
            .expect("Pi is missing, so there is a note");

        assert!(
            said.contains("no script is sourced, because \"Script path\" is empty."),
            "{said}"
        );
        assert!(said.contains("Set \"Script path\""), "{said}");
        assert!(
            !said.contains("Turn it on"),
            "environment-include is already on; telling the user to turn it on is wrong: {said}"
        );
    }

    /// A6 (Story 1 scenario 5): the one state in which "not found" is the true reason.
    #[test]
    fn with_the_script_applied_the_note_says_the_cli_is_not_on_the_path_sessions_get() {
        assert_eq!(
            note_for(&WITHOUT_PI, Some(SpawnEnv::Applied)).as_deref(),
            Some(
                "Pi Coding Agent was not found on the PATH sessions get for your home \
                 directory: the login PATH plus what the startup script adds. Install it, or \
                 make the script add its directory."
            ),
        );
    }

    /// A7 (Story 1 scenario 6).
    #[test]
    fn with_every_cli_found_there_is_no_note_in_any_state() {
        for env in [
            None,
            Some(SpawnEnv::IncludeOff),
            Some(SpawnEnv::NoScriptPath),
            Some(SpawnEnv::ScriptNotFound),
            Some(SpawnEnv::ScriptFailed),
            Some(SpawnEnv::ScriptTimedOut),
            Some(SpawnEnv::Applied),
        ] {
            assert_eq!(
                note_for(&AiCli::ALL, env),
                None,
                "nothing is missing, so there is nothing to explain ({env:?})"
            );
        }
    }

    /// A8 then A2 (Story 1 scenarios 7 and 2): no note before an answer, the reason when one
    /// arrives, and no note once the CLI is found, with nothing but the answers in between.
    #[test]
    fn the_note_appears_with_the_answer_and_goes_when_the_cli_is_found() {
        let mut app = base_app();
        let mut sent = connected_with_outbox(&mut app);
        assert_eq!(
            note(&app),
            None,
            "the service has not answered: naming a CLI as missing now would be a guess"
        );

        settings_opened_and_answered(
            &mut app,
            &mut sent,
            &[AiCli::ClaudeCode],
            Some(SpawnEnv::IncludeOff),
        );
        let said = note(&app).expect("two CLIs are missing");
        assert!(
            said.starts_with("A session would not find GitHub Copilot and Pi Coding Agent:")
                && said.contains("\"Source a script before each session\" is off"),
            "the note names what is missing with the reason of the answer that arrived: {said}"
        );

        // The user turned environment-include on and saved, which closed Settings. Opening it
        // again is the next answer (FR-013): no restart, and no message but the question.
        settings_opened_and_answered(&mut app, &mut sent, &AiCli::ALL, Some(SpawnEnv::Applied));
        assert_eq!(note(&app), None, "every CLI is found, so the note is gone");
        assert_eq!(
            app.core
                .session
                .availability
                .home()
                .map(|answer| answer.available.contains(&AiCli::Pi)),
            Some(true),
            "and the selector, drawn from the same answer, now offers Pi (SC-002)"
        );
        assert!(
            sent.try_recv().is_err(),
            "the answer itself causes no further message to the service"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Feature 037, Story 2: a start that opens the list instead says why, from the row's answer
// ---------------------------------------------------------------------------------------------

mod a_missing_default_says_why_the_list_opened {
    use super::*;
    use micold_client::features::session::{AvailabilityKey, StartMenu};
    use micold_core::cli_reason::{start_refusal, AttemptDir, Place, SpawnEnv};
    use micold_core::terminal::LaunchMode;

    const WITHOUT_PI: [AiCli; 2] = [AiCli::ClaudeCode, AiCli::Copilot];

    /// A9 (US2-AS1, FR-008, FR-015): the stored default is Pi, environment-include is off, and
    /// the row's own directory has no Pi. Pressing start says so in `cli_reason`'s words for the
    /// row's answer, opens the list, sends no start and keeps the default.
    #[test]
    fn pressing_start_says_include_is_off_opens_the_list_and_starts_nothing() {
        let mut app = app_on_demo(&[]);
        let mut sent =
            connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
        app.core.session.default_ai_cli = AiCli::Pi;
        for (req, _) in availability_requests(&mut sent) {
            let _ = update_inner(
                &mut app,
                Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
                    req,
                    available: WITHOUT_PI.to_vec(),
                    env: Some(SpawnEnv::IncludeOff),
                })),
            );
        }
        assert_eq!(
            app.core
                .session
                .availability
                .for_dir(Path::new(DEMO))
                .map(|answer| answer.asked_for.clone()),
            Some(AvailabilityKey::Dir(PathBuf::from(DEMO))),
            "fixture check: the row is drawn from its own directory's answer"
        );
        assert_eq!(app.core.notifications.queue.visible(), None);

        // The press, as `ui/sidebar.rs` publishes it: the intent's own reason travels with it.
        let StartIntent::OfferChoice {
            providers,
            unavailable_default,
        } = primary_press(&app, &SessionLocation::Default)
        else {
            panic!("fixture check: a default the row lacks opens the list");
        };
        let _ = update_inner(
            &mut app,
            Message::Session(SessionMsg::StartMenuOpened {
                location: SessionLocation::Default,
                unavailable_default,
            }),
        );

        let queue = &app.core.notifications.queue;
        assert_eq!(
            (
                queue.visible().map(|said| said.message.clone()),
                queue.pending()
            ),
            (
                Some(start_refusal(
                    AiCli::Pi,
                    SpawnEnv::IncludeOff,
                    Place::ThisComputer,
                    AttemptDir::Dir(Path::new(DEMO)),
                    LaunchMode::Fresh,
                )),
                0
            ),
            "one message, and it gives the reason of the answer the row offers from (FR-008)"
        );
        let said = queue
            .visible()
            .expect("one message is showing")
            .message
            .clone();
        for (fragment, why) in [
            (
                "A session would not find Pi Coding Agent:",
                "the CLI refused",
            ),
            ("sessions get only the login PATH", "the Off state's reason"),
            (
                "Or start this session on another AI CLI.",
                "a fresh start's ending",
            ),
        ] {
            assert!(said.contains(fragment), "{why} (`{fragment}`): {said}");
        }
        assert!(
            matches!(
                &app.core.session.start_menu,
                Some(StartMenu {
                    location: SessionLocation::Default,
                    ..
                })
            ),
            "the list is open on the row that was pressed"
        );
        assert_eq!(
            providers, WITHOUT_PI,
            "and it offers what a session there finds"
        );

        let mut after = Vec::new();
        while let Ok(msg) = sent.try_recv() {
            after.push(msg);
        }
        assert!(
            matches!(
                &after[..],
                [ClientMsg::AiCliAvailabilityRequest { cwd: Some(dir), .. }]
                    if dir == Path::new(DEMO)
            ),
            "the press asks about the row's directory again and sends no start: {after:?}"
        );
        assert_eq!(
            app.core.session.default_ai_cli,
            AiCli::Pi,
            "the stored default is the user's; a missing CLI does not rewrite it (FR-015)"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Feature 037, Story 3: a row's CLI list names what is not offered there, and why
// ---------------------------------------------------------------------------------------------

mod a_rows_cli_list_names_what_is_not_offered {
    use super::*;
    use micold_client::features::session::StartMenu;
    use micold_core::cli_reason::{explain, start_refusal, AttemptDir, Place, SpawnEnv};
    use micold_core::terminal::LaunchMode;

    const WITHOUT_PI: &[AiCli] = &[AiCli::ClaudeCode, AiCli::Copilot];
    const EVERY: &[AiCli] = &AiCli::ALL;

    /// One directory's answer: what a session there finds, and the state it was walked in.
    type Answer<'a> = (&'a str, &'a [AiCli], Option<SpawnEnv>);

    /// Answer each request in `asked` with its directory's entry of `dirs`. The home directory
    /// has every CLI and an applied script, so a note can only come from a row's own answer.
    fn answer_rows(app: &mut App, asked: &[(u64, Option<PathBuf>)], dirs: &[Answer<'_>]) {
        for (req, cwd) in asked {
            let (available, env) = match cwd {
                None => (EVERY, Some(SpawnEnv::Applied)),
                Some(dir) => dirs
                    .iter()
                    .find(|(d, _, _)| Path::new(d) == dir)
                    .map(|(_, available, env)| (*available, *env))
                    .unwrap_or_else(|| panic!("fixture: no answer given for {}", dir.display())),
            };
            let _ = update_inner(
                app,
                Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
                    req: *req,
                    available: available.to_vec(),
                    env,
                })),
            );
        }
    }

    /// `/repo/demo` with the worktrees `feat-a` and `feat-b`, connected, every row answered.
    fn app_with_rows(
        dirs: &[Answer<'_>],
    ) -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        use micold_core::worktree::WorktreeStatus::Valid;
        let mut app = app_on_demo(&[("feat-a", Valid, true), ("feat-b", Valid, true)]);
        let mut sent =
            connect_with_catalog_keeping_outbox(&mut app, snapshot_with(DEMO, Vec::new()));
        let asked = availability_requests(&mut sent);
        answer_rows(&mut app, &asked, dirs);
        (app, sent)
    }

    fn note(app: &App, dir: &str) -> Option<String> {
        app.core.session.start_menu_note(Path::new(dir))
    }

    fn offered(app: &App, dir: &str) -> Vec<AiCli> {
        app.core.session.offered_providers(Some(Path::new(dir)))
    }

    /// `explain`'s `{reason} {action}` for Pi missing in `dir` on this computer (contract W4, U6).
    fn pi_missing(env: SpawnEnv, dir: &str) -> String {
        let said = explain(
            &[AiCli::Pi],
            env,
            Place::ThisComputer,
            AttemptDir::Dir(Path::new(dir)),
        )
        .expect("one CLI is missing");
        format!("{} {}", said.reason, said.action)
    }

    fn list_is_open_on(app: &App, location: &SessionLocation) -> bool {
        matches!(&app.core.session.start_menu, Some(StartMenu { location: open, .. }) if open == location)
    }

    /// A15 (US3-AS1, FR-010, SC-007): the row's directory provides two CLIs and no Pi, and the
    /// startup script failed there. With the list open, its note names Pi, the reason, the action
    /// and the row's own directory, and the two CLIs are still offered.
    #[test]
    fn a_list_of_two_names_the_missing_cli_with_the_reason_and_action_for_the_rows_directory() {
        let (mut app, _sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, EVERY, Some(SpawnEnv::Applied)),
        ]);
        let row = SessionLocation::Worktree("feat-a".into());
        open_start_list(&mut app, row.clone());
        assert!(
            list_is_open_on(&app, &row),
            "fixture check: the list opened"
        );

        let said = note(&app, FEAT_A).expect("a list of two that lacks a third says so");
        assert_eq!(said, pi_missing(SpawnEnv::ScriptFailed, FEAT_A));
        for part in [
            "Pi Coding Agent",
            "the startup script exited with an error",
            "Fix the script named in \"Script path\".",
            FEAT_A,
        ] {
            assert!(said.contains(part), "the note lacks {part:?}: {said}");
        }
        assert_eq!(offered(&app, FEAT_A), WITHOUT_PI);
    }

    /// A16 (US3-AS1a, FR-010, SC-007): a row with one CLI has no chevron and no note.
    #[test]
    fn a_row_with_one_cli_has_no_chevron_and_no_note() {
        let (app, _sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, CLAUDE, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, EVERY, Some(SpawnEnv::Applied)),
        ]);

        assert!(
            !offers_a_choice(&app, FEAT_A),
            "one CLI: no chevron (026 FR-006)"
        );
        assert_eq!(note(&app, FEAT_A), None);
    }

    /// A17 (US3-AS1b, FR-010, FR-008, D6): one CLI, and it is not the stored default. The primary
    /// press opens the list with Story 2's message; the list itself has no note.
    #[test]
    fn a_missing_default_on_a_row_with_one_cli_is_explained_by_the_message_and_not_by_a_note() {
        let (mut app, _sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, CLAUDE, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, EVERY, Some(SpawnEnv::Applied)),
        ]);
        app.core.session.default_ai_cli = AiCli::Pi;
        let row = SessionLocation::Worktree("feat-a".into());

        let StartIntent::OfferChoice {
            unavailable_default,
            ..
        } = primary_press(&app, &row)
        else {
            panic!("fixture check: a default the row lacks opens the list");
        };
        let _ = update_inner(
            &mut app,
            Message::Session(SessionMsg::StartMenuOpened {
                location: row.clone(),
                unavailable_default,
            }),
        );

        assert!(list_is_open_on(&app, &row));
        assert_eq!(
            app.core
                .notifications
                .queue
                .visible()
                .map(|said| said.message.clone()),
            Some(start_refusal(
                AiCli::Pi,
                SpawnEnv::ScriptFailed,
                Place::ThisComputer,
                AttemptDir::Dir(Path::new(FEAT_A)),
                LaunchMode::Fresh,
            )),
            "the message carries the reason there (FR-008)"
        );
        assert_eq!(note(&app, FEAT_A), None, "and the list adds nothing (D6)");
    }

    /// A18 (US3-AS2, FR-011, SC-005): every CLI is offered, so nothing is said.
    #[test]
    fn a_row_with_every_cli_has_no_note() {
        let (mut app, _sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_B, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
        ]);
        open_start_list(&mut app, SessionLocation::Worktree("feat-a".into()));

        assert_eq!(offered(&app, FEAT_A), EVERY);
        assert_eq!(note(&app, FEAT_A), None);
    }

    /// A19 (US3-AS3, FR-012): two rows whose answers differ each say their own directory's, and
    /// opening, answering and closing one row's list leaves the other's note and offer alone.
    #[test]
    fn each_row_keeps_its_own_note_while_another_rows_list_is_opened_answered_and_closed() {
        const WITHOUT_COPILOT: &[AiCli] = &[AiCli::ClaudeCode, AiCli::Pi];
        let (mut app, mut sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, WITHOUT_COPILOT, Some(SpawnEnv::IncludeOff)),
        ]);
        let a_says = pi_missing(SpawnEnv::ScriptFailed, FEAT_A);
        let b_says = note(&app, FEAT_B).expect("feat-b lacks Copilot");
        assert_eq!(note(&app, FEAT_A), Some(a_says.clone()));
        assert!(
            b_says.contains("GitHub Copilot") && !b_says.contains("Pi Coding Agent"),
            "feat-b's note is about what feat-b lacks: {b_says}"
        );
        assert_ne!(a_says, b_says);

        // feat-b's list is opened, the press's own question is answered with a newer state, and
        // the list is closed again.
        open_start_list(&mut app, SessionLocation::Worktree("feat-b".into()));
        let asked = availability_requests(&mut sent);
        assert!(
            matches!(&asked[..], [(_, Some(dir))] if dir == Path::new(FEAT_B)),
            "fixture check: opening feat-b's list asks about feat-b only: {asked:?}"
        );
        assert_eq!(note(&app, FEAT_A), Some(a_says.clone()), "while it is open");
        answer_rows(
            &mut app,
            &asked,
            &[(FEAT_B, WITHOUT_PI, Some(SpawnEnv::ScriptTimedOut))],
        );
        assert_eq!(
            note(&app, FEAT_B),
            Some(pi_missing(SpawnEnv::ScriptTimedOut, FEAT_B)),
            "feat-b's note follows feat-b's newer answer (contract W6)"
        );
        let _ = update_inner(&mut app, Message::Session(SessionMsg::StartMenuDismissed));

        assert_eq!(note(&app, FEAT_A), Some(a_says));
        assert_eq!(offered(&app, FEAT_A), WITHOUT_PI);
    }

    /// A20 (US3-AS4, FR-010, FR-015): the list names Pi and has no item for it, and opening the
    /// list started nothing.
    #[test]
    fn the_cli_the_note_names_is_not_among_the_ones_offered_and_nothing_starts() {
        let (mut app, mut sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, EVERY, Some(SpawnEnv::Applied)),
        ]);
        open_start_list(&mut app, SessionLocation::Worktree("feat-a".into()));

        let said = note(&app, FEAT_A).expect("the list says what it does not offer");
        assert!(said.contains(&AiCli::Pi.to_string()), "{said}");
        assert!(
            !offered(&app, FEAT_A).contains(&AiCli::Pi),
            "a CLI that is named as not offered has no item to press"
        );

        let mut after = Vec::new();
        while let Ok(msg) = sent.try_recv() {
            after.push(msg);
        }
        assert!(
            after
                .iter()
                .all(|msg| matches!(msg, ClientMsg::AiCliAvailabilityRequest { .. })),
            "opening the list asks about the directory and starts nothing: {after:?}"
        );
    }
}

// --- `027` BUG-574 (T229, T230, FR-036c): out of date is measured against the running container -

mod bug_574_out_of_date_is_measured {
    use super::*;
    use micold_client::features::sandbox::{SandboxLocations, KEEP_RUNNING_SETTING};
    use micold_core::protocol::messages::CatalogSnapshot;
    use micold_core::sandbox::lifecycle::SandboxState;

    const P: &str = "/proj/P";
    const Q: &str = "/proj/Q";

    /// A catalog listing `paths`, in that order.
    fn catalog_of(paths: &[&str]) -> CatalogSnapshot {
        let mut catalog = CatalogSnapshot::default();
        for path in paths {
            catalog
                .projects
                .append(&mut snapshot_with(path, Vec::new()).projects);
        }
        catalog
    }

    /// The failed sandbox brought up again by a refused dial, ending in a container that shares
    /// `projects`. `mounted` is `None` for a container this bring-up created, `Some` for one it
    /// adopted, as `lifecycle::bring_up` reports them.
    fn started_sharing(app: &mut App, mounted: Option<&[&str]>, projects: &[&str]) {
        let _ = connection_failed(app);
        let mut started = a_started_sandbox();
        started.mounted = mounted.map(|m| m.iter().map(|p| p.to_string()).collect());
        let locations = SandboxLocations {
            projects: projects.iter().map(|p| p.to_string()).collect(),
            ..SandboxLocations::default()
        };
        let _ = update_inner(
            app,
            Message::Sandbox(SandboxMsg::Started(Box::new((started, locations)))),
        );
    }

    fn running(app: &App) -> bool {
        matches!(app.sandbox.state, SandboxState::Running(_))
    }

    fn stale(app: &App) -> bool {
        matches!(app.sandbox.state, SandboxState::Stale(_))
    }

    /// R1: created from `[P, Q]`, the service lists the same two in the other order.
    #[test]
    fn r1_the_same_projects_in_another_order_are_not_out_of_date() {
        let mut app = app_with_a_failed_sandbox();
        app.sandbox_boot.as_mut().expect("a boot plan").projects =
            vec![PathBuf::from(P), PathBuf::from(Q)];
        started_sharing(&mut app, None, &[P, Q]);

        the_service_answers_with(&mut app, catalog_of(&[Q, P]));

        assert!(
            running(&app),
            "the container shares every registered project; order is not a difference (FR-036c): \
             {:?}",
            app.sandbox.state
        );
    }

    /// R2: this client registered nothing yet; it adopted a container another window made, which
    /// shares P and Q, and the service lists P and Q.
    #[test]
    fn r2_an_adopted_container_sharing_every_registered_project_is_not_out_of_date() {
        let mut app = app_with_a_failed_sandbox();
        started_sharing(&mut app, Some(&[P, Q]), &[P, Q]);

        the_service_answers_with(&mut app, catalog_of(&[P, Q]));

        assert!(
            running(&app),
            "measured against the container that is running, not this client's plan (FR-036c): \
             {:?}",
            app.sandbox.state
        );
    }

    /// R3: out of date while a shared project is unregistered, and no longer once it is back.
    #[test]
    fn r3_out_of_date_clears_without_a_restart_once_the_projects_match_again() {
        let mut app = app_with_a_failed_sandbox();
        started_sharing(&mut app, Some(&[P, Q]), &[P, Q]);

        the_service_answers_with(&mut app, catalog_of(&[P]));
        assert!(
            stale(&app),
            "the container shares Q, which is no longer registered (FR-036c): {:?}",
            app.sandbox.state
        );

        feed(
            &mut app,
            DaemonMsg::CatalogChanged {
                catalog: catalog_of(&[P, Q]),
            },
        );
        assert!(
            running(&app),
            "Q is registered again, so nothing is out of date, and nothing restarted (FR-036c): \
             {:?}",
            app.sandbox.state
        );
        assert!(
            app.sandbox.persistent_notice().is_none(),
            "the notice clears with the state: {:?}",
            app.sandbox.persistent_notice()
        );
    }

    /// A project registered that the container does not share is out of date.
    #[test]
    fn a_registered_project_the_container_does_not_share_is_out_of_date() {
        let mut app = app_with_a_failed_sandbox();
        started_sharing(&mut app, Some(&[P, Q]), &[P, Q]);

        the_service_answers_with(&mut app, catalog_of(&[P, Q, "/proj/R"]));

        assert!(stale(&app), "R is not shared: {:?}", app.sandbox.state);
        assert_eq!(
            app.sandbox_boot.as_ref().map(|plan| plan.projects.len()),
            Some(3),
            "the plan follows the catalog, for the next bring-up"
        );
    }

    /// T230: a keep-running change is its own reason. A catalog that matches the container does not
    /// clear it, and the notice names the setting rather than the projects.
    #[test]
    fn a_keep_running_change_is_not_cleared_by_a_matching_catalog() {
        let mut app = app_with_a_failed_sandbox();
        started_sharing(&mut app, None, &[]);
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
        let _ = update_inner(
            &mut app,
            Message::Settings(SettingsMsg::SurviveLogoutToggled(true)),
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
        assert!(
            stale(&app),
            "setup: the save marked the sandbox out of date"
        );

        the_service_answers_with(&mut app, CatalogSnapshot::default());

        assert!(
            stale(&app),
            "only a new container clears a keep-running change (FR-036c): {:?}",
            app.sandbox.state
        );
        let notice = app
            .sandbox
            .persistent_notice()
            .expect("an out-of-date sandbox says so");
        assert!(
            notice.contains(KEEP_RUNNING_SETTING) && !notice.contains("project"),
            "the notice names the setting, not an unshared project (FR-036c): {notice}"
        );
    }

    /// T231 through the reducer: a container another window put in place is adopted and measured
    /// against the last catalog; under the host placement, the report is dropped.
    #[test]
    fn a_replaced_container_is_measured_against_the_last_catalog() {
        use micold_core::sandbox::runtime::ContainerId;
        let theirs = || {
            SandboxMsg::Replaced(Box::new((
                ContainerId("new".into()),
                SandboxLocations {
                    projects: vec![P.to_string(), Q.to_string()],
                    ..SandboxLocations::default()
                },
            )))
        };
        let mut app = app_with_a_failed_sandbox();
        started_sharing(&mut app, Some(&[P]), &[P]);
        the_service_answers_with(&mut app, catalog_of(&[P, Q]));
        assert!(stale(&app), "setup: the container held shares only P");

        let _ = update_inner(&mut app, Message::Sandbox(theirs()));

        assert_eq!(
            app.sandbox.state,
            SandboxState::Running(ContainerId("new".into())),
            "the replacement shares every registered project (FR-036c)"
        );

        let mut app = app_with_a_failed_sandbox();
        started_sharing(&mut app, Some(&[P]), &[P]);
        app.placement.kind = PlacementKind::HostProcess;
        let before = app.sandbox.clone();
        let _ = update_inner(&mut app, Message::Sandbox(theirs()));
        assert_eq!(
            app.sandbox, before,
            "a report outliving a move to the host is dropped"
        );
    }
}

// ---- Feature 040, T025: the pull request reading, through the shell --------------------------
//
// The reducer's rules are `tests/features_pr_status.rs`. These drive the shell's half: which daemon
// messages start a reading, what the reading asks and of whom, and that nothing else of the
// application changes. The 10-second timer a reading returns is never run: each test drops that
// task and sends the timer's message itself where it wants the bound to run out.
mod pr_status {
    use super::*;
    use crate::shell::capabilities::IssueTooling;
    use micold_client::features::pr_status::{Msg, Phase};
    use micold_core::git::GitRemote;
    use micold_core::github::IssueSource;
    use micold_core::protocol::messages::{OperationResult, RefusalReason};
    use micold_core::pull_request::{
        CheckStatus, FakePullRequestSource, PrState, PullRequestSource, PullRequestStatus,
        ReadingFailure, ReviewState,
    };
    use std::collections::{BTreeMap, VecDeque};

    const FAKE_GH: &str = "/fake/bin/gh";

    struct Rig {
        app: App,
        rx: iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
        source: Arc<FakePullRequestSource>,
    }

    /// Connected, `DEMO` active, the switch on, and tooling that finds `gh` at `gh` (or nowhere).
    fn rig(gh: Option<&str>, source: FakePullRequestSource) -> Rig {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let source = Arc::new(source);
        let found = gh.map(PathBuf::from);
        let tooling = IssueTooling {
            locate_gh: Arc::new(move |_: Option<&str>| found.clone()),
            source: Arc::new(|_: PathBuf| -> Arc<dyn IssueSource + Send + Sync> {
                unreachable!("a pull request reading never builds the issue source")
            }),
            pull_requests: {
                let source = Arc::clone(&source);
                Arc::new(move |_: PathBuf| {
                    Arc::clone(&source) as Arc<dyn PullRequestSource + Send + Sync>
                })
            },
        };
        let mut app = base_app();
        app.caps = app.caps.clone().with_issue_tooling(tooling);
        app.daemon = Some(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(PathBuf::from(DEMO));
        let _ = shell::pr_status::enabled_changed(&mut app, true);
        Rig { app, rx, source }
    }

    fn status(number: u64) -> PullRequestStatus {
        PullRequestStatus {
            number,
            title: format!("Pull request {number}"),
            url: format!("https://github.com/o/r/pull/{number}"),
            state: PrState::Open {
                checks: CheckStatus::Passing,
            },
            review: ReviewState::None,
            head: "a".repeat(40),
        }
    }

    fn statuses() -> BTreeMap<String, PullRequestStatus> {
        BTreeMap::from([("feat/a".to_string(), status(7))])
    }

    fn github_remote() -> Vec<GitRemote> {
        vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }]
    }

    /// `DEMO` with one worktree per entry: its directory and the branch it is on, if any.
    fn listing(
        worktrees: &[(&str, Option<&str>)],
    ) -> micold_core::protocol::messages::CatalogSnapshot {
        let mut catalog = snapshot_with(DEMO, Vec::new());
        for (dir, branch) in worktrees {
            catalog.projects[0].worktrees.push(WorktreeSnapshot {
                dir_name: dir.to_string(),
                branch: branch.map(str::to_string),
                display_name: dir.to_string(),
                status: micold_core::protocol::messages::WorktreeStatus::Clean,
                path: PathBuf::from(DEMO).join(".claude/worktrees").join(dir),
                included: false,
                user_created: true,
            });
        }
        catalog
    }

    fn one_worktree() -> micold_core::protocol::messages::CatalogSnapshot {
        listing(&[("a", Some("feat/a"))])
    }

    /// Run `work` and every message it produces back through the shell until nothing is left.
    fn settle(app: &mut App, work: Task<Message>) {
        let mut queue: VecDeque<Message> = messages(work).into();
        while let Some(message) = queue.pop_front() {
            queue.extend(messages(update_inner(app, message)));
        }
    }

    /// A daemon message whose follow-up is dropped: for a reading that is the 10-second timer.
    fn daemon(app: &mut App, event: DaemonMsg) {
        let _ = shell::daemon_sync::on_daemon_event(app, event);
    }

    fn attached(app: &mut App) {
        daemon(
            app,
            DaemonMsg::Attached {
                project: PathBuf::from(DEMO),
                sessions: vec![],
            },
        );
    }

    fn listed(app: &mut App, catalog: micold_core::protocol::messages::CatalogSnapshot) {
        daemon(app, DaemonMsg::CatalogChanged { catalog });
    }

    /// Everything sent to the daemon so far, drained from the outbox.
    fn sent(rig: &mut Rig) -> Vec<ClientMsg> {
        let mut sent = Vec::new();
        while let Ok(msg) = rig.rx.try_recv() {
            sent.push(msg);
        }
        sent
    }

    /// The `req` of every `RemoteList` sent so far.
    fn remote_lists(rig: &mut Rig) -> Vec<u64> {
        sent(rig)
            .into_iter()
            .filter_map(|msg| match msg {
                ClientMsg::RemoteList { req, .. } => Some(req),
                _ => None,
            })
            .collect()
    }

    fn answer_remotes(app: &mut App, req: u64, remotes: Vec<GitRemote>) {
        let work = shell::daemon_sync::on_daemon_event(
            app,
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RemoteList { remotes },
            },
        );
        settle(app, work);
    }

    /// `Attached`, the listing, and the remotes answered with a GitHub remote: one whole reading.
    fn read_once(rig: &mut Rig, catalog: micold_core::protocol::messages::CatalogSnapshot) {
        attached(&mut rig.app);
        listed(&mut rig.app, catalog);
        let asked = remote_lists(rig);
        assert_eq!(
            asked.len(),
            1,
            "the listing after `Attached` asks for the remotes once"
        );
        answer_remotes(&mut rig.app, asked[0], github_remote());
    }

    /// A rig whose first reading landed `statuses()`.
    fn holding() -> Rig {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_answer(statuses()),
        );
        read_once(&mut rig, one_worktree());
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        rig
    }

    fn seq_under_way(app: &App) -> u64 {
        match app.core.pr_status.phase {
            Phase::Reading { seq, .. } => seq,
            Phase::Idle => panic!("a reading is under way"),
        }
    }

    /// Start a second reading on a rig that holds statuses, by turning the switch off and on, and
    /// put the statuses back so that what the reading's end does to them can be seen.
    fn read_again(rig: &mut Rig) -> u64 {
        let _ = shell::pr_status::enabled_changed(&mut rig.app, false);
        let _ = shell::pr_status::enabled_changed(&mut rig.app, true);
        rig.app.core.pr_status.statuses = statuses();
        let asked = remote_lists(rig);
        assert_eq!(asked.len(), 1, "switching on while held reads");
        asked[0]
    }

    // A10, U89, U91: what the reading asks, and that its answer lands.
    #[test]
    fn pr_status_reads_the_listed_branches_after_attached_and_the_listing() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_answer(statuses()),
        );
        attached(&mut rig.app);
        assert!(sent(&mut rig).is_empty(), "`Attached` alone asks nothing");

        listed(
            &mut rig.app,
            listing(&[
                ("b", Some("feat/b")),
                ("detached", None),
                ("a", Some("feat/a")),
                ("b-again", Some("feat/b")),
            ]),
        );
        let all = sent(&mut rig);
        let asked: Vec<u64> = all
            .iter()
            .filter_map(|msg| match msg {
                ClientMsg::RemoteList { req, .. } => Some(*req),
                _ => None,
            })
            .collect();
        assert_eq!(asked.len(), 1);
        assert!(rig.source.calls().is_empty(), "the remotes come first");

        answer_remotes(&mut rig.app, asked[0], github_remote());
        assert_eq!(
            rig.source.calls(),
            vec![(
                "o/r".to_string(),
                vec!["feat/b".to_string(), "feat/a".to_string()]
            )],
            "listing order, deduplicated, the detached worktree left out"
        );
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(rig.app.core.pr_status.read_at.is_some());
        // FR-018a: a reading never asks for a listing.
        let all: Vec<ClientMsg> = all.into_iter().chain(sent(&mut rig)).collect();
        assert!(
            !all.iter()
                .any(|msg| matches!(msg, ClientMsg::WorktreeRefresh { .. })),
            "{all:?}"
        );
    }

    // U90: only the first listing after `Attached` reads.
    #[test]
    fn pr_status_a_later_listing_starts_nothing() {
        let mut rig = holding();
        listed(
            &mut rig.app,
            listing(&[("a", Some("feat/a")), ("c", Some("feat/c"))]),
        );
        assert!(remote_lists(&mut rig).is_empty());
        assert_eq!(rig.source.calls().len(), 1);
    }

    // A11, A14 (FR-026): no GitHub remote, nothing is sent to GitHub and what was shown is removed.
    #[test]
    fn pr_status_without_a_github_remote_never_calls_the_source_and_clears() {
        let mut rig = holding();
        let req = read_again(&mut rig);
        answer_remotes(
            &mut rig.app,
            req,
            vec![GitRemote {
                name: "origin".into(),
                url: "git@example.com:o/r.git".into(),
            }],
        );
        assert_eq!(rig.source.calls().len(), 1, "only the first reading's call");
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(nothing_was_reported(&rig.app));
    }

    // A10 (FR-025): `gh` not found.
    #[test]
    fn pr_status_without_gh_never_calls_the_source_and_clears() {
        let mut rig = rig(None, FakePullRequestSource::new());
        rig.app.core.pr_status.statuses = statuses();
        read_once(&mut rig, one_worktree());
        assert!(rig.source.calls().is_empty());
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(nothing_was_reported(&rig.app));
    }

    // U96 (FR-021): the remotes unanswered for 10 seconds end the reading as a passing failure.
    #[test]
    fn pr_status_remotes_unanswered_for_ten_seconds_keep_the_statuses() {
        let mut rig = holding();
        let req = read_again(&mut rig);
        let seq = seq_under_way(&rig.app);

        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(nothing_was_reported(&rig.app));

        // The answer arriving after all is nobody's.
        answer_remotes(&mut rig.app, req, github_remote());
        assert_eq!(rig.source.calls().len(), 1);
    }

    // The timer of a reading that was answered in time changes nothing.
    #[test]
    fn pr_status_the_timer_of_an_answered_reading_changes_nothing() {
        let mut rig = holding();
        let before = rig.app.core.pr_status.clone();
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq: 0, req: 0 }),
        );
        settle(&mut rig.app, work);
        assert_eq!(rig.app.core.pr_status, before);
    }

    // The daemon failing the remotes request is a passing failure, and never a notice.
    #[test]
    fn pr_status_a_failed_remotes_request_keeps_the_statuses_without_a_notice() {
        let mut rig = holding();
        let req = read_again(&mut rig);
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            DaemonMsg::OperationError {
                req,
                kind: micold_core::protocol::messages::ErrorKind::Internal,
                message: "git failed".into(),
                detail: None,
            },
        );
        settle(&mut rig.app, work);
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(nothing_was_reported(&rig.app));
    }

    /// After the hold ended: nothing shown, nothing under way, and listings start nothing.
    fn assert_released(rig: &mut Rig) {
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        let _ = sent(rig);
        listed(&mut rig.app, one_worktree());
        assert!(
            remote_lists(rig).is_empty(),
            "a window that holds nothing reads nothing"
        );
        assert_eq!(rig.source.calls().len(), 1, "only the first reading's call");
    }

    // A40, U92 (SC-007).
    #[test]
    fn pr_status_displaced_clears_and_reads_nothing() {
        let mut rig = holding();
        daemon(
            &mut rig.app,
            DaemonMsg::Displaced {
                project: PathBuf::from(DEMO),
                by: other_window(),
            },
        );
        assert_released(&mut rig);
    }

    // A41, U93 (SC-007).
    #[test]
    fn pr_status_refused_as_busy_clears_and_reads_nothing() {
        let mut rig = holding();
        daemon(
            &mut rig.app,
            DaemonMsg::Refused {
                reason: RefusalReason::ProjectBusy {
                    project: PathBuf::from(DEMO),
                    holder: other_window(),
                    since_secs: 12,
                },
            },
        );
        assert_released(&mut rig);
    }

    // Losing another project changes nothing here.
    #[test]
    fn pr_status_displaced_from_another_project_keeps_the_statuses() {
        let mut rig = holding();
        daemon(
            &mut rig.app,
            DaemonMsg::Displaced {
                project: PathBuf::from("/repo/other"),
                by: other_window(),
            },
        );
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        assert!(rig.app.core.pr_status.held);
    }

    // U94: a project switch.
    #[test]
    fn pr_status_a_project_switch_clears_and_reads_nothing() {
        let mut rig = holding();
        shell::daemon_sync::switch_daemon_attachment(
            &mut rig.app,
            Some(PathBuf::from(DEMO)),
            Path::new("/repo/other"),
        );
        assert_released(&mut rig);
    }

    // U95: a disconnect, with a reading under way: its remotes request is dropped without a notice.
    #[test]
    fn pr_status_a_disconnect_clears_and_reads_nothing() {
        let mut rig = holding();
        let _ = read_again(&mut rig);
        let _ = shell::daemon_sync::on_disconnected(&mut rig.app);
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(!rig.app.core.pr_status.held);
        assert!(nothing_was_reported(&rig.app));
    }

    // A42 (SC-007): a take-over reads once, as on opening.
    #[test]
    fn pr_status_a_take_over_reads_once() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_answer(statuses())
                .with_answer(statuses()),
        );
        read_once(&mut rig, one_worktree());
        daemon(
            &mut rig.app,
            DaemonMsg::Displaced {
                project: PathBuf::from(DEMO),
                by: other_window(),
            },
        );
        assert!(rig.app.core.pr_status.statuses.is_empty());

        read_once(&mut rig, one_worktree());
        assert_eq!(rig.source.calls().len(), 2);
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
    }

    fn settings(pr_status_enabled: bool) -> DaemonMsg {
        DaemonMsg::SettingsChanged {
            settings: micold_core::protocol::messages::DaemonSettings {
                pr_status_enabled,
                ..quiet_settings()
            },
        }
    }

    // A31, U86: the switch turning on while held reads once; turning off clears.
    #[test]
    fn pr_status_the_switch_reads_once_when_turned_on_and_clears_when_turned_off() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_answer(statuses()),
        );
        let _ = shell::pr_status::enabled_changed(&mut rig.app, false);
        attached(&mut rig.app);
        listed(&mut rig.app, one_worktree());
        assert!(remote_lists(&mut rig).is_empty(), "off: nothing is read");

        daemon(&mut rig.app, settings(true));
        let asked = remote_lists(&mut rig);
        assert_eq!(asked.len(), 1);
        daemon(&mut rig.app, settings(true));
        assert!(
            remote_lists(&mut rig).is_empty(),
            "the same value again reads nothing"
        );
        answer_remotes(&mut rig.app, asked[0], github_remote());
        assert_eq!(rig.app.core.pr_status.statuses, statuses());

        daemon(&mut rig.app, settings(false));
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert!(remote_lists(&mut rig).is_empty());
        assert_eq!(rig.source.calls().len(), 1);
    }

    /// What FR-020 says a reading leaves alone, as text: the worktree list with its selection and
    /// expansion, the sessions, the workspace and the sidebar's own state.
    fn untouched(app: &App) -> String {
        format!(
            "{:?}\n{:?}\n{:?}\n{:?}\n{}",
            app.core.worktree,
            app.core.session,
            app.core.workspace,
            app.core.sidebar,
            app.display_offset,
        )
    }

    // A12, U97 (FR-020, SC-004): every way a reading ends leaves the rest of the application as it
    // was and raises nothing.
    #[test]
    fn pr_status_a_finished_reading_changes_nothing_else_and_reports_nothing() {
        for failure in [
            None,
            Some(ReadingFailure::Unavailable),
            Some(ReadingFailure::Passing),
            Some(ReadingFailure::RateLimited { until: u64::MAX }),
        ] {
            let source = FakePullRequestSource::new().with_answer(statuses());
            let source = match failure {
                Some(failure) => source.with_failure(failure),
                None => source.with_answer(BTreeMap::new()),
            };
            let mut rig = rig(Some(FAKE_GH), source);
            read_once(&mut rig, one_worktree());
            let req = read_again(&mut rig);
            let before = untouched(&rig.app);

            answer_remotes(&mut rig.app, req, github_remote());

            assert_eq!(rig.source.calls().len(), 2, "{failure:?}");
            assert_eq!(rig.app.core.pr_status.phase, Phase::Idle, "{failure:?}");
            assert_eq!(untouched(&rig.app), before, "{failure:?}");
            assert!(nothing_was_reported(&rig.app), "{failure:?}");
            assert!(rig.app.core.worktree_form.worktree_error.is_none());
        }
    }
}

/// Feature 039 (#572 item 5, close finding F9): the glue between the window's halves of a
/// notification — the show leaves the thread the update's work runs on, its result reaches the
/// `AttentionShown` arm, and a click's raise comes before the switch and the selection. Each test
/// was seen red under one mutant of `shell/daemon_sync.rs`.
mod attention_glue {
    use super::*;
    use micold_client::features::attention::{DesktopNotification, DesktopNotifier, NotifyError};
    use micold_core::project::{Availability, Project};
    use micold_core::session::{Session, SessionId, SessionLocation};
    use std::sync::Mutex;
    use std::thread::ThreadId;

    /// A notifier that records the thread each show ran on and answers `answer`.
    struct Recording {
        threads: Mutex<Vec<ThreadId>>,
        answer: Result<(), NotifyError>,
    }

    impl Recording {
        fn answering(answer: Result<(), NotifyError>) -> Arc<Self> {
            Arc::new(Self {
                threads: Mutex::new(Vec::new()),
                answer,
            })
        }

        fn threads(&self) -> Vec<ThreadId> {
            self.threads.lock().unwrap().clone()
        }
    }

    impl DesktopNotifier for Recording {
        fn show(&self, _notification: DesktopNotification) -> Result<(), NotifyError> {
            self.threads
                .lock()
                .unwrap()
                .push(std::thread::current().id());
            self.answer.clone()
        }
    }

    /// An app whose active project, a folder that exists, holds one session; and that session.
    fn app_with_a_session(
        folder: &std::path::Path,
        notifier: Arc<dyn DesktopNotifier>,
    ) -> (App, SessionId) {
        let mut app = base_app();
        app.caps.set_notifier(notifier);
        let project = folder.to_path_buf();
        app.core.workspace.projects.push(Project::new(
            project.clone(),
            true,
            Availability::Available,
        ));
        app.core.workspace.active = Some(project.clone());
        let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        let id = session.id;
        app.core.workspace.sessions.insert(project, vec![session]);
        (app, id)
    }

    /// (a) The show waits on the system's bus, so it runs on a blocking task: not in the update,
    /// and not on the thread that drives the update's work.
    #[test]
    fn a_granted_claim_shows_the_notification_off_the_thread_that_runs_the_work() {
        let folder = tempfile::tempdir().unwrap();
        let notifier = Recording::answering(Ok(()));
        let (mut app, session) = app_with_a_session(folder.path(), notifier.clone());

        let work = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::AttentionGranted { session, seq: 1 },
        );
        assert!(
            notifier.threads().is_empty(),
            "the update itself shows nothing"
        );
        let out = messages(work);

        let threads = notifier.threads();
        assert_eq!(threads.len(), 1, "the work shows the notification once");
        assert_ne!(
            threads[0],
            std::thread::current().id(),
            "the show ran on the thread that drives the work, not a blocking one"
        );
        assert!(
            matches!(
                out.as_slice(),
                [Message::Connection(ConnectionMsg::AttentionShown(Ok(())))]
            ),
            "the result comes back as AttentionShown"
        );
    }

    /// (b) A refused show comes back through the `AttentionShown` arm, which logs it once.
    #[test]
    fn a_refused_show_reaches_the_attention_shown_arm() {
        let folder = tempfile::tempdir().unwrap();
        let notifier = Recording::answering(Err(NotifyError::Refused("busy".into())));
        let (mut app, session) = app_with_a_session(folder.path(), notifier);

        let work = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::AttentionGranted { session, seq: 1 },
        );
        for message in messages(work) {
            let _ = update(&mut app, message);
        }

        assert!(
            app.core.attention.failure_logged,
            "the refusal was not handed to the attention state"
        );
    }

    /// The order the actions of `work` arrive in: `raise` for the window raise's first request
    /// (answered "no window", which ends the raise), `message` for each message.
    fn action_order(work: Task<Message>) -> Vec<&'static str> {
        use iced::futures::StreamExt;
        let Some(stream) = iced_runtime::task::into_stream(work) else {
            return Vec::new();
        };
        let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
        runtime.block_on(async {
            let order = stream
                .map(|action| match action {
                    iced_runtime::Action::Window(iced_runtime::window::Action::GetLatest(
                        reply,
                    )) => {
                        let _ = reply.send(None);
                        "raise"
                    }
                    iced_runtime::Action::Output(_) => "message",
                    _ => "other",
                })
                .collect::<Vec<_>>();
            tokio::time::timeout(std::time::Duration::from_secs(10), order)
                .await
                .expect("the work an update returned has to finish")
        })
    }

    /// (c) A click's reveal raises the window first, then dispatches the selection (N5, N6).
    #[test]
    fn a_reveal_raises_the_window_before_the_selection() {
        let folder = tempfile::tempdir().unwrap();
        let (mut app, session) = app_with_a_session(folder.path(), Recording::answering(Ok(())));

        let work = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            DaemonMsg::RevealSession {
                project: folder.path().to_path_buf(),
                session,
                activation: None,
            },
        );

        assert_eq!(action_order(work), ["raise", "message"]);
    }
}
