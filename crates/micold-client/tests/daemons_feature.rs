//! The Daemons reducer: list rows, add/edit validation, per-daemon actions, live state, details
//! (feature 491, T030, FR-010, FR-011, FR-015, US3 scenario 4).

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use daemons_fixture::{wt, World};
use micold_client::features::daemons::{self, DraftRuntime, State};
use micold_client::features::Outcome;
use micold_core::daemons::{DaemonEvent, DaemonState, MAX_CONNECTION_ERRORS, MAX_LOG_LINES};
use micold_core::provision::{ProvisionEvent, Step};

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
    // A blank name is the required error on the field, not a refusal with a sentence (feature 937).
    assert!(daemons::save(&mut state, &mut w.model())
        .unwrap()
        .is_empty());
    assert_eq!(w.registry.entries().len(), 2, "nothing is added");

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
    send_own_reference(&mut state, "::bad ref::");
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
        daemons::Msg::Provisioning(id, ProvisionEvent::Entered(Step::Copy)),
    );
    daemons::update(
        &mut state,
        &mut w.model(),
        daemons::Msg::Provisioning(id, ProvisionEvent::CopyPercent(40)),
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
use micold_core::sandbox::image::{ImageChoice, OwnImage};
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

/// Choose "My own image" and type `reference` in its field.
fn send_own_reference(state: &mut State, reference: &str) {
    let draft = state.draft.as_mut().unwrap();
    draft.profile.image = ImageChoice::Own(OwnImage::Reference(String::new()));
    draft.image_reference = reference.to_string();
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
        Msg::ImageKindChosen(daemons::ImageKind::Own),
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
    assert_eq!(
        p.image,
        ImageChoice::Own(OwnImage::Archive("/tmp/image.tar".into()))
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
    ] {
        daemons::begin_edit(&mut state, &w.model(), boxed);
        send(&mut state, &mut w, msg);
        send(&mut state, &mut w, Msg::Saved);
        let err = state.form_error.clone().expect("refused");
        assert_eq!(err.field, field, "{}", err.message);
        assert_eq!(profile_of(&w, boxed), before);
    }
    // "My own image" with nothing filled in is the required error on the image field.
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(
        &mut state,
        &mut w,
        Msg::ImageKindChosen(daemons::ImageKind::Own),
    );
    send(&mut state, &mut w, Msg::Saved);
    assert!(state.form_error.is_none());
    assert_eq!(profile_of(&w, boxed), before);
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
    send(&mut state, &mut w, Msg::CpuLimitEdited("3".into()));
    let d = daemons::begin_edit(&mut state, &w.model(), three).unwrap();
    assert_eq!(d.cpus, "2", "its own saved default, not the other form's 3");
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
    let text = state.prompt().expect("FR-004, FR-010").text.clone();
    assert!(text.contains("2 running sessions"), "{text}");
    assert_eq!(
        profile_of(&w, boxed).budget.pids,
        pids_before,
        "not applied yet"
    );
    send(&mut state, &mut w, Msg::RemovalConfirmed);
    assert_eq!(profile_of(&w, boxed).budget.pids, Some(300));
}

/// FR-014, US2 #4: the restart confirmation is the same on "My own image" as on the basic image.
#[test]
fn a_running_daemon_on_its_own_image_asks_before_new_limits_apply() {
    use daemons::Msg;
    let mut w = World::new();
    let mut state = State::default();
    let boxed = w.boxed;
    let before = profile_of(&w, boxed).budget.memory_bytes;
    daemons::start(&mut state, &mut w.model(), boxed);
    daemons::begin_edit(&mut state, &w.model(), boxed);
    send(
        &mut state,
        &mut w,
        Msg::ImageKindChosen(daemons::ImageKind::Own),
    );
    send(
        &mut state,
        &mut w,
        Msg::ImageEdited("registry.example/own:1".into()),
    );
    send(&mut state, &mut w, Msg::MemoryLimitEdited("6144".into()));
    let out = send(&mut state, &mut w, Msg::Saved);
    assert!(out.is_empty(), "{out:?}");
    assert!(state.prompt().is_some());
    assert_eq!(profile_of(&w, boxed).budget.memory_bytes, before);
    send(&mut state, &mut w, Msg::RemovalConfirmed);
    assert_ne!(profile_of(&w, boxed).budget.memory_bytes, before);
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
    state
        .repository_marks
        .leave(micold_client::features::window::FieldId::DaemonRepository(
            id,
        ));
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
    let field = micold_client::features::window::FieldId::DaemonRepository(id);
    let (_, emptied) = state.repository_field(id);
    assert_eq!(
        state.repository_marks.state(field, emptied),
        micold_core::required::FieldRequirement::Required,
        "a successful add empties the field without marking it (feature 937)"
    );
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

/// Feature 890 FR-004, FR-005: a form opens with `Connection` only; a toggle flips one section and
/// touches neither the draft nor the saved settings; a section holding the refused field is open.
#[test]
fn sections_open_by_toggle_and_by_error_without_touching_the_draft() {
    use daemons::Section;
    let mut w = World::new();
    let mut state = State::default();
    add_container(&mut state, "Sec", "sec-ctr", "7050");
    let open = |s: &State| -> Vec<bool> {
        Section::of(DraftRuntime::Container)
            .iter()
            .map(|x| s.section_open(*x))
            .collect()
    };
    assert_eq!(open(&state), vec![true, false, false, false, false, false]);

    let draft = state.draft.clone();
    let saved = w.registry.clone();
    let toggle = |state: &mut State, w: &mut World, s| {
        send(state, w, daemons::Msg::SectionToggled(s));
    };
    toggle(&mut state, &mut w, Section::Network);
    assert_eq!(open(&state), vec![true, false, false, true, false, false]);
    toggle(&mut state, &mut w, Section::Connection);
    assert_eq!(open(&state), vec![false, false, false, true, false, false]);
    toggle(&mut state, &mut w, Section::Network);
    assert_eq!(open(&state), vec![false; 6]);
    assert_eq!(state.draft, draft, "toggling must not edit the draft");
    assert_eq!(w.registry, saved, "toggling must not touch saved settings");

    state.draft.as_mut().unwrap().cpus = "many".into();
    send(&mut state, &mut w, daemons::Msg::Saved);
    assert_eq!(state.form_error.as_ref().map(|e| e.field), Some("cpus"));
    assert_eq!(
        open(&state),
        vec![false, false, false, false, true, false],
        "the section holding the error is open"
    );

    // Discarding the form and opening a new one forgets what the user flipped.
    send(&mut state, &mut w, daemons::Msg::DraftDiscarded);
    send(&mut state, &mut w, daemons::Msg::AddOpened);
    assert_eq!(
        open(&state),
        vec![true, false, false, false, false, false],
        "a reopened form is back to Connection only"
    );
}

/// FR-004, US1 AS2: every kind opens on its first section only, for an add and for an edit; a
/// change of kind drops what the user had flipped.
#[test]
fn every_kind_opens_on_connection_only_and_a_kind_change_resets() {
    use daemons::Section;
    let only_connection = |state: &State, runtime| {
        Section::of(runtime)
            .iter()
            .map(|s| (*s, state.section_open(*s)))
            .all(|(s, open)| open == (s == Section::Connection))
    };
    let mut w = World::new();
    let mut state = State::default();
    for runtime in [
        DraftRuntime::Container,
        DraftRuntime::Kubernetes,
        DraftRuntime::Ssh,
        DraftRuntime::Host,
    ] {
        daemons::begin_add(&mut state);
        send(&mut state, &mut w, daemons::Msg::RuntimeChosen(runtime));
        assert!(only_connection(&state, runtime), "{runtime:?} add");
    }
    // Stored daemons of each kind, opened for edit.
    add_container(&mut state, "C", "c-ctr", "7051");
    daemons::save(&mut state, &mut w.model()).unwrap();
    fill_ssh(&mut state, "S", false);
    daemons::save(&mut state, &mut w.model()).unwrap();
    daemons::begin_add(&mut state);
    state.draft.as_mut().unwrap().name = "K".into();
    send(
        &mut state,
        &mut w,
        daemons::Msg::RuntimeChosen(DraftRuntime::Kubernetes),
    );
    let kube = &mut state.draft.as_mut().unwrap().kube;
    kube.context = "ctx".into();
    kube.namespace = "ns".into();
    daemons::save(&mut state, &mut w.model()).unwrap();
    for entry in w.registry.entries().to_vec() {
        daemons::begin_edit(&mut state, &w.model(), entry.id);
        let runtime = state.draft.as_ref().unwrap().runtime;
        assert!(only_connection(&state, runtime), "{runtime:?} edit");
    }
    // Flip a section, then change the kind: the new kind starts from its default.
    daemons::begin_add(&mut state);
    send(
        &mut state,
        &mut w,
        daemons::Msg::SectionToggled(Section::Sessions),
    );
    send(
        &mut state,
        &mut w,
        daemons::Msg::RuntimeChosen(DraftRuntime::Host),
    );
    assert!(only_connection(&state, DraftRuntime::Host));
}

/// FR-004, US1 AS4: a refused field in a closed section of any kind opens that section, and the
/// section falls back to the user's choice once the error is gone.
#[test]
fn a_refused_field_opens_its_section_in_every_kind() {
    use daemons::{FormError, Section};
    let mut w = World::new();
    let mut state = State::default();
    let cases = [
        (DraftRuntime::Container, "cpus", Section::Resources),
        (DraftRuntime::Kubernetes, "cpu_limit", Section::Resources),
        (
            DraftRuntime::Kubernetes,
            "node_selector",
            Section::Placement,
        ),
        (DraftRuntime::Ssh, "host", Section::Connection),
    ];
    for (runtime, field, section) in cases {
        daemons::begin_add(&mut state);
        send(&mut state, &mut w, daemons::Msg::RuntimeChosen(runtime));
        // The user closes Connection, so only the error can open a section there.
        if section == Section::Connection {
            send(
                &mut state,
                &mut w,
                daemons::Msg::SectionToggled(Section::Connection),
            );
        }
        assert!(!state.section_open(section));
        state.form_error = Some(FormError {
            field,
            message: "refused".into(),
        });
        for other in Section::of(runtime) {
            assert_eq!(
                state.section_open(*other),
                *other == section || *other == Section::Connection,
                "{runtime:?} {field} {other:?}"
            );
        }
        state.form_error = None;
        assert!(!state.section_open(section), "{field}: falls back");
    }
}

/// FR-004, clarification D3: a required field left empty keeps `Connection` open after the user
/// closed it; filling it lets the user's choice stand.
#[test]
fn a_required_field_left_empty_keeps_connection_open() {
    use daemons::Section;
    let mut w = World::new();
    let mut state = State::default();
    for runtime in [
        DraftRuntime::Container,
        DraftRuntime::Kubernetes,
        DraftRuntime::Ssh,
    ] {
        daemons::begin_add(&mut state);
        send(&mut state, &mut w, daemons::Msg::RuntimeChosen(runtime));
        send(
            &mut state,
            &mut w,
            daemons::Msg::SectionToggled(Section::Connection),
        );
        assert!(!state.section_open(Section::Connection), "{runtime:?}");
        // A refused save marks every empty required field missing at once.
        send(&mut state, &mut w, daemons::Msg::Saved);
        assert!(
            state.section_open(Section::Connection),
            "{runtime:?}: the empty required field opens its section"
        );
    }
}

/// Review A (M2): a click on a section an error holds open is not recorded, so the section is
/// neither a dead header now nor snapped shut under the user's cursor once the error is gone.
#[test]
fn a_click_on_a_section_an_error_holds_open_is_not_recorded() {
    use daemons::{FormError, Section};
    let mut w = World::new();
    let mut state = State::default();
    daemons::begin_add(&mut state);
    send(
        &mut state,
        &mut w,
        daemons::Msg::RuntimeChosen(DraftRuntime::Kubernetes),
    );
    for (field, section) in [
        ("cpu_limit", Section::Resources),
        ("namespace", Section::Connection),
    ] {
        state.form_error = Some(FormError {
            field,
            message: "refused".into(),
        });
        send(&mut state, &mut w, daemons::Msg::SectionToggled(section));
        assert!(state.section_open(section), "{field}: the click is refused");
        let default_open = section == Section::Connection;
        state.form_error = None;
        assert_eq!(
            state.section_open(section),
            default_open,
            "{field}: no stale flip"
        );
    }
}

/// Review A (M2): the Kubernetes "one key, one pull file" refusal names `allowed`; it opens
/// Credentials, where it is shown.
#[test]
fn the_allowed_credentials_refusal_opens_credentials() {
    use daemons::{FormError, Section};
    let mut w = World::new();
    let mut state = State::default();
    daemons::begin_add(&mut state);
    send(
        &mut state,
        &mut w,
        daemons::Msg::RuntimeChosen(DraftRuntime::Kubernetes),
    );
    assert!(!state.section_open(Section::Credentials));
    state.form_error = Some(FormError {
        field: "allowed",
        message: "refused".into(),
    });
    assert!(state.section_open(Section::Credentials));
}

/// Review A (M2): choosing the kind already chosen leaves the sections as the user set them.
#[test]
fn choosing_the_same_kind_again_keeps_the_sections_the_user_opened() {
    use daemons::Section;
    let mut w = World::new();
    let mut state = State::default();
    daemons::begin_add(&mut state);
    send(
        &mut state,
        &mut w,
        daemons::Msg::SectionToggled(Section::Resources),
    );
    assert!(state.section_open(Section::Resources));
    send(
        &mut state,
        &mut w,
        daemons::Msg::RuntimeChosen(DraftRuntime::Container),
    );
    assert!(state.section_open(Section::Resources));
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

// Feature 489, M5 (T058): the host picker, resolved duplicates and the management actions.

use micold_core::ssh::config_hosts::parse_g;
use micold_core::ssh::SshTarget;

#[test]
fn choosing_a_host_from_the_picker_fills_only_the_host_and_clears_the_other_fields() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    send(
        &mut state,
        &mut w,
        daemons::Msg::SshHostsLoaded(vec!["build".into(), "web".into()]),
    );
    assert_eq!(
        state.ssh_hosts,
        ["build", "web"],
        "the picker lists the configured hosts"
    );
    send(
        &mut state,
        &mut w,
        daemons::Msg::SshHostChosen("web".into()),
    );
    let d = state.draft.as_ref().unwrap();
    assert_eq!(d.ssh_host, "web");
    assert_eq!(
        (
            d.ssh_user.as_str(),
            d.ssh_port.as_str(),
            d.ssh_key_file.as_str()
        ),
        ("", "", ""),
        "user, port and key come from the SSH configuration"
    );
    daemons::save(&mut state, &mut w.model()).unwrap();
    let entry = w.registry.entries().last().unwrap();
    assert_eq!(entry.runtime.label(), "Remote over SSH");
}

#[test]
fn entered_fields_are_stored_as_typed_for_a_host_not_in_the_configuration() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    state.draft.as_mut().unwrap().ssh_key_file = String::new();
    daemons::save(&mut state, &mut w.model()).unwrap();
    let entry = w.registry.entries().last().unwrap();
    match &entry.runtime {
        DaemonRuntime::Ssh(s) => {
            assert_eq!(
                (
                    s.target.host.as_str(),
                    s.target.user.as_deref(),
                    s.target.port
                ),
                ("devbox", Some("dev"), Some(2222))
            );
        }
        other => panic!("not ssh: {other:?}"),
    }
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
    );
    let dialled: Vec<u16> = specs
        .iter()
        .filter_map(|s| match &s.dial {
            micold_client::daemon::Dial::Container { port, .. }
            | micold_client::daemon::Dial::ContainerSsh { port, .. } => Some(*port),
            _ => None,
        })
        .collect();
    assert!(
        dialled.contains(&port),
        "the start dials the stored port {port}: {dialled:?}"
    );
}

