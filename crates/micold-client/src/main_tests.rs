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
        git_credentials: crate::shell::scoped_credential::GitCredentials::idle(),
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        pane_layouts: HashMap::new(),
        viewed_terminals_sent: None,
        viewed_dirty: false,
        pane_saved: HashMap::new(),
        pane_restored: Default::default(),
        pane_synced_displayed: None,
        pane_synced_project: None,
        pane_sizes: Default::default(),
        pane_sent: Default::default(),
        pane_refusal: None,
        divider_dragging: false,
        pending_drops: Vec::new(),
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
        daemon_catalogs: Default::default(),
        displaced: HashMap::new(),
        disconnected: Default::default(),
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        composer: iced::widget::text_editor::Content::new(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: Default::default(),
        build_mismatch: Default::default(),
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        pending_daemon: HashMap::new(),
        pending_clones: HashMap::new(),
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

/// Feature 1121 T027, FR-005: the app's update collapses the expanded card on the Overview's
/// `Expansion(WindowBlurred)`, and the window regaining focus expands none (Edge Case *Window
/// regains focus*). That the mounted Overview publishes this message on the window's blur is
/// `ui/overview.rs` `the_window_losing_focus_publishes_the_expansion_blur` (review A F4, B F1);
/// the root's focus arm names no feature.
#[test]
fn expansion_window_blurred_collapses_and_refocus_expands_none() {
    use micold_client::features::overview::Msg as OverviewMsg;
    use micold_core::overview_expansion::Event;
    use micold_core::project::{Availability, Project};
    use micold_core::session::SessionLocation;

    let mut app = base_app();
    let path = PathBuf::from("repo");
    app.core.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    app.core.workspace.active = Some(path);
    app.core.overview.open = true;
    let _ = update_inner(
        &mut app,
        Message::Overview(OverviewMsg::Expansion(Event::FocusGained(
            SessionLocation::Default,
        ))),
    );
    assert_eq!(
        app.core.overview.expansion.expanded(),
        Some(&SessionLocation::Default)
    );
    let _ = update_inner(
        &mut app,
        Message::Overview(OverviewMsg::Expansion(Event::WindowBlurred)),
    );
    assert_eq!(app.core.overview.expansion.expanded(), None);
    let _ = update_inner(&mut app, Message::WindowFocusChanged(true));
    assert_eq!(app.core.overview.expansion.expanded(), None);
}

/// Review A F1 (M4), FR-013: opening the Overview through the app puts the keyboard on the first
/// card, and that card's focus report, which follows on the next frame, expands nothing.
#[test]
fn expansion_none_after_the_opening_focus() {
    use micold_client::features::overview::Msg as OverviewMsg;
    use micold_core::overview_expansion::Event;
    use micold_core::project::{Availability, Project};
    use micold_core::session::SessionLocation;

    let mut app = base_app();
    let path = PathBuf::from("repo");
    app.core.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    app.core.workspace.active = Some(path);
    let opened = update_inner(
        &mut app,
        Message::Overview(OverviewMsg::Toggled {
            settings_open: false,
        }),
    );
    assert!(app.core.overview.open);
    assert!(opened.units() > 0, "the opening focuses the first card");
    let _ = update_inner(
        &mut app,
        Message::Overview(OverviewMsg::Expansion(Event::FocusGained(
            SessionLocation::Default,
        ))),
    );
    assert_eq!(app.core.overview.expansion.expanded(), None);
}

/// Feature 1121 T038/T043, FR-008: `CardPaged` on an expanded card answers with a task that pages
/// its body; with no card expanded it answers with nothing. What that task does, one visible
/// height down or up on the expanded card's `card_scroll_id` and no other scrollable, is
/// `ui/overview.rs` `a_page_scrolls_the_named_body_by_its_visible_height` (review B F1, M4).
#[test]
fn expansion_card_paged_scrolls() {
    use micold_client::features::overview::Msg as OverviewMsg;
    use micold_core::overview_expansion::Event;
    use micold_core::session::SessionLocation;

    use micold_core::project::{Availability, Project};

    let mut app = base_app();
    // A project, so the Default card is listed and its expansion stands.
    let path = PathBuf::from("repo");
    app.core.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    app.core.workspace.active = Some(path);
    app.core.overview.open = true;
    let paged = |app: &mut App| {
        update_inner(
            app,
            Message::Overview(OverviewMsg::CardPaged { down: true }),
        )
        .units()
    };
    assert_eq!(paged(&mut app), 0, "nothing expanded: nothing to page");
    let _ = update_inner(
        &mut app,
        Message::Overview(OverviewMsg::Expansion(Event::FocusGained(
            SessionLocation::Default,
        ))),
    );
    assert!(paged(&mut app) > 0, "the expanded card's body is paged");
}

#[test]
fn terminal_resized_remembers_the_pane_size_for_future_spawns() {
    // Reproduces the reported bug: a freshly spawned session used to always start at the
    // hardcoded INIT_ROWS x INIT_COLS default, filling only that fixed area until the next
    // window resize reconciled it. `TerminalResized` (published by the pane widget whenever
    // its live size changes) must now be remembered on `App` so `spawn_pty` call sites can
    // seed new sessions at the pane's actual current size instead.
    let mut app = App {
        git_credentials: crate::shell::scoped_credential::GitCredentials::idle(),
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        pane_layouts: HashMap::new(),
        viewed_terminals_sent: None,
        viewed_dirty: false,
        pane_saved: HashMap::new(),
        pane_restored: Default::default(),
        pane_synced_displayed: None,
        pane_synced_project: None,
        pane_sizes: Default::default(),
        pane_sent: Default::default(),
        pane_refusal: None,
        divider_dragging: false,
        pending_drops: Vec::new(),
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
        daemon_catalogs: Default::default(),
        displaced: HashMap::new(),
        disconnected: Default::default(),
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        composer: iced::widget::text_editor::Content::new(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: Default::default(),
        build_mismatch: Default::default(),
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        pending_daemon: HashMap::new(),
        pending_clones: HashMap::new(),
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

/// Feature 813 (T014, FR-023): picking a session while the Overview is open closes it and shows
/// that session.
#[test]
fn selecting_a_session_closes_the_overview_and_shows_it() {
    let (mut app, _rx) = connected_app();
    app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
    app.core.overview.open = true;
    let id = SessionId::new();

    let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

    assert!(!app.core.overview.open);
    assert_eq!(app.core.session.active, Some(id));
}

/// Feature 971 (T012, FR-007): an activation request keeps the Overview up, and the delay's
/// `ActivationDue` then follows it: the Overview closes and the session shows. A second
/// `ActivationDue` does nothing.
#[test]
fn an_overview_activation_follows_once_its_delay_is_due() {
    // The delay is a `tokio::time::sleep`, which needs a runtime to be built.
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = runtime.enter();
    use micold_client::features::overview::Msg as OverviewMsg;
    let (mut app, _rx) = connected_app();
    app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
    app.core.overview.open = true;
    let id = SessionId::new();
    let activation = micold_core::overview::Activation::OpenSession(id);

    let _ = update_inner(
        &mut app,
        Message::Overview(OverviewMsg::ActivationRequested(activation)),
    );
    assert!(
        app.core.overview.open,
        "the Overview stays up for the ripple"
    );
    assert_eq!(app.core.session.active, None, "nothing is followed yet");

    let _ = update_inner(&mut app, Message::Overview(OverviewMsg::ActivationDue));
    assert!(!app.core.overview.open);
    assert_eq!(app.core.session.active, Some(id));

    app.core.session.active = None;
    let _ = update_inner(&mut app, Message::Overview(OverviewMsg::ActivationDue));
    assert_eq!(
        app.core.session.active, None,
        "a second due follows nothing"
    );
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
    let (mut app, mut rx) = connected_app();
    app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
    let id = SessionId::new();
    app.last_grid = Some((220, 60));

    let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

    match rx.try_recv() {
        Ok(ClientMsg::SessionResize {
            session,
            cols,
            rows,
            ..
        }) => {
            assert_eq!(session, id);
            assert_eq!((cols, rows), (220, 60));
        }
        other => panic!("expected the size first, got {other:?}"),
    }
    assert!(
        matches!(
            rx.try_recv(),
            Ok(ClientMsg::SessionStart { session, .. }) if session == id
        ),
        "the start must follow the size, not precede it"
    );
}

/// The pane has not been laid out yet (nothing has published a size): the start goes out alone
/// and the daemon's own default applies. A zero-size guess here would be worse than no guess.
#[test]
fn displaying_a_session_before_the_pane_has_a_size_sends_only_the_start() {
    let (mut app, mut rx) = connected_app();
    app.core.workspace.active = Some(std::path::PathBuf::from("/tmp/project"));
    let id = SessionId::new();
    assert_eq!(app.last_grid, None);

    let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

    assert!(
        matches!(
            rx.try_recv(),
            Ok(ClientMsg::SessionStart { session, .. }) if session == id
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
    app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
    app.core.session.active = Some(id);

    let _ = update_inner(
        &mut app,
        Message::Session(SessionMsg::TerminalRestartRequested),
    );
    let sent: Vec<ClientMsg> = drain(&mut rx);
    assert!(
        sent.iter()
            .any(|m| matches!(m, ClientMsg::SessionRestart { session, .. } if *session == id)),
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
    let sent: Vec<ClientMsg> = drain(&mut rx);
    assert!(
        sent.iter()
            .any(|m| matches!(m, ClientMsg::SessionStart { session, .. } if *session == id))
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
        jira_enabled: false,
        jira_connections: Default::default(),
        github_issues_enabled: true,
        jira_base_urls: Default::default(),
        plan_usage_enabled: true,
        plan_usage_warn_percent: 80,
        scrollback_lines: micold_core::settings::DEFAULT_SCROLLBACK_LINES,
        env_include_enabled: false,
        env_include_script_path: String::new(),
        env_include_timeout_secs: micold_core::settings::DEFAULT_ENV_INCLUDE_TIMEOUT_SECS,
        default_ai_cli: AiCli::ClaudeCode,
        default_prompt_template: String::new(),
        pi_activity_component: true,
        save_terminal_history: true,
        tool_server_enabled: true,
        desktop_notifications: true,
        notification_kinds: Default::default(),
        long_task_threshold_secs: 60,
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
            plan_usage: None,
        }),
    );
    drain(&mut rx)
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
        .position(|m| matches!(m, ClientMsg::SessionStart { session, .. } if *session == id))
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
            ClientMsg::SessionStart { session, .. } => Some(*session),
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
            ClientMsg::SessionStart { session, .. } => Some(*session),
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
    let mut app = base_app_fields();
    app.ensure_test_registry();
    app
}

fn base_app_fields() -> App {
    App {
        git_credentials: crate::shell::scoped_credential::GitCredentials::idle(),
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
        core: {
            // As startup seeds it from the loaded settings (feature 613): a save validates it.
            let mut core = State::default();
            core.session.long_task_threshold_secs =
                micold_core::attention::LONG_TASK_THRESHOLD.as_secs();
            core
        },
        reported_scheme: None,
        grids: HashMap::new(),
        pane_layouts: HashMap::new(),
        viewed_terminals_sent: None,
        viewed_dirty: false,
        pane_saved: HashMap::new(),
        pane_restored: Default::default(),
        pane_synced_displayed: None,
        pane_synced_project: None,
        pane_sizes: Default::default(),
        pane_sent: Default::default(),
        pane_refusal: None,
        divider_dragging: false,
        pending_drops: Vec::new(),
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
        daemon_catalogs: Default::default(),
        displaced: HashMap::new(),
        disconnected: Default::default(),
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        composer: iced::widget::text_editor::Content::new(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: Default::default(),
        build_mismatch: Default::default(),
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        pending_daemon: HashMap::new(),
        pending_clones: HashMap::new(),
        probe: None,
        scene_ready: false,
        scene_frames: 0,
        ripples_animating: Arc::new(AtomicUsize::new(0)),
        scene_ripple_frames: std::cell::Cell::new(0),
    }
}

type SentRx = iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>;

/// A window already connected to a daemon: `base_app()` with an outbox, and the receiver that reads
/// what the app sent.
pub(crate) fn connected_app() -> (App, SentRx) {
    let (tx, rx) = iced::futures::channel::mpsc::unbounded();
    let mut app = base_app();
    app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
    (app, rx)
}

/// Everything sent to the daemon since the last drain.
pub(crate) fn drain(rx: &mut SentRx) -> Vec<ClientMsg> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

/// What `pick` makes of each message sent since the last drain, in order, skipping the `None`s.
pub(crate) fn sent_matching<T>(
    rx: &mut SentRx,
    pick: impl FnMut(ClientMsg) -> Option<T>,
) -> Vec<T> {
    drain(rx).into_iter().filter_map(pick).collect()
}

/// Run `work` and every message it produces back through the shell until nothing is left.
pub(crate) fn settle(app: &mut App, work: Task<Message>) {
    let mut queue: std::collections::VecDeque<Message> = messages(work).into();
    while let Some(message) = queue.pop_front() {
        queue.extend(messages(update_inner(app, message)));
    }
}

/// Answer the `RemoteList` request `req` with `remotes`.
pub(crate) fn answer_remotes(app: &mut App, req: u64, remotes: Vec<micold_core::git::GitRemote>) {
    let work = shell::daemon_sync::on_daemon_event(
        app,
        micold_core::daemons::DaemonId(1),
        DaemonMsg::OperationOk {
            req,
            result: micold_core::protocol::messages::OperationResult::RemoteList { remotes },
        },
    );
    settle(app, work);
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
        container_name: micold_core::sandbox::CONTAINER_NAME.to_string(),
        port: micold_core::endpoint::DEFAULT_SANDBOX_PORT,
    });
    app
}

#[test]
fn accepting_the_fallback_moves_the_connection_to_a_host_process() {
    // The defect this was written for: `SandboxFallbackAccepted` used to record consent and
    // return `Task::none()`, and nothing else changed. But `daemon::connections` dials from
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
    let mut cache = GridCache::new();
    cache.apply(&frame_of(
        session,
        micold_core::protocol::messages::SessionProcess::Primary,
        viewport_top,
        rows,
    ));
    cache
}

/// A full frame for one terminal of `session` (feature 484).
fn frame_of(
    session: SessionId,
    process: micold_core::protocol::messages::SessionProcess,
    viewport_top: i64,
    rows: u16,
) -> micold_core::protocol::grid::GridFrame {
    use micold_core::protocol::grid::{
        GridFrame, WireCursor, WireCursorShape, WireLine, WireStyle,
    };
    GridFrame {
        process,
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
    }
}

/// An `App` scrolled-ready: one session, its grid at the tail, and a socket to read.
fn app_at_the_tail() -> (
    App,
    iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
) {
    let (mut app, rx) = connected_app();
    let id = SessionId::new();
    app.core.session.active = Some(id);
    app.grids.insert(
        micold_core::protocol::messages::TerminalRef {
            session: id,
            process: micold_core::protocol::messages::SessionProcess::Primary,
        },
        at_the_tail(id, 4000, 69),
    );
    (app, rx)
}

fn scrollback_ranges(rx: &mut SentRx) -> Vec<Range<LineId>> {
    sent_matching(rx, |msg| match msg {
        ClientMsg::ScrollbackRequest { ranges, .. } => Some(ranges),
        _ => None,
    })
    .into_iter()
    .flatten()
    .collect()
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

    let _ = shell::daemon_sync::on_disconnected(&mut app, micold_core::daemons::DaemonId(1));

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
    let (mut app, rx) = connected_app();
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

    let _ = shell::daemon_sync::on_disconnected(&mut app, micold_core::daemons::DaemonId(1));

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
    let _ = shell::daemon_sync::on_disconnected(&mut app, micold_core::daemons::DaemonId(1));
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
    let (mut app, _rx) = connected_app();
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    app.core
        .update(Message::WorktreeForm(FormMsg::NameChanged("probe".into())));

    let _ = shell::daemon_sync::on_disconnected(&mut app, micold_core::daemons::DaemonId(1));

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

    let _ = shell::daemon_sync::on_disconnected(&mut app, micold_core::daemons::DaemonId(1));

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

// --- Feature 802 (M1): a person's start at an occupied location asks, it does not refuse --------

const OCCUPANT_ID: &str = "00000000-0000-0000-0000-000000000001";

fn occupied_loc() -> micold_core::session::SessionLocation {
    micold_core::session::SessionLocation::Worktree("feat-x".into())
}

/// A connected app on `/repo/demo` whose catalog holds `sessions` (all at `feat-x`), the first main.
fn app_with_sessions_at_feat_x(
    count: usize,
) -> (App, SentRx, Vec<micold_core::session::SessionId>) {
    use micold_core::session::Session;
    let (mut app, rx) = connected_app();
    let project = PathBuf::from("/repo/demo");
    app.core.workspace.active = Some(project.clone());
    let sessions: Vec<Session> = (0..count)
        .map(|i| {
            let mut s = Session::start_new(occupied_loc(), AiCli::ClaudeCode);
            s.main = i == 0;
            s
        })
        .collect();
    let ids = sessions.iter().map(|s| s.id).collect();
    app.core.workspace.sessions.insert(project, sessions);
    (app, rx, ids)
}

fn creates(rx: &mut SentRx) -> Vec<(CreateTarget, bool)> {
    sent_matching(rx, |m| match m {
        ClientMsg::SessionCreate {
            provider,
            allow_additional_session,
            ..
        } => Some(((provider,), allow_additional_session)),
        _ => None,
    })
}

type CreateTarget = (AiCli,);

fn start(app: &mut App, provider: AiCli, automatic: bool) {
    let _ = shell::daemon_sync::on_session_start_requested(
        app,
        occupied_loc(),
        provider,
        micold_core::session::SessionKind::Terminal,
        automatic,
    );
}

fn pending_create(app: &App) -> u64 {
    *app.pending_ops
        .iter()
        .find(|(_, op)| matches!(op, PendingOp::CreateSession { .. }))
        .expect("a create is pending")
        .0
}

const REFUSAL_TEXT: &str = "this worktree already has a main session";

fn refuse(app: &mut App, req: u64, detail: Option<&str>) {
    refuse_as(
        app,
        req,
        micold_core::protocol::messages::ErrorKind::Refused,
        detail,
    );
}

fn refuse_as(
    app: &mut App,
    req: u64,
    kind: micold_core::protocol::messages::ErrorKind,
    detail: Option<&str>,
) {
    let _ = shell::daemon_sync::on_daemon_event(
        app,
        micold_core::daemons::DaemonId(1),
        DaemonMsg::OperationError {
            req,
            kind,
            message: REFUSAL_TEXT.into(),
            detail: detail.map(str::to_owned),
        },
    );
}

/// A start of `provider` at the free `feat-x`, sent and left pending; returns its request id.
fn pending_start(app: &mut App, rx: &mut SentRx, provider: AiCli, automatic: bool) -> u64 {
    start(app, provider, automatic);
    let _ = creates(rx);
    pending_create(app)
}

/// Asserts the refusal left no confirmation and showed the service's own text.
fn assert_stays_a_notice(app: &App) {
    assert_eq!(app.core.additional_request(), None);
    assert_eq!(
        visible_notice(app).as_deref(),
        Some(REFUSAL_TEXT),
        "the service's text is shown as worded"
    );
}

fn visible_notice(app: &App) -> Option<String> {
    app.core
        .notifications
        .queue
        .visible()
        .map(|n| n.message.clone())
}

/// US1 s1, FR-001, FR-007: a start for an occupied location opens the confirmation and sends nothing.
#[test]
fn a_start_for_an_occupied_location_opens_the_confirmation_and_sends_no_create() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(1);
    let before = app.core.workspace.sessions.clone();
    start(&mut app, AiCli::Copilot, false);
    assert_eq!(
        app.core.additional_request().map(|(location, _)| location),
        Some(occupied_loc())
    );
    assert!(
        creates(&mut rx).is_empty(),
        "nothing is created before the user confirms"
    );
    assert!(app.pending_ops.is_empty());
    assert_eq!(visible_notice(&app), None);
    assert_eq!(app.core.workspace.sessions, before);
}

/// FR-004: a stale start the daemon refused as occupied opens the confirmation, with no notice.
#[test]
fn a_refusal_naming_the_occupant_opens_the_confirmation_not_an_error() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(0);
    start(&mut app, AiCli::Pi, false);
    assert_eq!(creates(&mut rx), vec![((AiCli::Pi,), false)]);
    let req = pending_create(&app);
    refuse(&mut app, req, Some(OCCUPANT_ID));
    assert_eq!(
        app.core.additional_request().map(|(location, _)| location),
        Some(occupied_loc())
    );
    assert_eq!(
        app.core.additional_request(),
        Some((occupied_loc(), AiCli::Pi))
    );
    assert_eq!(visible_notice(&app), None, "no error shows");
}

/// FR-004: a refusal that lands after the person switched project must not reopen the
/// confirmation, whose confirm would create in the then-active project.
#[test]
fn a_refusal_after_a_project_switch_stays_a_notice() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(0);
    start(&mut app, AiCli::Pi, false);
    let _ = creates(&mut rx);
    let req = pending_create(&app);
    app.core.workspace.active = Some(std::path::PathBuf::from("/repo/other"));
    refuse(&mut app, req, Some(OCCUPANT_ID));
    assert_eq!(app.core.additional_request(), None);
    assert!(visible_notice(&app).is_some());
}

/// FR-004: a refusal that names no occupant shows the service's text.
#[test]
fn a_refusal_without_an_occupant_stays_a_notice() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(0);
    let req = pending_start(&mut app, &mut rx, AiCli::ClaudeCode, false);
    refuse(&mut app, req, None);
    assert_stays_a_notice(&app);
}

/// FR-004: an automatic start's refusal shows the service's text even when it names the occupant.
#[test]
fn an_automatic_refusal_stays_a_notice() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(0);
    let req = pending_start(&mut app, &mut rx, AiCli::ClaudeCode, true);
    refuse(&mut app, req, Some(OCCUPANT_ID));
    assert_stays_a_notice(&app);
}

/// FR-004, T007: the refusal of an already-confirmed (flagged) create is a plain notice, never a
/// second confirmation.
#[test]
fn the_refusal_of_a_confirmed_create_stays_a_notice() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(1);
    start(&mut app, AiCli::Copilot, false);
    let _ = shell::daemon_sync::on_additional_session_confirmed(&mut app);
    assert_eq!(creates(&mut rx), vec![((AiCli::Copilot,), true)]);
    let req = pending_create(&app);
    refuse(&mut app, req, Some(OCCUPANT_ID));
    assert_stays_a_notice(&app);
}

/// T007: the typed kind decides, so a non-`Refused` error with an occupant-shaped detail is a notice.
#[test]
fn a_non_refusal_error_carrying_a_session_id_stays_a_notice() {
    use micold_core::protocol::messages::ErrorKind;
    for kind in [
        ErrorKind::Busy,
        ErrorKind::Internal,
        ErrorKind::AlreadyExists,
    ] {
        let (mut app, mut rx, _) = app_with_sessions_at_feat_x(0);
        let req = pending_start(&mut app, &mut rx, AiCli::Pi, false);
        refuse_as(&mut app, req, kind, Some(OCCUPANT_ID));
        assert_stays_a_notice(&app);
    }
}

/// T007: a `Refused` whose detail is not a session id names no occupant, so it is a notice.
#[test]
fn a_refusal_with_a_detail_that_is_not_a_session_id_stays_a_notice() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(0);
    let req = pending_start(&mut app, &mut rx, AiCli::Pi, false);
    refuse(&mut app, req, Some("not-a-uuid"));
    assert_stays_a_notice(&app);
}

/// FR-004: the startup auto-start asks for an automatic start.
#[test]
fn the_startup_auto_start_requests_an_automatic_start() {
    let (mut app, _rx) = connected_app();
    app.core.workspace.active = Some(PathBuf::from("/repo/demo"));
    let sent = messages(compose_scene(&mut app));
    assert!(
        sent.iter().any(|m| matches!(
            m,
            Message::Session(SessionMsg::StartRequested {
                automatic: true,
                ..
            })
        )),
        "the scene's start is automatic, got {sent:?}"
    );
}

/// FR-004: an automatic start at an occupied location is sent, not turned into the confirmation.
#[test]
fn an_automatic_start_at_an_occupied_location_sends_the_create() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(1);
    start(&mut app, AiCli::ClaudeCode, true);
    assert_eq!(app.core.additional_request(), None);
    assert_eq!(creates(&mut rx), vec![((AiCli::ClaudeCode,), false)]);
}

/// US1 s2, FR-002: confirming sends exactly one flagged create with the chosen CLI.
#[test]
fn confirming_sends_one_flagged_create_with_the_chosen_cli() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(1);
    start(&mut app, AiCli::Copilot, false);
    let _ = shell::daemon_sync::on_additional_session_confirmed(&mut app);
    assert_eq!(creates(&mut rx), vec![((AiCli::Copilot,), true)]);
    assert_eq!(app.core.additional_request(), None);
}

/// US1 s3, FR-003: cancelling sends nothing, changes nothing and shows no error.
#[test]
fn cancelling_creates_nothing_and_leaves_the_catalog() {
    let (mut app, mut rx, _) = app_with_sessions_at_feat_x(1);
    let before = app.core.workspace.sessions.clone();
    start(&mut app, AiCli::Copilot, false);
    app.core
        .update(Message::Session(SessionMsg::AdditionalCancelled));
    assert!(creates(&mut rx).is_empty());
    assert_eq!(app.core.workspace.sessions, before);
    assert_eq!(app.core.additional_request(), None);
    assert_eq!(visible_notice(&app), None);
}

/// US1 s4 regression: the row entry opens the confirmation.
#[test]
fn the_row_entry_opens_the_confirmation() {
    let (mut app, _rx, _) = app_with_sessions_at_feat_x(2);
    app.core
        .update(Message::Session(SessionMsg::AdditionalRequested(
            occupied_loc(),
        )));
    assert_eq!(
        app.core.additional_request().map(|(location, _)| location),
        Some(occupied_loc())
    );
    assert!(confirmation_is_shown(&app), "the dialog is on screen");
}

/// US1 s4 regression: a start another live session blocks opens the confirmation.
#[test]
fn the_blocked_start_opens_the_confirmation() {
    let (mut app, _rx, ids) = app_with_sessions_at_feat_x(2);
    let sessions = app
        .core
        .workspace
        .sessions
        .get_mut(Path::new("/repo/demo"))
        .unwrap();
    sessions[0].lifecycle = micold_core::session::SessionLifecycle::Running;
    sessions[1].lifecycle = micold_core::session::SessionLifecycle::Idle;
    app.core
        .update(Message::Session(SessionMsg::StartBlocked(ids[1])));
    assert_eq!(app.core.session.blocked_start, Some(ids[1]));
    assert!(
        app.core.session_start_block_text(ids[1]).is_some(),
        "the confirmation has text to show"
    );
    assert!(confirmation_is_shown(&app), "the dialog is on screen");
}

/// BUG-1219: the passive paths (switch, reconnect, attach-resume) view a blocked session without
/// reopening the confirmation after Cancel; an explicit selection still prompts.
#[test]
fn a_cancelled_blocked_start_does_not_reopen_on_a_passive_view() {
    let (mut app, _rx, ids) = app_with_sessions_at_feat_x(2);
    let sessions = app
        .core
        .workspace
        .sessions
        .get_mut(Path::new("/repo/demo"))
        .unwrap();
    sessions[0].lifecycle = micold_core::session::SessionLifecycle::Running;
    sessions[1].lifecycle = micold_core::session::SessionLifecycle::Idle;
    app.core.session.active = Some(ids[1]);

    let _ = shell::daemon_sync::on_session_selected(&mut app, ids[1]);
    assert_eq!(
        app.core.session.blocked_start,
        Some(ids[1]),
        "explicit start prompts"
    );

    app.core
        .update(Message::Session(SessionMsg::AdditionalCancelled));
    shell::daemon_sync::view_and_start_quietly(&mut app, ids[1]);
    assert_eq!(
        app.core.session.blocked_start, None,
        "a passive view does not reprompt"
    );
    assert!(!confirmation_is_shown(&app));
}

fn confirmation_is_shown(app: &App) -> bool {
    micold_client::ui::confirm_additional_session::dialog(
        &app.core,
        micold_core::theme::ColorScheme::Dark,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
    )
    .is_some()
}

/// FR-004: with several sessions the confirmation names the main one, not a helper.
#[test]
fn with_several_sessions_the_confirmation_names_the_main_one() {
    use micold_core::session::SessionLabel;
    let (mut app, _rx, _) = app_with_sessions_at_feat_x(3);
    for (session, name) in app
        .core
        .workspace
        .sessions
        .get_mut(Path::new("/repo/demo"))
        .unwrap()
        .iter_mut()
        .zip(["the-main-one", "second-one", "third-one"])
    {
        session.label = SessionLabel::Named(name.into());
    }
    start(&mut app, AiCli::ClaudeCode, false);
    let (place, existing) =
        micold_client::ui::confirm_additional_session::target_text(&app.core).unwrap();
    assert!(place.starts_with("The worktree"), "got {place:?}");
    assert_eq!(existing.as_deref(), Some("the-main-one"));
    let text =
        micold_client::ui::confirm_additional_session::body_text(&place, existing.as_deref());
    assert!(text.contains("the-main-one"));
    assert!(!text.contains("second-one") && !text.contains("third-one"));
}

/// The daemon's catalog push for `ids` (the first main) at `feat-x`, on `/repo/demo`.
fn catalog_of(ids: &[micold_core::session::SessionId]) -> DaemonMsg {
    let sessions = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let mut s = summary(*id, "s", WireLifecycle::Running);
            s.worktree_dir = Some("feat-x".into());
            s.main = i == 0;
            s
        })
        .collect();
    DaemonMsg::CatalogChanged {
        catalog: snapshot_with("/repo/demo", sessions),
    }
}

/// US2 s1-s2 (T010): after the confirmed create the daemon's catalog lists both sessions, the
/// original still main and the new one selected; a real delete of one leaves the other.
#[test]
fn after_confirming_both_sessions_are_listed_and_deleting_one_leaves_the_other() {
    use micold_core::protocol::messages::OperationResult;
    let (mut app, mut rx, ids) = app_with_sessions_at_feat_x(1);
    let daemon = micold_core::daemons::DaemonId(1);
    let sessions_of = |app: &App| {
        app.core
            .workspace
            .sessions
            .get(Path::new("/repo/demo"))
            .cloned()
            .unwrap_or_default()
    };
    start(&mut app, AiCli::ClaudeCode, false);
    let _ = shell::daemon_sync::on_additional_session_confirmed(&mut app);
    assert_eq!(creates(&mut rx), vec![((AiCli::ClaudeCode,), true)]);
    let req = pending_create(&app);

    // The daemon creates the helper, pushes the catalog, then answers the create.
    let helper_id = micold_core::session::SessionId::new();
    let _ = shell::daemon_sync::on_daemon_event(&mut app, daemon, catalog_of(&[ids[0], helper_id]));
    let _ = shell::daemon_sync::on_daemon_event(
        &mut app,
        daemon,
        DaemonMsg::OperationOk {
            req,
            result: OperationResult::SessionCreated { session: helper_id },
        },
    );
    let listed = sessions_of(&app);
    assert_eq!(listed.len(), 2, "both sessions are listed");
    assert!(
        listed.iter().find(|s| s.id == ids[0]).unwrap().main,
        "the original stays main"
    );
    assert!(
        !listed.iter().find(|s| s.id == helper_id).unwrap().main,
        "the new one is a helper"
    );
    assert_eq!(
        app.core.session.active,
        Some(helper_id),
        "the new session is selected"
    );

    // Deleting the helper goes to the daemon, whose next push no longer lists it.
    let _ = shell::daemon_sync::on_session_close_requested(&mut app, helper_id);
    assert!(drain(&mut rx)
        .iter()
        .any(|m| matches!(m, ClientMsg::SessionDelete { session, .. } if *session == helper_id)));
    let _ = shell::daemon_sync::on_daemon_event(&mut app, daemon, catalog_of(&[ids[0]]));
    let left = sessions_of(&app);
    assert_eq!(left.iter().map(|s| s.id).collect::<Vec<_>>(), vec![ids[0]]);
    assert!(left[0].main, "the original is still main");
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
        micold_core::daemons::DaemonId(1),
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
        micold_core::daemons::DaemonId(1),
        DaemonMsg::OperationOk {
            req,
            result: micold_core::protocol::messages::OperationResult::WorktreeCreated {
                dir_name: "feat-probe-two".into(),
                session: None,
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

/// Feature 491, FR-006: the daemon chosen in the form when the create was sent is the worktree's
/// binding once the daemon reports it created, and not before.
#[test]
fn a_created_worktree_is_bound_to_the_daemon_chosen_in_the_form() {
    use micold_core::daemons::{DaemonRegistry, DaemonRuntime};
    let (mut app, _rx) = connected_app();
    let project = PathBuf::from("/repo/demo");
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let other = registry
        .add(
            "Other",
            DaemonRuntime::Container(micold_core::daemons::ContainerSettings {
                profile: Default::default(),
                container_name: "other".into(),
                port: 7001,
                migration: None,
            }),
            true,
        )
        .unwrap();
    app.core.settings.daemons = registry;
    // The chosen daemon must be reachable: a create for a daemon that is down fails (bug 1159).
    let (tx, _rx_other) = iced::futures::channel::mpsc::unbounded();
    let links = &mut app.core.settings.links;
    links.insert(other, micold_core::daemons::DaemonState::Starting);
    links.connected(other, micold_client::daemon::Outbox::new(tx));
    app.core.workspace.active = Some(project.clone());
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    app.core
        .update(Message::WorktreeForm(FormMsg::DaemonChosen(other)));
    shell::daemon_sync::send_worktree_create(
        &mut app,
        project.clone(),
        micold_core::naming::DerivedNames {
            dir_name: "feat-probe-two".into(),
            branch: "feat/probe-two".into(),
        },
        micold_core::worktree::CreateMode::NewBranch,
    );
    let req = pending_create_req(&app);
    assert!(
        !app.core.workspace.bindings.contains_key(&project),
        "nothing is bound before the daemon says the worktree exists"
    );

    let _ = shell::daemon_sync::on_daemon_event(
        &mut app,
        micold_core::daemons::DaemonId(1),
        DaemonMsg::OperationOk {
            req,
            result: micold_core::protocol::messages::OperationResult::WorktreeCreated {
                dir_name: "feat-probe-two".into(),
                session: None,
            },
        },
    );

    assert_eq!(
        app.core.workspace.bindings[&project]["feat-probe-two"],
        other
    );
}

/// Feature 776, FR-006 (T018): the session a worktree was created with is selected when the
/// daemon reports the create, as a `SessionCreated` would select it.
#[test]
fn a_created_worktrees_returned_session_is_selected() {
    use micold_core::session::{AiCli, Session, SessionLocation};
    let (mut app, _rx) = app_creating_a_worktree();
    let req = pending_create_req(&app);
    let session = Session::start_new(
        SessionLocation::Worktree("feat-probe-two".to_string()),
        AiCli::ClaudeCode,
    );
    let id = session.id;
    app.core
        .workspace
        .sessions
        .insert(PathBuf::from("/repo/demo"), vec![session]);
    app.core.session.active = None;

    let _ = shell::daemon_sync::on_daemon_event(
        &mut app,
        micold_core::daemons::DaemonId(1),
        DaemonMsg::OperationOk {
            req,
            result: micold_core::protocol::messages::OperationResult::WorktreeCreated {
                dir_name: "feat-probe-two".into(),
                session: Some(id),
            },
        },
    );

    assert_eq!(
        app.core.session.active,
        Some(id),
        "the worktree's own session is the one in front of the user"
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
    app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));

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
    app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));

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
    let (mut app, mut rx) = connected_app();
    app.core.settings.settings_draft = Some(SettingsDraft {
        terminal: TerminalDraft {
            scrollback_lines: "20000".into(),
            save_terminal_history: true,
        },
        environment: EnvironmentDraft {
            enabled: false,
            script_path: "/tmp/does-not-exist.sh".into(),
            timeout_secs: "15".into(),
            default_ai_cli: AiCli::Copilot,
            default_prompt_template: String::new(),
            default_prompt_editor: Default::default(),
            pi_activity_component: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            notification_kinds: Default::default(),
            long_task_threshold_secs: "60".to_string(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: "80".to_string(),
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
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::ToolServerToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = drain(&mut rx);
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
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
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::DesktopNotificationsToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = drain(&mut rx);
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

/// U143, narrowed by BUG-570 (FR-026a): a save that leaves the switch as the page opened with it
/// does not name it, so it cannot undo another window's change.
#[test]
fn saving_without_touching_desktop_notifications_does_not_name_them() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = drain(&mut rx);
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            desktop_notifications,
            ..
        } => Some(*desktop_notifications),
        _ => None,
    });
    assert_eq!(
        told, None,
        "a save that changed nothing tells the service nothing: {sent:?}"
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
                    jira_enabled: false,
                    jira_connections: Default::default(),
                    github_issues_enabled: true,
                    jira_base_urls: Default::default(),
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

/// A50 (BUG-475, FR-026b, US4 scenario 8): another window's save reaches a Settings page that is
/// open here. Each field this window's user has not touched shows the value now stored; a field
/// they edited keeps their edit.
#[test]
fn an_open_settings_page_shows_another_windows_save_and_keeps_its_own_edits() {
    let mut app = base_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::ScrollbackChanged("4321".into())),
    );

    // Another window saves: notifications off, a new script path, and a scrollback limit this
    // window's user has already edited.
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                desktop_notifications: false,
                env_include_script_path: "/elsewhere/env.sh".into(),
                scrollback_lines: 9999,
                ..quiet_settings()
            },
        },
    );

    let draft = app
        .core
        .settings
        .settings_draft
        .as_ref()
        .expect("the page stays open");
    assert!(
        !draft.environment.desktop_notifications,
        "an untouched switch shows the value another window saved"
    );
    assert_eq!(
        draft.environment.script_path, "/elsewhere/env.sh",
        "an untouched field shows the value another window saved"
    );
    assert_eq!(
        draft.terminal.scrollback_lines, "4321",
        "a field this window's user edited keeps their edit"
    );
}

