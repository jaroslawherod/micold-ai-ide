//! T012 — wire round-trip fidelity (contracts/messages.md, protocol.md §3).
//!
//! Every `ClientMsg`/`DaemonMsg` round-trips under JSON (the control plane), every grid type
//! round-trips under `postcard` (the grid plane), and a `GridFrame` survives encode→decode
//! byte-identical with its wide-char spacer and zerowidth marks intact.

use std::path::PathBuf;

use micold_core::attach::{
    AttachItem, AttachOutcome, AttachResult, AttachTarget, DiscoveryNote, DiscoveryReport,
    RefuseReason, ResumableSession, ResumableStatus, SkipReason, UnresumableReason,
};
use micold_core::attention::{NotificationKind, NotificationKinds};
use micold_core::cli_reason::SpawnEnv;
use micold_core::git::GitRemote;
use micold_core::mcp::policy::CrossSessionAccess;
use micold_core::occupancy::ExistingSession;

use micold_core::protocol::envelope::{Encoding, EnvelopeError, EnvelopeHeader, Kind, HEADER_LEN};
use micold_core::protocol::grid::{
    CellExtras, GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine,
    WireStyle,
};
use micold_core::protocol::messages::PrSend;
use micold_core::protocol::messages::SessionDefinition;
use micold_core::protocol::messages::{
    ActivitySignal, BranchContainment, CatalogSnapshot, ClientIdentity, ClientInstance, ClientMsg,
    ConfirmOperation, DaemonMsg, DaemonSettings, ErrorKind, ExitStatus, LogEntry, LogSink,
    MergedBranchQuery, OperationResult, ProjectSnapshot, RefusalReason, SessionProcess,
    SessionSummary, ShellOpenFailure, TerminalRef, WireLifecycle, WorktreeSnapshot, WorktreeStatus,
};
use micold_core::protocol::messages::{
    PermissionRequest, PromptChoice, PromptId, QuestionAnswer, Resolution, WithdrawReason,
};
use micold_core::pull_request::BranchPrState;
use micold_core::session::PermissionMode;
use micold_core::session::{AiCli, SessionId, SessionLabel, ShellInstanceId};
use micold_core::structured::permission::{Question, QuestionOption, RequestKind};
use micold_core::theme::ColorScheme;
use micold_core::worktree::{CreateMode, CreateStage};
use uuid::Uuid;

fn sid() -> SessionId {
    SessionId::from_uuid(Uuid::from_u128(0x1234_5678_9abc_def0_1122_3344_5566_7788))
}

fn json_roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let bytes = serde_json::to_vec(value).expect("json encode");
    let back: T = serde_json::from_slice(&bytes).expect("json decode");
    assert_eq!(&back, value, "json round-trip mismatch");
}

fn postcard_roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let bytes = postcard::to_stdvec(value).expect("postcard encode");
    let back: T = postcard::from_bytes(&bytes).expect("postcard decode");
    assert_eq!(&back, value, "postcard round-trip mismatch");
}

fn sample_client_msgs() -> Vec<ClientMsg> {
    vec![
        ClientMsg::RepoClone {
            url: "git@github.com:acme/app.git".into(),
            dest: "/data/repos/app-1a2b3c4d".into(),
        },
        ClientMsg::GitCredentialGrant {
            repo: scoped_repo(),
            token: micold_core::scoped_credential::SecretToken::new("ghs_wire_secret"),
            expires_at: 1_900_000_000,
        },
        ClientMsg::GitCredentialRevoke {
            repo: scoped_repo(),
        },
        ClientMsg::Hello {
            protocol_version: 1,
            schema_hash: [7u8; 32],
            client_build: "client-abc".into(),
            client_instance: micold_core::protocol::messages::ClientInstance::current(),
            client_package_version: "0.4.0".into(),
            // Feature 027. `Some` here on purpose: an `Option` that is only ever encoded as `None`
            // in the round-trip sample would not prove the token survives the wire.
            auth_token: Some(micold_core::protocol::messages::PresentedToken::new(
                "a".repeat(64),
            )),
            client_fingerprint: "b7f3a1c9".into(),
            require_fingerprint_match: true,
        },
        ClientMsg::Attach {
            project: PathBuf::from("/repo"),
            force: true,
        },
        ClientMsg::Detach {
            project: PathBuf::from("/repo"),
        },
        ClientMsg::Goodbye,
        // `006` BUG-007: both schemes, so neither is the only value the codec ever sees.
        ClientMsg::TerminalColorScheme {
            scheme: ColorScheme::Dark,
        },
        ClientMsg::TerminalColorScheme {
            scheme: ColorScheme::Light,
        },
        // Feature 039, W1.1: a session in view, nothing in view, and a window without focus.
        ClientMsg::WindowView {
            focused: true,
            in_view: Some(sid()),
        },
        ClientMsg::WindowView {
            focused: true,
            in_view: None,
        },
        ClientMsg::WindowView {
            focused: false,
            in_view: None,
        },
        ClientMsg::SessionInput {
            session: sid(),
            serial: 42,
            bytes: vec![0x1b, b'[', b'A'],
            process: None,
        },
        ClientMsg::SessionInput {
            session: sid(),
            serial: 43,
            bytes: vec![b'x'],
            process: Some(SessionProcess::Shell(ShellInstanceId(2))),
        },
        ClientMsg::SessionResize {
            session: sid(),
            cols: 120,
            rows: 40,
            process: None,
        },
        ClientMsg::SessionResize {
            session: sid(),
            cols: 80,
            rows: 24,
            process: Some(SessionProcess::Primary),
        },
        ClientMsg::SetViewedTerminals {
            project: PathBuf::from("/p"),
            terminals: vec![
                TerminalRef {
                    session: sid(),
                    process: SessionProcess::Primary,
                },
                TerminalRef {
                    session: sid(),
                    process: SessionProcess::Shell(ShellInstanceId(1)),
                },
            ],
        },
        ClientMsg::SetPaneLayout {
            project: PathBuf::from("/p"),
            layout: Some("{\"layout_version\":1}".into()),
        },
        ClientMsg::SetPaneLayout {
            project: PathBuf::from("/p"),
            layout: None,
        },
        ClientMsg::SessionStart {
            session: sid(),
            allow_additional_session: false,
        },
        ClientMsg::SessionStart {
            session: sid(),
            allow_additional_session: true,
        },
        ClientMsg::SessionRestart {
            session: sid(),
            allow_additional_session: false,
        },
        ClientMsg::SessionRestart {
            session: sid(),
            allow_additional_session: true,
        },
        ClientMsg::SessionStop { session: sid() },
        ClientMsg::SessionKill { session: sid() },
        ClientMsg::SessionInterrupt { session: sid() },
        ClientMsg::SessionAttachProcess {
            session: sid(),
            process: micold_core::protocol::messages::SessionProcess::Shell(
                micold_core::session::ShellInstanceId(3),
            ),
        },
        ClientMsg::SessionOpenShell {
            session: sid(),
            instance: micold_core::session::ShellInstanceId(3),
        },
        ClientMsg::SessionCloseShell {
            session: sid(),
            instance: micold_core::session::ShellInstanceId(3),
        },
        ClientMsg::SessionRestartShell {
            session: sid(),
            instance: micold_core::session::ShellInstanceId(3),
        },
        ClientMsg::SetViewedSession {
            project: PathBuf::from("/repo"),
            session: Some(sid()),
        },
        ClientMsg::ScrollbackRequest {
            session: sid(),
            req: 9,
            ranges: vec![LineId(10)..LineId(20), LineId(100)..LineId(150)],
        },
        ClientMsg::ProjectAdd {
            req: 1,
            path: PathBuf::from("/a"),
        },
        ClientMsg::ProjectActivate {
            req: 1,
            path: PathBuf::from("/a"),
        },
        ClientMsg::ProjectRemove {
            req: 2,
            path: PathBuf::from("/a"),
        },
        ClientMsg::ProjectRename {
            req: 3,
            path: PathBuf::from("/a"),
            display_name: "Alpha".into(),
        },
        ClientMsg::WorktreeCreate {
            req: 4,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::NewBranch,
            session: None,
        },
        // Feature 776: a definition travels whole.
        ClientMsg::WorktreeCreate {
            req: 40,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::NewBranch,
            session: Some(SessionDefinition {
                ai_cli: AiCli::ClaudeCode,
                prompt: Some("fix it".into()),
            }),
        },
        // Feature 016: the non-default modes and both read-only branch queries must survive the
        // wire too — `TrackRemote` carries a payload, so it is the interesting one.
        ClientMsg::WorktreeCreate {
            req: 41,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::ReuseLocal,
            session: None,
        },
        ClientMsg::WorktreeCreate {
            req: 42,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::TrackRemote {
                remote: "origin".into(),
            },
            session: None,
        },
        ClientMsg::BranchPreflight {
            req: 43,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
        },
        ClientMsg::BranchList {
            req: 44,
            project: PathBuf::from("/a"),
        },
        ClientMsg::RemoteList {
            req: 45,
            project: PathBuf::from("/a"),
        },
        // Feature 040 (FR-015): one query per merged branch, and the form with none.
        ClientMsg::MergedBranchCheck {
            req: 46,
            project: PathBuf::from("/a"),
            checks: merged_branch_queries(),
        },
        ClientMsg::MergedBranchCheck {
            req: 47,
            project: PathBuf::from("/a"),
            checks: vec![],
        },
        ClientMsg::WorktreeDelete {
            req: 5,
            project: PathBuf::from("/a"),
            dir_name: "feat-x".into(),
            stop_sessions: false,
            delete_branch: true,
        },
        ClientMsg::WorktreeRename {
            req: 6,
            project: PathBuf::from("/a"),
            dir_name: "feat-x".into(),
            display_name: "X".into(),
        },
        // Feature 029: the user-initiated re-read. Correlated like the read-only RPCs above,
        // because its reply is the only thing that ends the button's busy state (029 FR-007).
        ClientMsg::WorktreeRefresh {
            req: 45,
            project: PathBuf::from("/a"),
        },
        // Feature 582 (T003): the attach messages, with both item kinds on the wire.
        ClientMsg::AttachDiscover {
            req: 82,
            project: PathBuf::from("/a"),
        },
        ClientMsg::AttachApply {
            req: 83,
            project: PathBuf::from("/a"),
            targets: vec![
                AttachItem::Session {
                    id: Uuid::nil(),
                    replace: true,
                    allow_additional_session: false,
                },
                AttachItem::Session {
                    id: Uuid::nil(),
                    replace: false,
                    allow_additional_session: true,
                },
            ],
        },
        ClientMsg::SessionCreate {
            req: 7,
            project: PathBuf::from("/a"),
            worktree_dir: "feat-x".into(),
            provider: AiCli::ClaudeCode,
            kind: Default::default(),
            permission_mode: Default::default(),
            allow_additional_session: false,
        },
        // The same message on the second provider (feature 026, T021). Both variants ride the
        // wire, not just the default one — an encoding that only ever saw `ClaudeCode` would
        // round-trip fine and still lose `Copilot`.
        ClientMsg::SessionCreate {
            req: 71,
            project: PathBuf::from("/a"),
            worktree_dir: "feat-y".into(),
            provider: AiCli::Copilot,
            kind: Default::default(),
            permission_mode: Default::default(),
            allow_additional_session: true,
        },
        ClientMsg::SessionDelete {
            req: 8,
            session: sid(),
        },
        ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 9,
            scrollback_lines: Some(50_000),
            env_include_enabled: Some(false),
            env_include_script_path: Some("/custom/rc".into()),
            env_include_timeout_secs: Some(20),
            default_ai_cli: Some(AiCli::Copilot),
            pi_activity_component: Some(false),
            save_terminal_history: Some(false),
            // Feature 034 (FR-004): turning the binding off must cross the wire as `false`.
            tool_server_enabled: Some(false),
            // Feature 034 (FR-016): the option crosses the wire as the value chosen.
            cross_session_access: Some(CrossSessionAccess::ConfirmEachSend),
            pr_status_enabled: Some(true),
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: Some(false),
            jira_base_url: None,
            desktop_notifications: Some(false),
            // Feature 613 (W5.4, W5.6): the whole set of kinds, and the threshold in seconds.
            notification_kinds: Some(NotificationKinds {
                needs_permission: false,
                session_error: true,
                long_task_finished: false,
                turn_finished: true,
            }),
            long_task_threshold_secs: Some(20),
            diff_layout: None,
        },
        // And the "leave it unchanged" form, which is what every settings save that is not about
        // the AI CLI sends.
        ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 91,
            scrollback_lines: Some(50_000),
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: None,
            jira_base_url: None,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        },
        ClientMsg::LogLocationRequest { req: 10 },
        ClientMsg::RecentErrorsRequest { req: 11, limit: 20 },
        ClientMsg::SetLogLevel {
            req: 12,
            directives: "micold_daemon=debug".into(),
        },
        ClientMsg::TurnInterrupt {
            session: sid(),
            turn: micold_core::structured::TurnId(9),
        },
        ClientMsg::QueueEdit {
            req: 90,
            session: sid(),
            entry: micold_core::structured::QueueEntryId(1),
            text: "edited".into(),
            keep: vec![1],
        },
        ClientMsg::QueueRemove {
            req: 91,
            session: sid(),
            entry: micold_core::structured::QueueEntryId(2),
        },
        ClientMsg::QueueTake {
            req: 92,
            session: sid(),
            entry: micold_core::structured::QueueEntryId(3),
        },
        ClientMsg::QueueRelease {
            req: 93,
            session: sid(),
        },
        ClientMsg::Ping { nonce: 0xdead_beef },
        // Feature 034 M5 (FR-014): a window's answer to an agent's destructive request.
        ClientMsg::ConfirmationAnswer { id: 7, allow: true },
        ClientMsg::ConfirmationAnswer {
            id: 8,
            allow: false,
        },
    ]
}