/// FR-004: the question counts the daemon's running sessions, one and none included.
#[test]
fn the_running_daemon_question_counts_its_sessions() {
    use daemons::Msg;
    for (live, expected) in [(0, "0 running sessions"), (1, "1 running session ")] {
        let mut w = World::new();
        let mut state = State::default();
        let boxed = w.boxed;
        w.sessions.insert(
            boxed,
            (0..live)
                .map(|_| micold_core::session::SessionId::new())
                .collect(),
        );
        daemons::start(&mut state, &mut w.model(), boxed);
        daemons::begin_edit(&mut state, &w.model(), boxed);
        send(&mut state, &mut w, Msg::PidLimitEdited("300".into()));
        send(&mut state, &mut w, Msg::Saved);
        let text = state.prompt().expect("a running daemon asks").text.clone();
        assert!(text.contains(expected), "{text}");
    }
}

/// FR-004: every refused field of the Container and Resources sections opens its section, so the
/// error is never hidden in a closed one.
#[test]
fn every_section_field_error_opens_its_section() {
    use daemons::{FormError, Section};
    for (field, section) in [
        ("image", Section::Container),
        ("image_path", Section::Container),
        ("cpus", Section::Resources),
        ("memory", Section::Resources),
        ("pids", Section::Resources),
        ("storage", Section::Resources),
    ] {
        let mut state = State::default();
        add_container(&mut state, "Sec", "sec-ctr", "7050");
        assert!(
            !state.section_open(section),
            "{field}: closed to begin with"
        );
        state.form_error = Some(FormError {
            field,
            message: "refused".into(),
        });
        for other in Section::of(DraftRuntime::Container) {
            let expected = *other == section || *other == Section::Connection;
            assert_eq!(state.section_open(*other), expected, "{field}");
        }
    }
}