/// A50, the service-held switch (BUG-475, FR-026b, review A): the pull request switch follows the
/// service on an open page only while its user has not switched it there. Another window's save
/// of some other setting neither resets their edit nor, when untouched, leaves it stale.
#[test]
fn an_open_settings_page_keeps_an_edited_pr_status_switch_and_follows_an_untouched_one() {
    let mut app = base_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PrStatusToggled(true)),
    );
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                desktop_notifications: false,
                ..quiet_settings()
            },
        },
    );
    let draft = app.core.settings.settings_draft.as_ref().expect("open");
    assert!(
        draft.github.pr_status_enabled,
        "another window's save of another setting keeps this page's edit"
    );

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                pr_status_enabled: true,
                ..quiet_settings()
            },
        },
    );
    let draft = app.core.settings.settings_draft.as_ref().expect("open");
    assert!(
        draft.github.pr_status_enabled,
        "an untouched switch shows the value another window saved"
    );
    assert_eq!(
        draft.baseline.as_ref().map(|b| b.pr_status_enabled),
        Some(true),
        "and counts as unchanged, so a save does not send it"
    );
}

/// Feature 804, T007a / D3: a form save of the legacy-mirrored daemon carries the saved entry
/// into the single-daemon `daemon` block that is still written for rollback.
#[test]
fn a_daemon_form_save_syncs_the_legacy_daemon_block() {
    use micold_core::daemons::{ContainerSettings, DaemonRegistry, DaemonRuntime};
    use micold_core::sandbox::SandboxProfile;

    let mut start = micold_core::settings::Settings::default();
    start.daemon.placement = PlacementKind::HostProcess;
    let store = Arc::new(micold_core::settings::FakeSettingsStore::loaded(
        start.clone(),
    ));
    let mut app = base_app();
    app.caps = app.caps.clone().with_settings(store.clone());

    let profile = SandboxProfile {
        survive_logout: true,
        ..SandboxProfile::default()
    };
    let mut entries = start.daemons.clone();
    entries[0].runtime = DaemonRuntime::Container(ContainerSettings {
        profile: profile.clone(),
        container_name: "saved".into(),
        port: 4321,
        migration: None,
    });
    let registry = DaemonRegistry::new(entries, start.next_daemon_id);

    app.apply_daemon_outcome(&micold_client::features::Outcome::DaemonRegistryChanged(
        registry,
    ));

    let written = store
        .saves()
        .pop()
        .expect("the registry change was written");
    assert_eq!(written.daemon.placement, PlacementKind::LocalSandbox);
    assert_eq!(written.daemon.sandbox, profile);
}

/// A49 (BUG-570, FR-026a, US4 scenario 7): a page opened before another window turned both
/// switches off, then saved after changing only another setting, must not turn them back on: not
/// at the service, not in the file, not in this window.
#[test]
fn a_stale_settings_page_saves_only_what_its_user_changed() {
    use micold_core::settings::SettingsStore as _;

    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let store = Arc::new(micold_core::settings::FakeSettingsStore::loaded(
        micold_core::settings::Settings::default(),
    ));
    let mut app = base_app();
    app.caps = app.caps.clone().with_settings(store.clone());
    app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));

    // Window A's save, echoed by the service to every window and written to the file.
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                desktop_notifications: false,
                tool_server_enabled: false,
                ..quiet_settings()
            },
        },
    );
    store
        .save(&micold_core::settings::Settings {
            desktop_notifications: false,
            tool_server_enabled: false,
            ..store.load().settings
        })
        .expect("the fake store saves");
    drain(&mut rx);

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PiActivityComponentToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let sent: Vec<ClientMsg> = drain(&mut rx);
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            pi_activity_component,
            desktop_notifications,
            tool_server_enabled,
            ..
        } => Some((
            *pi_activity_component,
            *desktop_notifications,
            *tool_server_enabled,
        )),
        _ => None,
    });
    assert_eq!(
        told,
        Some((Some(false), None, None)),
        "the service is told the one change, and nothing about the switches: {sent:?}"
    );
    let saved = store.saves().last().cloned().expect("the save is written");
    assert_eq!(
        (
            saved.pi_activity_component,
            saved.desktop_notifications,
            saved.tool_server_enabled
        ),
        (false, false, false),
        "the file gets the change and keeps the other window's switches"
    );
    assert!(
        !app.core.session.desktop_notifications && !app.core.session.tool_server_enabled,
        "this window's own copy keeps the switches off"
    );
}

/// What a save told the service about the pull request switch: `None` when no `SettingsSet` went
/// out, `Some(None)` when one went out without the field, `Some(Some(v))` when it carried `v`.
fn pr_status_told(rx: &mut SentRx) -> Option<Option<bool>> {
    drain(rx).into_iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            pr_status_enabled, ..
        } => Some(pr_status_enabled),
        _ => None,
    })
}

/// U105 (feature 040, FR-029, FR-030): turning **Show pull request status on worktrees** on and
/// saving tells the connected service, which stores it and tells every window.
#[test]
fn turning_the_pr_status_switch_on_and_saving_tells_the_service() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PrStatusToggled(true)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert_eq!(pr_status_told(&mut rx), Some(Some(true)));
}

/// U105: a save that leaves the switch where the service has it does not name it, so a save never
/// writes over another window's change to it.
#[test]
fn a_save_that_leaves_the_pr_status_switch_alone_does_not_send_it() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    // A save that changed nothing the service owns tells it nothing at all (BUG-570).
    assert_eq!(pr_status_told(&mut rx), None);
}

/// U105: turning it off when the service has it on is a change, and is sent as `Some(false)`.
#[test]
fn turning_the_pr_status_switch_off_and_saving_tells_the_service() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                pr_status_enabled: true,
                ..quiet_settings()
            },
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::PrStatusToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert_eq!(pr_status_told(&mut rx), Some(Some(false)));
}

/// U106: an open Settings page follows the service's value when another window changes it.
#[test]
fn the_open_draft_follows_a_pr_status_change_from_the_service() {
    let mut app = base_app();
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let shown = |app: &App| {
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("the page is open")
            .github
            .pr_status_enabled
    };
    assert!(!shown(&app), "off on a first start");

    for reported in [true, false] {
        feed(
            &mut app,
            DaemonMsg::SettingsChanged {
                settings: DaemonSettings {
                    jira_enabled: false,
                    jira_connections: Default::default(),
                    github_issues_enabled: true,
                    jira_base_urls: Default::default(),
                    pr_status_enabled: reported,
                    ..quiet_settings()
                },
            },
        );
        assert_eq!(
            shown(&app),
            reported,
            "the page shows what the service says"
        );
    }
}

/// Feature 533 (FR-022): turning the Jira switch on and saving tells the service, and names
/// nothing else of the Jira fields (a connection is changed by its own message, M2).
#[test]
fn turning_the_jira_switch_on_and_saving_tells_the_service() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::JiraEnabledToggled(true)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let told: Vec<_> = drain(&mut rx)
        .into_iter()
        .filter_map(|msg| match msg {
            ClientMsg::SettingsSet {
                jira_enabled,
                jira_connection,
                pr_status_enabled,
                ..
            } => Some((jira_enabled, jira_connection, pr_status_enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(told, vec![(Some(true), None, None)]);
}

/// Feature 1122 M5 (D4, FR-010, contract W2): saving the open project's Jira base URL, with the
/// connection's fields blank, tells the service that one project's change and no connection change,
/// and puts it in force in this window; the trailing `/` is removed.
#[test]
fn saving_a_jira_base_url_tells_the_service_that_projects_change() {
    use micold_core::jira::{connection_key, JiraBaseUrl};
    use micold_core::settings::JiraBaseUrlChange;
    let (mut app, mut rx) = connected_app();
    app.core.workspace.active = Some(PathBuf::from("/work/app"));
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: true,
                ..quiet_settings()
            },
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    app.core
        .settings
        .settings_draft
        .as_mut()
        .expect("open")
        .jira
        .follow(
            Some(PathBuf::from("/work/app")),
            &Default::default(),
            &Default::default(),
            false,
        );
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::JiraBaseUrlChanged(
            "https://example.atlassian.net/".into(),
        )),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    let key = connection_key(std::path::Path::new("/work/app"));
    let base = JiraBaseUrl::parse("https://example.atlassian.net").expect("valid");
    let told: Vec<_> = drain(&mut rx)
        .into_iter()
        .filter_map(|msg| match msg {
            ClientMsg::SettingsSet {
                jira_connection,
                jira_base_url,
                ..
            } => Some((jira_connection, jira_base_url.map(|change| *change))),
            _ => None,
        })
        .collect();
    assert_eq!(
        told,
        vec![(
            None,
            Some(JiraBaseUrlChange {
                project: key.clone(),
                url: Some(base.clone()),
            })
        )]
    );
    assert_eq!(app.core.settings.jira_base_urls.get(&key), Some(&base));
}

/// Feature 533 (US2 AS5, T078): Save over a switch turned off with a stored connection raises the
/// question and tells the service nothing; each answer is applied by the save it held.
#[test]
fn switching_jira_off_over_a_stored_connection_asks_and_each_answer_is_saved() {
    use micold_client::features::settings::KeepOrRemove;

    for (choice, removes) in [(KeepOrRemove::Keep, false), (KeepOrRemove::Remove, true)] {
        let (mut app, mut rx) = connected_app();
        let connection = micold_core::jira::JiraConnection::parse(
            "https://acme.atlassian.net",
            "ABC",
            "me@acme.com",
        )
        .expect("a valid connection");
        app.core.workspace.active = Some(PathBuf::from("/work/app"));
        feed(
            &mut app,
            DaemonMsg::SettingsChanged {
                settings: DaemonSettings {
                    jira_enabled: true,
                    jira_connections: [("/work/app".to_owned(), connection)].into(),
                    ..quiet_settings()
                },
            },
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
        // The test workspace lists no project, so the page is pointed at the one holding the
        // connection, as `on_settings_opened` does for the open project.
        let connections = app.core.settings.jira_connections.clone();
        app.core
            .settings
            .settings_draft
            .as_mut()
            .expect("open")
            .jira
            .follow(
                Some(PathBuf::from("/work/app")),
                &connections,
                &Default::default(),
                true,
            );
        let _ = update_inner(
            &mut app,
            Message::Settings(SettingsMsg::JiraEnabledToggled(false)),
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

        let draft = app.core.settings.settings_draft.as_ref().expect("open");
        assert!(draft.jira.prompt.is_some(), "Save asks before applying");
        let told = |rx: &mut SentRx| -> Vec<(Option<bool>, bool)> {
            drain(rx)
                .into_iter()
                .filter_map(|msg| match msg {
                    ClientMsg::SettingsSet {
                        jira_enabled,
                        jira_connection,
                        ..
                    } => Some((jira_enabled, jira_connection.is_some())),
                    _ => None,
                })
                .collect()
        };
        assert_eq!(
            told(&mut rx),
            vec![],
            "nothing is applied until it is answered"
        );

        let _ = update_inner(
            &mut app,
            Message::Settings(SettingsMsg::JiraKeepOrRemoveChosen(choice)),
        );
        assert_eq!(told(&mut rx), vec![(Some(false), removes)], "{removes}");
    }
}

/// Feature 533 (FR-022): the window adopts the service's Jira switch and connections, and an open
/// untouched Settings page follows them.
#[test]
fn the_window_and_the_open_draft_follow_the_services_jira_settings() {
    let mut app = base_app();
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    assert!(!app.core.settings.jira_enabled, "off on a first start");

    let connection = micold_core::jira::JiraConnection::parse(
        "https://acme.atlassian.net",
        "ABC",
        "me@acme.com",
    )
    .expect("a valid connection");
    let mut connections = std::collections::BTreeMap::new();
    connections.insert("/work/app".to_owned(), connection);
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: true,
                jira_connections: connections.clone(),
                ..quiet_settings()
            },
        },
    );

    assert!(app.core.settings.jira_enabled);
    assert_eq!(app.core.settings.jira_connections, connections);
    assert!(
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("the page is open")
            .jira
            .enabled,
        "the open page shows what the service says"
    );
}

/// Feature 1102 (FR-008): turning GitHub issues off and saving tells the service that one field.
#[test]
fn turning_github_issues_off_and_saving_tells_the_service() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::GithubIssuesToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    let told: Vec<_> = drain(&mut rx)
        .into_iter()
        .filter_map(|msg| match msg {
            ClientMsg::SettingsSet {
                github_issues_enabled,
                jira_enabled,
                pr_status_enabled,
                ..
            } => Some((github_issues_enabled, jira_enabled, pr_status_enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(told, vec![(Some(false), None, None)]);
}

/// A save that leaves the switch alone does not name it, so it never overwrites another window's.
#[test]
fn a_save_that_leaves_github_issues_alone_does_not_send_it() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    assert!(drain(&mut rx).into_iter().all(|msg| !matches!(
        msg,
        ClientMsg::SettingsSet {
            github_issues_enabled: Some(_),
            jira_base_url: None,
            ..
        }
    )));
}

/// Feature 1102 (FR-008): the window adopts the service's switch, and an open untouched Settings
/// page follows it; a page whose user edited it keeps the edit.
#[test]
fn the_window_and_the_open_draft_follow_the_services_github_issues_switch() {
    let mut app = base_app();
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    assert!(
        app.core.settings.github_issues_enabled,
        "on on a first start"
    );
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                github_issues_enabled: false,
                jira_base_urls: Default::default(),
                ..quiet_settings()
            },
        },
    );
    assert!(!app.core.settings.github_issues_enabled);
    let shown = |app: &App| {
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("the page is open")
            .github
            .issues_enabled
    };
    assert!(!shown(&app), "the open page shows what the service says");

    // The user edits it on the page; the service then reports the other value: the edit stays.
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::GithubIssuesToggled(true)),
    );
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                github_issues_enabled: false,
                jira_base_urls: Default::default(),
                pr_status_enabled: true,
                ..quiet_settings()
            },
        },
    );
    assert!(shown(&app), "an edit on the page is not overwritten");
}

/// An app whose New worktree form is open on a GitHub remote.
fn app_with_github_form() -> App {
    let mut app = base_app();
    app.core.workspace.active = Some(PathBuf::from("/work/acme"));
    app.core.update(Message::WorktreeForm(FormMsg::Opened));
    app.core
        .update(Message::WorktreeForm(FormMsg::RemotesListed(Ok(vec![
            micold_core::git::GitRemote {
                name: "origin".into(),
                url: "git@github.com:o/r.git".into(),
            },
        ]))));
    app
}

fn form_offers_github(app: &App) -> bool {
    app.core
        .worktree_form
        .form
        .as_ref()
        .expect("the form is open")
        .github_repo()
        .is_some()
}

/// Feature 1102 (FR-005, SC-004, US3 AS1): the service's report of the switch reaches the form
/// already open in this window, with no reopening, both ways.
#[test]
fn an_open_form_follows_the_services_github_issues_switch() {
    let mut app = app_with_github_form();
    assert!(form_offers_github(&app));
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                github_issues_enabled: false,
                jira_base_urls: Default::default(),
                ..quiet_settings()
            },
        },
    );
    assert!(!form_offers_github(&app), "off drops GitHub issues");
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                ..quiet_settings()
            },
        },
    );
    assert!(form_offers_github(&app), "on offers them again");
}

/// The unconnected save puts the value in force (the test below) and then tells the form with the
/// same sync the service's report uses; Settings and the form are both dialogs, so a save seldom
/// has a form open, but the sync must hold when one is.
#[test]
fn the_github_issues_sync_reaches_the_open_form() {
    let mut app = app_with_github_form();
    app.core.settings.github_issues_enabled = false;
    crate::shell::issues::sync_github_issues_availability(&mut app);
    assert!(!form_offers_github(&app));
}

/// A save with no service to echo it still puts the new value in force for this window, whatever
/// the write does after (spec edge case).
#[test]
fn a_save_without_a_service_puts_the_github_issues_switch_in_force() {
    let mut app = base_app();
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::GithubIssuesToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    assert!(
        !app.core.settings.github_issues_enabled,
        "the new value is in force for this session"
    );
}

/// Feature 1102 (spec edge case): a settings write that fails leaves the switch in force for the
/// session and still tells the service; the page reopened shows it off.
#[test]
fn a_failed_settings_save_keeps_github_issues_in_force_and_still_tells_the_service() {
    let (mut app, mut rx) = connected_app();
    app.caps = app.caps.clone().with_settings(Arc::new(
        micold_core::settings::FakeSettingsStore::new()
            .failing_save(std::io::ErrorKind::PermissionDenied),
    ));
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::GithubIssuesToggled(false)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    assert!(
        !app.core.settings.github_issues_enabled,
        "the failed write does not roll the switch back"
    );
    let told = sent_matching(&mut rx, |msg| match msg {
        ClientMsg::SettingsSet {
            github_issues_enabled,
            ..
        } => Some(github_issues_enabled),
        _ => None,
    });
    assert_eq!(told, vec![Some(false)], "the service is told despite it");
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let draft = app.core.settings.settings_draft.as_ref().expect("open");
    assert!(
        !draft.github.issues_enabled,
        "the reopened page shows it off"
    );
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
        let (mut app, mut rx) = connected_app();
        // The window holds another value, so choosing this one is a change to save.
        app.core.session.cross_session_access = if chosen == CrossSessionAccess::Off {
            CrossSessionAccess::Auto
        } else {
            CrossSessionAccess::Off
        };
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
        let _ = update_inner(
            &mut app,
            Message::Settings(SettingsMsg::CrossSessionAccessChanged(chosen)),
        );
        let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

        let sent: Vec<ClientMsg> = drain(&mut rx);
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
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
    assert!(app.active_outbox().is_none());
    app.core.settings.settings_draft = Some(SettingsDraft {
        terminal: TerminalDraft {
            scrollback_lines: "20000".into(),
            save_terminal_history: true,
        },
        environment: EnvironmentDraft {
            enabled: true,
            script_path: String::new(),
            timeout_secs: "15".into(),
            default_ai_cli: AiCli::ClaudeCode,
            default_prompt_template: String::new(),
            default_prompt_editor: Default::default(),
            pi_activity_component: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            notification_kinds: Default::default(),
            long_task_threshold_secs: "60".to_string(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: "80".to_string(),
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

// --- A Settings save leaves the service where it is (feature 804) -----

/// Turn the keep-running opt-in on in the open Settings draft. The Settings page no longer has the
/// control (feature 804), but a save still applies a changed draft, which is what these tests
/// exercise.
fn keep_running_in_the_draft(app: &mut App) {
    app.core
        .settings
        .settings_draft
        .as_mut()
        .expect("the Settings view is open")
        .daemon
        .profile
        .survive_logout = true;
}

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
            save_terminal_history: true,
        },
        // A default `EnvironmentDraft` holds an *empty* timeout, which `validate` rejects —
        // and a save that never validates would pass the assertions below for the wrong
        // reason, reporting "nothing was applied" about a form that was simply never saved.
        environment: EnvironmentDraft {
            enabled: false,
            script_path: String::new(),
            timeout_secs: "5".into(),
            default_ai_cli: AiCli::ClaudeCode,
            default_prompt_template: String::new(),
            default_prompt_editor: Default::default(),
            pi_activity_component: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            notification_kinds: Default::default(),
            long_task_threshold_secs: "60".to_string(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: "80".to_string(),
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

/// Feature 804: Settings no longer moves the service, so its save never asks about the placement
/// and never restarts anything — it is the ordinary save every other setting goes through.
#[test]
fn a_settings_save_neither_asks_nor_restarts() {
    let mut app = app_saving_a_placement(PlacementKind::HostProcess, PlacementKind::HostProcess);

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert_eq!(
        app.placement.kind,
        PlacementKind::HostProcess,
        "saving a scrollback size moved where sessions run"
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                plan_usage_enabled: true,
                plan_usage_warn_percent: 80,
                default_ai_cli: AiCli::ClaudeCode,
                default_prompt_template: String::new(),
                scrollback_lines: 12_345,
                env_include_enabled: false,
                env_include_script_path: "/authoritative/from-daemon.sh".into(),
                env_include_timeout_secs: 30,
                pi_activity_component: true,
                save_terminal_history: true,
                tool_server_enabled: true,
                desktop_notifications: true,
                notification_kinds: Default::default(),
                long_task_threshold_secs: 60,
                diff_layout: Default::default(),
                cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                pr_status_enabled: false,
            },
            plan_usage: None,
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                plan_usage_enabled: true,
                plan_usage_warn_percent: 80,
                default_ai_cli: AiCli::ClaudeCode,
                default_prompt_template: String::new(),
                scrollback_lines: 5_000,
                env_include_enabled: false,
                env_include_script_path: "/tmp/after.sh".into(),
                env_include_timeout_secs: 45,
                pi_activity_component: true,
                save_terminal_history: true,
                tool_server_enabled: true,
                desktop_notifications: true,
                notification_kinds: Default::default(),
                long_task_threshold_secs: 60,
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
        git_credentials: crate::shell::scoped_credential::GitCredentials::idle(),
        caps: Capabilities::real()
            .without_settings()
            .without_projects()
            .with_link_opener(Arc::new(crate::shell::link_opener::NoopLinkOpener))
            .with_issue_tooling(crate::shell::capabilities::IssueTooling::none()),
        core: State::default(),
        reported_scheme: None,
        grids: HashMap::new(),
        pane_layouts: HashMap::new(),
        viewed_terminals_sent: None,
        viewed_dirty: false,
        pane_saved: HashMap::new(),
        pane_restored: Default::default(),
        pane_synced_displayed: None,
        pane_synced_project: None,
        pane_sizes: Default::default(),
        pane_sent: Default::default(),
        pane_refusal: None,
        divider_dragging: false,
        pending_drops: Vec::new(),
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
        daemon_catalogs: Default::default(),
        displaced: HashMap::new(),
        disconnected: Default::default(),
        placement: micold_client::daemon::Placement::default(),
        sandbox: micold_client::features::sandbox::Sandbox::default(),
        composer: iced::widget::text_editor::Content::new(),
        sandbox_boot: None,
        sandbox_bring_up: None,
        version_mismatch: Default::default(),
        build_mismatch: Default::default(),
        next_req: 0,
        scrollback_inflight: HashMap::new(),
        pending_ops: HashMap::new(),
        pending_daemon: HashMap::new(),
        pending_clones: HashMap::new(),
        probe: None,
        scene_ready: false,
        scene_frames: 0,
        ripples_animating: Arc::new(AtomicUsize::new(0)),
        scene_ripple_frames: std::cell::Cell::new(0),
    };

    app.ensure_test_registry();
    assert_eq!(connection_status(&app), ConnectionStatus::Connected);

    app.disconnected.insert(micold_core::daemons::DaemonId(1));
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

    app.build_mismatch.insert(
        micold_core::daemons::DaemonId(1),
        ("client-1".into(), "daemon-0".into()),
    );
    assert_eq!(
        connection_status(&app),
        ConnectionStatus::BuildMismatch {
            client_build: "client-1".into(),
            daemon_build: "daemon-0".into(),
        },
        "a same-contract build mismatch must win over a takeover"
    );

    app.version_mismatch
        .insert(micold_core::daemons::DaemonId(1), (2, 1, "daemon-0".into()));
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
        legacy: false,
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                plan_usage_enabled: true,
                plan_usage_warn_percent: 80,
                default_ai_cli: AiCli::ClaudeCode,
                default_prompt_template: String::new(),
                scrollback_lines: 10_000,
                env_include_enabled: false,
                env_include_script_path: String::new(),
                env_include_timeout_secs: 30,
                pi_activity_component: false,
                save_terminal_history: true,
                tool_server_enabled: true,
                desktop_notifications: true,
                notification_kinds: Default::default(),
                long_task_threshold_secs: 60,
                diff_layout: Default::default(),
                cross_session_access: micold_core::mcp::policy::CrossSessionAccess::Auto,
                pr_status_enabled: false,
            },
            plan_usage: None,
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
    keep_running_in_the_draft(&mut app);
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
            plan_usage: None,
        }),
    );
    drain(&mut rx);
    rx
}

/// Every directory an availability request sent since the last drain named.
fn availability_asked_for(rx: &mut SentRx) -> Vec<Option<PathBuf>> {
    sent_matching(rx, |msg| match msg {
        ClientMsg::AiCliAvailabilityRequest { cwd, .. } => Some(cwd),
        _ => None,
    })
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
    let asked = sent_matching(&mut rx, |msg| match msg {
        ClientMsg::AiCliAvailabilityRequest { req, .. } => Some(req),
        _ => None,
    });
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

/// `/repo/demo` open with the given worktrees, each `(dir, status, _)`; every worktree is listed
/// (552 FR-001), so the third element is ignored.
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

/// `/repo/demo` with one valid worktree and one whose
/// directory is gone: D = 2 rows to answer for (the root and `feat-a`).
fn app_with_mixed_rows() -> App {
    use micold_core::worktree::WorktreeStatus::{Missing, Valid};
    app_on_demo(&[("feat-a", Valid, true), ("feat-gone", Missing, true)])
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
            plan_usage: None,
        }),
    );
    rx
}

