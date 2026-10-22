//! A structured session's commands (#943): the command list the CLI reported, merged with the
//! terminal-only list (research R1, R2), and how a draft that starts with `/` is read (R3).

pub mod palette;
mod terminal_only;

pub use terminal_only::{TerminalOnly, BUILT_INS, CHECKED_AGAINST, CLI_MARKED_WHY, TERMINAL_ONLY};

use super::{ControlOutcome, ReportedCommand, ReportedModel, SequencedEvent, StructuredEvent};
use serde_json::Value;
use std::collections::HashMap;

/// The app's own command: it opens the model picker and is never sent (FR-018).
pub const MODEL_COMMAND: &str = "model";

/// What the palette says of `/model` when the CLI gives no description.
const MODEL_DESCRIPTION: &str = "Choose the model for this session";

/// The longest description kept from the CLI's answer, in bytes (data-model *Events*).
pub const MAX_DESCRIPTION_BYTES: usize = 512;

/// A command's kind, in the palette's group order (FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CommandKind {
    Skill,
    Custom,
    BuiltIn,
    /// Handled by the app, never sent: `/model`.
    App,
}

/// One command of a session's list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    /// The name, without the `/`.
    pub name: String,
    pub kind: CommandKind,
    pub description: Option<String>,
    pub argument_hint: Option<String>,
    pub aliases: Vec<String>,
    /// `Some(why)` for a command that opens in the terminal (FR-010).
    pub terminal_only: Option<&'static str>,
}

impl Command {
    /// Whether `name` is this command's name or one of its aliases.
    pub fn answers_to(&self, name: &str) -> bool {
        self.name == name || self.aliases.iter().any(|alias| alias == name)
    }
}

/// Where a command list came from (FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListSource {
    /// What the session's CLI reported at its latest start the history holds.
    Session,
    /// None known: the built-ins the app knows (US2 scenario 8).
    Fallback,
}

/// A session's commands (research R1): every documented terminal-only command and `/model` are
/// always in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandList {
    commands: Vec<Command>,
    source: ListSource,
}

impl CommandList {
    pub fn commands(&self) -> &[Command] {
        &self.commands
    }

    pub fn source(&self) -> ListSource {
        self.source
    }

    /// The list a history implies (R1): the latest `CommandsReported` note's commands, kinds
    /// from the latest `init`'s `skills`; when that note reports none (its spawn's `initialize`
    /// was refused) or there is none, the latest `init`'s names; without either,
    /// [`fallback`](Self::fallback). A later report replaces an earlier one.
    pub fn from_history(events: &[SequencedEvent]) -> Self {
        let reported = events
            .iter()
            .rev()
            .find_map(|e| match &e.event {
                StructuredEvent::CommandsReported { commands, .. } => Some(commands),
                _ => None,
            })
            .filter(|commands| !commands.is_empty());
        let init = events.iter().rev().find_map(|e| match &e.event {
            StructuredEvent::Init {
                slash_commands,
                skills,
                terminal_slash_commands,
                ..
            } => Some((slash_commands, skills, terminal_slash_commands)),
            _ => None,
        });
        if reported.is_none() && init.is_none() {
            return Self::fallback();
        }
        let none = Vec::new();
        let (names, skills, cli_terminal) = init.unwrap_or((&none, &none, &none));
        // One slot per time a name is listed as a skill, so a skill and a custom command of one
        // name stay apart (Edge Cases *Name clashes*).
        let mut skill_slots: HashMap<&str, usize> = HashMap::new();
        for skill in skills {
            *skill_slots.entry(skill).or_default() += 1;
        }
        let mut is_skill = |candidates: &[&str]| {
            candidates
                .iter()
                .any(|name| match skill_slots.get_mut(*name) {
                    Some(slots) if *slots > 0 => {
                        *slots -= 1;
                        true
                    }
                    _ => false,
                })
        };
        let commands = match reported {
            Some(reported) => reported
                .iter()
                .map(|c| {
                    let names: Vec<&str> = std::iter::once(c.name.as_str())
                        .chain(c.aliases.iter().map(String::as_str))
                        .collect();
                    let kind = if c.name == MODEL_COMMAND {
                        CommandKind::App
                    } else if is_skill(&names) {
                        CommandKind::Skill
                    } else if c.builtin {
                        CommandKind::BuiltIn
                    } else {
                        CommandKind::Custom
                    };
                    Command {
                        name: c.name.clone(),
                        kind,
                        description: c.description.clone(),
                        argument_hint: c.argument_hint.clone(),
                        aliases: c.aliases.clone(),
                        terminal_only: None,
                    }
                })
                .collect(),
            None => names
                .iter()
                .map(|name| {
                    let kind = if name == MODEL_COMMAND {
                        CommandKind::App
                    } else if is_skill(&[name.as_str()]) {
                        CommandKind::Skill
                    } else {
                        CommandKind::Custom
                    };
                    bare(name.clone(), kind)
                })
                .collect(),
        };
        completed(commands, cli_terminal, ListSource::Session)
    }