/// Feature 806, T016, FR-018, FR-022: a start step and the copy percent are one daemon's own; a
/// failure makes only that daemon unreachable, with the shared reason in its recent errors.
#[test]
fn provisioning_events_and_a_start_failure_belong_to_one_daemon() {
    use micold_core::provision::{FailureCause, ProvisionFailure};
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "First", true);
    daemons::save(&mut state, &mut w.model()).unwrap();
    fill_ssh(&mut state, "Second", true);
    // Another port: the same host, user and port is one daemon (FR-016).
    state.draft.as_mut().unwrap().ssh_port = "2223".into();
    daemons::save(&mut state, &mut w.model()).unwrap();
    let (a, b) = (w.registry.entries()[2].id, w.registry.entries()[3].id);

    for (id, event) in [
        (a, ProvisionEvent::Entered(Step::Copy)),
        (a, ProvisionEvent::CopyPercent(40)),
        (b, ProvisionEvent::Entered(Step::Probe)),
    ] {
        daemons::update(
            &mut state,
            &mut w.model(),
            daemons::Msg::Provisioning(id, event),
        );
    }
    let activity = |state: &State, w: &mut World, id| {
        let vm = daemons::views(state, &w.model());
        vm.rows
            .iter()
            .find(|r| r.id == id)
            .unwrap()
            .activity
            .clone()
    };
    assert_eq!(
        activity(&state, &mut w, a).as_deref(),
        Some("copying the daemon (40%)")
    );
    assert_eq!(
        activity(&state, &mut w, b).as_deref(),
        Some("checking the daemon build")
    );

    let ctx = micold_core::ssh::ReasonContext {
        host: "devbox".into(),
        port: 2222,
        timeout_secs: 15,
        platform: micold_core::ssh::ClientPlatform::Linux,
    };
    let failure = ProvisionFailure::new(Step::Copy, FailureCause::CopyFailed)
        .with_output("cat: No space left on device");
    let reason = failure.reason(&ctx);
    daemons::daemon_event(
        &mut state,
        &mut w.links,
        a,
        DaemonEvent::StartFailed(reason.clone()),
    );

    assert_eq!(
        w.links.state(a),
        Some(&DaemonState::Unreachable {
            reason: reason.clone()
        })
    );
    assert_eq!(
        w.links.state(b),
        Some(&DaemonState::Starting),
        "the other is untouched"
    );
    assert_eq!(
        daemons::details(&state, a).errors.last().unwrap().reason,
        reason
    );
    assert!(reason.contains("No space left on device"));
    assert!(daemons::details(&state, b).errors.is_empty());
    assert_eq!(
        activity(&state, &mut w, a),
        None,
        "a failed start shows no step"
    );
}