/// Every availability request sent since the last drain, as `(req, cwd)`.
fn availability_requests(rx: &mut SentRx) -> Vec<(u64, Option<PathBuf>)> {
    sent_matching(rx, |msg| match msg {
        ClientMsg::AiCliAvailabilityRequest { req, cwd } => Some((req, cwd)),
        _ => None,
    })
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

/// Whether the row for `dir` has a second CLI to choose. Claude Code alone still shows the chevron
/// for its structured entry (938 FR-022); tests/features_session.rs pins that.
fn offers_a_choice(app: &App, dir: &str) -> bool {
    app.core.session.known_clis(Some(Path::new(dir))).len() >= 2
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
/// for one whose directory is gone.
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
        jira_enabled: false,
        jira_connections: Default::default(),
        github_issues_enabled: true,
        jira_base_urls: Default::default(),
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
            save_terminal_history: settings.save_terminal_history,
        },
        environment: EnvironmentDraft {
            enabled: settings.env_include_enabled,
            script_path: settings.env_include_script_path.clone(),
            timeout_secs: settings.env_include_timeout_secs.to_string(),
            default_ai_cli: settings.default_ai_cli,
            default_prompt_template: String::new(),
            default_prompt_editor: Default::default(),
            pi_activity_component: settings.pi_activity_component,
            tool_server_enabled: settings.tool_server_enabled,
            desktop_notifications: settings.desktop_notifications,
            notification_kinds: settings.notification_kinds,
            long_task_threshold_secs: settings.long_task_threshold_secs.to_string(),
            plan_usage_enabled: settings.plan_usage_enabled,
            plan_usage_warn_percent: settings.plan_usage_warn_percent.to_string(),
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
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
        // On, as a connected service reports them by default.
        app.core.session.pi_activity_component = true;
        app.core.session.save_terminal_history = true;
        app.core.session.tool_server_enabled = true;
        app.core.session.desktop_notifications = true;
        (app, probe)
    }

    /// The settings this window holds, as a store that has them would load them.
    fn as_stored(app: &App) -> micold_core::settings::Settings {
        crate::shell::persist::window_settings(app, micold_core::settings::Settings::default())
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
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(std::env::temp_dir());
        let id = SessionId::new();

        let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));

        assert!(
            matches!(rx.try_recv(), Ok(ClientMsg::SessionStart { session, .. }) if session == id),
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

    #[test]
    fn confirming_a_blocked_start_sends_the_start_with_the_additional_flag() {
        let (mut app, _probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(std::env::temp_dir());
        let id = SessionId::new();
        app.core.session.blocked_start = Some(id);

        let _ = update_inner(&mut app, Message::Session(SessionMsg::AdditionalConfirmed));

        let sent = std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        assert!(
            sent.iter().any(|m| matches!(
                m,
                ClientMsg::SessionStart { session, allow_additional_session: true }
                    if *session == id
            )),
            "T032: confirming sends SessionStart with allow_additional_session: true"
        );
        assert_eq!(app.core.session.blocked_start, None);
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
    /// settings store `store` makes from what the window holds, and a resolver that sources
    /// nothing: a save with the feature on must not run a real shell over the shared temp
    /// directory. Settings is open and its check has landed.
    fn saving_app(
        path: &str,
        store: impl FnOnce(micold_core::settings::Settings) -> micold_core::settings::FakeSettingsStore,
    ) -> (
        App,
        Arc<FakeScriptPathProbe>,
        Arc<micold_core::settings::FakeSettingsStore>,
    ) {
        let (mut app, probe) = app_with(path, ProbeAnswer::Missing);
        let store = Arc::new(store(as_stored(&app)));
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
            saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
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

    /// U48 (review B). The form seeds its pull request switch from the stored value, so a Settings
    /// save that does not touch it keeps `pr_status_enabled` rather than writing it off (FR-030).
    #[test]
    fn a_settings_save_keeps_the_stored_pr_status_switch() {
        let path = stored_path();
        let (mut app, _probe, store) = saving_app(&path, |window| {
            micold_core::settings::FakeSettingsStore::loaded(micold_core::settings::Settings {
                pr_status_enabled: true,
                ..window
            })
        });

        save_and_check(&mut app);

        assert_eq!(
            store.saves().last().map(|s| s.pr_status_enabled),
            Some(true),
            "a save that does not touch the switch leaves it as stored"
        );
    }

    /// What one save told the connected service: the fields of its `SettingsSet`.
    #[derive(Debug, Default, Clone, PartialEq)]
    struct Told {
        scrollback_lines: Option<usize>,
        env_include_enabled: Option<bool>,
        env_include_script_path: Option<String>,
        env_include_timeout_secs: Option<u64>,
        default_ai_cli: Option<AiCli>,
        pi_activity_component: Option<bool>,
        save_terminal_history: Option<bool>,
        tool_server_enabled: Option<bool>,
        desktop_notifications: Option<bool>,
        cross_session_access: Option<micold_core::mcp::policy::CrossSessionAccess>,
        pr_status_enabled: Option<bool>,
        notification_kinds: Option<micold_core::attention::NotificationKinds>,
        long_task_threshold_secs: Option<u64>,
    }

    /// A [`saving_app`] over `store`, connected to a service whose messages land in the receiver.
    fn connected_saving_app(
        path: &str,
        store: impl FnOnce(micold_core::settings::Settings) -> micold_core::settings::FakeSettingsStore,
    ) -> (
        App,
        Arc<micold_core::settings::FakeSettingsStore>,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (mut app, _probe, store) = saving_app(path, store);
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        (app, store, rx)
    }

    /// Open Settings, edit the draft with `edit`, save, and return the `SettingsSet` the save
    /// sent, or `None` when it sent none.
    fn save_edited(
        app: &mut App,
        rx: &mut SentRx,
        edit: impl FnOnce(&mut SettingsDraft),
    ) -> Option<Told> {
        let _ = open_and_check(app);
        drain(rx);
        edit(
            app.core
                .settings
                .settings_draft
                .as_mut()
                .expect("Settings is open"),
        );
        save_and_check(app);
        drain(rx).into_iter().find_map(|msg| match msg {
            ClientMsg::SettingsSet {
                scrollback_lines,
                env_include_enabled,
                env_include_script_path,
                env_include_timeout_secs,
                default_ai_cli,
                pi_activity_component,
                save_terminal_history,
                tool_server_enabled,
                desktop_notifications,
                cross_session_access,
                pr_status_enabled,
                notification_kinds,
                long_task_threshold_secs,
                ..
            } => Some(Told {
                scrollback_lines,
                env_include_enabled,
                env_include_script_path,
                env_include_timeout_secs,
                default_ai_cli,
                pi_activity_component,
                save_terminal_history,
                tool_server_enabled,
                desktop_notifications,
                cross_session_access,
                pr_status_enabled,
                notification_kinds,
                long_task_threshold_secs,
            }),
            _ => None,
        })
    }

    /// One edit of the open draft.
    type Edit = dyn FnOnce(&mut SettingsDraft);

    /// U182 (BUG-570, FR-026a): a save tells the service each service-owned field the user
    /// changed and `None` for every other; a save that changed none of them sends nothing.
    #[test]
    fn a_save_sends_some_only_for_the_service_owned_fields_the_user_changed() {
        use micold_core::mcp::policy::CrossSessionAccess;
        let path = stored_path();
        let (mut app, _store, mut rx) =
            connected_saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
        let cases: Vec<(&str, Box<Edit>, Told)> = vec![
            (
                "scrollback",
                Box::new(|d| d.terminal.scrollback_lines = "777".into()),
                Told {
                    scrollback_lines: Some(777),
                    ..Told::default()
                },
            ),
            (
                "environment include on",
                Box::new(|d| d.environment.enabled = true),
                Told {
                    env_include_enabled: Some(true),
                    ..Told::default()
                },
            ),
            (
                "script path",
                Box::new(|d| d.environment.script_path = "/elsewhere/env.sh".into()),
                Told {
                    env_include_script_path: Some("/elsewhere/env.sh".into()),
                    ..Told::default()
                },
            ),
            (
                "timeout",
                Box::new(|d| d.environment.timeout_secs = "17".into()),
                Told {
                    env_include_timeout_secs: Some(17),
                    ..Told::default()
                },
            ),
            (
                "default AI CLI",
                Box::new(|d| d.environment.default_ai_cli = AiCli::Copilot),
                Told {
                    default_ai_cli: Some(AiCli::Copilot),
                    ..Told::default()
                },
            ),
            (
                "Pi activity component",
                Box::new(|d| d.environment.pi_activity_component = false),
                Told {
                    pi_activity_component: Some(false),
                    ..Told::default()
                },
            ),
            (
                "save terminal history",
                Box::new(|d| d.terminal.save_terminal_history = false),
                Told {
                    save_terminal_history: Some(false),
                    ..Told::default()
                },
            ),
            (
                "tool server",
                Box::new(|d| d.environment.tool_server_enabled = false),
                Told {
                    tool_server_enabled: Some(false),
                    ..Told::default()
                },
            ),
            (
                "desktop notifications",
                Box::new(|d| d.environment.desktop_notifications = false),
                Told {
                    desktop_notifications: Some(false),
                    ..Told::default()
                },
            ),
            (
                "cross-session access",
                Box::new(|d| d.environment.cross_session_access = CrossSessionAccess::Off),
                Told {
                    cross_session_access: Some(CrossSessionAccess::Off),
                    ..Told::default()
                },
            ),
            (
                "notification kinds",
                Box::new(|d| d.environment.notification_kinds.session_error = false),
                Told {
                    notification_kinds: Some(micold_core::attention::NotificationKinds {
                        session_error: false,
                        ..micold_core::attention::NotificationKinds::default()
                    }),
                    ..Told::default()
                },
            ),
            (
                "long-task threshold",
                Box::new(|d| d.environment.long_task_threshold_secs = "321".into()),
                Told {
                    long_task_threshold_secs: Some(321),
                    ..Told::default()
                },
            ),
        ];

        for (field, edit, expected) in cases {
            assert_eq!(
                save_edited(&mut app, &mut rx, edit),
                Some(expected),
                "a save that changed only the {field}"
            );
        }
        assert_eq!(
            save_edited(&mut app, &mut rx, |d| {
                d.appearance.theme = micold_core::theme::ThemePreference::Dark
            }),
            None,
            "a save that changed only the theme tells the service nothing"
        );
    }

    /// U183 (BUG-570, FR-026a): a save sets only the fields the user changed on the stored
    /// document, so a theme and sandbox settings another window stored meanwhile are kept.
    #[test]
    fn a_save_keeps_stored_fields_the_user_did_not_change() {
        let path = stored_path();
        let (mut app, store, mut rx) =
            connected_saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
        let _ = open_and_check(&mut app);

        // Another window's save, while this page is open.
        let mut elsewhere = micold_core::settings::SettingsStore::load(&*store).settings;
        elsewhere.theme = micold_core::theme::ThemePreference::Dark;
        elsewhere.daemon.sandbox.survive_logout = !elsewhere.daemon.sandbox.survive_logout;
        elsewhere.daemon.sandbox.budget.pids = Some(321);
        micold_core::settings::SettingsStore::save(&*store, &elsewhere).expect("saves");

        let draft = app
            .core
            .settings
            .settings_draft
            .as_mut()
            .expect("Settings is open");
        draft.terminal.scrollback_lines = "4321".into();
        // Another limit of the same sandbox profile (review A, F1).
        draft.daemon.memory_mib = "1234".into();
        save_and_check(&mut app);
        drain(&mut rx);

        let saved = store.saves().last().cloned().expect("the save is written");
        let mut expected = micold_core::settings::Settings {
            scrollback_lines: 4321,
            ..elsewhere.clone()
        };
        expected.daemon.sandbox.budget.memory_bytes = saved.daemon.sandbox.budget.memory_bytes;
        assert_ne!(
            saved.daemon.sandbox.budget.memory_bytes, elsewhere.daemon.sandbox.budget.memory_bytes,
            "the new memory limit is written"
        );
        assert_eq!(
            saved, expected,
            "the file gets the new scrollback and memory limit and keeps everything else as stored"
        );
    }

    /// U184 (BUG-570, FR-026a): a field retyped to the value the page opened with is unchanged.
    #[test]
    fn a_field_retyped_to_its_opening_value_counts_as_unchanged() {
        let path = stored_path();
        let (mut app, _store, mut rx) =
            connected_saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
        let opened_with = app.scrollback_lines.to_string();

        let told = save_edited(&mut app, &mut rx, |d| {
            d.terminal.scrollback_lines = format!(" {opened_with}");
            d.environment.desktop_notifications = false;
        });

        assert_eq!(
            told,
            Some(Told {
                desktop_notifications: Some(false),
                ..Told::default()
            }),
            "the scrollback was retyped, not changed, so the service is not told it"
        );
    }

    #[test]
    fn a_save_checks_the_saved_path_even_when_the_path_did_not_change() {
        let path = stored_path();
        let (mut app, probe, _store) =
            saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
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

    /// Carry the stored daemon registry onto a drafted `Settings` (feature 491): the settings form
    /// does not hold it, and a save keeps what is stored.
    fn keep_registry(
        drafted: &mut micold_core::settings::Settings,
        store: &micold_core::settings::FakeSettingsStore,
    ) {
        let stored = micold_core::settings::SettingsStore::load(store).settings;
        drafted.daemons = stored.daemons;
        drafted.next_daemon_id = stored.next_daemon_id;
        drafted.legacy_default_daemon = stored.legacy_default_daemon;
    }

    #[test]
    fn a_save_with_a_missing_path_writes_exactly_the_drafted_settings_and_nothing_of_the_check() {
        let path = stored_path();
        let (mut app, _probe, store) =
            saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
        let mut expected = valid_draft(&app).into_settings();
        // The form does not hold the registry (feature 491); the stored one is kept.
        keep_registry(&mut expected, &store);

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
        let (mut app, _probe, _store) = saving_app(&path, |window| {
            micold_core::settings::FakeSettingsStore::loaded(window)
                .failing_save(std::io::ErrorKind::PermissionDenied)
        });

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
            jira_enabled: false,
            jira_connections: Default::default(),
            github_issues_enabled: true,
            jira_base_urls: Default::default(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: 80,
            default_ai_cli: AiCli::ClaudeCode,
            default_prompt_template: String::new(),
            scrollback_lines: 5_000,
            env_include_enabled: true,
            env_include_script_path: path.to_string(),
            env_include_timeout_secs: 10,
            pi_activity_component: false,
            save_terminal_history: true,
            tool_server_enabled: true,
            desktop_notifications: true,
            notification_kinds: Default::default(),
            long_task_threshold_secs: 60,
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
    fn a_reconnect_rechecks_the_stored_path_while_settings_is_open() {
        let (mut app, _probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));
        let _ = open_and_check(&mut app);
        let new_path = std::env::temp_dir()
            .join("changed-while-away.sh")
            .to_str()
            .expect("utf-8 temp dir")
            .to_string();

        let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
        let _ = crate::shell::daemon_sync::on_connected(
            &mut app,
            micold_core::daemons::DaemonId(1),
            micold_client::daemon::Outbox::new(tx),
            micold_core::protocol::messages::CatalogSnapshot::default(),
            settings_saved_elsewhere(&new_path),
            None,
        );

        assert!(
            matches!(
                &app.core.settings.script_check,
                micold_client::features::settings::ScriptCheck::Pending { .. }
            ),
            "the reconnect re-checks the path it now holds, got {:?}",
            app.core.settings.script_check
        );
    }

    #[test]
    fn a_reconnect_checks_nothing_while_settings_is_closed() {
        let (mut app, probe) = app_with(&stored_path(), ProbeAnswer::Missing);
        app.caps = app
            .caps
            .clone()
            .with_env_include(Arc::new(FakeEnvIncludeResolver::default()));

        let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
        let _ = crate::shell::daemon_sync::on_connected(
            &mut app,
            micold_core::daemons::DaemonId(1),
            micold_client::daemon::Outbox::new(tx),
            micold_core::protocol::messages::CatalogSnapshot::default(),
            settings_saved_elsewhere(&stored_path()),
            None,
        );

        assert_eq!(
            app.core.settings.script_check,
            micold_client::features::settings::ScriptCheck::Idle,
            "no page, no check (FR-006)"
        );
        assert!(probe.calls().is_empty());
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
            saving_app(path, micold_core::settings::FakeSettingsStore::loaded);
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
        app.env_include_timeout_secs = 7;
        let store = Arc::new(micold_core::settings::FakeSettingsStore::loaded(as_stored(
            &app,
        )));
        app.caps = app.caps.clone().with_settings(store.clone());
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
        let mut draft = valid_draft(&app).into_settings();
        keep_registry(&mut draft, &store);
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
            saving_app(&path, micold_core::settings::FakeSettingsStore::loaded);
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
            micold_core::daemons::DaemonId(1),
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
        DescriptionPage, FakeIssueSource, Issue, IssueLoadError, IssuePage, IssueSnapshot,
        IssueSource,
    };
    use micold_core::naming::ConventionalType;
    use micold_core::protocol::messages::{ErrorKind, OperationResult};
    use micold_core::typeahead::Direction;
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
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(PathBuf::from(PROJECT));
        IssueRig {
            app,
            rx,
            located_with,
            constructed,
            source,
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
    fn remote_lists_sent(rx: &mut SentRx) -> Vec<(u64, PathBuf)> {
        sent_matching(rx, |msg| match msg {
            ClientMsg::RemoteList { req, project } => Some((req, project)),
            _ => None,
        })
    }

    fn github_remote() -> Vec<GitRemote> {
        vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/r.git".into(),
        }]
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

    // --- 1102: the GitHub issues switch off (FR-002, FR-009, SC-001) -----------------------------

    /// With the switch off, opening the form and trying every named event reaches neither `gh`
    /// lookup nor a source: no issue list, search, description or details request is made. The
    /// remotes are still asked for (FR-009).
    #[test]
    fn with_the_switch_off_no_issue_request_is_ever_made() {
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_page(page(issues())),
        );
        rig.app.core.settings.github_issues_enabled = false;
        send(&mut rig.app, FormMsg::Opened);
        let sent = remote_lists_sent(&mut rig.rx);
        assert_eq!(sent.len(), 1, "the remotes are still read (FR-009)");
        answer_remotes(&mut rig.app, sent[0].0, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        send(&mut rig.app, FormMsg::IssueRetry);
        send(&mut rig.app, FormMsg::IssueQueryChanged("flick".into()));
        assert!(form(&rig.app).github_repo().is_none());
        assert_ne!(form(&rig.app).source, BranchSource::Issue);
        assert!(rig.located_with.lock().unwrap().is_empty(), "no gh lookup");
        assert!(rig.constructed.lock().unwrap().is_empty(), "no gh source");
        // The search, descriptions and details starters are built from these requests, which the
        // reducer only forms from a known repository.
        let held = &rig.app.core.worktree_form;
        assert!(held.issue_search_request().is_none());
        assert!(held.issue_description_request().is_none());
        assert!(held.issue_details_request().is_none());
    }

    /// With the switch on, the same steps do load (the default is unchanged: US2 AS2).
    #[test]
    fn with_the_switch_on_the_issue_source_still_loads() {
        let rig = loaded_rig();
        assert!(!rig.constructed.lock().unwrap().is_empty());
        assert!(!form(&rig.app).issue_matches.is_empty());
    }

    // --- 1102: the Overview's issue titles are GitHub issue reads, so the switch gates them -----

    /// The rig with the Overview's project holding one worktree of ticket 42.
    fn titles_rig(enabled: bool) -> IssueRig {
        use micold_core::worktree::{Worktree, WorktreeStatus};
        let mut rig = issue_rig(
            Some(FAKE_GH),
            FakeIssueSource::new().with_lookup(Ok(IssueSnapshot {
                number: 42,
                title: "Crash when opening empty project".into(),
                labels: vec![],
            })),
        );
        rig.app.core.settings.github_issues_enabled = enabled;
        rig.app.core.worktree.worktrees = vec![Worktree {
            dir_name: "feat-42_crash".into(),
            path: PathBuf::from("/repo/demo/.claude/worktrees/feat-42_crash"),
            branch: Some("feat/42_crash".into()),
            status: WorktreeStatus::Valid,
            included: false,
        }];
        rig
    }

    /// Off: opening the Overview reads no title and records none, not even "unavailable", so
    /// turning the switch on reads them next time. The remotes are still asked for once, for the
    /// issue links' repository (feature 1122, R2), and their answer reads no title.
    #[test]
    fn with_the_switch_off_opening_the_overview_reads_no_title() {
        let mut rig = titles_rig(false);
        let work = crate::shell::issues::on_overview_opened(&mut rig.app);
        settle(&mut rig.app, work);
        let sent = remote_lists_sent(&mut rig.rx);
        assert_eq!(sent.len(), 1, "the repository is asked for");
        answer_remotes(&mut rig.app, sent[0].0, github_remote());
        assert!(rig.app.core.overview.titles.is_empty(), "nothing recorded");
        assert!(rig.located_with.lock().unwrap().is_empty(), "no gh lookup");
        assert!(rig.source.lookup_calls().is_empty(), "no title read");
    }

    /// Off: a remotes answer arriving after the switch went off reads nothing and settles the
    /// asked titles as unavailable, without locating `gh`.
    #[test]
    fn with_the_switch_off_the_remotes_answer_reads_no_title() {
        let mut rig = titles_rig(false);
        rig.app
            .core
            .overview
            .titles
            .insert(42, micold_client::features::overview::TitleLookup::Pending);
        let work = crate::shell::issues::on_title_remotes(
            &mut rig.app,
            PathBuf::from(PROJECT),
            vec![42],
            Some(github_remote()),
        );
        settle(&mut rig.app, work);
        assert!(
            micold_client::features::overview::TitleLookup::Unavailable
                == rig.app.core.overview.titles[&42]
        );
        assert!(rig.located_with.lock().unwrap().is_empty(), "no gh lookup");
        assert!(rig.constructed.lock().unwrap().is_empty(), "no gh source");
        assert!(rig.source.lookup_calls().is_empty(), "no title read");
    }

    /// On: the same steps read the title (the default is unchanged).
    #[test]
    fn with_the_switch_on_the_overview_still_reads_its_titles() {
        let mut rig = titles_rig(true);
        let work = crate::shell::issues::on_overview_opened(&mut rig.app);
        settle(&mut rig.app, work);
        let sent = remote_lists_sent(&mut rig.rx);
        assert_eq!(sent.len(), 1, "the remotes are asked for");
        answer_remotes(&mut rig.app, sent[0].0, github_remote());
        assert_eq!(rig.source.lookup_calls(), vec![("o/r".to_string(), 42)]);
        assert!(
            micold_client::features::overview::TitleLookup::Known(
                "Crash when opening empty project".into()
            ) == rig.app.core.overview.titles[&42]
        );
    }

    // --- 1122: the issue links' repository (R2) and opening a link (R10) -------------------------

    fn repo_named(url: &str) -> micold_client::features::overview::RepoLookup {
        micold_client::features::overview::RepoLookup::Known(
            micold_core::github::GithubRepo::from_remote_url(url),
        )
    }

    /// R2: opening the Overview with the repository unknown asks for the remotes even with the
    /// GitHub issues switch off, and the answer records the repository before the switch's gate.
    #[test]
    fn repo_read_on_overview_open() {
        let mut rig = titles_rig(false);
        rig.app.core.overview.open = true;
        let work = crate::shell::issues::on_overview_opened(&mut rig.app);
        settle(&mut rig.app, work);
        let sent = remote_lists_sent(&mut rig.rx);
        assert_eq!(sent.len(), 1, "the repository is asked for once");
        assert_eq!(
            rig.app.core.overview.repo,
            micold_client::features::overview::RepoLookup::Asked
        );
        let again = crate::shell::issues::on_overview_opened(&mut rig.app);
        settle(&mut rig.app, again);
        assert!(remote_lists_sent(&mut rig.rx).is_empty(), "asked only once");
        answer_remotes(&mut rig.app, sent[0].0, github_remote());
        assert_eq!(
            rig.app.core.overview.repo,
            repo_named("git@github.com:o/r.git")
        );
        assert!(rig.source.lookup_calls().is_empty(), "no title read");
    }

    /// R2, U6: a project switch with the Overview open forgets the repository; the new project's
    /// `Attached` reads it, and its cards then link to the new repository.
    #[test]
    fn repo_reread_on_attach() {
        repo_reread_after(|project| DaemonMsg::Attached {
            project,
            sessions: vec![],
        });
    }

    /// R2, U6 (visual pass M5 r1): a switch in a read-only window ends in `Refused` (another
    /// window holds the project), not `Attached`; that answer reads the repository all the same.
    #[test]
    fn repo_reread_on_a_read_only_switch() {
        repo_reread_after(|project| DaemonMsg::Refused {
            reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                project,
                holder: super::other_window(),
                since_secs: 12,
            },
        });
    }

    /// Switch the rig's window, its Overview open on a known repository, to another project; the
    /// daemon answers the attach with `answer`, which must read the new project's repository.
    fn repo_reread_after(answer: fn(PathBuf) -> DaemonMsg) {
        let other = PathBuf::from("/repo/other");
        let mut rig = titles_rig(true);
        rig.app.core.overview.open = true;
        let work = crate::shell::issues::on_overview_opened(&mut rig.app);
        settle(&mut rig.app, work);
        for (req, _) in remote_lists_sent(&mut rig.rx) {
            answer_remotes(&mut rig.app, req, github_remote());
        }
        assert_eq!(
            rig.app.core.overview.repo,
            repo_named("git@github.com:o/r.git")
        );
        crate::shell::daemon_sync::switch_daemon_attachment(
            &mut rig.app,
            Some((
                PathBuf::from(PROJECT),
                micold_core::daemons::Binding::Bound(micold_core::daemons::DaemonId(1)),
            )),
            &other,
        );
        rig.app.core.workspace.active = Some(other.clone());
        assert_eq!(
            rig.app.core.overview.repo,
            micold_client::features::overview::RepoLookup::Unknown
        );
        let _ = drain(&mut rig.rx);
        let work = crate::shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            micold_core::daemons::DaemonId(1),
            answer(other.clone()),
        );
        settle(&mut rig.app, work);
        let sent = remote_lists_sent(&mut rig.rx);
        let read = sent
            .iter()
            .find(|(_, project)| *project == other)
            .expect("the attach's answer reads the new project's repository");
        let new_remote = vec![GitRemote {
            name: "origin".into(),
            url: "git@github.com:o/s.git".into(),
        }];
        answer_remotes(&mut rig.app, read.0, new_remote);
        let link = micold_client::ui::overview::issue_link_for(
            &rig.app.core,
            &micold_core::session::SessionLocation::Worktree("feat-42_crash".into()),
        )
        .expect("the new project's card links to its issue");
        assert_eq!(link.address, "https://github.com/o/s/issues/42");
    }

    /// R10, FR-004: a click on a card's issue link hands the address recomputed from state to the
    /// link opener; with the link gone (the switch off) nothing is opened.
    #[test]
    fn issue_link_activated_opens_the_address() {
        let opener = Arc::new(super::pr_status_open::Recording::default());
        let mut rig = titles_rig(true);
        rig.app.caps = rig.app.caps.clone().with_link_opener(opener.clone());
        rig.app.core.overview.open = true;
        rig.app.core.overview.repo = repo_named("git@github.com:o/r.git");
        let location = micold_core::session::SessionLocation::Worktree("feat-42_crash".into());
        let click = |app: &mut App| {
            let work = update_inner(
                app,
                Message::Overview(micold_client::features::overview::Msg::IssueLinkActivated(
                    location.clone(),
                )),
            );
            settle(app, work);
        };
        click(&mut rig.app);
        assert_eq!(
            opener.opened.lock().unwrap().clone(),
            vec!["https://github.com/o/r/issues/42".to_string()]
        );
        assert!(rig.app.core.overview.open, "the Overview stays open");
        rig.app.core.settings.github_issues_enabled = false;
        click(&mut rig.app);
        assert_eq!(opener.opened.lock().unwrap().len(), 1, "the link is gone");
    }

    /// FR-013: a browser that cannot be opened from an issue link gives feature 031's notification,
    /// naming the address and the opener's reason, and the Overview stays as it was.
    #[test]
    fn issue_link_whose_browser_fails_to_open_notifies_the_address_and_reason() {
        let opener = Arc::new(super::pr_status_open::Recording {
            fail: true,
            ..Default::default()
        });
        let mut rig = titles_rig(true);
        rig.app.caps = rig.app.caps.clone().with_link_opener(opener.clone());
        rig.app.core.overview.open = true;
        rig.app.core.overview.repo = repo_named("git@github.com:o/r.git");
        let before = rig.app.core.overview.clone();
        let work = update_inner(
            &mut rig.app,
            Message::Overview(micold_client::features::overview::Msg::IssueLinkActivated(
                micold_core::session::SessionLocation::Worktree("feat-42_crash".into()),
            )),
        );
        settle(&mut rig.app, work);
        let shown = rig
            .app
            .core
            .notifications
            .queue
            .visible()
            .map(|n| n.message.clone());
        assert_eq!(
            shown.as_deref(),
            Some("Couldn't open https://github.com/o/r/issues/42: no launcher")
        );
        assert_eq!(
            rig.app.core.overview, before,
            "nothing else of the Overview changes"
        );
    }

    // --- 1123: a pull request row's number opens it (U7, R8) -----------------------------------

    /// `titles_rig`'s window with `numbers` listed open on its worktree's branch, the Overview
    /// open, and `opener` as the link opener.
    fn pr_link_rig(numbers: &[u64], opener: &Arc<super::pr_status_open::Recording>) -> IssueRig {
        let mut rig = titles_rig(false);
        rig.app.caps = rig.app.caps.clone().with_link_opener(opener.clone());
        rig.app.core.overview.open = true;
        read_numbers(&mut rig.app, numbers);
        rig
    }

    /// A successful reading listing `numbers` open on `feat/42_crash`.
    fn read_numbers(app: &mut App, numbers: &[u64]) {
        super::list_numbers(app, "feat/42_crash", numbers);
    }

    fn click_number(app: &mut App, location: &micold_core::session::SessionLocation, n: u64) {
        let work = update_inner(
            app,
            Message::Overview(
                micold_client::features::overview::Msg::PullRequestLinkActivated(
                    location.clone(),
                    n,
                ),
            ),
        );
        settle(app, work);
    }

    fn crash() -> micold_core::session::SessionLocation {
        micold_core::session::SessionLocation::Worktree("feat-42_crash".into())
    }

    /// U7, FR-009: a click on "#12" asks the link opener for the address the reading gave #12;
    /// the Overview stays open.
    #[test]
    fn pull_request_link_opens_the_listed_address() {
        let opener = Arc::new(super::pr_status_open::Recording::default());
        let mut rig = pr_link_rig(&[12, 11], &opener);
        click_number(&mut rig.app, &crash(), 12);
        assert_eq!(
            opener.opened.lock().unwrap().clone(),
            vec!["https://github.com/o/r/pull/12".to_string()]
        );
        assert!(rig.app.core.overview.open, "the Overview stays open");
    }

    /// R8: after a reading without #12, or for a card no longer there, a click opens nothing.
    #[test]
    fn pull_request_link_opens_nothing_once_gone() {
        let opener = Arc::new(super::pr_status_open::Recording::default());
        let mut rig = pr_link_rig(&[12], &opener);
        read_numbers(&mut rig.app, &[11]);
        click_number(&mut rig.app, &crash(), 12);
        let gone = micold_core::session::SessionLocation::Worktree("feat-43_gone".into());
        click_number(&mut rig.app, &gone, 11);
        assert!(opener.opened.lock().unwrap().is_empty());
    }

    /// FR-014 (review A M2 r1 F1): a listed address that is no GitHub page is never handed to the
    /// browser, as for the sidebar's and the changes pane's pull request links.
    #[test]
    fn pull_request_link_opens_no_address_off_github() {
        let opener = Arc::new(super::pr_status_open::Recording::default());
        let mut rig = pr_link_rig(&[12], &opener);
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "https://example.com/pull/12",
        ] {
            if let Some(list) = rig.app.core.pr_status.lists.get_mut("feat/42_crash") {
                let mut prs = list.prs().to_vec();
                prs[0].url = url.into();
                *list = micold_core::pr_list::BranchPrList::new(prs, false, false);
            }
            click_number(&mut rig.app, &crash(), 12);
        }
        assert!(opener.opened.lock().unwrap().is_empty());
    }

    /// FR-012, Story 3 sc. 5: a browser that cannot be opened gives feature 031's notification,
    /// naming the address and the reason, and nothing of the Overview changes.
    #[test]
    fn pull_request_link_failure_notifies() {
        let opener = Arc::new(super::pr_status_open::Recording {
            fail: true,
            ..Default::default()
        });
        let mut rig = pr_link_rig(&[12], &opener);
        let before = rig.app.core.overview.clone();
        click_number(&mut rig.app, &crash(), 12);
        let shown = rig
            .app
            .core
            .notifications
            .queue
            .visible()
            .map(|n| n.message.clone());
        assert_eq!(
            shown.as_deref(),
            Some("Couldn't open https://github.com/o/r/pull/12: no launcher")
        );
        assert_eq!(rig.app.core.overview, before, "nothing else changes");
    }

    // --- 837: the picked issue's details (US2, US3) ---------------------------------------------

    use micold_client::features::worktree_form::IssueDetailsState;
    use micold_core::issue_details::{
        Blockers, IssueDetails, Kind, PlainText, Reference, RelatedIssue, RelatedList,
        DETAILS_BOUND,
    };

    /// Details with every kind read and empty.
    fn nothing(number: u64) -> IssueDetails {
        IssueDetails {
            number,
            blocked_by: Kind::Read(Blockers {
                list: RelatedList::new(Vec::new(), 0, 0),
                open: 0,
            }),
            blocking: Kind::Read(RelatedList::new(Vec::new(), 0, 0)),
            parent: Kind::Read(None),
            sub_issues: Kind::Read(micold_core::issue_details::SubIssues {
                list: RelatedList::new(Vec::new(), 0, 0),
                closed: 0,
            }),
            labels: Kind::Read(RelatedList::new(Vec::new(), 0, 0)),
            assignees: Kind::Read(RelatedList::new(Vec::new(), 0, 0)),
            milestone: Kind::Read(None),
            projects: Kind::Read(RelatedList::new(Vec::new(), 0, 0)),
            closing_prs: Kind::Read(RelatedList::new(Vec::new(), 0, 0)),
            linked_prs: Kind::Read(RelatedList::new(Vec::new(), 0, 0)),
        }
    }

    /// Details of `number`, blocked by one open issue (#10).
    fn blocked(number: u64) -> IssueDetails {
        IssueDetails {
            blocked_by: Kind::Read(Blockers {
                list: RelatedList::new(
                    vec![RelatedIssue {
                        reference: Reference::Here(10),
                        title: PlainText::new("Parser rewrite"),
                        open: true,
                    }],
                    1,
                    0,
                ),
                open: 1,
            }),
            ..nothing(number)
        }
    }

    /// Pick `number` the way the shell hands the pick to the reducer, and leave the work it returns
    /// for the test to run (or not): the read is then still awaited.
    fn pick_without_settling(app: &mut App, number: u64) -> Task<Message> {
        update_inner(
            app,
            Message::WorktreeForm(FormMsg::IssuePicked {
                number,
                mapping: vec![],
            }),
        )
    }

    fn details_of(app: &App) -> &IssueDetailsState {
        &form(app).issue_details
    }

    fn loaded_details(rig: &IssueRig, number: u64, details: IssueDetails) -> FormMsg {
        let IssueDetailsState::Loading { seq, .. } = details_of(&rig.app).clone() else {
            panic!("the form awaits no details: {:?}", details_of(&rig.app));
        };
        FormMsg::IssueDetailsLoaded {
            seq,
            number,
            result: Ok(Box::new(details)),
        }
    }

    /// T020 — picking a GitHub issue moves the details to `Loading` and starts exactly one read
    /// with the list's `gh`, the repository and the number; its answer moves them to `Loaded`
    /// (US2-1, FR-001).
    #[test]
    fn details_a_pick_starts_one_read_and_its_answer_is_shown() {
        let source = FakeIssueSource::new()
            .with_page(page(issues()))
            .with_details(Ok(blocked(42)));
        let mut rig = issue_rig(Some(FAKE_GH), source);
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert_eq!(*details_of(&rig.app), IssueDetailsState::Idle);

        let work = pick_without_settling(&mut rig.app, 42);
        assert!(
            matches!(
                details_of(&rig.app),
                IssueDetailsState::Loading { number: 42, .. }
            ),
            "{:?}",
            details_of(&rig.app)
        );
        settle(&mut rig.app, work);
        assert_eq!(
            rig.source.details_calls(),
            [("o/r".to_string(), 42)],
            "exactly one read, for the picked number"
        );
        assert_eq!(
            rig.constructed.lock().unwrap().last(),
            Some(&PathBuf::from(FAKE_GH)),
            "the read runs the gh the list was read with"
        );
        assert!(
            matches!(
                details_of(&rig.app),
                IssueDetailsState::Loaded { number: 42, details } if details.is_blocked()
            ),
            "{:?}",
            details_of(&rig.app)
        );
    }

    /// T020 — the read is bound: its deadline is `DETAILS_BOUND` after the pick (FR-006, SC-005).
    #[test]
    fn details_the_read_is_given_the_details_bound() {
        let mut rig = loaded_rig();
        let before = std::time::Instant::now();
        let work = pick_without_settling(&mut rig.app, 42);
        settle(&mut rig.app, work);
        let after = std::time::Instant::now();
        let deadlines = rig.source.details_deadlines();
        assert_eq!(deadlines.len(), 1);
        assert!(
            deadlines[0] >= before + DETAILS_BOUND && deadlines[0] <= after + DETAILS_BOUND,
            "the deadline is DETAILS_BOUND after the pick"
        );
    }

    /// T020 — an answer for an earlier pick, or for another number, is dropped after a re-pick
    /// (US2-5, FR-012).
    #[test]
    fn details_an_answer_for_an_earlier_pick_is_dropped() {
        let mut rig = loaded_rig();
        let _first = pick_without_settling(&mut rig.app, 42);
        let IssueDetailsState::Loading { seq: first, .. } = details_of(&rig.app).clone() else {
            panic!("loading");
        };
        let _second = pick_without_settling(&mut rig.app, 7);
        let awaited = details_of(&rig.app).clone();
        assert!(
            matches!(awaited, IssueDetailsState::Loading { number: 7, seq } if seq > first),
            "{awaited:?}"
        );
        send(
            &mut rig.app,
            FormMsg::IssueDetailsLoaded {
                seq: first,
                number: 42,
                result: Ok(Box::new(blocked(42))),
            },
        );
        assert_eq!(*details_of(&rig.app), awaited, "the older seq is dropped");
        let IssueDetailsState::Loading { seq, .. } = awaited.clone() else {
            panic!("loading");
        };
        send(
            &mut rig.app,
            FormMsg::IssueDetailsLoaded {
                seq,
                number: 42,
                result: Ok(Box::new(blocked(42))),
            },
        );
        assert_eq!(*details_of(&rig.app), awaited, "another number is dropped");
        let msg = loaded_details(&rig, 7, nothing(7));
        send(&mut rig.app, msg);
        assert!(matches!(
            details_of(&rig.app),
            IssueDetailsState::Loaded { number: 7, .. }
        ));
    }

    /// T020 — leaving the issue source clears the pick: the state goes `Idle` and a late answer is
    /// dropped (FR-012).
    #[test]
    fn details_a_cleared_pick_drops_the_late_answer() {
        let mut rig = loaded_rig();
        let _work = pick_without_settling(&mut rig.app, 42);
        let IssueDetailsState::Loading { seq, .. } = details_of(&rig.app).clone() else {
            panic!("loading");
        };
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::New));
        assert_eq!(*details_of(&rig.app), IssueDetailsState::Idle);
        send(
            &mut rig.app,
            FormMsg::IssueDetailsLoaded {
                seq,
                number: 42,
                result: Ok(Box::new(blocked(42))),
            },
        );
        assert_eq!(*details_of(&rig.app), IssueDetailsState::Idle);
    }

    /// T020 — an answer that arrives after the form closed is dropped, with no form to land on.
    #[test]
    fn details_a_closed_form_drops_the_answer() {
        let mut rig = loaded_rig();
        let _work = pick_without_settling(&mut rig.app, 42);
        let IssueDetailsState::Loading { seq, .. } = details_of(&rig.app).clone() else {
            panic!("loading");
        };
        send(&mut rig.app, FormMsg::Cancelled);
        send(
            &mut rig.app,
            FormMsg::IssueDetailsLoaded {
                seq,
                number: 42,
                result: Ok(Box::new(blocked(42))),
            },
        );
        assert!(rig.app.core.worktree_form.form.is_none());
    }

    /// T020 — Create while the details load submits at once, with what it submits without them
    /// (US2-4, FR-006).
    #[test]
    fn details_create_does_not_wait_for_them() {
        let mut rig = loaded_rig();
        let _work = pick_without_settling(&mut rig.app, 42);
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Fix));
        let _ = remote_lists_sent(&mut rig.rx);
        send(&mut rig.app, FormMsg::Submitted);
        assert_eq!(
            preflights_sent(&mut rig.rx),
            vec!["fix/42_crash-when-opening-empty-project".to_string()],
            "the same request as issue_submit_creates_like_a_new_branch"
        );
        assert!(
            matches!(details_of(&rig.app), IssueDetailsState::Loading { .. }),
            "the read is still out"
        );
    }

    /// T020 — the New and Existing sources start no read (SC-006, FR-010).
    #[test]
    fn details_other_sources_start_no_read() {
        let mut rig = loaded_rig();
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::New));
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Existing));
        send(&mut rig.app, FormMsg::TicketChanged("42".into()));
        assert!(rig.source.details_calls().is_empty());
        assert_eq!(*details_of(&rig.app), IssueDetailsState::Idle);
    }

    /// T020 — a Jira pick starts no read and a picked number above `u32::MAX` has no request
    /// (SC-006, form-details §2).
    #[test]
    fn details_a_jira_pick_or_a_huge_number_has_no_request() {
        let mut rig = loaded_rig();
        let state = &mut rig.app.core.worktree_form;
        let form = state.form.as_mut().expect("the form is open");
        form.set_picked_issue(Some(u64::from(u32::MAX) + 1));
        assert!(
            matches!(form.issue_details, IssueDetailsState::Loading { .. }),
            "the state still follows the pick"
        );
        assert_eq!(state.issue_details_request(), None, "no number to ask for");

        let state = &mut rig.app.core.worktree_form;
        let form = state.form.as_mut().expect("the form is open");
        form.provider = micold_core::jira::IssueProvider::Jira;
        form.set_picked_issue(Some(42));
        assert_eq!(form.issue_details, IssueDetailsState::Idle);
        assert_eq!(state.issue_details_request(), None);
        assert!(rig.source.details_calls().is_empty());
    }

    /// T028 / US3-3 — a failed read is a `Failed` state with the reason's sentence, and Create
    /// works as without it (FR-006, FR-007).
    #[test]
    fn details_a_failed_read_is_one_sentence_and_create_still_works() {
        let source = FakeIssueSource::new()
            .with_page(page(issues()))
            .with_details(Err(IssueLoadError::NotSignedIn));
        let mut rig = issue_rig(Some(FAKE_GH), source);
        open_with(&mut rig, github_remote());
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        let work = pick_without_settling(&mut rig.app, 42);
        settle(&mut rig.app, work);
        let repo = form(&rig.app).github_repo().cloned().expect("a repo");
        assert_eq!(
            *details_of(&rig.app),
            IssueDetailsState::Failed {
                number: 42,
                message: IssueLoadError::NotSignedIn.message(&repo),
            }
        );
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Fix));
        let _ = remote_lists_sent(&mut rig.rx);
        send(&mut rig.app, FormMsg::Submitted);
        assert_eq!(
            preflights_sent(&mut rig.rx),
            vec!["fix/42_crash-when-opening-empty-project".to_string()]
        );
    }

    /// T028 / US3-1 — a kind that cannot be read is a "Not readable: …" line among the others.
    #[test]
    fn details_a_not_readable_kind_is_a_line_beside_the_others() {
        let mut rig = loaded_rig();
        let _work = pick_without_settling(&mut rig.app, 42);
        let mut details = blocked(42);
        details.projects = Kind::NotReadable("GitHub refused access".into());
        let msg = loaded_details(&rig, 42, details);
        send(&mut rig.app, msg);
        let IssueDetailsState::Loaded { details, .. } = details_of(&rig.app) else {
            panic!("loaded");
        };
        let lines = details.display_lines();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert_eq!(lines[0].label, "Blocked by");
        assert_eq!(lines[1].label, "Projects");
        assert_eq!(lines[1].text, "Not readable: GitHub refused access");
    }

    /// T028 / US3-4 — Create with a blocked issue loaded submits as today.
    #[test]
    fn details_a_blocked_issue_is_still_created() {
        let mut rig = loaded_rig();
        let _work = pick_without_settling(&mut rig.app, 42);
        let msg = loaded_details(&rig, 42, blocked(42));
        send(&mut rig.app, msg);
        send(&mut rig.app, FormMsg::TypeSelected(ConventionalType::Fix));
        let _ = remote_lists_sent(&mut rig.rx);
        send(&mut rig.app, FormMsg::Submitted);
        assert_eq!(
            preflights_sent(&mut rig.rx),
            vec!["fix/42_crash-when-opening-empty-project".to_string()]
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
            micold_core::daemons::DaemonId(1),
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
        rig.app.core.workspace.active = Some(std::path::PathBuf::from("/repo/other"));
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
        rig.app.disconnect_test_daemon();
        send(&mut rig.app, FormMsg::Opened);
        assert_eq!(form(&rig.app).github, unreachable);
        assert!(
            nothing_was_reported(&rig.app),
            "the caption says it; no toast"
        );

        let mut rig = issue_rig(Some(FAKE_GH), FakeIssueSource::new());
        send(&mut rig.app, FormMsg::Opened);
        let work =
            shell::daemon_sync::on_disconnected(&mut rig.app, micold_core::daemons::DaemonId(1));
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
            bodies: Vec::new(),
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
    fn preflights_sent(rx: &mut SentRx) -> Vec<String> {
        sent_matching(rx, |msg| match msg {
            ClientMsg::BranchPreflight { branch, .. } => Some(branch),
            _ => None,
        })
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
            micold_core::daemons::DaemonId(1),
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

    /// A22 (blank) — a blank label refuses the save: its field reads "required" and nothing is
    /// written (US3-5, FR-019; feature 937, FR-005).
    #[test]
    fn issue_an_invalid_mapping_blank_label_is_not_saved() {
        use micold_client::features::window::FieldId;
        use micold_core::required::FieldRequirement;
        let (mut rig, store) = file_rig(labelled(), micold_core::issue_types::default_mapping());
        save_with_added_label(&mut rig, "");
        let draft = rig
            .app
            .core
            .settings
            .settings_draft
            .as_ref()
            .expect("still open");
        assert_eq!(draft.section, SettingsSection::GithubIssues);
        assert_eq!(
            draft.required.state_in(
                &draft.required_fields(SettingsSection::GithubIssues),
                FieldId::IssueMappingLabel(3)
            ),
            FieldRequirement::Missing,
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

    const WITHOUT_PI: [AiCli; 4] = [
        AiCli::ClaudeCode,
        AiCli::Copilot,
        AiCli::Codex,
        AiCli::OpenCode,
    ];

    /// Open Settings, which asks about the home directory, and answer that request.
    fn settings_opened_and_answered(
        app: &mut App,
        sent: &mut Sent,
        available: &[AiCli],
        env: Option<SpawnEnv>,
    ) {
        let _ = update_inner(app, Message::Settings(SettingsMsg::Opened));
        let asked = sent_matching(sent, |msg| match msg {
            ClientMsg::AiCliAvailabilityRequest { req, cwd: None } => Some(req),
            _ => None,
        });
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
        let said = note(&app).expect("four CLIs are missing");
        assert!(
            said.starts_with(
                "A session would not find GitHub Copilot, Pi Coding Agent, Codex and OpenCode:"
            ) && said.contains("\"Source a script before each session\" is off"),
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

    const WITHOUT_PI: [AiCli; 4] = [
        AiCli::ClaudeCode,
        AiCli::Copilot,
        AiCli::Codex,
        AiCli::OpenCode,
    ];

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

        let after = drain(&mut sent);
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
// Feature 037, Story 3: a row's CLI list shows every CLI, and says why one is disabled
// ---------------------------------------------------------------------------------------------

mod a_rows_cli_list_shows_every_cli_and_says_why_one_is_disabled {
    use super::*;
    use micold_client::features::session::{CliEntryAvailability, StartMenu, StartMenuEntry};
    use micold_core::cli_reason::{explain, start_refusal, AttemptDir, Place, SpawnEnv};
    use micold_core::terminal::LaunchMode;

    const WITHOUT_PI: &[AiCli] = &[
        AiCli::ClaudeCode,
        AiCli::Copilot,
        AiCli::Codex,
        AiCli::OpenCode,
    ];
    const EVERY: &[AiCli] = &AiCli::ALL;

    /// One directory's answer: what a session there finds, and the state it was walked in.
    type Answer<'a> = (&'a str, &'a [AiCli], Option<SpawnEnv>);

    /// Answer each request in `asked` with its directory's entry of `dirs`. The home directory
    /// has every CLI and an applied script, so a reason can only come from a row's own answer.
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

    fn entries(app: &App, dir: &str) -> Vec<StartMenuEntry> {
        app.core.session.start_menu_entries(Path::new(dir))
    }

    /// The reason `cli`'s entry gives, `None` for an available entry.
    fn reason(app: &App, dir: &str, cli: AiCli) -> Option<String> {
        entries(app, dir)
            .into_iter()
            .find(|entry| entry.cli == cli)
            .and_then(|entry| match entry.availability {
                CliEntryAvailability::Available => None,
                CliEntryAvailability::Unavailable { reason } => Some(reason),
            })
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

    /// A15, A21 (US3-AS1, FR-010, SC-007): the row's directory provides four CLIs and no Pi, and
    /// the startup script failed there. With the list open, it has an entry per supported CLI in
    /// supported order, Pi's carries the reason, the action and the row's own directory, and the
    /// four CLIs are still offered.
    #[test]
    fn a_list_shows_every_cli_and_the_missing_one_carries_the_reason_for_the_rows_directory() {
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

        let clis: Vec<AiCli> = entries(&app, FEAT_A).iter().map(|e| e.cli).collect();
        let mut expected = AiCli::ALL.to_vec();
        expected.insert(1, AiCli::ClaudeCode); // the structured entry (938)
        assert_eq!(clis, expected, "an entry per supported CLI, in order");
        let said = reason(&app, FEAT_A, AiCli::Pi).expect("Pi is unavailable there");
        assert_eq!(said, pi_missing(SpawnEnv::ScriptFailed, FEAT_A));
        for part in [
            "Pi Coding Agent",
            "the startup script exited with an error",
            "Fix the script named in \"Script path\".",
            FEAT_A,
        ] {
            assert!(said.contains(part), "the reason lacks {part:?}: {said}");
        }
        assert_eq!(offered(&app, FEAT_A), WITHOUT_PI);
    }

    /// A16 (US3-AS1a, FR-010, SC-007): a row with one CLI has no chevron, and its entries still
    /// show the other CLIs as unavailable.
    #[test]
    fn a_row_with_one_cli_has_no_chevron_and_its_entries_mark_the_rest_unavailable() {
        let (app, _sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, CLAUDE, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, EVERY, Some(SpawnEnv::Applied)),
        ]);

        assert!(
            !offers_a_choice(&app, FEAT_A),
            "one CLI: no chevron (026 FR-006)"
        );
        assert_eq!(reason(&app, FEAT_A, AiCli::ClaudeCode), None);
        assert!(reason(&app, FEAT_A, AiCli::Pi).is_some());
    }

    /// A17, A22 (US3-AS1b, FR-010, FR-008, D6): one CLI, and it is not the stored default. The
    /// primary press opens the list with Story 2's message, and the list has every entry.
    #[test]
    fn a_missing_default_on_a_row_with_one_cli_opens_the_list_with_the_message_and_every_entry() {
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
        assert_eq!(entries(&app, FEAT_A).len(), AiCli::ALL.len() + 1);
        assert_eq!(
            reason(&app, FEAT_A, AiCli::Pi),
            Some(pi_missing(SpawnEnv::ScriptFailed, FEAT_A)),
            "and Pi is a disabled entry with the same reason"
        );
    }

    /// A18, A23 (US3-AS2, FR-011, SC-005): every CLI is offered, so every entry is available and
    /// none has a reason.
    #[test]
    fn a_row_with_every_cli_has_every_entry_available() {
        let (mut app, _sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_B, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
        ]);
        open_start_list(&mut app, SessionLocation::Worktree("feat-a".into()));

        assert_eq!(offered(&app, FEAT_A), EVERY);
        assert!(entries(&app, FEAT_A)
            .iter()
            .all(|e| e.availability == CliEntryAvailability::Available));
    }

    /// A19 (US3-AS3, FR-012): two rows whose answers differ each carry their own directory's
    /// reasons, and opening, answering and closing one row's list leaves the other's alone.
    #[test]
    fn each_row_keeps_its_own_reasons_while_another_rows_list_is_opened_answered_and_closed() {
        const WITHOUT_COPILOT: &[AiCli] = &[AiCli::ClaudeCode, AiCli::Pi];
        let (mut app, mut sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, WITHOUT_COPILOT, Some(SpawnEnv::IncludeOff)),
        ]);
        let a_says = pi_missing(SpawnEnv::ScriptFailed, FEAT_A);
        let b_says = reason(&app, FEAT_B, AiCli::Copilot).expect("feat-b lacks Copilot");
        assert_eq!(reason(&app, FEAT_A, AiCli::Pi), Some(a_says.clone()));
        assert!(
            b_says.contains("GitHub Copilot") && !b_says.contains("Pi Coding Agent"),
            "feat-b's reason is about what feat-b lacks: {b_says}"
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
        assert_eq!(
            reason(&app, FEAT_A, AiCli::Pi),
            Some(a_says.clone()),
            "while it is open"
        );
        answer_rows(
            &mut app,
            &asked,
            &[(FEAT_B, WITHOUT_PI, Some(SpawnEnv::ScriptTimedOut))],
        );
        assert_eq!(
            reason(&app, FEAT_B, AiCli::Pi),
            Some(pi_missing(SpawnEnv::ScriptTimedOut, FEAT_B)),
            "feat-b's reason follows feat-b's newer answer (contract W6)"
        );
        let _ = update_inner(&mut app, Message::Session(SessionMsg::StartMenuDismissed));

        assert_eq!(reason(&app, FEAT_A, AiCli::Pi), Some(a_says));
        assert_eq!(offered(&app, FEAT_A), WITHOUT_PI);
    }

    /// A20, A24 (US3-AS4, FR-010, FR-015): the list has an entry for Pi that is not among the
    /// ones offered, and opening the list started nothing. The item that entry draws has no
    /// message (`menu_anatomy::start_list_items`).
    #[test]
    fn the_unavailable_cli_is_shown_but_not_offered_and_nothing_starts() {
        let (mut app, mut sent) = app_with_rows(&[
            (DEMO, EVERY, Some(SpawnEnv::Applied)),
            (FEAT_A, WITHOUT_PI, Some(SpawnEnv::ScriptFailed)),
            (FEAT_B, EVERY, Some(SpawnEnv::Applied)),
        ]);
        open_start_list(&mut app, SessionLocation::Worktree("feat-a".into()));

        let said = reason(&app, FEAT_A, AiCli::Pi).expect("the list says why Pi is disabled");
        assert!(said.contains(&AiCli::Pi.to_string()), "{said}");
        assert!(
            !offered(&app, FEAT_A).contains(&AiCli::Pi),
            "a disabled entry is not among the CLIs a session can start on"
        );

        let after = drain(&mut sent);
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
        keep_running_in_the_draft(&mut app);
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
        ReadingFailure, ReviewState, StatusReading,
    };
    use std::collections::BTreeMap;

    const FAKE_GH: &str = "/fake/bin/gh";

    struct Rig {
        app: App,
        rx: iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
        source: Arc<FakePullRequestSource>,
    }

    /// Connected, `DEMO` active, the switch on, and tooling that finds `gh` at `gh` (or nowhere).
    fn rig(gh: Option<&str>, source: FakePullRequestSource) -> Rig {
        let source = Arc::new(source);
        let (app, rx) = app_over(
            gh,
            Arc::clone(&source) as Arc<dyn PullRequestSource + Send + Sync>,
        );
        Rig { app, rx, source }
    }

    /// The app of [`rig`] over any source.
    fn app_over(
        gh: Option<&str>,
        source: Arc<dyn PullRequestSource + Send + Sync>,
    ) -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (tx, rx) = iced::futures::channel::mpsc::unbounded();
        let found = gh.map(PathBuf::from);
        let tooling = IssueTooling {
            locate_gh: Arc::new(move |_: Option<&str>| found.clone()),
            source: Arc::new(|_: PathBuf| -> Arc<dyn IssueSource + Send + Sync> {
                unreachable!("a pull request reading never builds the issue source")
            }),
            pull_requests: Arc::new(move |_: PathBuf| Arc::clone(&source)),
        };
        let mut app = base_app();
        app.caps = app.caps.clone().with_issue_tooling(tooling);
        app.connect_test_daemon(micold_client::daemon::Outbox::new(tx));
        app.core.workspace.active = Some(PathBuf::from(DEMO));
        let _ = shell::pr_status::enabled_changed(&mut app, true);
        (app, rx)
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
            conflict: false,
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
            });
        }
        catalog
    }

    fn one_worktree() -> micold_core::protocol::messages::CatalogSnapshot {
        listing(&[("a", Some("feat/a"))])
    }

    /// A daemon message whose follow-up is dropped: for a reading that is the 10-second timer.
    fn daemon(app: &mut App, event: DaemonMsg) {
        let _ = shell::daemon_sync::on_daemon_event(app, micold_core::daemons::DaemonId(1), event);
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
        drain(&mut rig.rx)
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

    // Feature 1102 (FR-009): the GitHub issues switch off leaves the pull request status reading
    // alone: the remotes are still asked for and the branches still read.
    #[test]
    fn pr_status_still_reads_with_the_github_issues_switch_off() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_answer(statuses()),
        );
        rig.app.core.settings.github_issues_enabled = false;
        attached(&mut rig.app);
        listed(&mut rig.app, listing(&[("a", Some("feat/a"))]));
        let asked = remote_lists(&mut rig);
        assert_eq!(asked.len(), 1, "the remotes are still asked for");
        answer_remotes(&mut rig.app, asked[0], github_remote());
        assert_eq!(
            rig.source.calls(),
            vec![("o/r".to_string(), vec!["feat/a".to_string()])]
        );
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
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

    /// Press the refresh control on a rig that holds, and return the refresh request's `req`.
    fn press_refresh(rig: &mut Rig) -> u64 {
        // The task is the 30-second bound; it is dropped, not waited for.
        let _ = shell::daemon_sync::on_worktree_refresh_requested(&mut rig.app);
        assert!(rig.app.core.worktree.refreshing, "the control is busy");
        let _ = sent(rig);
        let refresh = *rig
            .app
            .pending_ops
            .iter()
            .find(|(_, op)| matches!(op, PendingOp::WorktreeRefresh))
            .expect("the press asks the daemon to refresh")
            .0;
        refresh
    }

    // A34, A36 (S4, FR-018a, FR-027): the refresh acknowledged returns the control to idle, shows
    // its notice, and only then starts a reading for the branches the updated listing shows.
    #[test]
    fn pr_status_refresh_ack_reads_after_the_control_is_idle() {
        let mut rig = holding();
        let refresh = press_refresh(&mut rig);
        // The daemon broadcasts the new listing before it acknowledges (029 contract §4).
        listed(
            &mut rig.app,
            listing(&[("a", Some("feat/a")), ("c", Some("feat/c"))]),
        );
        assert!(
            remote_lists(&mut rig).is_empty(),
            "the listing alone starts nothing"
        );

        // The follow-up is the reading's 10-second bound; it is dropped, not waited for.
        daemon(
            &mut rig.app,
            DaemonMsg::OperationOk {
                req: refresh,
                result: OperationResult::Ack,
            },
        );
        assert!(!rig.app.core.worktree.refreshing, "the control is idle");
        assert_eq!(
            rig.app
                .core
                .notifications
                .queue
                .visible()
                .map(|n| n.message.as_str()),
            Some("Worktree list refreshed.")
        );
        let asked = remote_lists(&mut rig);
        assert_eq!(asked.len(), 1, "one reading starts");
        answer_remotes(&mut rig.app, asked[0], github_remote());
        assert_eq!(
            rig.source
                .calls()
                .last()
                .map(|(_, branches)| branches.clone()),
            Some(vec!["feat/a".to_string(), "feat/c".to_string()]),
            "the branches the updated listing shows"
        );
    }

    // A34 (S4), the timed-out variant: a refresh that timed out ends the same way, and still reads.
    #[test]
    fn pr_status_refresh_timed_out_reads_too() {
        let mut rig = holding();
        let refresh = press_refresh(&mut rig);
        let _ = shell::daemon_sync::on_worktree_refresh_timed_out(&mut rig.app, refresh);
        assert!(!rig.app.core.worktree.refreshing);
        assert_eq!(remote_lists(&mut rig).len(), 1);
    }

    // A34 (S4), the failed variant: a refresh the daemon failed ends the same way, and still reads.
    #[test]
    fn pr_status_refresh_failed_reads_too() {
        let mut rig = holding();
        let refresh = press_refresh(&mut rig);
        daemon(
            &mut rig.app,
            DaemonMsg::OperationError {
                req: refresh,
                kind: micold_core::protocol::messages::ErrorKind::Internal,
                message: "the refresh failed".into(),
                detail: None,
            },
        );
        assert!(!rig.app.core.worktree.refreshing);
        assert_eq!(remote_lists(&mut rig).len(), 1);
    }

    // A36 (FR-027): a failed reading changes neither the control nor the notice the refresh shows.
    #[test]
    fn pr_status_refresh_then_a_failed_reading_adds_nothing_to_the_refresh() {
        let mut rig = holding();
        let refresh = press_refresh(&mut rig);
        // The follow-up is the reading's 10-second bound; it is dropped, not waited for.
        daemon(
            &mut rig.app,
            DaemonMsg::OperationOk {
                req: refresh,
                result: OperationResult::Ack,
            },
        );
        let notice_before = rig
            .app
            .core
            .notifications
            .queue
            .visible()
            .map(|n| n.message.clone());
        let asked = remote_lists(&mut rig);
        assert_eq!(asked.len(), 1);
        let seq = seq_under_way(&rig.app);
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq, req: asked[0] }),
        );
        settle(&mut rig.app, work);
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        assert!(!rig.app.core.worktree.refreshing);
        assert_eq!(
            rig.app
                .core
                .notifications
                .queue
                .visible()
                .map(|n| n.message.clone()),
            notice_before
        );
        assert_eq!(rig.app.core.notifications.queue.pending(), 0);
    }

    // A32 (S3): the interval's tick starts one reading; a tick while the switch is off starts none.
    #[test]
    fn pr_status_tick_starts_a_reading() {
        let mut rig = holding();
        let _ = update_inner(&mut rig.app, Message::PrStatus(Msg::Tick));
        assert_eq!(remote_lists(&mut rig).len(), 1);
        assert!(matches!(
            rig.app.core.pr_status.phase,
            Phase::Reading { .. }
        ));
    }

    #[test]
    fn pr_status_tick_with_the_switch_off_starts_nothing() {
        let mut rig = holding();
        let _ = shell::pr_status::enabled_changed(&mut rig.app, false);
        let _ = sent(&mut rig);
        let _ = update_inner(&mut rig.app, Message::PrStatus(Msg::Tick));
        assert!(sent(&mut rig).is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
    }

    /// A source that tells the test it was called and then waits to be let go.
    struct BlockingSource {
        entered: std::sync::mpsc::Sender<()>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl PullRequestSource for BlockingSource {
        fn read(
            &self,
            _: &micold_core::github::GithubRepo,
            _: &[String],
            _: u64,
        ) -> Result<StatusReading, ReadingFailure> {
            self.entered.send(()).expect("the test listens");
            self.release
                .lock()
                .expect("not poisoned")
                .recv()
                .expect("the test lets the source go");
            Ok(StatusReading {
                statuses: statuses(),
                ..StatusReading::default()
            })
        }

        fn file_at(
            &self,
            _: &micold_core::github::GithubRepo,
            _: &str,
            _: &micold_core::review::RelPath,
            _: u64,
        ) -> Result<Vec<u8>, micold_core::review::browse::FileReadFailure> {
            Ok(Vec::new())
        }

        fn list_branches(
            &self,
            _: &micold_core::github::GithubRepo,
            _: &[String],
            _: u64,
        ) -> Result<Vec<micold_core::pull_request::BranchPullRequest>, ReadingFailure> {
            Ok(Vec::new())
        }

        fn diff(
            &self,
            _: &micold_core::github::GithubRepo,
            _: u64,
            _: u64,
        ) -> Result<String, DiffReadFailure> {
            Ok(String::new())
        }
    }

    // A12 (FR-021, SC-005): the update that starts the source returns without calling it, and a
    // selection message is applied while the source is still reading.
    #[test]
    fn pr_status_a_selection_is_applied_while_the_source_is_reading() {
        let (entered_tx, entered) = std::sync::mpsc::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let source = Arc::new(BlockingSource {
            entered: entered_tx,
            release: std::sync::Mutex::new(release_rx),
        });
        let (mut app, mut rx) = app_over(Some(FAKE_GH), source);
        let mut rig_sent = || drain(&mut rx);
        attached(&mut app);
        listed(&mut app, one_worktree());
        let req = rig_sent()
            .into_iter()
            .find_map(|msg| match msg {
                ClientMsg::RemoteList { req, .. } => Some(req),
                _ => None,
            })
            .expect("the listing asks for the remotes");

        let work = shell::daemon_sync::on_daemon_event(
            &mut app,
            micold_core::daemons::DaemonId(1),
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RemoteList {
                    remotes: github_remote(),
                },
            },
        );
        assert!(
            entered.try_recv().is_err(),
            "the update returned without calling the source"
        );
        let reading = std::thread::spawn(move || messages(work));
        entered
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("the source is reading");

        let id = SessionId::new();
        let _ = update_inner(&mut app, Message::Session(SessionMsg::Selected(id)));
        assert!(
            rig_sent().iter().any(
                |msg| matches!(msg, ClientMsg::SessionStart { session, .. } if *session == id)
            ),
            "the selection was applied while the reading was outstanding"
        );
        assert!(matches!(app.core.pr_status.phase, Phase::Reading { .. }));

        release.send(()).expect("the source waits");
        for message in reading.join().expect("the reading finished") {
            let _ = update_inner(&mut app, message);
        }
        assert_eq!(app.core.pr_status.statuses, statuses());
    }

    /// A rig that holds `statuses()`, then reads a second time and gets `second` (the tick's reading
    /// when `by_tick`), returning once that reading has ended.
    fn holding_then(second: Result<BTreeMap<String, PullRequestStatus>, ReadingFailure>) -> Rig {
        let source = FakePullRequestSource::new().with_answer(statuses());
        let source = match second {
            Ok(answer) => source.with_answer(answer),
            Err(failure) => source.with_failure(failure),
        };
        let mut rig = rig(Some(FAKE_GH), source);
        read_once(&mut rig, one_worktree());
        tick_and_answer(&mut rig);
        assert_eq!(rig.source.calls().len(), 2);
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        rig
    }

    /// A tick, and the answer of the remotes it asked for.
    fn tick_and_answer(rig: &mut Rig) {
        let _ = update_inner(&mut rig.app, Message::PrStatus(Msg::Tick));
        let asked = remote_lists(rig);
        assert_eq!(asked.len(), 1, "the tick starts one reading");
        answer_remotes(&mut rig.app, asked[0], github_remote());
    }

    // A33 (FR-020): a tick's reading that differs replaces the branch's status and leaves the rest.
    #[test]
    fn pr_status_tick_replaces_the_status_and_nothing_else() {
        let other = BTreeMap::from([("feat/a".to_string(), status(9))]);
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_answer(statuses())
                .with_answer(other.clone())
                .with_answer(BTreeMap::new()),
        );
        read_once(&mut rig, one_worktree());
        let before = untouched(&rig.app);
        tick_and_answer(&mut rig);
        assert_eq!(rig.app.core.pr_status.statuses, other);
        assert_eq!(untouched(&rig.app), before);
        // A reading replaces the whole map: a branch whose pull request is gone is not kept.
        tick_and_answer(&mut rig);
        assert!(rig.app.core.pr_status.statuses.is_empty());
    }

    // A35 (FR-022): a tick and three refreshes during a reading add no call; one more when it ends.
    #[test]
    fn pr_status_tick_and_refreshes_during_a_reading_add_one_call_when_it_ends() {
        let mut rig = holding();
        let req = read_again(&mut rig);
        let _ = update_inner(&mut rig.app, Message::PrStatus(Msg::Tick));
        for _ in 0..3 {
            let _ = shell::pr_status::refresh_ended(&mut rig.app);
        }
        assert!(
            remote_lists(&mut rig).is_empty(),
            "nothing starts meanwhile"
        );
        assert_eq!(rig.source.calls().len(), 1);

        answer_remotes(&mut rig.app, req, github_remote());
        assert_eq!(rig.source.calls().len(), 2, "the reading itself");
        assert_eq!(remote_lists(&mut rig).len(), 1, "exactly one more starts");
    }

    // A37 (FR-024, SC-006): after `RateLimited` a tick adds no call and the statuses stay.
    #[test]
    fn pr_status_tick_after_rate_limited_adds_no_call() {
        let mut rig = holding_then(Err(ReadingFailure::RateLimited { until: u64::MAX }));
        let _ = sent(&mut rig);
        let _ = update_inner(&mut rig.app, Message::PrStatus(Msg::Tick));
        assert!(remote_lists(&mut rig).is_empty());
        assert_eq!(rig.source.calls().len(), 2);
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
    }

    // A38 (FR-019, FR-025): after `Passing` the statuses stay, nothing is reported, the next tick reads.
    #[test]
    fn pr_status_tick_after_passing_keeps_the_statuses_and_reads_again() {
        let mut rig = holding_then(Err(ReadingFailure::Passing));
        assert_eq!(rig.app.core.pr_status.statuses, statuses());
        assert!(nothing_was_reported(&rig.app));
        tick_and_answer(&mut rig);
        assert_eq!(rig.source.calls().len(), 3);
    }

    // A39 (FR-025): after `Unavailable` the statuses and `removable` are empty, nothing is reported,
    // the next tick reads.
    #[test]
    fn pr_status_tick_after_unavailable_clears_and_reads_again() {
        let mut rig = holding_then(Err(ReadingFailure::Unavailable));
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert!(rig.app.core.pr_status.removable.is_empty());
        assert!(nothing_was_reported(&rig.app));
        tick_and_answer(&mut rig);
        assert_eq!(rig.source.calls().len(), 3);
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

        // Nor does a timer whose request is not the one the reading waits for.
        let req = read_again(&mut rig);
        let seq = seq_under_way(&rig.app);
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut {
                seq,
                req: req + 1000,
            }),
        );
        settle(&mut rig.app, work);
        assert!(
            matches!(rig.app.core.pr_status.phase, Phase::Reading { .. }),
            "the reading is still waiting for its own answer"
        );
    }

    // The daemon failing the remotes request is a passing failure, and never a notice.
    #[test]
    fn pr_status_a_failed_remotes_request_keeps_the_statuses_without_a_notice() {
        let mut rig = holding();
        let req = read_again(&mut rig);
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            micold_core::daemons::DaemonId(1),
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
            Some((
                PathBuf::from(DEMO),
                micold_core::daemons::Binding::Bound(micold_core::daemons::DaemonId(1)),
            )),
            Path::new("/repo/other"),
        );
        assert_released(&mut rig);
    }

    // U95: a disconnect, with a reading under way: its remotes request is dropped without a notice.
    #[test]
    fn pr_status_a_disconnect_clears_and_reads_nothing() {
        let mut rig = holding();
        let _ = read_again(&mut rig);
        let _ =
            shell::daemon_sync::on_disconnected(&mut rig.app, micold_core::daemons::DaemonId(1));
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
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
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
        daemon(&mut rig.app, settings(true));
        assert!(
            remote_lists(&mut rig).is_empty(),
            "the same value again, after the reading ended, reads nothing"
        );

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

    // A33, U97 (FR-020, SC-004): every way a reading ends leaves the rest of the application as it
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

    // ---- Feature 040, T049: the merged-branch question (reading-and-wire §2 step 4, §3) --------
    //
    // The 10-second bound on the daemon's answer is a task the source's end returns. These tests
    // run the source's task themselves and drop that timer, as the helpers above drop the remotes'
    // one, and send the timer's message where they want the bound to run out.

    use micold_core::protocol::messages::{BranchContainment, ErrorKind, MergedBranchQuery};

    fn head(c: char) -> String {
        c.to_string().repeat(40)
    }

    fn with_state(state: PrState, head_char: char) -> PullRequestStatus {
        PullRequestStatus {
            state,
            head: head(head_char),
            ..status(1)
        }
    }

    /// One branch per kind of pull request: only `feat/m` is merged.
    fn mixed() -> BTreeMap<String, PullRequestStatus> {
        BTreeMap::from([
            ("feat/m".to_string(), with_state(PrState::Merged, 'b')),
            (
                "feat/o".to_string(),
                with_state(
                    PrState::Open {
                        checks: CheckStatus::None,
                    },
                    'c',
                ),
            ),
            (
                "feat/d".to_string(),
                with_state(
                    PrState::Draft {
                        checks: CheckStatus::None,
                    },
                    'd',
                ),
            ),
            ("feat/c".to_string(), with_state(PrState::Closed, 'e')),
        ])
    }

    fn mixed_listing() -> micold_core::protocol::messages::CatalogSnapshot {
        listing(&[
            ("m", Some("feat/m")),
            ("o", Some("feat/o")),
            ("d", Some("feat/d")),
            ("c", Some("feat/c")),
        ])
    }

    /// Run the reading up to the point where the source has answered: the remotes answered, the
    /// source's task run, its message handled and its timer dropped.
    fn read_to_source_end(rig: &mut Rig, req: u64) {
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            micold_core::daemons::DaemonId(1),
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::RemoteList {
                    remotes: github_remote(),
                },
            },
        );
        for message in messages(work) {
            let _ = update_inner(&mut rig.app, message);
        }
    }

    /// A rig whose reading of `answer` has ended at the source and, if it asked, now waits for the
    /// daemon. Returns it with the `MergedBranchCheck` it sent, if any.
    fn waiting_for_merged_check(
        answer: BTreeMap<String, PullRequestStatus>,
        catalog: micold_core::protocol::messages::CatalogSnapshot,
    ) -> (Rig, Option<(u64, Vec<MergedBranchQuery>)>) {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_answer(answer),
        );
        attached(&mut rig.app);
        listed(&mut rig.app, catalog);
        let asked = remote_lists(&mut rig);
        assert_eq!(asked.len(), 1);
        read_to_source_end(&mut rig, asked[0]);
        let mut checks: Vec<(u64, Vec<MergedBranchQuery>)> = sent(&mut rig)
            .into_iter()
            .filter_map(|msg| match msg {
                ClientMsg::MergedBranchCheck {
                    req,
                    project,
                    checks,
                } => {
                    assert_eq!(project, PathBuf::from(DEMO));
                    Some((req, checks))
                }
                _ => None,
            })
            .collect();
        assert!(checks.len() <= 1, "at most one question per reading");
        let check = checks.pop();
        (rig, check)
    }

    fn answer_check(rig: &mut Rig, req: u64, answers: Vec<BranchContainment>) {
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            micold_core::daemons::DaemonId(1),
            DaemonMsg::OperationOk {
                req,
                result: OperationResult::MergedBranchCheck { answers },
            },
        );
        settle(&mut rig.app, work);
    }

    fn removable(rig: &Rig) -> Vec<&str> {
        rig.app
            .core
            .pr_status
            .removable
            .iter()
            .map(String::as_str)
            .collect()
    }

    /// The reading ended with the statuses applied and nothing suggested, and said nothing.
    fn assert_applied_without_a_suggestion(rig: &Rig) {
        assert_eq!(rig.app.core.pr_status.statuses, mixed());
        assert!(removable(rig).is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
        assert!(nothing_was_reported(&rig.app));
    }

    // A25, A27, A29, U122 (FR-015): one query per merged branch with its `head`, none for an open,
    // draft or closed one; and nothing but the question is sent (SC-010).
    #[test]
    fn pr_status_merged_asks_one_query_per_merged_branch() {
        let (rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
        let (_, queries) = check.expect("a merged branch is asked about");
        assert_eq!(
            queries,
            vec![MergedBranchQuery {
                branch: "feat/m".into(),
                head: head('b'),
            }]
        );
        // The statuses wait for the answer: the reading is not over.
        assert!(matches!(
            rig.app.core.pr_status.phase,
            Phase::Reading { .. }
        ));
    }

    // A27 (FR-016, SC-010): a reading that ends with a removable branch sent no message that
    // deletes anything, and the mark removes nothing.
    #[test]
    fn pr_status_merged_sends_nothing_that_deletes() {
        let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
        let (req, _) = check.expect("asked");
        answer_check(&mut rig, req, vec![BranchContainment::Contained]);
        assert_eq!(removable(&rig), ["feat/m"]);
        let after: Vec<ClientMsg> = sent(&mut rig);
        assert!(
            !after.iter().any(|msg| matches!(
                msg,
                ClientMsg::WorktreeDelete { .. }
                    | ClientMsg::SessionDelete { .. }
                    | ClientMsg::WorktreeRename { .. }
            )),
            "{after:?}"
        );
        assert_eq!(rig.app.core.worktree.worktrees.len(), 4);
    }

    // U123 (FR-023): with no merged branch the daemon is not asked, and the reading ends at once.
    #[test]
    fn pr_status_merged_is_not_sent_without_a_merged_branch() {
        let mut open_only = mixed();
        open_only.retain(|_, status| !matches!(status.state, PrState::Merged));
        let (rig, check) = waiting_for_merged_check(open_only.clone(), mixed_listing());
        assert!(check.is_none());
        assert_eq!(rig.app.core.pr_status.statuses, open_only);
        assert!(removable(&rig).is_empty());
        assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
    }

    // A30 (story 3 scenario 6): a branch with a merged pull request and a newer open one is held as
    // the open one, so it is not asked about and not removable.
    #[test]
    fn pr_status_merged_a_branch_with_a_newer_open_pull_request_is_not_asked_about() {
        let newer = BTreeMap::from([(
            "feat/m".to_string(),
            with_state(
                PrState::Open {
                    checks: CheckStatus::Passing,
                },
                'f',
            ),
        )]);
        let (rig, check) = waiting_for_merged_check(newer, mixed_listing());
        assert!(check.is_none());
        assert!(removable(&rig).is_empty());
    }

    // A25, A28, U124 (FR-015, FR-017): `Contained` marks the branch; `Beyond` and `Unknown` do not.
    #[test]
    fn pr_status_merged_only_contained_marks_the_branch() {
        for (answer, marked) in [
            (BranchContainment::Contained, true),
            (BranchContainment::Beyond, false),
            (BranchContainment::Unknown, false),
        ] {
            let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
            let (req, _) = check.expect("asked");
            answer_check(&mut rig, req, vec![answer]);
            assert_eq!(rig.app.core.pr_status.statuses, mixed(), "{answer:?}");
            assert_eq!(removable(&rig) == ["feat/m"], marked, "{answer:?}");
            assert_eq!(rig.app.core.pr_status.phase, Phase::Idle);
            assert!(nothing_was_reported(&rig.app));
        }
    }

    // U125 (FR-021): no answer within the bound applies the statuses with no suggestion; the answer
    // arriving after all is nobody's.
    #[test]
    fn pr_status_merged_unanswered_for_ten_seconds_applies_the_statuses_without_a_suggestion() {
        let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
        let (req, _) = check.expect("asked");
        let seq = seq_under_way(&rig.app);
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::MergedCheckTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert_applied_without_a_suggestion(&rig);

        answer_check(&mut rig, req, vec![BranchContainment::Contained]);
        assert!(removable(&rig).is_empty());
    }

    // U125: the timer of a question that was answered in time changes nothing.
    #[test]
    fn pr_status_merged_the_timer_of_an_answered_question_changes_nothing() {
        let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
        let (req, _) = check.expect("asked");
        let seq = seq_under_way(&rig.app);
        answer_check(&mut rig, req, vec![BranchContainment::Contained]);
        let before = rig.app.core.pr_status.clone();
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::MergedCheckTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert_eq!(rig.app.core.pr_status, before);
    }

    // U125: a refusal applies the statuses with no suggestion, and raises nothing.
    #[test]
    fn pr_status_merged_a_refusal_applies_the_statuses_without_a_suggestion() {
        let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
        let (req, _) = check.expect("asked");
        let work = shell::daemon_sync::on_daemon_event(
            &mut rig.app,
            micold_core::daemons::DaemonId(1),
            DaemonMsg::OperationError {
                req,
                kind: ErrorKind::InvalidInput,
                message: "not a repository".into(),
                detail: None,
            },
        );
        settle(&mut rig.app, work);
        assert_applied_without_a_suggestion(&rig);
    }

    // U125: an answer list of another length is no answer.
    #[test]
    fn pr_status_merged_an_answer_list_of_another_length_applies_no_suggestion() {
        for answers in [
            vec![],
            vec![BranchContainment::Contained, BranchContainment::Contained],
        ] {
            let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
            let (req, _) = check.expect("asked");
            answer_check(&mut rig, req, answers);
            assert_applied_without_a_suggestion(&rig);
        }
    }

    // An answer for a project no longer active is nobody's: the hold ended meanwhile.
    #[test]
    fn pr_status_merged_an_answer_after_the_hold_ended_changes_nothing() {
        let (mut rig, check) = waiting_for_merged_check(mixed(), mixed_listing());
        let (req, _) = check.expect("asked");
        daemon(
            &mut rig.app,
            DaemonMsg::Displaced {
                project: PathBuf::from(DEMO),
                by: other_window(),
            },
        );
        answer_check(&mut rig, req, vec![BranchContainment::Contained]);
        assert!(rig.app.core.pr_status.statuses.is_empty());
        assert!(removable(&rig).is_empty());
        assert!(nothing_was_reported(&rig.app));
    }

    // ---- Feature 775, T007: the Changes view's pull request list, through the shell ----------

    use micold_client::features::changes::{Msg as ChangesMsg, PrDiffLoad, PrList, Source};
    use micold_core::pull_request::DiffReadFailure;
    use micold_core::session::SessionLocation;

    fn branch_pr(number: u64) -> micold_core::pull_request::BranchPullRequest {
        micold_core::pull_request::BranchPullRequest {
            number,
            title: format!("Pull request {number}"),
            url: format!("https://github.com/o/r/pull/{number}"),
            state: micold_core::pull_request::BranchPrState::Open,
            created_at: "2026-10-10T09:00:00Z".into(),
            base: "main".into(),
            head: "a".repeat(40),
            head_branch: "feat-a".into(),
            checks: micold_core::pull_request::CheckStatus::None,
        }
    }

    /// Open the Changes view of `DEMO`'s worktree `a` and ask for its pull requests, as the shell
    /// does right after opening. The timer the ask returns is never run.
    fn ask_list(rig: &mut Rig, branch: &str) {
        let _ = rig.app.core.update_changes(ChangesMsg::Opened {
            project: PathBuf::from(DEMO),
            entry: SessionLocation::Worktree("a".into()),
        });
        let _ = crate::shell::changes::update(
            &mut rig.app,
            ChangesMsg::PullRequestsWanted {
                branch: Some(branch.to_string()),
            },
        );
    }

    fn view_list(app: &App) -> &PrList {
        &app.core
            .changes
            .open
            .as_ref()
            .expect("the view is open")
            .pull_requests
    }

    // FR-010: a read asks the repository of its own project's remote, whatever the branch's name.
    #[test]
    fn pr_list_read_asks_only_the_repository_of_its_own_project() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_list_answer(vec![branch_pr(1)])
                .with_list_answer(vec![branch_pr(2)]),
        );
        ask_list(&mut rig, "feat/a");
        let first = remote_lists(&mut rig);
        assert_eq!(first.len(), 1);
        answer_remotes(&mut rig.app, first[0], github_remote());
        assert_eq!(
            rig.source.list_calls(),
            vec![("o/r".to_string(), vec!["feat/a".to_string()])]
        );
        assert!(matches!(
            view_list(&rig.app),
            PrList::Ready { prs, .. } if prs.len() == 1 && prs[0].number == 1
        ));

        // Another project with a branch of the same name reads its own repository.
        let other = "/projects/other";
        rig.app.core.workspace.active = Some(PathBuf::from(other));
        let _ = rig.app.core.update_changes(ChangesMsg::Opened {
            project: PathBuf::from(other),
            entry: SessionLocation::Worktree("a".into()),
        });
        let _ = crate::shell::changes::update(
            &mut rig.app,
            ChangesMsg::PullRequestsWanted {
                branch: Some("feat/a".into()),
            },
        );
        let second = remote_lists(&mut rig);
        assert_eq!(second.len(), 1);
        answer_remotes(
            &mut rig.app,
            second[0],
            vec![GitRemote {
                name: "origin".into(),
                url: "git@github.com:o2/r2.git".into(),
            }],
        );
        assert_eq!(
            rig.source.list_calls(),
            vec![
                ("o/r".to_string(), vec!["feat/a".to_string()]),
                ("o2/r2".to_string(), vec!["feat/a".to_string()])
            ]
        );
    }

    // ---- Feature 1003, T010: the list read asks for the worktree's branch set ----------------

    /// Run `git -C dir args` as if at Unix second `time`, so the reflog's seconds are the test's.
    fn git_at(dir: &std::path::Path, time: u64, args: &[&str]) {
        use std::process::Command;
        let date = format!("@{time} +0000");
        let out = micold_core::process::no_window(&mut Command::new("git"))
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_COMMITTER_DATE", &date)
            .env("GIT_AUTHOR_DATE", &date)
            .output()
            .expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// A repository under `tmp` with one worktree per `(dir, branches)`: added with `-b` on the
    /// first branch, then `switch -c` to each next one; the first is deleted when there are more.
    /// Listed in the app as worktrees of `DEMO`, each on its last branch.
    fn worktrees_with_history(
        app: &mut App,
        tmp: &tempfile::TempDir,
        worktrees: &[(&str, &[&str])],
    ) {
        let repo = tmp.path().join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        git_at(&repo, 1_000, &["init", "-q"]);
        git_at(&repo, 1_000, &["config", "user.email", "t@t.test"]);
        git_at(&repo, 1_000, &["config", "user.name", "T"]);
        git_at(
            &repo,
            1_000,
            &["commit", "-q", "--allow-empty", "-m", "root"],
        );
        let mut listed = Vec::new();
        for (i, (dir, branches)) in worktrees.iter().enumerate() {
            let wt = tmp.path().join(dir);
            let t = 2_000 + 10_000 * i as u64;
            git_at(
                &repo,
                t,
                &[
                    "worktree",
                    "add",
                    "-q",
                    "-b",
                    branches[0],
                    wt.to_str().unwrap(),
                ],
            );
            for (k, branch) in branches.iter().enumerate().skip(1) {
                let at = t + 1_000 * k as u64;
                git_at(
                    &wt,
                    at - 100,
                    &["commit", "-q", "--allow-empty", "-m", "work"],
                );
                git_at(&wt, at, &["switch", "-q", "-c", branch]);
            }
            if branches.len() > 1 {
                git_at(&wt, t + 5_000, &["branch", "-q", "-D", branches[0]]);
            }
            listed.push(micold_core::worktree::Worktree {
                dir_name: dir.to_string(),
                path: wt,
                branch: branches.last().map(|b| b.to_string()),
                status: micold_core::worktree::WorktreeStatus::Valid,
                included: false,
            });
        }
        app.core.worktree.worktrees = listed;
    }

    /// Open the Changes view of `DEMO`'s worktree `dir`, on `current`, and ask for its pull
    /// requests; answer the remotes with a GitHub one.
    fn ask_branch_set(rig: &mut Rig, dir: &str, current: Option<&str>) {
        let _ = rig.app.core.update_changes(ChangesMsg::Opened {
            project: PathBuf::from(DEMO),
            entry: SessionLocation::Worktree(dir.into()),
        });
        let _ = crate::shell::changes::update(
            &mut rig.app,
            ChangesMsg::PullRequestsWanted {
                branch: current.map(str::to_string),
            },
        );
        let asked = remote_lists(rig);
        assert_eq!(asked.len(), 1, "one remotes ask per read");
        answer_remotes(&mut rig.app, asked[0], github_remote());
    }

    fn set(names: &[&str]) -> std::collections::BTreeSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    /// The branches each `list_branches` call asked for, as sets.
    fn asked_sets(rig: &Rig) -> Vec<std::collections::BTreeSet<String>> {
        rig.source
            .list_calls()
            .into_iter()
            .map(|(repo, branches)| {
                assert_eq!(repo, "o/r");
                branches.into_iter().collect()
            })
            .collect()
    }

    fn pr_on(number: u64, branch: &str) -> micold_core::pull_request::BranchPullRequest {
        let mut pr = branch_pr(number);
        pr.head_branch = branch.into();
        pr
    }

    // US1 scenario 1, FR-001, FR-002: a worktree that had A (deleted), then B, and is on C asks for
    // all three, and lists every pull request the answer holds.
    #[test]
    fn pr_list_branch_set_asks_every_branch_created_in_the_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![
                pr_on(3, "C"),
                pr_on(2, "B"),
                pr_on(1, "A"),
            ]),
        );
        worktrees_with_history(&mut rig.app, &tmp, &[("a", &["A", "B", "C"])]);
        ask_branch_set(&mut rig, "a", Some("C"));
        assert_eq!(asked_sets(&rig), vec![set(&["A", "B", "C"])]);
        assert!(matches!(
            view_list(&rig.app),
            PrList::Ready { prs, .. } if prs.iter().map(|p| p.number).collect::<Vec<_>>() == [3, 2, 1]
        ));
        assert!(nothing_was_reported(&rig.app));
    }

    // US3 scenario 2, FR-009, SC-004: a branch created in the worktree after the view opened is in
    // the branch set the next status trigger asks for.
    #[test]
    fn pr_list_refresh_branch_set_includes_a_branch_created_after_the_view_opened() {
        let tmp = tempfile::tempdir().unwrap();
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_list_answer(vec![pr_on(1, "A")])
                .with_list_answer(vec![pr_on(2, "B"), pr_on(1, "A")]),
        );
        worktrees_with_history(&mut rig.app, &tmp, &[("a", &["A"])]);
        ask_branch_set(&mut rig, "a", Some("A"));
        assert_eq!(asked_sets(&rig), vec![set(&["A"])]);
        let _ = sent(&mut rig);

        git_at(&tmp.path().join("a"), 90_000, &["switch", "-q", "-c", "B"]);
        rig.app.core.worktree.worktrees[0].branch = Some("B".into());
        tick(&mut rig);
        let (req, _) = list_pending(&rig.app).expect("the trigger asked for the list");
        answer_remotes(&mut rig.app, req, github_remote());
        assert_eq!(asked_sets(&rig), vec![set(&["A"]), set(&["A", "B"])]);
        assert!(matches!(
            view_list(&rig.app),
            PrList::Ready { prs, .. } if prs.iter().map(|p| p.number).collect::<Vec<_>>() == [2, 1]
        ));
    }

    // US1 scenario 4, FR-014: two worktrees of one project each ask only their own set.
    #[test]
    fn pr_list_branch_set_two_worktrees_each_ask_only_their_own() {
        let tmp = tempfile::tempdir().unwrap();
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_list_answer(vec![pr_on(1, "C")])
                .with_list_answer(vec![pr_on(2, "Y")]),
        );
        worktrees_with_history(
            &mut rig.app,
            &tmp,
            &[("a", &["A", "B", "C"]), ("b", &["X", "Y"])],
        );
        ask_branch_set(&mut rig, "a", Some("C"));
        ask_branch_set(&mut rig, "b", Some("Y"));
        assert_eq!(
            asked_sets(&rig),
            vec![set(&["A", "B", "C"]), set(&["X", "Y"])]
        );
    }

    // FR-002 with no local git (the session service runs elsewhere): the current branch alone,
    // and no notice.
    #[test]
    fn pr_list_branch_set_without_local_git_asks_the_current_branch_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![pr_on(3, "C")]),
        );
        worktrees_with_history(&mut rig.app, &tmp, &[("a", &["A", "B", "C"])]);
        rig.app.caps = rig.app.caps.clone().without_local_git();
        ask_branch_set(&mut rig, "a", Some("C"));
        assert_eq!(asked_sets(&rig), vec![set(&["C"])]);
        assert!(matches!(view_list(&rig.app), PrList::Ready { prs, .. } if prs.len() == 1));
        assert!(nothing_was_reported(&rig.app));
    }

    // Principle II: a second ask while the first is under way shares its read.
    #[test]
    fn pr_list_read_a_second_ask_shares_the_read_under_way() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![branch_pr(1)]),
        );
        ask_list(&mut rig, "feat/a");
        let _ = crate::shell::changes::update(
            &mut rig.app,
            ChangesMsg::PullRequestsWanted {
                branch: Some("feat/a".into()),
            },
        );
        let asked = remote_lists(&mut rig);
        assert_eq!(asked.len(), 1, "one read for two askers");
        answer_remotes(&mut rig.app, asked[0], github_remote());
        assert_eq!(rig.source.list_calls().len(), 1);
    }

    // FR-008, 1003 FR-011: no `gh`, or no GitHub remote, leaves the muted line and no notice.
    #[test]
    fn pr_list_read_without_gh_is_unavailable_and_silent() {
        let mut rig = rig(None, FakePullRequestSource::new());
        ask_list(&mut rig, "feat/a");
        let asked = remote_lists(&mut rig);
        answer_remotes(&mut rig.app, asked[0], github_remote());
        assert!(matches!(
            view_list(&rig.app),
            PrList::Unavailable { last: None }
        ));
        assert!(rig.source.list_calls().is_empty());
        assert!(nothing_was_reported(&rig.app));
    }

    #[test]
    fn pr_list_read_without_a_github_remote_is_unavailable_and_silent() {
        let mut rig = rig(Some(FAKE_GH), FakePullRequestSource::new());
        ask_list(&mut rig, "feat/a");
        let asked = remote_lists(&mut rig);
        answer_remotes(
            &mut rig.app,
            asked[0],
            vec![GitRemote {
                name: "origin".into(),
                url: "git@example.com:o/r.git".into(),
            }],
        );
        assert!(matches!(
            view_list(&rig.app),
            PrList::Unavailable { last: None }
        ));
        assert!(rig.source.list_calls().is_empty());
        assert!(nothing_was_reported(&rig.app));
    }

    /// The pending `RemoteList` of the list read: its `req` and `seq`.
    fn list_pending(app: &App) -> Option<(u64, u64)> {
        app.pending_ops.iter().find_map(|(req, op)| match op {
            crate::shell::daemon_sync::PendingOp::PrListRemotes { seq, .. } => Some((*req, *seq)),
            _ => None,
        })
    }

    // The 10-second bound covers the remotes: unanswered, the read ends passing and its entry goes.
    #[test]
    fn pr_list_read_remotes_timeout_ends_the_read_and_frees_the_entry() {
        let mut rig = rig(Some(FAKE_GH), FakePullRequestSource::new());
        ask_list(&mut rig, "feat/a");
        let (req, seq) = list_pending(&rig.app).expect("the list read waits for the remotes");
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert!(list_pending(&rig.app).is_none(), "the entry does not leak");
        assert!(matches!(view_list(&rig.app), PrList::Unavailable { .. }));
        assert!(nothing_was_reported(&rig.app));
    }

    // A disconnect while the remotes are pending ends the read: the view must not stay Loading.
    #[test]
    fn pr_list_read_a_disconnect_ends_the_read_and_raises_no_notice() {
        let mut rig = rig(Some(FAKE_GH), FakePullRequestSource::new());
        ask_list(&mut rig, "feat/a");
        let (req, _) = list_pending(&rig.app).expect("pending");
        let daemon = *rig.app.pending_daemon.get(&req).expect("bound to a daemon");
        let work = shell::daemon_sync::on_disconnected(&mut rig.app, daemon);
        settle(&mut rig.app, work);
        assert!(list_pending(&rig.app).is_none());
        assert!(matches!(view_list(&rig.app), PrList::Unavailable { .. }));
        assert!(nothing_was_reported(&rig.app));
    }

    // ---- Feature 775, T027: the list follows the status triggers --------------------------------

    /// The interval's tick, as the shell receives it. The timers it returns are never run: the
    /// remotes' bound would end the read it bounds.
    fn tick(rig: &mut Rig) {
        let _ = update_inner(&mut rig.app, Message::PrStatus(Msg::Tick));
    }

    /// Open the view and complete its first list read, so a refresh finds it idle.
    fn listed_once(rig: &mut Rig) {
        ask_list(rig, "feat/a");
        let (req, _) = list_pending(&rig.app).expect("pending");
        answer_remotes(&mut rig.app, req, github_remote());
        assert!(matches!(view_list(&rig.app), PrList::Ready { .. }));
        let _ = sent(rig);
    }

    // SC-004: a status trigger while a view is open asks the list again.
    #[test]
    fn pr_list_refresh_a_trigger_with_a_view_open_requests_the_list() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_list_answer(vec![branch_pr(1)])
                .with_list_answer(vec![branch_pr(1), branch_pr(2)]),
        );
        listed_once(&mut rig);
        tick(&mut rig);
        let (req, _) = list_pending(&rig.app).expect("the trigger asked for the list");
        answer_remotes(&mut rig.app, req, github_remote());
        assert!(matches!(
            view_list(&rig.app),
            PrList::Ready { prs, .. } if prs.len() == 2
        ));
        assert!(nothing_was_reported(&rig.app));
    }

    #[test]
    fn pr_list_refresh_a_trigger_with_no_view_open_asks_nothing_of_the_list() {
        let mut rig = rig(Some(FAKE_GH), FakePullRequestSource::new());
        tick(&mut rig);
        assert!(list_pending(&rig.app).is_none());
        assert!(rig.source.list_calls().is_empty());
    }

    #[test]
    fn pr_list_refresh_is_skipped_while_paused_and_while_a_read_is_under_way() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![branch_pr(1)]),
        );
        listed_once(&mut rig);
        let far = u64::MAX;
        rig.app.core.pr_status.pause_until = Some(far);
        tick(&mut rig);
        assert!(list_pending(&rig.app).is_none(), "paused: no read");
        assert!(matches!(view_list(&rig.app), PrList::Ready { .. }));

        rig.app.core.pr_status.pause_until = None;
        tick(&mut rig);
        let first = list_pending(&rig.app).expect("asked");
        tick(&mut rig);
        assert_eq!(
            list_pending(&rig.app),
            Some(first),
            "a read under way is not asked again"
        );
        assert!(matches!(
            view_list(&rig.app),
            PrList::Loading { again: false, .. }
        ));
    }

    // R3: Unavailable is re-asked on a trigger, and on nothing else.
    #[test]
    fn pr_list_refresh_unavailable_is_asked_again_only_on_a_trigger() {
        let mut rig = rig(None, FakePullRequestSource::new());
        ask_list(&mut rig, "feat/a");
        let (req, seq) = list_pending(&rig.app).expect("pending");
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert!(matches!(view_list(&rig.app), PrList::Unavailable { .. }));
        assert!(list_pending(&rig.app).is_none(), "nothing asks by itself");
        tick(&mut rig);
        assert!(list_pending(&rig.app).is_some(), "the trigger asks again");
    }

    // R3: a rate limit named by a list read pauses the status reading.
    #[test]
    fn pr_list_refresh_a_rate_limited_list_read_sets_the_pause() {
        let mut rig = rig(Some(FAKE_GH), FakePullRequestSource::new());
        ask_list(&mut rig, "feat/a");
        let seq = list_pending(&rig.app).expect("pending").1;
        let _ = crate::shell::changes::update(
            &mut rig.app,
            ChangesMsg::PullRequestsRead {
                seq,
                read_at: 1,
                result: Err(micold_core::pull_request::ReadingFailure::RateLimited {
                    until: 5_000,
                }),
            },
        );
        assert_eq!(rig.app.core.pr_status.pause_until, Some(5_000));
    }

    // ---- Feature 775, T023: a pull request's diff read, through the shell ----------------------

    const TWO_FILE_DIFF: &str =
        "diff --git a/x.rs b/x.rs\n--- a/x.rs\n+++ b/x.rs\n@@ -1 +1 @@\n-a\n+b\n\
diff --git a/y.rs b/y.rs\nnew file mode 100644\n--- /dev/null\n+++ b/y.rs\n@@ -0,0 +1 @@\n+c\n";

    /// The view with pull request 1 listed and chosen: the diff read is waiting for the remotes.
    fn choose_listed_pr(rig: &mut Rig) {
        ask_list(rig, "feat/a");
        let (req, _) = list_pending(&rig.app).expect("pending");
        answer_remotes(&mut rig.app, req, github_remote());
        // The timer the choice returns is never run: it would end the read it bounds.
        let _ = crate::shell::changes::update(&mut rig.app, ChangesMsg::SourceChosen(Some(1)));
    }

    /// The pending `RemoteList` of the diff read: its `req` and `seq`.
    fn diff_pending(app: &App) -> Option<(u64, u64)> {
        app.pending_ops.iter().find_map(|(req, op)| match op {
            crate::shell::daemon_sync::PendingOp::PrDiffRemotes { seq, .. } => Some((*req, *seq)),
            _ => None,
        })
    }

    fn view_diff(app: &App) -> PrDiffLoad {
        match &app.core.changes.open.as_ref().expect("open").source {
            Source::PullRequest(pr) => pr.diff.clone(),
            Source::Changes => panic!("a pull request is chosen"),
        }
    }

    // FR-004: choosing a pull request reads its diff once, from its own repository, and lists it.
    #[test]
    fn pr_diff_read_runs_the_source_and_lists_the_files() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_list_answer(vec![branch_pr(1)])
                .with_diff_answer(TWO_FILE_DIFF.to_string()),
        );
        choose_listed_pr(&mut rig);
        let (req, _) = diff_pending(&rig.app).expect("the diff read waits for the remotes");
        answer_remotes(&mut rig.app, req, github_remote());
        assert_eq!(rig.source.diff_calls(), vec![("o/r".to_string(), 1)]);
        assert!(matches!(
            view_diff(&rig.app),
            PrDiffLoad::Ready { files, .. } if files.len() == 2
        ));
        assert!(diff_pending(&rig.app).is_none());
        assert!(nothing_was_reported(&rig.app));
    }

    // A failed read ends as Failed and raises no notice.
    #[test]
    fn pr_diff_read_a_failure_raises_no_notice() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new()
                .with_list_answer(vec![branch_pr(1)])
                .with_diff_failure(DiffReadFailure::TooLarge),
        );
        choose_listed_pr(&mut rig);
        let (req, _) = diff_pending(&rig.app).expect("pending");
        answer_remotes(&mut rig.app, req, github_remote());
        assert_eq!(view_diff(&rig.app), PrDiffLoad::TooLarge);
        assert!(nothing_was_reported(&rig.app));
    }

    // The remotes unanswered: the timer ends the read and frees the entry.
    #[test]
    fn pr_diff_read_remotes_timeout_ends_the_read() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![branch_pr(1)]),
        );
        choose_listed_pr(&mut rig);
        let (req, seq) = diff_pending(&rig.app).expect("pending");
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert!(diff_pending(&rig.app).is_none());
        assert_eq!(view_diff(&rig.app), PrDiffLoad::Failed);
        assert!(rig.source.diff_calls().is_empty());
        assert!(nothing_was_reported(&rig.app));
    }

    // A disconnect while the remotes are pending ends the read: the view must not stay Loading.
    #[test]
    fn pr_diff_read_a_disconnect_ends_the_read_and_raises_no_notice() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![branch_pr(1)]),
        );
        choose_listed_pr(&mut rig);
        let (req, _) = diff_pending(&rig.app).expect("pending");
        let daemon = *rig.app.pending_daemon.get(&req).expect("bound to a daemon");
        let work = shell::daemon_sync::on_disconnected(&mut rig.app, daemon);
        settle(&mut rig.app, work);
        assert!(diff_pending(&rig.app).is_none());
        assert_eq!(view_diff(&rig.app), PrDiffLoad::Failed);
        assert!(nothing_was_reported(&rig.app));
    }

    // After the answer the timer is a no-op: the read it bounded is under way and keeps its result.
    #[test]
    fn pr_list_read_a_timeout_after_the_remotes_answered_does_nothing() {
        let mut rig = rig(
            Some(FAKE_GH),
            FakePullRequestSource::new().with_list_answer(vec![branch_pr(1)]),
        );
        ask_list(&mut rig, "feat/a");
        let (req, seq) = list_pending(&rig.app).expect("pending");
        answer_remotes(&mut rig.app, req, github_remote());
        let work = update_inner(
            &mut rig.app,
            Message::PrStatus(Msg::RemotesTimedOut { seq, req }),
        );
        settle(&mut rig.app, work);
        assert!(matches!(
            view_list(&rig.app),
            PrList::Ready { prs, .. } if prs.len() == 1
        ));
    }

    // FR-014: with the setting off nothing is asked of the daemon or of GitHub.
    #[test]
    fn pr_list_read_with_the_setting_off_sends_nothing() {
        let mut rig = rig(Some(FAKE_GH), FakePullRequestSource::new());
        let _ = shell::pr_status::enabled_changed(&mut rig.app, false);
        let _ = sent(&mut rig);
        ask_list(&mut rig, "feat/a");
        assert!(remote_lists(&mut rig).is_empty());
        assert!(rig.source.list_calls().is_empty());
        assert!(matches!(view_list(&rig.app), PrList::Absent));
    }
}

