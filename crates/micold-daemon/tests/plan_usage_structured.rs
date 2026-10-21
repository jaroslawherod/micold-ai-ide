//! Feature 944, US4 (T017): a structured session's plan-limit events feed feature 490's plan-usage
//! reading (contracts/plan-usage-source.md *Feed*, *Guarantees*; FR-010–FR-013, SC-004).
//!
//! The stand-in CLI of `support/structured.rs` replays `text-turn.jsonl` with its reset times moved
//! to after the test's clock, so the recorded windows stay current whenever the test runs.
// unix-only: the stand-in CLI of support/structured.rs is put first on PATH as `claude`.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use futures_util::{SinkExt, StreamExt};
use micold_core::plan_usage::{LimitWindow, UsageReading};
use micold_core::protocol::codec::Frame;
use micold_core::protocol::messages::DaemonMsg;
use micold_core::session::{SessionId, SessionLocation};
use micold_core::structured::StructuredEvent;
use micold_core::terminal::LaunchMode;
use micold_daemon::state::DaemonState;

#[path = "support/conn.rs"]
mod conn;
#[path = "support/history.rs"]
mod history;
#[path = "support/mcp.rs"]
mod mcp;
#[path = "support/structured.rs"]
mod structured;
use conn::{answer, connect, settings_set, Client, SettingsPatch};
use history::{service, service_saving};
use structured::{
    events_until, fake_stream_cli, fixture, stream_script, structured_session, viewer,
};

/// The recorded reset times of `text-turn.jsonl`: the 5-hour window's (also the event's top-level
/// one) and the weekly one's.
const RECORDED_FIVE_HOUR_RESET: &str = "1792363200";
const RECORDED_WEEKLY_RESET: &str = "1792904400";

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// One structured session in a project, and `text-turn.jsonl` with its reset times moved to one
/// hour and six days from now, written beside it.
struct Case {
    project: tempfile::TempDir,
    session: micold_core::session::Session,
    turn: PathBuf,
    five_hour_reset: i64,
    weekly_reset: i64,
}

impl Case {
    fn new() -> Case {
        fake_stream_cli();
        let project = tempfile::tempdir().unwrap();
        let (five_hour_reset, weekly_reset) = (now() + 3600, now() + 6 * 24 * 3600);
        let recorded = std::fs::read_to_string(fixture("text-turn.jsonl")).unwrap();
        let shifted = recorded
            .replace(RECORDED_FIVE_HOUR_RESET, &five_hour_reset.to_string())
            .replace(RECORDED_WEEKLY_RESET, &weekly_reset.to_string());
        let turn = project.path().join("turn.jsonl");
        std::fs::write(&turn, shifted).unwrap();
        Case {
            project,
            session: structured_session(SessionLocation::Default),
            turn,
            five_hour_reset,
            weekly_reset,
        }
    }

    fn id(&self) -> SessionId {
        self.session.id
    }

    fn dir(&self) -> &Path {
        self.project.path()
    }

    /// A service over the case's session with its plan-usage feed running, as `server::run`
    /// starts it.
    fn daemon(&self) -> Arc<DaemonState> {
        self.with_feed(service(self.dir(), vec![self.session.clone()]))
    }

    fn with_feed(&self, state: Arc<DaemonState>) -> Arc<DaemonState> {
        micold_daemon::server::spawn_plan_usage_feed(Arc::clone(&state));
        state
    }

    /// The fixture's two windows, as a reading of them holds them.
    fn windows(&self) -> Vec<LimitWindow> {
        vec![
            LimitWindow {
                key: "five_hour".into(),
                used_tenths: 0,
                resets_at: Some(self.five_hour_reset),
            },
            LimitWindow {
                key: "seven_day".into(),
                used_tenths: 100,
                resets_at: Some(self.weekly_reset),
            },
        ]
    }

    /// The number of lines the stand-in writes for `script`'s `replay` of the shifted turn.
    fn turn_lines(&self) -> usize {
        std::fs::read_to_string(&self.turn).unwrap().lines().count()
    }

