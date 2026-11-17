//! The shared sidebar-order vectors (#1732, FR-012, research R10): the orchestrator sends rows in a
//! computed order and the web client's reducer (`packages/protocol/src/order.ts`) keeps it. This test
//! builds each case's changes with the orchestrator's own [`diff`], checks that [`apply`] reaches the
//! target snapshot, and compares the cases with the checked-in
//! `packages/protocol/tests/vectors/sidebar-order.json`, which `order-vectors.test.ts` replays in
//! jest. `MICOLD_WRITE_VECTORS=1` rewrites the file (`scripts/api-types.sh` does that); without it a
//! stale file fails.

use micold_core::client_api::diff::{apply, diff};
use micold_core::client_api::messages::{
    Change, Daemon, DaemonStatus, Location, Project, Session, SessionKind, SessionState, Snapshot,
};
use serde_json::{json, Value};

const VECTORS: &str = "../../packages/protocol/tests/vectors/sidebar-order.json";

fn daemon(id: &str, status: DaemonStatus) -> Daemon {
    Daemon {
        id: id.into(),
        name: format!("{id} host"),
        status,
    }
}

fn project(id: &str, daemon_id: &str) -> Project {
    Project {
        id: id.into(),
        daemon_id: daemon_id.into(),
        name: format!("{id} repo"),
    }
}

fn location(id: &str, project_id: &str, is_default: bool) -> Location {
    Location {
        id: id.into(),
        project_id: project_id.into(),
        name: id.into(),
        branch: if is_default {
            "main".into()
        } else {
            format!("feature-{id}")
        },
        is_default,
    }
}

fn session(id: &str, location_id: &str, state: SessionState) -> Session {
    Session {
        id: id.into(),
        location_id: location_id.into(),
        name: format!("{id} chat"),
        kind: SessionKind::Structured,
        state,
        reachable: true,
    }
}

/// Two daemons, each with projects, a root and worktrees, and sessions under each location.
fn base() -> Snapshot {
    use SessionState::*;
    Snapshot {
        epoch: 7,
        seq: 0,
        daemons: vec![
            daemon("d1", DaemonStatus::Online),
            daemon("d2", DaemonStatus::Online),
        ],
        projects: vec![
            project("d1:p1", "d1"),
            project("d1:p2", "d1"),
            project("d2:p1", "d2"),
        ],
        locations: vec![
            location("d1:p1#", "d1:p1", true),
            location("d1:p1#wt-a", "d1:p1", false),
            location("d1:p2#", "d1:p2", true),
            location("d2:p1#", "d2:p1", true),
            location("d2:p1#wt-b", "d2:p1", false),
        ],
        sessions: vec![
            session("s1", "d1:p1#", Working),
            session("s2", "d1:p1#", Idle),
            session("s3", "d1:p1#wt-a", Waiting),
            session("s4", "d1:p2#", Stopped),
            session("s5", "d2:p1#", Working),
            session("s6", "d2:p1#wt-b", Error),
        ],
    }
}

/// The row keys in display order, the rule of `order.ts` `rowKeys`: daemons, each daemon's projects,
/// each project's locations followed by their sessions, every level in list order.
fn row_keys(s: &Snapshot) -> Vec<String> {
    let mut keys = Vec::new();
    for d in &s.daemons {
        for p in s.projects.iter().filter(|p| p.daemon_id == d.id) {
            for l in s.locations.iter().filter(|l| l.project_id == p.id) {
                let at = format!("{}/{}/{}", d.id, p.id, l.id);
                keys.push(at.clone());
                for x in s.sessions.iter().filter(|x| x.location_id == l.id) {
                    keys.push(format!("{at}/{}", x.id));
                }
            }
        }
    }
    keys
}

