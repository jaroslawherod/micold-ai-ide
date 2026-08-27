//! The two surfaces that offer a CLI never offer one that is not installed (feature 026, T071 —
//! FR-006).
//!
//! FR-006's rule is about what the user is *shown*, and both surfaces read the availability set
//! T014a put on `State`: the Settings select takes `state.session.available_providers` directly, the
//! session start list takes `State::offered_providers()`. The pure layer is covered in
//! `features_session.rs`; what is covered here is that the drawn surfaces actually follow it —
//! neither reaches for `AiCli::ALL`, which is the one-token change that would make both of them
//! wrong while every pure test stayed green.
//!
//! So these tests read the pixels: render the real `ui::view`, open the control the way a person
//! would, and assert on the strings the renderer painted. Each claim is asserted in both
//! directions — an uninstalled CLI is absent, and the same CLI *appears* once it is installed —
//! because "GitHub Copilot was not painted" is also what a list that never opened looks like.
//!
//! Named by `display_name()` on both surfaces (FR-006, T058e): these are menus, and `claude` /
//! `copilot` is the row-label register.

#[path = "support/mod.rs"]
mod support;

use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::session::StartMenu;
use micold_client::features::settings::{EnvironmentDraft, SettingsDraft, SettingsSection};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::{AiCli, SessionLocation};
use support::layout as lay;

const PROJECT: &str = "/fixture/providers";

/// The Default AI CLI select inside Settings' Environment section.
///
/// Settings is a full-surface view as of feature 027, not a dialog, so the path runs down the
/// content area rather than an overlay: the section's controls column has the select first, above
/// the environment-include toggle and its two fields.
const SETTINGS_SELECT: &[usize] = &[0, 0, 1, 0, 1, 0, 0, 1];

fn with_project() -> State {
    let mut workspace = support::workspace_with(vec![(PROJECT, vec![])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };
    state.sidebar.width = 300;
    state.window.window_size = (1280, 800);
    state
}

/// Every string the given state paints, with the select at `press_at` opened first — or, with no
/// path, with whatever the state already opens given time to appear.
fn painted(state: &State, press_at: Option<&[usize]>) -> Vec<String> {
    let mut renderer = lay::renderer();
    let build = || {
        micold_client::ui::view(
            state,
            None,
            None,
            0,
            None,
            &EnvIncludeOutcome::Disabled,
            &ConnectionStatus::Connected,
            &micold_client::features::sandbox::Sandbox::default(),
        )
    };
    let drawn = match press_at {
        Some(path) => lay::painted_text_pressing(build(), &mut renderer, path),
        None => lay::painted_text_settled(build(), &mut renderer),
    };
    drawn.into_iter().map(|text| text.content).collect()
}

/// Settings open, with the draft's default set to one of the CLIs that *is* installed.
///
/// That last part is deliberate and not incidental. A select paints its current value on the
/// trigger as well as its options in the list, and the trigger is not an offer — it is a report of
/// what the setting says. A draft pointing at an uninstalled CLI would therefore paint that CLI's
/// name for a reason FR-006 has nothing to say about, and every assertion here reads the painted
/// strings without being able to tell the two apart. Keeping the draft on an installed CLI leaves
/// the list as the only thing that can name one.
fn settings_state(available: &[AiCli]) -> State {
    let mut state = with_project();
    state.session.available_providers = available.to_vec();
    state.settings.settings_draft = Some(SettingsDraft {
        // Feature 027 turned Settings into a sectioned full-surface view, and the Default AI
        // CLI select lives in Environment — the section has to be the shown one, or the
        // control this test presses is not on screen at all.
        section: SettingsSection::Environment,
        environment: EnvironmentDraft {
            default_ai_cli: available.first().copied().unwrap_or(AiCli::ClaudeCode),
            ..EnvironmentDraft::default()
        },
        ..SettingsDraft::default()
    });
    state
}

fn start_menu_state(available: &[AiCli]) -> State {
    let mut state = with_project();
    state.session.available_providers = available.to_vec();
    state.session.start_menu = Some(StartMenu {
        location: SessionLocation::Default,
        anchor: (400, 300),
    });
    state
}

#[test]
fn the_settings_select_lists_only_the_installed_clis() {
    let only_claude = painted(&settings_state(&[AiCli::ClaudeCode]), Some(SETTINGS_SELECT));
    assert!(
        only_claude.iter().any(|s| s == "Claude Code"),
        "the installed CLI must be in the open list — painted: {only_claude:?}"
    );
    assert!(
        !only_claude.iter().any(|s| s.contains("Copilot")),
        "an uninstalled CLI must not be offered anywhere in Settings — painted: {only_claude:?}"
    );

    // The control: the same surface, the same press, with Copilot installed. Without this, the
    // assertion above passes for a list that never opened at all.
    let both = painted(
        &settings_state(&[AiCli::ClaudeCode, AiCli::Copilot]),
        Some(SETTINGS_SELECT),
    );
    assert!(
        both.iter().any(|s| s == "GitHub Copilot"),
        "with Copilot installed the open list must offer it — painted: {both:?}"
    );
    assert!(
        both.iter().any(|s| s == "Claude Code"),
        "…alongside the other installed CLI — painted: {both:?}"
    );
}

#[test]
fn the_session_start_list_offers_only_the_installed_clis() {
    let only_claude = painted(&start_menu_state(&[AiCli::ClaudeCode]), None);
    assert!(
        only_claude.iter().any(|s| s == "Claude Code"),
        "the per-session override list must offer the installed CLI — painted: {only_claude:?}"
    );
    assert!(
        !only_claude.iter().any(|s| s.contains("Copilot")),
        "…and must not offer the one that is not installed — painted: {only_claude:?}"
    );

    let both = painted(
        &start_menu_state(&[AiCli::ClaudeCode, AiCli::Copilot]),
        None,
    );
    assert!(
        both.iter().any(|s| s == "GitHub Copilot"),
        "with both installed the same list offers both — painted: {both:?}"
    );
}

/// The two surfaces cannot disagree about what exists (FR-006).
///
/// Not a restatement of the two tests above: they each pin one surface against `AiCli::ALL`, and
/// this pins them against *each other* — one reads `state.session.available_providers` and the other
/// `State::offered_providers()`, so "both are filtered" and "both are filtered the same way" are
/// different claims.
#[test]
fn the_settings_select_and_the_start_list_name_the_same_clis() {
    let available = [AiCli::Copilot];

    let settings = painted(&settings_state(&available), Some(SETTINGS_SELECT));
    let start = painted(&start_menu_state(&available), None);

    for which in AiCli::ALL {
        let name = which.provider().display_name();
        assert_eq!(
            settings.iter().any(|s| s == name),
            start.iter().any(|s| s == name),
            "the Settings select and the start list disagree about {name} — settings: \
             {settings:?}, start: {start:?}"
        );
    }
    assert!(
        settings.iter().any(|s| s == "GitHub Copilot"),
        "…and they agree on a CLI that is actually offered, not merely on an empty pair of lists"
    );
}
