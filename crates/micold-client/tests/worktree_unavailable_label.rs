//! What a worktree row says when its daemon is not connected (feature 491, T048, SC-002, R7,
//! FR-019). The strings are the contract's table.

use iced::futures::channel::mpsc;
use micold_client::app::State;
use micold_client::daemon::Outbox;
use micold_client::links::{session_lost_label, unavailable_label, DaemonLinks, LabelFacts};
use micold_core::daemons::{
    Binding, ContainerSettings, DaemonEvent, DaemonId, DaemonRegistry, DaemonRuntime, DaemonState,
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
fn paths_mapped_shows_only_while_the_daemon_is_connected() {
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
    assert_eq!(
        label(&registry, &links, boxed, mapped).as_deref(),
        Some("daemon Box unavailable"),
        "an unreachable daemon outranks the informational label"
    );
}

#[test]
fn lost_sessions_are_disconnected_while_the_daemon_reconnects() {
    let (registry, mut links, _, boxed) = fixture();
    let lost = LabelFacts {
        lost_sessions: true,
        ..Default::default()
    };
    assert_eq!(
        label(&registry, &links, boxed, lost),
        None,
        "still connected"
    );
    links.lost(boxed, "gone");
    assert_eq!(
        label(&registry, &links, boxed, lost).as_deref(),
        Some("disconnected"),
        "FR-019: it may return"
    );
    links.apply(boxed, DaemonEvent::GiveUp);
    assert_eq!(
        label(&registry, &links, boxed, lost).as_deref(),
        Some("disconnected"),
        "unreachable after a connection was established"
    );
}

#[test]
fn lost_sessions_of_a_daemon_that_never_connected_name_it() {
    let (registry, mut links, _, boxed) = fixture();
    links.insert(
        boxed,
        DaemonState::Unreachable {
            reason: "refused".into(),
        },
    );
    let lost = LabelFacts {
        lost_sessions: true,
        ..Default::default()
    };
    assert_eq!(
        label(&registry, &links, boxed, lost).as_deref(),
        Some("lost to daemon Box")
    );
}

#[test]
fn a_missing_container_runtime_is_said_for_a_container_and_not_a_host() {
    let (registry, mut links, host, boxed) = fixture();
    let missing = LabelFacts {
        runtime_missing: true,
        ..Default::default()
    };
    links.lost(boxed, "gone");
    links.lost(host, "gone");
    assert_eq!(
        label(&registry, &links, boxed, missing).as_deref(),
        Some("container runtime not found")
    );
    // A host daemon has no container runtime to miss.
    assert_eq!(
        label(&registry, &links, host, missing).as_deref(),
        Some("daemon Host unavailable")
    );
}

#[test]
fn a_row_with_no_daemon_says_so() {
    let (registry, links, _, _) = fixture();
    assert_eq!(
        unavailable_label(&registry, &links, Binding::NoDaemon, LabelFacts::default()).as_deref(),
        Some("no daemon")
    );
}

#[test]
fn a_session_on_a_dropped_daemon_is_lost_not_finished() {
    let (registry, mut links, host, boxed) = fixture();
    let running = SessionLifecycle::Running;
    let label = |links: &DaemonLinks, id, absent| {
        session_lost_label(&registry, links, id, &running, absent)
    };
    assert_eq!(label(&links, boxed, false), None);
    links.lost(boxed, "gone");
    assert_eq!(label(&links, boxed, false).as_deref(), Some("disconnected"));
    assert_eq!(label(&links, host, false), None);
    // Only live sessions are labelled.
    let idle = SessionLifecycle::Idle;
    assert_eq!(
        session_lost_label(&registry, &links, boxed, &idle, true),
        None
    );
}

/// T042, FR-019: reconnected, a session the daemon no longer has is lost to it, not disconnected;
/// so is one on a daemon that was stopped.
#[test]
fn a_session_missing_after_a_reconnect_is_lost_to_its_daemon() {
    let (registry, mut links, _, boxed) = fixture();
    let running = SessionLifecycle::Running;
    links.lost(boxed, "gone");
    links.connected(boxed, Outbox::new(mpsc::unbounded().0));
    assert_eq!(
        session_lost_label(&registry, &links, boxed, &running, false),
        None,
        "present in the fresh catalog: running again"
    );
    assert_eq!(
        session_lost_label(&registry, &links, boxed, &running, true).as_deref(),
        Some("lost to daemon Box")
    );
    links.apply(boxed, DaemonEvent::Stop);
    assert_eq!(
        session_lost_label(&registry, &links, boxed, &running, false).as_deref(),
        Some("lost to daemon Box")
    );
}

/// A window state whose worktrees `feat-a` (on Box) and `feat-b` (on Host) are in the active
/// project, with the fixture's links held as the window's own.
fn window(boxed_sessions: &[SessionLifecycle]) -> (State, DaemonId, DaemonId) {
    use micold_core::session::{Session, SessionLocation};
    let (registry, links, host, boxed) = fixture();
    let mut state = State::default();
    state.settings.daemons = registry;
    state.settings.links = links;
    let project = std::path::PathBuf::from("/repo");
    state.workspace.active = Some(project.clone());
    state.workspace.bindings.insert(
        project.clone(),
        [("feat-a".to_string(), boxed), ("feat-b".to_string(), host)]
            .into_iter()
            .collect(),
    );
    let sessions = boxed_sessions
        .iter()
        .map(|lifecycle| {
            let mut s = Session::start_new(
                SessionLocation::Worktree("feat-a".into()),
                micold_core::session::AiCli::ClaudeCode,
            );
            s.lifecycle = lifecycle.clone();
            s
        })
        .collect();
    state.workspace.sessions.insert(project, sessions);
    (state, host, boxed)
}

#[test]
fn the_sidebar_row_label_shows_the_status_in_place_of_the_chip() {
    let (mut state, _, boxed) = window(&[]);
    assert_eq!(state.daemon_row_label("feat-a"), "Box");
    state.settings.links.lost(boxed, "gone");
    assert_eq!(state.daemon_row_label("feat-a"), "daemon Box unavailable");
    assert_eq!(
        state.daemon_row_label("feat-b"),
        "Host",
        "other rows unchanged"
    );
}

#[test]
fn a_running_session_on_a_dropped_daemon_makes_its_row_disconnected() {
    let (mut state, _, boxed) = window(&[SessionLifecycle::Running]);
    state.settings.links.lost(boxed, "gone");
    assert_eq!(state.daemon_row_label("feat-a"), "disconnected");
    assert_eq!(state.daemon_row_label("feat-b"), "Host");
}

#[test]
fn an_idle_session_does_not_make_its_row_lost() {
    let (mut state, _, boxed) = window(&[SessionLifecycle::Running]);
    state
        .workspace
        .sessions
        .get_mut(std::path::Path::new("/repo"))
        .unwrap()[0]
        .lifecycle = SessionLifecycle::Idle;
    state.settings.links.lost(boxed, "gone");
    assert_eq!(state.daemon_row_label("feat-a"), "daemon Box unavailable");
}

#[test]
fn a_missing_runtime_shows_on_a_container_row_only() {
    let (mut state, _, boxed) = window(&[]);
    state.settings.runtime_missing = true;
    state.settings.links.lost(boxed, "gone");
    assert_eq!(
        state.daemon_row_label("feat-a"),
        "container runtime not found"
    );
    assert_eq!(state.daemon_row_label("feat-b"), "Host");
}
