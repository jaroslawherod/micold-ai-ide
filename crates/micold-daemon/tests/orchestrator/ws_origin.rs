//! #1732 US3 scenarios 1-2, FR-004, SC-003: `/api/ws` refuses a foreign `Origin` with 403 before
//! any message, and logs the refused origin at WARN.

use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::ws_support::{refused_status, Api, Ws, ALLOWED};

/// Everything logged while the returned guard lives, on this thread (the test's runtime is
/// current-thread, so the listener's tasks log here too).
fn capture() -> (Arc<Mutex<Vec<u8>>>, tracing::subscriber::DefaultGuard) {
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let logged = Arc::new(Mutex::new(Vec::new()));
    let sink = Sink(Arc::clone(&logged));
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .with_ansi(false)
        .with_writer(move || sink.clone())
        .finish();
    (logged, tracing::subscriber::set_default(subscriber))
}

#[tokio::test]
async fn a_foreign_origin_is_refused_with_403_and_logged() {
    let (logged, _guard) = capture();
    let api = Api::start().await;
    let status = refused_status(Ws::connect(api.addr, Some("https://evil.example")).await);
    assert_eq!(status, 403);
    let text = String::from_utf8(logged.lock().unwrap().clone()).unwrap();
    assert!(text.contains("WARN"), "{text}");
    assert!(
        text.contains("refused origin https://evil.example"),
        "{text}"
    );
}

#[tokio::test]
async fn the_right_host_on_another_port_is_refused() {
    let api = Api::start().await;
    let status = refused_status(Ws::connect(api.addr, Some("http://127.0.0.1:9999")).await);
    assert_eq!(status, 403);
}

#[tokio::test]
async fn every_foreign_attempt_is_refused() {
    let api = Api::start().await;
    for origin in [
        "http://localhost:8731",
        "https://127.0.0.1:8731",
        "null",
        "http://127.0.0.1",
    ] {
        assert_eq!(
            refused_status(Ws::connect(api.addr, Some(origin)).await),
            403,
            "{origin}"
        );
    }
}

#[tokio::test]
async fn the_allowed_origin_is_accepted() {
    let api = Api::start().await;
    let mut ws = Ws::connect(api.addr, Some(ALLOWED))
        .await
        .expect("accepted");
    assert_eq!(
        ws.ask(serde_json::json!({"id":1,"type":"ping"})).await["type"],
        "pong"
    );
}

#[tokio::test]
async fn no_origin_is_accepted_from_loopback() {
    let api = Api::start().await;
    let mut ws = Ws::connect(api.addr, None).await.expect("accepted");
    assert_eq!(
        ws.ask(serde_json::json!({"id":1,"type":"ping"})).await["type"],
        "pong"
    );
}

#[tokio::test]
async fn another_path_is_404() {
    let api = Api::start().await;
    assert_eq!(
        refused_status(Ws::connect_path(api.addr, "/other", Some(ALLOWED)).await),
        404
    );
}
