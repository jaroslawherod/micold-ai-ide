//! Feature 1289, A4 (model): the setup command, a run's record and its transitions, the row tag
//! texts (contract U2) and the reason lines that end a run's output (SC-004).

use micold_core::setup::{Failure, RunId, SetupCommand, SetupRun, SetupState};

fn command(text: &str) -> SetupCommand {
    SetupCommand::parse(text).expect("a non-blank command parses")
}

fn running() -> SetupRun {
    SetupRun::start(RunId(7), command("mise install"), 100)
}

/// Every state a run can end in.
fn ended_states() -> Vec<SetupState> {
    vec![
        SetupState::Succeeded,
        SetupState::Failed(Failure::Exit(2)),
        SetupState::Failed(Failure::Signal(9)),
        SetupState::Failed(Failure::NotStarted("No such file or directory".into())),
        SetupState::Stopped,
        SetupState::Interrupted,
    ]
}

fn all_states() -> Vec<SetupState> {
    let mut states = ended_states();
    states.push(SetupState::Running);
    states
}

#[test]
fn a_blank_command_is_no_command() {
    assert_eq!(SetupCommand::parse(""), None);
    assert_eq!(SetupCommand::parse("  \n\t"), None);
}

#[test]
fn a_command_is_trimmed_and_keeps_its_inner_newlines() {
    assert_eq!(command("  a\nb  ").as_str(), "a\nb");
}

#[test]
fn a_started_run_is_running_and_has_no_end() {
    let run = running();
    assert_eq!(run.id(), RunId(7));
    assert_eq!(run.command().as_str(), "mise install");
    assert_eq!(run.state(), &SetupState::Running);
    assert_eq!(run.started_at(), 100);
    assert_eq!(run.ended_at(), None);
}

#[test]
fn a_running_run_ends_in_every_ended_state() {
    for to in ended_states() {
        let ended = running()
            .finish(to.clone(), 130)
            .unwrap_or_else(|refused| panic!("Running -> {to:?} refused: {refused:?}"));
        assert_eq!(ended.state(), &to);
        assert_eq!(ended.ended_at(), Some(130), "{to:?}");
        assert_eq!(ended.id(), RunId(7));
        assert_eq!(ended.started_at(), 100);
    }
}

#[test]
fn running_to_running_is_refused() {
    let refused = running().finish(SetupState::Running, 130).unwrap_err();
    assert_eq!(
        refused.run.state(),
        &SetupState::Running,
        "the run comes back as it was"
    );
    assert_eq!(refused.run.ended_at(), None);
}

#[test]
fn an_ended_run_never_changes_in_place() {
    for from in ended_states() {
        let ended = running().finish(from.clone(), 130).unwrap();
        for to in all_states() {
            let refused = ended
                .clone()
                .finish(to.clone(), 140)
                .expect_err(&format!("{from:?} -> {to:?} must be refused"));
            assert_eq!(
                refused.run, ended,
                "{from:?} -> {to:?} leaves the record as it was"
            );
        }
    }
}

#[test]
fn ended_at_is_set_exactly_when_the_run_is_not_running() {
    let run = running();
    assert_eq!(
        run.ended_at().is_some(),
        run.state() != &SetupState::Running
    );
    for to in ended_states() {
        let ended = running().finish(to, 130).unwrap();
        assert_eq!(
            ended.ended_at().is_some(),
            ended.state() != &SetupState::Running
        );
    }
}

#[test]
fn each_state_has_its_row_tag_text() {
    // Contract U2.
    assert_eq!(SetupState::Running.label(), "Setting up…");
    assert_eq!(SetupState::Succeeded.label(), "Set up");
    assert_eq!(SetupState::Failed(Failure::Exit(2)).label(), "Setup failed");
    assert_eq!(
        SetupState::Failed(Failure::Signal(9)).label(),
        "Setup failed"
    );
    assert_eq!(
        SetupState::Failed(Failure::NotStarted("x".into())).label(),
        "Setup failed"
    );
    assert_eq!(SetupState::Stopped.label(), "Setup stopped");
    assert_eq!(SetupState::Interrupted.label(), "Setup interrupted");
}

