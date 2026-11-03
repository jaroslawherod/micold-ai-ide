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
/// never reaches the daemon's status, and the start script shows its outcome, reason and time only,
/// never its variables or output (FR-029).
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

/// One line of the Configuration section: a sentence, or a block of file text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigRow {
    Note(String),
    Code { text: String, toml: bool },
}

/// Everything the Configuration section shows, in order: the file the application delivered, what
/// the daemon resolved from it, the reach of each token and the start script's last run.
pub fn configuration_rows(
    delivered: Option<String>,
    status: Option<&ConfigStatus>,
) -> Vec<ConfigRow> {
    let note = |t: &str| ConfigRow::Note(t.to_string());
    let mut rows = Vec::new();
    match delivered {
        Some(text) => {
            rows.push(note("Delivered file"));
            rows.push(ConfigRow::Code { text, toml: true });
        }
        None => rows.push(note("This daemon is given no configuration file")),
    }
    let Some(status) = status else {
        rows.push(note("The daemon has not reported its configuration"));
        return rows;
    };
    rows.extend(configuration_lines(status).into_iter().map(ConfigRow::Note));
    rows.push(note("Resolved values"));
    rows.push(ConfigRow::Code {
        text: status.resolved.clone(),
        toml: false,
    });
    rows
}

fn configuration_block<'a>(
    delivered: Option<String>,
    status: Option<&ConfigStatus>,
    roles: micold_core::tokens::Roles,
) -> Element<'a, Message> {
    let mut block =
        column![group("Configuration", roles)].spacing(micold_core::tokens::spacing::XS);
    for row in configuration_rows(delivered, status) {
        block = match row {
            ConfigRow::Note(t) => block.push(note(t, roles)),
            ConfigRow::Code { text, toml: true } => {
                block.push(CodeBlock::new(text, roles).language("toml"))
            }
            ConfigRow::Code { text, .. } => block.push(CodeBlock::new(text, roles)),
        };
    }
    block.into()
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

    fn texts(rows: &[ConfigRow]) -> String {
        rows.iter()
            .map(|r| match r {
                ConfigRow::Note(t) | ConfigRow::Code { text: t, .. } => t.as_str(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The daemon reads the secret from its file and reports only the path and readability; the
    /// delivered file names the secret's path. Feed the real value to the daemon's resolver and
    /// show everything the dialog renders from it. It guards against the view ever reading a token
    /// file; the status type itself holds no secret content.
    #[test]
    fn no_secret_value_is_in_any_rendered_text() {
        use micold_core::daemon_config;
        let dir = std::env::temp_dir().join(format!("micold-1535-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let secret_path = dir.join("work");
        std::fs::write(&secret_path, SECRET).unwrap();
        let toml = format!(
            "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\n[listen]\nsocket = \"auto\"\n[[github.tokens]]\nname = \"work\"\ntoken_file = \"{}\"\n",
            secret_path.display()
        );
        let config = daemon_config::parse(&toml, "test").expect("the fixture parses");
        let mut s = status();
        s.resolved = daemon_config::resolved_view(&config).to_string();
        let rows = configuration_rows(Some(toml.clone()), Some(&s));
        let text = texts(&rows);
        std::fs::remove_dir_all(&dir).ok();
        assert!(
            text.contains("Token work"),
            "the fixture reached the view:\n{text}"
        );
        assert!(!text.contains(SECRET), "secret value shown:\n{text}");
    }

    #[test]
    fn no_report_says_so_and_still_shows_the_delivered_file() {
        let text = texts(&configuration_rows(Some("schema = 1\n".into()), None));
        assert!(text.contains("Delivered file") && text.contains("schema = 1"));
        assert!(text.contains("The daemon has not reported its configuration"));
        assert!(!text.contains("Resolved values"));
    }

    #[test]
    fn no_delivered_file_and_a_report_show_the_resolved_values() {
        let rows = configuration_rows(None, Some(&status()));
        let text = texts(&rows);
        assert!(text.contains("This daemon is given no configuration file"));
        assert!(!text.contains("Delivered file"));
        assert!(text.contains("Resolved values"));
        assert!(rows.contains(&ConfigRow::Code {
            text: status().resolved,
            toml: false
        }));
    }

    #[test]
    fn a_failed_load_is_shown_as_not_applied() {
        let mut s = status();
        s.last_error = Some("daemon.listen: bad port".into());
        let text = texts(&configuration_rows(None, Some(&s)));
        assert!(text.contains("Not applied: daemon.listen: bad port"));
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

    /// FR-029: the start script's output tail may echo what it read, so no rendered text has it.
    #[test]
    fn the_start_script_output_tail_is_never_rendered() {
        const TAIL: &str = "TAIL_ECHOED_SECRET_9f3a";
        let mut s = status();
        s.startup.as_mut().expect("the fixture ran").tail = vec![TAIL.into()];
        let lines = configuration_lines(&s).join("\n");
        let rows = texts(&configuration_rows(Some("schema = 1\n".into()), Some(&s)));
        assert!(!lines.contains(TAIL), "tail in the lines:\n{lines}");
        assert!(!rows.contains(TAIL), "tail in the rows:\n{rows}");
        assert!(
            lines.contains("Start script:"),
            "the fixture reached the line"
        );
    }

    /// Every outcome and every reason has its own exact line (US6: outcome and reason).
    #[test]
    fn the_start_script_line_names_each_outcome_and_reason() {
        let cases = [
            (StartupOutcome::Ok, None, "ok"),
            (
                StartupOutcome::Skipped,
                None,
                "skipped (nothing it reads changed)",
            ),
            (
                StartupOutcome::Warned,
                Some(StartupReason::Exit(3)),
                "failed, daemon serves (exit 3)",
            ),
            (
                StartupOutcome::Failed,
                Some(StartupReason::Signal(9)),
                "failed, daemon refused to start (signal 9)",
            ),
            (
                StartupOutcome::Warned,
                Some(StartupReason::Timeout),
                "failed, daemon serves (timed out)",
            ),
            (
                StartupOutcome::Failed,
                Some(StartupReason::Spawn("no such file".into())),
                "failed, daemon refused to start (could not start: no such file)",
            ),
            (
                StartupOutcome::Failed,
                Some(StartupReason::FileMissing("/x/start.sh".into())),
                "failed, daemon refused to start (file unreadable: /x/start.sh)",
            ),
        ];
        for (outcome, reason, want) in cases {
            let mut s = status();
            s.startup = Some(StartupReport {
                outcome,
                reason,
                tail: Vec::new(),
                finished_at_secs: 1_700_000_000,
            });
            let lines = configuration_lines(&s);
            assert_eq!(
                lines.last().map(String::as_str),
                Some(format!("Start script: {want} at 1700000000").as_str()),
                "{lines:?}"
            );
        }
    }
}
