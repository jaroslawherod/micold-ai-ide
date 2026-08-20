//! The Settings dialog, rendered as a Material modal overlay within the main window (feature
//! 006, FR-019/FR-020). Currently exposes the embedded-terminal scrollback limit.

use crate::app::{Message, State};
use crate::features::settings::SettingsDraft;
use crate::features::window::FieldId;
use crate::ui::focus::TrackFocus;
use crate::ui::material::{self, Button, Checkbox, Select, SurfaceKind, Text, TextField, TypeRole};
use iced::widget::{column, row};
use iced::{Element, Length};
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::session::AiCli;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{self};

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
    focused: Option<FieldId>,
    available_providers: &'a [AiCli],
) -> Element<'a, Message> {
    let r = tokens::roles(scheme);

    let input = TextField::new("", &draft.scrollback_lines, r)
        .label("Scrollback lines")
        .supporting("Lines kept per terminal")
        .track_focus(FieldId::SettingsScrollback, focused)
        .on_input(Message::SettingsScrollbackChanged)
        .on_submit(Message::SettingsSaved);

    // Environment-include: enabled flag, script path, timeout — grouped as one visually
    // distinct set, separate from the scrollback field above (FR-015).
    let env_include_enabled_checkbox = Checkbox::new("Enabled", draft.env_include_enabled, r)
        .track_focus(FieldId::SettingsEnvIncludeEnabled, focused)
        .on_toggle(Message::SettingsEnvIncludeEnabledToggled);
    let env_include_path_input = TextField::new("", &draft.env_include_script_path, r)
        .label("Script path")
        .track_focus(FieldId::SettingsEnvIncludePath, focused)
        .on_input(Message::SettingsEnvIncludePathChanged)
        .on_submit(Message::SettingsSaved);
    let env_include_timeout_input = TextField::new("", &draft.env_include_timeout, r)
        .label("Timeout")
        .supporting("Seconds")
        .track_focus(FieldId::SettingsEnvIncludeTimeout, focused)
        .on_input(Message::SettingsEnvIncludeTimeoutChanged)
        .on_submit(Message::SettingsSaved);

    // The Default AI CLI (feature 026, FR-003/FR-006). The shared `Select` component, not a
    // bespoke control (Principle VIII) — and the options are `available_providers`, which arrives
    // as a slice because this view is dispatched through the shared `DialogView` fn-pointer and
    // sees `&State` and nothing else (T014a).
    //
    // Named by `display_name()` through `Display`, which is a menu's register. `command()` — the
    // `claude`/`copilot` a sidebar row carries — is not used here: a menu entry is not a label in a
    // width budget (Clarifications 2026-08-18).
    let default_ai_cli_select = Select::new(
        available_providers,
        Some(draft.default_ai_cli),
        Message::SettingsDefaultAiCliChanged,
        r,
    )
    .label("Default AI CLI")
    .supporting("Used for new sessions unless you choose otherwise");

    let mut fields = material::dialog::fields(column![
        Text::new("Settings", TypeRole::Headline, r),
        // No free-standing label above the field: it carries its own now (§7.7, FR-031a). Leaving
        // this one would have shown the field's name twice — once above the container and once
        // inside it — which is the duplication the migration exists to remove.
        input,
        default_ai_cli_select,
        // This one stays. It names the *group* below it — a checkbox and two fields — rather than
        // any single control, so it is a section heading and has no container to move into.
        Text::new("Environment include", TypeRole::Label, r).muted(),
        env_include_enabled_checkbox,
        env_include_path_input,
        env_include_timeout_input,
    ]);

    if let Some((label, diagnostic)) = failure_label_and_diagnostic(env_include_outcome) {
        fields = fields.push(Text::new(label, TypeRole::Caption, r).tint(r.error));
        if !diagnostic.is_empty() {
            fields = fields.push(Text::new(diagnostic, TypeRole::Caption, r).muted());
        }
    }

    if let Some(error) = &draft.error {
        fields = fields.push(Text::new(error.clone(), TypeRole::Caption, r).tint(r.error));
    }

    let actions = material::dialog::actions(row![
        Button::filled("Save", r).on_press(Message::SettingsSaved),
        Button::outlined("Cancel", r).on_press(Message::SettingsCancelled),
    ]);

    let dialog = material::Surface::new(
        material::dialog::body(fields, actions),
        SurfaceKind::Dialog,
        r,
    )
    .width(Length::Fixed(420.0));

    dialog.into()
}

/// This dialog's body, built from the state that opened it — `None` when the surface is open but
/// the live state it renders is absent, so nothing is drawn rather than an empty dialog.
///
/// The uniform shape every registered dialog has, and the reason `ui::view` no longer needs a
/// match: the registration line in [`crate::overlay::registry`] names this beside the surface it
/// draws, so a dialog says where its own state lives instead of a central arm saying it for them
/// all (feature 021, T035 — FR-008, FR-009).
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    state.settings_draft.as_ref().map(|draft| {
        modal(
            draft,
            scheme,
            env_include_outcome,
            state.focused_field,
            &state.available_providers,
        )
    })
}
