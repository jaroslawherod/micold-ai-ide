//! #1732 US2 scenarios 1-2, FR-003, SC-002, SC-006: subscribe returns a snapshot, changes follow
//! in order, and a client that reconnects with `since` gets exactly the changes it missed, or
//! `resync_needed` when they are not held.

use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_daemon::catalog::Catalog;
use micold_daemon::orchestrator::directory::LinkDirectory;
use serde_json::{json, Value};

use crate::ws_support::{fast, serve, Api, Scripted, Ws, ALLOWED};

fn subscribe(id: u64, since: Option<(u64, u64)>) -> Value {
    match since {
        None => json!({"id":id,"type":"subscribe"}),
        Some((epoch, seq)) => json!({"id":id,"type":"subscribe","since":{"epoch":epoch,"seq":seq}}),
    }
}

/// A scripted feed and a subscribed client: the directory, the address, the client, the epoch
/// and the subscribed seq.
async fn subscribed() -> (Arc<Scripted>, std::net::SocketAddr, Ws, u64, u64) {
    let directory = Arc::new(Scripted::default());
    let addr = serve(directory.clone(), fast()).await;
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    let reply = ws.ask(subscribe(1, None)).await;
    assert_eq!(reply["type"], "subscribed", "{reply}");
    let (epoch, seq) = (
        reply["epoch"].as_u64().unwrap(),
        reply["seq"].as_u64().unwrap(),
    );
    (directory, addr, ws, epoch, seq)
}

