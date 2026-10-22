//! #943 T010 — the session command list (FR-007, FR-009, FR-011; research R1, R2): the CLI's
//! `initialize` answer read into reported commands and models, and merged with the `init` lists
//! and the terminal-only list. Fixtures: `tests/fixtures/stream-json/README.md`.

use micold_core::structured::commands::{
    parse_initialize, CommandKind, CommandList, ListSource, BUILT_INS, TERMINAL_ONLY,
};
use micold_core::structured::{
    ControlOutcome, ReportedCommand, ReportedModel, SequencedEvent, StructuredEvent,
};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{events_of, lines_of, sequenced};

/// The outcome of the fixture's one `initialize` request.
fn initialize_outcome() -> ControlOutcome {
    events_of(&lines_of("command-initialize"))
        .into_iter()
        .find_map(|e| match e.event {
            StructuredEvent::ControlResponse { outcome, .. } => Some(outcome),
            _ => None,
        })
        .expect("command-initialize answers its initialize request")
}

fn reported() -> (Vec<ReportedCommand>, Vec<ReportedModel>) {
    parse_initialize(&initialize_outcome())
}

fn command<'a>(commands: &'a [ReportedCommand], name: &str) -> &'a ReportedCommand {
    commands
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("{name} reported"))
}

#[test]
fn the_initialize_answer_gives_every_command_and_model() {
    let (commands, models) = reported();
    assert_eq!(commands.len(), 70, "every command the fixture reports");
    let config = command(&commands, "config");
    assert_eq!(config.description.as_deref(), Some("Set a setting by key"));
    assert_eq!(config.argument_hint.as_deref(), Some("key=value"));
    assert_eq!(config.aliases, ["settings"]);
    assert!(config.builtin);
    let hello = command(&commands, "hello");
    assert_eq!(
        hello.description.as_deref(),
        Some("Say hello to whatever the arguments name. (project)")
    );
    assert_eq!(hello.argument_hint.as_deref(), Some("<what to greet>"));
    assert!(!hello.builtin, "a project command is not built in");
    assert_eq!(
        command(&commands, "context").argument_hint,
        None,
        "an empty hint is none"
    );
    assert!(command(&commands, "greet").aliases.is_empty());
    let values: Vec<&str> = models.iter().map(|m| m.value.as_str()).collect();
    assert_eq!(values, ["default", "sonnet", "fable", "opus", "haiku"]);
    let haiku = &models[4];
    assert_eq!(haiku.display_name.as_deref(), Some("Haiku"));
    assert!(haiku.description.is_some());
}

#[test]
fn entries_without_a_usable_name_or_value_are_dropped_and_descriptions_are_cut() {
    let long = "é".repeat(300);
    let answer = serde_json::json!({
        "commands": [
            {"name": ""}, {"name": "  "}, {"name": 7}, {"description": "no name"}, "text",
            {"name": "kept", "description": long, "argumentHint": 5, "aliases": ["a", 3, ""], "builtin": "yes"},
        ],
        "models": [{"value": ""}, {"value": null}, {"displayName": "x"}, {"value": "m", "displayName": 2}],
    });
    let (commands, models) = parse_initialize(&ControlOutcome::Success(answer.to_string()));
    assert_eq!(commands.len(), 1);
    let kept = &commands[0];
    assert_eq!(kept.name, "kept");
    let description = kept.description.as_deref().expect("a description");
    assert_eq!(
        description.len(),
        512,
        "cut to 512 bytes at a char boundary"
    );
    assert!(long.starts_with(description));
    assert_eq!(kept.argument_hint, None);
    assert_eq!(kept.aliases, ["a"]);
    assert!(!kept.builtin, "only `true` is built in");
    assert_eq!(
        models,
        [ReportedModel {
            value: "m".into(),
            display_name: None,
            description: None
        }]
    );
    let (commands, _) = reported();
    let dataviz = command(&commands, "dataviz")
        .description
        .as_deref()
        .unwrap();
    assert!(
        dataviz.len() <= 512,
        "a recorded long description is cut too"
    );
}