fn sample_summary() -> SessionSummary {
    SessionSummary {
        id: sid(),
        worktree_dir: Some("feat-x".into()),
        provider: AiCli::Copilot,
        kind: Default::default(),
        // Not the default, so a field that never encoded would read back `Default` (#942).
        permission_mode: PermissionMode::AcceptEdits,
        title: SessionLabel::Named("Fix login".into()),
        lifecycle: WireLifecycle::Failed {
            reason: "crash loop".into(),
            attempts: 3,
        },
        activity: ActivitySignal::AwaitingInput,
        // Non-zero on purpose: a summary for a session the daemon has already accepted input for is
        // the case that matters (FR-028a), and a zero here would let a dropped field round-trip.
        input_serial: 4_096,
        // Two, for the same reason: an empty vec would survive a field that never encoded
        // (`012` FR-008, BUG-003).
        live_shells: vec![ShellInstanceId(1), ShellInstanceId(7)],
        // Non-zero, as above: a field that never encoded would read back as `0` (feature 039).
        attention_seq: 5,
        // `true`, for the same reason: a field that never encoded would read back as `false`.
        unread: true,
        // Present on purpose: a field that never encoded would read back as `None` (feature 551).
        origin: Some(micold_core::origin::SessionOrigin {
            id: SessionId::from_uuid(Uuid::from_u128(0xabcd)),
            status: micold_core::origin::OriginStatus::Removed,
        }),
        // One run no default produces: a field that never encoded would read back empty (552).
        agents: vec![micold_core::agent_run::AgentRun {
            agent_id: "a15f822c8aa66a621".into(),
            agent_type: Some("Explore".into()),
            transcript: None,
            state: micold_core::agent_run::AgentRunState::Running,
            provider: AiCli::ClaudeCode,
        }],
        main: false,
        paused_on_user: false,
        hook_activity: None,
    }
}

fn sample_catalog() -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from("/a")),
        projects: vec![ProjectSnapshot {
            pane_layout: Some("{\"layout_version\":1}".into()),
            path: PathBuf::from("/a"),
            display_name: "Alpha".into(),
            is_git_repo: true,
            available: true,
            worktrees: vec![WorktreeSnapshot {
                dir_name: "feat-x".into(),
                branch: Some("feat/x".into()),
                display_name: "X".into(),
                status: WorktreeStatus::Clean,
                path: PathBuf::from("/a/.claude/worktrees/feat-x"),
                included: false,
            }],
            sessions: vec![sample_summary()],
        }],
        // Empty here: the per-directory failures have their own round-trip test below (BUG-454).
        env_include_failures: Vec::new(),
    }
}

fn wide_char_line() -> WireLine {
    // A wide char (CJK '世') occupies two cells: the char cell + a spacer sentinel. An emoji with a
    // combining ZWJ sequence rides in `extras`.
    WireLine {
        id: LineId(3),
        text: "世\u{0}a".to_string(), // '世', WIDE_CHAR_SPACER sentinel, 'a'
        runs: vec![StyleRun { len: 3, style: 0 }],
        extras: vec![CellExtras {
            col: 2,
            zerowidth: vec!['\u{200d}', '\u{2764}'],
            hyperlink: Some(0),
        }],
        wrapped: true,
    }
}

fn sample_grid_frame() -> GridFrame {
    GridFrame {
        session: sid(),
        process: SessionProcess::Primary,
        seq: 7,
        generation: 2,
        full: true,
        viewport_top: LineId(0),
        oldest_available: LineId(-5),
        cols: 80,
        rows: 24,
        cursor: WireCursor {
            line: LineId(3),
            col: 4,
            shape: WireCursorShape::Beam,
            visible: true,
            blinking: false,
        },
        styles: vec![WireStyle {
            fg: WireColor::Rgb(200, 100, 50),
            bg: WireColor::Named(0),
            flags: 0b0000_0011,
            underline_color: Some(WireColor::Indexed(9)),
        }],
        hyperlinks: vec!["https://example.com".into()],
        lines: vec![wide_char_line()],
        mode: 0x0000_00ff,
        input_serial: Some(41),
    }
}