/// The daemon id each change upserts, and its seq.
fn upserted(changes: &[Value]) -> Vec<(u64, String)> {
    changes
        .iter()
        .map(|c| {
            assert_eq!(c["kind"], "upsert", "{c}");
            (
                c["seq"].as_u64().unwrap(),
                c["entity"]["value"]["id"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// Read changes until `n` arrived.
async fn read_n(ws: &mut Ws, n: usize) -> Vec<(u64, String)> {
    let mut out = Vec::new();
    while out.len() < n {
        out.extend(upserted(&ws.changes().await));
    }
    out
}

#[tokio::test]
async fn subscribe_without_since_returns_a_snapshot() {
    let api = Api::start().await;
    let _d = api.daemon("d1", Catalog::ephemeral()).await;
    let mut ws = api.client().await;
    let reply = ws.ask(subscribe(1, None)).await;
    assert_eq!(reply["type"], "subscribed");
    assert!(reply["epoch"].as_u64().unwrap() > 0);
    assert!(
        reply["epoch"].as_u64().unwrap() < (1 << 53),
        "a JavaScript number holds it"
    );
    assert_eq!(reply["snapshot"]["daemons"][0]["id"], "d1", "{reply}");
    assert_eq!(reply["snapshot"]["seq"], reply["seq"]);
}

#[tokio::test]
async fn changes_arrive_in_order_with_strictly_increasing_seq() {
    let (directory, _addr, mut ws, _epoch, seq) = subscribed().await;
    for n in 0..5 {
        directory.add_daemon(&format!("d{n}"));
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
    let got = read_n(&mut ws, 5).await;
    let seqs: Vec<u64> = got.iter().map(|(s, _)| *s).collect();
    assert_eq!(seqs, (seq + 1..=seq + 5).collect::<Vec<_>>());
    let ids: Vec<&str> = got.iter().map(|(_, id)| id.as_str()).collect();
    assert_eq!(ids, ["d0", "d1", "d2", "d3", "d4"]);
}

#[tokio::test]
async fn a_reconnect_with_since_gets_exactly_the_missed_changes() {
    let (directory, addr, mut ws, epoch, _) = subscribed().await;
    directory.add_daemon("seen");
    let seen = read_n(&mut ws, 1).await;
    let last = seen[0].0;
    drop(ws);
    for n in 0..3 {
        directory.add_daemon(&format!("missed{n}"));
    }
    let mut again = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    let reply = again.ask(subscribe(2, Some((epoch, last)))).await;
    assert_eq!(reply["type"], "subscribed", "{reply}");
    assert!(reply.get("snapshot").is_none());
    let got = read_n(&mut again, 3).await;
    assert_eq!(
        got.iter().map(|(_, id)| id.as_str()).collect::<Vec<_>>(),
        ["missed0", "missed1", "missed2"]
    );
    assert_eq!(got[0].0, last + 1);
}

#[tokio::test]
async fn another_epoch_or_a_position_ahead_needs_a_resync() {
    let (_directory, addr, _ws, epoch, seq) = subscribed().await;
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    let stale = ws.ask(subscribe(2, Some((epoch + 1, seq)))).await;
    assert_eq!(
        (stale["type"].as_str(), stale["reason"].as_str()),
        (Some("resync_needed"), Some("epoch_changed"))
    );
    let ahead = ws.ask(subscribe(3, Some((epoch, seq + 50)))).await;
    assert_eq!(ahead["reason"], "ahead");
    // After a resync the client subscribes afresh.
    assert!(ws.ask(subscribe(4, None)).await["snapshot"].is_object());
}

#[tokio::test]
async fn a_position_older_than_the_log_needs_a_resync() {
    let (directory, addr, mut ws, epoch, _) = subscribed().await;
    for n in 0..1100 {
        directory.add_daemon(&format!("d{n:04}"));
    }
    read_n(&mut ws, 1100).await;
    let mut late = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    let reply = late.ask(subscribe(2, Some((epoch, 1)))).await;
    assert_eq!(
        (reply["type"].as_str(), reply["reason"].as_str()),
        (Some("resync_needed"), Some("too_old"))
    );
}

#[tokio::test]
async fn a_hundred_changes_over_ten_reconnects_arrive_exactly_once_in_order() {
    let (directory, addr, ws, epoch, start) = subscribed().await;
    drop(ws);
    let mut cursor = start;
    let mut got = Vec::new();
    let mut made = 0;
    for round in 0..10 {
        let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
        let reply = ws.ask(subscribe(round, Some((epoch, cursor)))).await;
        assert_eq!(reply["type"], "subscribed", "{reply}");
        // Five made while connected; five more, made while it was cut, arrive with them.
        for _ in 0..5 {
            directory.add_daemon(&format!("c{made:03}"));
            made += 1;
        }
        let batch = read_n(&mut ws, if round == 0 { 5 } else { 10 }).await;
        cursor = batch.last().unwrap().0;
        got.extend(batch);
        drop(ws);
        for _ in 0..5 {
            if made < 100 {
                directory.add_daemon(&format!("c{made:03}"));
                made += 1;
            }
        }
        // Let the poller, now without a subscriber, see them or stop: both must stay exact.
        tokio::time::sleep(Duration::from_millis(60)).await;
    }
    assert_eq!(made, 100);
    let seqs: Vec<u64> = got.iter().map(|(s, _)| *s).collect();
    assert_eq!(
        seqs,
        (start + 1..=start + 95).collect::<Vec<_>>(),
        "no gap, no duplicate"
    );
    // The last five were made after the last read: one more resume gets exactly them.
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    ws.ask(subscribe(99, Some((epoch, cursor)))).await;
    got.extend(read_n(&mut ws, 5).await);
    let ids: Vec<String> = got.into_iter().map(|(_, id)| id).collect();
    assert_eq!(
        ids,
        (0..100).map(|n| format!("c{n:03}")).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_daemon_linking_appears_within_two_seconds() {
    let api = Api::start_with(Default::default()).await;
    let mut ws = api.client().await;
    let reply = ws.ask(subscribe(1, None)).await;
    assert_eq!(reply["snapshot"]["daemons"], json!([]));
    // The link's registration pokes the feed: no wait for the 1 s tick.
    let linked = Instant::now();
    let _d = api.daemon("late", Catalog::ephemeral()).await;
    let changes = ws.changes().await;
    assert!(
        linked.elapsed() < Duration::from_secs(2),
        "{:?}",
        linked.elapsed()
    );
    assert!(
        changes.iter().any(|c| c["entity"]["value"]["id"] == "late"),
        "{changes:?}"
    );
}

/// [`Scripted`], with a read that takes 600 ms, so the connection stops reading the feed.
struct SlowRead(Arc<Scripted>);

impl LinkDirectory for SlowRead {
    fn list(
        &self,
    ) -> futures_util::future::BoxFuture<'_, micold_core::client_api::messages::Snapshot> {
        self.0.list()
    }

    fn read_session<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: &'a str,
    ) -> futures_util::future::BoxFuture<
        'a,
        Result<
            Vec<micold_core::client_api::messages::ContentEntry>,
            micold_daemon::orchestrator::directory::ApiError,
        >,
    > {
        use futures_util::FutureExt;
        async {
            tokio::time::sleep(Duration::from_millis(600)).await;
            Ok(Vec::new())
        }
        .boxed()
    }

    fn changed(&self) -> futures_util::future::BoxFuture<'_, ()> {
        self.0.changed()
    }
}

#[tokio::test]
async fn a_subscriber_that_falls_behind_reads_the_missed_changes_from_the_log() {
    let scripted = Arc::new(Scripted::default());
    let options = micold_daemon::orchestrator::ws::ApiOptions {
        backlog: 1,
        ..fast()
    };
    let addr = serve(Arc::new(SlowRead(scripted.clone())), options).await;
    let mut ws = Ws::connect(addr, Some(ALLOWED)).await.unwrap();
    let seq = ws.ask(subscribe(1, None)).await["seq"].as_u64().unwrap();
    ws.send(json!({"id":2,"type":"read_session","daemon":"d","project":"p","session":"s"}))
        .await;
    for n in 0..4 {
        scripted.add_daemon(&format!("d{n}"));
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
    let got = read_n(&mut ws, 4).await;
    let expected: Vec<(u64, String)> = (0..4).map(|n| (seq + 1 + n, format!("d{n}"))).collect();
    assert_eq!(got, expected);
}
