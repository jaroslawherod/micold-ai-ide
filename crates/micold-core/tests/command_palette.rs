//! #943 T013 — the command palette's render-free half (FR-007, FR-008, FR-008a; SC-002, SC-003;
//! research R10): when it is open, what it lists in which order, and what accepting fills in.

use micold_core::structured::commands::palette::{accept, filter, palette_query, PaletteSelection};
use micold_core::structured::commands::{CommandKind, CommandList};
use micold_core::structured::{parse_line, ReportedCommand, StructuredEvent};
use micold_core::typeahead::Direction;
use std::time::{Duration, Instant};

#[path = "support/stream_json.rs"]
mod stream_json;
use stream_json::{events_of, lines_of, sequenced};

fn reported(name: &str) -> ReportedCommand {
    ReportedCommand {
        name: name.into(),
        description: Some(format!("What {name} does")),
        argument_hint: None,
        aliases: vec![],
        builtin: false,
    }
}

/// A session list of `names` (custom commands) beside the terminal-only entries and `/model`.
fn list_of(names: &[&str]) -> CommandList {
    CommandList::from_history(&sequenced([StructuredEvent::CommandsReported {
        commands: names.iter().map(|n| reported(n)).collect(),
        models: vec![],
    }]))
}

fn names(list: &CommandList, query: &str) -> Vec<String> {
    filter(list, query)
        .into_iter()
        .map(|i| list.commands()[i].name.clone())
        .collect()
}

#[test]
fn the_palette_is_open_while_the_caret_is_in_a_leading_command_name() {
    assert_eq!(palette_query("/", 1), Some(""));
    assert_eq!(palette_query("/sp", 3), Some("sp"));
    assert_eq!(
        palette_query("/sp x", 3),
        Some("sp"),
        "caret at the name's end"
    );
    assert_eq!(
        palette_query("/speckit", 3),
        Some("sp"),
        "the name up to the caret"
    );
    for (draft, caret) in [
        ("x/sp", 4),
        ("/ sp", 4),
        ("/ ", 1),
        ("/sp x", 4),
        ("/sp x", 5),
        ("", 0),
        ("/sp", 0),
        ("/sp", 9),
        ("/é", 2),
    ] {
        assert_eq!(palette_query(draft, caret), None, "{draft:?} at {caret}");
    }
    assert_eq!(palette_query("/é", 3), Some("é"));
}

#[test]
fn matches_are_ordered_equal_prefix_word_prefix_then_the_rest() {
    let list = list_of(&[
        "wasp",
        "x_sp",
        "speckit-plan",
        "my-spec",
        "SPARK",
        "a:sp-thing",
        "sp",
        "spec",
        "unrelated",
    ]);
    assert_eq!(
        names(&list, "sp"),
        [
            "sp",
            "SPARK",
            "spec",
            "speckit-plan",
            "a:sp-thing",
            "my-spec",
            "x_sp",
            "wasp"
        ]
    );
    assert_eq!(names(&list, "SP")[0], "sp", "case is ignored");
    assert_eq!(names(&list, "zzz"), Vec::<String>::new());
}

#[test]
fn an_empty_query_lists_everything_grouped_by_kind_then_by_name() {
    let init = parse_line(
        r#"{"type":"system","subtype":"init","session_id":"s","skills":["zeta","alpha"]}"#,
    );
    let mut built_in = reported("compact");
    built_in.builtin = true;
    let history = sequenced([
        StructuredEvent::CommandsReported {
            commands: vec![
                reported("hello"),
                built_in,
                reported("zeta"),
                reported("bye"),
                reported("alpha"),
            ],
            models: vec![],
        },
        init,
    ]);
    let list = CommandList::from_history(&history);
    let all = filter(&list, "");
    assert_eq!(all.len(), list.commands().len(), "everything");
    let shown: Vec<(CommandKind, &str)> = all
        .iter()
        .map(|&i| (list.commands()[i].kind, list.commands()[i].name.as_str()))
        .collect();
    assert_eq!(
        &shown[..4],
        [
            (CommandKind::Skill, "alpha"),
            (CommandKind::Skill, "zeta"),
            (CommandKind::Custom, "bye"),
            (CommandKind::Custom, "hello"),
        ]
    );
    let kinds: Vec<CommandKind> = shown.iter().map(|(k, _)| *k).collect();
    let mut sorted = kinds.clone();
    sorted.sort();
    assert_eq!(kinds, sorted, "grouped Skill, Custom, BuiltIn, App");
    assert_eq!(shown.last(), Some(&(CommandKind::App, "model")));
}

