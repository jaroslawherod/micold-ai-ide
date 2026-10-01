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
use crate::features::settings::{
    missing_cli_notice, script_path_notice, NoticeLine, ScriptCheck, SettingsDraft, SettingsSection,
};
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{Checkbox, Select, TextField};
use crate::ui::settings::{caution, field_note, note, page};
use iced::Element;
use micold_core::cli_reason;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::mcp::policy::CrossSessionAccess;
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
    ("pi_activity_component", "PiActivityComponentToggled"),
    ("tool_server_enabled", "ToolServerToggled"),
    ("cross_session_access", "CrossSessionAccessChanged"),
];

/// The Environment page.
pub fn view<'a>(
    draft: &'a SettingsDraft,
    outcome: &'a EnvIncludeOutcome,
    script_check: &'a ScriptCheck,
    availability: Option<&'a CliAvailability>,
    focused: Option<FieldId>,
    roles: Roles,
) -> Element<'a, Message> {
    let enabled = Checkbox::new(cli_reason::LABEL_ENABLED, draft.environment.enabled, roles)
        .track_focus(FieldId::SettingsEnvIncludeEnabled, focused)
        .on_toggle(|v| Message::Settings(SettingsMsg::EnvIncludeEnabledToggled(v)));

    let path = TextField::new("", &draft.environment.script_path, roles)
        .label(cli_reason::LABEL_SCRIPT_PATH)
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
        .label(cli_reason::LABEL_TIMEOUT)
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

    // FR-012e: one application-wide switch, beside the default-CLI choice it is held with. Its
    // label says what it does rather than naming a fault, and turning it off is not one (FR-012f):
    // a Pi session started without it runs normally and its badge reads unknown.
    let pi_activity = Checkbox::new(
        "Show activity for Pi sessions",
        draft.environment.pi_activity_component,
        roles,
    )
    .track_focus(FieldId::SettingsPiActivityComponent, focused)
    .on_toggle(|v| Message::Settings(SettingsMsg::PiActivityComponentToggled(v)));
    let pi_activity = field_note(
        pi_activity,
        Some("Loads a small reporter of this app into each new Pi session. Off: the badge reads unknown."),
        roles,
    );

    // Feature 034, FR-004: whether new sessions get the service's tools. The same checkbox-and-note
    // row as the Pi switch above; the note says it applies to sessions started afterwards.
    let tool_server = Checkbox::new(
        "Let AI sessions manage worktrees and sessions",
        draft.environment.tool_server_enabled,
        roles,
    )
    .track_focus(FieldId::SettingsToolServer, focused)
    .on_toggle(|v| Message::Settings(SettingsMsg::ToolServerToggled(v)));
    let tool_server = field_note(
        tool_server,
        Some("New AI sessions get this app's tools. Sessions already running keep what they started with."),
        roles,
    );

    // Feature 034, FR-016: whether an agent may read another session's terminal and type into it.
    // A separate option from the binding above it, with three values, so it is the shared `Select`
    // (Principle VIII) rather than a second checkbox. The values are drawn by their `Display`.
    let cross_session = Select::new(
        &CrossSessionAccess::ALL,
        Some(draft.environment.cross_session_access),
        |v| Message::Settings(SettingsMsg::CrossSessionAccessChanged(v)),
        roles,
    )
    .label("Let agents read and type into other sessions")
    .supporting("Applies to the next request, also from sessions already running");
    let cross_session = field_note(
        cross_session,
        Some("Auto: no confirmation. Confirm each send: you approve each message an agent types; reading needs no approval. Off: both are refused."),
        roles,
    );

    let mut controls: Vec<Element<'a, Message>> =
        vec![cli, pi_activity, tool_server, cross_session];
    controls.extend([enabled.into(), path.into(), timeout.into()]);

    // What the stored path's check found, and how the last resolution went (spec 035,
    // contracts/settings-indication.md §2). The wording is decided in the reducer's module; this
    // only picks the tone for each line.
    controls.extend(
        script_path_notice(script_check, outcome)
            .into_iter()
            .map(|line| match line {
                NoticeLine::Caution(text) => caution(text, roles),
                NoticeLine::Note(text) => note(text, roles),
            }),
    );

    page(
        "Environment",
        "What each session inherits before the agent starts.",
        controls,
        roles,
    )
}
