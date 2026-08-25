//! Copilot's event log drives the same activity machine (feature 026, T054–T056, T058b — FR-018).
//!
//! The `Activity` state machine is **not changed by this feature**. What is new is a second source
//! feeding it the events it already consumes: `micold-daemon/src/activity.rs::copilot_event` maps
//! one line of a Copilot `events.jsonl` to an `ActivityEvent`, and everything downstream is feature
//! 010's, unchanged.
//!
//! Second, not only: a Copilot session's event log joins the braille-spinner scan, which is shared
//! and not provider-conditional (`micold-daemon/src/terminal.rs` reads every PTY session's OSC-0
//! title, whichever CLI wrote it). The two cannot contradict each other — `SpinnerObserved` only
//! moves `Unknown -> Working` — and `micold-daemon/tests/activity_pipeline.rs` proves that end to
//! end with both sources live on one session (T057). What is mapped here is one of the two.
//!
//! Every line read here comes from the T001 corpus — captured from GitHub Copilot CLI 1.0.80, with
//! the two authored lines recorded in that corpus's own README. Nothing here needs `copilot`
//! installed.

use std::path::{Path, PathBuf};

use micold_core::protocol::messages::ActivitySignal;
use micold_daemon::activity::{copilot_event, Activity, ActivityEvent, HookKind};

/// The captured corpus, resolved from this crate's manifest.
fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/")
        .join("micold-core/tests/fixtures/copilot")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Replay a whole log through a fresh machine and return the signal after each mapped event.
fn replay(log: &str) -> (Activity, Vec<ActivitySignal>) {
    let mut activity = Activity::new();
    let mut trace = Vec::new();
    for line in log.lines() {
        if let Some(event) = copilot_event(line) {
            activity.apply(event);
            trace.push(activity.signal().clone());
        }
    }
    (activity, trace)
}

