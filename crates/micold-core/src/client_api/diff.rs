//! The changes between two snapshots (data-model.md *Diff*, research R8): what the feed sends so a
//! client holding `old` reaches `new` with the #1735 reducer (`packages/protocol/src/order.ts`,
//! mirrored by [`apply`]). That reducer never moves an existing row and places a new one after its
//! `after_key` (or last), so a row whose place changed is removed and upserted again. A change of a
//! session's state or a daemon's status alone is sent as the small kind.

use super::messages::{
    ChangeKind, Daemon, Entity, EntityRef, Location, Project, Session, Snapshot,
};

/// A row of one of the snapshot's lists.
trait Row: Clone + PartialEq {
    fn id(&self) -> &str;
    fn entity(self) -> Entity;
    fn reference(id: String) -> EntityRef;
    /// The small change from `old` to `self`, when only that differs.
    fn small(&self, _old: &Self) -> Option<ChangeKind> {
        None
    }
}

impl Row for Daemon {
    fn id(&self) -> &str {
        &self.id
    }
    fn entity(self) -> Entity {
        Entity::Daemon(self)
    }
    fn reference(id: String) -> EntityRef {
        EntityRef::Daemon(id)
    }
    fn small(&self, old: &Self) -> Option<ChangeKind> {
        (Daemon {
            status: old.status,
            ..self.clone()
        } == *old)
            .then(|| ChangeKind::DaemonStatus {
                daemon_id: self.id.clone(),
                status: self.status,
            })
    }
}

impl Row for Project {
    fn id(&self) -> &str {
        &self.id
    }
    fn entity(self) -> Entity {
        Entity::Project(self)
    }
    fn reference(id: String) -> EntityRef {
        EntityRef::Project(id)
    }
}

impl Row for Location {
    fn id(&self) -> &str {
        &self.id
    }
    fn entity(self) -> Entity {
        Entity::Location(self)
    }
    fn reference(id: String) -> EntityRef {
        EntityRef::Location(id)
    }
}

impl Row for Session {
    fn id(&self) -> &str {
        &self.id
    }
    fn entity(self) -> Entity {
        Entity::Session(self)
    }
    fn reference(id: String) -> EntityRef {
        EntityRef::Session(id)
    }
    fn small(&self, old: &Self) -> Option<ChangeKind> {
        (Session {
            state: old.state,
            ..self.clone()
        } == *old)
            .then(|| ChangeKind::SessionState {
                session_id: self.id.clone(),
                state: self.state,
            })
    }
}

/// The removals and the upserts that turn `old` into `new` for one list.
fn list<T: Row>(
    old: &[T],
    new: &[T],
    removes: &mut Vec<ChangeKind>,
    upserts: &mut Vec<ChangeKind>,
) {
    let in_new = |id: &str| new.iter().any(|n| n.id() == id);
    let remove = |id: &str| ChangeKind::Remove {
        entity: T::reference(id.to_owned()),
    };
    for gone in old.iter().filter(|o| !in_new(o.id())) {
        removes.push(remove(gone.id()));
    }
    // What the client holds, kept in step with what is emitted.
    let mut held: Vec<T> = old.iter().filter(|o| in_new(o.id())).cloned().collect();
    for (at, row) in new.iter().enumerate() {
        if held.get(at).is_some_and(|h| h.id() == row.id()) {
            if held[at] != *row {
                upserts.push(row.small(&held[at]).unwrap_or_else(|| ChangeKind::Upsert {
                    entity: row.clone().entity(),
                    after_key: None,
                }));
                held[at] = row.clone();
            }
            continue;
        }
        let found = held.iter().position(|h| h.id() == row.id());
        if at == 0 {
            // The reducer cannot insert first: take out everything held before this row.
            for before in held.drain(..found.unwrap_or(held.len())) {
                upserts.push(remove(before.id()));
            }
            if held.first().is_some_and(|h| h.id() == row.id()) {
                if held[0] != *row {
                    upserts.push(ChangeKind::Upsert {
                        entity: row.clone().entity(),
                        after_key: None,
                    });
                    held[0] = row.clone();
                }
                continue;
            }
        } else if let Some(found) = found {
            upserts.push(remove(row.id()));
            held.remove(found);
        }
        upserts.push(ChangeKind::Upsert {
            entity: row.clone().entity(),
            after_key: (at > 0).then(|| new[at - 1].id().to_owned()),
        });
        held.insert(at, row.clone());
    }
}

