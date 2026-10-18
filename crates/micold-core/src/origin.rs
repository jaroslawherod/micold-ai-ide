//! Which session started another one (feature 551).
//!
//! A session started by an agent records the session that started it as its *origin*. This module
//! holds the pure half: the wire/runtime shape ([`SessionOrigin`], [`OriginStatus`]) and the one
//! check every write path runs before an origin is recorded ([`validate_origin`], research R4).

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::session::{SessionId, SessionLocation};

/// Whether the session an origin names is still in the catalog.
///
/// Runtime state: the daemon overlays it from the catalog, as it does `activity`. Only the id is
/// stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginStatus {
    /// The origin is in the catalog, or a provider reported it and it has not appeared yet.
    Known,
    /// The origin was archived (deleted). The child stays, top-level, with the "origin removed"
    /// marker (FR-018).
    Removed,
}

impl OriginStatus {
    /// The wire/tool spelling: `"known"` or `"removed"`.
    pub fn as_str(self) -> &'static str {
        match self {
            OriginStatus::Known => "known",
            OriginStatus::Removed => "removed",
        }
    }
}

/// The session that started a session, and whether it is still there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionOrigin {
    pub id: SessionId,
    pub status: OriginStatus,
}

/// What [`validate_origin`] needs to know about one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginNode {
    pub id: SessionId,
    pub location: SessionLocation,
    /// The origin the session already has, if any (used to walk the chain).
    pub origin: Option<SessionId>,
    /// Archived sessions are not valid origins for a newly created session.
    pub archived: bool,
}

/// Why an origin was not recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginRejected {
    /// The candidate is the session itself.
    SelfReference,
    /// The candidate is not in the session's project and worktree.
    CrossWorktree,
    /// The candidate's own chain of origins reaches the session.
    Cycle,
    /// The candidate is not in the catalog and no provider reported it, or it is archived.
    Unknown,
}

/// The longest origin chain the walk follows before it gives up and calls it a cycle.
pub const MAX_ORIGIN_DEPTH: usize = 64;

/// Check that `candidate` may be recorded as the origin of `child` (R4). One pure function used by
/// every write path.
///
/// `lookup` resolves a session id of the child's project. A candidate it does not know is accepted
/// only when `reported_by_provider` (it is kept pending until it appears); the full check then
/// runs when it does.
pub fn validate_origin(
    child: &OriginNode,
    candidate: SessionId,
    lookup: &dyn Fn(SessionId) -> Option<OriginNode>,
    reported_by_provider: bool,
) -> Result<(), OriginRejected> {
    if candidate == child.id {
        return Err(OriginRejected::SelfReference);
    }
    let Some(node) = lookup(candidate) else {
        return if reported_by_provider {
            Ok(())
        } else {
            Err(OriginRejected::Unknown)
        };
    };
    if node.location != child.location {
        return Err(OriginRejected::CrossWorktree);
    }
    if node.archived && !reported_by_provider {
        return Err(OriginRejected::Unknown);
    }
    let mut next = node.origin;
    for _ in 0..MAX_ORIGIN_DEPTH {
        let Some(id) = next else {
            return Ok(());
        };
        if id == child.id {
            return Err(OriginRejected::Cycle);
        }
        next = lookup(id).and_then(|n| n.origin);
    }
    Err(OriginRejected::Cycle)
}

/// Why a session with an origin is shown top-level: its origin is not among the sessions listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orphan {
    /// The origin has not appeared yet (FR-016).
    Pending,
    /// The origin was deleted (FR-018).
    Removed,
}

/// Where a session sits in the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreePlace {
    /// A session with no origin, or the member where a cycle was cut.
    Root,
    /// Nested under its origin.
    Child,
    /// Top-level, with its origin missing from the list.
    Orphan(Orphan),
}

impl TreePlace {
    /// Whether the row carries the child marker (FR-007, SC-004): children and orphans do.
    pub fn is_child(self) -> bool {
        !matches!(self, TreePlace::Root)
    }
}

/// One session in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    pub id: SessionId,
    /// 0 for a top-level row, one more for each origin above it.
    pub depth: usize,
    pub place: TreePlace,
}

/// The sessions of one location as a forest, derived from their origins and never stored.
///
/// Every session appears exactly once (FR-008), children follow their origin depth-first, and
/// roots and siblings keep the input order.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionTree {
    pub rows: Vec<TreeRow>,
}

