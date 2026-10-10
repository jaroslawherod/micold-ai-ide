//! Feature 490, M1: plan-usage readings reach the service over the hook receiver's
//! `POST /status/<session-uuid>` route (contracts/status-line-relay.md S3; research R5, R6, R7),
//! and every window gets the one stored reading (FR-018, FR-019).

#[path = "support/attention.rs"]
mod attention;

use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use attention::{connect_with_welcome, received, session_id, Service, Window};
use micold_core::plan_usage::{LimitWindow, UsageReading};
use micold_core::protocol::messages::DaemonMsg;
use micold_core::session::SessionId;
use micold_daemon::hooks::{self, HookReceiver};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const A: u128 = 0x490A;
const B: u128 = 0x490B;
/// The route's body bound (S3).
const MAX_STATUS_BODY: usize = 16 * 1024;

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// A service with live Claude Code sessions, its hook receiver serving, and one window.
struct Receiver {
    service: Service,
    receiver: HookReceiver,
    addr: String,
    _settings_dir: tempfile::TempDir,
}

impl Receiver {
    async fn with_sessions(sessions: &[SessionId]) -> Self {
        let service = Service::with_sessions(sessions);
        let settings_dir = tempfile::tempdir().unwrap();
        let (receiver, listener) = HookReceiver::bind(settings_dir.path().to_path_buf())
            .await
            .expect("the receiver binds");
        let addr = listener.local_addr().unwrap().to_string();
        tokio::spawn(hooks::serve(
            listener,
            receiver.tokens(),
            Arc::clone(&service.state),
        ));
        Self {
            service,
            receiver,
            addr,
            _settings_dir: settings_dir,
        }
    }

    async fn window(&self) -> (Window, Option<UsageReading>) {
        match connect_with_welcome(&self.service.state, "plan-usage").await {
            (window, DaemonMsg::Welcome { plan_usage, .. }) => (window, plan_usage),
            (_, other) => unreachable!("a Welcome, not {other:?}"),
        }
    }

    /// POST `body` to `session`'s status route with its own token.
    async fn status(&self, session: SessionId, body: &str) -> u16 {
        let token = self.receiver.token_for(session);
        let path = format!("/status/{}", session.0);
        request(&self.addr, "POST", &path, Some(&token), body.len(), body).await
    }
}

/// Send one request and return the status code of the answer.
async fn request(
    addr: &str,
    method: &str,
    path: &str,
    token: Option<&str>,
    content_length: usize,
    body: &str,
) -> u16 {
    let mut stream = TcpStream::connect(addr).await.expect("connect");
    let auth = token
        .map(|t| format!("Authorization: Bearer {t}\r\n"))
        .unwrap_or_default();
    let head = format!(
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\n{auth}Content-Length: {content_length}\r\nConnection: close\r\n\r\n{body}",
    );
    stream.write_all(head.as_bytes()).await.expect("write");
    stream.flush().await.expect("flush");
    let mut response = Vec::new();
    stream.read_to_end(&mut response).await.expect("read");
    String::from_utf8_lossy(&response)
        .split_whitespace()
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("a status code in the response line")
}

/// A `rate_limits` body: the 5-hour window at `five_hour` percent, the weekly one at 18%.
fn rate_limits(five_hour: f64, resets_in: i64) -> String {
    serde_json::json!({
        "five_hour": {"used_percentage": five_hour, "resets_at": now() + resets_in},
        "seven_day": {"used_percentage": 18, "resets_at": now() + 4 * 86_400},
    })
    .to_string()
}

fn usage_changes(msgs: &[DaemonMsg]) -> Vec<Option<UsageReading>> {
    msgs.iter()
        .filter_map(|msg| match msg {
            DaemonMsg::PlanUsageChanged { reading } => Some(reading.clone()),
            _ => None,
        })
        .collect()
}

fn five_hour_tenths(reading: &UsageReading) -> u32 {
    reading
        .windows
        .iter()
        .find(|w| w.key == "five_hour")
        .map(|w: &LimitWindow| w.used_tenths)
        .expect("a 5-hour window")
}

#[tokio::test]
async fn only_a_post_is_a_route() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let token = rx.receiver.token_for(session_id(A));
    let path = format!("/status/{}", session_id(A).0);
    assert_eq!(
        request(&rx.addr, "GET", &path, Some(&token), 0, "").await,
        405
    );
}

#[tokio::test]
async fn any_other_path_is_not_found() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let token = rx.receiver.token_for(session_id(A));
    let body = rate_limits(42.0, 3600);
    for path in [
        format!("/usage/{}", session_id(A).0),
        "/status/not-a-uuid".to_owned(),
        format!("/status/{}/more", session_id(A).0),
        "/status/".to_owned(),
    ] {
        assert_eq!(
            request(&rx.addr, "POST", &path, Some(&token), body.len(), &body).await,
            404,
            "{path}"
        );
    }
}

