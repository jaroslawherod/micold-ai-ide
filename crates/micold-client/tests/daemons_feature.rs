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
        DaemonState::Reconnecting {
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
    assert!(matches!(
        w.links.state(boxed),
        Some(DaemonState::Reconnecting { .. })
    ));
    // Reconnect restarts the attempts of a reconnecting daemon; it does not start another.
    daemons::reconnect(&mut state, &mut w.model(), boxed);
    assert!(matches!(
        w.links.state(boxed),
        Some(DaemonState::Reconnecting { .. })
    ));
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

// Feature 804, M1: the container settings of the Session service page live in the daemon form.

use micold_core::daemons::DaemonRuntime;
use micold_core::sandbox::image::ImageSourceKind;
use micold_core::sandbox::runtime::RuntimeKind;
use micold_core::sandbox::{CredentialShare, NetworkPosture};

fn profile_of(
    w: &World,
    id: micold_core::daemons::DaemonId,
) -> micold_core::sandbox::SandboxProfile {
    match &w.registry.get(id).unwrap().runtime {
        DaemonRuntime::Container(c) => c.profile.clone(),
        other => panic!("not a container: {other:?}"),
    }
}

fn send(state: &mut State, w: &mut World, msg: daemons::Msg) -> Vec<Outcome> {
    daemons::update(state, &mut w.model(), msg)
}

#[test]
fn profile_fields_round_trip_through_save() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    daemons::begin_edit(&mut state, &w.model(), boxed);
    for msg in [
        Msg::ContainerRuntimeChosen(RuntimeKind::Podman),
        Msg::ImageKindChosen(ImageSourceKind::ImportedFile),
        Msg::ImagePathEdited("/tmp/image.tar".into()),
        Msg::CredentialToggled(CredentialShare::GitConfig, true),
        Msg::NetworkChosen(NetworkPosture::NoOutbound),
        Msg::CpuLimitEdited("1.5".into()),
        Msg::MemoryLimitEdited("2048".into()),
        Msg::PidLimitEdited("600".into()),
        Msg::StorageLimitEdited("4096".into()),
        Msg::SurviveLogoutToggled(true),
    ] {
        send(&mut state, &mut w, msg);
    }
    daemons::save(&mut state, &mut w.model()).unwrap();
    let p = profile_of(&w, boxed);
    assert_eq!(p.runtime, RuntimeKind::Podman);
    assert_eq!(p.image.kind, ImageSourceKind::ImportedFile);
    assert_eq!(
        p.image.path.as_deref(),
        Some(std::path::Path::new("/tmp/image.tar"))
    );
    assert!(p.credentials.contains(&CredentialShare::GitConfig));
    assert_eq!(p.network, NetworkPosture::NoOutbound);
    assert_eq!(p.budget.cpus_milli.map(|c| c.0), Some(1500));
    assert_eq!(p.budget.pids, Some(600));
    assert!(p.budget.memory_bytes.is_some() && p.budget.storage_bytes.is_some());
    assert!(p.survive_logout);

    // Reopened, the form shows what was stored.
    let d = daemons::begin_edit(&mut state, &w.model(), boxed).unwrap();
    assert_eq!(
        (
            d.cpus.as_str(),
            d.memory_mib.as_str(),
            d.pids.as_str(),
            d.storage_mib.as_str()
        ),
        ("1.5", "2048", "600", "4096")
    );
    assert_eq!(d.image_path, "/tmp/image.tar");
}