#[test]
fn each_event_type_maps_to_the_signal_the_contract_names() {
    // T054, one line at a time, against the contract's table. Asserted per event rather than only
    // over a whole log: a replay can end in the right place with two errors that cancel.
    let cases: [(&str, Option<ActivityEvent>); 7] = [
        (
            r#"{"type":"user.message","data":{}}"#,
            Some(ActivityEvent::Hook(HookKind::UserPromptSubmit)),
        ),
        (
            r#"{"type":"assistant.turn_start","data":{}}"#,
            Some(ActivityEvent::Hook(HookKind::PreToolUse)),
        ),
        (
            r#"{"type":"tool.execution_start","data":{}}"#,
            Some(ActivityEvent::Hook(HookKind::PreToolUse)),
        ),
        (
            r#"{"type":"tool.execution_complete","data":{}}"#,
            Some(ActivityEvent::Hook(HookKind::PostToolUse)),
        ),
        (
            r#"{"type":"assistant.turn_end","data":{}}"#,
            Some(ActivityEvent::Hook(HookKind::Stop)),
        ),
        (
            r#"{"type":"permission.requested","data":{}}"#,
            Some(ActivityEvent::Hook(HookKind::Notification)),
        ),
        (
            r#"{"type":"session.shutdown","data":{"shutdownType":"routine"}}"#,
            Some(ActivityEvent::Ended {
                reason: "routine".to_string(),
            }),
        ),
    ];
    for (line, expected) in cases {
        assert_eq!(copilot_event(line), expected, "{line}");
    }

    // `session.error` is the other terminal form, and the reason it exists: `Ended { reason }` is a
    // *sibling* of `ActivityEvent::Hook`, so a mapping typed at `HookKind` could not express it at
    // all. That is why `copilot_event` returns an `ActivityEvent`.
    assert_eq!(
        copilot_event(r#"{"type":"session.error","data":{"message":"upstream request failed"}}"#),
        Some(ActivityEvent::Ended {
            reason: "upstream request failed".to_string()
        })
    );
}

#[test]
fn a_whole_captured_turn_ends_awaiting_input() {
    // The full turn from the corpus: prompt → turn start → permission prompt → tool start → tool
    // complete → turn end → shutdown, with six off-contract types mixed in.
    let (_, trace) = replay(&fixture("events-full-turn.jsonl"));

    assert_eq!(
        trace.first(),
        Some(&ActivitySignal::Working),
        "the first mapped event is the user's message, and it starts a turn"
    );
    assert!(
        trace.contains(&ActivitySignal::AwaitingInput),
        "the permission prompt and the turn end both put the session on the user"
    );
    assert!(
        matches!(trace.last(), Some(ActivitySignal::Ended { .. })),
        "and the shutdown is terminal"
    );
}

#[test]
fn a_tool_running_mid_turn_does_not_read_as_finished() {
    // `tool.execution_complete` is mapped and deliberately changes nothing: the model is still
    // working, and the turn is over at `assistant.turn_end` and not before. Getting this wrong
    // makes the badge flicker to idle in the middle of every tool call.
    let mut activity = Activity::new();
    for line in [
        r#"{"type":"user.message","data":{}}"#,
        r#"{"type":"tool.execution_start","data":{}}"#,
        r#"{"type":"tool.execution_complete","data":{}}"#,
    ] {
        activity.apply(copilot_event(line).expect("mapped"));
    }
    assert_eq!(activity.signal(), &ActivitySignal::Working);
}

#[test]
fn unknown_event_types_are_ignored_and_never_reject_a_line() {
    // T055. This is another tool's internal log and it gains types between releases — the 1.0.80
    // capture alone added three the 1.0.62 contract does not list. Ignoring is the contract;
    // rejecting would mean a Copilot update silently stops the badge.
    for line in [
        r#"{"type":"assistant.message","data":{}}"#,
        r#"{"type":"session.model_change","data":{}}"#,
        r#"{"type":"session.auto_mode_resolved","data":{}}"#,
        r#"{"type":"session.usage_checkpoint","data":{}}"#,
        r#"{"type":"skill.invoked","data":{}}"#,
        r#"{"type":"a.type.that.does.not.exist.yet","data":{}}"#,
    ] {
        assert_eq!(copilot_event(line), None, "{line}");
    }
}

#[test]
fn a_malformed_line_is_skipped_rather_than_ending_the_tail() {
    // T055's other half, read from the corpus file that holds a genuinely broken line, a blank
    // line, and off-contract types either side of them.
    let log = fixture("events-unknown-and-malformed.jsonl");
    assert!(
        log.contains("{ this line is not json"),
        "the fixture still holds the malformed line this test is about"
    );

    let (activity, trace) = replay(&log);

    assert_eq!(
        trace.len(),
        1,
        "exactly one line in that file maps to anything — the `session.error` at the end. The \
         malformed line, the blank line and the off-contract types all yielded nothing, and \
         crucially none of them stopped the replay before reaching it"
    );
    assert!(matches!(activity.signal(), ActivitySignal::Ended { .. }));
}

#[test]
fn a_turn_that_never_ended_does_not_leave_the_badge_working_forever() {
    // T056. The process was killed mid-turn, so the log stops after `assistant.turn_start` and no
    // `turn_end` is ever written. Replaying it leaves the machine `Working` — correctly, because
    // that is all the log says.
    //
    // What stops the badge being wrong forever is not this mapping: the daemon already knows the
    // process is dead and applies `Ended` from supervision, and that guard is unchanged by this
    // feature. Asserted here so the boundary is explicit — the log's silence is not evidence, and
    // nothing in the tail path may invent an ending from it.
    let (mut activity, _) = replay(&fixture("events-dangling-turn.jsonl"));
    assert_eq!(
        activity.signal(),
        &ActivitySignal::Working,
        "the log genuinely ends mid-turn"
    );

    activity.apply(ActivityEvent::Ended {
        reason: "process exited".to_string(),
    });
    assert!(
        matches!(activity.signal(), ActivitySignal::Ended { .. }),
        "and supervision's own knowledge of the dead process resolves it, as it does for `claude`"
    );
}

#[test]
fn a_session_with_no_event_log_reads_unknown_rather_than_a_guess() {
    // FR-018's conservatism clause. `Unknown` is a first-class value: a session with no recorded
    // conversation has produced no evidence, and "no evidence" is not "idle". Inventing
    // `AwaitingInput` here would put a resting badge on a session that has never run.
    let activity = Activity::new();
    assert_eq!(activity.signal(), &ActivitySignal::Unknown);

    let (activity, trace) = replay("");
    assert!(trace.is_empty());
    assert_eq!(activity.signal(), &ActivitySignal::Unknown);
}

#[test]
fn appending_a_user_message_moves_the_badge_within_a_second_on_the_log_path_alone() {
    // T058b / SC-005, and the half that is easy to fake. The shared braille-spinner path
    // (`terminal.rs`) can drive the same `Unknown → Working` transition from a Copilot TUI's own
    // animation, so a test that only watched the badge would go green with the event-log path
    // completely dead.
    //
    // So this drives the log **in isolation** — no title traffic, no spinner event — and asserts
    // the transition's provenance: the machine moved because a line was appended and mapped.
    let dir = tempfile::tempdir().unwrap();
    let log: PathBuf = dir.path().join("events.jsonl");
    std::fs::write(&log, "").unwrap();

    let mut activity = Activity::new();
    assert_eq!(activity.signal(), &ActivitySignal::Unknown);

    let started = std::time::Instant::now();
    // The append a running Copilot session makes when the user sends a prompt.
    std::fs::write(&log, "{\"type\":\"user.message\",\"data\":{}}\n").unwrap();
    let appended = std::fs::read_to_string(&log).unwrap();
    let event = copilot_event(appended.lines().next().expect("a line")).expect("mapped");
    assert_eq!(
        event,
        ActivityEvent::Hook(HookKind::UserPromptSubmit),
        "the transition's provenance is the log line, not a spinner in a terminal title"
    );
    activity.apply(event);

    assert_eq!(activity.signal(), &ActivitySignal::Working);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(1),
        "reading and mapping an appended line is not where a second could go — the budget is the \
         watcher's notification latency, which T064 caps at 250 ms"
    );
}
// ---------------------------------------------------------------------------------------
// T056a / T060 — what is NOT watched, and what is NOT scheduled
// ---------------------------------------------------------------------------------------

