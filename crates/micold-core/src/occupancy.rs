//! The one-session-per-location rule (feature 776).
//!
//! A worktree, or the project root (`SessionLocation::Default`), hosts at most one session, whatever
//! its state. This module is the single place that says whether a location is taken, whether a
//! start would put a second live session beside another (legacy catalogs only), and in what words
//! either refusal reaches a person or an agent. Pure: the daemon enforces it under its catalog lock
//! and the client's affordance reads the same functions.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::session::{Session, SessionId, SessionLocation};

/// The session a refusal names (FR-003): its id and the name the sidebar displays.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExistingSession {
    /// The session's id; `start_session` on it brings it back.
    pub id: SessionId,
    /// The session's displayed label.
    pub name: String,
}

impl ExistingSession {
    fn of(session: &Session) -> Self {
        Self {
            id: session.id,
            name: session.label.display().to_string(),
        }
    }
}

/// Whether a location can take a new session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Occupancy {
    /// No un-archived session has the location.
    Free,
    /// An un-archived session has it: the first in list order.
    Taken(ExistingSession),
}

/// Whether starting one session would run it beside another live session of its location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartBlock {
    /// Nothing else of this location is starting, running or restarting.
    Clear,
    /// Another un-archived session of the location is live (legacy catalogs, FR-013).
    Blocked {
        /// The live session.
        live: ExistingSession,
    },
}

/// Whether `location` holds an un-archived session among `sessions` (FR-001, FR-010, FR-011).
pub fn occupancy(sessions: &[Session], location: &SessionLocation) -> Occupancy {
    sessions
        .iter()
        .find(|s| !s.archived && &s.location == location)
        .map_or(Occupancy::Free, |s| {
            Occupancy::Taken(ExistingSession::of(s))
        })
}

/// Whether starting `id` is blocked by another live session of its location (FR-013).
///
/// `live_ids` are the sessions the caller knows to be hosted by a process; a session is also live
/// when its lifecycle says so. An unknown `id` is `Clear`.
pub fn start_block(
    sessions: &[Session],
    id: SessionId,
    live_ids: &HashSet<SessionId>,
) -> StartBlock {
    let Some(target) = sessions.iter().find(|s| s.id == id) else {
        return StartBlock::Clear;
    };
    sessions
        .iter()
        .find(|s| {
            s.id != id
                && !s.archived
                && s.location == target.location
                && (s.lifecycle.is_live() || live_ids.contains(&s.id))
        })
        .map_or(StartBlock::Clear, |s| StartBlock::Blocked {
            live: ExistingSession::of(s),
        })
}

fn place(location: &SessionLocation) -> &'static str {
    match location {
        SessionLocation::Default => "the project root",
        SessionLocation::Worktree(_) => "this worktree",
    }
}

/// The refusal of a second session, naming the existing one (FR-003).
pub fn refusal_text(existing: &ExistingSession, location: &SessionLocation) -> String {
    format!(
        "{} already has a session: \"{}\" ({}); start_session on {} brings it back",
        place(location),
        existing.name,
        existing.id.0,
        existing.id.0
    )
}

