//! The Settings dialog, rendered as a Material modal overlay within the main window (feature
//! 006, FR-019/FR-020). Currently exposes the embedded-terminal scrollback limit.

use crate::app::{Message, SettingsDraft};
use crate::ui::material::{self, Button, Checkbox, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self, spacing};

/// The failure category + diagnostic text for the most recent environment-include resolution
/// attempt, or `None` when it succeeded (or the feature is disabled) — FR-012/FR-013.
fn failure_label_and_diagnostic(outcome: &EnvIncludeOutcome) -> Option<(&'static str, &str)> {
    match outcome {
        EnvIncludeOutcome::Disabled | EnvIncludeOutcome::Success => None,
        EnvIncludeOutcome::MissingScript => Some(("Script not found", "")),
        EnvIncludeOutcome::NonZeroExit { diagnostic, .. } => {
            Some(("Exited with an error", diagnostic))
        }
        EnvIncludeOutcome::TimedOut { diagnostic } => Some(("Timed out", diagnostic)),
    }
}

/// The Settings dialog as the dialog body; `ui::view` wraps it in the shared
/// [`Modal`](crate::ui::material::Modal) transition. `env_include_outcome` is the most recent
/// environment-include resolution attempt's result, rendered as a read-only failure note when
/// it didn't succeed (feature 011, FR-012/FR-013).
pub fn modal<'a>(
    draft: &'a SettingsDraft,
    scheme: ColorScheme,
    env_include_outcome: &'a EnvIncludeOutcome,
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = TextField::new("", &draft.scrollback_lines, r)
        .label("Scrollback lines")
        .supporting("Lines kept per terminal")
        .on_input(Message::SettingsScrollbackChanged)
        .on_submit(Message::SettingsSaved);

    // Environment-include: enabled flag, script path, timeout — grouped as one visually
    // distinct set, separate from the scrollback field above (FR-015).
    let env_include_enabled_checkbox = Checkbox::new("Enabled", draft.env_include_enabled, r)
        .on_toggle(Message::SettingsEnvIncludeEnabledToggled);
    let env_include_path_input = TextField::new("", &draft.env_include_script_path, r)
        .label("Script path")
        .on_input(Message::SettingsEnvIncludePathChanged)
        .on_submit(Message::SettingsSaved);
    let env_include_timeout_input = TextField::new("", &draft.env_include_timeout, r)
        .label("Timeout")
        .supporting("Seconds")
        .on_input(Message::SettingsEnvIncludeTimeoutChanged)
        .on_submit(Message::SettingsSaved);

    let mut fields = column![
        Text::new("Settings", TypeRole::Headline, r),
        // No free-standing label above the field: it carries its own now (§7.7, FR-031a). Leaving
        // this one would have shown the field's name twice — once above the container and once
        // inside it — which is the duplication the migration exists to remove.
        input,
        // This one stays. It names the *group* below it — a checkbox and two fields — rather than
        // any single control, so it is a section heading and has no container to move into.
        Text::new("Environment include", TypeRole::Label, r).muted(),
        env_include_enabled_checkbox,
        env_include_path_input,
        env_include_timeout_input,
    ]
    .spacing(spacing::MD);

    if let Some((label, diagnostic)) = failure_label_and_diagnostic(env_include_outcome) {
        fields = fields.push(Text::new(label, TypeRole::Caption, r).tint(r.error));
        if !diagnostic.is_empty() {
            fields = fields.push(Text::new(diagnostic, TypeRole::Caption, r).muted());
        }
    }

    if let Some(error) = &draft.error {
        fields = fields.push(Text::new(error.clone(), TypeRole::Caption, r).tint(r.error));
    }

    let actions = row![
        Button::filled("Save", r).on_press(Message::SettingsSaved),
        Button::outlined("Cancel", r).on_press(Message::SettingsCancelled),
    ]
    .spacing(spacing::SM);

    let dialog = material::Surface::new(fields.push(actions), SurfaceKind::Dialog, r)
        .padding(spacing::LG)
        .width(Length::Fixed(420.0));

    dialog.into()
}