#[test]
fn a_second_alias_of_the_same_machine_is_refused_naming_the_first_daemon() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "First", false);
    state.draft.as_mut().unwrap().ssh_host = "a".into();
    state.draft.as_mut().unwrap().ssh_user = String::new();
    state.draft.as_mut().unwrap().ssh_port = String::new();
    let same = parse_g("hostname 10.0.0.1\nuser me\nport 22\n");
    for host in ["a", "b"] {
        let target = SshTarget::new(host, None, None, None).unwrap();
        send(
            &mut state,
            &mut w,
            daemons::Msg::SshResolved(target, same.clone()),
        );
    }
    daemons::save(&mut state, &mut w.model()).unwrap();

    fill_ssh(&mut state, "Second", false);
    let d = state.draft.as_mut().unwrap();
    d.ssh_host = "b".into();
    d.ssh_user = String::new();
    d.ssh_port = String::new();
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "host");
    assert_eq!(err.message, "b is already used by daemon First.");
    assert_eq!(w.registry.entries().len(), 3, "nothing was added");
}

#[test]
fn a_bad_port_names_its_field() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    state.draft.as_mut().unwrap().ssh_port = "70000".into();
    let err = daemons::save(&mut state, &mut w.model()).unwrap_err();
    assert_eq!(err.field, "port");
}