#[test]
fn a_bad_limit_or_archive_names_its_field_and_changes_nothing() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    let before = profile_of(&w, boxed);
    for (msg, field) in [
        (Msg::CpuLimitEdited("lots".into()), "cpus"),
        (Msg::MemoryLimitEdited("1".into()), "memory"),
        (Msg::PidLimitEdited("x".into()), "pids"),
        (Msg::StorageLimitEdited("1".into()), "storage"),
        (Msg::ImagePathEdited("/tmp/x.tar".into()), "image_path"),
    ] {
        daemons::begin_edit(&mut state, &w.model(), boxed);
        send(&mut state, &mut w, msg);
        send(&mut state, &mut w, Msg::Saved);
        let err = state.form_error.clone().expect("refused");
        assert_eq!(err.field, field, "{}", err.message);
        assert_eq!(profile_of(&w, boxed), before);
    }
}

#[test]
fn two_daemons_edit_independently_and_drafts_are_kept_apart() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    let (host, boxed) = (w.host, w.boxed);
    // A second container daemon.
    add_container(&mut state, "Three", "c3", "7003");
    daemons::save(&mut state, &mut w.model()).unwrap();
    let three = w.registry.entries()[2].id;
    let three_before = profile_of(&w, three).budget.pids;

    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(&mut state, &mut w, Msg::PidLimitEdited("300".into()));
    daemons::save(&mut state, &mut w.model()).unwrap();
    assert_eq!(profile_of(&w, boxed).budget.pids, Some(300));
    assert_eq!(profile_of(&w, three).budget.pids, three_before, "FR-003");

    // An unsaved draft of one form does not show in the other.
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(&mut state, &mut w, Msg::CpuLimitEdited("2".into()));
    let d = daemons::begin_edit(&mut state, &w.model(), three).unwrap();
    assert_eq!(d.cpus, "");
    assert_eq!(
        d.pids,
        three_before.map(|p| p.to_string()).unwrap_or_default()
    );
    let _ = host;

    // Renaming keeps the profile; removing discards it.
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(&mut state, &mut w, Msg::NameEdited("Renamed".into()));
    daemons::save(&mut state, &mut w.model()).unwrap();
    assert_eq!(profile_of(&w, boxed).budget.pids, Some(300));
    daemons::request_removal(&mut state, &w.model(), boxed);
    daemons::confirm(&mut state, &mut w.model());
    assert!(w.registry.get(boxed).is_none());
}

#[test]
fn a_running_daemon_asks_before_new_container_settings_apply() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    let pids_before = profile_of(&w, boxed).budget.pids;
    daemons::start(&mut state, &mut w.model(), boxed);
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(&mut state, &mut w, Msg::PidLimitEdited("300".into()));
    let out = send(&mut state, &mut w, Msg::Saved);
    assert!(out.is_empty());
    assert!(state.prompt().is_some(), "FR-004, FR-010");
    assert_eq!(
        profile_of(&w, boxed).budget.pids,
        pids_before,
        "not applied yet"
    );
    send(&mut state, &mut w, Msg::RemovalConfirmed);
    assert_eq!(profile_of(&w, boxed).budget.pids, Some(300));
}

#[test]
fn an_ssh_entry_shows_no_container_profile_and_switching_to_ssh_drops_it() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    daemons::save(&mut state, &mut w.model()).unwrap();
    let ssh = w.registry.entries()[2].id;

    // Unedited: opened and saved again, the entry is as it was.
    let before = w.registry.get(ssh).unwrap().clone();
    daemons::begin_edit(&mut state, &w.model(), ssh);
    send(&mut state, &mut w, Msg::Saved);
    assert_eq!(w.registry.get(ssh).unwrap(), &before);

    // A container daemon switched to SSH and saved holds no container settings.
    let boxed = w.boxed;
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(&mut state, &mut w, Msg::PidLimitEdited("300".into()));
    send(&mut state, &mut w, Msg::RuntimeChosen(DraftRuntime::Ssh));
    send(&mut state, &mut w, Msg::SshHostEdited("devbox".into()));
    daemons::save(&mut state, &mut w.model()).unwrap();
    assert!(matches!(
        w.registry.get(boxed).unwrap().runtime,
        DaemonRuntime::Ssh(_)
    ));
}

