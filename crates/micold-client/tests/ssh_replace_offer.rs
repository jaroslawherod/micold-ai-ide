//! Replacing a mismatched remote daemon (feature 489, T075, FR-011): the Replace action follows what
//! the daemon said of its sessions, is offered only for an older daemon proved idle, and ends
//! nothing otherwise.

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use micold_client::features::daemons::{self, Msg, State};
use micold_client::features::Outcome;
use micold_core::daemons::{DaemonEvent, DaemonId, DaemonRuntime, DaemonState};
use micold_core::ssh::{ReplaceOffer, SshSettings, SshTarget};

fn world(client: &str, daemon: &str) -> (daemons_fixture::World, State, DaemonId) {
    let mut w = daemons_fixture::World::new();
    let target = SshTarget::new("devbox", Some("dev"), None, None).unwrap();
    let remote = w
        .registry
        .add("Dev", DaemonRuntime::Ssh(SshSettings { target }), true)
        .unwrap();
    w.links = micold_client::links::DaemonLinks::at_launch(&w.registry);
    w.links.apply(
        remote,
        DaemonEvent::Refused {
            client: client.into(),
            daemon: daemon.into(),
        },
    );
    (w, State::default(), remote)
}

fn row(w: &mut daemons_fixture::World, state: &State, id: DaemonId) -> (bool, bool, String) {
    let vm = daemons::views(state, &w.model());
    let r = vm.rows.iter().find(|r| r.id == id).unwrap();
    (
        r.replace_offered,
        r.replacing,
        r.explanation.clone().unwrap(),
    )
}

#[test]
fn replace_is_offered_only_when_the_older_daemon_proved_it_has_no_sessions() {
    for (offer, offered) in [
        (None, false),
        (Some(ReplaceOffer::Unknown), false),
        (Some(ReplaceOffer::Blocked { running: 2 }), false),
        (Some(ReplaceOffer::Available), true),
    ] {
        let (mut w, mut state, id) = world("0.5.3", "0.5.2");
        if let Some(offer) = offer {
            daemons::update(&mut state, &mut w.model(), Msg::OfferReported(id, offer));
        }
        assert_eq!(row(&mut w, &state, id).0, offered, "{offer:?}");
        let out = daemons::update(&mut state, &mut w.model(), Msg::ReplaceRequested(id));
        assert_eq!(
            matches!(out.as_slice(), [Outcome::RemoteReplaceRequested { daemon, .. }] if *daemon == id),
            offered,
            "{offer:?}"
        );
    }
}

#[test]
fn a_newer_daemon_is_never_offered_a_replacement() {
    let (mut w, mut state, id) = world("0.5.2", "0.5.3");
    daemons::update(
        &mut state,
        &mut w.model(),
        Msg::OfferReported(id, ReplaceOffer::Available),
    );
    let (offered, _, text) = row(&mut w, &state, id);
    assert!(!offered);
    assert!(text.contains("Update this app."), "{text}");
    let out = daemons::update(&mut state, &mut w.model(), Msg::ReplaceRequested(id));
    assert!(out.is_empty());
}

#[test]
fn the_explanation_follows_the_offer() {
    let (mut w, mut state, id) = world("0.5.3", "0.5.2");
    daemons::update(
        &mut state,
        &mut w.model(),
        Msg::OfferReported(id, ReplaceOffer::Blocked { running: 3 }),
    );
    let text = row(&mut w, &state, id).2;
    assert!(text.contains("3 session"), "{text}");
}

#[test]
fn a_replacement_runs_once_and_a_success_reconnects() {
    let (mut w, mut state, id) = world("0.5.3", "0.5.2");
    daemons::update(
        &mut state,
        &mut w.model(),
        Msg::OfferReported(id, ReplaceOffer::Available),
    );
    let out = daemons::update(&mut state, &mut w.model(), Msg::ReplaceRequested(id));
    assert_eq!(out.len(), 1);
    let (offered, replacing, _) = row(&mut w, &state, id);
    assert!(!offered && replacing, "under way: no second click");
    assert!(daemons::update(&mut state, &mut w.model(), Msg::ReplaceRequested(id)).is_empty());
    daemons::update(&mut state, &mut w.model(), Msg::Replaced(id, Ok(())));
    assert!(
        !matches!(w.links.state(id), Some(DaemonState::VersionMismatch { .. })),
        "the daemon is dialled again"
    );
}

#[test]
fn a_refused_replacement_leaves_the_daemon_and_says_why() {
    let (mut w, mut state, id) = world("0.5.3", "0.5.2");
    daemons::update(
        &mut state,
        &mut w.model(),
        Msg::OfferReported(id, ReplaceOffer::Available),
    );
    daemons::update(&mut state, &mut w.model(), Msg::ReplaceRequested(id));
    daemons::update(
        &mut state,
        &mut w.model(),
        Msg::Replaced(id, Err(ReplaceOffer::Blocked { running: 1 })),
    );
    let (offered, replacing, text) = row(&mut w, &state, id);
    assert!(!offered && !replacing);
    assert!(text.contains("1 session"), "{text}");
    assert!(matches!(
        w.links.state(id),
        Some(DaemonState::VersionMismatch { .. })
    ));
}
