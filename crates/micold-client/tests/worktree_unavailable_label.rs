//! What a worktree row says when its daemon is not connected (feature 491, T048, SC-002, R7,
//! FR-019). The strings are the contract's table.

use iced::futures::channel::mpsc;
use micold_client::daemon::Outbox;
use micold_client::links::{session_lost_label, unavailable_label, DaemonLinks, LabelFacts};
use micold_core::daemons::{
    Binding, ContainerSettings, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime,
};
use micold_core::sandbox::SandboxProfile;
use micold_core::session::SessionLifecycle;

fn fixture() -> (DaemonRegistry, DaemonLinks, DaemonId, DaemonId) {
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, true).unwrap();
    let boxed = registry
        .add(
            "Box",
            DaemonRuntime::Container(ContainerSettings {
                profile: SandboxProfile::default(),
                container_name: "box".into(),
                port: 7001,
            }),
            true,
        )
        .unwrap();
    let mut links = DaemonLinks::at_launch(&registry);
    links.connected(host, Outbox::new(mpsc::unbounded().0));
    links.connected(boxed, Outbox::new(mpsc::unbounded().0));
    (registry, links, host, boxed)
}

fn label(r: &DaemonRegistry, l: &DaemonLinks, id: DaemonId, facts: LabelFacts) -> Option<String> {
    unavailable_label(r, l, Binding::Bound(id), facts)
}

#[test]
fn rows_on_a_connected_daemon_are_unchanged() {
    let (registry, links, host, boxed) = fixture();
    assert_eq!(label(&registry, &links, host, LabelFacts::default()), None);
    assert_eq!(label(&registry, &links, boxed, LabelFacts::default()), None);
}

#[test]
fn an_unreachable_daemon_says_unavailable_and_only_its_rows_do() {
    let (registry, mut links, host, boxed) = fixture();
    links.lost(boxed, "gone");
    assert_eq!(
        label(&registry, &links, boxed, LabelFacts::default()).as_deref(),
        Some("daemon Box unavailable")
    );
    assert_eq!(label(&registry, &links, host, LabelFacts::default()), None);
}

#[test]
fn a_state_change_reaches_the_row_in_the_same_step() {
    let (registry, mut links, _, boxed) = fixture();
    links.apply(boxed, DaemonEvent::Stop);
    assert!(label(&registry, &links, boxed, LabelFacts::default()).is_some());
    links.apply(boxed, DaemonEvent::Start);
    links.connected(boxed, Outbox::new(mpsc::unbounded().0));
    assert_eq!(label(&registry, &links, boxed, LabelFacts::default()), None);
}

#[test]
fn a_version_mismatch_shows_both_versions_and_the_way_out() {
    let (registry, mut links, _, boxed) = fixture();
    links.apply(
        boxed,
        DaemonEvent::Refused {
            client: "1.2.0".into(),
            daemon: "1.1.0".into(),
        },
    );
    let text = label(&registry, &links, boxed, LabelFacts::default()).unwrap();
    assert!(
        text.starts_with("daemon Box version mismatch: client 1.2.0, daemon 1.1.0"),
        "{text}"
    );
    assert!(text.contains("restart"), "{text}");
}

#[test]
fn sessions_lost_runtime_missing_and_mapped_paths_have_their_own_strings() {
    let (registry, mut links, _, boxed) = fixture();
    let mapped = LabelFacts {
        paths_mapped: true,
        ..Default::default()
    };
    assert_eq!(
        label(&registry, &links, boxed, mapped).as_deref(),
        Some("paths are mapped")
    );
    links.lost(boxed, "gone");
    let lost = LabelFacts {
        lost_sessions: true,
        ..Default::default()
    };
    assert_eq!(
        label(&registry, &links, boxed, lost).as_deref(),
        Some("lost to daemon Box")
    );
    let missing = LabelFacts {
        runtime_missing: true,
        ..Default::default()
    };
    assert_eq!(
        label(&registry, &links, boxed, missing).as_deref(),
        Some("container runtime not found")
    );
    assert_eq!(
        unavailable_label(&registry, &links, Binding::NoDaemon, LabelFacts::default()).as_deref(),
        Some("no daemon")
    );
}

#[test]
fn a_session_on_a_dropped_daemon_is_lost_not_finished() {
    let (registry, mut links, host, boxed) = fixture();
    let running = SessionLifecycle::Running;
    assert_eq!(session_lost_label(&registry, &links, boxed, &running), None);
    links.lost(boxed, "gone");
    assert_eq!(
        session_lost_label(&registry, &links, boxed, &running).as_deref(),
        Some("lost to daemon Box")
    );
    assert_eq!(session_lost_label(&registry, &links, host, &running), None);
}

#[test]
fn the_sidebar_row_label_shows_the_status_in_place_of_the_chip() {
    use micold_client::app::State;
    let (registry, mut links, host, boxed) = fixture();
    let mut state = State::default();
    state.settings.daemons = registry;
    state.workspace.active = Some("/repo".into());
    state.workspace.bindings.insert(
        "/repo".into(),
        [("feat-a".to_string(), boxed), ("feat-b".to_string(), host)]
            .into_iter()
            .collect(),
    );
    let facts = LabelFacts::default();
    let of =
        |state: &State, links: &DaemonLinks, key| state.daemon_status_label_of(key, links, facts);
    assert_eq!(of(&state, &links, "feat-a"), "Box");
    links.lost(boxed, "gone");
    assert_eq!(of(&state, &links, "feat-a"), "daemon Box unavailable");
    assert_eq!(of(&state, &links, "feat-b"), "Host", "other rows unchanged");
}
