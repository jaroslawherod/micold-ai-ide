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
use micold_client::features::session::{AvailabilitySource, CliAvailability};
use micold_client::features::settings::{SettingsDraft, SettingsSection};
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
    let mut state = State::default();
    state.session.available_providers = Some(CliAvailability {
        // Claude Code present, Copilot missing — so there is a notice to look at, and the
        // select still has an option, which is the ordinary case rather than an empty form.
        available: vec![AiCli::ClaudeCode],
        source,
    });
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
    let notice = x_of(&painted, "isn't installed on this computer");

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