    fn replay(&self) -> String {
        format!("replay {}\n", self.turn.display())
    }
}

/// The `PlanUsageChanged` readings `client` receives within `wait`, stopping at the `stop`th.
async fn usage_changes(
    client: &mut Client,
    stop: usize,
    wait: Duration,
) -> Vec<Option<UsageReading>> {
    let mut got = Vec::new();
    let deadline = tokio::time::Instant::now() + wait;
    while got.len() < stop {
        let left = deadline.saturating_duration_since(tokio::time::Instant::now());
        match tokio::time::timeout(left, client.next()).await {
            Ok(Some(Ok(Frame::Control(DaemonMsg::PlanUsageChanged { reading })))) => {
                got.push(reading)
            }
            Ok(Some(Ok(_))) => {}
            Ok(Some(Err(e))) => panic!("codec error: {e:?}"),
            Ok(None) => panic!("the connection closed"),
            Err(_) => break,
        }
    }
    got
}

/// Every event of `id` a viewer receives, once there are `n`.
async fn history_of(state: &Arc<DaemonState>, case: &Case, n: usize) -> Vec<StructuredEvent> {
    let mut client = viewer(state, case.dir(), case.id()).await;
    structured::events_of(&events_until(&mut client, case.id(), |got| got.len() >= n).await)
}