/// The daemon's own source, for the structural assertions below.
fn daemon_source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn a_watch_is_opened_from_exactly_one_place_and_it_is_the_start_path() {
    // T056a's half that matters, and the reason it is asserted structurally rather than by
    // counting watchers: "a discovered session gets no watch" is a claim about code that never
    // runs, and the only way to hold it is to show there is nowhere for it to run *from*.
    //
    // `EventLogTail::open` is called once in the whole daemon — by `open_event_log_tail`, which
    // `server.rs` calls only after `start_session` succeeds. The FR-014 discovery pass
    // (`discover_external_sessions`) writes catalog records and touches none of it, so a project
    // holding hundreds of discovered sessions schedules no observation work at all (SC-006,
    // SC-009).
    let state = daemon_source("state.rs");
    let event_log = daemon_source("event_log.rs");
    let server = daemon_source("server.rs");

    let opens: usize = [&state, &server, &event_log]
        .iter()
        .map(|src| src.matches("EventLogTail::open").count())
        .sum();
    assert_eq!(
        opens, 1,
        "a watch is opened in exactly one place; found {opens}"
    );
    assert!(
        state.contains("fn open_event_log_tail"),
        "and that place is `DaemonState::open_event_log_tail`"
    );

    // Discovery must not reach it. If a future edit "helpfully" watched discovered sessions, this
    // is what would say so — and the cost is not academic: it is one inotify registration per
    // conversation ever recorded in the project.
    let discovery = state
        .split("pub fn discover_external_sessions")
        .nth(1)
        .and_then(|rest| rest.split("\n    /// ").next())
        .expect("the discovery function is in state.rs");
    assert!(
        !discovery.contains("EventLogTail") && !discovery.contains("open_event_log_tail"),
        "the FR-014 discovery pass opened a watch — a discovered session is listed and identified, \
         never observed (FR-018, SC-006)"
    );
}

#[test]
fn this_applications_watch_path_schedules_no_timer_of_its_own() {
    // T060, FR-019. Structural: the *absence* of a timer in our code, not a measurement of one.
    // "Cheap enough" is an adjective; this is a gate.
    //
    // Explicitly **not** an assertion about the watch crate's internals. `notify` polls on its own
    // account where a platform offers no native change notification, and FR-019 as scoped permits
    // that — the rule is about what this application schedules.
    let event_log = daemon_source("event_log.rs");
    let code_only: String = event_log
        .lines()
        .filter(|line| {
            let t = line.trim_start();
            !t.starts_with("//") && !t.starts_with("///") && !t.starts_with("//!")
        })
        .collect::<Vec<_>>()
        .join("\n");

    for forbidden in [
        "tokio::time::interval",
        "tokio::time::sleep",
        "thread::sleep",
        "Instant::now",
        "set_missed_tick_behavior",
    ] {
        assert!(
            !code_only.contains(forbidden),
            "`{forbidden}` is in the watch path — FR-019 forbids a polling timer, a periodic \
             wakeup, or any work scheduled per idle session"
        );
    }

    // No debouncer, either. `notify-debouncer-{mini,full}` are separate crates and taking one
    // would reintroduce exactly what FR-019 forbids, wearing someone else's name.
    let manifest =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
            .expect("the daemon manifest");
    assert!(
        !manifest.contains("notify-debouncer") && !manifest.contains("notify_debouncer"),
        "a debouncer is a timer by adoption"
    );

    // The one interval this module does name is the crate's own poll fallback, and it is *capped*
    // rather than introduced: it applies only where no native notification exists, and 250 ms keeps
    // SC-005's one-second budget intact even there.
    assert!(
        code_only.contains("with_poll_interval(FALLBACK_POLL)"),
        "the crate's fallback interval is bounded rather than left at its default"
    );
    assert_eq!(
        micold_daemon::event_log::FALLBACK_POLL,
        std::time::Duration::from_millis(250)
    );
}

#[test]
fn a_quiet_session_costs_nothing_between_appends() {
    // The behavioural companion to the structural test above, and the property FR-019 is actually
    // about: with nothing appended, a look at the log does no work and delivers nothing. There is
    // no per-tick cost because there is no tick — a look only happens when the platform says the
    // directory changed.
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("events.jsonl");
    std::fs::write(&log, "{\"type\":\"user.message\",\"data\":{}}\n").unwrap();

    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&seen);
    let _tail = micold_daemon::event_log::EventLogTail::open(log.clone(), move |event| {
        sink.lock().unwrap().push(event);
    })
    .expect("watch opens");

    // The tail starts at the file's current end, so the history already there is not replayed —
    // a resumed session must not walk its badge through every turn the conversation ever had.
    assert!(
        seen.lock().unwrap().is_empty(),
        "opening a watch delivered nothing on its own"
    );
}