fn sample_daemon_msgs() -> Vec<DaemonMsg> {
    vec![
        DaemonMsg::Welcome {
            daemon_build: "daemon-abc".into(),
            catalog: sample_catalog(),
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                plan_usage_enabled: true,
                plan_usage_warn_percent: 80,
                scrollback_lines: 10_000,
                env_include_enabled: true,
                env_include_script_path: "/home/user/.bashrc".into(),
                env_include_timeout_secs: 10,
                default_ai_cli: AiCli::ClaudeCode,
                pi_activity_component: true,
                save_terminal_history: true,
                tool_server_enabled: true,
                cross_session_access: CrossSessionAccess::Auto,
                pr_status_enabled: true,
                desktop_notifications: true,
                notification_kinds: NotificationKinds {
                    turn_finished: true,
                    ..NotificationKinds::default()
                },
                long_task_threshold_secs: 120,
                diff_layout: Default::default(),
            },
            plan_usage: Some(sample_usage_reading()),
        },
        DaemonMsg::Refused {
            reason: RefusalReason::ProjectBusy {
                project: PathBuf::from("/a"),
                holder: ClientIdentity::new("other", ClientInstance::current()),
                since_secs: 120,
            },
        },
        // Feature 027's two refusals. `AuthRejected` is fieldless on purpose — a refusal that
        // described *how* wrong the token was would be an oracle for recovering it.
        DaemonMsg::Refused {
            reason: RefusalReason::AuthRejected,
        },
        DaemonMsg::Refused {
            reason: RefusalReason::StaleDevImage {
                client_fingerprint: "b7f3a1c9".into(),
                daemon_fingerprint: "0011223344556677".into(),
                image: "micold-daemon:dev".into(),
            },
        },
        DaemonMsg::Attached {
            project: PathBuf::from("/a"),
            sessions: vec![sample_summary()],
        },
        DaemonMsg::Displaced {
            project: PathBuf::from("/a"),
            by: ClientIdentity::new("other-client", ClientInstance::current()),
        },
        DaemonMsg::Pong { nonce: 5 },
        DaemonMsg::OperationProgress {
            req: 7,
            stage: CreateStage::CreatingWorktree,
            detail: None,
        },
        // The detail-carrying shape too (BUG-009, T123): a live line rides the same frame, so both
        // variants have to survive the round trip.
        DaemonMsg::OperationProgress {
            req: 7,
            stage: CreateStage::SettingUpSubmodules,
            detail: Some("Receiving objects:  47% (470/1000)".into()),
        },
        DaemonMsg::CatalogChanged {
            catalog: sample_catalog(),
        },
        DaemonMsg::SessionChanged {
            session: sid(),
            summary: sample_summary(),
        },
        DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                plan_usage_enabled: true,
                plan_usage_warn_percent: 80,
                scrollback_lines: 1_000,
                env_include_enabled: false,
                env_include_script_path: String::new(),
                env_include_timeout_secs: 5,
                default_ai_cli: AiCli::Pi,
                pi_activity_component: false,
                save_terminal_history: false,
                tool_server_enabled: false,
                cross_session_access: CrossSessionAccess::Off,
                pr_status_enabled: false,
                desktop_notifications: true,
                notification_kinds: NotificationKinds {
                    turn_finished: true,
                    ..NotificationKinds::default()
                },
                long_task_threshold_secs: 120,
                diff_layout: Default::default(),
            },
        },
        DaemonMsg::SessionTitleChanged {
            session: sid(),
            title: Some("t".into()),
        },
        DaemonMsg::SessionBell { session: sid() },
        DaemonMsg::SessionExited {
            session: sid(),
            status: ExitStatus {
                code: Some(1),
                signal: None,
            },
            restarting: true,
        },
        DaemonMsg::ClipboardStore {
            session: sid(),
            content: "clip".into(),
        },
        // BUG-592 (feature 010): both refusal reasons.
        DaemonMsg::ShellOpenFailed {
            session: sid(),
            instance: ShellInstanceId(2),
            reason: ShellOpenFailure::WorkingDirMissing,
        },
        DaemonMsg::ShellOpenFailed {
            session: sid(),
            instance: ShellInstanceId(3),
            reason: ShellOpenFailure::Other("no such session in the catalog".into()),
        },
        DaemonMsg::ScrollbackResponse {
            session: sid(),
            req: 9,
            oldest_available: LineId(-5),
            newest: LineId(100),
            lines: vec![wide_char_line()],
            styles: vec![],
            hyperlinks: vec!["https://example.com".into()],
            more: true,
        },
        DaemonMsg::OperationOk {
            req: 7,
            result: OperationResult::SessionCreated { session: sid() },
        },
        // Feature 582 (T003): the discovery answer and the per-target attach results.
        DaemonMsg::AttachReport {
            req: 82,
            report: DiscoveryReport {
                sessions: vec![ResumableSession {
                    id: Uuid::nil(),
                    provider: AiCli::Pi,
                    title: Some("t".into()),
                    last_activity: std::time::UNIX_EPOCH + std::time::Duration::from_secs(5),
                    target: AttachTarget::Worktree {
                        dir_name: "w".into(),
                    },
                    status: ResumableStatus::Unresumable(UnresumableReason::NoLocation),
                }],
                notes: vec![DiscoveryNote {
                    provider: None,
                    path: Some(PathBuf::from("/x")),
                    reason: SkipReason::SandboxStoreNotReadable,
                }],
            },
        },
        DaemonMsg::OperationOk {
            req: 44,
            result: OperationResult::WorktreeCreated {
                dir_name: "feat-x".into(),
                session: Some(sid()),
            },
        },
        DaemonMsg::OperationOk {
            req: 83,
            result: OperationResult::AttachApplied {
                results: vec![
                    AttachResult {
                        item: AttachItem::Session {
                            id: Uuid::nil(),
                            replace: false,
                            allow_additional_session: false,
                        },
                        outcome: AttachOutcome::AlreadyAttached,
                        existing: None,
                    },
                    AttachResult {
                        item: AttachItem::Session {
                            id: Uuid::nil(),
                            replace: false,
                            allow_additional_session: false,
                        },
                        outcome: AttachOutcome::Refused(RefuseReason::LocationHasSession),
                        existing: Some(ExistingSession {
                            id: sid(),
                            name: "Fix login".into(),
                        }),
                    },
                ],
            },
        },
        DaemonMsg::OperationOk {
            req: 8,
            result: OperationResult::RemoteList {
                remotes: vec![
                    GitRemote {
                        name: "origin".into(),
                        url: "git@github.com:o/r.git".into(),
                    },
                    GitRemote {
                        name: "upstream".into(),
                        url: "https://gitlab.com/u/r.git".into(),
                    },
                ],
            },
        },
        DaemonMsg::OperationOk {
            req: 9,
            result: OperationResult::RemoteList { remotes: vec![] },
        },
        // Feature 040 (FR-015, FR-017): every answer the daemon can give, in one reply.
        DaemonMsg::OperationOk {
            req: 46,
            result: OperationResult::MergedBranchCheck {
                answers: vec![
                    BranchContainment::Contained,
                    BranchContainment::Beyond,
                    BranchContainment::Unknown,
                ],
            },
        },
        DaemonMsg::OperationError {
            req: 4,
            kind: ErrorKind::GitFailed,
            message: "worktree create failed".into(),
            detail: Some("fatal: branch 'feat/x' already exists".into()),
        },
        // Feature 034 M5 (FR-014): the prompt every window shows, once per operation kind, and
        // its withdrawal.
        DaemonMsg::ConfirmationRequested {
            id: 1,
            project: PathBuf::from("/a"),
            caller: sid(),
            caller_label: "planner".into(),
            operation: ConfirmOperation::DeleteWorktree {
                stop_sessions: true,
                delete_branch: false,
            },
            target_label: "feat-x".into(),
            expires_in_ms: 60_000,
        },
        DaemonMsg::ConfirmationRequested {
            id: 2,
            project: PathBuf::from("/a"),
            caller: sid(),
            caller_label: "planner".into(),
            operation: ConfirmOperation::DeleteSession,
            target_label: "reviewer".into(),
            expires_in_ms: 59_000,
        },
        DaemonMsg::ConfirmationRequested {
            id: 3,
            project: PathBuf::from("/a"),
            caller: sid(),
            caller_label: "planner".into(),
            operation: ConfirmOperation::StopSession,
            target_label: "reviewer".into(),
            expires_in_ms: 1,
        },
        DaemonMsg::ConfirmationRequested {
            id: 4,
            project: PathBuf::from("/a"),
            caller: sid(),
            caller_label: "planner".into(),
            operation: ConfirmOperation::InterruptSession,
            target_label: "reviewer".into(),
            expires_in_ms: 0,
        },
        DaemonMsg::ConfirmationRequested {
            id: 5,
            project: PathBuf::from("/a"),
            caller: sid(),
            caller_label: "planner".into(),
            operation: ConfirmOperation::SendInput,
            target_label: "reviewer".into(),
            expires_in_ms: 30_000,
        },
        DaemonMsg::ConfirmationWithdrawn { id: 1 },
        DaemonMsg::SessionErrorNotice {
            project: PathBuf::from("/a"),
            session: sid(),
        },
        DaemonMsg::LogLocation {
            req: 10,
            path: Some(PathBuf::from("/var/log/micold/daemon.log")),
            sink: LogSink::File,
        },
        DaemonMsg::RecentErrors {
            req: 11,
            entries: vec![LogEntry {
                timestamp_secs: 1_700_000_000,
                level: "ERROR".into(),
                target: "micold_daemon::supervisor".into(),
                message: "session gave up after 3 attempts".into(),
            }],
        },
        DaemonMsg::RepoCloned {
            dest: "/data/repos/app-1a2b3c4d".into(),
        },
        DaemonMsg::RepoCloneFailed {
            dest: "/data/repos/app-1a2b3c4d".into(),
            failure: micold_core::protocol::messages::CloneFailureWire::Other("fatal: x".into()),
        },
        DaemonMsg::RepoCloneFailed {
            dest: "/data/repos/app-1a2b3c4d".into(),
            failure: micold_core::protocol::messages::CloneFailureWire::DiskFull,
        },
        DaemonMsg::GitCredentialTargets {
            targets: vec![
                micold_core::protocol::messages::CredentialTarget {
                    project: "/work/app".into(),
                    push_url: Some("git@github.com:acme/app.git".into()),
                },
                micold_core::protocol::messages::CredentialTarget {
                    project: "/work/scratch".into(),
                    push_url: None,
                },
            ],
            git_too_old: Some("2.30.0".into()),
        },
        DaemonMsg::StructuredInput {
            session: sid(),
            phase: micold_core::protocol::messages::TurnPhase::Busy {
                turn: micold_core::structured::TurnId(4),
                interrupt: micold_core::structured::InterruptAsk::Asked,
            },
            hold: false,
            queue: vec![micold_core::structured::QueuedSummary {
                id: micold_core::structured::QueueEntryId(1),
                text: "queued".into(),
                attachments: vec![],
            }],
        },
    ]
}

fn scoped_repo() -> micold_core::github::GithubRepo {
    micold_core::github::GithubRepo::from_remote_url("https://github.com/acme/app").unwrap()
}

/// #1002: the token survives both wires, and nothing that prints the message shows it.
#[test]
fn a_credential_grant_round_trips_and_prints_without_its_token() {
    let grant = ClientMsg::GitCredentialGrant {
        repo: scoped_repo(),
        token: micold_core::scoped_credential::SecretToken::new("ghs_wire_secret"),
        expires_at: 1_900_000_000,
    };
    json_roundtrip(&grant);
    postcard_roundtrip(&grant);
    let shown = format!("{grant:?}");
    assert!(
        shown.contains("redacted") && !shown.contains("ghs_wire_secret"),
        "{shown}"
    );
}

#[test]
fn the_protocol_version_is_63() {
    assert_eq!(micold_core::protocol::version::PROTOCOL_VERSION, 63);
}

/// #942 M4 (FR-013, FR-015): a summary and a create carry the session's permission mode on both
/// wires, and an older frame without it decodes as `Default`.
#[test]
fn the_permission_mode_rides_the_summary_and_the_create_and_defaults_when_absent() {
    for mode in PermissionMode::ALL {
        let summary = SessionSummary {
            permission_mode: mode,
            ..sample_summary()
        };
        json_roundtrip(&summary);
        postcard_roundtrip(&summary);
        let create = ClientMsg::SessionCreate {
            req: 3,
            project: "/p".into(),
            worktree_dir: "wt".into(),
            provider: AiCli::ClaudeCode,
            allow_additional_session: false,
            kind: micold_core::session::SessionKind::Structured,
            permission_mode: mode,
        };
        json_roundtrip(&create);
        postcard_roundtrip(&create);
    }

    let mut old = serde_json::to_value(sample_summary()).expect("a summary encodes");
    old.as_object_mut()
        .expect("an object")
        .remove("permission_mode")
        .expect("the summary carries its mode");
    let back: SessionSummary = serde_json::from_value(old).expect("an older summary decodes");
    assert_eq!(back.permission_mode, PermissionMode::Default);

    let create: ClientMsg = serde_json::from_str(
        r#"{"SessionCreate":{"req":1,"project":"/a","worktree_dir":"d","provider":"ClaudeCode","kind":"Structured"}}"#,
    )
    .expect("an older create decodes");
    let ClientMsg::SessionCreate {
        permission_mode, ..
    } = create
    else {
        panic!("not a SessionCreate: {create:?}");
    };
    assert_eq!(permission_mode, PermissionMode::Default);
}

