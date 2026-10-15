//! Feature 552, M2 (FR-004..FR-008, FR-014): agent runs reported by the provider are recorded on
//! their origin session, survive a restart, and never move the origin's activity.

use micold_core::agent_run::{AgentRunState, AgentSignal};
use micold_core::protocol::messages::ActivitySignal;
use micold_core::session::AiCli;
use micold_daemon::state::DaemonState;

#[path = "support/attention.rs"]
mod attention;
use attention::{
    catalog_on, find_summary, process_that_exits, session_id, summary, wait_dead, Service,
};

fn started(id: &str) -> AgentSignal {
    AgentSignal::Started {
        provider: AiCli::ClaudeCode,
        agent_id: id.into(),
        agent_type: Some("Explore".into()),
    }
}

fn stopped(id: &str) -> AgentSignal {
    AgentSignal::Stopped {
        provider: AiCli::ClaudeCode,
        agent_id: id.into(),
        agent_type: Some("Explore".into()),
        transcript: Some("/t/a.jsonl".into()),
    }
}

/// A report for one session changes that session's runs alone, and not its activity (FR-007,
/// FR-014); a session no report names has none (FR-006).
#[test]
fn a_report_records_a_run_on_its_origin_only_and_leaves_activity_alone() {
    let (a, b) = (session_id(0xA), session_id(0xB));
    let service = Service::with_sessions(&[a, b]);
    service.works(a);
    let before = summary(&service.state.catalog_snapshot(), a).activity;
    assert_eq!(before, ActivitySignal::Working);

    assert!(service.state.note_agent_signal(a, started("a1")));
    assert!(
        !service.state.note_agent_signal(a, started("a1")),
        "a repeat start changes nothing"
    );
    assert!(service.state.note_agent_signal(a, stopped("a1")));

    let snapshot = service.state.catalog_snapshot();
    let origin = summary(&snapshot, a);
    assert_eq!(origin.agents.len(), 1);
    assert_eq!(origin.agents[0].agent_id, "a1");
    assert_eq!(origin.agents[0].state, AgentRunState::Stopped);
    assert_eq!(
        origin.activity, before,
        "a sub-agent's end is not the origin's"
    );
    assert!(summary(&snapshot, b).agents.is_empty());
}

/// FR-008: a report for a session the catalog does not hold records nothing and creates nothing.
#[test]
fn a_report_for_an_unknown_session_records_nothing() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    let stranger = session_id(0xDEAD);
    assert!(!service.state.note_agent_signal(stranger, started("x")));
    let snapshot = service.state.catalog_snapshot();
    assert!(find_summary(&snapshot, stranger).is_none());
    assert_eq!(
        snapshot
            .projects
            .iter()
            .map(|p| p.sessions.len())
            .sum::<usize>(),
        1
    );
}

/// FR-005: the runs are written, and a restart brings back a run recorded as running as stopped.
#[test]
fn runs_survive_a_restart_and_a_running_one_comes_back_stopped() {
    let a = session_id(0xA);
    let service = Service::with_sessions(&[a]);
    service.state.note_agent_signal(a, started("running"));
    service.state.note_agent_signal(a, stopped("done"));
    assert!(
        service.state.has_unsaved_attention(),
        "the change waits to be written"
    );
    service.state.persist_attention();

    let restarted = DaemonState::new(catalog_on(service.store.path()));
    let agents = summary(&restarted.catalog_snapshot(), a).agents;
    assert_eq!(agents.len(), 2);
    assert!(agents.iter().all(|r| r.state == AgentRunState::Stopped));
    let done = agents.iter().find(|r| r.agent_id == "done").unwrap();
    assert_eq!(done.transcript.as_deref(), Some("/t/a.jsonl"));
}

/// FR-005: when the origin's process exits, a run still going can no longer be stopped by a report.
#[test]
fn an_origin_exit_marks_its_running_runs_stopped() {
    let a = session_id(0xA);
    let service = Service::with_processes(&[a], process_that_exits);
    service.state.note_agent_signal(a, started("a1"));
    wait_dead(&service.live[0]);
    service.state.supervise_exited_sessions();
    let agents = summary(&service.state.catalog_snapshot(), a).agents;
    assert_eq!(agents[0].state, AgentRunState::Stopped);
}

/// Edge: deleting the origin removes its runs with it; another session's are untouched.
#[test]
fn deleting_the_origin_removes_its_runs() {
    let (a, b) = (session_id(0xA), session_id(0xB));
    let service = Service::with_sessions(&[a, b]);
    service.state.note_agent_signal(a, started("a1"));
    service.state.note_agent_signal(b, started("b1"));
    service.state.delete_session(a).expect("deleted");
    let snapshot = service.state.catalog_snapshot();
    assert!(find_summary(&snapshot, a).is_none());
    assert_eq!(summary(&snapshot, b).agents.len(), 1);
}

/// FR-003, US3 scenario 2 (feature 552, M4): no Copilot sub-agent event was confirmed in a real
/// log (research R8), so Copilot's event log never records a run, whatever it carries. Sub-agent
/// looking lines map to no activity event and the session stays a user session.
#[test]
fn a_copilot_session_never_gains_runs_from_its_event_log() {
    let id = session_id(0xC);
    let service = Service::with_sessions_of(&[(id, AiCli::Copilot)]);
    for line in [
        r#"{"type":"subagent.started","data":{"agentName":"explore"}}"#,
        r#"{"type":"subagent.completed","data":{"agentName":"explore"}}"#,
        r#"{"type":"assistant.turn_start","data":{}}"#,
        r#"{"type":"assistant.turn_end","data":{}}"#,
    ] {
        if let Some(event) = micold_daemon::activity::copilot_event(line) {
            service.note(id, event);
        }
    }
    assert!(summary(&service.state.catalog_snapshot(), id)
        .agents
        .is_empty());
}
