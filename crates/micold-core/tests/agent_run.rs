//! Feature 552, FR-003..FR-006: the rules for the agent runs recorded against an origin session.

use micold_core::agent_run::{
    apply_signal, stop_running, AgentRun, AgentRunState, AgentSignal, MAX_RUNS_PER_ORIGIN,
};
use micold_core::session::AiCli;

fn start(id: &str) -> AgentSignal {
    AgentSignal::Started {
        provider: AiCli::ClaudeCode,
        agent_id: id.into(),
        agent_type: Some("Explore".into()),
    }
}

fn stop(id: &str, transcript: &str) -> AgentSignal {
    AgentSignal::Stopped {
        provider: AiCli::ClaudeCode,
        agent_id: id.into(),
        agent_type: Some("Explore".into()),
        transcript: Some(transcript.into()),
    }
}

fn state_of<'a>(runs: &'a [AgentRun], id: &str) -> &'a AgentRun {
    runs.iter().find(|r| r.agent_id == id).expect("run present")
}

#[test]
fn a_start_records_a_running_run_and_a_repeat_changes_nothing() {
    let mut runs = Vec::new();
    assert!(apply_signal(&mut runs, start("a1")));
    assert_eq!(runs.len(), 1);
    assert_eq!(state_of(&runs, "a1").state, AgentRunState::Running);
    assert_eq!(state_of(&runs, "a1").agent_type.as_deref(), Some("Explore"));
    assert!(!apply_signal(&mut runs, start("a1")), "idempotent");
    assert_eq!(runs.len(), 1);
}

#[test]
fn a_stop_marks_the_run_stopped_and_keeps_the_transcript() {
    let mut runs = Vec::new();
    apply_signal(&mut runs, start("a1"));
    assert!(apply_signal(&mut runs, stop("a1", "/t/a1.jsonl")));
    let run = state_of(&runs, "a1");
    assert_eq!(run.state, AgentRunState::Stopped);
    assert_eq!(run.transcript.as_deref(), Some("/t/a1.jsonl"));
}

#[test]
fn a_stop_before_any_start_inserts_a_stopped_run_and_a_late_start_keeps_it_stopped() {
    let mut runs = Vec::new();
    assert!(apply_signal(&mut runs, stop("a1", "/t/a1.jsonl")));
    assert_eq!(state_of(&runs, "a1").state, AgentRunState::Stopped);
    assert!(!apply_signal(&mut runs, start("a1")));
    assert_eq!(state_of(&runs, "a1").state, AgentRunState::Stopped);
    assert_eq!(runs.len(), 1);
}

#[test]
fn the_list_is_capped_and_drops_the_oldest_stopped_run_first() {
    let mut runs = Vec::new();
    apply_signal(&mut runs, start("running-oldest"));
    for n in 0..MAX_RUNS_PER_ORIGIN {
        apply_signal(&mut runs, stop(&format!("s{n}"), "/t"));
    }
    assert_eq!(runs.len(), MAX_RUNS_PER_ORIGIN);
    assert!(runs.iter().any(|r| r.agent_id == "running-oldest"));
    assert!(
        runs.iter().all(|r| r.agent_id != "s0"),
        "oldest stopped went first"
    );
    assert!(runs.iter().any(|r| r.agent_id == "s1"));
}

#[test]
fn stopping_every_running_run_leaves_the_stopped_ones_alone() {
    let mut runs = Vec::new();
    apply_signal(&mut runs, start("a1"));
    apply_signal(&mut runs, stop("a2", "/t/a2"));
    assert!(stop_running(&mut runs));
    assert!(runs.iter().all(|r| r.state == AgentRunState::Stopped));
    assert_eq!(state_of(&runs, "a2").transcript.as_deref(), Some("/t/a2"));
    assert!(!stop_running(&mut runs), "nothing left to stop");
}

#[test]
fn a_full_list_of_running_runs_drops_the_oldest_running_run() {
    let mut runs = Vec::new();
    for i in 0..=MAX_RUNS_PER_ORIGIN {
        apply_signal(&mut runs, start(&format!("r{i}")));
    }
    assert_eq!(runs.len(), MAX_RUNS_PER_ORIGIN);
    assert!(runs.iter().all(|r| r.agent_id != "r0"));
    assert_eq!(runs[0].agent_id, "r1");
    assert_eq!(
        runs.last().unwrap().agent_id,
        format!("r{MAX_RUNS_PER_ORIGIN}")
    );
}

#[test]
fn a_stop_fills_an_agent_type_the_start_lacked() {
    let mut runs = Vec::new();
    apply_signal(
        &mut runs,
        AgentSignal::Started {
            provider: AiCli::ClaudeCode,
            agent_id: "a".into(),
            agent_type: None,
        },
    );
    assert_eq!(state_of(&runs, "a").agent_type, None);
    let changed = apply_signal(
        &mut runs,
        AgentSignal::Stopped {
            provider: AiCli::ClaudeCode,
            agent_id: "a".into(),
            agent_type: Some("Explore".into()),
            transcript: None,
        },
    );
    assert!(changed);
    assert_eq!(state_of(&runs, "a").agent_type.as_deref(), Some("Explore"));
}
