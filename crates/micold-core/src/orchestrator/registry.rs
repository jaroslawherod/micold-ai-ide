//! The registry: a pure state machine (data-model.md). Every method takes the time it needs, so it
//! is tested with no socket and no clock. Shared behind one `Mutex` by the daemon crate, which
//! never holds the lock across an `.await`.

use std::collections::BTreeMap;
use std::time::Instant;

use super::announce::{LinkAnnouncement, RefuseReason};
use super::liveness::Heartbeat;
use super::record::{DaemonRecord, Health};
use crate::protocol::keepalive::KeepaliveAction;
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
    /// The call applied; nothing is due.
    Idle,
    /// Send the daemon a `LinkPing` now.
    SendPing,
    /// The daemon has been silent past the deadline: its record is `offline`.
    Expired,
}

/// The daemon registry.
#[derive(Debug, Default)]
pub struct Registry {
    records: BTreeMap<WireDaemonId, DaemonRecord>,
    /// Per id: the link and, until `Hello` is confirmed, what it announced.
    links: BTreeMap<WireDaemonId, (LinkToken, Option<LinkAnnouncement>)>,
    /// The heartbeat of each confirmed link.
    beats: BTreeMap<LinkToken, Heartbeat>,
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

    /// A good `Hello` arrived on `link` at `now`: upsert the record `online`, last seen
    /// `wall_secs`, and start the link's heartbeat.
    pub fn confirm(
        &mut self,
        link: LinkToken,
        now: Instant,
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
        self.beats.insert(link, Heartbeat::new(now));
        Ok(Confirmed {
            snapshot: self.snapshot(),
        })
    }

    /// A frame, of any kind, arrived on `link` at `now` (wall time `wall_secs`): the daemon is
    /// `online` and last seen then. Another daemon's record is never touched. A record that needs
    /// an update stays so: a reachable daemon of the wrong version is not usable.
    pub fn answered(
        &mut self,
        link: LinkToken,
        now: Instant,
        wall_secs: u64,
    ) -> (LinkAction, Option<Snapshot>) {
        let Some(beat) = self.beats.get_mut(&link) else {
            return (LinkAction::Stale, None);
        };
        beat.answered(now);
        let record = self.record_of(link);
        let changed = record.is_some_and(|record| {
            let before = (record.health, record.last_seen);
            if record.health == Health::Offline {
                record.health = Health::Online;
            }
            record.last_seen = wall_secs;
            before != (record.health, record.last_seen)
        });
        (LinkAction::Idle, changed.then(|| self.snapshot()))
    }

    /// The link loop's timer fired at `now`: what is due. `Expired` turns an `online` record
    /// `offline` (its last seen stays the last answer's time) and returns the new snapshot.
    pub fn tick(&mut self, link: LinkToken, now: Instant) -> (LinkAction, Option<Snapshot>) {
        let Some(beat) = self.beats.get_mut(&link) else {
            return (LinkAction::Stale, None);
        };
        match beat.tick(now) {
            KeepaliveAction::Idle => (LinkAction::Idle, None),
            KeepaliveAction::SendPing => (LinkAction::SendPing, None),
            KeepaliveAction::Expired => {
                let went_offline = self.record_of(link).is_some_and(|record| {
                    let online = record.health == Health::Online;
                    if online {
                        record.health = Health::Offline;
                    }
                    online
                });
                (LinkAction::Expired, went_offline.then(|| self.snapshot()))
            }
        }
    }

    fn record_of(&mut self, link: LinkToken) -> Option<&mut DaemonRecord> {
        let id = self
            .links
            .iter()
            .find(|(_, (token, pending))| *token == link && pending.is_none())
            .map(|(id, _)| id)?;
        self.records.get_mut(id)
    }

    /// The transport of `link` ended: its reservation or live slot is released. The record keeps
    /// its health; the link's own loop lets the deadline run out first.
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
        self.beats.remove(&link);
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
    use std::time::Duration;

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

    fn t0() -> Instant {
        static T0: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
        *T0.get_or_init(Instant::now)
    }

    fn at(secs: u64) -> Instant {
        t0() + Duration::from_secs(secs)
    }

    fn online(registry: &mut Registry, id: &str, wall: u64) -> LinkToken {
        let (link, _) = reserve(registry, &ann(id, id));
        registry.confirm(link, t0(), wall).unwrap();
        link
    }

