//! #1732 FR-001, FR-002, FR-006, FR-008, FR-014a: requests answered with their id from the real linked
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

/// Poll `list_sessions` until its first row's `reachable` is `want`; that reply.
async fn sessions_when_reachable(ws: &mut crate::ws_support::Ws, want: bool) -> serde_json::Value {
    crate::support::within("the session's reachable flag flips", async {
        loop {
            let reply = ws.ask(json!({"id":21,"type":"list_sessions"})).await;
            if reply["rows"][0]["reachable"] == want {
                return reply;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
}

/// FR-014a: a daemon whose link drops keeps its sessions and locations in the list, sessions
/// marked unreachable; once it re-links they are reachable again.
#[tokio::test]
async fn a_dropped_daemons_sessions_stay_listed_as_unreachable_until_it_relinks() {
    let api = Api::start().await;
    let project = tempfile::tempdir().unwrap();
    let session = crate::history::ai_session();
    let id = session.id.to_string();
    let d = api
        .daemon("d1", crate::history::catalog(project.path(), vec![session]))
        .await;
    let mut ws = api.client().await;
    let before = ws.ask(json!({"id":20,"type":"list_sessions"})).await;
    assert_eq!(before["rows"][0]["reachable"], true, "{before}");

    d.drop_link();
    let dropped = sessions_when_reachable(&mut ws, false).await;
    let rows = dropped["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "the row is kept, not removed: {dropped}");
    assert_eq!(rows[0]["id"], id);
    let snapshot = api
        .client()
        .await
        .ask(json!({"id":23,"type":"subscribe"}))
        .await;
    let location = &rows[0]["location_id"];
    assert!(
        snapshot["snapshot"]["locations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| &l["id"] == location),
        "the session's location is kept: {snapshot}"
    );
    // Not asserted: `list_daemons` still reads `online` here, the ledger's open follow-up
    // "daemon header Online while its sessions are Unreachable".

    let _relinked =
        crate::linked_daemon::Daemon::link_state(&api.orchestrator, "d1", d.state.clone());
    let back = sessions_when_reachable(&mut ws, true).await;
    assert_eq!(back["rows"][0]["id"], id, "{back}");
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