/// The changes that turn `old` into `new`: removals first, then upserts and small changes, parents
/// before children, each list in `new`'s order.
pub fn diff(old: &Snapshot, new: &Snapshot) -> Vec<ChangeKind> {
    let (mut removes, mut upserts) = (Vec::new(), Vec::new());
    list(&old.daemons, &new.daemons, &mut removes, &mut upserts);
    list(&old.projects, &new.projects, &mut removes, &mut upserts);
    list(&old.locations, &new.locations, &mut removes, &mut upserts);
    list(&old.sessions, &new.sessions, &mut removes, &mut upserts);
    removes.reverse();
    removes.extend(upserts);
    removes
}

fn upsert<T: Row>(rows: &mut Vec<T>, row: T, after_key: Option<&str>) {
    if let Some(at) = rows.iter().position(|r| r.id() == row.id()) {
        rows[at] = row;
    } else {
        let at = after_key
            .and_then(|key| rows.iter().position(|r| r.id() == key))
            .map_or(rows.len(), |at| at + 1);
        rows.insert(at, row);
    }
}

/// `changes` applied to `snapshot` the way the client's reducer (`order.ts` `applyChanges`) does.
pub fn apply(snapshot: &Snapshot, changes: &[ChangeKind]) -> Snapshot {
    let mut out = snapshot.clone();
    for change in changes {
        match change.clone() {
            ChangeKind::SessionState { session_id, state } => {
                if let Some(s) = out.sessions.iter_mut().find(|s| s.id == session_id) {
                    s.state = state;
                }
            }
            ChangeKind::DaemonStatus { daemon_id, status } => {
                if let Some(d) = out.daemons.iter_mut().find(|d| d.id == daemon_id) {
                    d.status = status;
                }
            }
            ChangeKind::Upsert { entity, after_key } => {
                let after = after_key.as_deref();
                match entity {
                    Entity::Daemon(r) => upsert(&mut out.daemons, r, after),
                    Entity::Project(r) => upsert(&mut out.projects, r, after),
                    Entity::Location(r) => upsert(&mut out.locations, r, after),
                    Entity::Session(r) => upsert(&mut out.sessions, r, after),
                }
            }
            ChangeKind::Remove { entity } => match entity {
                EntityRef::Daemon(id) => out.daemons.retain(|r| r.id != id),
                EntityRef::Project(id) => out.projects.retain(|r| r.id != id),
                EntityRef::Location(id) => out.locations.retain(|r| r.id != id),
                EntityRef::Session(id) => out.sessions.retain(|r| r.id != id),
            },
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client_api::messages::{DaemonStatus, SessionKind, SessionState};

    fn daemon(id: &str, status: DaemonStatus) -> Daemon {
        Daemon {
            id: id.into(),
            name: format!("Daemon {id}"),
            status,
        }
    }

    fn session(id: &str, state: SessionState) -> Session {
        Session {
            id: id.into(),
            location_id: "l".into(),
            name: id.into(),
            kind: SessionKind::Structured,
            state,
            reachable: true,
        }
    }

    fn snap(daemons: Vec<Daemon>, sessions: Vec<Session>) -> Snapshot {
        Snapshot {
            daemons,
            sessions,
            ..Snapshot::default()
        }
    }

    #[test]
    fn equal_snapshots_have_no_changes() {
        let s = snap(vec![daemon("a", DaemonStatus::Online)], vec![]);
        assert!(diff(&s, &s).is_empty());
    }

    #[test]
    fn a_state_change_alone_is_the_small_kind() {
        let old = snap(vec![], vec![session("s", SessionState::Idle)]);
        let new = snap(vec![], vec![session("s", SessionState::Working)]);
        assert_eq!(
            diff(&old, &new),
            [ChangeKind::SessionState {
                session_id: "s".into(),
                state: SessionState::Working
            }]
        );
    }

    #[test]
    fn a_status_change_alone_is_the_small_kind() {
        let old = snap(vec![daemon("a", DaemonStatus::Online)], vec![]);
        let new = snap(vec![daemon("a", DaemonStatus::Offline)], vec![]);
        assert_eq!(
            diff(&old, &new),
            [ChangeKind::DaemonStatus {
                daemon_id: "a".into(),
                status: DaemonStatus::Offline
            }]
        );
    }

    #[test]
    fn removals_come_first_and_a_new_row_follows_its_neighbour() {
        let old = snap(
            vec![
                daemon("a", DaemonStatus::Online),
                daemon("b", DaemonStatus::Online),
            ],
            vec![],
        );
        let new = snap(
            vec![
                daemon("a", DaemonStatus::Online),
                daemon("c", DaemonStatus::Online),
            ],
            vec![],
        );
        let changes = diff(&old, &new);
        assert_eq!(
            changes[0],
            ChangeKind::Remove {
                entity: EntityRef::Daemon("b".into())
            }
        );
        assert_eq!(
            changes[1],
            ChangeKind::Upsert {
                entity: Entity::Daemon(daemon("c", DaemonStatus::Online)),
                after_key: Some("a".into())
            }
        );
        assert_eq!(apply(&old, &changes), new);
    }

    /// A small deterministic generator, so the property runs the same cases every time.
    struct Rng(u64);

    impl Rng {
        fn next(&mut self, below: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % below
        }
    }

    fn random_snapshot(rng: &mut Rng) -> Snapshot {
        const STATES: [SessionState; 3] = [
            SessionState::Idle,
            SessionState::Working,
            SessionState::Error,
        ];
        let mut ids: Vec<u64> = (0..8).filter(|_| rng.next(3) > 0).collect();
        // Shuffle, so a row's place changes between snapshots.
        for i in (1..ids.len()).rev() {
            ids.swap(i, rng.next(i as u64 + 1) as usize);
        }
        let daemons = ids
            .iter()
            .map(|i| {
                let status = if rng.next(2) == 0 {
                    DaemonStatus::Online
                } else {
                    DaemonStatus::Offline
                };
                daemon(&format!("d{i}"), status)
            })
            .collect();
        let sessions = ids
            .iter()
            .map(|i| {
                let mut s = session(&format!("s{i}"), STATES[rng.next(3) as usize]);
                if rng.next(4) == 0 {
                    s.name = format!("renamed {i}");
                }
                s
            })
            .collect();
        snap(daemons, sessions)
    }

    #[test]
    fn applying_the_diff_to_old_yields_new() {
        let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
        for case in 0..2000 {
            let old = random_snapshot(&mut rng);
            let new = random_snapshot(&mut rng);
            assert_eq!(
                apply(&old, &diff(&old, &new)),
                new,
                "case {case}: {old:?} -> {new:?}"
            );
        }
    }

    #[test]
    fn a_diff_against_a_retained_snapshot_catches_up_over_a_gap() {
        // The poller stopped at `kept`; two passes' worth of change happened before it restarted.
        let kept = snap(
            vec![daemon("a", DaemonStatus::Online)],
            vec![session("s", SessionState::Idle)],
        );
        let later = snap(
            vec![
                daemon("a", DaemonStatus::Offline),
                daemon("b", DaemonStatus::Online),
            ],
            vec![],
        );
        assert_eq!(apply(&kept, &diff(&kept, &later)), later);
    }
}