/// Feature 489, T036, FR-012: the repository step refuses a relative path and an unconnected
/// daemon with a sentence under the field, and a connected daemon's absolute path becomes the
/// identity of a remote project.
#[test]
fn the_repository_step_refuses_what_it_cannot_ask_and_names_a_connected_daemons_path() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    daemons::save(&mut state, &mut w.model()).unwrap();
    let id = w.registry.entries()[2].id;

    let set = |state: &mut State, path: &str| {
        state.repositories.insert(id, path.to_string());
    };
    set(&mut state, "relative/repo");
    let out = daemons::add_repository(&mut state, &w.model(), id);
    assert!(out.is_empty());
    assert!(state.repository_error.is_some(), "a relative path is named");

    set(&mut state, "/home/dev/repo");
    let out = daemons::add_repository(&mut state, &w.model(), id);
    assert!(out.is_empty(), "the daemon is not connected");
    let (for_id, message) = state.repository_error.clone().unwrap();
    assert_eq!(for_id, id);
    assert!(message.contains("not connected"), "{message}");

    daemons::daemon_event(&mut state, &mut w.links, id, DaemonEvent::Start);
    daemons::daemon_event(&mut state, &mut w.links, id, DaemonEvent::Connected);
    let out = daemons::add_repository(&mut state, &w.model(), id);
    let [Outcome::RemoteRepositoryRequested { path }] = out.as_slice() else {
        panic!("one request: {out:?}")
    };
    assert_eq!(
        micold_core::ssh::RemoteLocation::parse(path)
            .unwrap()
            .daemon,
        id
    );
    assert!(path.to_string_lossy().ends_with("/home/dev/repo"));
    assert!(state.repository_error.is_none());
    assert!(!state.repositories.contains_key(&id), "the field empties");
}

// Feature 804, M2: the Daemons rail row's Sharing badge follows the daemons, from any section.

#[test]
fn the_sharing_badge_follows_the_stored_daemons_and_the_open_form() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    assert!(
        !daemons::views(&state, &w.model()).shares_credentials,
        "no daemon shares, so no badge"
    );

    // The open form counts before it is saved.
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(
        &mut state,
        &mut w,
        Msg::CredentialToggled(CredentialShare::GitConfig, true),
    );
    assert!(daemons::views(&state, &w.model()).shares_credentials);

    // Saved, it counts with the form closed.
    send(&mut state, &mut w, Msg::Saved);
    state.draft = None;
    assert!(
        daemons::views(&state, &w.model()).shares_credentials,
        "a stored daemon shares, so the badge stays"
    );

    // The form replaces its own daemon's stored shares: unticking in the form clears the badge.
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(
        &mut state,
        &mut w,
        Msg::CredentialToggled(CredentialShare::GitConfig, false),
    );
    assert!(!daemons::views(&state, &w.model()).shares_credentials);
}

/// FR-012, FR-014, FR-015: a form opens with Container only; a toggle flips one section and touches
/// neither the draft nor the saved settings; a section holding the refused field is open.
#[test]
fn sections_open_by_toggle_and_by_error_without_touching_the_draft() {
    use daemons::Section;
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Sec", "sec-ctr", "7050");
    let open = |s: &State| Section::ALL.map(|x| s.section_open(x));
    assert_eq!(open(&state), [true, false, false, false, false]);

    let draft = state.draft.clone();
    let saved = w.registry.clone();
    send(
        &mut state,
        &mut w,
        daemons::Msg::SectionToggled(Section::Network),
    );
    assert_eq!(open(&state), [true, false, true, false, false]);
    send(
        &mut state,
        &mut w,
        daemons::Msg::SectionToggled(Section::Container),
    );
    assert_eq!(open(&state), [false, false, true, false, false]);
    send(
        &mut state,
        &mut w,
        daemons::Msg::SectionToggled(Section::Network),
    );
    assert_eq!(open(&state), [false; 5]);
    assert_eq!(state.draft, draft, "toggling must not edit the draft");
    assert_eq!(w.registry, saved, "toggling must not touch saved settings");

    state.draft.as_mut().unwrap().cpus = "many".into();
    send(&mut state, &mut w, daemons::Msg::Saved);
    assert_eq!(state.form_error.as_ref().map(|e| e.field), Some("cpus"));
    assert_eq!(
        open(&state),
        [false, false, false, true, false],
        "the section holding the error is open"
    );

    send(&mut state, &mut w, daemons::Msg::DraftDiscarded);
    send(&mut state, &mut w, daemons::Msg::AddOpened);
    assert_eq!(open(&state), [true, false, false, false, false]);
}

