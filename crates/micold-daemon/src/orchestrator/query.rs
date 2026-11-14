//! What the orchestrator answers that is not a link: the listing, and the refusal of everything a
//! session needs (it hosts none).

use micold_core::orchestrator::{Health, LinkQuery};
use micold_core::protocol::messages::{ClientMsg, DaemonMsg, ErrorKind};
use micold_core::protocol::version::PROTOCOL_VERSION;

use super::Shared;

/// The sentence a session request is refused with.
pub const NOT_HOSTED: &str = "not hosted by the orchestrator";

/// The listing frame body for `query`: `{ "daemons": [...], "orchestrator_version": N }`.
pub fn listing(shared: &Shared, query: LinkQuery) -> Vec<u8> {
    let LinkQuery::List = query;
    // A `needs-update` record carries both versions: its own `protocol_version` and this build's.
    let daemons: Vec<serde_json::Value> = shared
        .registry()
        .list()
        .into_iter()
        .map(|record| {
            let needs_update = record.health == Health::NeedsUpdate;
            let mut value = serde_json::to_value(record).expect("a record encodes");
            if needs_update {
                value["orchestrator_version"] = PROTOCOL_VERSION.into();
            }
            value
        })
        .collect();
    serde_json::to_vec(&serde_json::json!({
        "daemons": daemons,
        "orchestrator_version": PROTOCOL_VERSION,
    }))
    .expect("a listing encodes")
}

/// The answer to a post-`Hello` message on an accepted link, `None` when there is none to give.
///
/// A session start is refused with [`NOT_HOSTED`]; a project's session list is empty. No session is
/// ever spawned, so this holds no state.
pub fn not_hosted(msg: &ClientMsg) -> Option<DaemonMsg> {
    match msg {
        ClientMsg::SessionCreate { req, .. } => Some(DaemonMsg::OperationError {
            req: *req,
            kind: ErrorKind::Refused,
            message: NOT_HOSTED.to_owned(),
            detail: None,
        }),
        ClientMsg::Attach { project, .. } => Some(DaemonMsg::Attached {
            project: project.clone(),
            sessions: Vec::new(),
        }),
        _ => None,
    }
}
