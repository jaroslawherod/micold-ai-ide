//! The one stage that can take minutes says so while it does — T043, SC-004.
//!
//! First-time enable is allowed five minutes. Five *silent* minutes reads as a hang, and the user's
//! remedy for a hang is to kill the application — which is why the runtime's progress callbacks are
//! an obligation in the contract (C-8) rather than a nicety, and why they have to reach the screen.
//!
//! Two halves. The wording is pure and checked directly; whether the indicator is on screen at all
//! is checked by laying the view out, because an indeterminate bar **animates**, and one rendered
//! while nothing is happening would request frames at rest for as long as the app is open
//! (SC-017, `idle_requests_no_frames.rs`).

mod support;

use micold_client::app::Message;
use micold_client::ui::{sandbox_indicator, stage_line};
use micold_core::sandbox::lifecycle::{Failure, SandboxState, Stage};
use micold_core::sandbox::runtime::{ContainerId, Progress, RuntimeError, RuntimeKind};
use micold_core::theme::ColorScheme;
use micold_core::tokens;
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

/// And the widget itself is absent — not merely empty — while nothing is coming up.
///
/// The bar is indeterminate, so it drives its own animation for as long as it exists. Rendering one
/// unconditionally would request a frame every tick for the life of the process, which is the
/// property `idle_requests_no_frames.rs` exists to protect.
#[test]
fn the_indicator_occupies_nothing_unless_the_sandbox_is_coming_up() {
    let renderer = lay::renderer();
    let roles = tokens::roles(ColorScheme::Light);

    let at_rest =
        lay::resolve::<Message>(sandbox_indicator(&SandboxState::Disabled, roles), &renderer);
    let root = at_rest.first().expect("a root record");
    assert_eq!(
        (root.width, root.height),
        (0.0, 0.0),
        "an indicator with size at rest is one that is animating at rest"
    );

    let in_flight = lay::resolve::<Message>(
        sandbox_indicator(&acquiring("Downloading", Some("3f4a1b2c"), Some(47)), roles),
        &renderer,
    );
    let root = in_flight.first().expect("a root record");
    assert!(
        root.height > 0.0,
        "the acquiring stage rendered nothing, so SC-004's five minutes are silent"
    );
}