/// SC-002 over one list: a name's shortest prefix no other name starts with lists it first, and
/// so does its full name.
fn assert_sc_002(list: &CommandList, what: &str) {
    let all: Vec<&str> = list.commands().iter().map(|c| c.name.as_str()).collect();
    for name in &all {
        let first = |query: &str| names(list, query).into_iter().next();
        assert_eq!(
            first(name).as_deref(),
            Some(*name),
            "{what}: full name {name}"
        );
        let lower = name.to_lowercase();
        let unique = (1..=name.len())
            .filter(|&n| name.is_char_boundary(n))
            .find(|&n| {
                let prefix = &lower[..n];
                all.iter()
                    .filter(|other| other.to_lowercase().starts_with(prefix))
                    .all(|other| other == name)
            });
        if let Some(n) = unique {
            assert_eq!(
                first(&name[..n]).as_deref(),
                Some(*name),
                "{what}: unique prefix {:?} of {name}",
                &name[..n]
            );
        }
    }
}

#[test]
fn every_recorded_command_is_first_for_its_unique_prefix_and_its_full_name() {
    let (commands, models) = micold_core::structured::commands::parse_initialize(
        &events_of(&lines_of("command-initialize"))
            .into_iter()
            .find_map(|e| match e.event {
                StructuredEvent::ControlResponse { outcome, .. } => Some(outcome),
                _ => None,
            })
            .expect("an initialize answer"),
    );
    let mut history = vec![StructuredEvent::CommandsReported { commands, models }];
    history.extend(
        events_of(&lines_of("command-initialize"))
            .into_iter()
            .map(|e| e.event),
    );
    assert_sc_002(
        &CommandList::from_history(&sequenced(history)),
        "initialize",
    );
    for name in ["command-initialize", "command-skill", "auth-failure"] {
        assert_sc_002(
            &CommandList::from_history(&events_of(&lines_of(name))),
            name,
        );
    }
}

#[test]
fn accepting_fills_the_command_and_a_space_with_the_caret_after_it() {
    assert_eq!(accept("greet"), ("/greet ".to_string(), 7));
    assert_eq!(
        accept("é"),
        ("/é ".to_string(), 4),
        "the caret is a byte offset"
    );
}

#[test]
fn the_selection_starts_on_the_first_match_and_moves_without_wrapping() {
    let mut selection = PaletteSelection::new(3);
    assert_eq!(selection.highlight(), Some(0));
    selection.step(Direction::Prev, 3);
    assert_eq!(selection.highlight(), Some(0));
    selection.step(Direction::Next, 3);
    selection.step(Direction::Next, 3);
    selection.step(Direction::Next, 3);
    assert_eq!(selection.highlight(), Some(2));
    selection.reset(5);
    assert_eq!(
        selection.highlight(),
        Some(0),
        "a new query selects the first match"
    );
    selection.reset(0);
    assert_eq!(selection.highlight(), None, "nothing to select");
    assert_eq!(PaletteSelection::new(0).highlight(), None);
}

#[test]
fn filtering_two_hundred_commands_stays_under_100_ms_every_time() {
    let names: Vec<String> = (0..200)
        .map(|i| match i % 4 {
            0 => format!("speckit-step-{i}"),
            1 => format!("plugin:skill-{i}"),
            2 => format!("custom_command_{i}"),
            _ => format!("tool{i}"),
        })
        .collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let list = list_of(&refs);
    let queries = [
        "", "s", "sp", "spec", "skill-1", "_1", "zz", "tool19", "PLUGIN",
    ];
    let mut slowest = Duration::ZERO;
    for i in 0..1_000 {
        let started = Instant::now();
        let shown = filter(&list, queries[i % queries.len()]);
        slowest = slowest.max(started.elapsed());
        std::hint::black_box(shown);
    }
    assert!(
        slowest < Duration::from_millis(100),
        "slowest filter {slowest:?} (SC-003)"
    );
}
