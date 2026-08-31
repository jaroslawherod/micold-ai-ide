//! The Environment section — the script sourced before each session starts (feature 011; feature
//! 027, FR-027).
//!
//! It arrives here whole: the checkbox, the path, the timeout, and the read-only note reporting
//! how the last resolution went. That note is the reason this section takes an argument the others
//! do not — it is not a setting, it is the outcome of applying three of them, and a user changing
//! the path needs to see it beside the field they just changed rather than in a snackbar that has
//! already gone.

use crate::app::Message;
use crate::features::session::CliAvailability;
use crate::features::settings::Msg as SettingsMsg;
use crate::features::settings::{missing_cli_notice, SettingsDraft, SettingsSection};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, Select, TextField};
use crate::ui::settings::{caution, field_note, note, page};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::AiCli;
use micold_core::tokens::Roles;

/// What this section renders. See [`crate::ui::settings`].
// Read by `tests/settings_sections.rs`, which is a separate crate and cannot be seen from here —
// so to the compiler this is unused. Deleting it would take the gate's evidence with it.
#[allow(dead_code)]
pub const SETTINGS: &[(&str, &str)] = &[
    ("env_include_enabled", "EnvIncludeEnabledToggled"),
    ("env_include_script_path", "EnvIncludePathChanged"),
    ("env_include_timeout_secs", "EnvIncludeTimeoutChanged"),
    ("default_ai_cli", "DefaultAiCliChanged"),
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
    availability: Option<&'a CliAvailability>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let enabled = Checkbox::new(
        "Source a script before each session",
        draft.environment.enabled,
        roles,
    )
    .track_focus(FieldId::SettingsEnvIncludeEnabled, focused)
    .on_toggle(|v| Message::Settings(SettingsMsg::EnvIncludeEnabledToggled(v)));

    let path = TextField::new("", &draft.environment.script_path, roles)
        .label("Script path")
        .supporting("Run in a shell; its exported variables reach every session")
        .error(super::error_for(
            draft,
            SettingsSection::Environment,
            FieldId::SettingsEnvIncludePath,
        ))
        .track_focus(FieldId::SettingsEnvIncludePath, focused)
        .on_input(|v| Message::Settings(SettingsMsg::EnvIncludePathChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));

    let timeout = TextField::new("", &draft.environment.timeout_secs, roles)
        .label("Timeout")
        .supporting("Seconds")
        .error(super::error_for(
            draft,
            SettingsSection::Environment,
            FieldId::SettingsEnvIncludeTimeout,
        ))
        .track_focus(FieldId::SettingsEnvIncludeTimeout, focused)
        .on_input(|v| Message::Settings(SettingsMsg::EnvIncludeTimeoutChanged(v)))
        .on_submit(Message::Settings(SettingsMsg::Saved));

    // The Default AI CLI (feature 026, FR-003/FR-006). The shared `Select` component, not a
    // bespoke control (Principle VIII) -- and the options are what the *service* reported, so a CLI
    // that is not installed where sessions run is not offered (feature 027, FR-023c). Empty until
    // the service answers, which is a select with no options for the moment before the first reply
    // rather than a list of guesses.
    //
    // Not offering it is not the same as explaining it, which is FR-023b: a CLI the user expected
    // to find here is simply absent, and an absence answers no question. So the notice below says
    // which one is missing and what would have to provide it. This is one of the two places it
    // appears -- the other is where the image itself is chosen -- and it is deliberately *not* at
    // session start, by which point the user has committed to something the app already knew.
    //
    // Named by `display_name()` through `Display`, which is a menu's register. `command()` -- the
    // `claude`/`copilot` a sidebar row carries -- is not used here: a menu entry is not a label in
    // a width budget (Clarifications 2026-08-18).
    //
    // It sits above the include controls because it answers the first question this section's own
    // line asks -- which agent starts -- and the script is what that agent then inherits.
    let options: &'a [AiCli] = availability.map(|a| a.available.as_slice()).unwrap_or(&[]);
    let default_ai_cli = Select::new(
        options,
        Some(draft.environment.default_ai_cli),
        |v| Message::Settings(SettingsMsg::DefaultAiCliChanged(v)),
        roles,
    )
    .label("Default AI CLI")
    .supporting("Used for new sessions unless you choose otherwise");

    // Attached to the select rather than stacked after it, so the sentence sits in the column the
    // select's own supporting line sits in. See `field_note`.
    let cli = field_note(default_ai_cli, missing_cli_notice(availability), roles);

    let mut controls: Vec<Element<'a, Message>> = vec![cli];
    controls.extend([enabled.into(), path.into(), timeout.into()]);

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
