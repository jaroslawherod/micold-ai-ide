//! #1732 US2, FR-005, FR-008: the application-level heartbeat; a silent client is released; five
//! malformed frames in a row close with 1008, an internal failure with 1011.

use std::sync::Arc;
use std::time::Duration;

use futures_util::future::{BoxFuture, FutureExt};
use micold_core::client_api::messages::{ContentEntry, Snapshot};
use micold_daemon::orchestrator::directory::{ApiError, LinkDirectory};
use micold_daemon::orchestrator::ws::{ApiOptions, IDLE};
use serde_json::json;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;

use crate::ws_support::{fast, serve, Api, Scripted, Ws, ALLOWED};

#[tokio::test]
async fn a_ping_is_answered_with_a_pong_of_the_same_id() {
    let api = Api::start().await;
    let mut ws = api.client().await;
    assert_eq!(
        ws.ask(json!({"id":9,"type":"ping"})).await,
        json!({"id":9,"type":"pong"})
    );
}

#[test]
fn a_client_is_released_after_two_heartbeat_intervals() {
    assert_eq!(IDLE, Duration::from_secs(40));
}

#[tokio::test]
async fn a_silent_client_is_released() {
    let options = ApiOptions {
        idle: Duration::from_millis(300),
        ..fast()
    };
    let addr = serve(Arc::new(Scripted::default()), options).await;
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    assert_eq!(
        ws.closed().await,
        None,
        "released with no close frame of ours"
    );
}

#[tokio::test]
async fn a_talking_client_is_kept() {
    let options = ApiOptions {
        idle: Duration::from_millis(400),
        ..fast()
    };
    let addr = serve(Arc::new(Scripted::default()), options).await;
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    for id in 0..6 {
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(ws.ask(json!({"id":id,"type":"ping"})).await["type"], "pong");
    }
}

#[tokio::test]
async fn five_malformed_frames_in_a_row_close_with_1008() {
    let api = Api::start().await;
    let mut ws = api.client().await;
    for _ in 0..4 {
        ws.send_text("nope".into()).await;
        assert_eq!(ws.next().await["code"], "malformed");
    }
    ws.send_text("nope".into()).await;
    assert_eq!(ws.next().await["code"], "malformed");
    assert_eq!(ws.closed().await, Some(CloseCode::Policy));
}

#[tokio::test]
async fn a_good_request_resets_the_malformed_count() {
    let api = Api::start().await;
    let mut ws = api.client().await;
    for round in 0..3 {
        for _ in 0..4 {
            ws.send_text("nope".into()).await;
            assert_eq!(ws.next().await["code"], "malformed");
        }
        assert_eq!(
            ws.ask(json!({"id":round,"type":"ping"})).await["type"],
            "pong"
        );
    }
}

/// A directory that fails inside.
struct Broken;

impl LinkDirectory for Broken {
    fn list(&self) -> BoxFuture<'_, Snapshot> {
        async { panic!("the directory failed") }.boxed()
    }

    fn read_session<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: &'a str,
    ) -> BoxFuture<'a, Result<Vec<ContentEntry>, ApiError>> {
        async { panic!("the directory failed") }.boxed()
    }

    fn changed(&self) -> BoxFuture<'_, ()> {
        std::future::pending().boxed()
    }
}

#[tokio::test]
async fn an_internal_failure_closes_with_1011() {
    let addr = serve(Arc::new(Broken), fast()).await;
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    ws.send(json!({"id":1,"type":"list_daemons"})).await;
    assert_eq!(ws.closed().await, Some(CloseCode::Error));
}
