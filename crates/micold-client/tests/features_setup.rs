//! A worktree's setup run as the client keeps it, and where a project's setup command is sent
//! (feature 1289, M5: test A16; FR-015, FR-017, SC-004; contracts C3, U2, U5).

use std::collections::BTreeMap;
use std::path::PathBuf;

use micold_client::features::setup::{
    on_change, on_connect, row_tag, update, Effect, Msg, Names, State, Tone,
};
use micold_core::daemons::DaemonId;
use micold_core::local_time::UtcOffset;
use micold_core::session::SessionLocation;
use micold_core::settings::SetupCommandChange;
use micold_core::setup::{Failure, RunId, SetupCommand, SetupRun, SetupRunSummary, SetupState};

const HOST: DaemonId = DaemonId(1);
const CONTAINER: DaemonId = DaemonId(2);

fn command(text: &str) -> SetupCommand {
    SetupCommand::parse(text).unwrap()
}

fn running(id: u64) -> SetupRunSummary {
    SetupRunSummary {
        run: SetupRun::start(RunId(id), command("mise install"), 1_000),
        output_lines: 0,
    }
}

fn ended(id: u64, to: SetupState) -> SetupRunSummary {
    SetupRunSummary {
        run: SetupRun::start(RunId(id), command("mise install"), 1_000)
            .finish(to, 1_060)
            .unwrap(),
        output_lines: 3,
    }
}

fn wt(name: &str) -> SessionLocation {
    SessionLocation::Worktree(name.to_owned())
}

fn changed(location: SessionLocation, run: Option<SetupRunSummary>) -> Msg {
    Msg::Changed {
        daemon: HOST,
        project: PathBuf::from("/src/app"),
        names: Names {
            project: "app".to_owned(),
            location: match &location {
                SessionLocation::Worktree(d) => d.clone(),
                SessionLocation::Default => "Default".to_owned(),
            },
        },
        location,
        run,
    }
}

fn failed(code: i32) -> SetupState {
    SetupState::Failed(Failure::Exit(code))
}

#[test]
fn a_change_stores_and_replaces_the_summary_and_none_removes_it() {
    let mut state = State::default();
    let project = PathBuf::from("/src/app");
    assert_eq!(
        update(&mut state, changed(wt("feat-a"), Some(running(1)))),
        Effect::None
    );
    assert_eq!(
        state.summary(HOST, &project, &wt("feat-a")),
        Some(&running(1))
    );
    update(
        &mut state,
        changed(wt("feat-a"), Some(ended(1, SetupState::Succeeded))),
    );
    assert_eq!(
        state.summary(HOST, &project, &wt("feat-a")),
        Some(&ended(1, SetupState::Succeeded)),
        "the latest summary replaces the one before"
    );
    assert_eq!(
        state.summary(CONTAINER, &project, &wt("feat-a")),
        None,
        "keyed by daemon"
    );
    update(
        &mut state,
        changed(SessionLocation::Default, Some(running(2))),
    );
    update(&mut state, changed(wt("feat-a"), None));
    assert_eq!(
        state.summary(HOST, &project, &wt("feat-a")),
        None,
        "the record went away"
    );
    assert!(state
        .summary(HOST, &project, &SessionLocation::Default)
        .is_some());
}

#[test]
fn a_failed_run_notifies_once_with_the_reason() {
    let mut state = State::default();
    update(&mut state, changed(wt("feat-a"), Some(running(7))));
    assert_eq!(
        update(&mut state, changed(wt("feat-a"), Some(ended(7, failed(2))))),
        Effect::NotifyError("Setup failed in app / feat-a: Exited with status 2.".to_owned())
    );
    // The same run again, as the catch-up after a reconnect sends it.
    assert_eq!(
        update(&mut state, changed(wt("feat-a"), Some(ended(7, failed(2))))),
        Effect::None,
        "one notification per run"
    );
}

#[test]
fn a_failure_learned_in_the_catch_up_notifies_once() {
    let mut state = State::default();
    let catch_up = || changed(SessionLocation::Default, Some(ended(3, failed(1))));
    assert!(
        matches!(update(&mut state, catch_up()), Effect::NotifyError(text)
        if text == "Setup failed in app / Default: Exited with status 1.")
    );
    assert_eq!(update(&mut state, catch_up()), Effect::None);
}

#[test]
fn a_start_failure_names_its_reason() {
    let mut state = State::default();
    let effect = update(
        &mut state,
        changed(
            wt("feat-a"),
            Some(ended(
                4,
                SetupState::Failed(Failure::NotStarted("No such file".into())),
            )),
        ),
    );
    assert_eq!(
        effect,
        Effect::NotifyError("Setup failed in app / feat-a: Could not start: No such file.".into())
    );
}

