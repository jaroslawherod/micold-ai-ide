//! The setup output view at 10,000 lines builds only the rows on screen, and keeps its action row
//! inside the dialog at the smallest window (feature 1289, M6: geometry gate G2; contracts U3, U4,
//! U6; FR-016, FR-021, FR-024).

mod support;

use micold_client::app::{State, MIN_WINDOW_SIZE};
use micold_client::features::setup::{update, Msg, Names};
use micold_client::ui::confirm_delete::SETUP_RUNNING_LINE;
use micold_core::daemons::{DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::SessionLocation;
use micold_core::setup::{Failure, RunId, SetupCommand, SetupRun, SetupRunSummary, SetupState};
use micold_core::theme::ColorScheme;
use std::path::PathBuf;
use support::layout::{painted_text_settled_at, renderer, Overflow};
use support::sidebar_fixture::{deliver, listed, state_with_active};

const DIR: &str = "feat-a";

/// The project `/repo` open on one host daemon with the worktree `feat-a`, in the smallest window.
fn fixture() -> (State, DaemonId) {
    let mut state = state_with_active();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, false).unwrap();
    state.settings.daemons = registry;
    state.settings.legacy_default_daemon = Some(host);
    deliver(&mut state, vec![listed(DIR)]);
    state.window.window_size = (MIN_WINDOW_SIZE.width as u16, MIN_WINDOW_SIZE.height as u16);
    (state, host)
}

fn worktree() -> SessionLocation {
    SessionLocation::Worktree(DIR.to_owned())
}

/// `feat-a` has a run record, ended in `to` or still running.
fn report(state: &mut State, daemon: DaemonId, to: Option<SetupState>) {
    let run = SetupRun::start(
        RunId(1),
        SetupCommand::parse("mise trust && mise install").unwrap(),
        1_000,
    );
    let run = match to {
        Some(to) => run.finish(to, 1_060).unwrap(),
        None => run,
    };
    update(
        &mut state.setup,
        Msg::Changed {
            daemon,
            project: PathBuf::from("/repo"),
            location: worktree(),
            run: Some(SetupRunSummary {
                run,
                output_lines: 0,
            }),
            names: Names {
                project: "repo".into(),
                location: DIR.into(),
            },
        },
    );
}

/// The view of `feat-a` open, holding `lines` from line `first_line`.
fn open_with(state: &mut State, daemon: DaemonId, first_line: u64, lines: Vec<String>) {
    let key = (daemon, PathBuf::from("/repo"), worktree());
    update(&mut state.setup, Msg::ViewOpened(key));
    if lines.is_empty() {
        return;
    }
    update(
        &mut state.setup,
        Msg::Output {
            daemon,
            project: PathBuf::from("/repo"),
            location: worktree(),
            run: RunId(1),
            first_line,
            lines,
            dropped: first_line,
        },
    );
}

/// What the dialog paints in the smallest window.
fn painted(state: &State) -> Vec<Overflow> {
    let outcome = EnvIncludeOutcome::Disabled;
    let body = micold_client::ui::setup_output::dialog(state, ColorScheme::Dark, &outcome)
        .expect("the view is open");
    let mut renderer = renderer();
    painted_text_settled_at(body, &mut renderer, MIN_WINDOW_SIZE).0
}

fn find<'a>(painted: &'a [Overflow], text: &str) -> &'a Overflow {
    painted
        .iter()
        .find(|p| p.content == text)
        .unwrap_or_else(|| panic!("{text:?} not painted: {:?}", contents(painted)))
}

fn contents(painted: &[Overflow]) -> Vec<&str> {
    painted.iter().map(|p| p.content.as_str()).collect()
}

fn numbered(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("line {i}")).collect()
}

/// G2: 10,000 lines build only the rows on screen, and the action row stays inside the window.
#[test]
fn ten_thousand_lines_build_only_the_visible_rows_and_the_actions_stay_in_the_dialog() {
    let (mut state, host) = fixture();
    report(&mut state, host, None);
    open_with(&mut state, host, 0, numbered(10_000));
    let shown = painted(&state);
    let lines = shown
        .iter()
        .filter(|p| p.content.starts_with("line "))
        .count();
    assert!(lines > 0, "no line painted: {:?}", contents(&shown));
    assert!(lines < 100, "{lines} rows built of 10,000");
    let window = MIN_WINDOW_SIZE;
    for action in ["Copy output", "Close"] {
        let at = find(&shown, action);
        let bottom = at.on_screen.y + at.natural_height;
        assert!(
            at.on_screen.y >= 0.0 && bottom <= window.height,
            "{action} at {}..{bottom} in a {}dp window",
            at.on_screen.y,
            window.height
        );
    }
    let close = find(&shown, "Close");
    let copy = find(&shown, "Copy output");
    assert!(
        copy.on_screen.x < close.on_screen.x,
        "Close is the last action"
    );
}