/// Feature 613, T035 (S3, W5.4): toggling one kind changes only that kind in the draft, and the
/// save sends the whole draft value.
#[test]
fn toggling_a_kind_changes_only_it_and_saving_sends_the_whole_value() {
    use micold_core::attention::{NotificationKind, NotificationKinds};
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::NotificationKindToggled(
            NotificationKind::TurnFinished,
            true,
        )),
    );
    let mut expected = NotificationKinds::default();
    expected.set(NotificationKind::TurnFinished, true);
    assert_eq!(
        app.core
            .settings
            .settings_draft
            .as_ref()
            .expect("open")
            .environment
            .notification_kinds,
        expected
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    let sent: Vec<ClientMsg> = drain(&mut rx);
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            notification_kinds, ..
        } => Some(*notification_kinds),
        _ => None,
    });
    assert_eq!(told, Some(Some(expected)), "{sent:?}");
}

/// Feature 613, T060 (S6, US2.11): editing the threshold touches only its text; saving "20"
/// sends 20; a bad value sends nothing and names the field.
#[test]
fn the_threshold_is_edited_as_text_and_saved_as_a_number() {
    let (mut app, mut rx) = connected_app();
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: quiet_settings(),
        },
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let draft = |app: &App| {
        app.core
            .settings
            .settings_draft
            .clone()
            .expect("the page is open")
    };
    let before = draft(&app);
    assert_eq!(before.environment.long_task_threshold_secs, "60");
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::LongTaskThresholdChanged("9".into())),
    );
    let mut expected = before.environment.clone();
    expected.long_task_threshold_secs = "9".into();
    assert_eq!(draft(&app).environment, expected);

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    let sent: Vec<ClientMsg> = drain(&mut rx);
    assert!(
        !sent
            .iter()
            .any(|m| matches!(m, ClientMsg::SettingsSet { .. })),
        "a refused save sends nothing: {sent:?}"
    );

    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::LongTaskThresholdChanged("20".into())),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));
    let sent: Vec<ClientMsg> = drain(&mut rx);
    let told = sent.iter().find_map(|msg| match msg {
        ClientMsg::SettingsSet {
            long_task_threshold_secs,
            ..
        } => Some(*long_task_threshold_secs),
        _ => None,
    });
    assert_eq!(told, Some(Some(20)), "{sent:?}");
}

