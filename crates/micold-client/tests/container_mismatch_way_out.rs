//! A container daemon in version mismatch has a way out (BUG-1165, T100; spec 806 FR-007, FR-017):
//! its row explains the mismatch like an SSH daemon's (feature 489, FR-011), Replace is offered only
//! for an older daemon proved idle, Stop is the stated way out otherwise and is never hidden by the
//! client's stale session list, and the host daemon is untouched by any of it (spec 491 FR-007).

#[path = "support/daemons_fixture.rs"]
mod daemons_fixture;

use micold_client::features::daemons::{self, Msg, State};
use micold_client::features::Outcome;
use micold_core::daemons::{DaemonEvent, DaemonId, DaemonState, DaemonView};
use micold_core::ssh::ReplaceOffer;

/// The fixture's container daemon `Box` (two live sessions) started and refused by the handshake.
fn world(client: &str, daemon: &str) -> (daemons_fixture::World, State) {
    let mut w = daemons_fixture::World::new();
    w.links.apply(w.boxed, DaemonEvent::Start);
    w.links.apply(
        w.boxed,
        DaemonEvent::Refused {
            client: client.into(),
            daemon: daemon.into(),
        },
    );
    (w, State::default())
}

fn row(w: &mut daemons_fixture::World, state: &State, id: DaemonId, legacy: bool) -> DaemonView {
    let boxed = w.boxed;
    let model = daemons::Model {
        legacy_container: legacy.then_some(boxed),
        ..w.model()
    };
    let vm = daemons::views(state, &model);
    vm.rows.into_iter().find(|r| r.id == id).unwrap()
}

fn offer(w: &mut daemons_fixture::World, state: &mut State, offer: Option<ReplaceOffer>) {
    if let Some(offer) = offer {
        let id = w.boxed;
        daemons::update(state, &mut w.model(), Msg::OfferReported(id, offer));
    }
}

fn replace(w: &mut daemons_fixture::World, state: &mut State) -> Vec<Outcome> {
    let id = w.boxed;
    daemons::update(state, &mut w.model(), Msg::ReplaceRequested(id))
}

#[test]
fn the_row_names_both_versions_the_older_side_and_the_way_out() {
    for legacy in [false, true] {
        let (mut w, state) = world("0.5.3", "0.5.2");
        let boxed = w.boxed;
        let text = row(&mut w, &state, boxed, legacy)
            .explanation
            .expect("a mismatched container daemon is explained");
        assert!(text.contains("client 0.5.3, daemon 0.5.2"), "{text}");
        assert!(text.contains("daemon is older"), "{text}");
        assert!(text.contains("Stop this daemon"), "{text}");
    }
}

#[test]
fn replace_is_offered_only_for_an_older_daemon_proved_idle() {
    for (proved, offered) in [
        (None, false),
        (Some(ReplaceOffer::Unknown), false),
        (Some(ReplaceOffer::Blocked { running: 2 }), false),
        (Some(ReplaceOffer::Available), true),
    ] {
        let (mut w, mut state) = world("0.5.3", "0.5.2");
        let boxed = w.boxed;
        offer(&mut w, &mut state, proved);
        assert_eq!(
            row(&mut w, &state, boxed, false).replace_offered,
            offered,
            "{proved:?}"
        );
        let out = replace(&mut w, &mut state);
        assert_eq!(
            matches!(out.as_slice(), [Outcome::RemoteReplaceRequested { daemon, .. }] if *daemon == boxed),
            offered,
            "{proved:?}: {out:?}"
        );
    }
}

#[test]
fn a_newer_container_daemon_is_never_replaced() {
    let (mut w, mut state) = world("0.5.2", "0.5.3");
    let boxed = w.boxed;
    offer(&mut w, &mut state, Some(ReplaceOffer::Available));
    let view = row(&mut w, &state, boxed, false);
    assert!(!view.replace_offered);
    assert!(view.explanation.unwrap().contains("Update this app."));
    assert!(replace(&mut w, &mut state).is_empty());
}

#[test]
fn a_legacy_container_is_never_offered_a_replacement() {
    let (mut w, mut state) = world("0.5.3", "0.5.2");
    let boxed = w.boxed;
    offer(&mut w, &mut state, Some(ReplaceOffer::Available));
    assert!(!row(&mut w, &state, boxed, true).replace_offered);
}

#[test]
fn with_sessions_running_or_uncounted_the_way_out_is_the_rows_stop() {
    for proved in [ReplaceOffer::Blocked { running: 2 }, ReplaceOffer::Unknown] {
        let (mut w, mut state) = world("0.5.3", "0.5.2");
        let boxed = w.boxed;
        offer(&mut w, &mut state, Some(proved));
        for legacy in [false, true] {
            let view = row(&mut w, &state, boxed, legacy);
            let text = view.explanation.unwrap();
            assert!(text.contains("Stop this daemon"), "{text}");
            assert!(!text.contains("yourself"), "{text}");
            assert!(
                !view.stop_blocked,
                "legacy={legacy}: a mismatched daemon cannot report its sessions; the client's \
                 cached list must not hide Stop"
            );
        }
    }
}

#[test]
fn stopping_a_mismatched_container_daemon_stops_its_container() {
    let (mut w, mut state) = world("0.5.3", "0.5.2");
    let boxed = w.boxed;
    let out = daemons::update(&mut state, &mut w.model(), Msg::Stopped(boxed));
    assert!(
        matches!(out.as_slice(), [Outcome::ContainerStopRequested { daemon }] if *daemon == boxed),
        "{out:?}"
    );
    assert_eq!(w.links.state(boxed), Some(&DaemonState::Stopped));

    // A daemon that can still be spoken to keeps its container on Stop, as before.
    let mut w = daemons_fixture::World::new();
    let boxed = w.boxed;
    w.links.apply(boxed, DaemonEvent::Start);
    let out = daemons::update(&mut state, &mut w.model(), Msg::Stopped(boxed));
    assert!(out.is_empty(), "{out:?}");
}

#[test]
fn the_host_daemon_is_untouched() {
    let (mut w, mut state) = world("0.5.3", "0.5.2");
    let host = w.host;
    let before = row(&mut w, &state, host, false);
    offer(&mut w, &mut state, Some(ReplaceOffer::Available));
    replace(&mut w, &mut state);
    let after = row(&mut w, &state, host, true);
    assert_eq!(w.links.state(host), Some(&DaemonState::Stopped));
    assert_eq!(after.explanation, None);
    assert!(!after.replace_offered && !after.stop_blocked);
    assert_eq!(before.state, after.state);
}