/// A pending tool request, as every client is shown it (#942 contracts/wire.md).
fn permission_request(kind: RequestKind) -> PermissionRequest {
    PermissionRequest {
        id: PromptId(3),
        session: sid(),
        tool_use_id: Some("toolu_1".into()),
        tool_name: "Bash".into(),
        input_preview: r#"{"command":"npm test"}"#.into(),
        input_bytes: 22,
        kind,
        always_allow: Some("Bash: \"npm test\"".into()),
    }
}

/// #942 M2 (T026, contracts/wire.md): the permission messages of both directions round-trip with
/// every request kind, choice and resolution.
#[test]
fn the_permission_messages_round_trip() {
    let question = Question {
        header: "DB".into(),
        text: "Which database?".into(),
        options: vec![
            QuestionOption {
                label: "Postgres".into(),
                description: "".into(),
            },
            QuestionOption {
                label: "SQLite".into(),
                description: "file".into(),
            },
        ],
        multi_select: true,
    };
    let kinds = [
        RequestKind::Tool,
        RequestKind::PlanApproval {
            plan: "1. do it".into(),
        },
        RequestKind::Question {
            questions: vec![question],
        },
    ];
    let mut daemon: Vec<DaemonMsg> = kinds
        .into_iter()
        .map(|kind| DaemonMsg::PermissionRequested {
            request: permission_request(kind),
        })
        .collect();
    for resolution in [
        Resolution::Allowed,
        Resolution::AllowedAlways,
        Resolution::Denied,
        Resolution::PlanApproved {
            mode: PermissionMode::AcceptEdits,
        },
        Resolution::KeptPlanning,
        Resolution::Answered,
        Resolution::Declined,
        Resolution::Withdrawn {
            reason: WithdrawReason::AgentHungUp,
        },
    ] {
        daemon.push(DaemonMsg::PermissionResolved {
            session: sid(),
            id: PromptId(3),
            tool_use_id: Some("toolu_1".into()),
            resolution,
        });
    }
    daemon.push(DaemonMsg::PermissionResolved {
        session: sid(),
        id: PromptId(4),
        tool_use_id: None,
        resolution: Resolution::Denied,
    });
    daemon.push(DaemonMsg::PermissionInputText {
        req: 9,
        text: r#"{"file_path":"/a","content":"x"}"#.into(),
    });
    for msg in &daemon {
        json_roundtrip(msg);
    }

    let choices = [
        PromptChoice::Allow,
        PromptChoice::AlwaysAllow,
        PromptChoice::Deny { message: None },
        PromptChoice::Deny {
            message: Some("not now".into()),
        },
        PromptChoice::ApprovePlan { accept_edits: true },
        PromptChoice::KeepPlanning {
            note: Some("more".into()),
        },
        PromptChoice::Answer {
            answers: vec![QuestionAnswer {
                labels: vec!["SQLite".into()],
                other: Some("and a cache".into()),
            }],
        },
        PromptChoice::DeclineQuestion,
    ];
    for (req, choice) in choices.into_iter().enumerate() {
        json_roundtrip(&ClientMsg::PermissionDecision {
            req: req as u64,
            session: sid(),
            id: PromptId(3),
            choice,
        });
    }
    json_roundtrip(&ClientMsg::PermissionInput {
        req: 20,
        session: sid(),
        id: PromptId(3),
    });
    // #942 M5 (FR-017): every mode a running session can be changed to, on both wires.
    for (req, mode) in (21u64..).zip(PermissionMode::ALL) {
        let change = ClientMsg::SetPermissionMode {
            req,
            session: sid(),
            mode,
        };
        json_roundtrip(&change);
        postcard_roundtrip(&change);
    }
}

/// Two merged branches and the last commit of each one's pull request (feature 040).
fn merged_branch_queries() -> Vec<MergedBranchQuery> {
    vec![
        MergedBranchQuery {
            branch: "feat/a".into(),
            head: "1111111111111111111111111111111111111111".into(),
        },
        MergedBranchQuery {
            branch: "fix/b".into(),
            head: "2222222222222222222222222222222222222222".into(),
        },
    ]
}

/// U56 (feature 040, FR-015): the merged-branch question keeps its queries in order with each
/// branch beside its head, and the reply keeps one answer per query in that order — the order is
/// the only thing that ties an answer to its branch.
#[test]
fn a_merged_branch_check_and_its_answers_round_trip_in_order() {
    let asked = ClientMsg::MergedBranchCheck {
        req: 46,
        project: PathBuf::from("/a"),
        checks: merged_branch_queries(),
    };
    let bytes = serde_json::to_vec(&asked).unwrap();
    match serde_json::from_slice::<ClientMsg>(&bytes).unwrap() {
        ClientMsg::MergedBranchCheck {
            req,
            project,
            checks,
        } => {
            assert_eq!((req, project), (46, PathBuf::from("/a")));
            assert_eq!(checks, merged_branch_queries());
        }
        other => panic!("expected MergedBranchCheck, got {other:?}"),
    }

    let answers = vec![
        BranchContainment::Beyond,
        BranchContainment::Contained,
        BranchContainment::Unknown,
    ];
    let answered = DaemonMsg::OperationOk {
        req: 46,
        result: OperationResult::MergedBranchCheck {
            answers: answers.clone(),
        },
    };
    let bytes = serde_json::to_vec(&answered).unwrap();
    match serde_json::from_slice::<DaemonMsg>(&bytes).unwrap() {
        DaemonMsg::OperationOk {
            req: 46,
            result: OperationResult::MergedBranchCheck { answers: back },
        } => assert_eq!(back, answers),
        other => panic!("expected a MergedBranchCheck result, got {other:?}"),
    }
}