// Feature 805: a container daemon saved with an empty port gets a free one, kept from then on.

fn stored_port(w: &World, name: &str) -> u16 {
    let e = w
        .registry
        .entries()
        .iter()
        .find(|e| e.name.as_str() == name);
    match &e.expect("stored").runtime {
        micold_core::daemons::DaemonRuntime::Container(c) => c.port,
        other => panic!("not a container: {other:?}"),
    }
}

fn sequence(ports: Vec<u16>) -> impl Fn() -> std::io::Result<u16> {
    let it = std::cell::RefCell::new(ports.into_iter());
    move || {
        it.borrow_mut()
            .next()
            .ok_or_else(|| std::io::Error::other("empty"))
    }
}

#[test]
fn an_empty_port_is_picked_stored_and_shown_on_edit() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "c3", "");
    daemons::save(&mut state, &mut w.model()).unwrap();
    let port = stored_port(&w, "Three");
    assert_ne!(port, 0);
    let others = w.registry.container_ports();
    assert_eq!(
        others.iter().filter(|p| **p == port).count(),
        1,
        "not another daemon's port: {others:?}"
    );
    let id = w.registry.entries().last().unwrap().id;
    let d = daemons::begin_edit(&mut state, &w.model(), id).unwrap();
    assert_eq!(
        d.port,
        port.to_string(),
        "the edit view shows the picked port"
    );
}

#[test]
fn an_explicit_port_is_stored_as_given() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "c3", " 7003 ");
    daemons::save(&mut state, &mut w.model()).unwrap();
    assert_eq!(stored_port(&w, "Three"), 7003);
}

#[test]
fn a_zero_port_is_still_refused() {
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "c3", "0");
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(
        (err.field, err.message.as_str()),
        ("port", "Enter a port between 1 and 65535.")
    );
}

#[test]
fn an_explicit_duplicate_port_is_still_refused_as_a_duplicate() {
    let mut w = World::new();
    let mut state = State::default();
    let taken = w.registry.container_ports()[0];
    add_container(&mut state, "Three", "c3", &taken.to_string());
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "port");
    assert_ne!(
        err.message, "Enter a port between 1 and 65535.",
        "a duplicate is not reported as an invalid number"
    );
    assert!(
        err.message.contains(&taken.to_string()) || err.message.to_lowercase().contains("use"),
        "names the clash: {}",
        err.message
    );
    assert_eq!(w.registry.entries().len(), 2, "nothing was stored");
}

#[test]
fn a_host_daemon_edited_to_container_with_an_empty_port_gets_a_picked_one() {
    let mut w = World::new();
    let mut state = State::default();
    let host = w.host;
    daemons::begin_edit(&mut state, &w.model(), host);
    let draft = state.draft.as_mut().unwrap();
    draft.runtime = DraftRuntime::Container;
    draft.container_name = "host-ctr".into();
    draft.port = String::new();
    let next = sequence(vec![8123]);
    daemons::save(&mut state, &mut w.model_with(Vec::new(), &next)).unwrap();
    assert_eq!(stored_port(&w, "Host"), 8123);
}

