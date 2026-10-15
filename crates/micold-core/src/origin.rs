//! Which session started another one (feature 551).
//!
//! A session started by an agent records the session that started it as its *origin*. This module
//! holds the pure half: the wire/runtime shape ([`SessionOrigin`], [`OriginStatus`]) and the one
//! check every write path runs before an origin is recorded ([`validate_origin`], research R4).

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
}
