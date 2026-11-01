//! A rendered file is checked with `micold-daemon config check`'s check before delivery; a file
//! that fails is not delivered and its errors reach the Settings → Daemons view model (FR-013f,
//! #1525).

use crate::daemons_fixture;
use crate::fixtures;

use daemons_fixture::World;
use fixtures::{settings, KINDS};
use micold_client::daemon_config_render::{prepare, render, DaemonSettings};
use micold_client::features::daemons::{self, State};
use micold_core::daemon_config::IdleStopSetting;
use micold_core::daemon_config::{diagnose, Kind, Listen, SocketSpec};
use micold_core::daemons::{DaemonEntry, DaemonRuntime, FileSettings, ProjectSetting};
use micold_core::ssh::{SshSettings, SshTarget};

fn invalid() -> DaemonSettings {
    let mut s = settings(Kind::Container);
    s.client_token_file = None;
    s
}

#[test]
fn a_valid_file_is_delivered_as_rendered() {
    for kind in KINDS {
        let s = settings(kind);
        assert_eq!(prepare(&s, kind), Ok(render(&s, kind)), "{kind:?}");
    }
}

#[test]
fn an_invalid_file_yields_the_errors_of_config_check_and_is_not_delivered() {
    let s = invalid();
    let (config, expected) = diagnose(&render(&s, Kind::Container), "daemon.toml");
    assert!(config.is_none());
    let errors = prepare(&s, Kind::Container).expect_err("not delivered");
    assert_eq!(errors, expected, "the errors are those of config check");
    assert!(errors[0]
        .message
        .contains("`listen.tcp` requires `auth.client_token_file`"));
}

#[test]
fn the_errors_reach_the_daemons_view_model_and_a_valid_file_clears_them() {
    let mut w = World::new();
    let mut state = State::default();
    let id = w.boxed;
    let row = |state: &State, w: &mut World| {
        daemons::views(state, &w.model())
            .rows
            .into_iter()
            .find(|r| r.id == id)
            .unwrap()
    };

    assert_eq!(
        daemons::validate_config(&mut state, id, &invalid(), Kind::Container),
        None
    );
    let shown = row(&state, &mut w).config_errors;
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].contains("`listen.tcp` requires `auth.client_token_file`"));

    let valid = settings(Kind::Container);
    let text = daemons::validate_config(&mut state, id, &valid, Kind::Container);
    assert_eq!(text, Some(render(&valid, Kind::Container)));
    assert!(row(&state, &mut w).config_errors.is_empty());
}

/// What `from_entry` builds for each runtime from an entry with no file settings: the kind, the
/// name and how the daemon listens (fixed until the delivery issues, 5-7), with the sandbox token path.
fn expected(entry: &DaemonEntry) -> (DaemonSettings, Kind) {
    let name = entry.name.as_str();
    let host = || DaemonSettings::new(name, Listen::Socket(SocketSpec::Auto));
    let sandbox = || DaemonSettings {
        client_token_file: Some("/run/micold/secrets/client-token".into()),
        ..DaemonSettings::new(name, Listen::Tcp("0.0.0.0:7000".parse().unwrap()))
    };
    match &entry.runtime {
        DaemonRuntime::Host => (host(), Kind::Host),
        DaemonRuntime::Ssh(_) => (host(), Kind::Ssh),
        DaemonRuntime::Container(_) => (sandbox(), Kind::Container),
        DaemonRuntime::Kubernetes(_) => (sandbox(), Kind::Kubernetes),
        other => panic!("no file for {other:?}"),
    }
}

#[test]
fn every_configured_daemon_renders_the_file_of_its_runtime() {
    let w = World::new();
    let kube = daemons_fixture::kubernetes("ctx", "ns");
    let mut registry = w.registry.clone();
    registry.add("Pod", kube, false).unwrap();
    let target = SshTarget::new("box.example", None, None, None).unwrap();
    registry
        .add("Remote", DaemonRuntime::Ssh(SshSettings { target }), false)
        .unwrap();
    let mut kinds = Vec::new();
    for entry in registry.entries() {
        let name = entry.name.as_str();
        let (settings, kind) = DaemonSettings::from_entry(entry).expect("a runtime with a file");
        assert_eq!((settings.clone(), kind), expected(entry), "{name}");
        assert!(
            prepare(&settings, kind).is_ok(),
            "{name} renders a valid file"
        );
        kinds.push(kind);
    }
    for kind in KINDS {
        assert!(
            kinds.contains(&kind),
            "a {kind:?} daemon was checked: {kinds:?}"
        );
    }
}

#[test]
fn saving_a_daemon_checks_its_file() {
    let mut w = World::new();
    let mut state = State::default();
    daemons::begin_add(&mut state);
    let draft = state.draft.as_mut().unwrap();
    draft.name = "Fresh".into();
    draft.runtime = daemons::DraftRuntime::Container;
    draft.container_name = "fresh-ctr".into();
    draft.port = "7123".into();
    let out = daemons::update(&mut state, &mut w.model(), daemons::Msg::Saved);
    assert!(!out.is_empty(), "{:?}", state.form_error);
    let id = w.registry.entries().last().unwrap().id;
    assert!(
        state.config_checked(id),
        "the saved daemon's file is checked"
    );
    let shown = daemons::views(&state, &w.model())
        .rows
        .into_iter()
        .find(|r| r.id == id)
        .unwrap()
        .config_errors;
    assert!(shown.is_empty(), "a valid file shows no error: {shown:?}");
}