/// The refusal of a start beside a live session of the same location (FR-013).
pub fn start_block_text(live: &ExistingSession, location: &SessionLocation) -> String {
    format!(
        "session \"{}\" ({}) is running in {}; stop it first",
        live.name,
        live.id.0,
        place(location)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::{AiCli, SessionLabel, SessionLifecycle};

    fn session(location: SessionLocation, name: &str, lifecycle: SessionLifecycle) -> Session {
        let mut s = Session::start_new(location, AiCli::ClaudeCode);
        s.label = SessionLabel::Named(name.to_string());
        s.lifecycle = lifecycle;
        s
    }

    fn wt(name: &str) -> SessionLocation {
        SessionLocation::Worktree(name.to_string())
    }

    #[test]
    fn a_location_without_sessions_is_free() {
        assert_eq!(occupancy(&[], &wt("a")), Occupancy::Free);
        let other = session(wt("b"), "x", SessionLifecycle::Idle);
        assert_eq!(occupancy(&[other], &wt("a")), Occupancy::Free);
    }

    #[test]
    fn archived_sessions_do_not_occupy() {
        let mut s = session(wt("a"), "gone", SessionLifecycle::Idle);
        s.archived = true;
        assert_eq!(occupancy(&[s], &wt("a")), Occupancy::Free);
    }

    #[test]
    fn every_lifecycle_occupies_and_the_first_is_named() {
        let states = [
            SessionLifecycle::Idle,
            SessionLifecycle::Starting,
            SessionLifecycle::Running,
            SessionLifecycle::Restarting { attempts: 1 },
            SessionLifecycle::Failed {
                reason: "r".into(),
                attempts: 3,
            },
            SessionLifecycle::InterruptedResumable,
        ];
        for state in states {
            let first = session(wt("a"), "first", state.clone());
            let second = session(wt("a"), "second", SessionLifecycle::Running);
            let want = ExistingSession {
                id: first.id,
                name: "first".into(),
            };
            assert_eq!(
                occupancy(&[first, second], &wt("a")),
                Occupancy::Taken(want),
                "{state:?}"
            );
        }
    }

    #[test]
    fn the_project_root_is_a_location_like_any_worktree() {
        let s = session(SessionLocation::Default, "root", SessionLifecycle::Idle);
        assert!(matches!(
            occupancy(std::slice::from_ref(&s), &SessionLocation::Default),
            Occupancy::Taken(_)
        ));
        assert_eq!(occupancy(&[s], &wt("a")), Occupancy::Free);
    }

    #[test]
    fn a_locations_only_session_is_never_blocked() {
        let s = session(wt("a"), "only", SessionLifecycle::Idle);
        let id = s.id;
        assert_eq!(start_block(&[s], id, &HashSet::new()), StartBlock::Clear);
    }

    #[test]
    fn another_live_session_of_the_location_blocks_a_start() {
        for live in [
            SessionLifecycle::Starting,
            SessionLifecycle::Running,
            SessionLifecycle::Restarting { attempts: 1 },
        ] {
            let stopped = session(wt("a"), "stopped", SessionLifecycle::Idle);
            let running = session(wt("a"), "running", live);
            let (sid, rid) = (stopped.id, running.id);
            assert_eq!(
                start_block(&[stopped, running], sid, &HashSet::new()),
                StartBlock::Blocked {
                    live: ExistingSession {
                        id: rid,
                        name: "running".into()
                    }
                }
            );
        }
    }

    #[test]
    fn stopped_archived_and_foreign_sessions_do_not_block() {
        let a = session(wt("a"), "a", SessionLifecycle::Idle);
        let b = session(wt("a"), "b", SessionLifecycle::Idle);
        let mut c = session(wt("a"), "c", SessionLifecycle::Running);
        c.archived = true;
        let d = session(wt("z"), "d", SessionLifecycle::Running);
        let id = a.id;
        assert_eq!(
            start_block(&[a, b, c, d], id, &HashSet::new()),
            StartBlock::Clear
        );
    }

    #[test]
    fn refusal_text_holds_id_name_and_the_start_hint() {
        let e = ExistingSession {
            id: SessionId::new(),
            name: "Fix login".into(),
        };
        let text = refusal_text(&e, &wt("a"));
        assert!(text.starts_with("this worktree already has a session"));
        assert!(text.contains("\"Fix login\""));
        assert!(text.contains(&format!("start_session on {}", e.id.0)));
        let root = refusal_text(&e, &SessionLocation::Default);
        assert!(root.starts_with("the project root already has a session"));
    }

    #[test]
    fn start_block_text_names_the_live_session_and_says_stop() {
        let e = ExistingSession {
            id: SessionId::new(),
            name: "Live".into(),
        };
        let text = start_block_text(&e, &wt("a"));
        assert!(text.contains("\"Live\"") && text.contains("stop it first"));
    }
}