/// Feature 776, FR-006/FR-007: an older frame lacks every new field, and decodes with the old
/// behaviour.
#[test]
fn frames_without_the_feature_776_fields_decode_with_the_defaults() {
    let create: ClientMsg = serde_json::from_str(
        r#"{"WorktreeCreate":{"req":1,"project":"/a","branch":"b","dir_name":"d","mode":"NewBranch"}}"#,
    )
    .expect("decode WorktreeCreate without session");
    assert!(matches!(
        create,
        ClientMsg::WorktreeCreate { session: None, .. }
    ));

    let created: DaemonMsg = serde_json::from_str(
        r#"{"OperationOk":{"req":1,"result":{"WorktreeCreated":{"dir_name":"d"}}}}"#,
    )
    .expect("decode WorktreeCreated without session");
    assert!(matches!(
        created,
        DaemonMsg::OperationOk {
            result: OperationResult::WorktreeCreated { session: None, .. },
            ..
        }
    ));

    let item: AttachItem =
        serde_json::from_str(&format!(r#"{{"Session":{{"id":"{}"}}}}"#, Uuid::nil()))
            .expect("decode Session item without replace");
    assert_eq!(
        item,
        AttachItem::Session {
            id: Uuid::nil(),
            replace: false,
            allow_additional_session: false,
        }
    );

    // Feature 776, amendment 2026-10-13 (v42): the explicit request is absent from an older frame
    // and decodes to the refusal.
    for (frame, expect) in [
        (
            r#"{"SessionStart":{"session":"00000000-0000-0000-0000-000000000000"}}"#,
            "SessionStart",
        ),
        (
            r#"{"SessionRestart":{"session":"00000000-0000-0000-0000-000000000000"}}"#,
            "SessionRestart",
        ),
        (
            r#"{"SessionCreate":{"req":1,"project":"/a","worktree_dir":"d","provider":"ClaudeCode"}}"#,
            "SessionCreate",
        ),
    ] {
        match serde_json::from_str::<ClientMsg>(frame).unwrap_or_else(|e| panic!("{expect}: {e}")) {
            ClientMsg::SessionStart {
                allow_additional_session,
                ..
            }
            | ClientMsg::SessionRestart {
                allow_additional_session,
                ..
            }
            | ClientMsg::SessionCreate {
                allow_additional_session,
                ..
            } => assert!(!allow_additional_session, "{expect}"),
            other => panic!("{expect}: {other:?}"),
        }
    }

    let result: AttachResult = serde_json::from_str(&format!(
        r#"{{"item":{{"Session":{{"id":"{}"}}}},"outcome":"AlreadyAttached"}}"#,
        Uuid::nil()
    ))
    .expect("decode AttachResult without existing");
    assert_eq!(result.existing, None);
}

#[test]
fn a_blank_prompt_in_a_definition_is_none() {
    assert_eq!(
        SessionDefinition::new(AiCli::ClaudeCode, Some("  \n".into())).prompt,
        None
    );
    assert_eq!(
        SessionDefinition::new(AiCli::ClaudeCode, Some("go".into())).prompt,
        Some("go".into())
    );
}

#[test]
fn every_client_message_json_round_trips() {
    for msg in sample_client_msgs() {
        json_roundtrip(&msg);
    }
}

#[test]
fn every_daemon_message_json_round_trips() {
    for msg in sample_daemon_msgs() {
        json_roundtrip(&msg);
    }
}

/// Feature 613, wire W5.1: the grant carries the kind the service decided, as a snake-case string,
/// and every awaiting-input kind survives both wires.
#[test]
fn an_attention_grant_carries_its_kind_as_a_snake_case_string() {
    for (kind, encoded) in [
        (NotificationKind::NeedsPermission, "needs_permission"),
        (NotificationKind::LongTaskFinished, "long_task_finished"),
        (NotificationKind::TurnFinished, "turn_finished"),
    ] {
        let grant = DaemonMsg::AttentionGranted {
            session: sid(),
            seq: 7,
            kind,
        };
        let json = serde_json::to_value(&grant).expect("json encode");
        assert_eq!(
            json["AttentionGranted"]["kind"],
            serde_json::Value::String(encoded.into()),
            "the kind travels as its snake-case name (W5.1)"
        );
        json_roundtrip(&grant);
        postcard_roundtrip(&grant);
    }
}

/// Feature 613, wire W5.2: the error notice names the project and the session, carries no `req`
/// (it is not an operation and is never answered), and survives both wires.
#[test]
fn a_session_error_notice_round_trips_without_a_req() {
    let notice = DaemonMsg::SessionErrorNotice {
        project: PathBuf::from("/repo"),
        session: sid(),
    };
    let json = serde_json::to_value(&notice).expect("json encode");
    let fields = json["SessionErrorNotice"]
        .as_object()
        .expect("a struct variant");
    let mut names: Vec<&str> = fields.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(names, ["project", "session"], "no `req` (W5.2)");
    json_roundtrip(&notice);
    postcard_roundtrip(&notice);
}

/// U79 (feature 034, FR-016): every value of the cross-session option survives both directions,
/// and "leave it unchanged" stays distinct from each of them.
#[test]
fn the_cross_session_option_round_trips_in_daemon_settings_and_settings_set() {
    for access in CrossSessionAccess::ALL {
        let settings = DaemonSettings {
            jira_enabled: false,
            jira_connections: Default::default(),
            github_issues_enabled: true,
            jira_base_urls: Default::default(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: 80,
            scrollback_lines: 10_000,
            env_include_enabled: true,
            env_include_script_path: String::new(),
            env_include_timeout_secs: 10,
            default_ai_cli: AiCli::ClaudeCode,
            pi_activity_component: true,
            save_terminal_history: true,
            tool_server_enabled: true,
            cross_session_access: access,
            pr_status_enabled: false,
            desktop_notifications: true,
            notification_kinds: NotificationKinds {
                turn_finished: true,
                ..NotificationKinds::default()
            },
            long_task_threshold_secs: 120,
            diff_layout: Default::default(),
        };
        json_roundtrip(&DaemonMsg::SettingsChanged {
            settings: settings.clone(),
        });
        let set = ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 3,
            scrollback_lines: None,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: None,
            tool_server_enabled: None,
            cross_session_access: Some(access),
            pr_status_enabled: None,
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: None,
            jira_base_url: None,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        };
        json_roundtrip(&set);
        let bytes = serde_json::to_vec(&set).unwrap();
        match serde_json::from_slice::<ClientMsg>(&bytes).unwrap() {
            ClientMsg::SettingsSet {
                jira_enabled: None,
                jira_connection: None,
                github_issues_enabled: None,
                jira_base_url: None,
                cross_session_access,
                ..
            } => assert_eq!(cross_session_access, Some(access)),
            other => panic!("expected SettingsSet, got {other:?}"),
        }
    }
    assert_eq!(
        CrossSessionAccess::ALL,
        [
            CrossSessionAccess::Auto,
            CrossSessionAccess::ConfirmEachSend,
            CrossSessionAccess::Off
        ],
        "the three values FR-016 names, in the order Settings offers them"
    );
}

/// U57 (feature 040, FR-029, FR-030): the pull request switch survives both directions — on and
/// off in what the daemon reports, and on, off and "leave it as it is" in what a client sets,
/// each distinct from the other two.
#[test]
fn the_pull_request_switch_round_trips_in_daemon_settings_and_settings_set() {
    for on in [true, false] {
        let settings = DaemonSettings {
            jira_enabled: false,
            jira_connections: Default::default(),
            github_issues_enabled: true,
            jira_base_urls: Default::default(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: 80,
            scrollback_lines: 10_000,
            env_include_enabled: true,
            env_include_script_path: String::new(),
            env_include_timeout_secs: 10,
            default_ai_cli: AiCli::ClaudeCode,
            pi_activity_component: true,
            save_terminal_history: true,
            tool_server_enabled: true,
            cross_session_access: CrossSessionAccess::Auto,
            pr_status_enabled: on,
            desktop_notifications: true,
            notification_kinds: NotificationKinds {
                turn_finished: true,
                ..NotificationKinds::default()
            },
            long_task_threshold_secs: 120,
            diff_layout: Default::default(),
        };
        let bytes = serde_json::to_vec(&DaemonMsg::SettingsChanged { settings }).unwrap();
        match serde_json::from_slice::<DaemonMsg>(&bytes).unwrap() {
            DaemonMsg::SettingsChanged { settings } => assert_eq!(settings.pr_status_enabled, on),
            other => panic!("expected SettingsChanged, got {other:?}"),
        }
    }
    for chosen in [Some(true), Some(false), None] {
        let set = ClientMsg::SettingsSet {
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: None,
            jira_base_url: None,
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 3,
            scrollback_lines: None,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: chosen,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        };
        let bytes = serde_json::to_vec(&set).unwrap();
        match serde_json::from_slice::<ClientMsg>(&bytes).unwrap() {
            ClientMsg::SettingsSet {
                jira_enabled: None,
                jira_connection: None,
                github_issues_enabled: None,
                jira_base_url: None,
                pr_status_enabled,
                ..
            } => assert_eq!(pr_status_enabled, chosen),
            other => panic!("expected SettingsSet, got {other:?}"),
        }
    }
}

#[test]
fn grid_types_postcard_round_trip() {
    postcard_roundtrip(&LineId(-42));
    postcard_roundtrip(&WireColor::Rgb(1, 2, 3));
    postcard_roundtrip(&WireCursorShape::HollowBlock);
    postcard_roundtrip(&wide_char_line());
    postcard_roundtrip(&sample_grid_frame());
}

#[test]
fn grid_frame_survives_postcard_encode_decode_byte_identical() {
    let frame = sample_grid_frame();
    let once = postcard::to_stdvec(&frame).expect("encode");
    let decoded: GridFrame = postcard::from_bytes(&once).expect("decode");
    let twice = postcard::to_stdvec(&decoded).expect("re-encode");
    assert_eq!(
        once, twice,
        "grid frame not byte-identical across a round-trip"
    );
    assert_eq!(
        decoded, frame,
        "grid frame value changed across a round-trip"
    );

    // The wide-char spacer sentinel and the zerowidth marks specifically survive.
    let line = &decoded.lines[0];
    assert_eq!(
        line.text.chars().count(),
        3,
        "spacer cell must be preserved"
    );
    assert_eq!(
        line.text.chars().nth(1),
        Some('\u{0}'),
        "spacer sentinel lost"
    );
    assert_eq!(line.extras[0].zerowidth, vec!['\u{200d}', '\u{2764}']);
}

#[test]
fn grid_frame_also_json_round_trips_for_the_debug_wire() {
    // `MICOLD_WIRE=json` serializes the same GridFrame type as JSON; it must round-trip too.
    json_roundtrip(&sample_grid_frame());
}

#[test]
fn envelope_header_round_trips_and_rejects_garbage() {
    for (enc, kind) in [
        (Encoding::Json, Kind::Control),
        (Encoding::Postcard, Kind::Grid),
        (Encoding::PostcardLz4, Kind::Grid),
    ] {
        let header = EnvelopeHeader::new(enc, kind);
        let mut frame = header.to_bytes().to_vec();
        frame.extend_from_slice(b"payload");
        let (parsed, payload) = EnvelopeHeader::parse(&frame).expect("parse");
        assert_eq!(parsed, header);
        assert_eq!(payload, b"payload");
    }

    // A non-zero reserved field is rejected loudly.
    let mut bad = EnvelopeHeader::new(Encoding::Json, Kind::Control)
        .to_bytes()
        .to_vec();
    bad[2] = 1; // reserved low byte
    assert!(matches!(
        EnvelopeHeader::parse(&bad),
        Err(EnvelopeError::NonZeroReserved(1))
    ));

    // Unknown tags and short headers are specific errors, never silent defaults.
    assert!(matches!(
        EnvelopeHeader::parse(&[9, 0, 0, 0]),
        Err(EnvelopeError::UnknownEncoding(9))
    ));
    assert!(matches!(
        EnvelopeHeader::parse(&[0, 9, 0, 0]),
        Err(EnvelopeError::UnknownKind(9))
    ));
    assert!(matches!(
        EnvelopeHeader::parse(&[0, 0]),
        Err(EnvelopeError::ShortHeader(2))
    ));
    assert_eq!(HEADER_LEN, 4);
}

#[test]
fn an_availability_answer_round_trips_with_and_without_the_environment_state() {
    // Feature 037, contract A1: `env` says why a CLI is missing, and `None` says the service could
    // not tell. The two must still differ after the wire, or "unknown" would read as a reason.
    let answer = |env| DaemonMsg::AiCliAvailability {
        req: 7,
        available: vec![AiCli::ClaudeCode],
        env,
    };
    let known = answer(Some(SpawnEnv::ScriptTimedOut));
    let unknown = answer(None);
    json_roundtrip(&known);
    json_roundtrip(&unknown);

    let decode = |msg: &DaemonMsg| -> DaemonMsg {
        serde_json::from_slice(&serde_json::to_vec(msg).unwrap()).unwrap()
    };
    assert_ne!(
        decode(&known),
        decode(&unknown),
        "a known state and no state are different answers on the wire"
    );
}

#[test]
fn a_session_summary_carrying_a_derived_label_round_trips_on_both_wires() {
    // Feature 032: a label derived from the first turn rides `SessionSummary.title` as itself, never
    // as a title, so a client can tell the two apart after a restart (C8.4).
    let summary = SessionSummary {
        title: SessionLabel::Derived("/speckit-autopilot".into()),
        ..sample_summary()
    };
    json_roundtrip(&summary);
    postcard_roundtrip(&summary);
}

#[test]
fn a_send_input_confirmation_has_no_field_that_can_carry_the_input_text() {
    // Feature 034 (FR-018): the prompt names the operation and its target only. `SendInput` is a
    // unit variant, so there is nowhere on the wire for the text an agent wants to type.
    assert_eq!(
        serde_json::to_value(ConfirmOperation::SendInput).expect("encode"),
        serde_json::json!("SendInput")
    );
    let with_text = r#"{"SendInput":{"text":"rm -rf /"}}"#;
    assert!(
        serde_json::from_str::<ConfirmOperation>(with_text).is_err(),
        "a SendInput carrying text must not decode"
    );
}

/// 011 FR-022, BUG-454: the service's per-directory environment-include failures ride in the catalog
/// snapshot, each with its category and the script's captured output, so the Settings page can say
/// which session directories got no environment and why.
#[test]
fn per_directory_env_include_failures_round_trip_in_the_catalog_snapshot() {
    use micold_core::env_include::EnvIncludeOutcome;
    use micold_core::protocol::messages::EnvIncludeFailure;

    let mut catalog = sample_catalog();
    catalog.env_include_failures = vec![
        EnvIncludeFailure {
            dir: PathBuf::from("/a/.claude/worktrees/feat-x"),
            outcome: EnvIncludeOutcome::NonZeroExit {
                code: 3,
                diagnostic: "mise: config not trusted".into(),
            },
        },
        EnvIncludeFailure {
            dir: PathBuf::from("/b"),
            outcome: EnvIncludeOutcome::TimedOut {
                diagnostic: "still waiting".into(),
            },
        },
        EnvIncludeFailure {
            dir: PathBuf::from("/c"),
            outcome: EnvIncludeOutcome::MissingScript,
        },
    ];
    json_roundtrip(&DaemonMsg::CatalogChanged { catalog });
}

/// The list is optional in the encoding (`serde(default)`): a snapshot with no failures carries no
/// field, and one encoded without the field reads back with an empty list. That keeps every
/// catalog frame of a service with nothing to report as it was; it does not make an older peer
/// compatible, which the handshake's version and schema check refuses anyway.
#[test]
fn a_catalog_snapshot_without_env_include_failures_reads_back_with_none() {
    let catalog = sample_catalog();
    let encoded = serde_json::to_value(&catalog).expect("json encode");
    assert!(
        encoded.get("env_include_failures").is_none(),
        "an empty list is not encoded: {encoded}"
    );
    let back: CatalogSnapshot = serde_json::from_value(encoded).expect("json decode");
    assert!(back.env_include_failures.is_empty());
    assert_eq!(back, catalog);
}

/// Feature 482 (T007, contracts/review-wire.md): every review message and the diff layout setting
/// survive both wires.
#[test]
fn the_review_messages_and_the_diff_layout_round_trip_on_both_wires() {
    use micold_core::protocol::messages::ReviewEditOp;
    use micold_core::review::comment::{
        CommentId, CommentState, Move, Outcome, PrAnchor, ReviewComment, Scope,
    };
    use micold_core::review::{LineRange, RelPath, Side};
    use micold_core::settings::DiffLayout;

    let id = CommentId(Uuid::from_u128(0x0123_4567_89ab_4def_8123_4567_89ab_cdef));
    let edits = [
        ReviewEditOp::Add {
            path: "src/a b é.rs".into(),
            side: Side::Old,
            start: 3,
            end: 4,
            quote: vec!["x".into(), "y".into()],
            text: "why?".into(),
            pr: None,
        },
        // Feature 1149 (W18, W19).
        ReviewEditOp::Add {
            path: "src/a.rs".into(),
            side: Side::New,
            start: 1,
            end: 1,
            quote: vec!["x".into()],
            text: "on a pull request".into(),
            pr: Some(PrAnchor {
                number: 7,
                head_branch: "feat/x".into(),
                head_commit: "a".repeat(40),
            }),
        },
        ReviewEditOp::ClearSent {
            scope: Scope::PullRequest(7),
        },
        ReviewEditOp::DiscardPending {
            scope: Scope::WorkingTree,
        },
        ReviewEditOp::SetText {
            id,
            text: "changed".into(),
        },
        ReviewEditOp::Delete { id },
        ReviewEditOp::ClearSent {
            scope: Scope::WorkingTree,
        },
        ReviewEditOp::DiscardPending {
            scope: Scope::PullRequest(8),
        },
        // Feature 946 (W14, W15).
        ReviewEditOp::Mark {
            id,
            outcome: Outcome::Answered,
            note: "kept: see R4".into(),
        },
        ReviewEditOp::Mark {
            id,
            outcome: Outcome::Applied,
            note: String::new(),
        },
        ReviewEditOp::Reanchor {
            moves: vec![Move::new(
                id,
                LineRange::new(3, 4).unwrap(),
                LineRange::new(7, 8).unwrap(),
            )
            .unwrap()],
        },
        ReviewEditOp::Reanchor { moves: Vec::new() },
    ];
    for edit in edits {
        let msg = ClientMsg::ReviewEdit {
            req: 7,
            project: PathBuf::from("/p"),
            worktree_dir: "feature".into(),
            edit,
        };
        json_roundtrip(&msg);
        postcard_roundtrip(&msg);
    }
    let send = ClientMsg::ReviewSend {
        req: 8,
        project: PathBuf::from("/p"),
        worktree_dir: String::new(),
        outdated: vec![id],
        pr: None,
    };
    json_roundtrip(&send);
    postcard_roundtrip(&send);
    // Feature 1149 (W20): a pull request's send, every state of it.
    for state in [
        BranchPrState::Open,
        BranchPrState::Draft,
        BranchPrState::Merged,
        BranchPrState::Closed,
    ] {
        let send = ClientMsg::ReviewSend {
            req: 9,
            project: PathBuf::from("/p"),
            worktree_dir: "feature".into(),
            outdated: Vec::new(),
            pr: Some(PrSend {
                number: 7,
                title: "Fix é".into(),
                head_branch: "feat/x".into(),
                state,
                current_head: "c".repeat(40),
            }),
        };
        json_roundtrip(&send);
        postcard_roundtrip(&send);
    }

    // Feature 946 (W13): a comment in each state, and a report by each kind of reporter. A
    // `Report` has no public constructor, so the reported states come from their stored form.
    let reported = |by: serde_json::Value| -> CommentState {
        serde_json::from_value(serde_json::json!({ "reported": {
            "sent_at": 9, "outcome": "answered", "reply": "line 1\nline 2",
            "first_at": 10, "last_at": 11, "by": by
        } }))
        .expect("a reported state parses")
    };
    let states = [
        CommentState::Draft,
        CommentState::Sent { at: 9 },
        reported(serde_json::json!({ "agent": "00000000-0000-0000-0000-0000000000aa" })),
        reported(serde_json::json!("user")),
    ];
    for state in states {
        let comment = ReviewComment {
            id,
            path: RelPath::from_native("src/a.rs").unwrap(),
            side: Side::New,
            range: LineRange::new(1, 2).unwrap(),
            quote: vec!["a".into(), "b".into()],
            text: "t".into(),
            state,
            created: 1,
            pr: Some(PrAnchor {
                number: 7,
                head_branch: "feat/x".into(),
                head_commit: "a".repeat(40),
            }),
        };
        let changed = DaemonMsg::ReviewChanged {
            project: PathBuf::from("/p"),
            worktree_dir: "feature".into(),
            comments: vec![comment],
            sending: true,
        };
        json_roundtrip(&changed);
        postcard_roundtrip(&changed);
    }
    for started in [true, false] {
        let sent = OperationResult::ReviewSent {
            session: sid(),
            started,
        };
        json_roundtrip(&sent);
        postcard_roundtrip(&sent);
    }

    for layout in [DiffLayout::Unified, DiffLayout::SideBySide] {
        let settings = DaemonSettings {
            jira_enabled: false,
            jira_connections: Default::default(),
            github_issues_enabled: true,
            jira_base_urls: Default::default(),
            plan_usage_enabled: true,
            plan_usage_warn_percent: 80,
            scrollback_lines: 10_000,
            env_include_enabled: true,
            env_include_script_path: String::new(),
            env_include_timeout_secs: 10,
            default_ai_cli: AiCli::ClaudeCode,
            pi_activity_component: true,
            save_terminal_history: true,
            tool_server_enabled: true,
            cross_session_access: CrossSessionAccess::Auto,
            pr_status_enabled: false,
            desktop_notifications: true,
            notification_kinds: NotificationKinds::default(),
            long_task_threshold_secs: 60,
            diff_layout: layout,
        };
        let pushed = DaemonMsg::SettingsChanged { settings };
        json_roundtrip(&pushed);
        postcard_roundtrip(&pushed);
    }
    for chosen in [
        Some(DiffLayout::SideBySide),
        Some(DiffLayout::Unified),
        None,
    ] {
        let set = ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 3,
            scrollback_lines: None,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: None,
            jira_base_url: None,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: chosen,
        };
        json_roundtrip(&set);
        postcard_roundtrip(&set);
    }
}

/// Feature 483 (contracts/run-group-wire.md): a run group with a run in every status, as the
/// daemon pushes it.
fn sample_run_group() -> micold_core::runs::RunGroup {
    use micold_core::naming::{ConventionalType, DerivedNames, WorktreeNaming};
    use micold_core::runs::{GroupId, Run, RunGroup, RunStatus, RunStep};
    let statuses = [
        RunStatus::Creating,
        RunStatus::Starting,
        RunStatus::Prompted,
        RunStatus::PromptNotDelivered {
            reason: "the CLI never became ready".into(),
        },
        RunStatus::Failed {
            step: RunStep::Worktree,
            reason: "the branch exists".into(),
        },
        RunStatus::Failed {
            step: RunStep::Session,
            reason: "copilot is not installed".into(),
        },
        RunStatus::Picked,
    ];
    RunGroup {
        id: GroupId(Uuid::from_u128(0x483)),
        name: "login page".into(),
        naming: WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: Some("ABC-1".into()),
            name: "login page".into(),
        },
        prompt: "Add a login page".into(),
        base_branch: "main".into(),
        base_commit: "0123456789abcdef0123456789abcdef01234567".into(),
        created: std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000),
        runs: statuses
            .into_iter()
            .enumerate()
            .map(|(i, status)| Run {
                number: i as u8 + 1,
                provider: if i % 2 == 0 {
                    AiCli::ClaudeCode
                } else {
                    AiCli::Copilot
                },
                names: DerivedNames {
                    dir_name: format!("feat-abc-1_login-page-{}", i + 1),
                    branch: format!("feat/abc-1_login-page-{}", i + 1),
                },
                session: (i % 2 == 0).then(sid),
                status,
            })
            .collect(),
        winner: Some(7),
    }
}