#[test]
fn two_empty_adds_get_different_ports_even_when_the_source_repeats() {
    let mut w = World::new();
    let mut state = State::default();
    let next = sequence(vec![8001, 8001, 8002]);
    add_container(&mut state, "Three", "c3", "");
    daemons::save(&mut state, &mut w.model_with(Vec::new(), &next)).unwrap();
    add_container(&mut state, "Four", "c4", "");
    daemons::save(&mut state, &mut w.model_with(Vec::new(), &next)).unwrap();
    assert_eq!(stored_port(&w, "Three"), 8001);
    assert_eq!(stored_port(&w, "Four"), 8002);
}

#[test]
fn a_port_held_elsewhere_is_never_picked() {
    let mut w = World::new();
    let mut state = State::default();
    let next = sequence(vec![8001, 8002]);
    add_container(&mut state, "Three", "c3", "");
    daemons::save(&mut state, &mut w.model_with(vec![8001], &next)).unwrap();
    assert_eq!(stored_port(&w, "Three"), 8002);
}

#[test]
fn no_free_port_fails_the_save_and_stores_nothing() {
    let mut w = World::new();
    let mut state = State::default();
    let next = || Err(std::io::Error::other("no sockets"));
    add_container(&mut state, "Three", "c3", "");
    let err = daemons::save(&mut state, &mut w.model_with(Vec::new(), &next)).unwrap_err();
    assert_eq!(
        (err.field, err.message.as_str()),
        ("port", "No free port could be found; enter one yourself.")
    );
    assert_eq!(w.registry.entries().len(), 2, "the registry is unchanged");
}

fn rename_boxed(w: &mut World, clear_port: bool) {
    let mut state = State::default();
    let boxed = w.boxed;
    daemons::begin_edit(&mut state, &w.model(), boxed);
    let draft = state.draft.as_mut().unwrap();
    draft.name = "Box renamed".into();
    if clear_port {
        draft.port = String::new();
    }
    let next = sequence(vec![9999]);
    daemons::save(&mut state, &mut w.model_with(Vec::new(), &next)).unwrap();
}

#[test]
fn editing_with_the_port_cleared_keeps_the_stored_port() {
    let mut w = World::new();
    let before = stored_port(&w, "Box");
    rename_boxed(&mut w, true);
    assert_eq!(stored_port(&w, "Box renamed"), before);
}

#[test]
fn editing_with_the_port_untouched_keeps_the_stored_port() {
    let mut w = World::new();
    let before = stored_port(&w, "Box");
    rename_boxed(&mut w, false);
    assert_eq!(stored_port(&w, "Box renamed"), before);
}

#[test]
fn a_picked_port_survives_a_restart() {
    use micold_client::links::{bring_up, BringUp};
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Three", "c3", "");
    daemons::save(&mut state, &mut w.model()).unwrap();
    let port = stored_port(&w, "Three");
    let entries: Vec<_> = w
        .registry
        .entries()
        .iter()
        .map(|e| serde_json::from_str(&serde_json::to_string(e).unwrap()).unwrap())
        .collect();
    let restarted = micold_core::daemons::DaemonRegistry::new(entries, w.registry.next_id());
    let entry = restarted.entries().last().unwrap();
    assert_eq!(restarted.container_ports().last(), Some(&port));
    assert_eq!(
        bring_up(entry, &DaemonState::Starting, false),
        BringUp::Spawn,
        "a restarted daemon is brought up"
    );
    let links = micold_client::links::DaemonLinks::at_launch(&restarted);
    let specs = micold_client::daemon::actor_specs(
        &restarted,
        links.states(),
        std::path::Path::new("/state"),
        None,
        false,
    );
    let dialled: Vec<u16> = specs
        .iter()
        .filter_map(|s| match &s.dial {
            micold_client::daemon::Dial::Container { port, .. } => Some(*port),
            _ => None,
        })
        .collect();
    assert!(
        dialled.contains(&port),
        "the start dials the stored port {port}: {dialled:?}"
    );
}