fn row_errors(state: &State, w: &mut World, id: micold_core::daemons::DaemonId) -> Vec<String> {
    daemons::views(state, &w.model())
        .rows
        .into_iter()
        .find(|r| r.id == id)
        .unwrap()
        .config_errors
}

/// Edit daemon `id` and save the form unchanged apart from `change`.
fn save_edit(
    w: &mut World,
    id: micold_core::daemons::DaemonId,
    change: impl Fn(&mut daemons::Draft),
) -> State {
    let mut state = State::default();
    daemons::begin_edit(&mut state, &w.model(), id).expect("an editable daemon");
    change(state.draft.as_mut().unwrap());
    let out = daemons::update(&mut state, &mut w.model(), daemons::Msg::Saved);
    assert!(
        !out.is_empty(),
        "the save is stored: {:?}",
        state.form_error
    );
    state
}

#[test]
fn an_invalid_setting_typed_in_the_form_shows_its_error_in_the_daemons_row() {
    let mut w = World::new();
    let id = w.boxed;
    let mut state = save_edit(&mut w, id, |d| d.data_dir = "relative/data".into());
    let shown = row_errors(&state, &mut w, id);
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(shown[0].contains("daemon.data_dir"), "{shown:?}");
    assert_eq!(
        w.registry.get(id).unwrap().file.data_dir,
        Some("relative/data".into()),
        "the setting is stored as typed"
    );

    // Fixing the setting clears the error.
    daemons::begin_edit(&mut state, &w.model(), id).unwrap();
    state.draft.as_mut().unwrap().data_dir = "/srv/micold".into();
    daemons::update(&mut state, &mut w.model(), daemons::Msg::Saved);
    assert!(row_errors(&state, &mut w, id).is_empty());
}

#[test]
fn a_project_with_a_remote_and_no_clone_root_is_flagged_after_a_save() {
    let mut w = World::new();
    let id = w.boxed;
    let file = FileSettings {
        projects: vec![ProjectSetting {
            path: "/work/app".into(),
            remote: Some("git@example.com:app.git".into()),
        }],
        ..FileSettings::default()
    };
    w.registry.set_file(id, file).unwrap();
    let state = save_edit(&mut w, id, |_| {});
    let shown = row_errors(&state, &mut w, id);
    assert_eq!(shown.len(), 1, "{shown:?}");
    assert!(
        shown[0].contains("`storage.clone_root` is not set"),
        "{shown:?}"
    );
    assert_eq!(
        w.registry.get(id).unwrap().file.projects.len(),
        1,
        "the form keeps the projects it has no field for"
    );
}

#[test]
fn from_entry_builds_the_file_from_the_saved_settings() {
    let mut w = World::new();
    let id = w.boxed;
    let file = FileSettings {
        data_dir: Some("/srv/micold".into()),
        clone_root: Some("/srv/clones".into()),
        client_token_file: Some("/srv/token".into()),
        idle_stop: Some(IdleStopSetting::Secs(600)),
        projects: vec![ProjectSetting {
            path: "/srv/clones/app".into(),
            remote: Some("r".into()),
        }],
    };
    w.registry.set_file(id, file.clone()).unwrap();
    let (settings, kind) = DaemonSettings::from_entry(w.registry.get(id).unwrap()).unwrap();
    assert_eq!(kind, Kind::Container);
    assert_eq!(settings.data_dir, file.data_dir);
    assert_eq!(settings.clone_root, file.clone_root);
    assert_eq!(settings.client_token_file, file.client_token_file);
    assert_eq!(settings.idle_stop, file.idle_stop);
    assert_eq!(settings.projects, file.projects);
    assert!(prepare(&settings, kind).is_ok());
}

#[test]
fn file_settings_survive_the_stored_registry() {
    let mut w = World::new();
    let file = FileSettings {
        data_dir: Some("/d".into()),
        idle_stop: Some(IdleStopSetting::Off),
        ..FileSettings::default()
    };
    w.registry.set_file(w.boxed, file.clone()).unwrap();
    let json = serde_json::to_value(w.registry.get(w.boxed).unwrap()).unwrap();
    let back: DaemonEntry = serde_json::from_value(json).unwrap();
    assert_eq!(back.file, file);
    let host = serde_json::to_value(w.registry.get(w.host).unwrap()).unwrap();
    assert!(
        host.get("file").is_none(),
        "an entry with none stores none: {host}"
    );
}

#[test]
fn a_container_that_survives_logout_renders_idle_stop_off() {
    let mut w = World::new();
    let DaemonRuntime::Container(mut c) = daemons_fixture::container("keep", 7301) else {
        unreachable!("container() is a container")
    };
    c.profile.survive_logout = true;
    let id = w
        .registry
        .add("Keeper", DaemonRuntime::Container(c), false)
        .unwrap();
    let (settings, _) = DaemonSettings::from_entry(w.registry.get(id).unwrap()).unwrap();
    assert_eq!(settings.idle_stop, Some(IdleStopSetting::Off));
}