/// Feature 483 (T008): the run-group messages and results survive both wires.
#[test]
fn the_run_group_messages_round_trip_on_both_wires() {
    use micold_core::naming::{ConventionalType, WorktreeNaming};
    use micold_core::runs::{GroupId, Integration};
    let group = sample_run_group();
    let client = vec![
        ClientMsg::RunGroupCreate {
            req: 60,
            project: PathBuf::from("/a"),
            naming: WorktreeNaming {
                type_: Some(ConventionalType::Feat),
                ticket: None,
                name: "login page".into(),
            },
            prompt: "Add a login page".into(),
            base_branch: "main".into(),
            providers: vec![AiCli::ClaudeCode, AiCli::ClaudeCode, AiCli::Copilot],
        },
        ClientMsg::RunGroupPick {
            req: 61,
            project: PathBuf::from("/a"),
            group: group.id,
            run: 2,
        },
        ClientMsg::RunGroupDismiss {
            req: 62,
            project: PathBuf::from("/a"),
            group: GroupId(Uuid::from_u128(1)),
        },
    ];
    for msg in &client {
        json_roundtrip(msg);
        postcard_roundtrip(msg);
    }
    let daemon = vec![
        DaemonMsg::RunGroupsChanged {
            project: PathBuf::from("/a"),
            groups: vec![group.clone()],
        },
        DaemonMsg::RunGroupsChanged {
            project: PathBuf::from("/a"),
            groups: vec![],
        },
        DaemonMsg::OperationOk {
            req: 60,
            result: OperationResult::RunGroupCreated { group: group.id },
        },
        DaemonMsg::OperationOk {
            req: 61,
            result: OperationResult::RunPicked {
                group: group.id,
                run: 2,
                integration: Integration::FastForward {
                    base_tip: "aaaa".into(),
                },
            },
        },
        DaemonMsg::OperationOk {
            req: 61,
            result: OperationResult::RunPicked {
                group: group.id,
                run: 3,
                integration: Integration::MergeCommit {
                    commit: "bbbb".into(),
                },
            },
        },
    ];
    for msg in &daemon {
        json_roundtrip(msg);
        postcard_roundtrip(msg);
    }
}