/// Feature 613, T035/T060: a `SettingsChanged` from another window updates the stored values.
#[test]
fn another_windows_kinds_and_threshold_reach_the_stored_values() {
    use micold_core::attention::NotificationKinds;
    let mut app = base_app();
    let kinds = NotificationKinds {
        turn_finished: true,
        needs_permission: false,
        ..NotificationKinds::default()
    };
    feed(
        &mut app,
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                notification_kinds: kinds,
                long_task_threshold_secs: 15,
                ..quiet_settings()
            },
        },
    );
    assert_eq!(app.core.session.notification_kinds, kinds);
    assert_eq!(app.core.session.long_task_threshold_secs, 15);
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let env = &app
        .core
        .settings
        .settings_draft
        .as_ref()
        .expect("open")
        .environment;
    assert_eq!(env.notification_kinds, kinds);
    assert_eq!(env.long_task_threshold_secs, "15");
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
            micold_core::daemons::DaemonId(1),
            DaemonMsg::AttentionGranted {
                session,
                seq: 1,
                kind: micold_core::attention::NotificationKind::LongTaskFinished,
            },
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
            micold_core::daemons::DaemonId(1),
            DaemonMsg::AttentionGranted {
                session,
                seq: 1,
                kind: micold_core::attention::NotificationKind::LongTaskFinished,
            },
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
            micold_core::daemons::DaemonId(1),
            DaemonMsg::RevealSession {
                project: folder.path().to_path_buf(),
                session,
                activation: None,
            },
        );

        assert_eq!(action_order(work), ["raise", "message"]);
    }
}