#[test]
fn a_refused_or_unreadable_answer_reports_nothing() {
    for outcome in [
        ControlOutcome::Error {
            message: "no".into(),
            code: None,
        },
        ControlOutcome::Success("null".into()),
        ControlOutcome::Success("not json".into()),
        ControlOutcome::Success(r#"{"commands":{},"models":"x"}"#.into()),
    ] {
        assert_eq!(parse_initialize(&outcome), (vec![], vec![]), "{outcome:?}");
    }
}

fn note((commands, models): (Vec<ReportedCommand>, Vec<ReportedModel>)) -> StructuredEvent {
    StructuredEvent::CommandsReported { commands, models }
}

/// The daemon's note at spawn, then the fixture's lines, as a history holds them.
fn spawned_history() -> Vec<SequencedEvent> {
    let lines = events_of(&lines_of("command-initialize"));
    sequenced(std::iter::once(note(reported())).chain(lines.into_iter().map(|e| e.event)))
}

fn kinds_of(list: &CommandList, name: &str) -> Vec<CommandKind> {
    list.commands()
        .iter()
        .filter(|c| c.name == name)
        .map(|c| c.kind)
        .collect()
}

fn terminal_only(list: &CommandList, name: &str) -> bool {
    list.commands()
        .iter()
        .any(|c| c.name == name && c.terminal_only.is_some())
}

#[test]
fn the_kind_is_skill_then_built_in_then_custom_and_model_is_the_apps() {
    let list = CommandList::from_history(&spawned_history());
    assert_eq!(list.source(), ListSource::Session);
    assert_eq!(kinds_of(&list, "greet"), [CommandKind::Skill]);
    assert_eq!(kinds_of(&list, "hello"), [CommandKind::Custom]);
    assert_eq!(kinds_of(&list, "compact"), [CommandKind::BuiltIn]);
    assert_eq!(kinds_of(&list, "model"), [CommandKind::App]);
    assert_eq!(
        kinds_of(&list, "deep-research"),
        [CommandKind::Skill],
        "a bundled skill is built in and a skill: skill wins"
    );
    assert_eq!(
        kinds_of(&list, "startup-hook-skill"),
        [CommandKind::Skill],
        "the init lists it by its alias"
    );
    let compact = list
        .commands()
        .iter()
        .find(|c| c.name == "compact")
        .unwrap();
    assert_eq!(
        compact.description.as_deref(),
        Some("Free up context by summarizing the conversation so far")
    );
}

#[test]
fn terminal_only_commands_come_from_the_list_the_alias_and_the_cli() {
    let list = CommandList::from_history(&spawned_history());
    for name in [
        "config",
        "mcp",
        "agents",
        "doctor",
        "color",
        "focus",
        "reload-plugins",
    ] {
        assert!(terminal_only(&list, name), "{name} is terminal-only");
    }
    for entry in TERMINAL_ONLY {
        assert!(
            terminal_only(&list, entry.name),
            "{} listed even when the CLI does not report it",
            entry.name
        );
    }
    assert!(!terminal_only(&list, "compact"));
    assert!(!terminal_only(&list, "greet"));
    let config = list.commands().iter().find(|c| c.name == "config").unwrap();
    assert_eq!(config.aliases, ["settings"]);
    let login = list.commands().iter().filter(|c| c.name == "login").count();
    assert_eq!(login, 1, "listed once");
}

#[test]
fn a_later_report_replaces_an_earlier_one() {
    let one = |name: &str| ReportedCommand {
        name: name.into(),
        description: None,
        argument_hint: None,
        aliases: vec![],
        builtin: false,
    };
    let history = sequenced([
        note((vec![one("before")], vec![])),
        note((vec![one("after")], vec![])),
    ]);
    let list = CommandList::from_history(&history);
    assert!(kinds_of(&list, "before").is_empty());
    assert_eq!(kinds_of(&list, "after"), [CommandKind::Custom]);
}

#[test]
fn a_skill_and_a_custom_command_of_one_name_are_both_kept() {
    let dup = || ReportedCommand {
        name: "dup".into(),
        description: None,
        argument_hint: None,
        aliases: vec![],
        builtin: false,
    };
    let init = micold_core::structured::parse_line(
        r#"{"type":"system","subtype":"init","session_id":"s","slash_commands":["dup","dup"],"skills":["dup"]}"#,
    );
    let list = CommandList::from_history(&sequenced([note((vec![dup(), dup()], vec![])), init]));
    let mut kinds = kinds_of(&list, "dup");
    kinds.sort();
    assert_eq!(kinds, [CommandKind::Skill, CommandKind::Custom]);
}

#[test]
fn the_init_lists_alone_give_names_without_descriptions() {
    let list = CommandList::from_history(&events_of(&lines_of("command-skill")));
    assert_eq!(list.source(), ListSource::Session);
    assert_eq!(kinds_of(&list, "greet"), [CommandKind::Skill]);
    assert_eq!(kinds_of(&list, "hello"), [CommandKind::Custom]);
    assert_eq!(kinds_of(&list, "model"), [CommandKind::App]);
    assert!(terminal_only(&list, "doctor"));
    assert!(terminal_only(&list, "login"));
    let greet = list.commands().iter().find(|c| c.name == "greet").unwrap();
    assert_eq!(greet.description, None);
}

#[test]
fn with_neither_source_the_list_is_the_fallback() {
    let fallback = CommandList::fallback();
    assert_eq!(fallback.source(), ListSource::Fallback);
    assert_eq!(CommandList::from_history(&[]), fallback);
    let other = sequenced([StructuredEvent::ModelChanged { model: "m".into() }]);
    assert_eq!(CommandList::from_history(&other), fallback);
    for name in BUILT_INS {
        assert_eq!(kinds_of(&fallback, name), [CommandKind::BuiltIn], "{name}");
    }
    for entry in TERMINAL_ONLY {
        assert!(terminal_only(&fallback, entry.name), "{}", entry.name);
    }
    assert_eq!(kinds_of(&fallback, "model"), [CommandKind::App]);
}
