//! The Environment section — the script sourced before each session starts (feature 011; feature
//! 027, FR-027).
//!
//! It arrives here whole: the checkbox, the path, the timeout, and the read-only note reporting
//! how the last resolution went. That note is the reason this section takes an argument the others
//! do not — it is not a setting, it is the outcome of applying three of them, and a user changing
//! the path needs to see it beside the field they just changed rather than in a snackbar that has
//! already gone.

use crate::app::{FieldId, Message};
use crate::features::settings::{SettingsDraft, SettingsSection};
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, TextField};
use crate::ui::settings::{caution, note, page};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[
    ("env_include_enabled", "SettingsEnvIncludeEnabledToggled"),
    ("env_include_script_path", "SettingsEnvIncludePathChanged"),
    (
        "env_include_timeout_secs",
        "SettingsEnvIncludeTimeoutChanged",
    ),
];

/// The failure category and its diagnostic for the most recent resolution attempt, or `None` when
/// it succeeded or the feature is off (FR-012/FR-013).
fn failure(outcome: &EnvIncludeOutcome) -> Option<(&'static str, &str)> {
    match outcome {
        EnvIncludeOutcome::Disabled | EnvIncludeOutcome::Success => None,
        EnvIncludeOutcome::MissingScript => Some(("Script not found", "")),
        EnvIncludeOutcome::NonZeroExit { diagnostic, .. } => {
            Some(("Exited with an error", diagnostic))
        }
        EnvIncludeOutcome::TimedOut { diagnostic } => Some(("Timed out", diagnostic)),
    }
}

/// The Environment page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    outcome: &'a EnvIncludeOutcome,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let enabled = Checkbox::new(
        "Source a script before each session",
        draft.environment.enabled,
        roles,
    )
    .track_focus(FieldId::SettingsEnvIncludeEnabled, focused)
    .on_toggle(Message::SettingsEnvIncludeEnabledToggled);

    let path = TextField::new("", &draft.environment.script_path, roles)
        .label("Script path")
        .supporting("Run in a shell; its exported variables reach every session")
        .error(super::error_for(
            draft,
            SettingsSection::Environment,
            FieldId::SettingsEnvIncludePath,
        ))
        .track_focus(FieldId::SettingsEnvIncludePath, focused)
        .on_input(Message::SettingsEnvIncludePathChanged)
        .on_submit(Message::SettingsSaved);

    let timeout = TextField::new("", &draft.environment.timeout_secs, roles)
        .label("Timeout")
        .supporting("Seconds")
        .error(super::error_for(
            draft,
            SettingsSection::Environment,
            FieldId::SettingsEnvIncludeTimeout,
        ))
        .track_focus(FieldId::SettingsEnvIncludeTimeout, focused)
        .on_input(Message::SettingsEnvIncludeTimeoutChanged)
        .on_submit(Message::SettingsSaved);

    let mut controls: Vec<Element<'a, Message>> = vec![enabled.into(), path.into(), timeout.into()];

    if let Some((label, diagnostic)) = failure(outcome) {
        controls.push(caution(label, roles));
        if !diagnostic.is_empty() {
            controls.push(note(diagnostic, roles));
        }
    }

    page(
        "Environment",
        "What each session inherits before the agent starts.",
        controls,
        roles,
    )
}
