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
fn a_run_running_at_attach_notifies_when_it_fails() {
    let mut state = State::default();
    // The catch-up after an attach shows the run still going; it then fails while connected.
    update(
        &mut state,
        changed(SessionLocation::Default, Some(running(3))),
    );
    assert!(matches!(
        update(&mut state, changed(SessionLocation::Default, Some(ended(3, failed(1))))),
        Effect::NotifyError(text) if text == "Setup failed in app / Default: Exited with status 1."
    ));
}

#[test]
fn a_failure_already_over_when_the_client_attaches_does_not_notify() {
    // M5 review A-F3: the daemon replays its persisted records on every attach, so a run that
    // failed before this client saw it running is history, not news: each launch would otherwise
    // post it again (SC-004 says exactly one).
    let mut state = State::default();
    let catch_up = || changed(SessionLocation::Default, Some(ended(3, failed(1))));
    assert_eq!(update(&mut state, catch_up()), Effect::None);
    assert_eq!(update(&mut state, catch_up()), Effect::None);
    assert_eq!(
        state.summary(HOST, &PathBuf::from("/src/app"), &SessionLocation::Default),
        Some(&ended(3, failed(1))),
        "the row still shows it"
    );
    // A different run seen failed without being seen running is history too.
    update(
        &mut state,
        changed(SessionLocation::Default, Some(running(4))),
    );
    assert_eq!(
        update(
            &mut state,
            changed(SessionLocation::Default, Some(ended(5, failed(1))))
        ),
        Effect::None
    );
}

