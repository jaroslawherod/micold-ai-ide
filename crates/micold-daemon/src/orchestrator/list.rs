//! The orchestrator's own list (#1726 FR-001, FR-002, research R4): every linked daemon asked in
//! parallel, each under [`LIST_DEADLINE`], and the answers merged by the pure `merge_lists`. A
//! daemon the registry calls offline or out of date is named without asking.
//!
//! Each online daemon is asked the way any client asks it: the orchestrator opens a conversation
//! of its own on the daemon's link, says `Hello`, and reads the catalogue the `Welcome` carries.
//! The conversation ends as soon as the answer (or the deadline) is in, so the rows are always the
//! daemon's current truth and nothing is cached.

use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures_util::future::join_all;
use micold_core::orchestrator::list_merge::{merge_lists, DaemonAnswer, Merged};
use micold_core::orchestrator::route::{ClientId, ClientKey};
use micold_core::orchestrator::Health;
use micold_core::protocol::messages::{
    CatalogSnapshot, ClientInstance, ClientMsg, DaemonMsg, ListRows,
};
use micold_core::protocol::routing::{
    control_body, control_payload, decode_routed_payload, encode_opaque_payload, ConnectionId,
    ListKind, MissingReason, RoutedBody, RoutedHeader, Sequence, WireDaemonId,
};
use micold_core::protocol::version::{
    BUILD_FINGERPRINT, PACKAGE_VERSION, PROTOCOL_VERSION, SCHEMA_HASH,
};
use tokio::sync::mpsc;

use super::hub::{Opened, WRITER_QUEUE};
use super::Shared;

/// How long one daemon has to answer a list (FR-002: the whole list answers within 2.5 s).
pub const LIST_DEADLINE: Duration = Duration::from_secs(2);

/// The merged `kind` list of every daemon the registry knows.
pub async fn gather(shared: &Arc<Shared>, kind: ListKind) -> Merged {
    let records = shared.registry().list();
    let asks = records.into_iter().map(|record| {
        let shared = Arc::clone(shared);
        async move {
            let answer = match record.health {
                Health::Offline => Err(MissingReason::Offline),
                Health::NeedsUpdate => Err(MissingReason::NeedsUpdate),
                Health::Online => {
                    tokio::time::timeout(LIST_DEADLINE, ask(&shared, &record.id, kind))
                        .await
                        .unwrap_or(Err(MissingReason::Timeout))
                }
            };
            (record.id, answer) as DaemonAnswer
        }
    });
    merge_lists(kind, join_all(asks).await)
}

/// Ends the orchestrator's conversation with a daemon however [`ask`] returns.
struct Asker<'a>(&'a Shared, ClientId);

impl Drop for Asker<'_> {
    fn drop(&mut self) {
        self.0.hub().client_gone(self.1);
    }
}

/// The orchestrator's `Hello` to a daemon it lists (it holds no token).
fn hello() -> ClientMsg {
    ClientMsg::Hello {
        protocol_version: PROTOCOL_VERSION,
        schema_hash: SCHEMA_HASH,
        client_build: "orchestrator-list".to_owned(),
        client_instance: ClientInstance::current(),
        client_package_version: PACKAGE_VERSION.to_owned(),
        auth_token: None,
        client_fingerprint: BUILD_FINGERPRINT.to_owned(),
        require_fingerprint_match: false,
    }
}

/// Ask `daemon` for its `kind` rows: `Hello` on a conversation of the orchestrator's own, rows
/// from the `Welcome`.
async fn ask(
    shared: &Shared,
    daemon: &WireDaemonId,
    kind: ListKind,
) -> Result<ListRows, MissingReason> {
    let (writer, mut answers) = mpsc::channel::<Bytes>(WRITER_QUEUE);
    let asker = Asker(shared, shared.hub().register_client(writer));
    let key = ClientKey {
        client: asker.1,
        daemon: daemon.clone(),
        conn: ConnectionId(0),
    };
    let link_conn = match shared.hub().open_route(&key, false) {
        Some(Opened::Route { link_conn }) => link_conn,
        _ => return Err(MissingReason::Offline),
    };
    let hello = control_payload(&hello()).map_err(|e| MissingReason::Error(e.to_string()))?;
    let header = RoutedHeader {
        target: None,
        conn: ConnectionId(link_conn),
        seq: Sequence(1),
        req: None,
    };
    shared.hub().to_daemon(
        &key,
        link_conn,
        Bytes::from(encode_opaque_payload(&header, &hello)),
    );
    while let Some(payload) = answers.recv().await {
        let Ok(routed) = decode_routed_payload(&payload) else {
            continue;
        };
        let RoutedBody::Opaque(inner) = routed.body else {
            continue;
        };
        let Some(Ok(msg)) = control_body(inner).map(serde_json::from_slice::<DaemonMsg>) else {
            continue;
        };
        match msg {
            DaemonMsg::Welcome { catalog, .. } => return Ok(rows_of(kind, catalog)),
            DaemonMsg::Refused { reason } => {
                return Err(MissingReason::Error(format!("refused: {reason:?}")))
            }
            DaemonMsg::StreamEnded { reason } => {
                return Err(MissingReason::Error(format!("ended: {reason:?}")))
            }
            _ => {}
        }
    }
    Err(MissingReason::Offline)
}

/// The `kind` rows of a daemon's catalogue.
fn rows_of(kind: ListKind, catalog: CatalogSnapshot) -> ListRows {
    let projects = catalog.projects;
    match kind {
        ListKind::Projects => ListRows::Projects(projects),
        ListKind::Worktrees => {
            ListRows::Worktrees(projects.into_iter().flat_map(|p| p.worktrees).collect())
        }
        ListKind::Sessions => {
            ListRows::Sessions(projects.into_iter().flat_map(|p| p.sessions).collect())
        }
    }
}