fn has_plan_limit_event(events: &[StructuredEvent]) -> bool {
    events.iter().any(|event| {
        matches!(event, StructuredEvent::Unrecognised { raw } if raw.contains(r#""type":"rate_limit_event""#))
    })
}

/// Turn **Show Claude plan usage** on or off as a window does (feature 490's setting).
async fn switch(window: &mut Client, on: bool) {
    window
        .send(Frame::Control(settings_set(
            1,
            SettingsPatch {
                plan_usage_enabled: Some(on),
                ..Default::default()
            },
        )))
        .await
        .unwrap();
    assert!(matches!(
        answer(window, 1).await,
        DaemonMsg::OperationOk { .. }
    ));
}

// FR-010, SC-004, US4 s1 and s3: with the switch on, the event's windows reach every window within
// 10 s, and the event stays in the session's history.
#[tokio::test(flavor = "multi_thread")]
async fn with_the_switch_on_the_event_is_a_reading_every_window_gets_within_10_s() {
    let case = Case::new();
    let state = case.daemon();
    let mut first = connect(&state).await;
    let mut second = connect(&state).await;
    stream_script(case.dir(), &case.replay());
    let started = Instant::now();
    state
        .start_session(case.id(), LaunchMode::Fresh)
        .expect("starts");

    for window in [&mut first, &mut second] {
        let got = usage_changes(window, 1, Duration::from_secs(10)).await;
        assert!(started.elapsed() < Duration::from_secs(10));
        let reading = got
            .into_iter()
            .next()
            .flatten()
            .expect("a reading within 10 s");
        assert_eq!(reading.windows, case.windows());
        assert!(
            (reading.obtained_at - now()).abs() <= 10,
            "stamped by the service"
        );
    }
    assert_eq!(
        state.welcome_plan_usage().map(|r| r.windows),
        Some(case.windows())
    );
    assert!(has_plan_limit_event(
        &history_of(&state, &case, case.turn_lines()).await
    ));
    state.stop_session(case.id());
}

// FR-011, US4 s3: with the switch off, no reading is taken, kept or shown; the event stays in the
// history.
#[tokio::test(flavor = "multi_thread")]
async fn with_the_switch_off_no_reading_is_kept_or_shown() {
    let case = Case::new();
    let state = case.daemon();
    let mut window = connect(&state).await;
    switch(&mut window, false).await;
    stream_script(case.dir(), &case.replay());
    state
        .start_session(case.id(), LaunchMode::Fresh)
        .expect("starts");

    let events = history_of(&state, &case, case.turn_lines()).await;
    assert!(has_plan_limit_event(&events));
    assert_eq!(
        usage_changes(&mut window, 1, Duration::from_secs(2)).await,
        [],
        "nothing shown"
    );
    assert_eq!(state.welcome_plan_usage(), None, "nothing kept");
    switch(&mut window, true).await;
    assert_eq!(
        state.welcome_plan_usage(),
        None,
        "nothing kept to show later"
    );
    state.stop_session(case.id());
}

// FR-012, US4 s4: an event that cannot be read changes nothing; the readings before and after it
// are the only ones shown.
#[tokio::test(flavor = "multi_thread")]
async fn an_unreadable_event_leaves_the_current_reading_as_it_is() {
    let case = Case::new();
    let state = case.daemon();
    let mut window = connect(&state).await;
    let later = format!(
        r#"{{"type":"rate_limit_event","rate_limit_info":{{"unifiedWindows":{{"five_hour":{{"utilization":0.5,"resetsAt":{}}}}}}}}}"#,
        case.five_hour_reset
    );
    let script = format!(
        "{}{}\n{}\n{}\n",
        case.replay(),
        r#"{"type":"rate_limit_event","rate_limit_info":{"unifiedWindows":{"five_hour":{"utilization":"0.5"},"seven_day":-3}}}"#,
        r#"{"type":"rate_limit_event","rate_limit_info":"none"}"#,
        later
    );
    stream_script(case.dir(), &script);
    state
        .start_session(case.id(), LaunchMode::Fresh)
        .expect("starts");

    let got = usage_changes(&mut window, 3, Duration::from_secs(10)).await;
    let windows: Vec<Vec<LimitWindow>> = got
        .into_iter()
        .map(|r| r.expect("a reading").windows)
        .collect();
    assert_eq!(
        windows,
        [
            case.windows(),
            vec![LimitWindow {
                key: "five_hour".into(),
                used_tenths: 500,
                resets_at: Some(case.five_hour_reset),
            }],
        ],
        "the unreadable events in between showed nothing and kept the reading"
    );
    let events = history_of(&state, &case, case.turn_lines() + 3).await;
    let kept = events
        .iter()
        .filter(|e| matches!(e, StructuredEvent::Unrecognised { raw } if raw.contains("rate_limit_event")))
        .count();
    assert_eq!(kept, 4, "every plan-limit event stays in the history");
    state.stop_session(case.id());
}

// FR-013, US4 s5: a history restored after a restart, holding the event, yields no reading; only
// what the running CLI writes would.
#[tokio::test(flavor = "multi_thread")]
async fn a_restored_history_holding_the_event_yields_no_reading() {
    let case = Case::new();
    let saved = tempfile::tempdir().unwrap();
    let state = case.with_feed(service_saving(
        case.dir(),
        vec![case.session.clone()],
        saved.path(),
    ));
    stream_script(case.dir(), &case.replay());
    state
        .start_session(case.id(), LaunchMode::Fresh)
        .expect("starts");
    assert!(has_plan_limit_event(
        &history_of(&state, &case, case.turn_lines()).await
    ));
    micold_daemon::history::save_all_live(&state, micold_daemon::history::STOP_SAVE_BOUND).await;
    state.stop_session(case.id());
    drop(state);

    let restarted = case.with_feed(service_saving(
        case.dir(),
        vec![case.session.clone()],
        saved.path(),
    ));
    let mut window = connect(&restarted).await;
    stream_script(case.dir(), "");
    restarted
        .start_session(case.id(), LaunchMode::Fresh)
        .expect("starts again");
    let restored = history_of(&restarted, &case, case.turn_lines()).await;
    assert!(has_plan_limit_event(&restored), "the event was restored");
    assert_eq!(
        usage_changes(&mut window, 1, Duration::from_secs(2)).await,
        [],
        "a restored event is no reading"
    );
    assert_eq!(restarted.welcome_plan_usage(), None);
    restarted.stop_session(case.id());
}