#[test]
fn removing_an_ssh_daemon_asks_first_and_applies_to_that_daemon_only() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", false);
    daemons::save(&mut state, &mut w.model()).unwrap();
    let id = w.registry.entries().last().unwrap().id;
    let before = w.registry.entries().len();
    send(&mut state, &mut w, daemons::Msg::RemovalRequested(id));
    assert!(
        state.prompt().is_some(),
        "the 491 confirmation applies unchanged"
    );
    assert_eq!(
        w.registry.entries().len(),
        before,
        "nothing removed before confirming"
    );
    send(&mut state, &mut w, daemons::Msg::RemovalConfirmed);
    assert_eq!(w.registry.entries().len(), before - 1);
    assert!(w.registry.entries().iter().all(|e| e.id != id));
}

#[test]
fn recent_errors_of_an_ssh_daemon_are_listed_with_the_named_reason() {
    let mut w = World::new();
    let mut state = State::default();
    fill_ssh(&mut state, "Remote", true);
    daemons::save(&mut state, &mut w.model()).unwrap();
    let id = w.registry.entries().last().unwrap().id;
    daemons::daemon_event(
        &mut state,
        &mut w.links,
        id,
        DaemonEvent::StartFailed("host key for devbox has changed".into()),
    );
    send(&mut state, &mut w, daemons::Msg::DetailsToggled(id));
    let details = daemons::details(&state, id);
    assert!(
        details.errors.iter().any(|e| e.reason.contains("host key")),
        "{:?}",
        details.errors
    );
}

