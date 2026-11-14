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
use crate::protocol::version::PROTOCOL_VERSION;

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
    /// Refused; no record changed.
    Refused(RefuseReason),
    /// Refused for its protocol version, and the id is now listed `needs-update`.
    NeedsUpdate {
        /// The registry after the call.
        snapshot: Snapshot,
    },
}

/// Why a reserved link ended before it was confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimFailure {
    /// `Hello` named a protocol version or schema this build does not speak.
    VersionMismatch,
    /// `Hello` came from another build of the same contract.
    BuildMismatch,
    /// `Hello` failed authentication.
    AuthRejected,
    /// No `Hello` came, or the connection closed.
    Dropped,
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

/// Where an id's link is, derived from the registry's tables. An id with no entry is `Closed`: it
/// never linked, was restored, or its transport ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LinkState {
    /// The preamble was accepted and `Hello` is pending.
    Reserved,
    /// Confirmed and answering.
    Live,
    /// Confirmed, open, past its deadline: the record is `offline`.
    Silent,
    /// No link.
    Closed,
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

    /// Take an announcement: reserve a link for its id, or refuse (FR-006, FR-007). A reserved or
    /// live link refuses a rival `AlreadyLinked`, or `VersionMismatch` for a mismatched claim on a
    /// live id; neither changes a record. A mismatched claim on an id with no live link is refused
    /// and the id is recorded `needs-update`; a silent link on it is closed.
    pub fn claim(&mut self, announcement: &LinkAnnouncement) -> Claim {
        let mismatched = announcement.protocol_version != PROTOCOL_VERSION;
        match (self.link_state(&announcement.id), mismatched) {
            (LinkState::Reserved, _) | (LinkState::Live | LinkState::Silent, false) => {
                Claim::Refused(RefuseReason::AlreadyLinked)
            }
            (LinkState::Live, true) => Claim::Refused(RefuseReason::VersionMismatch),
            (LinkState::Silent | LinkState::Closed, true) => {
                if let Some((token, _)) = self.links.remove(&announcement.id) {
                    self.beats.remove(&token);
                }
                self.record_needs_update(announcement);
                Claim::NeedsUpdate {
                    snapshot: self.snapshot(),
                }
            }
            (LinkState::Closed, false) => {
                self.next_token += 1;
                let link = LinkToken(self.next_token);
                self.links
                    .insert(announcement.id.clone(), (link, Some(announcement.clone())));
                Claim::Reserved {
                    link,
                    snapshot: self.snapshot(),
                }
            }
        }
    }

    fn link_state(&self, id: &WireDaemonId) -> LinkState {
        match self.links.get(id) {
            None => LinkState::Closed,
            Some((_, Some(_))) => LinkState::Reserved,
            Some((token, None)) if self.beats.get(token).is_some_and(Heartbeat::is_silent) => {
                LinkState::Silent
            }
            Some(_) => LinkState::Live,
        }
    }

    /// List `announced` as `needs-update`, keeping the last seen of the record it replaces.
    fn record_needs_update(&mut self, announced: &LinkAnnouncement) {
        let mut record = DaemonRecord::new(
            announced.id.clone(),
            announced.kind,
            &announced.name,
            announced.protocol_version,
            announced.capabilities.clone(),
            announced.tenant.as_deref(),
        );
        record.health = Health::NeedsUpdate;
        record.last_seen = self
            .records
            .get(&announced.id)
            .map_or(0, |earlier| earlier.last_seen);
        self.records.insert(announced.id.clone(), record);
    }

    /// A reserved `link` ended without a confirmed `Hello`: the reservation is released. A
    /// `VersionMismatch` lists the id `needs-update`, as a mismatched claim does; the other
    /// reasons change no record. A token that is not a reservation is `Stale` and changes nothing,
    /// so a live link's record is never touched.
    pub fn claim_failed(
        &mut self,
        link: LinkToken,
        reason: ClaimFailure,
    ) -> (LinkAction, Option<Snapshot>) {
        let reserved = self
            .links
            .iter()
            .find(|(_, (token, pending))| *token == link && pending.is_some())
            .map(|(id, _)| id.clone());
        let Some(id) = reserved else {
            return (LinkAction::Stale, None);
        };
        let (_, pending) = self.links.remove(&id).expect("found above");
        let announced = pending.expect("filtered on a pending reservation");
        if reason != ClaimFailure::VersionMismatch {
            return (LinkAction::Idle, None);
        }
        self.record_needs_update(&announced);
        (LinkAction::Idle, Some(self.snapshot()))
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
    use crate::protocol::version::PROTOCOL_VERSION;

    fn ann(id: &str, name: &str) -> LinkAnnouncement {
        LinkAnnouncement {
            format: 1,
            id: WireDaemonId::new(id).unwrap(),
            kind: Kind::Host,
            name: name.to_owned(),
            protocol_version: PROTOCOL_VERSION,
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

    fn old(id: &str) -> LinkAnnouncement {
        LinkAnnouncement {
            protocol_version: PROTOCOL_VERSION - 1,
            ..ann(id, id)
        }
    }

    fn newer(id: &str) -> LinkAnnouncement {
        LinkAnnouncement {
            protocol_version: PROTOCOL_VERSION + 1,
            ..ann(id, id)
        }
    }

    fn record(registry: &Registry, id: &str) -> DaemonRecord {
        registry
            .list()
            .into_iter()
            .find(|record| record.id.to_string() == id)
            .unwrap()
    }

    #[test]
    fn an_older_or_newer_claim_with_no_link_is_refused_and_recorded_needs_update() {
        for announced in [old("a"), newer("a")] {
            let mut registry = Registry::new();
            let Claim::NeedsUpdate { snapshot } = registry.claim(&announced) else {
                panic!("not recorded needs-update")
            };
            let listed = record(&registry, "a");
            assert_eq!(listed.health, Health::NeedsUpdate);
            assert_eq!(listed.protocol_version, announced.protocol_version);
            assert_eq!(snapshot.records, registry.list());
            // Nothing is reserved: the id is free for a current-version claim.
            assert!(matches!(
                registry.claim(&ann("a", "a")),
                Claim::Reserved { .. }
            ));
        }
    }

    #[test]
    fn a_mismatched_claim_on_a_live_id_changes_neither_record_nor_link() {
        let mut registry = Registry::new();
        let link = online(&mut registry, "a", 100);
        let before = registry.list();
        assert_eq!(
            registry.claim(&old("a")),
            Claim::Refused(RefuseReason::VersionMismatch)
        );
        assert_eq!(registry.list(), before);
        assert_eq!(registry.answered(link, at(2), 102).0, LinkAction::Idle);
        assert_eq!(health(&registry, "a"), (Health::Online, 102));
    }

    #[test]
    fn a_mismatched_claim_on_a_silent_link_closes_it_and_records_needs_update() {
        let mut registry = Registry::new();
        let silent = online(&mut registry, "a", 100);
        registry.tick(silent, at(9));
        assert_eq!(health(&registry, "a"), (Health::Offline, 100));
        assert!(matches!(
            registry.claim(&old("a")),
            Claim::NeedsUpdate { .. }
        ));
        assert_eq!(health(&registry, "a").0, Health::NeedsUpdate);
        assert_eq!(registry.answered(silent, at(10), 110).0, LinkAction::Stale);
        assert_eq!(registry.tick(silent, at(20)), (LinkAction::Stale, None));
        assert_eq!(health(&registry, "a").0, Health::NeedsUpdate);
    }

    #[test]
    fn a_mismatched_claim_on_a_reserved_id_is_already_linked() {
        let mut registry = Registry::new();
        reserve(&mut registry, &ann("a", "A"));
        assert_eq!(
            registry.claim(&old("a")),
            Claim::Refused(RefuseReason::AlreadyLinked)
        );
        assert!(registry.list().is_empty());
    }

    #[test]
    fn a_needs_update_daemon_that_relinks_on_the_current_version_is_online() {
        let mut registry = Registry::new();
        registry.claim(&old("a"));
        let (link, _) = reserve(&mut registry, &ann("a", "a"));
        registry.confirm(link, t0(), 50).unwrap();
        assert_eq!(health(&registry, "a"), (Health::Online, 50));
        assert_eq!(record(&registry, "a").protocol_version, PROTOCOL_VERSION);
    }

    #[test]
    fn needs_update_keeps_the_last_seen_of_the_record_it_replaces() {
        let mut registry = Registry::new();
        let link = online(&mut registry, "a", 100);
        registry.closed(link);
        registry.claim(&old("a"));
        assert_eq!(health(&registry, "a"), (Health::NeedsUpdate, 100));
    }

    #[test]
    fn a_mismatched_claim_leaves_other_daemons_unchanged() {
        let mut registry = Registry::new();
        online(&mut registry, "b", 100);
        let before = record(&registry, "b");
        registry.claim(&old("a"));
        assert_eq!(record(&registry, "b"), before);
    }

    #[test]
    fn a_failed_reservation_for_the_version_records_needs_update_and_frees_the_id() {
        let mut registry = Registry::new();
        let (link, _) = reserve(&mut registry, &ann("a", "A"));
        let (action, snapshot) = registry.claim_failed(link, ClaimFailure::VersionMismatch);
        assert_eq!(action, LinkAction::Idle);
        assert_eq!(snapshot.unwrap().records, registry.list());
        let listed = record(&registry, "a");
        assert_eq!(listed.health, Health::NeedsUpdate);
        assert_eq!(listed.name, "A");
        assert!(matches!(
            registry.claim(&ann("a", "A")),
            Claim::Reserved { .. }
        ));
    }

    #[test]
    fn a_failed_reservation_never_overwrites_a_live_record() {
        let mut registry = Registry::new();
        let live = online(&mut registry, "a", 100);
        let before = registry.list();
        for reason in [ClaimFailure::VersionMismatch, ClaimFailure::Dropped] {
            assert_eq!(
                registry.claim_failed(live, reason),
                (LinkAction::Stale, None)
            );
        }
        assert_eq!(registry.list(), before);
        assert_eq!(registry.answered(live, at(1), 101).0, LinkAction::Idle);
    }

    #[test]
    fn build_mismatch_auth_rejected_and_dropped_change_no_record_and_release() {
        for reason in [
            ClaimFailure::BuildMismatch,
            ClaimFailure::AuthRejected,
            ClaimFailure::Dropped,
        ] {
            let mut registry = Registry::new();
            let (link, _) = reserve(&mut registry, &ann("a", "A"));
            assert_eq!(registry.claim_failed(link, reason).0, LinkAction::Idle);
            assert!(registry.list().is_empty(), "{reason:?}");
            assert!(matches!(
                registry.claim(&ann("a", "A")),
                Claim::Reserved { .. }
            ));
            assert_eq!(
                registry.claim_failed(link, reason),
                (LinkAction::Stale, None),
                "a released token is stale"
            );
        }
    }

    #[test]
    fn a_failed_reservation_leaves_an_existing_record_as_it_was() {
        let mut registry = Registry::new();
        let first = online(&mut registry, "a", 100);
        registry.closed(first);
        let before = registry.list();
        let (link, _) = reserve(&mut registry, &ann("a", "A"));
        registry.claim_failed(link, ClaimFailure::AuthRejected);
        assert_eq!(registry.list(), before);
    }
}