    fn health(registry: &Registry, id: &str) -> (Health, u64) {
        let record = registry
            .list()
            .into_iter()
            .find(|record| record.id.to_string() == id)
            .unwrap();
        (record.health, record.last_seen)
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
        let confirmed = registry.confirm(link, t0(), 1234).unwrap();
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
        registry.confirm(first, t0(), 1).unwrap();
        registry.closed(first);
        let mut again = ann("a", "new");
        again.capabilities.insert("cap".into());
        let (second, _) = reserve(&mut registry, &again);
        registry.confirm(second, t0(), 2).unwrap();
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
            registry.confirm(link, t0(), 1).unwrap();
        }
        let ids: Vec<_> = registry.list().iter().map(|r| r.id.to_string()).collect();
        assert_eq!(ids, ["alpha", "mid", "zed"]);
        assert!(Registry::new().list().is_empty());
    }

    #[test]
    fn every_mutating_call_returns_a_strictly_newer_snapshot() {
        let mut registry = Registry::new();
        let (link, claimed) = reserve(&mut registry, &ann("a", "A"));
        let confirmed = registry.confirm(link, t0(), 1).unwrap().snapshot;
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
            registry.confirm(link, t0(), 9).unwrap_err(),
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
        registry.confirm(link, t0(), 1).unwrap();
        assert_eq!(
            registry.claim(&ann("a", "B")),
            Claim::Refused(RefuseReason::AlreadyLinked)
        );
    }

    #[test]
    fn closing_a_link_frees_the_id_and_keeps_the_record() {
        let mut registry = Registry::new();
        let (link, _) = reserve(&mut registry, &ann("a", "A"));
        registry.confirm(link, t0(), 1).unwrap();
        assert_eq!(registry.closed(link).0, LinkAction::Idle);
        assert_eq!(registry.closed(link).0, LinkAction::Stale);
        assert_eq!(registry.list().len(), 1);
        assert!(matches!(
            registry.claim(&ann("a", "A")),
            Claim::Reserved { .. }
        ));
    }

    #[test]
    fn a_silent_link_goes_offline_once_with_its_last_seen_unchanged() {
        let mut registry = Registry::new();
        let link = online(&mut registry, "a", 100);
        assert_eq!(registry.tick(link, at(1)), (LinkAction::Idle, None));
        assert_eq!(registry.tick(link, at(3)), (LinkAction::SendPing, None));
        let (action, snapshot) = registry.tick(link, at(9));
        assert_eq!(action, LinkAction::Expired);
        assert_eq!(snapshot.unwrap().records, registry.list());
        assert_eq!(health(&registry, "a"), (Health::Offline, 100));
        assert_eq!(registry.tick(link, at(10)), (LinkAction::Idle, None));
        assert_eq!(health(&registry, "a"), (Health::Offline, 100));
    }

    #[test]
    fn an_answer_brings_an_offline_daemon_back_and_advances_last_seen() {
        let mut registry = Registry::new();
        let link = online(&mut registry, "a", 100);
        registry.tick(link, at(9));
        let (action, snapshot) = registry.answered(link, at(12), 112);
        assert_eq!(action, LinkAction::Idle);
        assert!(snapshot.is_some());
        assert_eq!(health(&registry, "a"), (Health::Online, 112));
        assert_eq!(registry.tick(link, at(20)), (LinkAction::SendPing, None));
    }

    #[test]
    fn an_answer_on_an_online_daemon_only_moves_last_seen() {
        let mut registry = Registry::new();
        let link = online(&mut registry, "a", 100);
        registry.answered(link, at(2), 102);
        assert_eq!(health(&registry, "a"), (Health::Online, 102));
        assert_eq!(registry.answered(link, at(2), 102).1, None);
    }

    #[test]
    fn closing_keeps_health_and_a_token_never_issued_changes_nothing() {
        let mut registry = Registry::new();
        let link = online(&mut registry, "a", 100);
        registry.closed(link);
        assert_eq!(health(&registry, "a"), (Health::Online, 100));
        assert_eq!(registry.tick(link, at(30)), (LinkAction::Stale, None));
        assert_eq!(
            registry.answered(link, at(30), 130),
            (LinkAction::Stale, None)
        );
        let (reserved, _) = reserve(&mut registry, &ann("b", "B"));
        assert_eq!(
            registry.answered(reserved, at(1), 5),
            (LinkAction::Stale, None)
        );
        assert_eq!(registry.tick(reserved, at(30)), (LinkAction::Stale, None));
        assert_eq!(health(&registry, "a"), (Health::Online, 100));
    }

    #[test]
    fn one_daemon_going_silent_leaves_the_others_unchanged() {
        let mut registry = Registry::new();
        let a = online(&mut registry, "a", 100);
        let b = online(&mut registry, "b", 100);
        registry.answered(b, at(8), 108);
        registry.tick(a, at(9));
        assert_eq!(registry.tick(b, at(9)), (LinkAction::SendPing, None));
        assert_eq!(health(&registry, "a"), (Health::Offline, 100));
        assert_eq!(health(&registry, "b"), (Health::Online, 108));
    }
}