fn with_migration(w: &mut World) {
    use micold_core::daemons::{DaemonRegistry, DaemonRuntime, MigrationNotice};
    let mut entries = w.registry.entries().to_vec();
    for entry in &mut entries {
        if let DaemonRuntime::Container(c) = &mut entry.runtime {
            c.migration = Some(MigrationNotice::MovedOffReleasedImage);
        }
    }
    w.registry = DaemonRegistry::new(entries, w.registry.next_id());
}

/// T058 (FR-016): a stored migration shows its sentence; clearing it after a start on the basic
/// image removes the sentence and reports there was one only once.
#[test]
fn a_stored_migration_shows_its_notice_until_it_is_cleared() {
    let mut w = World::new();
    with_migration(&mut w);
    let boxed = w.boxed;
    let state = State::default();
    let row = |w: &mut World| {
        daemons::views(&state, &w.model())
            .rows
            .into_iter()
            .find(|r| r.id == boxed)
            .unwrap()
    };
    assert_eq!(
        row(&mut w).notice.as_deref(),
        Some("Moved off the released daemon image; it moves to the basic image on its next start.")
    );
    assert!(w.registry.clear_migration(boxed));
    assert!(!w.registry.clear_migration(boxed));
    assert_eq!(row(&mut w).notice, None);
}

