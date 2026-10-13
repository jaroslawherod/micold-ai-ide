//! The Daemons reducer: list rows, add/edit validation, per-daemon actions, live state, details
//! (feature 491, T030, FR-010, FR-011, FR-015, US3 scenario 4).

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use daemons_fixture::{wt, World};
use micold_client::features::daemons::{self, DraftRuntime, State};
use micold_client::features::Outcome;
use micold_core::daemons::{DaemonEvent, DaemonState, MAX_CONNECTION_ERRORS, MAX_LOG_LINES};

fn add_container(state: &mut State, name: &str, ctr: &str, port: &str) {
    daemons::begin_add(state);
    let draft = state.draft.as_mut().unwrap();
    draft.name = name.into();
    draft.runtime = DraftRuntime::Container;
    draft.container_name = ctr.into();
    draft.port = port.into();
}

#[test]
fn rows_carry_the_same_fields_for_host_and_container() {
    let mut w = World::new();
    let state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    let vm = daemons::views(&state, &w.model());
    assert!(!vm.empty);
    let [h, b] = vm.rows.as_slice() else {
        panic!("two rows")
    };
    assert_eq!(
        (h.id, h.name.as_str(), h.runtime_label.as_str()),
        (host, "Host", "Host")
    );
    assert_eq!(
        (b.id, b.name.as_str(), b.runtime_label.as_str()),
        (boxed, "Box", "Container")
    );
    assert_eq!(h.state, DaemonState::Stopped);
    assert_eq!(h.version, None);
    assert_eq!(h.bound, vec![wt("/p/a", "")]);
    assert_eq!(b.bound, vec![wt("/p/a", "wt1"), wt("/p/b", "")]);
    assert!(
        b.endpoint.contains("7001"),
        "container endpoint names its port: {}",
        b.endpoint
    );
    assert!(!h.endpoint.is_empty());
}

#[test]
fn live_events_update_the_row_and_the_version_is_shown() {
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    daemons::start(&mut state, &mut w.model(), boxed);
    daemons::daemon_event(&mut state, &mut w.links, boxed, DaemonEvent::Connected);
    daemons::version_reported(&mut state, boxed, "0.9.1");
    let vm = daemons::views(&state, &w.model());
    let row = vm.rows.iter().find(|r| r.id == boxed).unwrap();
    assert_eq!(row.state, DaemonState::Connected);
    assert_eq!(row.version.as_deref(), Some("0.9.1"));
    daemons::daemon_event(
        &mut state,
        &mut w.links,
        boxed,
        DaemonEvent::Lost("eof".into()),
    );
    let vm = daemons::views(&state, &w.model());
    let row = vm.rows.iter().find(|r| r.id == boxed).unwrap();
    assert_eq!(
        row.state,
        DaemonState::Unreachable {
            reason: "eof".into()
        }
    );
}

#[test]
fn start_stop_and_reconnect_touch_one_daemon() {
    let mut w = World::new();
    let mut state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    daemons::start(&mut state, &mut w.model(), boxed);
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Starting));
    assert_eq!(w.links.state(host), Some(&DaemonState::Stopped));

    daemons::daemon_event(&mut state, &mut w.links, boxed, DaemonEvent::Connected);
    daemons::daemon_event(
        &mut state,
        &mut w.links,
        boxed,
        DaemonEvent::Lost("x".into()),
    );
    daemons::reconnect(&mut state, &mut w.model(), boxed);
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Starting));
    assert_eq!(w.links.state(host), Some(&DaemonState::Stopped));

    daemons::stop(&mut state, &mut w.model(), boxed);
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Stopped));
}

#[test]
fn blank_name_duplicate_and_second_host_name_their_field() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "  ", "c2", "7002");
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "name");

    add_container(&mut state, "box", "c2", "7002");
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "name", "duplicate ignores case: {}", err.message);

    daemons::begin_add(&mut state);
    let d = state.draft.as_mut().unwrap();
    d.name = "Another host".into();
    d.runtime = DraftRuntime::Host;
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "runtime");
    assert_eq!(w.registry.entries().len(), 2, "a refusal changes nothing");
}

#[test]
fn invalid_port_and_image_name_their_field() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "c3", "not-a-port");
    assert_eq!(
        daemons::save(&mut state, &mut w.model()).unwrap_err().field,
        "port"
    );
    add_container(&mut state, "Three", "c3", "0");
    assert_eq!(
        daemons::save(&mut state, &mut w.model()).unwrap_err().field,
        "port"
    );
    add_container(&mut state, "Three", "c3", "7003");
    state.draft.as_mut().unwrap().profile.image.reference = "::bad ref::".into();
    assert_eq!(
        daemons::save(&mut state, &mut w.model()).unwrap_err().field,
        "image"
    );
}

#[test]
fn a_container_in_use_names_the_other_daemon() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "box-ctr", "7003");
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "container_name");
    assert!(
        err.message.contains("Box"),
        "names the holder: {}",
        err.message
    );
    add_container(&mut state, "Three", "c3", "7001");
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "port");
    assert!(err.message.contains("Box"), "{}", err.message);
}