// --- `039` BUG-568: a window that does not hold its project has no session in view ------------

/// A focused, connected window on `/repo/demo` with one of its sessions selected, its outbox
/// drained: the window reports that session in view, as a precondition the tests below check.
fn focused_on_a_session_of(
    project: &Path,
) -> (
    App,
    iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    SessionId,
) {
    let session = SessionId::new();
    let mut app = app_on_project(project);
    app.window_focused = true;
    let (tx, mut rx) = iced::futures::channel::mpsc::unbounded();
    let _ = update(
        &mut app,
        Message::Connection(ConnectionMsg::Connected {
            outbox: micold_client::daemon::Outbox::new(tx),
            catalog: snapshot_with(
                &project.to_string_lossy(),
                vec![summary(session, "s", WireLifecycle::Running)],
            ),
            settings: quiet_settings(),
            plan_usage: None,
        }),
    );
    let _ = update(
        &mut app,
        Message::Session(micold_client::features::session::Msg::Selected(session)),
    );
    assert_eq!(
        window_views(&mut rx).last(),
        Some(&(true, Some(session))),
        "fixture check: a focused window that holds its project has its selected session in view"
    );
    (app, rx, session)
}

/// Every `WindowView` on the wire since the last drain, as `(focused, in_view)`.
fn window_views(rx: &mut SentRx) -> Vec<(bool, Option<SessionId>)> {
    sent_matching(rx, |msg| match msg {
        ClientMsg::WindowView { focused, in_view } => Some((focused, in_view)),
        _ => None,
    })
}

/// The reported defect (#568): another window takes the project over, and this one keeps telling
/// the service it has the selected session in view, so that session's attention events are taken
/// as watched and its unread mark is cleared. The window does not show it any more.
#[test]
fn a_window_displaced_from_its_project_reports_no_session_in_view() {
    let project = PathBuf::from("/repo/demo");
    let (mut app, mut rx, _) = focused_on_a_session_of(&project);

    let _ = update(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::Displaced {
            project: project.clone(),
            by: other_window(),
        })),
    );

    assert_eq!(
        window_views(&mut rx),
        vec![(true, None)],
        "a displaced window still has focus, but no session in view"
    );
}

/// Refused the project on a (re)attach (`010` BUG-023): the same read-only state, the same report.
#[test]
fn a_window_refused_its_project_reports_no_session_in_view() {
    let project = PathBuf::from("/repo/demo");
    let (mut app, mut rx, _) = focused_on_a_session_of(&project);

    let _ = update(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::Refused {
            reason: micold_core::protocol::messages::RefusalReason::ProjectBusy {
                project: project.clone(),
                holder: other_window(),
                since_secs: 3,
            },
        })),
    );

    assert_eq!(window_views(&mut rx), vec![(true, None)]);
}

/// Taking the project back brings the selected session back into view.
#[test]
fn a_window_that_takes_its_project_back_reports_the_session_in_view_again() {
    let project = PathBuf::from("/repo/demo");
    let (mut app, mut rx, session) = focused_on_a_session_of(&project);
    let _ = update(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::Displaced {
            project: project.clone(),
            by: other_window(),
        })),
    );
    let _ = window_views(&mut rx);

    let _ = update(
        &mut app,
        Message::Connection(ConnectionMsg::Event(DaemonMsg::Attached {
            project: project.clone(),
            sessions: Vec::new(),
        })),
    );

    assert_eq!(window_views(&mut rx), vec![(true, Some(session))]);
}

/// Feature 040, story 2 (contract pull-request-ui §4): **Open pull request** hands the address
/// GitHub reported to the link opener, and only when it is a GitHub address (FR-014; U116, U117).
mod pr_status_open {
    use super::*;
    use crate::shell::link_opener::LinkOpener;
    use micold_client::features::OpenFailure;
    use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
    use micold_core::worktree::WorktreeStatus::Valid;
    use std::sync::Mutex;

    #[derive(Default)]
    pub(super) struct Recording {
        pub(super) opened: Mutex<Vec<String>>,
        pub(super) fail: bool,
    }

    impl LinkOpener for Recording {
        fn open(&self, target: &str) -> Result<(), OpenFailure> {
            self.opened.lock().unwrap().push(target.to_string());
            if self.fail {
                Err(OpenFailure::LaunchFailed("no launcher".into()))
            } else {
                Ok(())
            }
        }

        fn reveal(&self, _path: &std::path::Path) -> Result<(), OpenFailure> {
            Ok(())
        }
    }

    fn status(url: &str) -> PullRequestStatus {
        PullRequestStatus {
            number: 7,
            title: "A title".into(),
            url: url.into(),
            state: PrState::Open {
                checks: CheckStatus::Failing,
            },
            review: ReviewState::None,
            head: "abc".into(),
            conflict: false,
        }
    }

    /// A window on `/repo/demo` whose `feat-a` row holds a pull request at `url`.
    fn app_holding(url: &str, opener: &Arc<Recording>) -> App {
        let mut app = app_on_demo(&[("feat-a", Valid, true)]);
        app.caps = app.caps.clone().with_link_opener(opener.clone());
        app.core
            .pr_status
            .statuses
            .insert("feat/feat-a".to_string(), status(url));
        app
    }

    fn open(app: &mut App, dir: &str) -> Vec<Message> {
        messages(update_inner(
            app,
            Message::Worktree(WorktreeMsg::PullRequestOpenRequested(dir.to_string())),
        ))
    }

    fn opened(opener: &Recording) -> Vec<String> {
        opener.opened.lock().unwrap().clone()
    }

    #[test]
    fn pr_status_open_hands_the_stored_address_to_the_opener_once() {
        let url = "https://github.com/o/r/pull/7";
        let opener = Arc::new(Recording::default());
        let mut app = app_holding(url, &opener);

        let _ = open(&mut app, "feat-a");

        assert_eq!(opened(&opener), vec![url.to_string()]);
    }

    #[test]
    fn pr_status_open_opens_nothing_for_an_address_that_is_not_github() {
        for url in [
            "http://github.com/o/r/pull/7",
            "https://github.com.evil.example/o/r/pull/7",
            "https://evilgithub.com/o/r/pull/7",
            "https://example.com/o/r/pull/7",
            "https://github.com",
            "HTTPS://GITHUB.COM/o/r/pull/7",
            " https://github.com/o/r/pull/7",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "",
        ] {
            let opener = Arc::new(Recording::default());
            let mut app = app_holding(url, &opener);

            let messages = open(&mut app, "feat-a");

            assert!(opened(&opener).is_empty(), "{url:?} must not be opened");
            assert!(messages.is_empty(), "{url:?} must raise nothing");
        }
    }

    #[test]
    fn pr_status_open_opens_nothing_when_the_row_lost_its_status() {
        let opener = Arc::new(Recording::default());
        let mut app = app_holding("https://github.com/o/r/pull/7", &opener);
        app.core.pr_status.statuses.clear();

        let _ = open(&mut app, "feat-a");
        let _ = open(&mut app, "no-such-row");

        assert!(opened(&opener).is_empty());
    }

    #[test]
    fn pr_status_open_opens_the_address_of_the_rows_own_branch() {
        let opener = Arc::new(Recording::default());
        let mut app = app_holding("https://github.com/o/r/pull/7", &opener);
        app.core.pr_status.statuses.insert(
            "feat/other".to_string(),
            status("https://github.com/o/r/pull/99"),
        );

        let _ = open(&mut app, "feat-a");

        assert_eq!(opened(&opener), vec!["https://github.com/o/r/pull/7"]);
    }

    /// FR-014: selection, sessions and the sidebar are what they were.
    #[test]
    fn pr_status_open_changes_no_application_state() {
        let opener = Arc::new(Recording::default());
        let mut app = app_holding("https://github.com/o/r/pull/7", &opener);
        let before = format!(
            "{:?}|{:?}|{:?}|{:?}",
            app.core.session.active,
            app.core.workspace.sessions,
            app.core.sidebar,
            app.core.worktree.worktrees
        );

        let _ = open(&mut app, "feat-a");

        assert_eq!(opened(&opener).len(), 1, "fixture check: it did open");
        let after = format!(
            "{:?}|{:?}|{:?}|{:?}",
            app.core.session.active,
            app.core.workspace.sessions,
            app.core.sidebar,
            app.core.worktree.worktrees
        );
        assert_eq!(before, after);
    }

    /// The opener's failure goes the way every opened link's does (feature 031).
    #[test]
    fn pr_status_open_reports_an_opener_failure_as_a_link_is() {
        let url = "https://github.com/o/r/pull/7";
        let opener = Arc::new(Recording {
            fail: true,
            ..Recording::default()
        });
        let mut app = app_holding(url, &opener);

        let messages = open(&mut app, "feat-a");

        assert_eq!(
            messages,
            vec![Message::Session(
                micold_client::features::session::Msg::LinkOpenFinished {
                    address: url.to_string(),
                    result: Err(OpenFailure::LaunchFailed("no launcher".into())),
                }
            )]
        );
    }
}

// ---- Feature 484: panes, client state (T008) ----

mod panes {
    use super::*;
    use micold_core::pane_layout::PaneLayout;
    use micold_core::project::{Availability, Project};
    use micold_core::protocol::messages::{SessionProcess, TerminalRef};
    use micold_core::session::{AiCli, Session, SessionLocation};

    fn t(session: SessionId, process: SessionProcess) -> TerminalRef {
        TerminalRef { session, process }
    }

    /// An app with a connected daemon and one project holding `n` sessions, the first selected.
    fn app_with_sessions(
        n: usize,
    ) -> (
        App,
        Vec<SessionId>,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let (mut app, rx) = connected_app();
        let project = PathBuf::from("/repo/panes");
        app.core.workspace.projects.push(Project::new(
            project.clone(),
            true,
            Availability::Available,
        ));
        app.core.workspace.active = Some(project.clone());
        let sessions: Vec<Session> = (0..n)
            .map(|_| Session::start_new(SessionLocation::Default, AiCli::ClaudeCode))
            .collect();
        let ids: Vec<SessionId> = sessions.iter().map(|s| s.id).collect();
        app.core.workspace.sessions.insert(project, sessions);
        app.core.session.active = Some(ids[0]);
        crate::shell::panes::sync(&mut app);
        (app, ids, rx)
    }

    fn layout_of(app: &App) -> PaneLayout {
        crate::shell::panes::layout(app).expect("a layout").clone()
    }

    /// A frame for terminal X changes only X's grid (FR-003).
    #[test]
    fn a_frame_updates_only_its_own_terminals_grid() {
        let (mut app, ids, _rx) = app_with_sessions(1);
        let s = ids[0];
        let shell = SessionProcess::Shell(micold_core::session::ShellInstanceId(1));
        let _ = crate::shell::daemon_sync::on_grid_frame(
            &mut app,
            frame_of(s, SessionProcess::Primary, 100, 10),
        );
        let _ = crate::shell::daemon_sync::on_grid_frame(&mut app, frame_of(s, shell, 5000, 10));
        assert_eq!(
            app.grids[&t(s, SessionProcess::Primary)].viewport_top().0,
            100
        );
        assert_eq!(app.grids[&t(s, shell)].viewport_top().0, 5000);
    }

    /// One pane behaves as today (FR-017): nothing about panes goes on the wire.
    #[test]
    fn a_project_with_one_pane_sends_no_viewed_terminals() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let _ = update(&mut app, Message::Session(SessionMsg::Selected(ids[0])));
        crate::shell::panes::sync(&mut app);
        assert_eq!(layout_of(&app).len(), 1);
        assert!(!drain(&mut rx)
            .iter()
            .any(|m| matches!(m, ClientMsg::SetViewedTerminals { .. })));
    }

    /// Selecting a terminal that is already shown focuses its pane instead of duplicating it.
    #[test]
    fn selecting_a_terminal_calls_show_or_focus() {
        let (mut app, ids, _rx) = app_with_sessions(2);
        let first = t(ids[0], SessionProcess::Primary);
        let second = t(ids[1], SessionProcess::Primary);
        // Two panes: the first shows session 0, the second session 1.
        let p0 = layout_of(&app).focused();
        crate::shell::panes::split_pane(
            &mut app,
            p0,
            micold_core::pane_layout::Axis::Vertical,
            (1000.0, 600.0),
            (10.0, 10.0),
        )
        .unwrap();
        crate::shell::panes::sync(&mut app);
        assert_eq!(layout_of(&app).terminals(), vec![first, second]);
        // Select session 0 again: its pane takes the focus, and nothing is duplicated.
        let _ = update(&mut app, Message::Session(SessionMsg::Selected(ids[0])));
        let l = layout_of(&app);
        assert_eq!(l.focused_terminal(), Some(first));
        assert_eq!(l.terminals(), vec![first, second]);
        // Selecting session 1 focuses the other pane.
        let _ = update(&mut app, Message::Session(SessionMsg::Selected(ids[1])));
        assert_eq!(layout_of(&app).focused_terminal(), Some(second));
    }

    /// Input and the viewed set follow the panes (FR-010); focus moves the active session.
    #[test]
    fn input_goes_to_the_focused_panes_terminal_and_the_daemon_streams_both() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = t(ids[0], SessionProcess::Primary);
        let second = t(ids[1], SessionProcess::Primary);
        let p0 = layout_of(&app).focused();
        crate::shell::panes::split_pane(
            &mut app,
            p0,
            micold_core::pane_layout::Axis::Vertical,
            (1000.0, 600.0),
            (10.0, 10.0),
        )
        .unwrap();
        let _ = drain(&mut rx);
        crate::shell::panes::sync(&mut app);
        let sent = drain(&mut rx);
        assert!(
            sent.iter().any(|m| matches!(m, ClientMsg::SetViewedTerminals { terminals, .. } if *terminals == vec![first, second])),
            "both terminals are streamed: {sent:?}"
        );
        // Focus the second pane: the active session follows it, and a key reaches it alone.
        let second_pane = layout_of(&app).pane_showing(second).unwrap();
        crate::shell::panes::focus_pane(&mut app, second_pane);
        assert_eq!(app.core.session.active, Some(ids[1]));
        let _ = update(
            &mut app,
            Message::Session(SessionMsg::TerminalBytes(b"x".to_vec())),
        );
        let inputs: Vec<_> = drain(&mut rx)
            .into_iter()
            .filter_map(|m| match m {
                ClientMsg::SessionInput {
                    session, process, ..
                } => Some((session, process)),
                _ => None,
            })
            .collect();
        assert_eq!(inputs, vec![(ids[1], Some(SessionProcess::Primary))]);
    }

    /// A project's panes never show in another project (FR-016).
    #[test]
    fn layouts_are_per_project() {
        let (mut app, ids, _rx) = app_with_sessions(1);
        let other = PathBuf::from("/repo/other");
        app.core.workspace.projects.push(Project::new(
            other.clone(),
            true,
            Availability::Available,
        ));
        app.core.workspace.active = Some(other.clone());
        app.core.session.active = None;
        crate::shell::panes::sync(&mut app);
        assert!(layout_of(&app).terminals().is_empty());
        assert!(!layout_of(&app)
            .terminals()
            .contains(&t(ids[0], SessionProcess::Primary)));
    }

    /// A removed session empties its pane and never collapses the tree.
    #[test]
    fn a_session_that_is_gone_empties_its_pane() {
        let (mut app, ids, _rx) = app_with_sessions(2);
        let project = app.core.workspace.active.clone().unwrap();
        let p0 = layout_of(&app).focused();
        crate::shell::panes::split_pane(
            &mut app,
            p0,
            micold_core::pane_layout::Axis::Vertical,
            (1000.0, 600.0),
            (10.0, 10.0),
        )
        .unwrap();
        app.core
            .workspace
            .sessions
            .get_mut(&project)
            .unwrap()
            .retain(|s| s.id != ids[1]);
        crate::shell::panes::sync(&mut app);
        let l = layout_of(&app);
        assert_eq!(l.len(), 2);
        assert_eq!(l.terminals(), vec![t(ids[0], SessionProcess::Primary)]);
    }

    // ---- M1 (T012, T015, T018): splitting, focus, per-pane sizes -----------------------------

    use micold_client::features::session::PaneMsg;
    use micold_core::pane_layout::{Axis, PaneId};

    fn pane_msg(app: &mut App, m: PaneMsg) {
        let _ = update(app, Message::Session(SessionMsg::Pane(m)));
    }

    fn size_pane(app: &mut App, pane: PaneId, cols: u16, rows: u16) {
        pane_msg(app, PaneMsg::Resized { pane, cols, rows });
    }

    /// Split the focused pane (a pane big enough to split) and return the new pane's id.
    fn split_focused(app: &mut App, axis: Axis) -> PaneId {
        let p = layout_of(app).focused();
        size_pane(app, p, 200, 60);
        pane_msg(app, PaneMsg::Split(p, axis));
        layout_of(app).focused()
    }

    /// SC-001: the split button, then one choice in the empty pane's picker = 2 interactions.
    #[test]
    fn a_split_then_one_choice_shows_the_terminal() {
        let (mut app, ids, _rx) = app_with_sessions(1);
        let shell = t(
            ids[0],
            SessionProcess::Shell(micold_core::session::ShellInstanceId(1)),
        );
        app.core.workspace.sessions.values_mut().next().unwrap()[0]
            .shells
            .push(micold_core::session::ShellInstance {
                id: micold_core::session::ShellInstanceId(1),
                lifecycle: micold_core::session::ShellLifecycle::Running,
            });
        // The shell is the one terminal no pane shows, so the new pane opens on it (FR-004).
        let new = split_focused(&mut app, Axis::Vertical);
        assert_eq!(layout_of(&app).pane(new).unwrap().terminal(), Some(shell));
        // With nothing left to show, the next split is an empty pane with a picker, and one
        // choice (Show) fills it.
        let empty = split_focused(&mut app, Axis::Horizontal);
        assert_eq!(layout_of(&app).pane(empty).unwrap().terminal(), None);
        pane_msg(&mut app, PaneMsg::Show(empty, shell));
        assert_eq!(layout_of(&app).len(), 3);
        assert_eq!(
            layout_of(&app).focused(),
            new,
            "a terminal shown elsewhere is focused, not duplicated"
        );
    }

    /// FR-004: a new pane opens on a terminal not shown elsewhere.
    #[test]
    fn a_new_pane_opens_on_an_unshown_terminal() {
        let (mut app, ids, _rx) = app_with_sessions(2);
        let new = split_focused(&mut app, Axis::Vertical);
        assert_eq!(
            layout_of(&app).pane(new).unwrap().terminal(),
            Some(t(ids[1], SessionProcess::Primary))
        );
        assert_eq!(
            app.core.session.active,
            Some(ids[1]),
            "the selection follows the focus"
        );
    }

    /// FR-001: a 7th pane is refused, with a reason the view shows.
    #[test]
    fn a_seventh_split_shows_the_refusal() {
        let (mut app, _ids, _rx) = app_with_sessions(1);
        for _ in 0..5 {
            split_focused(&mut app, Axis::Vertical);
            let f = layout_of(&app).focused();
            size_pane(&mut app, f, 200, 60);
        }
        assert_eq!(layout_of(&app).len(), 6);
        let f = layout_of(&app).focused();
        pane_msg(&mut app, PaneMsg::Split(f, Axis::Vertical));
        assert_eq!(layout_of(&app).len(), 6);
        assert!(
            app.pane_refusal.is_some_and(|r| r.contains('6')),
            "{:?}",
            app.pane_refusal
        );
        pane_msg(&mut app, PaneMsg::FocusPane(f));
        assert_eq!(
            app.pane_refusal, None,
            "the message clears on the next pane action"
        );
    }

    /// FR-009: the split chords split the focused pane and focus the new one; the focus chords
    /// move between panes and stay at the edge; a refused split shows its reason.
    #[test]
    fn pane_chords_split_and_move_focus() {
        use micold_client::keymap::PaneAction;
        use micold_core::pane_layout::Direction;
        let (mut app, _ids, _rx) = app_with_sessions(1);
        let first = layout_of(&app).focused();
        size_pane(&mut app, first, 200, 60);
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::SplitVertical));
        let second = layout_of(&app).focused();
        assert_ne!(second, first);
        size_pane(&mut app, second, 100, 60);
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::SplitHorizontal));
        let third = layout_of(&app).focused();
        assert_eq!(layout_of(&app).len(), 3);
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::Focus(Direction::Up)));
        assert_eq!(layout_of(&app).focused(), second);
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::Focus(Direction::Up)));
        assert_eq!(layout_of(&app).focused(), second, "no pane above: stays");
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::Focus(Direction::Left)));
        assert_eq!(layout_of(&app).focused(), first);
        pane_msg(
            &mut app,
            PaneMsg::Chord(PaneAction::Focus(Direction::Right)),
        );
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::Focus(Direction::Down)));
        assert_eq!(layout_of(&app).focused(), third);
        // A pane too small to split shows why.
        size_pane(&mut app, third, 4, 2);
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::SplitVertical));
        assert_eq!(layout_of(&app).len(), 3);
        assert!(app.pane_refusal.is_some());
    }

    /// A tab-strip or sidebar choice of a terminal already shown focuses that pane.
    #[test]
    fn selecting_a_shown_terminal_focuses_its_pane() {
        let (mut app, ids, _rx) = app_with_sessions(2);
        let second = split_focused(&mut app, Axis::Vertical);
        let first = layout_of(&app).panes()[0].id();
        assert_ne!(first, second);
        let _ = update(&mut app, Message::Session(SessionMsg::Selected(ids[0])));
        let l = layout_of(&app);
        assert_eq!(l.len(), 2);
        assert_eq!(l.focused(), first);
        assert_eq!(l.terminals().len(), 2, "no terminal is shown twice");
    }

    /// An exited terminal says so in its own pane header; the other pane is unaffected.
    #[test]
    fn an_exited_terminal_shows_its_state_in_its_pane_only() {
        let (mut app, ids, _rx) = app_with_sessions(1);
        let inst = micold_core::session::ShellInstanceId(1);
        app.core.workspace.sessions.values_mut().next().unwrap()[0]
            .shells
            .push(micold_core::session::ShellInstance {
                id: inst,
                lifecycle: micold_core::session::ShellLifecycle::Exited,
            });
        let primary = t(ids[0], SessionProcess::Primary);
        let shell = t(ids[0], SessionProcess::Shell(inst));
        assert_eq!(
            micold_client::ui::panes::terminal_status(&app.core, shell),
            "exited"
        );
        assert_ne!(
            micold_client::ui::panes::terminal_status(&app.core, primary),
            "exited"
        );
    }

    /// T015: a key reaches only the focused pane's terminal, for 2 to 6 panes.
    #[test]
    fn a_key_reaches_only_the_focused_panes_terminal() {
        for n in 2..=6usize {
            let (mut app, ids, mut rx) = app_with_sessions(n);
            for _ in 1..n {
                let f = layout_of(&app).focused();
                size_pane(&mut app, f, 200, 60);
                split_focused(&mut app, Axis::Vertical);
            }
            let l = layout_of(&app);
            assert_eq!(l.len(), n);
            for pane in l.panes() {
                pane_msg(&mut app, PaneMsg::FocusPane(pane.id()));
                let _ = drain(&mut rx);
                let _ = update(
                    &mut app,
                    Message::Session(SessionMsg::TerminalBytes(b"x".to_vec())),
                );
                let inputs: Vec<_> = drain(&mut rx)
                    .into_iter()
                    .filter_map(|m| match m {
                        ClientMsg::SessionInput {
                            session, process, ..
                        } => Some((session, process)),
                        _ => None,
                    })
                    .collect();
                let want = pane.terminal().unwrap();
                assert_eq!(inputs, vec![(want.session, Some(want.process))], "n={n}");
            }
            let _ = ids;
            assert_eq!(
                layout_of(&app)
                    .panes()
                    .iter()
                    .filter(|p| p.id() == layout_of(&app).focused())
                    .count(),
                1
            );
        }
    }

    /// Focus lives in the layout: a dialog, menu or text field taking the keyboard and giving it back
    /// leaves the same pane focused, and the next key still reaches that pane's terminal (US2-5).
    #[test]
    fn focus_returns_to_the_same_pane() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        pane_msg(&mut app, PaneMsg::FocusPane(first));
        let _ = update(
            &mut app,
            Message::Session(SessionMsg::TerminalFocusReleased),
        );
        assert_eq!(layout_of(&app).focused(), first);
        let _ = update(&mut app, Message::Session(SessionMsg::TerminalFocused));
        assert_eq!(layout_of(&app).focused(), first);
        assert_ne!(first, second);
        let _ = drain(&mut rx);
        let _ = update(
            &mut app,
            Message::Session(SessionMsg::TerminalBytes(b"x".to_vec())),
        );
        let inputs: Vec<_> = drain(&mut rx)
            .into_iter()
            .filter_map(|m| match m {
                ClientMsg::SessionInput { session, .. } => Some(session),
                _ => None,
            })
            .collect();
        assert_eq!(inputs, vec![ids[0]]);
    }

    /// T018: each pane's size goes to its own terminal's PTY and to no other process.
    #[test]
    fn each_pane_sizes_its_own_terminal() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        let _ = drain(&mut rx);
        size_pane(&mut app, first, 80, 24);
        size_pane(&mut app, second, 60, 20);
        let sent: Vec<_> = drain(&mut rx)
            .into_iter()
            .filter_map(|m| match m {
                ClientMsg::SessionResize {
                    session,
                    process,
                    cols,
                    rows,
                } => Some((session, process, cols, rows)),
                _ => None,
            })
            .collect();
        assert_eq!(
            sent,
            vec![
                (ids[0], Some(SessionProcess::Primary), 80, 24),
                (ids[1], Some(SessionProcess::Primary), 60, 20),
            ]
        );
        // Unchanged sizes stay off the wire (at most one send per change).
        size_pane(&mut app, first, 80, 24);
        assert!(drain(&mut rx)
            .iter()
            .all(|m| !matches!(m, ClientMsg::SessionResize { .. })));
    }

    /// A terminal no pane shows is not resized; shown again, it takes its new pane's size (US5-3).
    #[test]
    fn a_terminal_not_shown_is_not_resized() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        size_pane(&mut app, first, 80, 24);
        size_pane(&mut app, second, 60, 20);
        let _ = drain(&mut rx);
        let resizes = |rx: &mut SentRx| {
            drain(rx)
                .into_iter()
                .filter_map(|m| match m {
                    ClientMsg::SessionResize {
                        session,
                        cols,
                        rows,
                        ..
                    } => Some((session, cols, rows)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        // Resizing the first pane resizes only the terminal in it.
        size_pane(&mut app, first, 90, 24);
        assert_eq!(resizes(&mut rx), vec![(ids[0], 90, 24)]);
        // Closing the second pane stops showing its terminal: it keeps its size, nothing is sent.
        pane_msg(&mut app, PaneMsg::Close(second));
        size_pane(&mut app, first, 120, 40);
        assert!(resizes(&mut rx).iter().all(|(s, ..)| *s != ids[1]));
        // Shown again in a new pane, it is sized to that pane.
        let third = split_focused(&mut app, Axis::Vertical);
        let _ = drain(&mut rx);
        pane_msg(
            &mut app,
            PaneMsg::Show(third, t(ids[1], SessionProcess::Primary)),
        );
        size_pane(&mut app, third, 50, 15);
        assert!(resizes(&mut rx).contains(&(ids[1], 50, 15)));
    }

    // ---- M3 (T026): resize, close, swap ------------------------------------------------------

    use micold_core::pane_layout::SplitEvent;

    /// FR-007, SC-005: closing a pane sends nothing that stops, restarts or detaches a session, and
    /// the session stays in the workspace (tab strip and sidebar read it from there).
    #[test]
    fn closing_a_pane_leaves_every_session_running() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        assert_eq!(layout_of(&app).focused(), second);
        let _ = drain(&mut rx);
        pane_msg(&mut app, PaneMsg::Close(second));
        let l = layout_of(&app);
        assert_eq!(l.len(), 1);
        assert_eq!(l.focused(), first, "focus moves to the surviving pane");
        for m in drain(&mut rx) {
            assert!(
                !matches!(
                    m,
                    ClientMsg::SessionStop { .. }
                        | ClientMsg::SessionKill { .. }
                        | ClientMsg::SessionStart { .. }
                        | ClientMsg::SessionRestart { .. }
                        | ClientMsg::Detach { .. }
                ),
                "closing a pane sent {m:?}"
            );
        }
        let still: Vec<_> = app
            .core
            .workspace
            .sessions
            .values()
            .flatten()
            .map(|s| s.id)
            .collect();
        assert_eq!(still, ids, "both sessions remain");
        assert_eq!(app.core.session.active, Some(ids[0]));
    }

    /// Replacing a pane's terminal (picker, swap) is just as quiet.
    #[test]
    fn swapping_terminals_sends_no_session_lifecycle_message() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        let _ = drain(&mut rx);
        pane_msg(&mut app, PaneMsg::Gesture(SplitEvent::Swap(first, second)));
        let l = layout_of(&app);
        assert_eq!(
            l.pane(first).unwrap().terminal(),
            Some(t(ids[1], SessionProcess::Primary))
        );
        assert_eq!(
            l.pane(second).unwrap().terminal(),
            Some(t(ids[0], SessionProcess::Primary))
        );
        assert_eq!(l.focused(), second, "focus follows the moved terminal");
        assert_eq!(app.core.session.active, Some(ids[0]));
        for m in drain(&mut rx) {
            assert!(
                !matches!(
                    m,
                    ClientMsg::SessionStop { .. }
                        | ClientMsg::SessionKill { .. }
                        | ClientMsg::SessionStart { .. }
                        | ClientMsg::SessionRestart { .. }
                        | ClientMsg::Detach { .. }
                ),
                "swap sent {m:?}"
            );
        }
    }

    /// Closing a pane that is not focused keeps focus where it was; the last pane is refused with
    /// a visible reason.
    #[test]
    fn the_last_pane_cannot_be_closed_and_says_why() {
        let (mut app, _ids, _rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        pane_msg(&mut app, PaneMsg::FocusPane(first));
        pane_msg(&mut app, PaneMsg::Close(second));
        assert_eq!(layout_of(&app).focused(), first);
        assert_eq!(app.pane_refusal, None);
        pane_msg(&mut app, PaneMsg::Close(first));
        assert_eq!(layout_of(&app).len(), 1);
        assert_eq!(app.pane_refusal, Some("The last pane stays open."));
    }

    /// T038 (FR-001/FR-002): showing the reason relayouts the panes, and the measurement that
    /// follows is not a user action; it must not clear the reason it just made room for.
    #[test]
    fn a_refusal_survives_the_resize_it_causes() {
        let (mut app, _ids, _rx) = app_with_sessions(1);
        let f = layout_of(&app).focused();
        size_pane(&mut app, f, 4, 2);
        pane_msg(&mut app, PaneMsg::Split(f, Axis::Vertical));
        assert!(app.pane_refusal.is_some());
        size_pane(&mut app, f, 4, 1);
        assert!(
            app.pane_refusal.is_some(),
            "a Resized measurement cleared the refusal"
        );
        pane_msg(&mut app, PaneMsg::FocusPane(f));
        assert_eq!(app.pane_refusal, None, "a user action still clears it");
    }

    /// T039 (FR-010, FR-016): a focus resting on an empty pane survives a switch away and back.
    #[test]
    fn a_focus_on_an_empty_pane_survives_a_project_switch() {
        let (mut app, ids, _rx) = app_with_sessions(1);
        let first = layout_of(&app).focused();
        let empty = split_focused(&mut app, Axis::Vertical);
        // The new pane opens on no terminal (only one exists, and the first pane shows it).
        assert_eq!(layout_of(&app).pane(empty).unwrap().terminal(), None);
        assert_eq!(layout_of(&app).focused(), empty);
        let home = app.core.workspace.active.clone().unwrap();
        let other = PathBuf::from("/repo/other");
        app.core.workspace.projects.push(Project::new(
            other.clone(),
            true,
            Availability::Available,
        ));
        app.core.workspace.active = Some(other);
        app.core.session.active = None;
        crate::shell::panes::sync(&mut app);
        app.core.workspace.active = Some(home);
        app.core.session.active = Some(ids[0]);
        crate::shell::panes::sync(&mut app);
        assert_eq!(
            layout_of(&app).focused(),
            empty,
            "the project switch moved focus to pane {first:?}"
        );
    }

    /// FR-009: the close chord closes the focused pane.
    #[test]
    fn the_close_chord_closes_the_focused_pane() {
        use micold_client::keymap::PaneAction;
        let (mut app, _ids, _rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        split_focused(&mut app, Axis::Vertical);
        pane_msg(&mut app, PaneMsg::Chord(PaneAction::Close));
        assert_eq!(layout_of(&app).len(), 1);
        assert_eq!(layout_of(&app).focused(), first);
    }

    /// T018: while a divider is dragged the sizes are not sent; the release sends them once.
    #[test]
    fn a_divider_drag_sends_pane_sizes_once_on_release() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let first = layout_of(&app).focused();
        let second = split_focused(&mut app, Axis::Vertical);
        size_pane(&mut app, first, 100, 40);
        size_pane(&mut app, second, 100, 40);
        let _ = drain(&mut rx);
        for (bp, cols) in [(3000u16, 60u16), (3500, 70), (4000, 80)] {
            pane_msg(
                &mut app,
                PaneMsg::Gesture(SplitEvent::Drag {
                    index: 0,
                    basis_points: bp,
                }),
            );
            // The view lays the panes out again at the new share and reports their sizes.
            size_pane(&mut app, first, cols, 40);
            size_pane(&mut app, second, 200 - cols, 40);
        }
        assert!(
            drain(&mut rx)
                .iter()
                .all(|m| !matches!(m, ClientMsg::SessionResize { .. })),
            "nothing is sent mid-drag"
        );
        assert_eq!(layout_of(&app).ratio(0), Some(0.4));
        pane_msg(&mut app, PaneMsg::Gesture(SplitEvent::Release));
        let sent: Vec<_> = drain(&mut rx)
            .into_iter()
            .filter_map(|m| match m {
                ClientMsg::SessionResize {
                    session,
                    cols,
                    rows,
                    ..
                } => Some((session, cols, rows)),
                _ => None,
            })
            .collect();
        assert_eq!(sent, vec![(ids[0], 80, 40), (ids[1], 120, 40)]);
        assert!(!app.divider_dragging);
    }

    /// FR-005: a double press restores equal sizes.
    #[test]
    fn a_double_press_on_the_divider_restores_equal_sizes() {
        let (mut app, _ids, _rx) = app_with_sessions(2);
        split_focused(&mut app, Axis::Vertical);
        pane_msg(
            &mut app,
            PaneMsg::Gesture(SplitEvent::Drag {
                index: 0,
                basis_points: 2000,
            }),
        );
        pane_msg(&mut app, PaneMsg::Gesture(SplitEvent::Release));
        assert_eq!(layout_of(&app).ratio(0), Some(0.2));
        pane_msg(&mut app, PaneMsg::Gesture(SplitEvent::Reset(0)));
        assert_eq!(layout_of(&app).ratio(0), Some(0.5));
    }

    // ---- M4 (T033): the layout survives a restart ----------------------------------------------

    use micold_core::protocol::messages::{CatalogSnapshot, ProjectSnapshot};

    fn catalog_with_layout(project: &str, layout: Option<String>) -> CatalogSnapshot {
        CatalogSnapshot {
            schema_version: 1,
            last_active: Some(PathBuf::from(project)),
            projects: vec![ProjectSnapshot {
                pane_layout: layout,
                path: PathBuf::from(project),
                display_name: "demo".into(),
                is_git_repo: true,
                available: true,
                worktrees: Vec::new(),
                sessions: Vec::new(),
            }],
            env_include_failures: Vec::new(),
        }
    }

    fn sent_layouts(rx: &mut SentRx) -> Vec<String> {
        drain(rx)
            .into_iter()
            .filter_map(|m| match m {
                ClientMsg::SetPaneLayout { layout, .. } => layout,
                _ => None,
            })
            .collect()
    }

    /// `[first | second]` at 30/70, the second focused.
    fn two_pane_layout(first: TerminalRef, second: TerminalRef) -> PaneLayout {
        let mut l = PaneLayout::single();
        let a = l.focused();
        l.show(a, first).unwrap();
        l.split(
            a,
            Axis::Vertical,
            (1000.0, 600.0),
            (10.0, 10.0),
            Some(second),
        )
        .unwrap();
        l.set_ratio(0, 0.3, (1000.0, 600.0), (10.0, 10.0));
        l
    }

    /// US6 scenario 1: structure, ratios, contents and the focused pane come back from the snapshot,
    /// and the restored focus decides the displayed session.
    #[test]
    fn the_layout_is_restored_from_the_snapshot_on_connect() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let (first, second) = (
            t(ids[0], SessionProcess::Primary),
            t(ids[1], SessionProcess::Primary),
        );
        let stored = two_pane_layout(first, second);
        assert_eq!(stored.focused_terminal(), Some(second));
        let _ = drain(&mut rx);
        crate::shell::panes::restore(
            &mut app,
            &catalog_with_layout("/repo/panes", Some(stored.to_json())),
        );
        crate::shell::panes::sync(&mut app);
        assert_eq!(layout_of(&app), stored);
        assert_eq!(
            app.core.session.active,
            Some(ids[1]),
            "the focused pane's session is shown"
        );
        assert!(
            sent_layouts(&mut rx).is_empty(),
            "what was just restored is not sent back"
        );
    }

    /// US6 scenario 3: a terminal that no longer exists leaves an empty pane, the rest loads.
    #[test]
    fn a_gone_terminal_becomes_an_empty_pane_on_restore() {
        let (mut app, ids, _rx) = app_with_sessions(1);
        let gone = t(SessionId::new(), SessionProcess::Primary);
        let stored = two_pane_layout(t(ids[0], SessionProcess::Primary), gone);
        crate::shell::panes::restore(
            &mut app,
            &catalog_with_layout("/repo/panes", Some(stored.to_json())),
        );
        crate::shell::panes::sync(&mut app);
        let l = layout_of(&app);
        assert_eq!(l.len(), 2);
        assert_eq!(l.terminals(), vec![t(ids[0], SessionProcess::Primary)]);
        assert_eq!(l.ratio(0), Some(0.3));
    }

    /// US6 scenario 4 and FR-014: no layout, or one that cannot be read, is one pane on the last
    /// session.
    #[test]
    fn no_stored_layout_and_a_corrupt_one_both_give_one_pane() {
        for stored in [None, Some("{not json".to_string())] {
            let (mut app, ids, _rx) = app_with_sessions(2);
            crate::shell::panes::restore(&mut app, &catalog_with_layout("/repo/panes", stored));
            crate::shell::panes::sync(&mut app);
            let l = layout_of(&app);
            assert_eq!(l.len(), 1);
            assert_eq!(l.terminals(), vec![t(ids[0], SessionProcess::Primary)]);
            assert_eq!(app.core.session.active, Some(ids[0]));
        }
    }

    /// Only the first snapshot restores: a later one echoes what this client sent.
    #[test]
    fn a_later_snapshot_does_not_replace_the_layout() {
        let (mut app, ids, _rx) = app_with_sessions(2);
        let stored = two_pane_layout(
            t(ids[0], SessionProcess::Primary),
            t(ids[1], SessionProcess::Primary),
        );
        crate::shell::panes::restore(
            &mut app,
            &catalog_with_layout("/repo/panes", Some(stored.to_json())),
        );
        let first_pane = layout_of(&app).panes()[0].id();
        crate::shell::panes::focus_pane(&mut app, first_pane);
        let before = layout_of(&app);
        crate::shell::panes::restore(
            &mut app,
            &catalog_with_layout("/repo/panes", Some(stored.to_json())),
        );
        assert_eq!(layout_of(&app), before);
    }

    /// FR-013: every change to the layout goes to the daemon; an unchanged one does not.
    #[test]
    fn every_layout_change_sends_set_pane_layout() {
        let (mut app, _ids, mut rx) = app_with_sessions(2);
        assert!(
            sent_layouts(&mut rx).is_empty(),
            "an unsplit project stores nothing"
        );
        split_focused(&mut app, Axis::Vertical);
        let sent = sent_layouts(&mut rx);
        assert_eq!(sent.len(), 1);
        assert_eq!(PaneLayout::from_json(&sent[0]), Some(layout_of(&app)));
        crate::shell::panes::sync(&mut app);
        assert!(sent_layouts(&mut rx).is_empty(), "no change, no message");
        // A divider drag ends with one more.
        pane_msg(
            &mut app,
            PaneMsg::Gesture(SplitEvent::Drag {
                index: 0,
                basis_points: 2000,
            }),
        );
        pane_msg(&mut app, PaneMsg::Gesture(SplitEvent::Release));
        let sent = sent_layouts(&mut rx);
        assert_eq!(
            PaneLayout::from_json(sent.last().unwrap()),
            Some(layout_of(&app))
        );
        // Closing back to one pane stores that too.
        let focused = layout_of(&app).focused();
        pane_msg(&mut app, PaneMsg::Close(focused));
        let sent = sent_layouts(&mut rx);
        assert_eq!(
            PaneLayout::from_json(sent.last().unwrap()).map(|l| l.len()),
            Some(1)
        );
    }

    /// FR-016: restoring one project's layout leaves another project's alone.
    #[test]
    fn each_project_restores_its_own_layout() {
        let (mut app, ids, _rx) = app_with_sessions(2);
        let other = PathBuf::from("/repo/other");
        app.core.workspace.projects.push(Project::new(
            other.clone(),
            true,
            Availability::Available,
        ));
        let stored = two_pane_layout(
            t(ids[0], SessionProcess::Primary),
            t(ids[1], SessionProcess::Primary),
        );
        let mut catalog = catalog_with_layout("/repo/panes", None);
        catalog.projects.push(ProjectSnapshot {
            pane_layout: Some(stored.to_json()),
            path: other.clone(),
            display_name: "other".into(),
            is_git_repo: true,
            available: true,
            worktrees: Vec::new(),
            sessions: Vec::new(),
        });
        crate::shell::panes::restore(&mut app, &catalog);
        assert_eq!(
            layout_of(&app).len(),
            1,
            "the active project has none stored"
        );
        assert_eq!(app.pane_layouts.get(&other), Some(&stored));
    }

    /// Two panes over two running sessions; the second is focused. Returns the panes' terminals.
    fn two_running_panes(app: &mut App, ids: &[SessionId]) -> (TerminalRef, TerminalRef) {
        let first = t(ids[0], SessionProcess::Primary);
        let second = t(ids[1], SessionProcess::Primary);
        for id in ids {
            let (_, s) = app.core.workspace.find_session_mut(*id).unwrap();
            s.lifecycle = micold_core::session::SessionLifecycle::Running;
        }
        let _ = update(app, Message::Session(SessionMsg::Selected(ids[0])));
        let p0 = layout_of(app).focused();
        crate::shell::panes::split_pane(
            app,
            p0,
            micold_core::pane_layout::Axis::Vertical,
            (1000.0, 600.0),
            (10.0, 10.0),
        )
        .unwrap();
        let _ = update(app, Message::Session(SessionMsg::Selected(ids[1])));
        (first, second)
    }

    fn inputs(rx: &mut SentRx) -> Vec<(SessionId, Option<SessionProcess>, Vec<u8>)> {
        drain(rx)
            .into_iter()
            .filter_map(|m| match m {
                ClientMsg::SessionInput {
                    session,
                    process,
                    bytes,
                    ..
                } => Some((session, process, bytes)),
                _ => None,
            })
            .collect()
    }

    /// The files of one drop arrive as separate messages and are inserted as one run, in event
    /// order (feature 487, T012), at the pane they landed on, not the focused one (FR-005).
    #[test]
    fn files_dropped_in_one_turn_are_inserted_together_at_the_pane_they_landed_on() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let (first, second) = two_running_panes(&mut app, &ids);
        assert_eq!(layout_of(&app).focused_terminal(), Some(second));
        let unfocused = layout_of(&app).pane_showing(first).unwrap();
        let _ = drain(&mut rx);
        let drop = |app: &mut App, path: &str| {
            update(
                app,
                Message::Session(SessionMsg::Pane(PaneMsg::FileDropped(
                    unfocused,
                    PathBuf::from(path),
                ))),
            )
        };
        let _ = drop(&mut app, "/d/a b.png");
        let _ = drop(&mut app, "/d/c");
        let _ = drop(&mut app, "/d/it's");
        assert_eq!(app.pending_drops.len(), 3);
        assert!(
            inputs(&mut rx).is_empty(),
            "nothing is sent until the drop settles"
        );
        let _ = update(
            &mut app,
            Message::Session(SessionMsg::Pane(PaneMsg::DropsSettled)),
        );
        assert!(app.pending_drops.is_empty());
        let sent = inputs(&mut rx);
        assert_eq!(
            sent,
            vec![(
                ids[0],
                Some(SessionProcess::Primary),
                micold_client::keymap::paste_bytes(r"'/d/a b.png' '/d/c' '/d/it'\''s'", false),
            )]
        );
        assert!(
            !sent[0].2.iter().any(|b| *b == b'\r' || *b == b'\n'),
            "SC-004"
        );
        assert_eq!(
            layout_of(&app).focused_terminal(),
            Some(second),
            "a drop does not move focus"
        );
    }

    /// A drop on an exited terminal inserts nothing and says so (FR-012).
    #[test]
    fn a_drop_on_a_terminal_that_is_not_running_sends_nothing_and_notifies() {
        let (mut app, ids, mut rx) = app_with_sessions(2);
        let (first, _) = two_running_panes(&mut app, &ids);
        let (_, s) = app.core.workspace.find_session_mut(first.session).unwrap();
        s.lifecycle = micold_core::session::SessionLifecycle::Idle;
        let pane = layout_of(&app).pane_showing(first).unwrap();
        let _ = drain(&mut rx);
        let _ = update(
            &mut app,
            Message::Session(SessionMsg::Pane(PaneMsg::FileDropped(
                pane,
                PathBuf::from("/f"),
            ))),
        );
        let _ = update(
            &mut app,
            Message::Session(SessionMsg::Pane(PaneMsg::DropsSettled)),
        );
        assert!(inputs(&mut rx).is_empty());
        assert!(app.core.notifications.queue.is_active());
    }
}