/// U4: the title names the location, the header shows the command and the state in words.
#[test]
fn the_header_shows_the_location_the_command_and_the_state() {
    let (mut state, host) = fixture();
    report(&mut state, host, Some(SetupState::Failed(Failure::Exit(2))));
    open_with(&mut state, host, 0, vec!["Exited with status 2.".into()]);
    let shown = painted(&state);
    // The worktree by the name its row shows.
    find(
        &shown,
        &format!("Setup \u{2014} {}", state.worktree_display_name(DIR)),
    );
    find(&shown, "mise trust && mise install");
    assert!(
        shown
            .iter()
            .any(|p| p.content.starts_with("Setup failed at ")
                && p.content.ends_with(": Exited with status 2")),
        "{:?}",
        contents(&shown)
    );
    find(&shown, "Exited with status 2.");
}

/// U4, FR-021: dropped lines are counted in the first row; no output says so.
#[test]
fn the_first_row_counts_the_lines_not_kept_and_an_empty_view_says_no_output_yet() {
    let (mut state, host) = fixture();
    report(&mut state, host, None);
    open_with(&mut state, host, 42, numbered(3));
    let shown = painted(&state);
    let first = find(&shown, "42 earlier lines not kept");
    assert!(first.on_screen.y < find(&shown, "line 0").on_screen.y);

    let (mut state, host) = fixture();
    report(&mut state, host, None);
    open_with(&mut state, host, 0, Vec::new());
    find(&painted(&state), "No output yet.");
}

/// U3: **Setup output** sits above Delete while a record exists, and not before (M7 puts **Stop
/// setup** between them while the run runs).
#[test]
fn the_menu_offers_setup_output_above_delete_while_a_record_exists() {
    let (mut state, host) = fixture();
    let labels = |state: &State, location: &SessionLocation| -> Vec<String> {
        micold_client::ui::location_menu_items(state, location)
            .into_iter()
            .map(|item| item.label)
            .collect()
    };
    assert!(!labels(&state, &worktree()).contains(&"Setup output".to_owned()));
    report(&mut state, host, None);
    let items = labels(&state, &worktree());
    let at = items.iter().position(|l| l == "Setup output").unwrap();
    assert_eq!(items[at + 1], "Stop setup", "{items:?}");
    assert_eq!(items[at + 2], "Delete", "{items:?}");

    // On the Default row it is appended.
    assert!(!labels(&state, &SessionLocation::Default).contains(&"Setup output".to_owned()));
    update(
        &mut state.setup,
        Msg::Changed {
            daemon: host,
            project: PathBuf::from("/repo"),
            location: SessionLocation::Default,
            run: Some(SetupRunSummary {
                run: SetupRun::start(RunId(2), SetupCommand::parse("true").unwrap(), 0),
                output_lines: 0,
            }),
            names: Names {
                project: "repo".into(),
                location: "Default".into(),
            },
        },
    );
    let items = labels(&state, &SessionLocation::Default);
    let at = items.iter().position(|l| l == "Setup output").unwrap();
    assert_eq!(&items[at..], ["Setup output", "Stop setup"], "{items:?}");
}

/// U6: the delete confirmation says setup is stopped first exactly while it runs.
#[test]
fn the_delete_confirmation_says_setup_is_stopped_first_while_it_runs() {
    let (mut state, host) = fixture();
    let delete_paints_line = |state: &State| {
        let outcome = EnvIncludeOutcome::Disabled;
        let body = micold_client::ui::confirm_delete::dialog(state, ColorScheme::Dark, &outcome)
            .expect("a delete is pending");
        let mut renderer = renderer();
        painted_text_settled_at(body, &mut renderer, MIN_WINDOW_SIZE)
            .0
            .iter()
            .any(|p| p.content == SETUP_RUNNING_LINE)
    };
    state.worktree.delete_target = Some(DIR.to_owned());
    assert!(!delete_paints_line(&state), "no run record");
    report(&mut state, host, None);
    assert!(delete_paints_line(&state), "running");
    report(&mut state, host, Some(SetupState::Succeeded));
    assert!(!delete_paints_line(&state), "ended");
}

/// M6 review V-F2: choosing **Setup output** in a row's menu closes the menu, as Delete does.
#[test]
fn choosing_setup_output_closes_the_row_menu() {
    use micold_client::app::Message;
    use micold_client::features::worktree::Msg as WorktreeMsg;
    let (mut state, host) = fixture();
    report(&mut state, host, None);
    state.update(Message::Worktree(WorktreeMsg::MenuToggled(
        DIR.to_owned(),
        (120, 300),
    )));
    assert!(state.worktree.menu_open.is_some());
    state.update(Message::Setup(Msg::ViewOpened((
        host,
        PathBuf::from("/repo"),
        worktree(),
    ))));
    assert!(state.setup.view.is_some());
    assert_eq!(state.worktree.menu_open, None);
}

