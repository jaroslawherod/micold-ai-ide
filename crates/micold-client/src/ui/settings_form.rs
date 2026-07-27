//! The Settings dialog, rendered as a Material modal overlay within the main window (feature
//! 006, FR-019/FR-020). Currently exposes the embedded-terminal scrollback limit.

use crate::app::{Message, SettingsDraft};
use micold_core::tokens::{self, spacing, type_scale};
use crate::ui::cdk::overlay::Surface;
use crate::ui::material::Modal;
use crate::ui::style;
use iced::widget::{button, checkbox, column, container, row, text, text_input};
use iced::Length;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::theme::ColorScheme;

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

/// The Settings dialog as a modal surface, at transition `progress`
/// (1.0 = fully shown, 0.0 = hidden — see [`Modal`]). `env_include_outcome` is the most recent
/// environment-include resolution attempt's result, rendered as a read-only failure note when
/// it didn't succeed (feature 011, FR-012/FR-013).
pub fn modal<'a>(
    draft: &'a SettingsDraft,
    scheme: ColorScheme,
    progress: f32,
    env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Surface<'a, Message>> {
    let r = tokens::roles(scheme);

    let input = text_input("Scrollback lines", &draft.scrollback_lines)
        .on_input(Message::SettingsScrollbackChanged)
        .on_submit(Message::SettingsSaved)
        .padding(spacing::SM)
        .style(style::input(r));

    // Environment-include: enabled flag, script path, timeout — grouped as one visually
    // distinct set, separate from the scrollback field above (FR-015).
    let env_include_enabled_checkbox = checkbox(draft.env_include_enabled)
        .label("Enabled")
        .on_toggle(Message::SettingsEnvIncludeEnabledToggled)
        .style(style::checkbox(r));
    let env_include_path_input = text_input("Script path", &draft.env_include_script_path)
        .on_input(Message::SettingsEnvIncludePathChanged)
        .on_submit(Message::SettingsSaved)
        .padding(spacing::SM)
        .style(style::input(r));
    let env_include_timeout_input = text_input("Timeout (seconds)", &draft.env_include_timeout)
        .on_input(Message::SettingsEnvIncludeTimeoutChanged)
        .on_submit(Message::SettingsSaved)
        .padding(spacing::SM)
        .style(style::input(r));

    let mut fields = column![
        text("Settings").size(type_scale::HEADLINE),
        text("Terminal scrollback limit (lines)")
            .size(type_scale::LABEL)
            .style(style::muted(r)),
        input,
        text("Environment include")
            .size(type_scale::LABEL)
            .style(style::muted(r)),
        env_include_enabled_checkbox,
        env_include_path_input,
        env_include_timeout_input,
    ]
    .spacing(spacing::MD);

    if let Some((label, diagnostic)) = failure_label_and_diagnostic(env_include_outcome) {
        fields = fields.push(text(label).size(type_scale::LABEL).style(
            move |_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(style::color(r.error)),
            },
        ));
        if !diagnostic.is_empty() {
            fields = fields.push(
                text(diagnostic.to_string())
                    .size(type_scale::LABEL)
                    .style(style::muted(r)),
            );
        }
    }

    if let Some(error) = &draft.error {
        let error = error.clone();
        fields = fields.push(text(error).size(type_scale::LABEL).style(
            move |_theme: &iced::Theme| iced::widget::text::Style {
                color: Some(style::color(r.error)),
            },
        ));
    }

    let actions = row![
        button(text("Save").size(type_scale::BODY))
            .on_press(Message::SettingsSaved)
            .style(style::filled(r)),
        button(text("Cancel").size(type_scale::BODY))
            .on_press(Message::SettingsCancelled)
            .style(style::outlined(r)),
    ]
    .spacing(spacing::SM);

    let dialog = container(fields.push(actions))
        .padding(spacing::LG)
        .width(Length::Fixed(420.0))
        .style(style::dialog(r));

    Modal::new(dialog, r).progress(progress).into()
}
