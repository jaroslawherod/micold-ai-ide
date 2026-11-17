//! #1732 FR-001, FR-002, FR-006, FR-008: requests answered with their id from the real linked
//! daemons; anything unknown or malformed gets a typed error and the connection stays open; a
//! binary frame is refused and none is ever sent.

use micold_daemon::catalog::Catalog;
use serde_json::json;

use crate::ws_support::Api;

#[tokio::test]
async fn list_daemons_names_every_linked_daemon() {
    let api = Api::start().await;
    let _d = api.daemon("d1", Catalog::ephemeral()).await;
    let mut ws = api.client().await;
    let reply = ws.ask(json!({"id":1,"type":"list_daemons"})).await;
    assert_eq!(reply["type"], "daemons");
    assert_eq!(
        reply["rows"],
        json!([{"id":"d1","name":"Daemon d1","status":"online"}])
    );
}

#[tokio::test]
async fn list_sessions_holds_every_daemons_sessions() {
    let api = Api::start().await;
    let project = tempfile::tempdir().unwrap();
    let session = crate::history::ai_session();
    let id = session.id;
    let _d = api
        .daemon("d1", crate::history::catalog(project.path(), vec![session]))
        .await;
    let mut ws = api.client().await;
    let reply = ws.ask(json!({"id":2,"type":"list_sessions"})).await;
    assert_eq!(reply["type"], "sessions");
    let rows = reply["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "{reply}");
    assert_eq!(rows[0]["id"], id.to_string());
    assert_eq!(rows[0]["state"], "stopped");
    assert_eq!(rows[0]["kind"], "terminal");
    assert_eq!(rows[0]["reachable"], true);
}

#[tokio::test]
async fn an_unknown_type_is_a_typed_error_and_the_connection_stays() {
    let api = Api::start().await;
    let mut ws = api.client().await;
    let reply = ws.ask(json!({"id":3,"type":"foo"})).await;
    assert_eq!(reply["type"], "error");
    assert_eq!(reply["code"], "unknown_type");
    assert!(reply["message"].as_str().unwrap().contains("foo"));
    assert_eq!(ws.ask(json!({"id":4,"type":"ping"})).await["type"], "pong");
}

#[tokio::test]
async fn malformed_json_is_a_typed_error_and_the_connection_stays() {
    let api = Api::start().await;
    let mut ws = api.client().await;
    ws.send_text("{not json".into()).await;
    let reply = ws.next().await;
    assert_eq!(
        (reply["type"].as_str(), reply["code"].as_str()),
        (Some("error"), Some("malformed"))
    );
    assert_eq!(ws.ask(json!({"id":5,"type":"ping"})).await["type"], "pong");
}

#[tokio::test]
async fn a_binary_frame_is_malformed() {
    let api = Api::start().await;
    let mut ws = api.client().await;
    ws.send_binary(vec![1, 2, 3]).await;
    let reply = ws.next().await;
    assert_eq!(reply["code"], "malformed");
    assert_eq!(ws.ask(json!({"id":6,"type":"ping"})).await["id"], 6);
}
