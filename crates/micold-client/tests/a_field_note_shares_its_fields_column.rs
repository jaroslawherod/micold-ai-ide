//! A note about a control lines up with that control's own supporting text (feature 027, T146).
//!
//! The ninth gate, and the second one a §B.6 visual pass produced.
//!
//! A settings page stacks its controls at one left margin. A note pushed between two of them
//! therefore lands on the left edge of the control *below* it, and a left edge is what the eye
//! reads first — so FR-023b's "GitHub Copilot isn't installed…" sat in a column with the checkbox
//! under it rather than with the select it is about. Everything was green over that: the wording
//! tests read strings and never positions, and `layout_snapshot` records x-positions but a record
//! compares against what it was shown.
//!
//! The question nothing else asks is the relational one — a note and the supporting line of the
//! field it belongs to are two halves of one block, and two halves of one block share an edge.
//! Asserted by what the renderer actually drew rather than by node geometry, because a `Text` node
//! is as wide as its parent gives it and the inset is inside the paragraph's origin.
//!
//! Both points of choice FR-023b names are covered, since they are two call sites of one helper
//! and a helper used correctly in one place and not the other is exactly the drift this catches.

#[path = "support/mod.rs"]
mod support;

use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::session::{AvailabilityKey, AvailabilitySource, CliAvailability};
use micold_client::features::settings::{missing_cli_notice, SettingsDraft, SettingsSection};
use micold_core::cli_reason::SpawnEnv;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::AiCli;
use support::layout as lay;

/// Half a pixel, matching the other geometry gates.
const TOLERANCE: f32 = 0.5;

/// The supporting line under the Default AI CLI select.
const CLI_SUPPORTING: &str = "Used for new sessions unless you choose otherwise";

/// The supporting line under Image reference, in the Session service section.
const IMAGE_SUPPORTING: &str = "A digest or an exact tag; a moving tag cannot be reported in a bug";

fn settings_showing(section: SettingsSection, source: AvailabilitySource) -> State {
    // The script applied: with no known state there is no note to look at (037, W5).
    settings_showing_in(section, source, SpawnEnv::Applied)
}

fn settings_showing_in(
    section: SettingsSection,
    source: AvailabilitySource,
    env: SpawnEnv,
) -> State {
    let mut state = State::default();
    support::hold_home(
        &mut state,
        CliAvailability {
            // Claude Code present, Copilot and Pi missing — so there is a notice to look at, and the
            // select still has an option, which is the ordinary case rather than an empty form.
            available: vec![AiCli::ClaudeCode],
            source,
            env: Some(env),
            asked_for: AvailabilityKey::Home,
        },
    );
    state.settings.settings_draft = Some(SettingsDraft {
        section,
        ..SettingsDraft::default()
    });
    state.window.window_size = (1280, 900);
    state
}

/// Where the renderer put each painted string, by content.
fn painted(state: &State) -> Vec<(String, f32)> {
    let mut renderer = lay::renderer();
    let element = micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    );
    lay::painted_text(element, &mut renderer)
        .into_iter()
        .map(|drawn| (drawn.content, drawn.origin.x))
        .collect()
}

fn x_of(painted: &[(String, f32)], needle: &str) -> f32 {
    let hits: Vec<f32> = painted
        .iter()
        .filter(|(content, _)| content.contains(needle))
        .map(|(_, x)| *x)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "expected exactly one painted string containing {needle:?} — painted: {:?}",
        painted.iter().map(|(c, _)| c).collect::<Vec<_>>()
    );
    hits[0]
}

#[test]
fn the_missing_cli_notice_lines_up_with_the_select_it_is_about() {
    let painted = painted(&settings_showing(
        SettingsSection::Environment,
        AvailabilitySource::ThisComputer,
    ));

    let supporting = x_of(&painted, CLI_SUPPORTING);
    // "was" or "were", depending on how many CLIs are missing; the column is the same.
    let notice = x_of(&painted, "not found on the PATH");

    assert!(
        (notice - supporting).abs() <= TOLERANCE,
        "the notice starts at {notice} and the select's own supporting line at {supporting}; a \
         note about a control belongs in that control's column"
    );
}

#[test]
fn and_so_does_the_one_under_the_image_reference() {
    let painted = painted(&settings_showing(
        SettingsSection::Daemon,
        AvailabilitySource::Image("ghcr.io/example/my-own-image:3".to_string()),
    ));

    let supporting = x_of(&painted, IMAGE_SUPPORTING);
    let notice = x_of(&painted, "ghcr.io/example/my-own-image:3");

    assert!(
        (notice - supporting).abs() <= TOLERANCE,
        "the notice starts at {notice} and the field's own supporting line at {supporting}; the \
         two call sites of `field_note` must not drift apart"
    );
}

/// 037 U50 (FR-005, surface U2): the note under *Image reference* is the note under **Default AI
/// CLI**. One answer, one sentence, in both places it is chosen.
#[test]
fn the_two_notes_are_one_sentence() {
    let image = AvailabilitySource::Image("ghcr.io/example/my-own-image:3".to_string());
    let note_in = |section| {
        let state = settings_showing_in(section, image.clone(), SpawnEnv::IncludeOff);
        let expected = missing_cli_notice(state.session.availability.home())
            .expect("fixture check: two CLIs are missing, so there is a note");
        let shown: Vec<String> = painted(&state)
            .into_iter()
            .map(|(content, _)| content)
            .filter(|content| content.contains("would not find"))
            .collect();
        assert_eq!(
            shown,
            vec![expected.clone()],
            "the section paints the note once, whole"
        );
        expected
    };

    assert_eq!(
        note_in(SettingsSection::Environment),
        note_in(SettingsSection::Daemon),
        "the two points of choice read one answer and must say one thing about it"
    );
}

/// The control: without it, both assertions above also pass on a page that painted nothing.
///
/// A page is not the same shape as a `contains` miss, and `x_of` panics on zero hits — so this is
/// not redundant with that: it pins that the *section* under test is the one on screen, which is
/// the mistake a reordered `SettingsSection` would make silently.
#[test]
fn each_section_under_test_is_the_one_on_screen() {
    let environment = painted(&settings_showing(
        SettingsSection::Environment,
        AvailabilitySource::ThisComputer,
    ));
    assert!(
        environment.iter().any(|(c, _)| c == CLI_SUPPORTING),
        "Environment must be the shown section — painted: {:?}",
        environment.iter().map(|(c, _)| c).collect::<Vec<_>>()
    );

    let daemon = painted(&settings_showing(
        SettingsSection::Daemon,
        AvailabilitySource::Image("ghcr.io/example/my-own-image:3".to_string()),
    ));
    assert!(
        daemon.iter().any(|(c, _)| c == IMAGE_SUPPORTING),
        "Session service must be the shown section — painted: {:?}",
        daemon.iter().map(|(c, _)| c).collect::<Vec<_>>()
    );
}