impl SessionTree {
    /// Build the forest from `(id, origin)` pairs in catalog order.
    pub fn build(sessions: &[(SessionId, Option<SessionOrigin>)]) -> Self {
        let index: HashMap<SessionId, usize> = sessions
            .iter()
            .enumerate()
            .map(|(i, (id, _))| (*id, i))
            .collect();
        // The listed parent of each session, cut where a chain comes back to itself.
        let mut parent: Vec<Option<usize>> = sessions
            .iter()
            .map(|(_, o)| o.and_then(|o| index.get(&o.id).copied()))
            .collect();
        let mut settled = vec![false; sessions.len()];
        for start in 0..sessions.len() {
            let mut walk = Vec::new();
            let mut at = Some(start);
            while let Some(i) = at {
                if settled[i] {
                    break;
                }
                if walk.contains(&i) {
                    // `i` is the first member seen twice: it becomes the root of the loop.
                    parent[i] = None;
                    break;
                }
                walk.push(i);
                at = parent[i];
            }
            for i in walk {
                settled[i] = true;
            }
        }
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); sessions.len()];
        let mut roots = Vec::new();
        for (i, p) in parent.iter().enumerate() {
            match p {
                Some(p) => children[*p].push(i),
                None => roots.push(i),
            }
        }
        let mut rows = Vec::with_capacity(sessions.len());
        let mut stack: Vec<(usize, usize)> = roots.iter().rev().map(|&r| (r, 0)).collect();
        while let Some((i, depth)) = stack.pop() {
            let (id, origin) = sessions[i];
            let place = match (parent[i], origin) {
                (Some(_), _) => TreePlace::Child,
                (None, Some(o)) if !index.contains_key(&o.id) => {
                    TreePlace::Orphan(match o.status {
                        OriginStatus::Known => Orphan::Pending,
                        OriginStatus::Removed => Orphan::Removed,
                    })
                }
                (None, _) => TreePlace::Root,
            };
            rows.push(TreeRow { id, depth, place });
            stack.extend(children[i].iter().rev().map(|&c| (c, depth + 1)));
        }
        SessionTree { rows }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn id(n: u128) -> SessionId {
        SessionId::from_uuid(uuid::Uuid::from_u128(n))
    }

    fn wt(dir: &str) -> SessionLocation {
        SessionLocation::Worktree(dir.to_string())
    }

    fn node(n: u128, location: SessionLocation, origin: Option<u128>) -> OriginNode {
        OriginNode {
            id: id(n),
            location,
            origin: origin.map(id),
            archived: false,
        }
    }

    fn check(
        child: &OriginNode,
        candidate: u128,
        known: &[OriginNode],
        reported: bool,
    ) -> Result<(), OriginRejected> {
        let map: HashMap<SessionId, OriginNode> = known.iter().map(|n| (n.id, n.clone())).collect();
        validate_origin(child, id(candidate), &|i| map.get(&i).cloned(), reported)
    }

    #[test]
    fn a_session_is_not_its_own_origin() {
        let child = node(1, wt("a"), None);
        assert_eq!(
            check(&child, 1, std::slice::from_ref(&child), false),
            Err(OriginRejected::SelfReference)
        );
        assert_eq!(
            check(&child, 1, &[], true),
            Err(OriginRejected::SelfReference)
        );
    }

    #[test]
    fn an_unknown_candidate_is_accepted_only_when_a_provider_reported_it() {
        let child = node(1, wt("a"), None);
        assert_eq!(check(&child, 2, &[], false), Err(OriginRejected::Unknown));
        assert_eq!(check(&child, 2, &[], true), Ok(()));
    }

    #[test]
    fn a_candidate_in_another_location_is_rejected_and_the_same_one_accepted() {
        let child = node(1, wt("a"), None);
        let other = node(2, wt("b"), None);
        let root = node(3, SessionLocation::Default, None);
        let same = node(4, wt("a"), None);
        let known = [other, root, same];
        assert_eq!(
            check(&child, 2, &known, false),
            Err(OriginRejected::CrossWorktree)
        );
        assert_eq!(
            check(&child, 3, &known, false),
            Err(OriginRejected::CrossWorktree)
        );
        assert_eq!(check(&child, 4, &known, false), Ok(()));
    }

    #[test]
    fn a_chain_reaching_the_child_is_a_cycle() {
        let child = node(1, wt("a"), None);
        let known = [node(2, wt("a"), Some(3)), node(3, wt("a"), Some(1))];
        assert_eq!(check(&child, 2, &known, false), Err(OriginRejected::Cycle));
    }

    #[test]
    fn a_chain_within_the_bound_that_misses_the_child_is_accepted() {
        let child = node(1, wt("a"), None);
        let mut known = Vec::new();
        for n in 0..(MAX_ORIGIN_DEPTH as u128 - 2) {
            known.push(node(100 + n, wt("a"), Some(101 + n)));
        }
        known.push(node(100 + MAX_ORIGIN_DEPTH as u128 - 2, wt("a"), None));
        assert_eq!(check(&child, 100, &known, false), Ok(()));
    }

    /// A candidate (id 100) followed by a chain of `nodes` sessions in total, the last without an origin.
    fn chain_of(nodes: usize) -> Vec<OriginNode> {
        (0..nodes as u128)
            .map(|n| node(100 + n, wt("a"), (n + 1 < nodes as u128).then_some(101 + n)))
            .collect()
    }

    #[test]
    fn a_chain_of_exactly_the_maximum_depth_is_accepted() {
        let child = node(1, wt("a"), None);
        let known = chain_of(MAX_ORIGIN_DEPTH);
        assert_eq!(check(&child, 100, &known, false), Ok(()));
    }

    #[test]
    fn a_chain_one_longer_than_the_maximum_depth_is_rejected_as_a_cycle() {
        let child = node(1, wt("a"), None);
        let known = chain_of(MAX_ORIGIN_DEPTH + 1);
        assert_eq!(
            check(&child, 100, &known, false),
            Err(OriginRejected::Cycle)
        );
    }

    #[test]
    fn a_loop_that_never_reaches_the_child_terminates_as_a_cycle() {
        let child = node(1, wt("a"), None);
        let known = [node(2, wt("a"), Some(3)), node(3, wt("a"), Some(2))];
        assert_eq!(check(&child, 2, &known, false), Err(OriginRejected::Cycle));
    }

    #[test]
    fn an_archived_candidate_is_rejected_for_a_tool_server_spawn() {
        let child = node(1, wt("a"), None);
        let mut gone = node(2, wt("a"), None);
        gone.archived = true;
        assert_eq!(
            check(&child, 2, &[gone], false),
            Err(OriginRejected::Unknown)
        );
    }

    fn origin_of(n: u128, status: OriginStatus) -> Option<SessionOrigin> {
        Some(SessionOrigin { id: id(n), status })
    }

    fn known(n: u128) -> Option<SessionOrigin> {
        origin_of(n, OriginStatus::Known)
    }

    fn rows(sessions: &[(u128, Option<SessionOrigin>)]) -> Vec<(u128, usize, TreePlace)> {
        let input: Vec<_> = sessions.iter().map(|(n, o)| (id(*n), *o)).collect();
        SessionTree::build(&input)
            .rows
            .iter()
            .map(|r| (r.id.0.as_u128(), r.depth, r.place))
            .collect()
    }

    #[test]
    fn a_tree_nests_to_any_depth_and_keeps_catalog_order() {
        // 3 is a child of 2, which is a child of 1; 4 is another child of 1; 5 is a root.
        let got = rows(&[
            (1, None),
            (5, None),
            (3, known(2)),
            (4, known(1)),
            (2, known(1)),
        ]);
        assert_eq!(
            got,
            vec![
                (1, 0, TreePlace::Root),
                (4, 1, TreePlace::Child),
                (2, 1, TreePlace::Child),
                (3, 2, TreePlace::Child),
                (5, 0, TreePlace::Root),
            ]
        );
    }

    #[test]
    fn a_cycle_breaks_at_the_first_repeat_and_every_session_appears_once() {
        let got = rows(&[(1, known(2)), (2, known(1)), (3, known(3))]);
        assert_eq!(
            got,
            vec![
                (1, 0, TreePlace::Root),
                (2, 1, TreePlace::Child),
                (3, 0, TreePlace::Root),
            ]
        );
    }

    #[test]
    fn an_absent_origin_is_pending_or_removed_by_its_status() {
        let got = rows(&[
            (1, known(9)),
            (2, origin_of(8, OriginStatus::Removed)),
            (3, known(1)),
        ]);
        assert_eq!(
            got,
            vec![
                (1, 0, TreePlace::Orphan(Orphan::Pending)),
                (3, 1, TreePlace::Child),
                (2, 0, TreePlace::Orphan(Orphan::Removed)),
            ]
        );
        assert!(TreePlace::Orphan(Orphan::Pending).is_child());
        assert!(!TreePlace::Root.is_child());
    }
}
