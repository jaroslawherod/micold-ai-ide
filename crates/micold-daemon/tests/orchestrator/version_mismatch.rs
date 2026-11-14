//! User Story 3 (#1724, FR-006, SC-003): a daemon on another protocol version is refused naming
//! both versions and listed `needs-update`; a live record is never overwritten; a relink on the
//! current version turns it `online`.

use micold_core::orchestrator::{LinkVerdict, RefuseReason};
use micold_core::protocol::messages::{DaemonMsg, RefusalReason};
use micold_core::protocol::version::{PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH};

use crate::support::{announcement, FakeDaemon, Orchestrator};

async fn record_of(orchestrator: &Orchestrator, id: &str) -> Option<serde_json::Value> {
    orchestrator
        .daemons()
        .await
        .into_iter()
        .find(|daemon| daemon["id"] == id)
}

async fn health_of(orchestrator: &Orchestrator, id: &str) -> Option<String> {
    record_of(orchestrator, id)
        .await
        .map(|record| record["health"].as_str().unwrap().to_owned())
}

#[tokio::test]
async fn version_mismatch_an_older_daemon_is_refused_naming_both_versions_and_listed_needs_update()
{
    let orchestrator = Orchestrator::start().await;
    let _current = FakeDaemon::link(&orchestrator, &announcement("current")).await;

    let mut older = announcement("older");
    older.protocol_version = PROTOCOL_VERSION - 1;
    let verdict = FakeDaemon::dial(&orchestrator).await.announce(&older).await;
    let LinkVerdict::Refused {
        reason,
        orchestrator_version,
        daemon_version,
        message,
    } = verdict
    else {
        panic!("not refused: {verdict:?}")
    };
    assert_eq!(reason, RefuseReason::VersionMismatch);
    assert_eq!(orchestrator_version, PROTOCOL_VERSION);
    assert_eq!(daemon_version, PROTOCOL_VERSION - 1);
    assert!(
        message.contains(&PROTOCOL_VERSION.to_string())
            && message.contains(&(PROTOCOL_VERSION - 1).to_string()),
        "{message}"
    );

    let record = record_of(&orchestrator, "older").await.expect("listed");
    assert_eq!(record["health"], "needs-update");
    assert_eq!(record["protocol_version"], PROTOCOL_VERSION - 1);
    assert_eq!(record["orchestrator_version"], PROTOCOL_VERSION);
    assert_eq!(health_of(&orchestrator, "current").await.unwrap(), "online");
}

#[tokio::test]
async fn version_mismatch_a_newer_daemon_is_refused_the_same_way() {
    let orchestrator = Orchestrator::start().await;
    let mut newer = announcement("newer");
    newer.protocol_version = PROTOCOL_VERSION + 1;
    let verdict = FakeDaemon::dial(&orchestrator).await.announce(&newer).await;
    assert!(matches!(
        verdict,
        LinkVerdict::Refused {
            reason: RefuseReason::VersionMismatch,
            ..
        }
    ));
    assert_eq!(
        health_of(&orchestrator, "newer").await.unwrap(),
        "needs-update"
    );
}

#[tokio::test]
async fn version_mismatch_an_old_format_preamble_is_still_refused_and_listed() {
    let orchestrator = Orchestrator::start().await;
    let body = format!(
        r#"{{"format":1,"id":"box","kind":"container","name":"Box","protocol_version":{},
        "capabilities":["b","a"],"tenant":"acme","retired_field":true}}"#,
        PROTOCOL_VERSION - 1
    );
    let verdict = FakeDaemon::dial(&orchestrator)
        .await
        .announce_raw(body.into_bytes())
        .await;
    assert!(matches!(verdict, LinkVerdict::Refused { .. }));
    let record = record_of(&orchestrator, "box").await.expect("listed");
    assert_eq!(record["health"], "needs-update");
    assert_eq!(record["tenant"], "acme");
}

#[tokio::test]
async fn version_mismatch_a_live_daemon_is_never_overwritten() {
    let orchestrator = Orchestrator::start().await;
    let _live = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    let before = record_of(&orchestrator, "a").await.unwrap();

    let mut older = announcement("a");
    older.protocol_version = PROTOCOL_VERSION - 1;
    let verdict = FakeDaemon::dial(&orchestrator).await.announce(&older).await;
    assert!(matches!(
        verdict,
        LinkVerdict::Refused {
            reason: RefuseReason::VersionMismatch,
            ..
        }
    ));
    assert_eq!(record_of(&orchestrator, "a").await.unwrap(), before);
}

#[tokio::test]
async fn version_mismatch_a_current_version_relink_turns_needs_update_online() {
    let orchestrator = Orchestrator::start().await;
    let mut older = announcement("a");
    older.protocol_version = PROTOCOL_VERSION - 1;
    FakeDaemon::dial(&orchestrator).await.announce(&older).await;
    assert_eq!(health_of(&orchestrator, "a").await.unwrap(), "needs-update");

    let _relinked = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    let record = record_of(&orchestrator, "a").await.unwrap();
    assert_eq!(record["health"], "online");
    assert_eq!(record["protocol_version"], PROTOCOL_VERSION);
    assert!(record.get("orchestrator_version").is_none());
}

#[tokio::test]
async fn version_mismatch_a_different_schema_hash_is_refused_at_hello_and_listed() {
    let orchestrator = Orchestrator::start().await;
    let mut linked = FakeDaemon::dial(&orchestrator)
        .await
        .accepted(&announcement("a"))
        .await;
    let mut other_hash = SCHEMA_HASH;
    other_hash[0] ^= 1;
    linked.hello_with(other_hash, PACKAGE_VERSION).await;
    assert!(matches!(
        linked.next().await,
        DaemonMsg::Refused {
            reason: RefusalReason::VersionMismatch { .. }
        }
    ));
    assert_eq!(health_of(&orchestrator, "a").await.unwrap(), "needs-update");

    let _relinked = FakeDaemon::link(&orchestrator, &announcement("a")).await;
    assert_eq!(health_of(&orchestrator, "a").await.unwrap(), "online");
}

#[tokio::test]
async fn version_mismatch_a_build_mismatch_at_hello_leaves_the_registry_unchanged() {
    let orchestrator = Orchestrator::start().await;
    let mut linked = FakeDaemon::dial(&orchestrator)
        .await
        .accepted(&announcement("a"))
        .await;
    linked.hello_with(SCHEMA_HASH, "0.0.0-other-build").await;
    assert!(matches!(
        linked.next().await,
        DaemonMsg::Refused {
            reason: RefusalReason::BuildMismatch { .. }
        }
    ));
    assert!(orchestrator.daemons().await.is_empty());

    // The reservation was released: the id can link again.
    let _again = FakeDaemon::link(&orchestrator, &announcement("a")).await;
}