#[test]
fn a_valid_add_registers_persists_and_tracks_the_daemon() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "c3", "7003");
    let out = daemons::save(&mut state, &mut w.model()).unwrap();
    assert_eq!(w.registry.entries().len(), 3);
    let id = w.registry.entries()[2].id;
    assert!(w.links.state(id).is_some());
    assert!(state.draft.is_none());
    assert!(out
        .iter()
        .any(|o| matches!(o, Outcome::DaemonRegistryChanged(r) if r.entries().len() == 3)));
}

#[test]
fn errors_and_log_are_bounded_and_runtime_missing_carries_its_reason() {
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    for i in 0..(MAX_CONNECTION_ERRORS + 5) {
        daemons::daemon_event(&mut state, &mut w.links, boxed, DaemonEvent::Start);
        daemons::daemon_event(&mut state, &mut w.links, boxed, DaemonEvent::Connected);
        daemons::daemon_event(
            &mut state,
            &mut w.links,
            boxed,
            DaemonEvent::Lost(format!("e{i}")),
        );
    }
    for i in 0..(MAX_LOG_LINES + 3) {
        daemons::log_line(&mut state, boxed, format!("line {i}"));
    }
    let d = daemons::details(&state, boxed);
    assert_eq!(d.errors.len(), MAX_CONNECTION_ERRORS);
    assert_eq!(
        d.errors.last().unwrap().reason,
        format!("e{}", MAX_CONNECTION_ERRORS + 4)
    );
    assert_eq!(d.log.len(), MAX_LOG_LINES);
    assert_eq!(
        d.log.last().unwrap(),
        &format!("line {}", MAX_LOG_LINES + 2)
    );
    assert!(d.runtime_missing.is_none());

    daemons::runtime_missing(&mut state, boxed, "docker: not found on PATH");
    let d = daemons::details(&state, boxed);
    assert_eq!(
        d.runtime_missing.as_deref(),
        Some("docker: not found on PATH")
    );
    assert!(
        daemons::details(&state, w.host).errors.is_empty(),
        "per daemon"
    );
}

#[test]
fn no_daemons_is_an_empty_state_with_nothing_to_pick() {
    let mut w = World::new();
    let mut state = State::default();
    let ids = [w.host, w.boxed];
    for id in ids {
        w.registry.remove(id);
        w.links.remove(id);
    }
    let vm = daemons::views(&state, &w.model());
    assert!(vm.empty);
    assert!(vm.rows.is_empty());
    assert!(vm.bind_choices.is_empty(), "no runtime to pick");
    daemons::begin_add(&mut state);
    assert!(state.draft.is_some(), "the add action is still there");
}

fn fill_ssh(state: &mut State, name: &str, auto_start: bool) {
    daemons::begin_add(state);
    let draft = state.draft.as_mut().unwrap();
    draft.name = name.into();
    draft.runtime = DraftRuntime::Ssh;
    draft.ssh_host = "devbox".into();
    draft.ssh_user = "dev".into();
    draft.ssh_port = "2222".into();
    draft.auto_start = auto_start;
}

/// Feature 489, T020, FR-020: an SSH draft is added, started, shows the copy progress while
/// `Starting`, and once `Connected` shows the version and `user@host:port`.
#[test]
fn an_ssh_daemon_is_added_started_and_shows_progress_then_its_version_and_target() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", true);
    daemons::save(&mut state, &mut w.model()).unwrap();
    let id = w.registry.entries()[2].id;
    assert_eq!(
        w.links.state(id),
        Some(&DaemonState::Starting),
        "auto_start applies to an Ssh entry"
    );

    daemons::update(
        &mut state,
        &mut w.model(),
        daemons::Msg::CopyProgress(id, 40),
    );
    let vm = daemons::views(&state, &w.model());
    let row = vm.rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(row.state, DaemonState::Starting);
    assert_eq!(row.endpoint, "dev@devbox:2222");
    assert!(
        row.activity.as_deref().is_some_and(|a| a.contains("40%")),
        "{:?}",
        row.activity
    );

    daemons::daemon_event(&mut state, &mut w.links, id, DaemonEvent::Connected);
    daemons::version_reported(&mut state, id, "0.9.1");
    let vm = daemons::views(&state, &w.model());
    let row = vm.rows.iter().find(|r| r.id == id).unwrap();
    assert_eq!(row.state, DaemonState::Connected);
    assert_eq!(row.version.as_deref(), Some("0.9.1"));
    assert_eq!(row.endpoint, "dev@devbox:2222");
    assert_eq!(row.activity, None, "no progress once connected");
}

#[test]
fn an_ssh_daemon_without_auto_start_is_added_stopped_and_a_bad_host_names_its_field() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    daemons::save(&mut state, &mut w.model()).unwrap();
    let id = w.registry.entries()[2].id;
    assert_eq!(w.links.state(id), Some(&DaemonState::Stopped));

    daemons::begin_add(&mut state);
    let draft = state.draft.as_mut().unwrap();
    draft.name = "Bad".into();
    draft.runtime = DraftRuntime::Ssh;
    draft.ssh_host = "-oProxyCommand=x".into();
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert!(format!("{err:?}").contains("host"), "{err:?}");
    assert_eq!(w.registry.entries().len(), 3, "nothing was added");
}