/// M6 review V-F3: an output line too long for the list stops short of the list's scrollbar.
#[test]
fn a_long_line_is_clipped_clear_of_the_scrollbar() {
    let (mut state, host) = fixture();
    report(&mut state, host, None);
    let long = "x".repeat(400);
    let mut lines = vec![long.clone()];
    lines.extend(numbered(60));
    open_with(&mut state, host, 0, lines);
    let shown = painted(&state);
    let line = find(&shown, &long);
    // The command's box spans the dialog body, as the list's does.
    let body = find(&shown, "mise trust && mise install");
    let body_right = body.clip.x + body.clip.width;
    let line_right = line.clip.x + line.clip.width;
    assert!(
        line_right <= body_right - 6.0 + 0.01,
        "the line is clipped at {line_right}, the list ends at {body_right}"
    );
}

/// The project `/repo` has `command` set, as Settings › Project stores it.
fn set_command(state: &mut State, command: &str) {
    state.settings.setup_commands.insert(
        micold_core::jira::connection_key(std::path::Path::new("/repo")),
        SetupCommand::parse(command).unwrap(),
    );
}

fn menu_labels(state: &State, location: &SessionLocation) -> Vec<String> {
    micold_client::ui::location_menu_items(state, location)
        .into_iter()
        .map(|item| item.label)
        .collect()
}

/// M7, U3: **Run setup** follows **Setup output** above Delete when the project has a command and
/// nothing runs; **Stop setup** takes its place while the run runs; neither without a command.
#[test]
fn the_menu_offers_run_or_stop_setup_after_setup_output_above_delete() {
    let (mut state, host) = fixture();
    let before = menu_labels(&state, &worktree());
    assert!(!before.contains(&"Run setup".to_owned()), "no command");

    set_command(&mut state, "mise install");
    let items = menu_labels(&state, &worktree());
    let at = items.iter().position(|l| l == "Run setup").unwrap();
    assert_eq!(items[at + 1], "Delete", "{items:?}");
    assert!(!items.contains(&"Stop setup".to_owned()));

    report(&mut state, host, None);
    let items = menu_labels(&state, &worktree());
    assert!(!items.contains(&"Run setup".to_owned()), "while running");
    let output = items.iter().position(|l| l == "Setup output").unwrap();
    assert_eq!(items[output + 1], "Stop setup", "{items:?}");
    assert_eq!(items[output + 2], "Delete", "{items:?}");

    report(&mut state, host, Some(SetupState::Failed(Failure::Exit(2))));
    let items = menu_labels(&state, &worktree());
    let output = items.iter().position(|l| l == "Setup output").unwrap();
    assert_eq!(items[output + 1], "Run setup", "{items:?}");
    assert!(!items.contains(&"Stop setup".to_owned()));

    // Without a command a failed run cannot be re-run, but its output can still be read.
    state.settings.setup_commands.clear();
    let items = menu_labels(&state, &worktree());
    assert!(items.contains(&"Setup output".to_owned()));
    assert!(!items.contains(&"Run setup".to_owned()));
}

/// M7, US3 scenario 5: the Default row is offered **Run setup**, appended last.
#[test]
fn the_default_row_is_offered_run_setup_last() {
    let (mut state, _host) = fixture();
    assert!(!menu_labels(&state, &SessionLocation::Default).contains(&"Run setup".to_owned()));
    set_command(&mut state, "mise install");
    let items = menu_labels(&state, &SessionLocation::Default);
    assert_eq!(
        items.last().map(String::as_str),
        Some("Run setup"),
        "{items:?}"
    );
}

/// M7, U3: choosing either entry closes the row menu, as Setup output and Delete do.
#[test]
fn choosing_run_or_stop_setup_closes_the_row_menu() {
    use micold_client::app::Message;
    use micold_client::features::worktree::Msg as WorktreeMsg;
    let (mut state, host) = fixture();
    let key = (host, PathBuf::from("/repo"), worktree());
    for msg in [Msg::RunRequested(key.clone()), Msg::StopRequested(key)] {
        state.update(Message::Worktree(WorktreeMsg::MenuToggled(
            DIR.to_owned(),
            (120, 300),
        )));
        assert!(state.worktree.menu_open.is_some());
        state.update(Message::Setup(msg));
        assert!(state.worktree.menu_open.is_none());
    }
}

/// M7, U4: **Stop setup** sits before **Close** in the output view while the run runs, and is
/// absent once it ends; the action row stays inside the smallest window.
#[test]
fn the_output_view_offers_stop_setup_before_close_while_running() {
    let (mut state, host) = fixture();
    report(&mut state, host, None);
    open_with(&mut state, host, 0, numbered(3));
    let shown = painted(&state);
    let window = MIN_WINDOW_SIZE;
    let (copy, stop, close) = (
        find(&shown, "Copy output"),
        find(&shown, "Stop setup"),
        find(&shown, "Close"),
    );
    assert!(copy.on_screen.x < stop.on_screen.x && stop.on_screen.x < close.on_screen.x);
    for at in [copy, stop, close] {
        assert!(
            at.on_screen.y >= 0.0 && at.on_screen.y + at.natural_height <= window.height,
            "{} outside the {}dp window",
            at.content,
            window.height
        );
    }

    report(&mut state, host, Some(SetupState::Stopped));
    let shown = painted(&state);
    assert!(
        !shown.iter().any(|p| p.content == "Stop setup"),
        "{:?}",
        contents(&shown)
    );
}
