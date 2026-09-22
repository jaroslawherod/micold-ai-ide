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
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener)),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
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
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener)),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
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

/// BUG-002: connecting is one of the three moments the client asks which AI CLIs the service
/// can run (027 FR-023c, T144) — and the only one the user does not trigger by hand.
///
/// It asked into a handle it had not stored yet. `ask_cli_availability` returns early when
/// `app.daemon` is `None`, which it is on a first connect and — since `on_disconnected` clears
/// it — on every reconnect after, so the request was never sent. `available_providers` then
/// stayed `None` for the whole run, and `None` is read as the empty set by everything that
/// decides what to *offer*: no override chevron on the sidebar row (026 FR-004, FR-006), and a
/// default that is not installed started instead of offering the CLIs that are (026 FR-002).
/// Opening Settings was the only thing that could repair it, because the other two ask sites
/// are that view and the menu the missing chevron opens.
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

    let _ = update_inner(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::AiCliAvailability {
            req: 0,
            available: vec![AiCli::ClaudeCode, AiCli::Copilot],
        })),
    );
    assert!(
        app.core.session.start_affordance_offers_a_choice(),
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
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener)),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
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
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener)),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        stamper: SessionInputStamper::new(),
        selection: None,
        display_offset: 0,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        dismissing: None,
        window_focused: true,
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
    }
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

    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );

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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );
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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );
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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );

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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );

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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );

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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );
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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );
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
    let _ = update_inner(
        &mut app,
        Message::Sandbox(SandboxMsg::Started(Box::new(a_started_sandbox()))),
    );
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
        SandboxMsg::Started(Box::new(a_started_sandbox())),
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
