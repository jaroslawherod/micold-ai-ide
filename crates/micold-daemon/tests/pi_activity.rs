//! Pi's activity component drives the same activity machine (feature 029, T036 — FR-012, FR-012d).
//!
//! Pi reports busy and idle only to code loaded into its own process, so the daemon loads a
//! component of its own (`assets/pi-activity.ts`) that appends one `{"type","at"}` line per event to
//! a log outside Pi's session store. `micold-daemon/src/activity.rs::pi_event` maps one of those
//! lines to an `ActivityEvent`; the tail that reads them and the `Activity` state machine they feed
//! are the ones `copilot` already uses, unchanged.
//!
//! The mapping table asserted here is the one in `specs/029-pi-cli-provider/contracts/pi-cli.md`
//! §Activity signal. Nothing here needs `pi` installed.

use micold_core::protocol::messages::ActivitySignal;
use micold_daemon::activity::{pi_event, Activity, ActivityEvent, HookKind};

/// One line as the component writes it.
fn line(kind: &str) -> String {
    format!(r#"{{"type":"{kind}","at":"2026-09-13T10:00:00.000Z"}}"#)
}

/// Replay lines through a fresh machine and return the signal after each mapped event.
fn replay(lines: &[String]) -> (Activity, Vec<ActivitySignal>) {
    let mut activity = Activity::new();
    let mut trace = Vec::new();
    for l in lines {
        if let Some(event) = pi_event(l) {
            activity.apply(event);
            trace.push(activity.signal().clone());
        }
    }
    (activity, trace)
}

#[test]
fn each_event_type_maps_to_the_signal_the_contract_names() {
    // One line at a time, against the contract's table. Per event rather than over a whole replay:
    // a replay can land in the right place with two errors that cancel.
    let cases = [
        ("turn_start", HookKind::UserPromptSubmit),
        ("agent_start", HookKind::PreToolUse),
        ("tool_execution_start", HookKind::PreToolUse),
        ("tool_execution_end", HookKind::PostToolUse),
        ("agent_settled", HookKind::Stop),
        ("turn_end", HookKind::Stop),
    ];
    for (kind, expected) in cases {
        assert_eq!(
            pi_event(&line(kind)),
            Some(ActivityEvent::Hook(expected)),
            "{kind}"
        );
    }

    // The seventh is terminal, and a sibling of `Hook` — which is why the mapping returns an
    // `ActivityEvent` rather than a `HookKind`. The component carries no reason, so the daemon
    // supplies a neutral one rather than inventing a cause.
    assert!(
        matches!(
            pi_event(&line("session_shutdown")),
            Some(ActivityEvent::Ended { .. })
        ),
        "session_shutdown ends the session"
    );
}

#[test]
fn a_whole_turn_with_a_tool_call_ends_awaiting_input() {
    let (_, trace) = replay(&[
        line("turn_start"),
        line("agent_start"),
        line("tool_execution_start"),
        line("tool_execution_end"),
        line("turn_end"),
        line("agent_settled"),
    ]);
    assert_eq!(trace.first(), Some(&ActivitySignal::Working));
    assert_eq!(trace.last(), Some(&ActivitySignal::AwaitingInput));
}

#[test]
fn a_tool_finishing_mid_turn_does_not_read_as_finished() {
    // `tool_execution_end` is mapped and deliberately changes nothing: the turn is over at
    // `turn_end`, not before. Getting this wrong flickers the badge to idle on every tool call.
    let (activity, _) = replay(&[
        line("turn_start"),
        line("tool_execution_start"),
        line("tool_execution_end"),
    ]);
    assert_eq!(activity.signal(), &ActivitySignal::Working);
}

#[test]
fn unknown_event_types_are_ignored_and_never_reject_a_line() {
    // Pi gains event types between versions, and the component could be edited to subscribe to
    // more. Ignoring is the contract; rejecting would mean a Pi update silently stops the badge.
    for kind in [
        "session_start",
        "message_start",
        "message_update",
        "message_end",
        "tool_call",
        "tool_result",
        "model_select",
        "session_compact",
        "an_event_that_does_not_exist_yet",
    ] {
        assert_eq!(pi_event(&line(kind)), None, "{kind}");
    }
}

#[test]
fn junk_is_skipped_rather_than_ending_the_replay() {
    let (activity, trace) = replay(&[
        String::new(),
        "{ this line is not json".to_string(),
        r#"{"at":"2026-09-13T10:00:00.000Z"}"#.to_string(),
        r#"{"type":42}"#.to_string(),
        line("turn_start"),
    ]);
    assert_eq!(
        trace.len(),
        1,
        "only the last line maps, and nothing before it stopped the replay from reaching it"
    );
    assert_eq!(activity.signal(), &ActivitySignal::Working);
}

#[test]
fn copilot_and_pi_vocabularies_do_not_bleed_into_each_other() {
    // Two tails, two mappers. A Copilot line in a Pi log (or the reverse) is not evidence of
    // anything, and reading it as such would move a badge on a coincidence of spelling.
    assert_eq!(pi_event(r#"{"type":"user.message","data":{}}"#), None);
    assert_eq!(
        micold_daemon::activity::copilot_event(&line("turn_start")),
        None
    );
}

#[test]
fn a_session_with_no_activity_log_reads_unknown_rather_than_a_guess() {
    // FR-012d: the component was switched off, failed to load, or never wrote. "No evidence" is not
    // "idle", so the machine stays where it starts.
    let (activity, trace) = replay(&[]);
    assert!(trace.is_empty());
    assert_eq!(activity.signal(), &ActivitySignal::Unknown);
}

#[test]
fn a_turn_that_never_ended_is_resolved_by_supervision_not_by_the_log() {
    // The process was killed mid-turn: the log stops after `turn_start`. The log's silence is not
    // evidence, and the tail must not invent an ending from it — supervision already knows the
    // process is dead and applies `Ended`, exactly as for the other two CLIs.
    let (mut activity, _) = replay(&[line("turn_start")]);
    assert_eq!(activity.signal(), &ActivitySignal::Working);
    activity.apply(ActivityEvent::Ended {
        reason: "process exited".to_string(),
    });
    assert!(matches!(activity.signal(), ActivitySignal::Ended { .. }));
}