/// Feature 491 (FR-004, FR-007): the binary routes an op to its binding's daemon only, and one
/// daemon dropping leaves the other's outbox and pending ops alone.
mod two_daemons {
    use super::*;
    use micold_core::daemons::{
        Binding, ContainerSettings, DaemonEntry, DaemonId, DaemonName, DaemonRegistry,
        DaemonRuntime,
    };

    fn entry(id: u32, name: &str, runtime: DaemonRuntime) -> DaemonEntry {
        DaemonEntry {
            id: DaemonId(id),
            name: DaemonName::new(name).expect("a name"),
            runtime,
            auto_start: true,
            notice: None,
        }
    }

    fn two_connected() -> (
        App,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
        iced::futures::channel::mpsc::UnboundedReceiver<ClientMsg>,
    ) {
        let mut app = base_app();
        let container = DaemonRuntime::Container(ContainerSettings {
            profile: Default::default(),
            container_name: "box".into(),
            port: 7001,
            migration: None,
        });
        app.core.settings.daemons = DaemonRegistry::new(
            vec![
                entry(1, "Host", DaemonRuntime::Host),
                entry(2, "Box", container),
            ],
            3,
        );
        let (tx1, rx1) = iced::futures::channel::mpsc::unbounded();
        let (tx2, rx2) = iced::futures::channel::mpsc::unbounded();
        let links = &mut app.core.settings.links;
        links.insert(DaemonId(1), micold_core::daemons::DaemonState::Starting);
        links.insert(DaemonId(2), micold_core::daemons::DaemonState::Starting);
        links.connected(DaemonId(1), micold_client::daemon::Outbox::new(tx1));
        links.connected(DaemonId(2), micold_client::daemon::Outbox::new(tx2));
        (app, rx1, rx2)
    }

    fn send(app: &mut App, binding: Binding) {
        crate::shell::daemon_sync::send_op_bound(app, binding, PendingOp::DeleteSession, |req| {
            ClientMsg::SessionDelete {
                req,
                session: micold_core::session::SessionId(uuid::Uuid::nil()),
            }
        });
    }

    #[test]
    fn an_op_goes_to_its_bound_daemon_and_to_no_other() {
        let (mut app, mut rx1, mut rx2) = two_connected();
        send(&mut app, Binding::Bound(DaemonId(2)));
        assert!(rx2.try_recv().is_ok(), "the bound daemon got the op");
        assert!(rx1.try_recv().is_err(), "the other daemon heard nothing");
        assert_eq!(app.pending_daemon.values().next(), Some(&DaemonId(2)));
    }

    /// Bug 1159: a session started at a worktree bound to the container daemon is created there,
    /// whichever session happens to be on display (here none, so the active key is the Default's,
    /// which resolves to the host).
    #[test]
    fn a_session_start_goes_to_the_daemon_its_location_is_bound_to() {
        let (mut app, mut rx1, mut rx2) = two_connected();
        let project = PathBuf::from("/repo/p");
        app.core.workspace.active = Some(project.clone());
        app.core.workspace.bind(&project, "w", DaemonId(2));
        let _ = crate::shell::daemon_sync::on_session_start_requested(
            &mut app,
            micold_core::session::SessionLocation::Worktree("w".into()),
            micold_core::session::AiCli::ClaudeCode,
            micold_core::session::SessionKind::default(),
            true,
        );
        let created = |rx: &mut SentRx| {
            drain(rx)
                .into_iter()
                .any(|m| matches!(m, ClientMsg::SessionCreate { .. }))
        };
        assert!(created(&mut rx2), "the container daemon got the create");
        assert!(!created(&mut rx1), "the host daemon heard nothing");
    }

    /// Bug 1159: with the bound daemon down, the start fails visibly and reaches no other daemon.
    #[test]
    fn a_session_start_for_a_down_daemon_fails_visibly_and_does_not_fall_back() {
        let (mut app, mut rx1, _rx2) = two_connected();
        let project = PathBuf::from("/repo/p");
        app.core.workspace.active = Some(project.clone());
        app.core.workspace.bind(&project, "w", DaemonId(2));
        let _ = app.core.settings.links.lost(DaemonId(2), "test");
        let _ = crate::shell::daemon_sync::on_session_start_requested(
            &mut app,
            micold_core::session::SessionLocation::Worktree("w".into()),
            micold_core::session::AiCli::ClaudeCode,
            micold_core::session::SessionKind::default(),
            true,
        );
        assert!(drain(&mut rx1).is_empty(), "nothing fell back to the host");
        assert!(app.core.notifications.queue.is_active());
    }

    /// Bug 1159: a worktree created with the container daemon chosen in the form is created there.
    #[test]
    fn a_worktree_create_goes_to_the_daemon_chosen_in_the_form() {
        let (mut app, mut rx1, mut rx2) = two_connected();
        let project = PathBuf::from("/repo/p");
        app.core.workspace.active = Some(project.clone());
        app.core.update(Message::WorktreeForm(FormMsg::Opened));
        app.core
            .update(Message::WorktreeForm(FormMsg::DaemonChosen(DaemonId(2))));
        crate::shell::daemon_sync::send_worktree_create(
            &mut app,
            project,
            micold_core::naming::DerivedNames {
                dir_name: "feat-x".into(),
                branch: "feat/x".into(),
            },
            micold_core::worktree::CreateMode::NewBranch,
        );
        let created = |rx: &mut SentRx| {
            drain(rx)
                .into_iter()
                .any(|m| matches!(m, ClientMsg::WorktreeCreate { .. }))
        };
        assert!(created(&mut rx2), "the container daemon got the create");
        assert!(!created(&mut rx1), "the host daemon heard nothing");
    }

    /// Bug 1159: a worktree create for a down chosen daemon fails visibly, reaches no other daemon,
    /// and returns the form to editing.
    #[test]
    fn a_worktree_create_for_a_down_daemon_fails_and_frees_the_form() {
        let (mut app, mut rx1, _rx2) = two_connected();
        let project = PathBuf::from("/repo/p");
        app.core.workspace.active = Some(project.clone());
        app.core.update(Message::WorktreeForm(FormMsg::Opened));
        app.core
            .update(Message::WorktreeForm(FormMsg::DaemonChosen(DaemonId(2))));
        let _ = app.core.settings.links.lost(DaemonId(2), "test");
        crate::shell::daemon_sync::send_worktree_create(
            &mut app,
            project,
            micold_core::naming::DerivedNames {
                dir_name: "feat-x".into(),
                branch: "feat/x".into(),
            },
            micold_core::worktree::CreateMode::NewBranch,
        );
        assert!(drain(&mut rx1).is_empty(), "nothing fell back to the host");
        assert_eq!(
            form_status(&app),
            Some(micold_client::features::worktree_form::WorktreeFormStatus::Editing)
        );
    }

    #[test]
    fn forgetting_a_project_detaches_from_each_daemon_its_locations_are_bound_to() {
        let (mut app, mut rx1, mut rx2) = two_connected();
        let project = PathBuf::from("/repo/p");
        // Default resolves to the legacy default (daemon 1); the worktree is bound to daemon 2.
        app.core.workspace.bind(&project, "w", DaemonId(2));
        app.core.project.forget_target = Some(project.clone());
        let _ = crate::shell::daemon_sync::on_project_forget_confirmed(&mut app);
        let detached = |rx: &mut SentRx| {
            drain(rx)
                .into_iter()
                .any(|m| matches!(m, ClientMsg::Detach { project: p } if p == project))
        };
        assert!(detached(&mut rx1), "daemon 1 released the project");
        assert!(detached(&mut rx2), "daemon 2 released the project");
    }

    #[test]
    fn leaving_a_project_detaches_from_the_daemon_of_the_location_that_was_active() {
        let (mut app, mut rx1, mut rx2) = two_connected();
        let old = PathBuf::from("/repo/old");
        crate::shell::daemon_sync::switch_daemon_attachment(
            &mut app,
            Some((old.clone(), Binding::Bound(DaemonId(2)))),
            Path::new("/repo/new"),
        );
        assert!(matches!(
            rx2.try_recv(),
            Ok(ClientMsg::Detach { project }) if project == old
        ));
        assert!(!drain(&mut rx1)
            .into_iter()
            .any(|m| matches!(m, ClientMsg::Detach { .. })));
    }

    #[test]
    fn a_connection_to_a_stopped_daemon_is_dropped_without_adopting_anything() {
        let (mut app, _rx1, _rx2) = two_connected();
        let _ = app
            .core
            .settings
            .links
            .apply(DaemonId(2), micold_core::daemons::DaemonEvent::Stop);
        app.disconnected.insert(DaemonId(2));
        let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
        let _ = crate::shell::daemon_sync::on_connected(
            &mut app,
            DaemonId(2),
            micold_client::daemon::Outbox::new(tx),
            micold_core::protocol::messages::CatalogSnapshot::default(),
            quiet_settings(),
            None,
        );
        assert!(app.disconnected.contains(&DaemonId(2)));
        assert!(!app.daemon_catalogs.contains_key(&DaemonId(2)));
        assert!(app.core.settings.links.outbox(DaemonId(2)).is_none());
    }

    /// Feature 490, T034: the reading is the `Welcome`'s, then each push's, and none once the
    /// active daemon's connection is lost.
    #[test]
    fn the_plan_usage_reading_follows_the_welcome_the_pushes_and_the_disconnect() {
        use micold_core::plan_usage::{LimitWindow, UsageReading};
        let (mut app, _rx1, _rx2) = two_connected();
        let active = app
            .active_daemon()
            .unwrap_or_else(|| app.core.settings.legacy_daemon());
        let reading = UsageReading {
            windows: vec![LimitWindow {
                key: "five_hour".into(),
                used_tenths: 420,
                resets_at: None,
            }],
            obtained_at: 1,
        };
        let (tx, _rx) = iced::futures::channel::mpsc::unbounded();
        let _ = crate::shell::daemon_sync::on_connected(
            &mut app,
            active,
            micold_client::daemon::Outbox::new(tx),
            micold_core::protocol::messages::CatalogSnapshot::default(),
            quiet_settings(),
            Some(reading.clone()),
        );
        assert_eq!(
            app.core.plan_usage.reading,
            Some(reading.clone()),
            "Welcome"
        );

        let changed =
            |reading| micold_core::protocol::messages::DaemonMsg::PlanUsageChanged { reading };
        let _ = crate::shell::daemon_sync::on_daemon_event(&mut app, active, changed(None));
        assert_eq!(app.core.plan_usage.reading, None, "nothing current");
        let _ = crate::shell::daemon_sync::on_daemon_event(
            &mut app,
            active,
            changed(Some(reading.clone())),
        );
        assert_eq!(app.core.plan_usage.reading, Some(reading), "a push");

        let _ = crate::shell::daemon_sync::on_disconnected(&mut app, active);
        assert_eq!(app.core.plan_usage.reading, None, "FR-014");
    }

    #[test]
    fn one_daemon_dropping_leaves_the_other_connected_and_its_pending_ops() {
        let (mut app, mut rx1, mut rx2) = two_connected();
        send(&mut app, Binding::Bound(DaemonId(1)));
        let _ = app.core.settings.links.lost(DaemonId(2), "gone");
        assert!(app.core.settings.links.is_connected(DaemonId(1)));
        assert!(!app.core.settings.links.is_connected(DaemonId(2)));
        assert_eq!(app.pending_ops.len(), 1, "daemon 1's pending op survives");
        // An op for the down daemon is refused and never reroutes to the live one.
        let _ = rx1.try_recv();
        send(&mut app, Binding::Bound(DaemonId(2)));
        assert!(rx1.try_recv().is_err());
        assert!(rx2.try_recv().is_err());
        assert!(app.core.notifications.queue.is_active());
    }
}

// --- feature 533 T074: the shell's Jira request path, with the switch, the connection and the
// token file each taken away after the form opened --------------------------------------------
//
// `jira_source_state.rs` drives the reducer; these drive what only the shell does: the gate every
// request passes and the token read at request time. The source is a double that records the token
// it was asked with, so a request that went out when it should not have shows as a recorded call.

mod jira_request_path {
    use super::*;
    use micold_client::features::worktree_form::{
        BranchSource, IssueList, Msg as FormMsg, SearchState,
    };
    use micold_core::github::{Issue, IssueLoadError, IssuePage};
    use micold_core::jira::{connection_key, ApiToken, JiraConnection, JiraSource};
    use std::sync::Mutex;

    const PROJECT: &str = "/repo/demo";

    /// Records the token of every call; answers one issue, or the search's two.
    #[derive(Default)]
    struct Recording {
        tokens: Mutex<Vec<String>>,
        searches: Mutex<Vec<String>>,
    }

    impl Recording {
        fn seen(&self) -> Vec<String> {
            self.tokens.lock().unwrap().clone()
        }

        /// The text of every search the source was asked for.
        fn searched_for(&self) -> Vec<String> {
            self.searches.lock().unwrap().clone()
        }
    }

    fn found(key: &str, number: u64) -> Issue {
        Issue::jira(
            key,
            number,
            "Add login".to_string(),
            Vec::new(),
            "2026-10-01T00:00:00Z".to_string(),
        )
    }

    impl JiraSource for Recording {
        fn list_open(
            &self,
            _: &JiraConnection,
            token: &ApiToken,
            _: Option<&str>,
        ) -> Result<IssuePage, IssueLoadError> {
            self.tokens.lock().unwrap().push(token.expose().to_string());
            if token.expose() == "revoked" {
                return Err(IssueLoadError::Other("revoked".to_string()));
            }
            Ok(IssuePage {
                issues: vec![found("ABC-1", 1)],
                total_open: 9,
                next_cursor: Some("more".to_string()),
            })
        }

        fn search_open(
            &self,
            _: &JiraConnection,
            token: &ApiToken,
            text: &str,
        ) -> Result<Vec<Issue>, IssueLoadError> {
            self.tokens.lock().unwrap().push(token.expose().to_string());
            self.searches.lock().unwrap().push(text.to_string());
            Ok(vec![found("ABC-2", 2)])
        }
    }

    struct Rig {
        app: App,
        source: Arc<Recording>,
        dir: tempfile::TempDir,
    }

    impl Rig {
        fn token_file(&self) -> PathBuf {
            micold_core::store::jira_token_path(self.dir.path(), std::path::Path::new(PROJECT))
        }

        fn store_token(&self, token: &str) {
            std::fs::write(self.token_file(), token).unwrap();
        }
    }

    fn send(app: &mut App, msg: FormMsg) {
        let work = update_inner(app, Message::WorktreeForm(msg));
        settle(app, work);
    }

    fn issues(app: &App) -> &IssueList {
        &app.core
            .worktree_form
            .form
            .as_ref()
            .expect("form open")
            .issues
    }

    /// Switch on, a connection, the form open on a Jira-only project, a token file holding `first`.
    fn rig() -> Rig {
        let dir = tempfile::tempdir().unwrap();
        let source = Arc::new(Recording::default());
        let mut app = base_app();
        app.caps = app
            .caps
            .clone()
            .with_jira(source.clone(), dir.path().to_path_buf());
        app.core.workspace.active = Some(PathBuf::from(PROJECT));
        app.core.settings.jira_enabled = true;
        app.core.settings.jira_connections.insert(
            connection_key(std::path::Path::new(PROJECT)),
            JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap(),
        );
        let rig = Rig { app, source, dir };
        rig.store_token("first");
        let mut rig = rig;
        send(&mut rig.app, FormMsg::Opened);
        send(&mut rig.app, FormMsg::RemotesListed(Ok(Vec::new())));
        rig
    }

    fn failure(app: &App) -> String {
        match issues(app) {
            IssueList::Failed { error } => format!("{error:?}"),
            other => panic!("expected a failed load, found {other:?}"),
        }
    }

    #[test]
    fn a_connected_request_reads_the_token_file_and_lists_the_issues() {
        let mut rig = rig();
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert!(matches!(issues(&rig.app), IssueList::Loaded { .. }));
        let seen = rig.source.seen();
        assert!(
            !seen.is_empty() && seen.iter().all(|t| t == "first"),
            "{seen:?}"
        );
    }

    #[test]
    fn with_the_switch_off_nothing_is_asked() {
        let mut rig = rig();
        rig.app.core.settings.jira_enabled = false;
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert!(failure(&rig.app).contains("isn't connected"));
        assert_eq!(rig.source.seen(), Vec::<String>::new());
    }

    #[test]
    fn with_no_connection_nothing_is_asked() {
        let mut rig = rig();
        rig.app.core.settings.jira_connections.clear();
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert!(failure(&rig.app).contains("isn't connected"));
        assert_eq!(rig.source.seen(), Vec::<String>::new());
    }

    #[test]
    fn with_no_token_file_nothing_is_asked_and_settings_is_named() {
        let mut rig = rig();
        std::fs::remove_file(rig.token_file()).unwrap();
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        let message = failure(&rig.app);
        assert!(message.contains("no API token is stored"), "{message}");
        assert!(message.contains("Settings > Jira"), "{message}");
        assert_eq!(rig.source.seen(), Vec::<String>::new());
    }

    #[test]
    fn a_rotated_token_is_used_by_the_next_request() {
        let mut rig = rig();
        rig.store_token("revoked");
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        assert!(failure(&rig.app).contains("revoked"));
        rig.store_token("rotated\n");
        send(&mut rig.app, FormMsg::IssueRetry);
        assert!(matches!(issues(&rig.app), IssueList::Loaded { .. }));
        let seen = rig.source.seen();
        assert_eq!(seen[0], "revoked", "{seen:?}");
        assert!(
            seen.len() > 1 && seen[1..].iter().all(|t| t == "rotated"),
            "{seen:?}"
        );
    }

    /// Type `text` over an incomplete listing; `settle` runs the debounce and the search.
    fn start_search(rig: &mut Rig, text: &str) {
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        send(&mut rig.app, FormMsg::IssueQueryChanged(text.to_string()));
    }

    #[test]
    fn a_search_reads_the_token_file_and_finds_beyond_the_list() {
        let mut rig = rig();
        start_search(&mut rig, "zzz");
        let IssueList::Loaded { searched, .. } = issues(&rig.app) else {
            panic!("loaded");
        };
        assert_eq!(searched.len(), 1);
        assert_eq!(rig.source.seen().last().map(String::as_str), Some("first"));
        assert_eq!(rig.source.searched_for(), ["zzz"], "the typed text is sent");
    }

    #[test]
    fn a_search_with_no_token_file_asks_nothing() {
        let mut rig = rig();
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        let asked = rig.source.seen().len();
        std::fs::remove_file(rig.token_file()).unwrap();
        send(&mut rig.app, FormMsg::IssueQueryChanged("zzz".to_string()));
        assert_eq!(rig.source.seen().len(), asked, "the search asked nothing");
        assert!(rig.source.searched_for().is_empty());
    }

    #[test]
    fn a_search_with_the_switch_off_asks_nothing() {
        let mut rig = rig();
        send(&mut rig.app, FormMsg::SourceChanged(BranchSource::Issue));
        let asked = rig.source.seen().len();
        rig.app.core.settings.jira_enabled = false;
        send(&mut rig.app, FormMsg::IssueQueryChanged("zzz".to_string()));
        assert!(matches!(
            issues(&rig.app),
            IssueList::Loaded {
                search: SearchState::Failed { .. },
                ..
            }
        ));
        assert_eq!(rig.source.seen().len(), asked, "the search asked nothing");
    }
}

// ---- Feature 972 (FR-004, FR-005, FR-011): the root's half of the Overview card menu ----

/// A connected app on a project with the given worktrees, the Overview open.
fn app_in_overview_with(dirs: &[&str]) -> App {
    use micold_core::project::{Availability, Project};
    use micold_core::worktree::{Worktree, WorktreeStatus};
    let (mut app, _rx) = connected_app();
    let path = PathBuf::from("/repo");
    app.core.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    app.core.workspace.active = Some(path);
    app.core.worktree.worktrees = dirs
        .iter()
        .map(|dir| Worktree {
            dir_name: (*dir).to_string(),
            path: PathBuf::from(format!("/repo/.claude/worktrees/{dir}")),
            branch: Some(format!("feat/{dir}")),
            status: WorktreeStatus::Valid,
            included: false,
        })
        .collect();
    app.core.overview.open = true;
    app
}

fn open_card_menu(app: &mut App, dir: &str, origin: micold_client::features::overview::MenuOrigin) {
    use micold_client::features::overview::Msg;
    app.core.update(Message::Overview(Msg::CardMenuOpened {
        location: SessionLocation::Worktree(dir.to_string()),
        anchor: (10, 10),
        origin,
    }));
    assert!(app.core.overview.card_menu.is_some(), "fixture: menu open");
}