/// U89, U90, U91 (feature 041, contracts/setting.md §2): the setting reaches the window as the
/// service holds it, `SettingsSet` carries the choice, and `None` leaves it.
#[test]
fn the_save_terminal_history_setting_round_trips_on_the_wire() {
    for chosen in [Some(false), Some(true), None] {
        let asked = ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 1,
            scrollback_lines: None,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: chosen,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: None,
            jira_base_url: None,
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        };
        let json = serde_json::to_string(&asked).unwrap();
        match serde_json::from_str::<ClientMsg>(&json).unwrap() {
            ClientMsg::SettingsSet {
                jira_enabled: None,
                jira_connection: None,
                github_issues_enabled: None,
                jira_base_url: None,
                save_terminal_history,
                ..
            } => assert_eq!(save_terminal_history, chosen),
            other => panic!("expected SettingsSet, got {other:?}"),
        }
    }
}

#[test]
fn a_grid_frame_carries_its_process_under_postcard() {
    let mut frame = sample_grid_frame();
    assert_eq!(frame.process, SessionProcess::Primary);
    frame.process = SessionProcess::Shell(ShellInstanceId(3));
    postcard_roundtrip(&frame);
}

fn plan_usage_settings_set(
    plan_usage_enabled: Option<bool>,
    plan_usage_warn_percent: Option<u8>,
) -> ClientMsg {
    ClientMsg::SettingsSet {
        req: 1,
        scrollback_lines: None,
        env_include_enabled: None,
        env_include_script_path: None,
        env_include_timeout_secs: None,
        default_ai_cli: None,
        pi_activity_component: None,
        save_terminal_history: None,
        tool_server_enabled: None,
        cross_session_access: None,
        pr_status_enabled: None,
        jira_enabled: None,
        jira_connection: None,
        github_issues_enabled: None,
        jira_base_url: None,
        desktop_notifications: None,
        notification_kinds: None,
        long_task_threshold_secs: None,
        diff_layout: None,
        plan_usage_enabled,
        plan_usage_warn_percent,
    }
}

fn sample_usage_reading() -> micold_core::plan_usage::UsageReading {
    use micold_core::plan_usage::{LimitWindow, UsageReading};
    UsageReading {
        windows: vec![
            LimitWindow {
                key: "five_hour".into(),
                used_tenths: 420,
                resets_at: Some(1_791_552_600),
            },
            LimitWindow {
                key: "seven_day".into(),
                used_tenths: 180,
                resets_at: Some(1_791_849_600),
            },
        ],
        obtained_at: 1_791_547_320,
    }
}

/// Feature 490 (contracts/plan-usage-wire.md W1, W2, W4): the stored reading rides `Welcome`,
/// and `PlanUsageChanged` carries a reading or `None`.
#[test]
fn the_plan_usage_messages_round_trip_on_both_wires() {
    let DaemonMsg::Welcome {
        daemon_build,
        catalog,
        settings,
        ..
    } = sample_daemon_msgs().remove(0)
    else {
        panic!("the first sample is Welcome");
    };
    for plan_usage in [Some(sample_usage_reading()), None] {
        let welcome = DaemonMsg::Welcome {
            daemon_build: daemon_build.clone(),
            catalog: catalog.clone(),
            settings: settings.clone(),
            plan_usage,
        };
        // The catalog's defaulted fields are JSON-only, as every control message is.
        json_roundtrip(&welcome);
    }
    for reading in [Some(sample_usage_reading()), None] {
        let changed = DaemonMsg::PlanUsageChanged { reading };
        json_roundtrip(&changed);
        postcard_roundtrip(&changed);
    }
}

/// W4: the contract's example decodes verbatim.
#[test]
fn the_plan_usage_contract_example_decodes_as_written() {
    let example = r#"{"PlanUsageChanged":{"reading":{"windows":[
  {"key":"five_hour","used_tenths":420,"resets_at":1791552600},
  {"key":"seven_day","used_tenths":180,"resets_at":1791849600}],
  "obtained_at":1791547320}}}"#;
    assert_eq!(
        serde_json::from_str::<DaemonMsg>(example).unwrap(),
        DaemonMsg::PlanUsageChanged {
            reading: Some(sample_usage_reading())
        }
    );
}

/// W3: the service settings carry both fields, and `SettingsSet` carries a choice or `None`.
#[test]
fn the_plan_usage_settings_round_trip_in_daemon_settings_and_settings_set() {
    let DaemonMsg::Welcome { settings, .. } = sample_daemon_msgs().remove(0) else {
        panic!("the first sample is Welcome");
    };
    for (enabled, percent) in [(true, 80), (false, 50)] {
        let pushed = DaemonMsg::SettingsChanged {
            settings: DaemonSettings {
                jira_enabled: false,
                jira_connections: Default::default(),
                github_issues_enabled: true,
                jira_base_urls: Default::default(),
                plan_usage_enabled: enabled,
                plan_usage_warn_percent: percent,
                ..settings.clone()
            },
        };
        json_roundtrip(&pushed);
        postcard_roundtrip(&pushed);
    }
    for (enabled, percent) in [(Some(false), Some(65)), (Some(true), None), (None, None)] {
        let asked = plan_usage_settings_set(enabled, percent);
        json_roundtrip(&asked);
        match serde_json::from_str::<ClientMsg>(&serde_json::to_string(&asked).unwrap()).unwrap() {
            ClientMsg::SettingsSet {
                jira_enabled: None,
                jira_connection: None,
                github_issues_enabled: None,
                jira_base_url: None,
                plan_usage_enabled,
                plan_usage_warn_percent,
                ..
            } => {
                assert_eq!(plan_usage_enabled, enabled);
                assert_eq!(plan_usage_warn_percent, percent);
            }
            other => panic!("expected SettingsSet, got {other:?}"),
        }
    }
}

#[test]
fn a_session_summary_without_an_origin_decodes_to_none() {
    // Feature 551, FR-013: a payload written before protocol 47 has no `origin` key.
    let mut value = serde_json::to_value(sample_summary()).expect("encode");
    value.as_object_mut().expect("object").remove("origin");
    let decoded: SessionSummary = serde_json::from_value(value).expect("decodes without the field");
    assert_eq!(decoded.origin, None);
}

#[test]
fn an_origin_survives_a_session_changed_and_a_snapshot() {
    let summary = sample_summary();
    json_roundtrip(&DaemonMsg::SessionChanged {
        session: summary.id,
        summary: summary.clone(),
    });
    let mut catalog = sample_catalog();
    catalog.projects[0].sessions = vec![summary];
    json_roundtrip(&catalog);
}

/// Feature 552: the agent runs travel on the summary, and a frame from before them decodes empty.
#[test]
fn a_session_summary_carries_its_agent_runs_and_defaults_to_none() {
    let summary = sample_summary();
    let json = serde_json::to_string(&summary).unwrap();
    let back: SessionSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back.agents, summary.agents);
    assert_eq!(back.agents.len(), 1);

    let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
    value.as_object_mut().unwrap().remove("agents");
    let older: SessionSummary = serde_json::from_value(value).unwrap();
    assert!(older.agents.is_empty());
}

/// Feature 776, amendment 2026-10-14 (v44): the main marker rides the catalog snapshot, an older
/// record without it decodes `false`, and `SessionSetMain` round-trips.
#[test]
fn the_main_marker_and_set_main_round_trip() {
    use micold_core::protocol::messages::{ActivitySignal, SessionSummary, WireLifecycle};
    use micold_core::session::{AiCli, SessionLabel};

    let summary = SessionSummary {
        id: SessionId::new(),
        worktree_dir: Some("a".into()),
        title: SessionLabel::Pending,
        lifecycle: WireLifecycle::Idle,
        activity: ActivitySignal::Unknown,
        provider: AiCli::ClaudeCode,
        kind: Default::default(),
        permission_mode: Default::default(),
        input_serial: 0,
        live_shells: Vec::new(),
        attention_seq: 0,
        unread: false,
        agents: Vec::new(),
        origin: None,
        main: true,
        paused_on_user: false,
        hook_activity: None,
    };
    let json = serde_json::to_value(&summary).expect("encode");
    assert_eq!(json["main"], true);
    let back: SessionSummary = serde_json::from_value(json.clone()).expect("decode");
    assert!(back.main);
    let mut older = json;
    older.as_object_mut().expect("object").remove("main");
    let older: SessionSummary = serde_json::from_value(older).expect("decode without main");
    assert!(!older.main);

    postcard_roundtrip(&ClientMsg::SessionSetMain {
        id: SessionId::new(),
    });
}

/// Feature 905 (W1): `paused_on_user` rides the summary, and a frame without it decodes `false`.
#[test]
fn paused_on_user_round_trips_and_defaults_to_false() {
    let summary = SessionSummary {
        paused_on_user: true,
        hook_activity: None,
        ..sample_summary()
    };
    json_roundtrip(&summary);
    postcard_roundtrip(&summary);
    let mut json = serde_json::to_value(&summary).expect("encode");
    assert_eq!(json["paused_on_user"], true);
    json.as_object_mut()
        .expect("object")
        .remove("paused_on_user");
    let older: SessionSummary = serde_json::from_value(json).expect("decode without the field");
    assert!(!older.paused_on_user);
}

