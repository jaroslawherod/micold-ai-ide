//! T012 — wire round-trip fidelity (contracts/messages.md, protocol.md §3).
//!
//! Every `ClientMsg`/`DaemonMsg` round-trips under JSON (the control plane), every grid type
//! round-trips under `postcard` (the grid plane), and a `GridFrame` survives encode→decode
//! byte-identical with its wide-char spacer and zerowidth marks intact.

use std::path::PathBuf;

use micold_core::protocol::envelope::{Encoding, EnvelopeError, EnvelopeHeader, Kind, HEADER_LEN};
use micold_core::protocol::grid::{
    CellExtras, GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine,
    WireStyle,
};
use micold_core::protocol::messages::{
    ActivitySignal, CatalogSnapshot, ClientMsg, DaemonMsg, DaemonSettings, ErrorKind, ExitStatus,
    LogEntry, LogSink, OperationResult, ProjectSnapshot, RefusalReason, SessionSummary,
    WireLifecycle, WorktreeSnapshot, WorktreeStatus,
};
use micold_core::session::{AiCli, SessionId, SessionLabel, ShellInstanceId};
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
        ClientMsg::Hello {
            protocol_version: 1,
            schema_hash: [7u8; 32],
            client_build: "client-abc".into(),
            client_package_version: "0.4.0".into(),
        },
        ClientMsg::Attach {
            project: PathBuf::from("/repo"),
            force: true,
        },
        ClientMsg::Detach {
            project: PathBuf::from("/repo"),
        },
        ClientMsg::Goodbye,
        ClientMsg::SessionInput {
            session: sid(),
            serial: 42,
            bytes: vec![0x1b, b'[', b'A'],
        },
        ClientMsg::SessionResize {
            session: sid(),
            cols: 120,
            rows: 40,
        },
        ClientMsg::SessionStart { session: sid() },
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
        },
        // Feature 016: the non-default modes and both read-only branch queries must survive the
        // wire too — `TrackRemote` carries a payload, so it is the interesting one.
        ClientMsg::WorktreeCreate {
            req: 41,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::ReuseLocal,
        },
        ClientMsg::WorktreeCreate {
            req: 42,
            project: PathBuf::from("/a"),
            branch: "feat/x".into(),
            dir_name: "feat-x".into(),
            mode: CreateMode::TrackRemote {
                remote: "origin".into(),
            },
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
        ClientMsg::SessionCreate {
            req: 7,
            project: PathBuf::from("/a"),
            worktree_dir: "feat-x".into(),
            provider: AiCli::ClaudeCode,
        },
        // The same message on the second provider (feature 026, T021). Both variants ride the
        // wire, not just the default one — an encoding that only ever saw `ClaudeCode` would
        // round-trip fine and still lose `Copilot`.
        ClientMsg::SessionCreate {
            req: 71,
            project: PathBuf::from("/a"),
            worktree_dir: "feat-y".into(),
            provider: AiCli::Copilot,
        },
        ClientMsg::SessionDelete {
            req: 8,
            session: sid(),
        },
        ClientMsg::SettingsSet {
            req: 9,
            scrollback_lines: Some(50_000),
            env_include_enabled: Some(false),
            env_include_script_path: Some("/custom/rc".into()),
            env_include_timeout_secs: Some(20),
            default_ai_cli: Some(AiCli::Copilot),
        },
        // And the "leave it unchanged" form, which is what every settings save that is not about
        // the AI CLI sends.
        ClientMsg::SettingsSet {
            req: 91,
            scrollback_lines: Some(50_000),
            env_include_enabled: None,
            env_include_script_path: None,
            env_include_timeout_secs: None,
            default_ai_cli: None,
        },
        ClientMsg::LogLocationRequest { req: 10 },
        ClientMsg::RecentErrorsRequest { req: 11, limit: 20 },
        ClientMsg::SetLogLevel {
            req: 12,
            directives: "micold_daemon=debug".into(),
        },
        ClientMsg::Ping { nonce: 0xdead_beef },
    ]
}

fn sample_summary() -> SessionSummary {
    SessionSummary {
        id: sid(),
        worktree_dir: Some("feat-x".into()),
        provider: AiCli::Copilot,
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
    }
}

fn sample_catalog() -> CatalogSnapshot {
    CatalogSnapshot {
        schema_version: 1,
        last_active: Some(PathBuf::from("/a")),
        projects: vec![ProjectSnapshot {
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
                scrollback_lines: 10_000,
                env_include_enabled: true,
                env_include_script_path: "/home/user/.bashrc".into(),
                env_include_timeout_secs: 10,
                default_ai_cli: AiCli::ClaudeCode,
            },
        },
        DaemonMsg::Refused {
            reason: RefusalReason::ProjectBusy {
                project: PathBuf::from("/a"),
                holder: "other".into(),
                since_secs: 120,
            },
        },
        DaemonMsg::Attached {
            project: PathBuf::from("/a"),
            sessions: vec![sample_summary()],
        },
        DaemonMsg::Displaced {
            project: PathBuf::from("/a"),
            by: "other-client".into(),
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
                scrollback_lines: 1_000,
                env_include_enabled: false,
                env_include_script_path: String::new(),
                env_include_timeout_secs: 5,
                default_ai_cli: AiCli::Copilot,
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
        DaemonMsg::OperationError {
            req: 4,
            kind: ErrorKind::GitFailed,
            message: "worktree create failed".into(),
            detail: Some("fatal: branch 'feat/x' already exists".into()),
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
    ]
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
