//! What the orchestrator answers that is not a link or a client session: the listing. Everything a
//! session needs is routed to its daemon by [`super::client`] (#1726); the orchestrator hosts none.

use micold_core::orchestrator::{Health, LinkQuery};
use micold_core::protocol::version::PROTOCOL_VERSION;

use super::Shared;

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
