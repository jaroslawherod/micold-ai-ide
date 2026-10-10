//! Removing, rebinding and editing a running daemon (feature 491, T031, FR-011, FR-012, FR-014,
//! US3 scenarios 5 to 7).

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use daemons_fixture::{wt, World};
use iced::futures::channel::mpsc;
use micold_client::daemon::Outbox;
use micold_client::features::daemons::{self, State};
use micold_client::features::Outcome;
use micold_client::links::Route;
use micold_core::daemons::{Binding, DaemonRuntime, DaemonState};

fn connected(w: &mut World, state: &mut State, id: micold_core::daemons::DaemonId) {
    daemons::start(state, &mut w.model(), id);
    w.links.connected(id, Outbox::new(mpsc::unbounded().0));
}

#[test]
fn the_prompt_states_both_counts_and_that_nothing_is_deleted() {
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    let prompt = daemons::request_removal(&mut state, &w.model(), boxed).unwrap();
    assert_eq!(prompt.daemon, boxed);
    assert_eq!(prompt.name, "Box");
    assert_eq!(prompt.worktrees_affected, 2);
    assert_eq!(prompt.running_sessions, 2);
    assert!(prompt.text.contains("2 worktrees"), "{}", prompt.text);
    assert!(
        prompt.text.contains("2 running sessions"),
        "{}",
        prompt.text
    );
    assert!(
        prompt.text.contains("nothing on disk is deleted"),
        "{}",
        prompt.text
    );
    assert!(
        prompt.text.contains("container is not stopped or deleted"),
        "{}",
        prompt.text
    );
}

#[test]
fn cancel_changes_nothing() {
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    connected(&mut w, &mut state, boxed);
    let before = (w.registry.clone(), w.links.clone(), w.bindings.clone());
    daemons::request_removal(&mut state, &w.model(), boxed);
    daemons::cancel(&mut state);
    assert!(state.prompt().is_none());
    assert_eq!(
        (w.registry.clone(), w.links.clone(), w.bindings.clone()),
        before
    );
    assert!(
        daemons::confirm(&mut state, &mut w.model()).is_empty(),
        "nothing pending"
    );
}

#[test]
fn confirm_ends_its_sessions_removes_the_entry_and_bindings_read_no_daemon() {
    let mut w = World::new();
    let mut state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    connected(&mut w, &mut state, boxed);
    connected(&mut w, &mut state, host);
    let sessions = w.sessions[&boxed].clone();
    daemons::request_removal(&mut state, &w.model(), boxed);
    let out = daemons::confirm(&mut state, &mut w.model());

    assert!(out.contains(&Outcome::DaemonSessionsEnded {
        daemon: boxed,
        sessions: sessions.clone()
    }));
    assert!(out
        .iter()
        .any(|o| matches!(o, Outcome::DaemonRegistryChanged(r) if r.get(boxed).is_none())));
    assert!(w.registry.get(boxed).is_none());
    assert!(w.links.state(boxed).is_none());
    assert_eq!(
        w.links.state(host),
        Some(&DaemonState::Connected),
        "other daemon untouched"
    );
    assert_eq!(state.ended_by_removal(), sessions.as_slice());
    let stored = &w.bindings[std::path::Path::new("/p/a")];
    assert_eq!(
        Binding::resolve(stored, "wt1", &w.registry, None),
        Binding::NoDaemon
    );
    assert!(state.prompt().is_none());
}

#[test]
fn removing_a_stopped_daemon_still_ends_its_sessions_and_touches_no_container() {
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Stopped));
    daemons::request_removal(&mut state, &w.model(), boxed);
    let out = daemons::confirm(&mut state, &mut w.model());
    // The only outcomes are session endings and the registry write: nothing names a container.
    assert!(out.iter().all(|o| matches!(
        o,
        Outcome::DaemonSessionsEnded { .. } | Outcome::DaemonRegistryChanged(_)
    )));
    assert_eq!(state.ended_by_removal().len(), 2);
}

#[test]
fn a_no_daemon_worktree_starts_nothing_until_bound_explicitly() {
    let mut w = World::new();
    let mut state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    connected(&mut w, &mut state, boxed);
    connected(&mut w, &mut state, host);
    daemons::request_removal(&mut state, &w.model(), boxed);
    daemons::confirm(&mut state, &mut w.model());

    let route = daemons::route_for(&w.model(), &wt("/p/b", ""));
    assert!(matches!(route, Route::NoDaemon { .. }), "{route:?}");

    let out = daemons::bind_worktree(&mut w.model(), &wt("/p/b", ""), host).unwrap();
    assert_eq!(
        out,
        vec![Outcome::DaemonBound {
            project: "/p/b".into(),
            worktree: String::new(),
            daemon: host
        }]
    );
    assert_eq!(
        daemons::route_for(&w.model(), &wt("/p/b", "")),
        Route::To(host)
    );
}

#[test]
fn binding_to_an_unknown_daemon_is_refused() {
    let mut w = World::new();
    let gone = micold_core::daemons::DaemonId(99);
    assert!(daemons::bind_worktree(&mut w.model(), &wt("/p/b", ""), gone).is_err());
    assert!(w.bindings["/p/b".as_ref() as &std::path::Path][""] == w.boxed);
}

#[test]
fn editing_a_running_daemons_runtime_asks_the_counted_confirmation() {
    let mut w = World::new();
    let mut state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    connected(&mut w, &mut state, boxed);
    connected(&mut w, &mut state, host);
    daemons::begin_edit(&mut state, &w.model(), boxed).unwrap();
    state.draft.as_mut().unwrap().port = "7010".into();
    let out = daemons::save(&mut state, &mut w.model()).unwrap();
    assert!(out.is_empty(), "nothing applied before the confirmation");
    let prompt = state.prompt().unwrap();
    assert_eq!(prompt.running_sessions, 2);
    assert_eq!(prompt.worktrees_affected, 2);
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Connected));

    let out = daemons::confirm(&mut state, &mut w.model());
    let sessions = w.sessions[&boxed].clone();
    assert!(out.contains(&Outcome::DaemonSessionsEnded {
        daemon: boxed,
        sessions
    }));
    let DaemonRuntime::Container(c) = &w.registry.get(boxed).unwrap().runtime else {
        panic!("container")
    };
    assert_eq!(c.port, 7010);
    assert_eq!(
        w.links.state(boxed),
        Some(&DaemonState::Starting),
        "only its actor restarts"
    );
    assert_eq!(w.links.state(host), Some(&DaemonState::Connected));
    assert!(state.draft.is_none());
}

#[test]
fn a_rename_restarts_nothing_and_asks_nothing() {
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    connected(&mut w, &mut state, boxed);
    daemons::begin_edit(&mut state, &w.model(), boxed).unwrap();
    state.draft.as_mut().unwrap().name = "Renamed".into();
    let out = daemons::save(&mut state, &mut w.model()).unwrap();
    assert!(state.prompt().is_none());
    assert!(out
        .iter()
        .all(|o| matches!(o, Outcome::DaemonRegistryChanged(_))));
    assert_eq!(w.registry.get(boxed).unwrap().name.as_str(), "Renamed");
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Connected));
}