#[test]
fn each_ended_state_has_its_reason_line() {
    // SC-004: the reason is the output's last line.
    let line = |state: SetupState| state.reason_line();
    assert_eq!(
        line(SetupState::Failed(Failure::Exit(2))).as_deref(),
        Some("Exited with status 2.")
    );
    assert_eq!(
        line(SetupState::Failed(Failure::Signal(9))).as_deref(),
        Some("Ended by signal 9.")
    );
    assert_eq!(
        line(SetupState::Failed(Failure::NotStarted(
            "No such file or directory".into()
        )))
        .as_deref(),
        Some("Could not start: No such file or directory.")
    );
    assert_eq!(line(SetupState::Stopped).as_deref(), Some("Stopped."));
    assert_eq!(
        line(SetupState::Interrupted).as_deref(),
        Some("Interrupted: the daemon stopped.")
    );
    assert_eq!(
        line(SetupState::Succeeded).as_deref(),
        Some("Exited with status 0.")
    );
    assert_eq!(
        line(SetupState::Running),
        None,
        "a running run has no reason yet"
    );
}

#[test]
fn a_state_encodes_as_contract_c2_says() {
    assert_eq!(
        serde_json::to_value(SetupState::Failed(Failure::Exit(2))).unwrap(),
        serde_json::json!({"state": "failed", "failure": {"exit": 2}})
    );
    assert_eq!(
        serde_json::to_value(SetupState::Running).unwrap(),
        serde_json::json!({"state": "running"})
    );
    for state in all_states() {
        let json = serde_json::to_string(&state).unwrap();
        assert_eq!(
            serde_json::from_str::<SetupState>(&json).unwrap(),
            state,
            "{json}"
        );
    }
}

#[test]
fn a_state_round_trips_through_the_wire_codec() {
    // The wire is postcard, which cannot read an internally tagged enum.
    for state in all_states() {
        let bytes = postcard::to_allocvec(&state).unwrap();
        assert_eq!(postcard::from_bytes::<SetupState>(&bytes).unwrap(), state);
    }
}

#[test]
fn a_command_encodes_as_a_plain_string_and_a_blank_one_is_refused() {
    assert_eq!(
        serde_json::to_value(command("mise trust && mise install")).unwrap(),
        serde_json::json!("mise trust && mise install")
    );
    assert_eq!(
        serde_json::from_str::<SetupCommand>(r#"" a && b ""#).unwrap(),
        command("a && b")
    );
    assert!(serde_json::from_str::<SetupCommand>(r#""   ""#).is_err());
}

#[test]
fn a_run_record_round_trips_and_a_broken_one_is_refused() {
    let ended = running()
        .finish(SetupState::Failed(Failure::Exit(2)), 130)
        .unwrap();
    for run in [running(), ended] {
        let json = serde_json::to_string(&run).unwrap();
        assert_eq!(
            serde_json::from_str::<SetupRun>(&json).unwrap(),
            run,
            "{json}"
        );
        let bytes = postcard::to_allocvec(&run).unwrap();
        assert_eq!(postcard::from_bytes::<SetupRun>(&bytes).unwrap(), run);
    }
    // `ended_at` must be `Some` exactly when the run is not running.
    let running_with_end =
        r#"{"id":1,"command":"a","state":{"state":"running"},"started_at":1,"ended_at":2}"#;
    let ended_without_end =
        r#"{"id":1,"command":"a","state":{"state":"stopped"},"started_at":1,"ended_at":null}"#;
    assert!(serde_json::from_str::<SetupRun>(running_with_end).is_err());
    assert!(serde_json::from_str::<SetupRun>(ended_without_end).is_err());
}