fn open_dialog_id(app: &App) -> Option<&'static str> {
    micold_client::overlay::registry::open_dialog(&app.core).map(|open| open.id().as_str())
}

/// FR-004/FR-005 through the root: a picked entry closes the menu *and* the root runs the inner
/// message, so an in-place entry's dialog opens over the still-open Overview.
#[test]
fn the_root_runs_the_entry_a_card_menu_pick_carries() {
    use micold_client::features::overview::{MenuOrigin, Msg};
    use micold_client::features::worktree::Msg as WorktreeMsg;
    let mut app = app_in_overview_with(&["feat-a"]);
    open_card_menu(&mut app, "feat-a", MenuOrigin::Pointer);
    let inner = Message::Worktree(WorktreeMsg::DeleteRequested("feat-a".into()));

    let _ = update_inner(
        &mut app,
        Message::Overview(Msg::CardMenuEntryChosen(Box::new(inner))),
    );

    assert_eq!(
        app.core.overview.card_menu, None,
        "the pick closes the menu"
    );
    assert_eq!(
        open_dialog_id(&app),
        Some("confirm_worktree_delete"),
        "the root dispatched the inner message"
    );
    assert!(
        app.core.overview.open,
        "an in-place entry keeps the Overview"
    );
}

/// FR-010 through the root: Enter on the highlighted entry runs that entry.
#[test]
fn the_root_runs_the_highlighted_card_menu_entry_on_activation() {
    use micold_client::features::overview::{MenuOrigin, Msg};
    let mut app = app_in_overview_with(&["feat-a"]);
    open_card_menu(&mut app, "feat-a", MenuOrigin::Keyboard);
    let rename = micold_client::ui::overview::card_menu_items(
        &app.core,
        &SessionLocation::Worktree("feat-a".into()),
    )
    .iter()
    .position(|item| item.label == "Rename")
    .expect("the worktree's card menu lists Rename");
    app.core
        .overview
        .card_menu
        .as_mut()
        .expect("open")
        .highlighted = Some(rename);

    let _ = update_inner(&mut app, Message::Overview(Msg::CardMenuActivated));

    assert_eq!(
        app.core.overview.card_menu, None,
        "activating closes the menu"
    );
    assert!(
        app.core.worktree.rename_draft.is_some() && open_dialog_id(&app).is_some(),
        "and runs the entry: the rename dialog is open"
    );
}

/// FR-011, US3.3 (visual pass B12): the keyboard returns to the card the menu came from once no
/// dialog is up, and moves to the first card if the landed card has since gone.
#[test]
fn the_card_focus_return_waits_for_a_dialog_then_follows_the_card_or_the_first() {
    use micold_client::features::worktree::Msg as WorktreeMsg;
    let b = SessionLocation::Worktree("feat-b".into());
    let mut app = app_in_overview_with(&["feat-a", "feat-b"]);
    app.core.overview.focus_return = Some(micold_client::features::overview::FocusReturn {
        location: b.clone(),
        to_link: None,
    });
    let _ = update_inner(
        &mut app,
        Message::Worktree(WorktreeMsg::DeleteRequested("feat-b".into())),
    );
    assert!(
        card_focus_return(&mut app).is_none(),
        "a dialog is up: no focus move"
    );
    assert_eq!(
        app.core.overview.focus_return.as_ref().map(|r| &r.location),
        Some(&b),
        "and the return stays armed"
    );

    let _ = update_inner(&mut app, Message::Worktree(WorktreeMsg::DeleteCancelled));
    assert!(open_dialog_id(&app).is_none(), "fixture: dialog closed");
    assert!(
        card_focus_return(&mut app).is_some(),
        "no dialog: focus moves"
    );
    assert_eq!(app.core.overview.focus_return, None, "the return is spent");
    assert_eq!(
        app.core.overview.focus_landed,
        Some(b),
        "onto the card the menu came from"
    );
    assert!(
        card_focus_return(&mut app).is_none(),
        "landed and still listed: nothing more"
    );

    app.core
        .worktree
        .worktrees
        .retain(|w| w.dir_name != "feat-b");
    assert!(
        card_focus_return(&mut app).is_some(),
        "the landed card went: focus moves again"
    );
    assert_eq!(
        app.core.overview.focus_landed, None,
        "the landing is cleared"
    );
}

/// Feature 970, FR-008 (clarification 4): Save applies *Reduce motion*, and Cancel discards it.
#[test]
fn reduce_motion_is_applied_by_save_and_discarded_by_cancel() {
    let store = Arc::new(micold_core::settings::FakeSettingsStore::loaded(
        micold_core::settings::Settings::default(),
    ));
    let mut app = base_app();
    app.caps = app.caps.clone().with_settings(store.clone());

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::ReduceMotionToggled(true)),
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Cancelled));
    assert!(!app.core.settings.reduce_motion, "Cancel discards the edit");
    assert!(store.saves().is_empty(), "and writes nothing");

    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Opened));
    let _ = update_inner(
        &mut app,
        Message::Settings(SettingsMsg::ReduceMotionToggled(true)),
    );
    assert!(
        !app.core.settings.reduce_motion,
        "the draft alone changes nothing"
    );
    let _ = update_inner(&mut app, Message::Settings(SettingsMsg::Saved));

    assert!(app.core.settings.reduce_motion, "Save applies it");
    assert!(
        store
            .saves()
            .last()
            .is_some_and(|saved| saved.reduce_motion),
        "and writes it"
    );
}

/// T021 (feature 939, research R9): the shell routes a structured session's events to its
/// transcript, performs the transcript view's scroll requests, and drops the transcript with the
/// session.
mod structured_transcript {
    use super::*;
    use micold_client::features::structured_transcript::{Follow, TranscriptMsg};
    use micold_core::structured::{SequencedEvent, StructuredEvent};

    fn with_events() -> (App, SessionId) {
        let mut app = base_app();
        let _ = connect(&mut app, snapshot_with("/repo/demo", Vec::new()));
        let session = SessionId::new();
        let _ = update_inner(
            &mut app,
            Message::Connection(ConnectionMsg::Event(DaemonMsg::StructuredEvents {
                session,
                omitted: 0,
                events: vec![SequencedEvent {
                    seq: 0,
                    event: StructuredEvent::Unreadable { text: "hi".into() },
                }],
                live: false,
            })),
        );
        (app, session)
    }

    fn send(app: &mut App, msg: TranscriptMsg) -> usize {
        update_inner(app, Message::Session(SessionMsg::Transcript(msg))).units()
    }

    #[test]
    fn structured_events_reach_the_transcript() {
        let (app, s) = with_events();
        let transcript = app.core.structured_transcript.transcript(s);
        assert_eq!(
            transcript.map(|t| t.entries().len()),
            Some(1),
            "a structured events message lands in the session's transcript as one entry"
        );
    }

    #[test]
    fn the_view_messages_scroll_only_when_following_needs_it() {
        let (mut app, session) = with_events();
        let scrolled = |offset| TranscriptMsg::Scrolled {
            session,
            offset,
            viewport: 200,
            content: 1000,
        };
        assert!(
            send(&mut app, TranscriptMsg::Shown { session }) > 0,
            "snap to the end"
        );
        assert_eq!(send(&mut app, scrolled(10)), 0, "detaching moves nothing");
        assert_eq!(
            app.core.structured_transcript.follow(session),
            Follow::Detached { offset: 10 },
            "scrolling away from the end detaches the view at that offset"
        );
        assert!(
            send(&mut app, TranscriptMsg::Shown { session }) > 0,
            "back to the offset"
        );
        assert!(
            send(&mut app, scrolled(800)) > 0,
            "resumed: snap to the end"
        );
        assert!(
            send(&mut app, TranscriptMsg::JumpToLatest { session }) > 0,
            "jump to latest scrolls to the end"
        );
    }

    #[test]
    fn closing_a_session_drops_its_transcript() {
        let (mut app, s) = with_events();
        let _ = update_inner(&mut app, Message::Session(SessionMsg::CloseRequested(s)));
        assert!(
            app.core.structured_transcript.transcript(s).is_none(),
            "closing a session drops its transcript"
        );
    }
}

/// A successful reading listing `numbers` open on `branch`, a higher number newer (so drawn
/// first), each at its `o/r` address (feature 1123).
fn list_numbers(app: &mut App, branch: &str, numbers: &[u64]) {
    use micold_core::pr_list::{BranchPrList, ListedPr};
    use micold_core::pull_request::{BranchPrState, CheckStatus, ReviewState};
    let prs = numbers
        .iter()
        .map(|&number| ListedPr {
            number,
            url: format!("https://github.com/o/r/pull/{number}"),
            state: BranchPrState::Open,
            checks: CheckStatus::None,
            review: ReviewState::None,
            conflict: false,
            created_at: format!("2026-01-01T00:00:{number:02}Z"),
        })
        .collect();
    app.core.pr_status.enabled = true;
    app.core.pr_status.read_at = Some(100);
    app.core
        .pr_status
        .lists
        .insert(branch.into(), BranchPrList::new(prs, false, false));
}

// ---- Feature 1122 M4 (FR-008, FR-009, contract U6): the issue link by keyboard, at the root ----

/// The Overview on `dirs` of a project on `o/r` with the GitHub issues switch on, so a worktree
/// named for an issue has its link.
fn app_with_links(dirs: &[&str]) -> App {
    let mut app = app_in_overview_with(dirs);
    app.core.settings.github_issues_enabled = true;
    app.core.overview.repo = micold_client::features::overview::RepoLookup::Known(Some(
        micold_core::github::GithubRepo::from_remote_url("git@github.com:o/r.git")
            .expect("a GitHub remote"),
    ));
    app
}

/// Tab from `dir`'s card onto its link, as the expansion hears it.
fn tab_onto_link(app: &mut App, dir: &str) {
    use micold_client::features::overview::Msg;
    use micold_core::overview_expansion::Event;
    let l = SessionLocation::Worktree(dir.into());
    for event in [
        Event::FocusGained(l.clone()),
        Event::FocusLost(l.clone()),
        Event::LinkFocusGained(l, micold_core::overview_expansion::CardLink::Issue),
    ] {
        let _ = update_inner(app, Message::Overview(Msg::Expansion(event)));
    }
}

/// R9, contract U6: the link's Menu key opens the card's menu as `KeyboardFromLink`, the card's
/// as `Keyboard`; neither while an activation is pending or the Overview is closed.
#[test]
fn link_menu_opens_from_the_link() {
    use micold_client::features::overview::{MenuOrigin, Msg};
    let l = SessionLocation::Worktree("feat-1122_x".into());
    let mut app = app_with_links(&["feat-1122_x"]);
    assert_eq!(
        keyboard_menu_request(
            &app,
            Msg::LinkMenuRequested(l.clone(), micold_core::overview_expansion::CardLink::Issue)
        ),
        Some((
            l.clone(),
            MenuOrigin::KeyboardFromLink(micold_core::overview_expansion::CardLink::Issue)
        ))
    );
    assert_eq!(
        keyboard_menu_request(&app, Msg::CardMenuRequested(l.clone())),
        Some((l.clone(), MenuOrigin::Keyboard))
    );
    app.core.overview.open = false;
    assert_eq!(
        keyboard_menu_request(
            &app,
            Msg::LinkMenuRequested(l, micold_core::overview_expansion::CardLink::Issue)
        ),
        None
    );
}

/// FR-009: closing a menu opened from the link without a choice returns the keyboard to the
/// link; to the card when the link has gone meanwhile, or for the card's own menu.
#[test]
fn card_focus_return_focuses_the_link() {
    use micold_client::features::overview::{MenuOrigin, Msg};
    use micold_client::ui::overview::{card_id, card_link_id};
    let l = SessionLocation::Worktree("feat-1122_x".into());
    let returned = |origin: MenuOrigin, link_on: bool| {
        let mut app = app_with_links(&["feat-1122_x"]);
        open_card_menu(&mut app, "feat-1122_x", origin);
        let _ = update_inner(&mut app, Message::Overview(Msg::CardMenuDismissed));
        app.core.settings.github_issues_enabled = link_on;
        card_focus_return_id(&mut app)
    };
    assert_eq!(
        returned(
            MenuOrigin::KeyboardFromLink(micold_core::overview_expansion::CardLink::Issue),
            true
        ),
        Some(card_link_id(
            &l,
            micold_core::overview_expansion::CardLink::Issue
        ))
    );
    assert_eq!(
        returned(
            MenuOrigin::KeyboardFromLink(micold_core::overview_expansion::CardLink::Issue),
            false
        ),
        Some(card_id(&l))
    );
    assert_eq!(returned(MenuOrigin::Keyboard, true), Some(card_id(&l)));
}

/// Spec Edge Case *Jira base URL changes*, contract U6: with a link holding the keyboard, a
/// change that removes it (the GitHub issues switch off; the repository read as none) moves the
/// keyboard to its card, as a move inside the card; a change that keeps it moves nothing.
#[test]
fn link_focus_moves_to_card_when_link_goes() {
    use micold_client::features::overview::Msg;
    use micold_client::ui::overview::card_id;
    let l = SessionLocation::Worktree("feat-1122_x".into());
    /// One change that takes the link away.
    fn remove(app: &mut App, way: &str) {
        if way == "switch off" {
            app.core.settings.github_issues_enabled = false;
        } else {
            let _ = update_inner(
                app,
                Message::Overview(Msg::RepoRead {
                    project: PathBuf::from("/repo"),
                    repo: None,
                }),
            );
        }
    }
    for way in ["switch off", "no repository"] {
        let mut app = app_with_links(&["feat-1122_x"]);
        tab_onto_link(&mut app, "feat-1122_x");
        let held = LinkFocus::of(&app);
        assert_eq!(
            held.link,
            Some((l.clone(), micold_core::overview_expansion::CardLink::Issue)),
            "{way}"
        );
        remove(&mut app, way);
        assert_eq!(
            link_focus_after(&mut app, &held),
            Some(card_id(&l)),
            "{way}"
        );
        let expansion = &app.core.overview.expansion;
        assert_eq!(expansion.focused_link(), None, "{way}");
        assert_eq!(
            expansion.expanded(),
            Some(&l),
            "{way}: the card keeps its form"
        );
    }
    let mut kept = app_with_links(&["feat-1122_x"]);
    tab_onto_link(&mut kept, "feat-1122_x");
    let held = LinkFocus::of(&kept);
    assert_eq!(link_focus_after(&mut kept, &held), None, "the link stays");
}

/// Feature 1122 M5 (D3, D4, FR-012, contract U6): adopting the service's settings sets each
/// project's Jira base URL in force, and a `DaemonSettings` that clears the open project's base
/// while its Jira link holds the keyboard hands the keyboard to the link's card (T036's hand-off,
/// Jira case).
#[test]
fn jira_base_url_adopted() {
    use micold_client::ui::overview::{card_id, issue_link_for};
    use micold_core::jira::{connection_key, JiraBaseUrl};
    let l = SessionLocation::Worktree("feat-mic-87_add-login".into());
    let mut app = app_in_overview_with(&["feat-mic-87_add-login"]);
    let key = connection_key(std::path::Path::new("/repo"));
    let base = JiraBaseUrl::parse("https://example.atlassian.net").expect("valid");
    let mut settings = quiet_settings();
    settings.jira_enabled = true;
    settings.jira_base_urls.insert(key.clone(), base.clone());

    let _ = crate::shell::daemon_sync::on_settings_changed(&mut app, settings.clone());

    assert_eq!(app.core.settings.jira_base_urls.get(&key), Some(&base));
    assert_eq!(
        issue_link_for(&app.core, &l).map(|link| link.address),
        Some("https://example.atlassian.net/browse/MIC-87".to_owned())
    );
    tab_onto_link(&mut app, "feat-mic-87_add-login");
    let held = LinkFocus::of(&app);
    assert_eq!(
        held.link,
        Some((l.clone(), micold_core::overview_expansion::CardLink::Issue))
    );

    settings.jira_base_urls.clear();
    let _ = crate::shell::daemon_sync::on_settings_changed(&mut app, settings);

    assert!(app.core.settings.jira_base_urls.is_empty());
    assert_eq!(link_focus_after(&mut app, &held), Some(card_id(&l)));
    assert_eq!(app.core.overview.expansion.focused_link(), None);
}

/// Spec Edge Case *Card goes away*, contract U6: a listing without the card whose link holds
/// the keyboard moves it where feature 972 FR-011 moves it for a removed focused card.
#[test]
fn link_focus_follows_removed_card() {
    use micold_client::features::overview::Msg;
    use micold_client::ui::overview::{card_id, focus_return_target};
    let gone = SessionLocation::Worktree("feat-1122_x".into());
    let mut app = app_with_links(&["feat-7_y", "feat-1122_x"]);
    tab_onto_link(&mut app, "feat-1122_x");
    let held = LinkFocus::of(&app);
    app.core
        .worktree
        .worktrees
        .retain(|w| w.dir_name != "feat-1122_x");
    let _ = update_inner(&mut app, Message::Overview(Msg::GridScrolled));
    assert_eq!(app.core.overview.expansion.focused_link(), None);
    let target = focus_return_target(&app.core, &gone).expect("a card is left");
    assert_ne!(target, gone);
    assert_eq!(link_focus_after(&mut app, &held), Some(card_id(&target)));
}

/// One expansion event through the root's update, returning the control the keyboard is moved
/// to after it, as `update` does.
fn expansion_focus_move(
    app: &mut App,
    event: micold_core::overview_expansion::Event,
) -> Option<iced::advanced::widget::Id> {
    use micold_client::features::overview::Msg;
    let before = LinkFocus::of(app);
    let _ = update_inner(app, Message::Overview(Msg::Expansion(event)));
    link_focus_after(app, &before)
}

/// Review A F2 (M4): a project switch with a link holding the keyboard moves no focus: the
/// first card of the new project is not focused (and so not expanded, FR-013).
#[test]
fn a_project_switch_moves_no_focus_from_a_link() {
    use micold_client::features::overview::Msg;
    let mut app = app_with_links(&["feat-1122_x"]);
    tab_onto_link(&mut app, "feat-1122_x");
    let before = LinkFocus::of(&app);
    assert!(
        before.link.is_some(),
        "fixture: the link holds the keyboard"
    );
    app.core.workspace.active = Some(PathBuf::from("/other"));
    let _ = update_inner(&mut app, Message::Overview(Msg::ProjectSwitched));
    assert_eq!(link_focus_after(&mut app, &before), None);
}

/// FR-008 (visual pass r1, M4, criterion 1): Shift+Tab from card N+1 onto card N's link expands
/// card N, which rebuilds the link: the root hands the keyboard back to the link. The loss of
/// card N+1 that follows moves nothing.
#[test]
fn shift_tab_onto_the_previous_cards_link_keeps_the_keyboard_on_it() {
    use micold_client::ui::overview::card_link_id;
    use micold_core::overview_expansion::Event;
    let (a, b) = (
        SessionLocation::Worktree("feat-1122_x".into()),
        SessionLocation::Worktree("feat-1124_y".into()),
    );
    let mut app = app_with_links(&["feat-1122_x", "feat-1124_y"]);
    let _ = expansion_focus_move(&mut app, Event::FocusGained(b.clone()));
    assert_eq!(app.core.overview.expansion.expanded(), Some(&b), "fixture");
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(a.clone(), micold_core::overview_expansion::CardLink::Issue)
        ),
        Some(card_link_id(
            &a,
            micold_core::overview_expansion::CardLink::Issue
        ))
    );
    assert_eq!(app.core.overview.expansion.expanded(), Some(&a));
    assert_eq!(expansion_focus_move(&mut app, Event::FocusLost(b)), None);
    assert_eq!(
        app.core.overview.expansion.focused_link(),
        Some((&a, micold_core::overview_expansion::CardLink::Issue))
    );
}

/// FR-008 (visual pass r1, M4, criteria 6 and 7): Escape on the focused link collapses its card,
/// which rebuilds the link: the root hands the keyboard back to it, so Tab, Shift+Tab and the
/// Menu key go on from the link.
#[test]
fn escape_on_the_link_keeps_the_keyboard_on_it() {
    use micold_client::ui::overview::card_link_id;
    use micold_core::overview_expansion::Event;
    let a = SessionLocation::Worktree("feat-1122_x".into());
    let mut app = app_with_links(&["feat-1122_x"]);
    tab_onto_link(&mut app, "feat-1122_x");
    assert_eq!(app.core.overview.expansion.expanded(), Some(&a), "fixture");
    assert_eq!(
        expansion_focus_move(&mut app, Event::Escape),
        Some(card_link_id(
            &a,
            micold_core::overview_expansion::CardLink::Issue
        ))
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(expansion.expanded(), None);
    assert_eq!(
        expansion.focused_link(),
        Some((&a, micold_core::overview_expansion::CardLink::Issue))
    );
    // The hand-back's own report changes nothing.
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(a, micold_core::overview_expansion::CardLink::Issue)
        ),
        None
    );
    assert_eq!(app.core.overview.expansion.expanded(), None);
}

// ---- Feature 1123 M3 (FR-009, FR-010, contracts U5, U6): the numbers by keyboard, at the root ----

/// The card of `feat-1122_x` with its issue link and #11, #12, #13 listed: drawn #13, #12, #11,
/// so #11 is row 3, which the collapsed card does not draw.
fn app_with_numbers() -> (App, SessionLocation) {
    let mut app = app_with_links(&["feat-1122_x"]);
    list_numbers(&mut app, "feat/feat-1122_x", &[11, 12, 13]);
    (app, SessionLocation::Worktree("feat-1122_x".into()))
}

/// The keyboard on the focused, expanded card's number `n` (card lost, then number gained).
fn tab_onto_number(app: &mut App, l: &SessionLocation, n: u64) {
    use micold_core::overview_expansion::Event;
    for event in [
        Event::FocusGained(l.clone()),
        Event::FocusLost(l.clone()),
        Event::LinkFocusGained(l.clone(), CardLink::PullRequest(n)),
    ] {
        let _ = expansion_focus_move(app, event);
    }
    assert_eq!(
        app.core.overview.expansion.focused_link(),
        Some((l, CardLink::PullRequest(n))),
        "fixture: #{n} holds the keyboard"
    );
}

/// FR-010, R6: a row-3 number is drawn only while its card is expanded; a row-1 number and the
/// issue link either way; a number not listed never.
#[test]
fn card_link_shown_follows_the_form() {
    use micold_client::ui::overview::card_link_shown;
    use micold_core::overview_expansion::Event;
    let (mut app, l) = app_with_numbers();
    let shown = |app: &App, link| card_link_shown(&app.core, &l, link);
    assert!(!shown(&app, CardLink::PullRequest(11)), "row 3, collapsed");
    assert!(shown(&app, CardLink::PullRequest(13)), "row 1, collapsed");
    assert!(shown(&app, CardLink::Issue));
    let _ = expansion_focus_move(&mut app, Event::FocusGained(l.clone()));
    assert_eq!(app.core.overview.expansion.expanded(), Some(&l), "fixture");
    assert!(shown(&app, CardLink::PullRequest(11)), "row 3, expanded");
    assert!(shown(&app, CardLink::PullRequest(13)), "row 1, expanded");
    assert!(!shown(&app, CardLink::PullRequest(14)), "not listed");
}

/// U5, FR-010: Escape on a focused row-3 number collapses the card, which no longer draws it: the
/// card takes the keyboard as a move inside it, and stays collapsed, also across its own report.
#[test]
fn link_focus_escape_on_hidden_number_goes_to_card() {
    use micold_client::ui::overview::card_id;
    use micold_core::overview_expansion::Event;
    let (mut app, l) = app_with_numbers();
    tab_onto_number(&mut app, &l, 11);
    assert_eq!(app.core.overview.expansion.expanded(), Some(&l), "fixture");
    assert_eq!(
        expansion_focus_move(&mut app, Event::Escape),
        Some(card_id(&l))
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(expansion.expanded(), None);
    assert_eq!(expansion.focused_link(), None);
    assert_eq!(
        expansion_focus_move(&mut app, Event::FocusGained(l.clone())),
        None
    );
    assert_eq!(
        app.core.overview.expansion.expanded(),
        None,
        "stays collapsed"
    );
}

/// U5, FR-010: Escape on a focused row-1 number collapses the card, which still draws it: the
/// keyboard is handed back to the number by its id.
#[test]
fn link_focus_escape_on_row_one_number_stays() {
    use micold_client::ui::overview::card_link_id;
    use micold_core::overview_expansion::Event;
    let (mut app, l) = app_with_numbers();
    tab_onto_number(&mut app, &l, 13);
    assert_eq!(
        expansion_focus_move(&mut app, Event::Escape),
        Some(card_link_id(&l, CardLink::PullRequest(13)))
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(expansion.expanded(), None);
    assert_eq!(
        expansion.focused_link(),
        Some((&l, CardLink::PullRequest(13)))
    );
}

/// U5, spec Edge Cases: a reading without the pull request whose number holds the keyboard moves
/// the keyboard to its card, as a move inside the card; a reading that keeps it moves nothing.
#[test]
fn link_focus_moves_to_card_when_pull_request_goes() {
    use micold_client::ui::overview::card_id;
    let (mut app, l) = app_with_numbers();
    tab_onto_number(&mut app, &l, 12);
    let held = LinkFocus::of(&app);
    list_numbers(&mut app, "feat/feat-1122_x", &[10, 11, 12, 13]);
    assert_eq!(link_focus_after(&mut app, &held), None, "#12 stays");
    let held = LinkFocus::of(&app);
    list_numbers(&mut app, "feat/feat-1122_x", &[11, 13]);
    assert_eq!(link_focus_after(&mut app, &held), Some(card_id(&l)));
    let expansion = &app.core.overview.expansion;
    assert_eq!(expansion.focused_link(), None);
    assert_eq!(expansion.expanded(), Some(&l), "the card keeps its form");
}

/// Review A M3 r1 F1, R6: a reading that moves the focused number to another row (a newer pull
/// request sorts above it) hands the keyboard back to that number by its id, since the widget
/// that held it now draws another one; the stale report of the row's new number moves nothing.
#[test]
fn a_reading_that_moves_the_focused_number_refocuses_it() {
    use micold_client::ui::overview::card_link_id;
    let (mut app, l) = app_with_numbers();
    tab_onto_number(&mut app, &l, 12);
    let held = LinkFocus::of(&app);
    list_numbers(&mut app, "feat/feat-1122_x", &[11, 12, 13, 14]);
    assert_eq!(
        link_focus_after(&mut app, &held),
        Some(card_link_id(&l, CardLink::PullRequest(12)))
    );
    assert_eq!(
        expansion_focus_move(
            &mut app,
            micold_core::overview_expansion::Event::LinkFocusLost(
                l.clone(),
                CardLink::PullRequest(13)
            )
        ),
        None
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(
        expansion.focused_link(),
        Some((&l, CardLink::PullRequest(12)))
    );
    assert_eq!(expansion.expanded(), Some(&l));
}

/// Visual pass M3 r1, criteria 1 and 3, FR-010: iced reports a forward Tab between two numbers in
/// tree order, the loss before the gain. The loss of a row-3 number collapses the card until the
/// gain pairs with it, so for that moment the number is not drawn; that is the keyboard leaving,
/// not a number taken away under it: the root moves nothing (it used to hand the keyboard to the
/// card, so Tab from row 3 went back to the card, and Right from it too).
#[test]
fn a_move_off_a_row_three_number_moves_no_focus_to_the_card() {
    use micold_core::overview_expansion::Event;
    let (mut app, l) = app_with_numbers();
    list_numbers(&mut app, "feat/feat-1122_x", &[10, 11, 12, 13]);
    tab_onto_number(&mut app, &l, 11);
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusLost(l.clone(), CardLink::PullRequest(11))
        ),
        None,
        "Tab or an arrow off #11"
    );
    // The gain pairs with the loss and the card takes its form back: the root re-focuses #10 by
    // its id, as on any change of form (a no-op when it still holds the keyboard), and not the
    // expanded card's last number: no other card held the keyboard.
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(l.clone(), CardLink::PullRequest(10))
        ),
        Some(micold_client::ui::overview::card_link_id(
            &l,
            CardLink::PullRequest(10)
        ))
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(
        expansion.focused_link(),
        Some((&l, CardLink::PullRequest(10)))
    );
    assert_eq!(expansion.expanded(), Some(&l));
}

/// A forward Tab from row 1 to row 2, the collapsed card's last number, is no Shift+Tab into the
/// card: the keyboard stays on row 2.
#[test]
fn a_tab_onto_row_two_stays_on_it() {
    use micold_client::ui::overview::card_link_id;
    use micold_core::overview_expansion::Event;
    let (mut app, l) = app_with_numbers();
    list_numbers(&mut app, "feat/feat-1122_x", &[10, 11, 12, 13]);
    tab_onto_number(&mut app, &l, 13);
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusLost(l.clone(), CardLink::PullRequest(13))
        ),
        None
    );
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(l.clone(), CardLink::PullRequest(12))
        ),
        Some(card_link_id(&l, CardLink::PullRequest(12)))
    );
}

/// Visual pass M3 r1, criterion 1, FR-010 (Shift+Tab is the reverse of Tab): Shift+Tab from the
/// next card lands on the collapsed card's last drawn number, row 2; the card then expands, and
/// the root moves the keyboard on to the expanded card's last number, where Tab left it.
#[test]
fn shift_tab_onto_a_collapsed_cards_number_goes_to_its_last() {
    use micold_client::ui::overview::card_link_id;
    use micold_core::overview_expansion::Event;
    let (a, b) = (
        SessionLocation::Worktree("feat-1122_x".into()),
        SessionLocation::Worktree("feat-1124_y".into()),
    );
    let mut app = app_with_links(&["feat-1122_x", "feat-1124_y"]);
    list_numbers(&mut app, "feat/feat-1122_x", &[10, 11, 12, 13]);
    let _ = expansion_focus_move(&mut app, Event::FocusGained(b.clone()));
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(a.clone(), CardLink::PullRequest(12))
        ),
        Some(card_link_id(&a, CardLink::PullRequest(10)))
    );
    assert_eq!(expansion_focus_move(&mut app, Event::FocusLost(b)), None);
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(a.clone(), CardLink::PullRequest(10))
        ),
        None
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(
        expansion.focused_link(),
        Some((&a, CardLink::PullRequest(10)))
    );
    assert_eq!(expansion.expanded(), Some(&a));
}

/// FR-010, visual pass M3 r2 criterion 1: Shift+Tab from a focus-expanded card reports its loss
/// first (the grid updates the expanded card first; `ui::overview` test
/// `shift_tab_off_an_expanded_card_reports_its_loss_first`), so no card holds the keyboard when the
/// previous card's collapsed last number gains it: the keyboard still goes on to the expanded
/// card's last number, and the move inside that card (#12 lost, #10 gained) keeps #10.
#[test]
fn shift_tab_loss_first_onto_a_collapsed_cards_number_goes_to_its_last() {
    use micold_client::ui::overview::card_link_id;
    use micold_core::overview_expansion::Event;
    let (a, b) = (
        SessionLocation::Worktree("feat-1122_x".into()),
        SessionLocation::Worktree("feat-1124_y".into()),
    );
    let mut app = app_with_links(&["feat-1122_x", "feat-1124_y"]);
    list_numbers(&mut app, "feat/feat-1122_x", &[10, 11, 12, 13]);
    let _ = expansion_focus_move(&mut app, Event::FocusGained(b.clone()));
    assert_eq!(app.core.overview.expansion.expanded(), Some(&b));
    assert_eq!(expansion_focus_move(&mut app, Event::FocusLost(b)), None);
    assert_eq!(
        expansion_focus_move(
            &mut app,
            Event::LinkFocusGained(a.clone(), CardLink::PullRequest(12))
        ),
        Some(card_link_id(&a, CardLink::PullRequest(10)))
    );
    let numbers = [12, 10].map(CardLink::PullRequest);
    assert_eq!(
        expansion_focus_move(&mut app, Event::LinkFocusLost(a.clone(), numbers[0])),
        None
    );
    assert_eq!(
        expansion_focus_move(&mut app, Event::LinkFocusGained(a.clone(), numbers[1])),
        Some(card_link_id(&a, numbers[1]))
    );
    let expansion = &app.core.overview.expansion;
    assert_eq!(expansion.focused_link(), Some((&a, numbers[1])));
    assert_eq!(expansion.expanded(), Some(&a));
}

/// U6, FR-009: a number's Menu key opens the card's menu as `KeyboardFromLink` of that number.
#[test]
fn link_menu_opens_from_a_number() {
    use micold_client::features::overview::{MenuOrigin, Msg};
    let (app, l) = app_with_numbers();
    let number = CardLink::PullRequest(12);
    assert_eq!(
        keyboard_menu_request(&app, Msg::LinkMenuRequested(l.clone(), number)),
        Some((l, MenuOrigin::KeyboardFromLink(number)))
    );
}

/// U6, FR-009: closing a menu opened from a number without a choice returns the keyboard to the
/// number; a choice returns it to the card, and so does a number no longer drawn on close.
#[test]
fn card_focus_return_focuses_the_number() {
    use micold_client::features::overview::{MenuOrigin, Msg};
    use micold_client::ui::overview::{card_id, card_link_id};
    let number = CardLink::PullRequest(13);
    let returned = |close: Msg, listed: &[u64]| {
        let (mut app, _) = app_with_numbers();
        open_card_menu(
            &mut app,
            "feat-1122_x",
            MenuOrigin::KeyboardFromLink(number),
        );
        list_numbers(&mut app, "feat/feat-1122_x", listed);
        let _ = update_inner(&mut app, Message::Overview(close));
        card_focus_return_id(&mut app)
    };
    let l = SessionLocation::Worktree("feat-1122_x".into());
    assert_eq!(
        returned(Msg::CardMenuDismissed, &[11, 12, 13]),
        Some(card_link_id(&l, number))
    );
    assert_eq!(
        returned(
            Msg::CardMenuEntryChosen(Box::new(Message::Overview(Msg::GridScrolled))),
            &[11, 12, 13]
        ),
        Some(card_id(&l))
    );
    assert_eq!(
        returned(Msg::CardMenuDismissed, &[11, 12]),
        Some(card_id(&l)),
        "#13 is gone"
    );
}
