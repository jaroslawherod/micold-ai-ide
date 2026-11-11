//! The registry: a pure state machine (data-model.md). Every method takes the time it needs, so it
//! is tested with no socket and no clock. Shared behind one `Mutex` by the daemon crate, which
//! never holds the lock across an `.await`.

use std::collections::BTreeMap;

use super::announce::{LinkAnnouncement, RefuseReason};
use super::record::{DaemonRecord, Health};
use crate::protocol::routing::WireDaemonId;

/// One accepted link, in memory only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LinkToken(u64);

/// The registry as it stands after a change, stamped so a saver can skip an older one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    /// Strictly increasing across mutating calls.
    pub seq: u64,
    /// Every record, sorted by id.
    pub records: Vec<DaemonRecord>,
}

/// The answer to [`Registry::claim`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Claim {
    /// The announcement is accepted; `Hello` is pending. No record changed.
    Reserved {
        /// The reservation.
        link: LinkToken,
        /// The registry after the call.
        snapshot: Snapshot,
    },
    /// Refused.
    Refused(RefuseReason),
}

/// A confirmed link.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirmed {
    /// The registry after the call.
    pub snapshot: Snapshot,
}

/// Why [`Registry::confirm`] changed nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmRefused {
    /// The token is replaced, released or unknown.
    Stale,
    /// Another link for the id is live.
    AlreadyLinked,
}

/// What a call about one link found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkAction {
    /// The token is replaced, released or unknown: nothing changed.
    Stale,
    /// The call applied.
    Idle,
}

/// The daemon registry.
#[derive(Debug, Default)]
pub struct Registry {
    records: BTreeMap<WireDaemonId, DaemonRecord>,
    /// Per id: the link and, until `Hello` is confirmed, what it announced.
    links: BTreeMap<WireDaemonId, (LinkToken, Option<LinkAnnouncement>)>,
    next_token: u64,
    seq: u64,
}