#[tokio::test]
async fn a_missing_or_wrong_token_is_forbidden_before_the_body_is_read() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let _ = rx.receiver.token_for(session_id(A));
    let path = format!("/status/{}", session_id(A).0);
    let body = rate_limits(42.0, 3600);
    assert_eq!(
        request(&rx.addr, "POST", &path, None, body.len(), &body).await,
        403
    );
    assert_eq!(
        request(&rx.addr, "POST", &path, Some("wrong"), body.len(), &body).await,
        403
    );
    assert_eq!(
        request(
            &rx.addr,
            "POST",
            &path,
            Some("wrong"),
            MAX_STATUS_BODY + 1,
            ""
        )
        .await,
        403,
        "an over-bound body is not read before the token is checked"
    );
    let (_window, stored) = rx.window().await;
    assert_eq!(stored, None, "nothing is stored");
}

#[tokio::test]
async fn a_body_over_16_kib_is_too_large() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let token = rx.receiver.token_for(session_id(A));
    let path = format!("/status/{}", session_id(A).0);
    assert_eq!(
        request(
            &rx.addr,
            "POST",
            &path,
            Some(&token),
            MAX_STATUS_BODY + 1,
            ""
        )
        .await,
        413
    );
}

#[tokio::test]
async fn a_body_that_is_not_a_json_object_is_a_bad_request() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    for body in ["nope", "[1,2]", "\"five_hour\"", ""] {
        assert_eq!(rx.status(session_id(A), body).await, 400, "{body:?}");
    }
}

#[tokio::test]
async fn an_object_with_no_usable_window_is_accepted_and_stores_nothing() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let (mut window, _) = rx.window().await;
    for body in [
        "{}",
        r#"{"spend_limit":{"used_percentage":3}}"#,
        r#"{"five_hour":{"used_percentage":-1}}"#,
    ] {
        assert_eq!(rx.status(session_id(A), body).await, 200, "{body}");
    }
    assert_eq!(usage_changes(&received(&mut window).await), vec![]);
    let (_late, stored) = rx.window().await;
    assert_eq!(stored, None);
}

#[tokio::test]
async fn a_usable_reading_is_stored_and_pushed_to_every_window_stamped_by_the_service() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let (mut one, _) = rx.window().await;
    let (mut two, _) = rx.window().await;
    let before = now();
    assert_eq!(
        rx.status(session_id(A), &rate_limits(42.04, 3600)).await,
        200
    );
    let after = now();
    for window in [&mut one, &mut two] {
        let changes = usage_changes(&received(window).await);
        assert_eq!(changes.len(), 1, "one push per window: {changes:?}");
        let reading = changes[0].clone().expect("a reading");
        assert_eq!(five_hour_tenths(&reading), 420);
        assert!(
            (before..=after).contains(&reading.obtained_at),
            "obtained_at is the service's clock"
        );
    }
    let (_late, stored) = rx.window().await;
    assert_eq!(
        stored.map(|r| five_hour_tenths(&r)),
        Some(420),
        "W1: Welcome carries it"
    );
}

#[tokio::test]
async fn the_newest_reading_from_any_session_wins() {
    let rx = Receiver::with_sessions(&[session_id(A), session_id(B)]).await;
    let (mut window, _) = rx.window().await;
    assert_eq!(
        rx.status(session_id(A), &rate_limits(42.0, 3600)).await,
        200
    );
    assert_eq!(
        rx.status(session_id(B), &rate_limits(30.0, 3600)).await,
        200
    );
    let changes = usage_changes(&received(&mut window).await);
    let tenths: Vec<u32> = changes.iter().flatten().map(five_hour_tenths).collect();
    assert_eq!(tenths, vec![420, 300], "FR-019");
    let (_late, stored) = rx.window().await;
    assert_eq!(stored.map(|r| five_hour_tenths(&r)), Some(300));
}

#[tokio::test]
async fn an_idle_session_repeating_its_reading_does_not_undo_a_newer_one() {
    let rx = Receiver::with_sessions(&[session_id(A), session_id(B)]).await;
    let (mut window, _) = rx.window().await;
    let idle = rate_limits(42.0, 3600);
    assert_eq!(rx.status(session_id(A), &idle).await, 200);
    assert_eq!(
        rx.status(session_id(B), &rate_limits(62.0, 3600)).await,
        200
    );
    assert_eq!(rx.status(session_id(A), &idle).await, 200);
    let changes = usage_changes(&received(&mut window).await);
    let tenths: Vec<u32> = changes.iter().flatten().map(five_hour_tenths).collect();
    assert_eq!(tenths, vec![420, 620], "FR-019: a repeat is not newer");
    let (_late, stored) = rx.window().await;
    assert_eq!(stored.map(|r| five_hour_tenths(&r)), Some(620));
}

