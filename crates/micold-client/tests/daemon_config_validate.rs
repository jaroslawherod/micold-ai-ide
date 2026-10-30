//! A rendered file is checked with `micold-daemon config check`'s check before delivery; a file
//! that fails is not delivered and its errors reach the Settings → Daemons view model (FR-013f,
//! #1525).

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;
#[path = "support/daemon_config_fixtures.rs"]
mod fixtures;

use daemons_fixture::World;
use fixtures::{settings, KINDS};
use micold_client::daemon_config_render::{prepare, render, DaemonSettings};
use micold_client::features::daemons::{self, State};
use micold_core::daemon_config::{diagnose, Kind};

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
    assert_eq!(errors, expected);
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

#[test]
fn every_configured_daemon_renders_a_valid_file() {
    let w = World::new();
    for entry in w.registry.entries() {
        let (settings, kind) = DaemonSettings::from_entry(entry).expect("a runtime with a file");
        assert!(prepare(&settings, kind).is_ok(), "{}", entry.name.as_str());
    }
    let kube = daemons_fixture::kubernetes("ctx", "ns");
    let mut registry = w.registry.clone();
    let id = registry.add("Pod", kube, false).unwrap();
    let (settings, kind) = DaemonSettings::from_entry(registry.get(id).unwrap()).unwrap();
    assert_eq!(kind, Kind::Kubernetes);
    assert!(prepare(&settings, kind).is_ok());
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
    assert!(state.config_checked(w.registry.entries().last().unwrap().id));
}