/// Feature 533 (T006): the Jira switch and one project's connection change cross the wire, both
/// ways, and the echo carries the whole map.
#[test]
fn the_jira_switch_and_a_connection_change_round_trip() {
    use micold_core::jira::JiraConnection;
    use micold_core::settings::{JiraChange, JiraConnectionChange};

    let connection =
        JiraConnection::parse("https://acme.atlassian.net", "ABC", "me@acme.com").unwrap();
    for change in [
        JiraConnectionChange {
            project: "/work/app".into(),
            change: JiraChange::Upsert(connection.clone()),
        },
        JiraConnectionChange {
            project: "/work/app".into(),
            change: JiraChange::Remove,
        },
    ] {
        let mut asked = sample_client_msgs()
            .into_iter()
            .find(|m| matches!(m, ClientMsg::SettingsSet { .. }))
            .expect("a SettingsSet sample");
        if let ClientMsg::SettingsSet {
            jira_enabled,
            jira_connection,
            ..
        } = &mut asked
        {
            *jira_enabled = Some(true);
            *jira_connection = Some(change.clone());
        }
        json_roundtrip(&asked);
    }

    let mut settings = sample_daemon_msgs()
        .into_iter()
        .find_map(|m| match m {
            DaemonMsg::SettingsChanged { settings } => Some(settings),
            _ => None,
        })
        .expect("a SettingsChanged sample");
    settings.jira_enabled = true;
    settings
        .jira_connections
        .insert("/work/app".into(), connection.clone());
    json_roundtrip(&DaemonMsg::SettingsChanged { settings });
}

/// Feature 813 (W1): `hook_activity` rides the summary, and a frame without it decodes `None`.
#[test]
fn hook_activity_round_trips_and_defaults_to_none() {
    use micold_core::hook_activity::{HookActivity, Todo, TodoStatus};
    let summary = SessionSummary {
        hook_activity: Some(HookActivity {
            todos: vec![Todo {
                text: "write".into(),
                active: Some("writing".into()),
                status: TodoStatus::InProgress,
            }],
            prompt: Some("fix the bug".into()),
        }),
        ..sample_summary()
    };
    json_roundtrip(&summary);
    postcard_roundtrip(&summary);
    let mut json = serde_json::to_value(&summary).expect("encode");
    assert_eq!(json["hook_activity"]["prompt"], "fix the bug");
    json.as_object_mut()
        .expect("object")
        .remove("hook_activity");
    let older: SessionSummary = serde_json::from_value(json).expect("decode without the field");
    assert_eq!(older.hook_activity, None);
}

/// Feature 686: the pod clone messages arrived with protocol 51 (`main` was at 50).
#[test]
fn the_repo_clone_messages_are_protocol_51_or_later() {
    assert!(std::hint::black_box(micold_core::protocol::version::PROTOCOL_VERSION) >= 51);
}

/// Feature 1102 (W1, W2): the GitHub issues switch crosses the wire both ways, off and unchanged.
#[test]
fn the_github_issues_switch_round_trips() {
    for chosen in [None, Some(true), Some(false)] {
        let mut asked = sample_client_msgs()
            .into_iter()
            .find(|m| matches!(m, ClientMsg::SettingsSet { .. }))
            .expect("a SettingsSet sample");
        if let ClientMsg::SettingsSet {
            github_issues_enabled,
            ..
        } = &mut asked
        {
            *github_issues_enabled = chosen;
        }
        json_roundtrip(&asked);
    }
    let mut settings = sample_daemon_msgs()
        .into_iter()
        .find_map(|m| match m {
            DaemonMsg::SettingsChanged { settings } => Some(settings),
            _ => None,
        })
        .expect("a SettingsChanged sample");
    settings.github_issues_enabled = false;
    json_roundtrip(&DaemonMsg::SettingsChanged { settings });
}

// #942 T013 — the permission-prompt wire types (contracts/wire.md). No message carries them yet
// (M2 adds `PermissionRequested`), so each type round-trips on its own, under JSON (the control
// plane) and postcard.

fn permission_roundtrip<T>(value: &T)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    json_roundtrip(value);
    postcard_roundtrip(value);
}

#[test]
fn the_permission_prompt_wire_types_round_trip() {
    use micold_core::protocol::messages::{
        PermissionRequest, PromptChoice, PromptId, QuestionAnswer, Resolution, WithdrawReason,
    };
    use micold_core::session::PermissionMode;
    use micold_core::structured::permission::{Question, QuestionOption, RequestKind};

    permission_roundtrip(&PromptId(u64::MAX));
    let question = Question {
        header: "Color".into(),
        text: "Which color?".into(),
        options: vec![
            QuestionOption {
                label: "red".into(),
                description: String::new(),
            },
            QuestionOption {
                label: "blue".into(),
                description: "the sky".into(),
            },
        ],
        multi_select: true,
    };
    for (kind, always_allow) in [
        (RequestKind::Tool, Some("Bash: \"npm test\"".to_string())),
        (
            RequestKind::PlanApproval {
                plan: "1. é\n2. b".into(),
            },
            None,
        ),
        (
            RequestKind::Question {
                questions: vec![question],
            },
            None,
        ),
    ] {
        permission_roundtrip(&PermissionRequest {
            id: PromptId(7),
            session: sid(),
            tool_use_id: Some("toolu_1".into()),
            tool_name: "Bash".into(),
            input_preview: r#"{"command":"npm test"}"#.into(),
            input_bytes: 22,
            kind,
            always_allow,
        });
    }
    permission_roundtrip(&PermissionRequest {
        id: PromptId(0),
        session: sid(),
        tool_use_id: None,
        tool_name: String::new(),
        input_preview: String::new(),
        input_bytes: 0,
        kind: RequestKind::Tool,
        always_allow: None,
    });

    let answers = vec![
        QuestionAnswer {
            labels: vec!["red".into(), "blue".into()],
            other: None,
        },
        QuestionAnswer {
            labels: vec![],
            other: Some("teal".into()),
        },
    ];
    let choices = [
        PromptChoice::Allow,
        PromptChoice::AlwaysAllow,
        PromptChoice::Deny { message: None },
        PromptChoice::Deny {
            message: Some("not that".into()),
        },
        PromptChoice::ApprovePlan { accept_edits: true },
        PromptChoice::ApprovePlan {
            accept_edits: false,
        },
        PromptChoice::KeepPlanning {
            note: Some("smaller".into()),
        },
        PromptChoice::Answer { answers },
        PromptChoice::DeclineQuestion,
    ];
    for choice in &choices {
        permission_roundtrip(choice);
    }

    let reasons = [
        WithdrawReason::TurnEnded,
        WithdrawReason::ProcessEnded,
        WithdrawReason::SessionStopped,
        WithdrawReason::SessionRemoved,
        WithdrawReason::AgentHungUp,
    ];
    let mut resolutions = vec![
        Resolution::Allowed,
        Resolution::AllowedAlways,
        Resolution::Denied,
        Resolution::KeptPlanning,
        Resolution::Answered,
        Resolution::Declined,
    ];
    resolutions.extend(
        PermissionMode::ALL
            .into_iter()
            .map(|mode| Resolution::PlanApproved { mode }),
    );
    resolutions.extend(reasons.map(|reason| Resolution::Withdrawn { reason }));
    for resolution in &resolutions {
        permission_roundtrip(resolution);
    }
    for mode in PermissionMode::ALL {
        permission_roundtrip(&mode);
    }
}

/// Feature 1122 (D4, contracts/wire.md W1, W2): each project's Jira base URL in `DaemonSettings`,
/// and one project's change in `SettingsSet`, survive both wires.
fn settings_with_base_urls(
    jira_base_urls: std::collections::BTreeMap<String, micold_core::jira::JiraBaseUrl>,
) -> DaemonSettings {
    DaemonSettings {
        jira_enabled: true,
        jira_connections: Default::default(),
        github_issues_enabled: true,
        jira_base_urls,
        plan_usage_enabled: true,
        plan_usage_warn_percent: 80,
        scrollback_lines: 10_000,
        env_include_enabled: true,
        env_include_script_path: String::new(),
        env_include_timeout_secs: 10,
        default_ai_cli: AiCli::ClaudeCode,
        pi_activity_component: true,
        save_terminal_history: true,
        tool_server_enabled: true,
        cross_session_access: CrossSessionAccess::Auto,
        pr_status_enabled: false,
        desktop_notifications: true,
        notification_kinds: NotificationKinds::default(),
        long_task_threshold_secs: 120,
        diff_layout: Default::default(),
    }
}

fn base_url(url: &str) -> micold_core::jira::JiraBaseUrl {
    micold_core::jira::JiraBaseUrl::parse(url).expect("valid base URL")
}

#[test]
fn jira_base_urls_round_trip_in_daemon_settings() {
    let empty = settings_with_base_urls(Default::default());
    json_roundtrip(&empty);
    postcard_roundtrip(&empty);
    let two = settings_with_base_urls(
        [
            ("/work/a".to_owned(), base_url("https://a.atlassian.net")),
            (
                "/work/b".to_owned(),
                base_url("https://jira.example.com/jira"),
            ),
        ]
        .into(),
    );
    json_roundtrip(&two);
    postcard_roundtrip(&two);
}

#[test]
fn a_daemon_settings_entry_that_does_not_parse_fails_to_decode() {
    let two = settings_with_base_urls(
        [("/work/a".to_owned(), base_url("https://a.atlassian.net"))].into(),
    );
    let mut json = serde_json::to_value(&two).expect("json encode");
    json["jira_base_urls"]["/work/a"] = serde_json::json!("http://a.atlassian.net");
    assert!(serde_json::from_value::<DaemonSettings>(json).is_err());
}

#[test]
fn a_jira_base_url_change_round_trips_in_settings_set() {
    use micold_core::settings::JiraBaseUrlChange;
    for change in [
        None,
        Some(JiraBaseUrlChange {
            project: "/work/a".into(),
            url: Some(base_url("https://a.atlassian.net")),
        }),
        Some(JiraBaseUrlChange {
            project: "/work/a".into(),
            url: None,
        }),
    ] {
        let set = ClientMsg::SettingsSet {
            plan_usage_enabled: None,
            plan_usage_warn_percent: None,
            req: 5,
            scrollback_lines: None,
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
            pi_activity_component: None,
            save_terminal_history: None,
            tool_server_enabled: None,
            cross_session_access: None,
            pr_status_enabled: None,
            jira_enabled: None,
            jira_connection: None,
            github_issues_enabled: None,
            jira_base_url: change.clone().map(Box::new),
            desktop_notifications: None,
            notification_kinds: None,
            long_task_threshold_secs: None,
            diff_layout: None,
        };
        json_roundtrip(&set);
        postcard_roundtrip(&set);
        match serde_json::from_slice::<ClientMsg>(&serde_json::to_vec(&set).unwrap()).unwrap() {
            ClientMsg::SettingsSet { jira_base_url, .. } => {
                assert_eq!(jira_base_url.map(|c| *c), change)
            }
            other => panic!("expected SettingsSet, got {other:?}"),
        }
    }
}