#[tokio::test]
async fn ten_sessions_reporting_the_same_windows_cause_one_push() {
    let sessions: Vec<SessionId> = (0..10).map(|n| session_id(0x4900 + n)).collect();
    let rx = Receiver::with_sessions(&sessions).await;
    let (mut window, _) = rx.window().await;
    let body = rate_limits(42.0, 3600);
    for session in &sessions {
        assert_eq!(rx.status(*session, &body).await, 200);
    }
    assert_eq!(
        usage_changes(&received(&mut window).await).len(),
        1,
        "Principle II: one stored reading, one push"
    );
}

#[tokio::test]
async fn an_identical_reading_within_a_minute_is_not_pushed_again() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let (mut window, _) = rx.window().await;
    let body = rate_limits(42.0, 3600);
    assert_eq!(rx.status(session_id(A), &body).await, 200);
    assert_eq!(rx.status(session_id(A), &body).await, 200);
    assert_eq!(usage_changes(&received(&mut window).await).len(), 1, "R6");
}

#[tokio::test]
async fn welcome_carries_no_reading_once_every_window_has_reset() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let body = serde_json::json!({"five_hour": {"used_percentage": 42, "resets_at": now() + 2}})
        .to_string();
    assert_eq!(rx.status(session_id(A), &body).await, 200);
    let (_window, stored) = rx.window().await;
    assert!(stored.is_some(), "current at first");
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let (_late, stored) = rx.window().await;
    assert_eq!(stored, None, "R7: dropped once its reset has passed");
}

#[tokio::test]
async fn with_the_switch_off_a_reading_is_accepted_and_dropped() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    rx.service.state.set_plan_usage_enabled(false).unwrap();
    let (mut window, stored) = rx.window().await;
    assert_eq!(stored, None);
    assert_eq!(
        rx.status(session_id(A), &rate_limits(42.0, 3600)).await,
        200,
        "S3.2"
    );
    assert_eq!(usage_changes(&received(&mut window).await), vec![]);
    let (_late, stored) = rx.window().await;
    assert_eq!(stored, None);
}

#[tokio::test]
async fn a_session_the_service_does_not_run_cannot_store_a_reading() {
    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    let stranger = session_id(0x5747);
    assert_eq!(rx.status(stranger, &rate_limits(42.0, 3600)).await, 200);
    let (_window, stored) = rx.window().await;
    assert_eq!(stored, None);
}

/// A tracing writer that keeps every line, for the silence test.
#[derive(Clone, Default)]
struct Captured(Arc<Mutex<Vec<u8>>>);

impl std::io::Write for Captured {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for Captured {
    type Writer = Captured;
    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Captured {
    fn lines(&self) -> Vec<String> {
        String::from_utf8_lossy(&self.0.lock().unwrap())
            .lines()
            .map(str::to_owned)
            .collect()
    }
}

/// S3.1 (FR-015, FR-016, SC-003): a rejection is logged at debug, once per (session, answer)
/// until that session's next accepted reading, never at warn or above, and never with the body.
#[tokio::test]
async fn rejections_are_logged_at_debug_once_and_never_with_the_body() {
    let captured = Captured::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(captured.clone())
        .with_max_level(tracing::Level::TRACE)
        .with_ansi(false)
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let rx = Receiver::with_sessions(&[session_id(A)]).await;
    const SECRET: &str = "a-body-that-must-not-be-logged";
    let bad = format!("[\"{SECRET}\"]");
    let empty = format!("{{\"{SECRET}\": 1}}");
    for _ in 0..3 {
        assert_eq!(rx.status(session_id(A), &bad).await, 400);
        assert_eq!(rx.status(session_id(A), &empty).await, 200);
    }
    let status_lines = |captured: &Captured| -> Vec<String> {
        captured
            .lines()
            .into_iter()
            .filter(|line| line.contains("plan usage"))
            .collect()
    };
    let lines = status_lines(&captured);
    assert_eq!(lines.len(), 2, "once per (session, answer): {lines:#?}");
    assert!(lines.iter().all(|l| l.contains("DEBUG")), "{lines:#?}");

    assert_eq!(
        rx.status(session_id(A), &rate_limits(42.0, 3600)).await,
        200
    );
    assert_eq!(rx.status(session_id(A), &bad).await, 400);
    assert_eq!(
        status_lines(&captured).len(),
        3,
        "an accepted reading resets the once-only rule"
    );

    let all = captured.lines();
    assert!(
        all.iter().all(|l| !l.contains(SECRET)),
        "the body is never logged: {all:#?}"
    );
    assert!(
        all.iter()
            .filter(|l| l.contains("plan usage"))
            .all(|l| !l.contains("WARN") && !l.contains("ERROR")),
        "{all:#?}"
    );
}