impl Registry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Take an announcement: reserve a link for its id, or refuse. No record changes. A reserved
    /// or live link refuses a rival (`AlreadyLinked`).
    pub fn claim(&mut self, announcement: &LinkAnnouncement) -> Claim {
        if self.links.contains_key(&announcement.id) {
            return Claim::Refused(RefuseReason::AlreadyLinked);
        }
        self.next_token += 1;
        let link = LinkToken(self.next_token);
        self.links
            .insert(announcement.id.clone(), (link, Some(announcement.clone())));
        Claim::Reserved {
            link,
            snapshot: self.snapshot(),
        }
    }

    /// A good `Hello` arrived on `link`: upsert the record `online`, last seen `wall_secs`.
    pub fn confirm(
        &mut self,
        link: LinkToken,
        wall_secs: u64,
    ) -> Result<Confirmed, ConfirmRefused> {
        let Some((id, (_, pending))) = self
            .links
            .iter_mut()
            .find(|(_, (token, pending))| *token == link && pending.is_some())
        else {
            return Err(ConfirmRefused::Stale);
        };
        let id = id.clone();
        let announced = pending.take().expect("filtered on a pending reservation");
        let mut record = DaemonRecord::new(
            announced.id,
            announced.kind,
            &announced.name,
            announced.protocol_version,
            announced.capabilities,
            announced.tenant.as_deref(),
        );
        record.health = Health::Online;
        record.last_seen = wall_secs;
        self.records.insert(id, record);
        Ok(Confirmed {
            snapshot: self.snapshot(),
        })
    }

    /// The transport of `link` ended: its reservation or live slot is released. The record keeps
    /// its health.
    pub fn closed(&mut self, link: LinkToken) -> (LinkAction, Option<Snapshot>) {
        let held = self
            .links
            .iter()
            .find(|(_, (token, _))| *token == link)
            .map(|(id, _)| id.clone());
        let Some(id) = held else {
            return (LinkAction::Stale, None);
        };
        self.links.remove(&id);
        (LinkAction::Idle, Some(self.snapshot()))
    }

    /// Every record, sorted by id.
    pub fn list(&self) -> Vec<DaemonRecord> {
        self.records.values().cloned().collect()
    }

    fn snapshot(&mut self) -> Snapshot {
        self.seq += 1;
        Snapshot {
            seq: self.seq,
            records: self.list(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::daemon_config::Kind;
    use crate::orchestrator::record::Health;

    fn ann(id: &str, name: &str) -> LinkAnnouncement {
        LinkAnnouncement {
            format: 1,
            id: WireDaemonId::new(id).unwrap(),
            kind: Kind::Host,
            name: name.to_owned(),
            protocol_version: 81,
            capabilities: BTreeSet::new(),
            tenant: None,
        }
    }

    fn reserve(registry: &mut Registry, a: &LinkAnnouncement) -> (LinkToken, Snapshot) {
        match registry.claim(a) {
            Claim::Reserved { link, snapshot } => (link, snapshot),
            other => panic!("not reserved: {other:?}"),
        }
    }

    #[test]
    fn claim_on_an_unknown_id_reserves_and_changes_no_record() {
        let mut registry = Registry::new();
        let (_, snapshot) = reserve(&mut registry, &ann("a", "A"));
        assert!(registry.list().is_empty());
        assert!(snapshot.records.is_empty());
    }

    #[test]
    fn confirm_upserts_the_record_online_with_the_wall_time() {
        let mut registry = Registry::new();
        let (link, _) = reserve(&mut registry, &ann("a", " A "));
        let confirmed = registry.confirm(link, 1234).unwrap();
        let [record] = &registry.list()[..] else {
            panic!()
        };
        assert_eq!(record.health, Health::Online);
        assert_eq!(record.last_seen, 1234);
        assert_eq!(record.name, "A");
        assert_eq!(confirmed.snapshot.records, registry.list());
    }

    #[test]
    fn a_relink_updates_the_record_in_place() {
        let mut registry = Registry::new();
        let (first, _) = reserve(&mut registry, &ann("a", "old"));
        registry.confirm(first, 1).unwrap();
        registry.closed(first);
        let mut again = ann("a", "new");
        again.capabilities.insert("cap".into());
        let (second, _) = reserve(&mut registry, &again);
        registry.confirm(second, 2).unwrap();
        let [record] = &registry.list()[..] else {
            panic!("duplicated")
        };
        assert_eq!(record.name, "new");
        assert!(record.capabilities.contains("cap"));
        assert_eq!(record.last_seen, 2);
    }

    #[test]
    fn list_is_sorted_by_id() {
        let mut registry = Registry::new();
        for id in ["zed", "alpha", "mid"] {
            let (link, _) = reserve(&mut registry, &ann(id, id));
            registry.confirm(link, 1).unwrap();
        }
        let ids: Vec<_> = registry.list().iter().map(|r| r.id.to_string()).collect();
        assert_eq!(ids, ["alpha", "mid", "zed"]);
        assert!(Registry::new().list().is_empty());
    }

    #[test]
    fn every_mutating_call_returns_a_strictly_newer_snapshot() {
        let mut registry = Registry::new();
        let (link, claimed) = reserve(&mut registry, &ann("a", "A"));
        let confirmed = registry.confirm(link, 1).unwrap().snapshot;
        let (_, closed) = registry.closed(link);
        let closed = closed.unwrap();
        assert!(claimed.seq < confirmed.seq && confirmed.seq < closed.seq);
    }

    #[test]
    fn confirm_with_an_unknown_token_is_stale_and_changes_nothing() {
        let mut registry = Registry::new();
        let (link, _) = reserve(&mut registry, &ann("a", "A"));
        registry.closed(link);
        assert_eq!(
            registry.confirm(link, 9).unwrap_err(),
            ConfirmRefused::Stale
        );
        assert!(registry.list().is_empty());
    }

    #[test]
    fn a_rival_claim_on_a_reserved_or_live_id_is_already_linked() {
        let mut registry = Registry::new();
        let (link, _) = reserve(&mut registry, &ann("a", "A"));
        assert_eq!(
            registry.claim(&ann("a", "B")),
            Claim::Refused(RefuseReason::AlreadyLinked)
        );
        registry.confirm(link, 1).unwrap();
        assert_eq!(
            registry.claim(&ann("a", "B")),
            Claim::Refused(RefuseReason::AlreadyLinked)
        );
    }

    #[test]
    fn closing_a_link_frees_the_id_and_keeps_the_record() {
        let mut registry = Registry::new();
        let (link, _) = reserve(&mut registry, &ann("a", "A"));
        registry.confirm(link, 1).unwrap();
        assert_eq!(registry.closed(link).0, LinkAction::Idle);
        assert_eq!(registry.closed(link).0, LinkAction::Stale);
        assert_eq!(registry.list().len(), 1);
        assert!(matches!(
            registry.claim(&ann("a", "A")),
            Claim::Reserved { .. }
        ));
    }
}