#[test]
fn stopped_interrupted_and_succeeded_runs_notify_nothing() {
    let mut state = State::default();
    for (id, to) in [
        (1, SetupState::Stopped),
        (2, SetupState::Interrupted),
        (3, SetupState::Succeeded),
    ] {
        assert_eq!(
            update(
                &mut state,
                changed(wt("feat-a"), Some(ended(id, to.clone())))
            ),
            Effect::None,
            "{to:?}"
        );
    }
}

#[test]
fn a_worktree_recreated_under_the_same_name_notifies_again() {
    let mut state = State::default();
    assert!(matches!(
        update(&mut state, changed(wt("feat-a"), Some(ended(5, failed(2))))),
        Effect::NotifyError(_)
    ));
    update(&mut state, changed(wt("feat-a"), None));
    assert!(
        matches!(
            update(&mut state, changed(wt("feat-a"), Some(ended(9, failed(2))))),
            Effect::NotifyError(_)
        ),
        "a new run in the recreated worktree is a new failure"
    );
}

#[test]
fn the_row_tag_says_the_state_in_words_with_its_tint() {
    let utc = |_| UtcOffset::UTC;
    let cases = [
        (
            running(1),
            "Setting up…",
            "Setup running since 1970-01-01 00:16",
            Tone::Neutral,
        ),
        (
            ended(1, SetupState::Succeeded),
            "Set up",
            "Setup finished at 1970-01-01 00:17",
            Tone::Neutral,
        ),
        (
            ended(1, failed(2)),
            "Setup failed",
            "Setup failed at 1970-01-01 00:17: Exited with status 2",
            Tone::Error,
        ),
        (
            ended(1, SetupState::Stopped),
            "Setup stopped",
            "Setup stopped at 1970-01-01 00:17",
            Tone::Neutral,
        ),
        (
            ended(1, SetupState::Interrupted),
            "Setup interrupted",
            "Setup interrupted: the daemon stopped",
            Tone::Warning,
        ),
    ];
    for (summary, label, tooltip, tone) in cases {
        let tag = row_tag(&summary, utc);
        assert_eq!(
            (tag.label, tag.tooltip.as_str(), tag.tone),
            (label, tooltip, tone)
        );
    }
    let east = row_tag(&running(1), |_| UtcOffset::east(3600).unwrap());
    assert_eq!(
        east.tooltip, "Setup running since 1970-01-01 01:16",
        "local time"
    );
}

fn commands() -> BTreeMap<String, SetupCommand> {
    BTreeMap::from([
        ("/src/app".to_owned(), command("mise install")),
        ("/src/lib".to_owned(), command("make deps")),
    ])
}

#[test]
fn on_connect_each_held_project_with_a_command_is_sent_once() {
    let held = [
        PathBuf::from("/src/app"),
        PathBuf::from("/src/other"),
        PathBuf::from("/src/app"),
    ];
    assert_eq!(
        on_connect(&commands(), held),
        vec![(PathBuf::from("/src/app"), command("mise install"))],
        "a project without a command is not sent; one held twice is sent once"
    );
    assert_eq!(
        on_connect(&BTreeMap::new(), [PathBuf::from("/src/app")]),
        vec![]
    );
}

#[test]
fn a_changed_command_goes_to_each_daemon_that_holds_the_project_with_its_catalog_path() {
    let catalogs = BTreeMap::from([
        (
            HOST,
            vec![PathBuf::from("/src/app"), PathBuf::from("/src/lib")],
        ),
        // A container daemon whose paths are mapped from the host (`links::PATHS_MAPPED`): what is
        // sent is the path its own catalog holds.
        (CONTAINER, vec![PathBuf::from("/src/app")]),
        (DaemonId(3), vec![PathBuf::from("/src/lib")]),
    ]);
    let set = SetupCommandChange {
        project: "/src/app".to_owned(),
        command: Some(command("mise trust && mise install")),
    };
    assert_eq!(
        on_change(&set, &catalogs),
        vec![
            (HOST, PathBuf::from("/src/app"), set.command.clone()),
            (CONTAINER, PathBuf::from("/src/app"), set.command.clone()),
        ],
        "daemon 3 does not hold the project and is told nothing"
    );
    let cleared = SetupCommandChange {
        project: "/src/lib".to_owned(),
        command: None,
    };
    assert_eq!(
        on_change(&cleared, &catalogs),
        vec![
            (HOST, PathBuf::from("/src/lib"), None),
            (DaemonId(3), PathBuf::from("/src/lib"), None),
        ]
    );
    assert_eq!(
        on_change(&set, &BTreeMap::new()),
        vec![],
        "no daemon connected"
    );
}
