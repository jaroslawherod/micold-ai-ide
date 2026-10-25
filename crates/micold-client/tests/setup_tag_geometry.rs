//! A setup tag never moves a row's name or state icon (feature 1289, M5: geometry gate G1,
//! contract U2, FR-015).

mod support;

use iced::Element;
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sandbox::Sandbox;
use micold_client::features::setup::{update, Msg, Names};
use micold_core::daemons::{DaemonId, DaemonRegistry, DaemonRuntime};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::SessionLocation;
use micold_core::setup::{Failure, RunId, SetupCommand, SetupRun, SetupRunSummary, SetupState};
use std::path::PathBuf;
use support::layout::{painted_text_settled, renderer, Overflow};
use support::sidebar_fixture::{deliver, listed, state_with_active};

const LONG: &str = "a-worktree-name-long-enough-that-the-sidebar-has-to-cut-it-short-somewhere";

/// The project `/repo` open on one host daemon, with a worktree `dir`.
fn state_with(dir: &str) -> (State, DaemonId) {
    let mut state = state_with_active();
    let mut registry = DaemonRegistry::new(Vec::new(), 1);
    let host = registry.add("Host", DaemonRuntime::Host, false).unwrap();
    state.settings.daemons = registry;
    state.settings.legacy_default_daemon = Some(host);
    deliver(&mut state, vec![listed(dir)]);
    (state, host)
}

fn report(state: &mut State, daemon: DaemonId, location: SessionLocation, to: Option<SetupState>) {
    let run = SetupRun::start(
        RunId(1),
        SetupCommand::parse("mise install").unwrap(),
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
            location,
            run: Some(SetupRunSummary {
                run,
                output_lines: 0,
            }),
            names: Names {
                project: "repo".into(),
                location: "x".into(),
            },
        },
    );
}

fn painted(state: &State) -> Vec<Overflow> {
    let view: Element<'_, Message> = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &Sandbox::default(),
    );
    painted_text_settled(view, &mut renderer())
}

/// Every text painted on the line that holds `name` (a row's title: a worktree's branch, or
/// *Default*): the title and the state icon beside it.
fn name_line(texts: &[Overflow], name: &str) -> Vec<(String, iced::Point)> {
    let y = texts
        .iter()
        .find(|t| t.content.starts_with(name))
        .unwrap_or_else(|| panic!("{name} is painted: {:?}", contents(texts)))
        .origin
        .y;
    texts
        .iter()
        .filter(|t| (t.origin.y - y).abs() < 0.5)
        .map(|t| (t.content.clone(), t.origin))
        .collect()
}

fn contents(texts: &[Overflow]) -> Vec<&str> {
    texts.iter().map(|t| t.content.as_str()).collect()
}

#[test]
fn a_worktree_row_with_a_setup_tag_keeps_its_name_and_state_icon_where_they_were() {
    for to in [
        None,
        Some(SetupState::Succeeded),
        Some(SetupState::Failed(Failure::Exit(2))),
        Some(SetupState::Stopped),
        Some(SetupState::Interrupted),
    ] {
        let (bare, daemon) = state_with("feat-a");
        let mut tagged = bare.clone();
        report(
            &mut tagged,
            daemon,
            SessionLocation::Worktree("feat-a".into()),
            to.clone(),
        );
        let (before, after) = (painted(&bare), painted(&tagged));
        let label = to.as_ref().map_or("Setting up…", SetupState::label);
        assert!(
            contents(&after).contains(&label),
            "{label} is painted: {:?}",
            contents(&after)
        );
        assert!(!contents(&before).contains(&label));
        assert_eq!(
            name_line(&before, "feat/feat-a"),
            name_line(&after, "feat/feat-a"),
            "{to:?}"
        );
    }
}

#[test]
fn a_long_name_with_a_tag_is_truncated_not_pushed() {
    let (bare, daemon) = state_with(LONG);
    let mut tagged = bare.clone();
    report(
        &mut tagged,
        daemon,
        SessionLocation::Worktree(LONG.into()),
        None,
    );
    let (before, after) = (painted(&bare), painted(&tagged));
    let name = after
        .iter()
        .find(|t| t.content.starts_with("feat/a-worktree") && t.content.ends_with('…'))
        .unwrap_or_else(|| panic!("the name is cut: {:?}", contents(&after)));
    assert!(name.natural_width <= name.allowed_width + 0.5, "{name:?}");
    assert_eq!(
        name_line(&before, "feat/a-worktree"),
        name_line(&after, "feat/a-worktree")
    );
    let tag = after
        .iter()
        .find(|t| t.content == "Setting up…")
        .expect("the tag is painted whole");
    assert!(tag.natural_width <= tag.allowed_width + 0.5, "{tag:?}");
}

#[test]
fn the_default_row_gets_the_tag_too() {
    let (bare, daemon) = state_with("feat-a");
    let mut tagged = bare.clone();
    report(
        &mut tagged,
        daemon,
        SessionLocation::Default,
        Some(SetupState::Failed(Failure::Exit(2))),
    );
    let (before, after) = (painted(&bare), painted(&tagged));
    assert!(
        contents(&after).contains(&"Setup failed"),
        "{:?}",
        contents(&after)
    );
    assert_eq!(name_line(&before, "Default"), name_line(&after, "Default"));
    assert_eq!(
        name_line(&before, "feat/feat-a"),
        name_line(&after, "feat/feat-a")
    );
}
