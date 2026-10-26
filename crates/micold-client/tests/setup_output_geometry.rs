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

/// U3: **Setup output** sits directly above Delete while a record exists, and not before.
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
    assert_eq!(items[at + 1], "Delete", "{items:?}");

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
    assert_eq!(items.last().map(String::as_str), Some("Setup output"));
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
