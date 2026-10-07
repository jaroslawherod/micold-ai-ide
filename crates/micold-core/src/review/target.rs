//! Which running session receives a review prompt (feature 482, research R5).

use crate::clock::Uptime;
use crate::session::SessionId;

/// The candidate most recently active: the largest `Uptime`, ties broken by the larger
/// `SessionId`; `None` when no session of the entry runs (data-model `pick_target`).
pub fn pick_target(candidates: &[(SessionId, Uptime)]) -> Option<SessionId> {
    candidates
        .iter()
        .max_by_key(|(session, active)| (*active, *session))
        .map(|(session, _)| *session)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn sid(n: u128) -> SessionId {
        SessionId(Uuid::from_u128(n))
    }

    fn at(nanos: u64) -> Uptime {
        Uptime::from_nanos(nanos)
    }

    #[test]
    fn the_most_recently_active_session_is_picked() {
        let candidates = [(sid(1), at(50)), (sid(2), at(90)), (sid(3), at(10))];
        assert_eq!(pick_target(&candidates), Some(sid(2)));
    }

    #[test]
    fn a_tie_goes_to_the_larger_session_id() {
        let candidates = [(sid(7), at(40)), (sid(9), at(40)), (sid(8), at(40))];
        assert_eq!(
            pick_target(&candidates),
            Some(sid(9)),
            "deterministic for equal times"
        );
    }

    #[test]
    fn no_candidate_picks_none() {
        assert_eq!(pick_target(&[]), None, "no running session: start one (W7)");
    }
}