/// T058 (FR-017): a running released-image container says so, and its Stop is blocked while it
/// has a session; the host daemon and a container that is not legacy are never blocked.
#[test]
fn a_legacy_container_says_so_and_stops_only_with_no_session_running() {
    let mut w = World::new();
    let (host, boxed) = (w.host, w.boxed);
    let state = State::default();
    let rows = |w: &mut World| {
        let mut model = w.model();
        model.legacy_container = Some(boxed);
        daemons::views(&state, &model).rows
    };
    let by = |rows: &[micold_core::daemons::DaemonView], id| {
        rows.iter().find(|r| r.id == id).unwrap().clone()
    };
    let busy = by(&rows(&mut w), boxed);
    assert_eq!(
        busy.notice.as_deref(),
        Some(micold_core::daemons::LEGACY_NOTICE)
    );
    assert!(busy.stop_blocked, "two sessions are running in it");
    assert!(!by(&rows(&mut w), host).stop_blocked);
    w.sessions.insert(boxed, Vec::new());
    assert!(
        !by(&rows(&mut w), boxed).stop_blocked,
        "no session: it may restart"
    );
}

/// T062 (FR-016, FR-017): a start retires the notice only on the basic image and never for an
/// adopted released-image container.
#[test]
fn a_start_retires_the_migration_notice_only_on_the_basic_image() {
    use micold_core::daemons::{DaemonRegistry, DaemonRuntime};
    use micold_core::sandbox::image::{ImageChoice, OwnImage};
    let mut w = World::new();
    with_migration(&mut w);
    let boxed = w.boxed;
    assert!(
        !daemons::retire_migration(&mut w.registry, boxed, true),
        "adopted legacy"
    );
    assert!(
        !daemons::retire_migration(&mut w.registry, w.host, false),
        "not a container"
    );
    let mut entries = w.registry.entries().to_vec();
    for entry in &mut entries {
        if let DaemonRuntime::Container(c) = &mut entry.runtime {
            c.profile.image = ImageChoice::Own(OwnImage::Reference("box:1".into()));
        }
    }
    let mut own = DaemonRegistry::new(entries, w.registry.next_id());
    assert!(
        !daemons::retire_migration(&mut own, boxed, false),
        "own image"
    );
    assert!(daemons::retire_migration(&mut w.registry, boxed, false));
    assert!(
        !daemons::retire_migration(&mut w.registry, boxed, false),
        "once"
    );
}

/// FR-010, US1 #2: a new container daemon has only Claude Code ticked, the selection is edited per
/// daemon, and an emptied selection saves.
mod ai_cli_selection {
    use super::*;
    use daemons::Msg;
    use micold_core::sandbox::AiCliSelection;
    use micold_core::session::AiCli;

    #[test]
    fn a_newly_added_daemon_has_only_claude_code_ticked() {
        let mut state = State::default();
        daemons::begin_add(&mut state);
        assert_eq!(
            state.draft.as_ref().unwrap().profile.ai_clis,
            AiCliSelection::new_daemon()
        );
    }

    #[test]
    fn ticking_unticking_and_emptying_the_selection_save_per_daemon() {
        let mut w = World::new();
        let mut state = State::default();
        let boxed = w.boxed;
        daemons::begin_edit(&mut state, &w.model(), boxed);
        send(&mut state, &mut w, Msg::AiCliToggled(AiCli::Codex, true));
        send(&mut state, &mut w, Msg::AiCliToggled(AiCli::Codex, false));
        for cli in AiCli::ALL {
            send(&mut state, &mut w, Msg::AiCliToggled(cli, false));
        }
        daemons::save(&mut state, &mut w.model()).unwrap();
        assert!(
            profile_of(&w, boxed).ai_clis.0.is_empty(),
            "an empty selection saves"
        );
        daemons::begin_edit(&mut state, &w.model(), boxed);
        send(&mut state, &mut w, Msg::AiCliToggled(AiCli::Pi, true));
        daemons::save(&mut state, &mut w.model()).unwrap();
        assert_eq!(
            profile_of(&w, boxed).ai_clis.0.iter().collect::<Vec<_>>(),
            [&AiCli::Pi]
        );
    }