/// One vector: `start`, then the changes the orchestrator sends for each later snapshot in turn.
fn case(name: &str, start: Snapshot, steps: &[Snapshot]) -> Value {
    let mut changes = Vec::new();
    let mut at = start.clone();
    for next in steps {
        let batch = diff(&at, next);
        let reached = apply(&at, &batch);
        assert_eq!(&reached.daemons, &next.daemons, "{name}: daemons");
        assert_eq!(&reached.projects, &next.projects, "{name}: projects");
        assert_eq!(&reached.locations, &next.locations, "{name}: locations");
        assert_eq!(&reached.sessions, &next.sessions, "{name}: sessions");
        changes.extend(batch);
        at = next.clone();
    }
    let changes: Vec<Change> = changes
        .into_iter()
        .enumerate()
        .map(|(i, kind)| Change {
            seq: start.seq + 1 + i as u64,
            kind,
        })
        .collect();
    let sessions: serde_json::Map<String, Value> = at
        .sessions
        .iter()
        .map(|s| {
            (
                s.id.clone(),
                json!({ "state": s.state, "reachable": s.reachable }),
            )
        })
        .collect();
    json!({
        "name": name,
        "snapshot": start,
        "changes": changes,
        "expected_keys": row_keys(&at),
        "expected_sessions": sessions,
    })
}

fn vectors() -> Value {
    let start = base();

    // New rows land after their `after_key`: a worktree between two locations, a session at the top
    // of a location, a project, and a session state change in place.
    let mut grown = start.clone();
    grown
        .locations
        .insert(1, location("d1:p1#wt-c", "d1:p1", false));
    grown
        .sessions
        .insert(2, session("s7", "d1:p1#wt-c", SessionState::Working));
    grown
        .sessions
        .insert(5, session("s8", "d1:p2#", SessionState::Waiting));
    grown.projects.insert(1, project("d1:p3", "d1"));
    grown.locations.insert(3, location("d1:p3#", "d1:p3", true));
    grown.sessions[0].state = SessionState::Waiting;

    // A daemon goes offline: its rows keep their place and its sessions turn unreachable.
    let mut offline = start.clone();
    offline.daemons[0].status = DaemonStatus::Offline;
    for s in offline
        .sessions
        .iter_mut()
        .filter(|s| s.location_id.starts_with("d1:"))
    {
        s.reachable = false;
    }
    let mut back = offline.clone();
    back.daemons[0].status = DaemonStatus::Online;
    back.sessions.iter_mut().for_each(|s| s.reachable = true);

    // One key removed, then upserted again: it returns to its `after_key` place.
    let mut removed = start.clone();
    removed.sessions.remove(1);
    let restored = start.clone();

    let cases = vec![
        case(
            "grouping and nesting with new rows",
            start.clone(),
            &[grown],
        ),
        case("a daemon goes offline", start.clone(), &[offline.clone()]),
        case(
            "a daemon goes offline and comes back",
            start.clone(),
            &[offline, back],
        ),
        case("remove then upsert one key", start, &[removed, restored]),
    ];
    json!({
        "description": "FR-012 sidebar-order vectors, written by crates/micold-core/tests/client_api_vectors.rs (MICOLD_WRITE_VECTORS=1 or scripts/api-types.sh). Apply `changes` to `snapshot` in order; the row keys must equal `expected_keys` and each session's state and reachability `expected_sessions`.",
        "cases": cases,
    })
}

#[test]
fn the_checked_in_vectors_equal_the_generated_ones() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(VECTORS);
    let mut text = serde_json::to_string_pretty(&vectors()).expect("serializes");
    text.push('\n');
    if std::env::var_os("MICOLD_WRITE_VECTORS").is_some() {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("vectors dir");
        std::fs::write(&path, &text).expect("write vectors");
        return;
    }
    let committed = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run scripts/api-types.sh", path.display()));
    assert!(
        committed == text,
        "{} is stale; run scripts/api-types.sh",
        path.display()
    );
}

#[test]
fn removing_and_upserting_a_key_restores_the_order() {
    let start = base();
    let mut removed = start.clone();
    removed.sessions.remove(1);
    let back = apply(&removed, &diff(&removed, &start));
    assert_eq!(row_keys(&back), row_keys(&start));
}
