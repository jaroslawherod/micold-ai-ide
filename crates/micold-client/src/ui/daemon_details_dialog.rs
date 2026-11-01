//! A daemon's details, read-only, as a modal dialog with Close (feature 1092, FR-012).
//!
//! The body is the Daemons page's own [`details_block`](crate::ui::settings::daemons::details_block),
//! computed from live state on every view, so a new log line or error shows without reopening.

use crate::app::{Message, State};
use crate::features::daemons::Msg as DaemonsMsg;
use crate::ui::material::{dialog, Button, CodeBlock, ScrollingDialog};
use crate::ui::settings::daemons::details_block;
use crate::ui::settings::{group, note};
use iced::widget::{column, row};
use iced::Element;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::protocol::messages::{ConfigStatus, StartupOutcome, StartupReason};
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// This dialog's body, built from the state that opened it.
pub fn dialog<'a>(
    state: &'a State,
    scheme: ColorScheme,
    _env_include_outcome: &'a EnvIncludeOutcome,
) -> Option<Element<'a, Message>> {
    let id = state.daemon_list.details?;
    let name = state.settings.daemons.get(id)?.name.as_str().to_string();
    let r = tokens::roles(scheme);
    let details = crate::features::daemons::details(&state.daemon_list, id);
    let status = details.config.clone();
    let delivered = state
        .settings
        .daemons
        .get(id)
        .and_then(crate::daemon_config_render::DaemonSettings::from_entry)
        .map(|(settings, kind)| crate::daemon_config_render::render(&settings, kind));
    let restart = status.as_ref().is_some_and(|s| s.restart_needed);
    let body = column![
        details_block(details, r),
        configuration_block(delivered, status.as_ref(), r)
    ]
    .spacing(micold_core::tokens::spacing::MD);
    let mut buttons = row![].spacing(micold_core::tokens::spacing::SM);
    if restart {
        buttons = buttons.push(
            Button::filled("Restart to apply", r)
                .on_press(Message::Daemons(DaemonsMsg::RestartRequested(id))),
        );
    }
    buttons = buttons
        .push(Button::filled("Close", r).on_press(Message::Daemons(DaemonsMsg::DetailsClosed)));
    let actions = dialog::actions(buttons);
    let mut built =
        ScrollingDialog::new(name, Element::from(body), actions, r).actions_height(40.0);
    let (_, height) = state.window.window_size;
    if height > 0 {
        built = built.max_height(f32::from(height));
    }
    Some(built.into())
}

/// The sentences of the Configuration section, in order. Names and paths only: a secret's content
/// never reaches the daemon's status, and the start script's variables show by name (FR-029).
pub fn configuration_lines(status: &ConfigStatus) -> Vec<String> {
    let mut lines = vec![format!(
        "{} ({}, schema {})",
        status.path.display(),
        status.kind,
        status.schema
    )];
    if status.restart_needed {
        lines.push(format!(
            "Restart to apply: {}",
            status.changed_restart_keys.join(", ")
        ));
    }
    if let Some(error) = &status.last_error {
        lines.push(format!("Not applied: {error}"));
    }
    for token in &status.tokens {
        let reach = if token.repos.is_empty() {
            "every repository it reaches".to_string()
        } else {
            token.repos.join(", ")
        };
        lines.push(format!("Token {}: {reach}", token.name));
    }
    lines.push(match &status.startup {
        None => "Start script: has not run".to_string(),
        Some(run) => {
            // The outcome and reason only: the output tail may echo what the script read.
            let outcome = match run.outcome {
                StartupOutcome::Ok => "ok",
                StartupOutcome::Skipped => "skipped (nothing it reads changed)",
                StartupOutcome::Warned => "failed, daemon serves",
                StartupOutcome::Failed => "failed, daemon refused to start",
            };
            let reason = run.reason.as_ref().map_or_else(String::new, |r| match r {
                StartupReason::Exit(c) => format!(" (exit {c})"),
                StartupReason::Signal(n) => format!(" (signal {n})"),
                StartupReason::Timeout => " (timed out)".to_string(),
                StartupReason::Spawn(e) => format!(" (could not start: {e})"),
                StartupReason::FileMissing(f) => format!(" (file unreadable: {f})"),
            });
            format!(
                "Start script: {outcome}{reason} at {}",
                run.finished_at_secs
            )
        }
    });
    lines
}

/// Configuration: the file the application delivered, what the daemon resolved from it, the reach
/// of each token and the start script's last run.
fn configuration_block<'a>(
    delivered: Option<String>,
    status: Option<&ConfigStatus>,
    roles: micold_core::tokens::Roles,
) -> Element<'a, Message> {
    let mut block =
        column![group("Configuration", roles)].spacing(micold_core::tokens::spacing::XS);
    match delivered {
        Some(text) => {
            block = block.push(note("Delivered file", roles));
            block = block.push(CodeBlock::new(text, roles).language("toml"));
        }
        None => block = block.push(note("This daemon is given no configuration file", roles)),
    }
    let Some(status) = status else {
        return block
            .push(note("The daemon has not reported its configuration", roles))
            .into();
    };
    for line in configuration_lines(status) {
        block = block.push(note(line, roles));
    }
    block = block.push(note("Resolved values", roles));
    block
        .push(CodeBlock::new(status.resolved.clone(), roles))
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::protocol::messages::{StartupReport, TokenReach};

    const SECRET: &str = "ghp_SECRET_VALUE_1234";

    fn status() -> ConfigStatus {
        ConfigStatus {
            path: "/run/micold/config/daemon.toml".into(),
            schema: 1,
            kind: "container".into(),
            applied_hash: "ab".repeat(32),
            resolved: "token work: /run/secrets/work (readable, 0600)\n".into(),
            restart_needed: true,
            changed_restart_keys: vec!["daemon.log".into()],
            last_error: None,
            tokens: vec![
                TokenReach {
                    name: "work".into(),
                    repos: vec!["acme/*".into(), "acme/app".into()],
                },
                TokenReach {
                    name: "all".into(),
                    repos: Vec::new(),
                },
            ],
            startup: Some(StartupReport {
                outcome: StartupOutcome::Warned,
                reason: Some(StartupReason::Exit(3)),
                tail: vec!["line".into()],
                finished_at_secs: 1_700_000_000,
            }),
        }
    }

    #[test]
    fn the_configuration_names_reach_start_script_and_restart() {
        let text = configuration_lines(&status()).join("\n");
        for want in [
            "/run/micold/config/daemon.toml",
            "Restart to apply: daemon.log",
            "Token work: acme/*, acme/app",
            "Token all: every repository it reaches",
            "Start script: failed, daemon serves (exit 3) at 1700000000",
        ] {
            assert!(text.contains(want), "{want:?} missing from:\n{text}");
        }
    }

    #[test]
    fn no_secret_value_is_in_the_rendered_text() {
        // A secret's content is never in the status; only names and paths are.
        let mut s = status();
        s.resolved.push_str("secret env: NPM_REGISTRY\n");
        let text = configuration_lines(&s).join("\n") + &s.resolved;
        assert!(!text.contains(SECRET));
    }

    #[test]
    fn a_script_that_never_ran_and_no_restart_say_so() {
        let mut s = status();
        s.restart_needed = false;
        s.startup = None;
        let text = configuration_lines(&s).join("\n");
        assert!(text.contains("Start script: has not run"));
        assert!(!text.contains("Restart to apply"));
    }
}