    #[test]
    fn a_change_on_a_running_daemon_goes_through_the_restart_confirmation() {
        let mut w = World::new();
        let mut state = State::default();
        let boxed = w.boxed;
        let before = profile_of(&w, boxed).ai_clis;
        daemons::start(&mut state, &mut w.model(), boxed);
        daemons::begin_edit(&mut state, &w.model(), boxed);
        send(&mut state, &mut w, Msg::AiCliToggled(AiCli::Pi, false));
        send(&mut state, &mut w, Msg::AiCliToggled(AiCli::Codex, false));
        let out = send(&mut state, &mut w, Msg::Saved);
        assert!(out.is_empty());
        assert!(state.prompt().is_some(), "asks before applying");
        assert_eq!(profile_of(&w, boxed).ai_clis, before, "not applied yet");
        send(&mut state, &mut w, Msg::RemovalConfirmed);
        assert_ne!(profile_of(&w, boxed).ai_clis, before);
    }
}

/// US5 #1, FR-012: an AI CLI whose install fails is listed in the row and in the recent errors,
/// naming the CLI with the installer's output, and the row keeps it once the daemon connects.
mod install_warnings {
    use super::*;
    use daemons::Msg;
    use micold_core::provision::ProvisionWarning;
    use micold_core::session::AiCli;

    fn row(
        state: &State,
        w: &mut World,
        id: micold_core::daemons::DaemonId,
    ) -> micold_core::daemons::DaemonView {
        let vm = daemons::views(state, &w.model());
        vm.rows.into_iter().find(|r| r.id == id).unwrap()
    }

    fn provisioning(
        state: &mut State,
        w: &mut World,
        id: micold_core::daemons::DaemonId,
        e: ProvisionEvent,
    ) {
        send(state, w, Msg::Provisioning(id, e));
    }

    #[test]
    fn install_lines_show_in_the_row_while_the_clis_install() {
        let mut w = World::new();
        let mut state = State::default();
        let id = w.boxed;
        daemons::start(&mut state, &mut w.model(), id);
        provisioning(
            &mut state,
            &mut w,
            id,
            ProvisionEvent::Entered(Step::InstallAiClis),
        );
        provisioning(
            &mut state,
            &mut w,
            id,
            ProvisionEvent::InstallLine("added 3 packages".into()),
        );
        let activity = row(&state, &mut w, id).activity.unwrap();
        assert!(activity.contains("installing the AI CLIs"), "{activity}");
        assert!(activity.contains("added 3 packages"), "{activity}");
    }

    #[test]
    fn a_failed_install_is_listed_in_the_row_and_the_errors_and_survives_connecting() {
        let mut w = World::new();
        let mut state = State::default();
        let id = w.boxed;
        daemons::start(&mut state, &mut w.model(), id);
        let warning = ProvisionWarning {
            cli: AiCli::Codex,
            output: "npm ERR! 404 not found".into(),
        };
        provisioning(
            &mut state,
            &mut w,
            id,
            ProvisionEvent::Warned(warning.clone()),
        );
        daemons::daemon_event(&mut state, &mut w.links, id, DaemonEvent::Connected);
        let shown = row(&state, &mut w, id);
        assert_eq!(shown.state, DaemonState::Connected);
        assert_eq!(shown.install_warnings, [warning.text()]);
        assert!(shown.install_warnings[0].contains("npm ERR! 404 not found"));
        let errors = daemons::details(&state, id).errors;
        assert!(
            errors.iter().any(|e| e.reason == warning.text()),
            "recent errors name the CLI and the output: {errors:?}"
        );
    }

    #[test]
    fn the_next_start_clears_the_warnings() {
        let mut w = World::new();
        let mut state = State::default();
        let id = w.boxed;
        daemons::start(&mut state, &mut w.model(), id);
        let warning = ProvisionWarning {
            cli: AiCli::Pi,
            output: String::new(),
        };
        provisioning(&mut state, &mut w, id, ProvisionEvent::Warned(warning));
        daemons::daemon_event(&mut state, &mut w.links, id, DaemonEvent::Stop);
        daemons::start(&mut state, &mut w.model(), id);
        assert!(row(&state, &mut w, id).install_warnings.is_empty());
    }
}
