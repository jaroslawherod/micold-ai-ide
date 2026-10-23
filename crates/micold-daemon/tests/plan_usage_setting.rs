//! Feature 490, M1: the two plan-usage settings, held by the service (contracts/plan-usage-wire.md
//! W3; FR-002, FR-005, FR-018). Mirrors `settings_long_task_threshold.rs`.

#[path = "support/attention.rs"]
mod attention;

use std::time::{SystemTime, UNIX_EPOCH};

use attention::{
    connect_with_welcome, received, sends, session_id, settings_set, Service, SettingsPatch, Window,
};
use micold_core::plan_usage::{LimitWindow, UsageReading};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, DaemonSettings};
use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};

const A: u128 = 0x490A;

fn set(plan_usage_enabled: Option<bool>, plan_usage_warn_percent: Option<u8>) -> ClientMsg {
    settings_set(
        1,
        SettingsPatch {
            plan_usage_enabled,
            plan_usage_warn_percent,
            ..Default::default()
        },
    )
}

async fn window(service: &Service) -> (Window, DaemonSettings, Option<UsageReading>) {
    match connect_with_welcome(&service.state, "plan-usage-setting").await {
        (
            window,
            DaemonMsg::Welcome {
                settings,
                plan_usage,
                ..
            },
        ) => (window, settings, plan_usage),
        (_, other) => unreachable!("a Welcome, not {other:?}"),
    }
}

fn stored(service: &Service) -> Settings {
    JsonFileSettingsStore::at(service.store.path().join("settings.json"))
        .load()
        .settings
}

fn pushed_settings(msgs: &[DaemonMsg]) -> Option<DaemonSettings> {
    msgs.iter().rev().find_map(|msg| match msg {
        DaemonMsg::SettingsChanged { settings } => Some(settings.clone()),
        _ => None,
    })
}

fn reading() -> UsageReading {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    UsageReading {
        windows: vec![LimitWindow {
            key: "five_hour".into(),
            used_tenths: 420,
            resets_at: Some(now + 3600),
        }],
        obtained_at: now,
    }
}

#[tokio::test]
async fn the_service_settings_carry_both_fields_with_their_defaults() {
    let service = Service::with_sessions(&[]);
    let (_window, settings, _) = window(&service).await;
    assert!(settings.plan_usage_enabled, "FR-001");
    assert_eq!(settings.plan_usage_warn_percent, 80, "FR-005");
}

#[tokio::test]
async fn an_out_of_range_threshold_is_clamped_persisted_and_pushed_to_every_window() {
    let service = Service::with_sessions(&[]);
    let (mut one, _, _) = window(&service).await;
    let (mut two, _, _) = window(&service).await;
    let answers = sends(&mut one, set(None, Some(30))).await;
    assert!(
        answers
            .iter()
            .any(|msg| matches!(msg, DaemonMsg::OperationOk { req: 1, .. })),
        "{answers:?}"
    );
    assert_eq!(
        pushed_settings(&answers).map(|s| s.plan_usage_warn_percent),
        Some(50),
        "FR-005: clamped"
    );
    assert_eq!(
        pushed_settings(&received(&mut two).await).map(|s| s.plan_usage_warn_percent),
        Some(50),
        "FR-018: the other window too"
    );
    assert_eq!(stored(&service).plan_usage_warn_percent, 50, "persisted");
    assert!(
        stored(&service).plan_usage_enabled,
        "the other field is left alone"
    );
}

#[tokio::test]
async fn turning_the_switch_off_persists_it_and_clears_the_reading_in_every_window() {
    let service = Service::with_sessions(&[session_id(A)]);
    assert!(service.state.note_plan_usage(session_id(A), reading()));
    let (mut one, _, held) = window(&service).await;
    assert!(held.is_some(), "a reading is held");
    let (mut two, _, _) = window(&service).await;

    let answers = sends(&mut one, set(Some(false), None)).await;
    assert_eq!(
        pushed_settings(&answers).map(|s| s.plan_usage_enabled),
        Some(false)
    );
    for msgs in [answers, received(&mut two).await] {
        assert!(
            msgs.contains(&DaemonMsg::PlanUsageChanged { reading: None }),
            "FR-002: cleared: {msgs:?}"
        );
    }
    assert!(!stored(&service).plan_usage_enabled, "persisted");
    let (_late, settings, held) = window(&service).await;
    assert!(!settings.plan_usage_enabled);
    assert_eq!(held, None);
    assert!(
        !service.state.note_plan_usage(session_id(A), reading()),
        "nothing is kept while off"
    );
}

#[tokio::test]
async fn turning_the_switch_on_again_keeps_the_reading_empty_until_one_arrives() {
    let service = Service::with_sessions(&[session_id(A)]);
    service.state.set_plan_usage_enabled(false).unwrap();
    let (mut one, _, _) = window(&service).await;
    let answers = sends(&mut one, set(Some(true), None)).await;
    assert_eq!(
        pushed_settings(&answers).map(|s| s.plan_usage_enabled),
        Some(true)
    );
    assert!(
        !answers
            .iter()
            .any(|msg| matches!(msg, DaemonMsg::PlanUsageChanged { .. })),
        "nothing to clear: {answers:?}"
    );
    assert!(service.state.note_plan_usage(session_id(A), reading()));
}

/// Feature 490 T021 (research R9; FR-002, SC-007): the switch rewrites the settings file of every
/// live Claude Code session and writes or deletes its relay file.
#[tokio::test]
async fn the_switch_rewrites_every_live_sessions_settings_and_relay_file() {
    use micold_core::mcp::binding::ConfigLocations;
    use micold_daemon::hooks::{HookReceiver, StatusContext};

    let service = Service::with_sessions(&[session_id(A)]);
    let dir = tempfile::tempdir().unwrap();
    let (receiver, _listener) = HookReceiver::bind(dir.path().join("hooks")).await.unwrap();
    let context = StatusContext {
        cwd: dir.path().join("project"),
        locations: ConfigLocations::default(),
        guarded: true,
    };
    receiver.set_plan_usage(true);
    let settings = receiver.prepare_settings(session_id(A), context).unwrap();
    let relay = receiver.relay_file(session_id(A));
    service.state.set_hooks(receiver);
    let has_block = || {
        let doc: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
        doc.get("statusLine").is_some()
    };
    assert!(has_block() && relay.exists());

    let (mut one, _, _) = window(&service).await;
    sends(&mut one, set(Some(false), None)).await;
    assert!(!has_block(), "off: no statusLine block");
    assert!(!relay.exists(), "off: no relay file");

    sends(&mut one, set(Some(true), None)).await;
    assert!(has_block(), "on: the block is back");
    assert!(relay.exists(), "on: the relay file is back");
}
