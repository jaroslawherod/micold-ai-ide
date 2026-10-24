//! The one stage that can take minutes says so while it does — T043, SC-004.
//!
//! First-time enable is allowed five minutes. Five *silent* minutes reads as a hang, and the user's
//! remedy for a hang is to kill the application — which is why the runtime's progress callbacks are
//! an obligation in the contract (C-8) rather than a nicety, and why they have to reach the screen.
//!
//! Two halves. The wording is pure and checked directly; that the progress takes no row of its own
//! (it is the status bar's sandbox item, feature 1124) is checked by laying the window out.

mod support;

use micold_client::app::Message;
use micold_client::ui::stage_line;
use micold_core::sandbox::lifecycle::{Failure, SandboxState, Stage};
use micold_core::sandbox::runtime::{ContainerId, Progress, RuntimeError, RuntimeKind};
use support::layout as lay;

fn acquiring(stage: &str, detail: Option<&str>, percent: Option<u8>) -> SandboxState {
    SandboxState::Acquiring(Progress {
        stage: stage.to_string(),
        detail: detail.map(str::to_string),
        percent,
    })
}

/// The states that are *not* a bring-up in flight show nothing. Two reasons, and either alone
/// would be enough: there is no progress to report, and the bar animates.
#[test]
fn nothing_shows_unless_the_sandbox_is_coming_up() {
    for state in [
        SandboxState::Disabled,
        SandboxState::Running(ContainerId("c".into())),
        SandboxState::Stale(ContainerId("c".into())),
        SandboxState::Failed(Failure {
            stage: Stage::Probing,
            error: RuntimeError::NotInstalled {
                kind: RuntimeKind::Docker,
            },
        }),
    ] {
        assert!(
            stage_line(&state).is_none(),
            "{state:?} is not a bring-up in flight and must not show a progress indicator"
        );
    }
}

/// Every stage that *is* in flight names itself, and no two of them say the same thing — a bar that
/// reads "Working…" for four consecutive stages tells the user nothing about whether it is stuck.
#[test]
fn each_stage_of_the_bring_up_names_itself() {
    let lines: Vec<String> = [
        SandboxState::Probing,
        acquiring("Downloading", None, None),
        SandboxState::Starting,
    ]
    .iter()
    .map(|s| {
        stage_line(s)
            .unwrap_or_else(|| panic!("{s:?} is a stage in flight and must report one"))
            .label
    })
    .collect();

    for line in &lines {
        assert!(!line.is_empty());
    }
    let mut unique = lines.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        lines.len(),
        "two stages read the same: {lines:?}"
    );
}

/// What the runtime says reaches the screen. The percent especially: it is the only part that moves
/// on its own, and dropping it leaves a line that looks identical for the whole download.
#[test]
fn the_runtimes_own_progress_reaches_the_line() {
    let line = stage_line(&acquiring("Downloading", Some("3f4a1b2c"), Some(47)))
        .expect("acquiring reports a line");
    let detail = line
        .detail
        .expect("the runtime reported a stage, so there is a detail");

    assert!(detail.contains("Downloading"), "{detail}");
    assert!(
        detail.contains("47"),
        "the percent is the only part that moves: {detail}"
    );
    assert!(
        detail.contains("3f4a1b2c"),
        "the layer the runtime named: {detail}"
    );
}

/// A runtime that reports only a stage — no layer, no percent — still produces a readable line,
/// not one trailing a separator with nothing after it.
#[test]
fn a_bare_stage_produces_no_dangling_punctuation() {
    let line = stage_line(&acquiring("Extracting", None, None)).unwrap();
    let detail = line.detail.unwrap();
    assert_eq!(detail, "Extracting", "got {detail:?}");
}

/// The label says the image is being fetched, not merely that something is happening — this is the
/// stage a user is most likely to be watching, on a first enable, wondering whether to wait.
#[test]
fn the_acquiring_label_says_what_is_being_waited_for() {
    let line = stage_line(&acquiring("Downloading", None, None)).unwrap();
    assert!(
        line.label.to_lowercase().contains("image"),
        "the long stage has to say what it is fetching: {:?}",
        line.label
    );
}

/// No row appears between the toolbar and the body while the sandbox comes up, so the body's top
/// edge is where it is with the sandbox running (SC-003). The progress is the bar's, not a row's.
#[test]
fn the_window_column_has_no_row_above_the_body_during_bring_up() {
    let renderer = lay::renderer();
    let column_of = |state: SandboxState| {
        let under = support::covered_states::with_sandbox_state(state);
        let records = lay::resolve::<Message>(lay::view_of(&under), &renderer);
        let children: Vec<_> = records
            .iter()
            .filter(|r| r.layer == lay::Layer::Base && r.path.len() == 3 && r.path[..2] == [0, 0])
            .cloned()
            .collect();
        children
    };
    let running = column_of(SandboxState::Running(ContainerId("c".into())));
    assert_eq!(running.len(), 3, "toolbar, body, bar: {running:?}");

    for state in [
        SandboxState::Probing,
        acquiring("Downloading", Some("3f4a1b2c"), Some(47)),
        SandboxState::Starting,
    ] {
        let column = column_of(state.clone());
        assert_eq!(
            column.len(),
            3,
            "{state:?} added a row to the window column: {column:?}"
        );
        assert_eq!(
            column[1].y, running[1].y,
            "{state:?} moved the body's top edge"
        );
    }
}