    /// The list when the session's own is not known: [`BUILT_INS`], [`TERMINAL_ONLY`] and
    /// `/model` (FR-009).
    pub fn fallback() -> Self {
        let built_ins = BUILT_INS
            .iter()
            .map(|name| bare(name.to_string(), CommandKind::BuiltIn))
            .collect();
        completed(built_ins, &[], ListSource::Fallback)
    }
}

fn bare(name: String, kind: CommandKind) -> Command {
    Command {
        name,
        kind,
        description: None,
        argument_hint: None,
        aliases: Vec::new(),
        terminal_only: None,
    }
}

/// Marks the terminal-only commands, by [`TERMINAL_ONLY`] or the CLI's own list, by name or alias
/// (FR-011), and adds every [`TERMINAL_ONLY`] entry and `/model` the list lacks.
fn completed(
    mut commands: Vec<Command>,
    cli_terminal: &[String],
    source: ListSource,
) -> CommandList {
    for command in &mut commands {
        if command.kind == CommandKind::App {
            continue;
        }
        command.terminal_only = TERMINAL_ONLY
            .iter()
            .find(|t| {
                std::iter::once(&t.name)
                    .chain(t.aliases)
                    .any(|name| command.answers_to(name))
            })
            .map(|t| t.why)
            .or_else(|| {
                cli_terminal
                    .iter()
                    .any(|name| command.answers_to(name))
                    .then_some(CLI_MARKED_WHY)
            });
    }
    for entry in TERMINAL_ONLY {
        if !commands.iter().any(|c| c.answers_to(entry.name)) {
            let mut command = bare(entry.name.to_string(), CommandKind::BuiltIn);
            command.aliases = entry.aliases.iter().map(|a| a.to_string()).collect();
            command.terminal_only = Some(entry.why);
            commands.push(command);
        }
    }
    if !commands.iter().any(|c| c.kind == CommandKind::App) {
        let mut model = bare(MODEL_COMMAND.to_string(), CommandKind::App);
        model.description = Some(MODEL_DESCRIPTION.to_string());
        commands.push(model);
    }
    CommandList { commands, source }
}

