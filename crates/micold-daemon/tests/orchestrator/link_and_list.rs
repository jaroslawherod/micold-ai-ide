//! User Story 1, scenarios 1 and 3 (SC-001): a linked daemon is listed with every field.

use micold_core::protocol::version::PROTOCOL_VERSION;

use crate::support::{announcement, FakeDaemon, Orchestrator};

#[tokio::test]
async fn an_empty_orchestrator_lists_nothing() {
    let orchestrator = Orchestrator::start().await;
    assert!(orchestrator.daemons().await.is_empty());
    assert_eq!(
        orchestrator.list().await["orchestrator_version"],
        PROTOCOL_VERSION
    );
}

#[tokio::test]
async fn a_linked_daemon_is_listed_online_with_every_field() {
    let orchestrator = Orchestrator::start().await;
    let mut announced = announcement("box-1");
    announced.tenant = Some("acme".into());
    let _link = FakeDaemon::link(&orchestrator, &announced).await;

    let daemons = orchestrator.daemons().await;
    assert_eq!(daemons.len(), 1);
    let record = &daemons[0];
    assert_eq!(record["id"], "box-1");
    assert_eq!(record["kind"], "container");
    assert_eq!(record["name"], "Daemon box-1");
    assert_eq!(record["protocol_version"], PROTOCOL_VERSION);
    assert_eq!(
        record["capabilities"],
        serde_json::json!(["git", "sessions"])
    );
    assert_eq!(record["health"], "online");
    assert_eq!(record["last_seen"], orchestrator.clock.wall());
    assert_eq!(record["tenant"], "acme");
}

#[tokio::test]
async fn a_daemon_that_announces_no_tenant_lists_the_default() {
    let orchestrator = Orchestrator::start().await;
    let _link = FakeDaemon::link(&orchestrator, &announcement("plain")).await;
    assert_eq!(orchestrator.daemons().await[0]["tenant"], "default");
}

#[tokio::test]
async fn the_listing_is_sorted_by_id() {
    let orchestrator = Orchestrator::start().await;
    let _b = FakeDaemon::link(&orchestrator, &announcement("b")).await;
    let _a = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    let ids: Vec<_> = orchestrator
        .daemons()
        .await
        .iter()
        .map(|d| d["id"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(ids, ["a", "b"]);
}