#[test]
fn a_start_failure_names_its_reason() {
    let mut state = State::default();
    update(&mut state, changed(wt("feat-a"), Some(running(4))));
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
    update(&mut state, changed(wt("feat-a"), Some(running(5))));
    assert!(matches!(
        update(&mut state, changed(wt("feat-a"), Some(ended(5, failed(2))))),
        Effect::NotifyError(_)
    ));
    update(&mut state, changed(wt("feat-a"), None));
    update(&mut state, changed(wt("feat-a"), Some(running(9))));
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
fn on_connect_each_held_project_is_sent_its_command_or_none_once() {
    let held = [
        PathBuf::from("/src/app"),
        PathBuf::from("/src/other"),
        PathBuf::from("/src/app"),
    ];
    assert_eq!(
        on_connect(&commands(), held),
        vec![
            (PathBuf::from("/src/app"), Some(command("mise install"))),
            // M5 review A-F2: a command cleared while the daemon was away is cleared there too.
            (PathBuf::from("/src/other"), None),
        ],
        "every held project is told its command, or that it has none; one held twice once"
    );
    assert_eq!(
        on_connect(&BTreeMap::new(), [PathBuf::from("/src/app")]),
        vec![(PathBuf::from("/src/app"), None)]
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

// ---- M6: the setup output view (test A16 view; FR-016, FR-018, FR-021, FR-024; U4, U6) ----

use micold_client::features::setup::{
    delete_warns_setup, output_offered, rewatch, sync_watch, Key, Request, ROW_HEIGHT,
};
use micold_client::features::structured_transcript::Follow;

fn key(location: SessionLocation) -> Key {
    (HOST, PathBuf::from("/src/app"), location)
}

fn output(run: u64, first_line: u64, lines: &[&str]) -> Msg {
    output_dropped(run, first_line, 0, lines)
}

/// A batch whose daemon no longer keeps the run's first `dropped` lines.
fn output_dropped(run: u64, first_line: u64, dropped: u64, lines: &[&str]) -> Msg {
    Msg::Output {
        daemon: HOST,
        project: PathBuf::from("/src/app"),
        location: wt("feat-a"),
        run: RunId(run),
        first_line,
        lines: lines.iter().map(|l| (*l).to_owned()).collect(),
        dropped,
    }
}

fn scrolled(offset: u32, viewport: u32, content: u32) -> Msg {
    Msg::Scrolled {
        offset,
        viewport,
        content,
    }
}

fn follow(state: &State) -> Follow {
    state.view.as_ref().unwrap().follow
}

fn opened() -> State {
    let mut state = State::default();
    update(&mut state, Msg::ViewOpened(key(wt("feat-a"))));
    state
}

fn held(state: &State) -> (u64, Vec<String>) {
    let view = state.view.as_ref().expect("the view is open");
    (view.first_line, view.lines.clone())
}

#[test]
fn opening_the_view_watches_from_line_0_and_closing_it_unwatches() {
    let mut state = opened();
    assert_eq!(
        sync_watch(&mut state),
        vec![Request::Watch {
            key: key(wt("feat-a")),
            from_line: 0
        }]
    );
    assert_eq!(sync_watch(&mut state), vec![], "a watch is sent once");
    update(&mut state, Msg::ViewClosed);
    assert_eq!(state.view, None);
    assert_eq!(
        sync_watch(&mut state),
        vec![Request::Unwatch {
            key: key(wt("feat-a"))
        }]
    );
    assert_eq!(sync_watch(&mut state), vec![]);
}

#[test]
fn opening_another_location_unwatches_the_first_then_watches_the_second() {
    let mut state = opened();
    sync_watch(&mut state);
    update(&mut state, Msg::ViewOpened(key(SessionLocation::Default)));
    assert_eq!(
        sync_watch(&mut state),
        vec![
            Request::Unwatch {
                key: key(wt("feat-a"))
            },
            Request::Watch {
                key: key(SessionLocation::Default),
                from_line: 0
            },
        ]
    );
}

#[test]
fn contiguous_batches_append_and_an_overlap_is_not_held_twice() {
    let mut state = opened();
    update(&mut state, output(1, 0, &["a", "b"]));
    update(&mut state, output(1, 2, &["c"]));
    update(&mut state, output(1, 1, &["b", "c", "d"]));
    assert_eq!(
        held(&state),
        (0, vec!["a".into(), "b".into(), "c".into(), "d".into()])
    );
    assert_eq!(state.view.as_ref().unwrap().dropped(), 0);
}

#[test]
fn a_gap_in_first_line_shows_as_dropped_lines() {
    let mut state = opened();
    update(&mut state, output(1, 0, &["a", "b"]));
    update(&mut state, output(1, 7, &["h"]));
    assert_eq!(held(&state), (7, vec!["h".into()]));
    assert_eq!(state.view.as_ref().unwrap().dropped(), 7);
    // A backlog that starts past line 0: the daemon kept only its last lines (FR-021).
    let mut state = opened();
    update(&mut state, output_dropped(1, 12, 12, &["m"]));
    assert_eq!(state.view.as_ref().unwrap().dropped(), 12);
}

#[test]
fn the_view_keeps_at_most_the_last_10000_lines() {
    let mut state = opened();
    let lines: Vec<String> = (0..10_005).map(|i| i.to_string()).collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    update(&mut state, output(1, 0, &lines));
    let view = state.view.as_ref().unwrap();
    assert_eq!(view.lines.len(), 10_000);
    assert_eq!(view.dropped(), 5);
    assert_eq!(view.lines[0], "5");
}

#[test]
fn a_batch_for_a_new_run_resets_the_lines() {
    let mut state = opened();
    update(&mut state, output(1, 0, &["a", "b", "c"]));
    update(&mut state, output(2, 0, &["x"]));
    assert_eq!(held(&state), (0, vec!["x".into()]));
    assert_eq!(state.view.as_ref().unwrap().run, Some(RunId(2)));
}

#[test]
fn output_for_another_location_or_with_no_view_open_is_ignored() {
    let mut state = State::default();
    assert_eq!(update(&mut state, output(1, 0, &["a"])), Effect::None);
    let mut state = State::default();
    update(&mut state, Msg::ViewOpened(key(SessionLocation::Default)));
    update(&mut state, output(1, 0, &["a"]));
    assert_eq!(held(&state), (0, vec![]));
}

#[test]
fn following_scrolls_to_each_new_batch_and_scrolling_up_stops_it_until_the_bottom() {
    let mut state = opened();
    assert_eq!(state.view.as_ref().unwrap().follow, Follow::Following);
    assert_eq!(
        update(&mut state, output(1, 0, &["a"; 40])),
        Effect::ScrollToEnd
    );
    let content = (40.0 * ROW_HEIGHT) as u32;
    // Scrolled up: the follow stops and new lines move nothing.
    update(&mut state, scrolled(content - 200, 200, content));
    update(&mut state, scrolled(0, 200, content));
    assert_eq!(follow(&state), Follow::Detached { offset: 0 });
    assert_eq!(update(&mut state, output(1, 40, &["b"])), Effect::None);
    // Back at the bottom: it resumes.
    let content = content + ROW_HEIGHT as u32;
    update(&mut state, scrolled(content - 200, 200, content));
    assert_eq!(state.view.as_ref().unwrap().follow, Follow::Following);
    assert_eq!(
        update(&mut state, output(1, 41, &["c"])),
        Effect::ScrollToEnd
    );
}

#[test]
fn follow_toggled_stops_the_follow_and_starts_it_again_at_the_end() {
    let mut state = opened();
    assert_eq!(update(&mut state, Msg::FollowToggled), Effect::None);
    assert!(matches!(
        state.view.as_ref().unwrap().follow,
        Follow::Detached { .. }
    ));
    assert_eq!(update(&mut state, Msg::FollowToggled), Effect::ScrollToEnd);
    assert_eq!(state.view.as_ref().unwrap().follow, Follow::Following);
}

#[test]
fn copy_yields_the_kept_lines_joined_with_newlines() {
    let mut state = opened();
    update(
        &mut state,
        output_dropped(1, 3, 3, &["one", "two", "Exited with status 2."]),
    );
    assert_eq!(
        update(&mut state, Msg::CopyRequested),
        Effect::Copy("one\ntwo\nExited with status 2.".to_owned())
    );
    assert_eq!(
        update(&mut State::default(), Msg::CopyRequested),
        Effect::None
    );
}

#[test]
fn a_reconnect_with_the_view_open_watches_again_from_the_last_line_held() {
    let mut state = opened();
    sync_watch(&mut state);
    update(&mut state, output(1, 0, &["a", "b", "c"]));
    assert_eq!(
        rewatch(&mut state, HOST),
        Some(Request::Watch {
            key: key(wt("feat-a")),
            from_line: 3
        })
    );
    assert_eq!(
        rewatch(&mut state, CONTAINER),
        None,
        "another daemon's view"
    );
    update(&mut state, Msg::ViewClosed);
    assert_eq!(rewatch(&mut state, HOST), None);
}

#[test]
fn the_record_going_away_closes_the_view() {
    let mut state = opened();
    update(&mut state, changed(wt("feat-a"), Some(running(1))));
    update(&mut state, changed(wt("other"), None));
    assert!(state.view.is_some());
    update(&mut state, changed(wt("feat-a"), None));
    assert_eq!(state.view, None);
}

#[test]
fn output_is_offered_exactly_when_a_record_exists() {
    assert!(!output_offered(None));
    assert!(output_offered(Some(&running(1))));
    assert!(output_offered(Some(&ended(1, failed(2)))));
}

#[test]
fn the_delete_line_shows_exactly_while_setup_runs() {
    assert!(delete_warns_setup(Some(&running(1))));
    assert!(!delete_warns_setup(None));
    for to in [
        SetupState::Succeeded,
        failed(1),
        SetupState::Stopped,
        SetupState::Interrupted,
    ] {
        assert!(!delete_warns_setup(Some(&ended(1, to.clone()))), "{to:?}");
    }
}

// ---- M6 review round 1 (A-F1, A-F2, A-F3, A-F4, V-F1) ----

/// A-F1: a watch resumed after a reconnect from a line of the run the view held, answered with a
/// newer run, asks again from the first line the daemon keeps: the lines it holds are shown, not
/// counted as not kept.
#[test]
fn a_resumed_watch_answered_by_a_new_run_asks_again_from_the_first_kept_line() {
    let mut state = opened();
    sync_watch(&mut state);
    let lines: Vec<String> = (0..500).map(|i| format!("old {i}")).collect();
    let lines: Vec<&str> = lines.iter().map(String::as_str).collect();
    update(&mut state, output(7, 0, &lines));
    assert_eq!(
        rewatch(&mut state, HOST),
        Some(Request::Watch {
            key: key(wt("feat-a")),
            from_line: 500
        })
    );
    // Run 8 started while the daemon was away; it holds all 800 of its lines.
    assert_eq!(
        update(&mut state, output(8, 500, &["new 500"])),
        Effect::Watch(Request::Watch {
            key: key(wt("feat-a")),
            from_line: 0
        })
    );
    // A live batch sent before the new watch was answered is in its answer.
    update(&mut state, output(8, 501, &["new 501"]));
    update(&mut state, output(8, 0, &["new 0", "new 1"]));
    let view = state.view.as_ref().unwrap();
    assert_eq!(view.run, Some(RunId(8)));
    assert_eq!(held(&state), (0, vec!["new 0".into(), "new 1".into()]));
    assert_eq!(
        state.view.as_ref().unwrap().dropped(),
        0,
        "nothing shown as not kept"
    );

    // When the daemon dropped run 8's first lines, it asks from the first one kept.
    let mut state = opened();
    update(&mut state, output(7, 0, &["old"; 50]));
    rewatch(&mut state, HOST);
    assert_eq!(
        update(&mut state, output_dropped(8, 50, 20, &["new 50"])),
        Effect::Watch(Request::Watch {
            key: key(wt("feat-a")),
            from_line: 20
        })
    );
    update(&mut state, output_dropped(8, 20, 20, &["new 20"]));
    assert_eq!(held(&state), (20, vec!["new 20".into()]));
}

/// A-F2: a live batch sent under an earlier watch, landing before the backlog a reopened view
/// asked for, is not taken as the start of the output.
#[test]
fn a_live_batch_before_the_backlog_of_a_reopened_view_is_ignored() {
    let mut state = opened();
    sync_watch(&mut state);
    update(&mut state, Msg::ViewClosed);
    sync_watch(&mut state);
    update(&mut state, Msg::ViewOpened(key(wt("feat-a"))));
    sync_watch(&mut state);
    update(&mut state, output(1, 500, &["late"]));
    update(&mut state, output(1, 0, &["a", "b"]));
    assert_eq!(held(&state), (0, vec!["a".into(), "b".into()]));
    update(&mut state, output(1, 2, &["c"]));
    assert_eq!(held(&state), (0, vec!["a".into(), "b".into(), "c".into()]));
    assert_eq!(state.view.as_ref().unwrap().dropped(), 0);
}

/// Choosing **Setup output** again for the location shown keeps the view and its watch.
#[test]
fn opening_the_location_already_shown_keeps_the_view() {
    let mut state = opened();
    sync_watch(&mut state);
    update(&mut state, output(1, 0, &["a"]));
    update(&mut state, Msg::ViewOpened(key(wt("feat-a"))));
    assert_eq!(sync_watch(&mut state), vec![]);
    update(&mut state, output(1, 1, &["b"]));
    assert_eq!(held(&state), (0, vec!["a".into(), "b".into()]));
}

/// A-F3: a scroll report is judged against the content the list measured, so one from a frame
/// before the latest batch was laid out does not stop the follow.
#[test]
fn a_report_at_the_end_of_the_content_it_measured_keeps_following() {
    let mut state = opened();
    update(&mut state, output(1, 0, &["a"; 500]));
    update(&mut state, scrolled(9_800, 200, 10_000));
    update(&mut state, output(1, 500, &["b"; 500]));
    // Laid out before the second batch: at the end of the 500 rows it measured.
    update(&mut state, scrolled(9_800, 200, 10_000));
    assert_eq!(follow(&state), Follow::Following);
}

/// V-F1: a report with a new viewport height is a relayout (the window was resized), not the user
/// scrolling: a following view keeps following and snaps back to the end.
#[test]
fn a_resize_while_following_keeps_the_follow_and_snaps_to_the_end() {
    let mut state = opened();
    update(&mut state, output(1, 0, &["a"; 100]));
    let content = (100.0 * ROW_HEIGHT) as u32;
    update(&mut state, scrolled(content - 480, 480, content));
    assert_eq!(
        update(&mut state, scrolled(0, 180, content)),
        Effect::ScrollToEnd
    );
    assert_eq!(follow(&state), Follow::Following);
    assert_eq!(
        update(&mut state, output(1, 100, &["b"])),
        Effect::ScrollToEnd
    );
    // The user scrolling away at the new size still stops it.
    update(&mut state, scrolled(content + 20 - 180, 180, content + 20));
    update(&mut state, scrolled(0, 180, content + 20));
    assert_eq!(follow(&state), Follow::Detached { offset: 0 });
    // A resize while detached leaves it detached.
    assert_eq!(
        update(&mut state, scrolled(0, 300, content + 20)),
        Effect::None
    );
    assert!(matches!(follow(&state), Follow::Detached { .. }));
}