/// The commands and models of the CLI's `initialize` answer (R1, contracts/claude-commands.md).
/// Entries with a blank or non-string `name`/`value` are dropped, descriptions are cut to
/// [`MAX_DESCRIPTION_BYTES`]; a refused or unreadable answer reports nothing.
pub fn parse_initialize(outcome: &ControlOutcome) -> (Vec<ReportedCommand>, Vec<ReportedModel>) {
    let ControlOutcome::Success(text) = outcome else {
        return (Vec::new(), Vec::new());
    };
    let Ok(answer) = serde_json::from_str::<Value>(text) else {
        return (Vec::new(), Vec::new());
    };
    let entries = |key: &str| answer[key].as_array().cloned().unwrap_or_default();
    let commands = entries("commands")
        .iter()
        .filter_map(|c| {
            Some(ReportedCommand {
                name: non_blank(&c["name"])?,
                description: non_blank(&c["description"]).map(cut),
                argument_hint: non_blank(&c["argumentHint"]),
                aliases: c["aliases"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(non_blank)
                    .collect(),
                builtin: c["builtin"].as_bool() == Some(true),
            })
        })
        .collect();
    let models = entries("models")
        .iter()
        .filter_map(|m| {
            Some(ReportedModel {
                value: non_blank(&m["value"])?,
                display_name: non_blank(&m["displayName"]),
                description: non_blank(&m["description"]).map(cut),
            })
        })
        .collect();
    (commands, models)
}

/// A string value that is not blank.
fn non_blank(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .map(str::to_string)
}

/// `text` cut to [`MAX_DESCRIPTION_BYTES`] at a char boundary.
fn cut(mut text: String) -> String {
    if text.len() > MAX_DESCRIPTION_BYTES {
        let mut end = MAX_DESCRIPTION_BYTES;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

/// What sending a draft does (FR-001, FR-010, FR-018).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftCommand {
    /// Sent to the CLI as typed.
    Send,
    /// `/model`: opens the picker, or picks the name given.
    Model(Option<String>),
    /// A terminal-only command, by the name typed: offers **Open in terminal**.
    TerminalOnly(String),
}

/// A draft read as `/name args` (FR-003).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInvocation {
    /// The name, without the `/`.
    pub name: String,
    /// The rest as typed, without the whitespace that ends the name.
    pub args: String,
}

/// `Some` for a draft that starts with `/` and a name (it ends at whitespace or the end);
/// `None` for anything else, including `/` alone and `/` then whitespace.
pub fn parse_invocation(text: &str) -> Option<CommandInvocation> {
    let rest = text.strip_prefix('/')?;
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    let name = &rest[..end];
    if name.is_empty() {
        return None;
    }
    Some(CommandInvocation {
        name: name.to_string(),
        args: rest[end..].trim_start().to_string(),
    })
}

/// How the app handles sending `text` with `list` the session's commands (R3).
pub fn classify_draft(text: &str, list: &CommandList) -> DraftCommand {
    let Some(invocation) = parse_invocation(text) else {
        return DraftCommand::Send;
    };
    if invocation.name == MODEL_COMMAND {
        let name = invocation.args.trim();
        return DraftCommand::Model((!name.is_empty()).then(|| name.to_string()));
    }
    let terminal_only = list
        .commands()
        .iter()
        .any(|c| c.terminal_only.is_some() && c.answers_to(&invocation.name));
    if terminal_only {
        DraftCommand::TerminalOnly(invocation.name)
    } else {
        DraftCommand::Send
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structured::{parse_line, SequencedEvent};

    /// The fallback list plus an init whose CLI marks `doctor` terminal-only and offers `greet`.
    fn list() -> CommandList {
        let init = parse_line(
            r#"{"type":"system","subtype":"init","session_id":"s","slash_commands":["greet","doctor","compact","model","speckit-plan"],"skills":["greet","speckit-plan"],"terminal_slash_commands":["doctor"]}"#,
        );
        CommandList::from_history(&[SequencedEvent {
            seq: 0,
            event: init,
        }])
    }

    #[test]
    fn a_draft_is_sent_unless_it_is_model_or_a_terminal_only_command() {
        let list = list();
        let cases: &[(&str, DraftCommand)] = &[
            ("hello", DraftCommand::Send),
            ("/", DraftCommand::Send),
            ("/ x", DraftCommand::Send),
            ("/speckit-plan #1", DraftCommand::Send),
            ("/greet Ada", DraftCommand::Send),
            ("/nosuchcmd", DraftCommand::Send),
            ("tell me about /mcp", DraftCommand::Send),
            (" /mcp", DraftCommand::Send),
            ("/model", DraftCommand::Model(None)),
            ("/model   ", DraftCommand::Model(None)),
            ("/model\n", DraftCommand::Model(None)),
            ("/model haiku", DraftCommand::Model(Some("haiku".into()))),
            (
                "/model  claude-opus-5-5 ",
                DraftCommand::Model(Some("claude-opus-5-5".into())),
            ),
            ("/models", DraftCommand::Send),
            ("/mcp", DraftCommand::TerminalOnly("mcp".into())),
            (
                "/mcp reconnect all",
                DraftCommand::TerminalOnly("mcp".into()),
            ),
            ("/settings", DraftCommand::TerminalOnly("settings".into())),
            ("/login", DraftCommand::TerminalOnly("login".into())),
            ("/doctor", DraftCommand::TerminalOnly("doctor".into())),
        ];
        for (draft, expected) in cases {
            assert_eq!(&classify_draft(draft, &list), expected, "{draft:?}");
        }
    }

    #[test]
    fn the_fallback_list_alone_classifies_the_documented_terminal_only_commands() {
        let fallback = CommandList::fallback();
        assert_eq!(
            classify_draft("/config theme=dark", &fallback),
            DraftCommand::TerminalOnly("config".into())
        );
        assert_eq!(classify_draft("/doctor", &fallback), DraftCommand::Send);
    }

    #[test]
    fn an_invocation_keeps_its_arguments_as_typed() {
        let invocation = |text: &str| parse_invocation(text).map(|i| (i.name, i.args));
        assert_eq!(
            invocation("/greet Ada"),
            Some(("greet".into(), "Ada".into()))
        );
        assert_eq!(
            invocation(r#"/hello "big" world"#),
            Some(("hello".into(), r#""big" world"#.into()))
        );
        assert_eq!(
            invocation("/compact keep\nthe greetings\n"),
            Some(("compact".into(), "keep\nthe greetings\n".into()))
        );
        assert_eq!(
            invocation("/greet Zoë 日本"),
            Some(("greet".into(), "Zoë 日本".into()))
        );
        assert_eq!(
            invocation("/greet\nAda"),
            Some(("greet".into(), "Ada".into()))
        );
        assert_eq!(invocation("/clear"), Some(("clear".into(), String::new())));
        assert_eq!(
            invocation("/plugin:skill x"),
            Some(("plugin:skill".into(), "x".into()))
        );
        for text in ["hello", "/", "/ x", "", " /greet", "tell me about /mcp"] {
            assert_eq!(parse_invocation(text), None, "{text:?}");
        }
    }
}
