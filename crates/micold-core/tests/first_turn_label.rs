//! A session's label from the first thing the user typed in it (feature 032 — contract
//! `specs/032-untitled-session-labels/contracts/first-turn-label.md`, C2–C5).
//!
//! Three layers, tested separately so a failure says which one broke:
//!
//! - **the bounded prefix** (C2): how much of a record file is read, and which lines of it count;
//! - **shaping** (C5): one line, at most 80 user-perceived characters, cut with `…`;
//! - **the provider's turn rule** (C3 for `claude`): which record is the first *typed* turn, as
//!   opposed to text the CLI or the application inserted.

use std::path::{Path, PathBuf};

use micold_core::first_turn::{read_prefix, shape_label, LABEL_BUDGET_BYTES};

/// The longest label, in extended grapheme clusters (FR-003).
const MAX_GRAPHEMES: usize = 80;

// ---------------------------------------------------------------------------------------
// C5 — shaping
// ---------------------------------------------------------------------------------------

#[test]
fn whitespace_and_line_break_runs_collapse_to_one_space_and_ends_are_trimmed() {
    assert_eq!(
        shape_label("  Fix the\n\n  flaky \t login\r\ntest  ").as_deref(),
        Some("Fix the flaky login test"),
        "a row is one line: every whitespace run, line breaks included, is one space (C5.1)"
    );
}

#[test]
fn whitespace_only_text_yields_no_label() {
    assert_eq!(
        shape_label(" \n\t \u{3000} "),
        None,
        "an empty turn is no label source, so the next turn is looked at instead (C5.2)"
    );
}

#[test]
fn exactly_eighty_graphemes_pass_unchanged() {
    let text = "a".repeat(MAX_GRAPHEMES);
    assert_eq!(
        shape_label(&text).as_deref(),
        Some(text.as_str()),
        "the bound is inclusive: 80 characters fit (C5.3)"
    );
}

#[test]
fn eighty_one_graphemes_yield_the_first_seventy_nine_and_an_ellipsis() {
    let text = format!("{}XY", "a".repeat(MAX_GRAPHEMES - 1));
    let expected = format!("{}…", "a".repeat(MAX_GRAPHEMES - 1));
    assert_eq!(
        shape_label(&text).as_deref(),
        Some(expected.as_str()),
        "one over the bound is cut to 79 plus `…`, 80 in total, so the reader sees it was cut (C5.4)"
    );
}

#[test]
fn the_cut_never_splits_a_grapheme_cluster() {
    // Each of these is ONE user-perceived character made of several code points. Placed as the 79th
    // character of an 81-character line, a cut by `char` would keep only part of it.
    let family = "👨\u{200d}👩\u{200d}👧"; // ZWJ sequence
    let accented = "e\u{0301}"; // base + combining acute
    let cjk = "漢";
    for cluster in [family, accented, cjk] {
        let text = format!("{}{cluster}Z!", "a".repeat(MAX_GRAPHEMES - 2));
        let expected = format!("{}{cluster}…", "a".repeat(MAX_GRAPHEMES - 2));
        assert_eq!(
            shape_label(&text).as_deref(),
            Some(expected.as_str()),
            "{cluster:?} is kept whole at the cut (C5.5)"
        );
    }
}

// ---------------------------------------------------------------------------------------
// C2 — the bounded prefix
// ---------------------------------------------------------------------------------------

fn write_file(dir: &Path, name: &str, contents: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn the_prefix_is_at_most_one_mebibyte() {
    assert_eq!(
        LABEL_BUDGET_BYTES,
        1024 * 1024,
        "the FR-014 bound (research R7)"
    );
    let dir = tempfile::tempdir().unwrap();
    // Short complete lines, one line past the bound: every byte of it would be kept by a read with
    // no bound, so only the bound can stop at exactly 1 MiB.
    let line = b"0123456789abcde\n"; // 16 bytes, and 16 divides the bound
    let contents = line.repeat((LABEL_BUDGET_BYTES as usize) / line.len() + 1);
    assert_eq!(contents.len() as u64, LABEL_BUDGET_BYTES + 16);
    let path = write_file(dir.path(), "big.jsonl", &contents);

    let prefix = read_prefix(&path).expect("a readable file has a prefix");
    assert_eq!(
        prefix.len() as u64,
        LABEL_BUDGET_BYTES,
        "a file past the bound is read up to the bound and no further (C2.1)"
    );
}

#[test]
fn a_trailing_line_without_a_newline_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let path = write_file(dir.path(), "half.jsonl", b"{\"complete\":1}\n{\"being_writ");
    assert_eq!(
        read_prefix(&path).as_deref(),
        Some(&b"{\"complete\":1}\n"[..]),
        "a line still being written is not read — only lines ending in `\\n` count (C2.2)"
    );
}

#[test]
fn a_missing_file_has_no_prefix() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(
        read_prefix(&dir.path().join("never-written.jsonl")),
        None,
        "a missing record is `None`, never an error (FR-011)"
    );
}

// ---------------------------------------------------------------------------------------
// C3 — `claude`'s first typed turn
// ---------------------------------------------------------------------------------------

use micold_core::first_turn::claude_first_turn;

/// A synthetic `claude` transcript from `tests/fixtures/first_turn/claude/`.
fn claude_fixture(name: &str) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/first_turn/claude")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()))
}

fn claude_label(fixture: &str) -> Option<String> {
    claude_first_turn(&claude_fixture(fixture))
}

/// A transcript from whole records, one per line, each `\n`-terminated.
fn transcript(records: &[serde_json::Value]) -> Vec<u8> {
    records
        .iter()
        .flat_map(|record| format!("{record}\n").into_bytes())
        .collect()
}

/// A `claude` user record whose `message.content` is `content`.
fn user(content: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"type": "user", "message": {"role": "user", "content": content}})
}

/// What the user typed after whatever a test puts first.
const THE_PROMPT: &str = "The prompt the user typed";

#[test]
fn a_bare_prompt_sending_command_is_labelled_with_its_name() {
    // The reported case: `/speckit-autopilot` with no arguments, then `claude`'s `isMeta` expansion
    // of the skill (C3.5b). The name, slash included, is what the user typed (C3.5c).
    assert_eq!(
        claude_label("bare_skill.jsonl").as_deref(),
        Some("/speckit-autopilot")
    );
}

#[test]
fn a_command_with_arguments_is_labelled_with_its_arguments() {
    assert_eq!(
        claude_label("skill_with_args.jsonl").as_deref(),
        Some("the name of past session is still not shown"),
        "the arguments are what the user asked for; the name is only how (C3.5c)"
    );
}

#[test]
fn a_command_claude_answered_itself_is_skipped_for_the_next_prompt() {
    // `/model` is followed by a user `<local-command-stdout>`: `claude` handled it and sent no
    // prompt, so it is not a turn (C3.5a, D5).
    assert_eq!(
        claude_label("model_then_prompt.jsonl").as_deref(),
        Some("Why does the sidebar read New session?")
    );
}

#[test]
fn a_command_answered_by_a_system_record_is_skipped_for_the_next_prompt() {
    // `/reload-plugins` answers in a `system` / `local_command` record rather than a user one
    // (C3.5a).
    assert_eq!(
        claude_label("reload_plugins_system.jsonl").as_deref(),
        Some("Tidy the release notes")
    );
}

#[test]
fn a_last_line_command_that_opens_with_its_message_is_a_turn() {
    // A running transcript whose last record is the command: no follower yet, so the tag order
    // decides. `<command-message>` first is how a prompt-sending command is written (C3.5b′).
    assert_eq!(
        claude_label("last_line_command_message.jsonl").as_deref(),
        Some("/speckit-plan")
    );
}

#[test]
fn a_last_line_command_that_opens_with_its_name_is_not_a_turn() {
    // `<command-name>` first is how `claude` writes a command it handles itself (C3.5b′).
    assert_eq!(claude_label("last_line_command_name.jsonl"), None);
}

#[test]
fn records_claude_or_a_tool_wrote_are_never_turns() {
    // C3.2, C3.3: meta, compact summaries, tool results in either shape.
    let inserted = [
        serde_json::json!({"type": "user", "isMeta": true,
            "message": {"role": "user", "content": "An expanded skill"}}),
        serde_json::json!({"type": "user", "isCompactSummary": true,
            "message": {"role": "user", "content": "This session is being continued"}}),
        serde_json::json!({"type": "user", "toolUseResult": {"stdout": "out"},
            "message": {"role": "user", "content": "A tool's output"}}),
        user(serde_json::json!([{"type": "tool_result", "tool_use_id": "t1", "content": "out"}])),
    ];
    for record in inserted {
        assert_eq!(
            claude_first_turn(&transcript(&[record.clone(), user(THE_PROMPT.into())])).as_deref(),
            Some(THE_PROMPT),
            "{record} is not something the user typed"
        );
    }
}

#[test]
fn text_claude_or_the_application_inserted_is_never_a_turn() {
    // C3.4: command output, shell-mode input and output, and injected notices.
    for text in [
        "<local-command-caveat>Caveat: generated by local commands</local-command-caveat>",
        "<local-command-stdout>Compacted</local-command-stdout>",
        "<bash-input>ls</bash-input>",
        "<bash-stdout>Cargo.toml</bash-stdout>",
        "<task-notification><status>completed</status></task-notification>",
        "  <system-reminder>The date changed.</system-reminder>",
    ] {
        assert_eq!(
            claude_first_turn(&transcript(&[user(text.into()), user(THE_PROMPT.into())]))
                .as_deref(),
            Some(THE_PROMPT),
            "{text:?} is not something the user typed"
        );
    }
}

#[test]
fn a_list_contents_text_parts_form_the_label_and_an_image_adds_nothing() {
    assert_eq!(
        claude_label("image_prompt.jsonl").as_deref(),
        Some("[Image #1] what is wrong with this row?"),
        "the text part is the turn; the image part contributes no characters (C3.3)"
    );
}

#[test]
fn a_whitespace_only_prompt_is_skipped_for_the_next_turn() {
    assert_eq!(
        claude_label("whitespace_then_prompt.jsonl").as_deref(),
        Some("The real first turn"),
        "an empty turn is not the label; the next non-empty one is (C3.7)"
    );
}

#[test]
fn a_line_that_is_not_json_is_skipped() {
    assert_eq!(
        claude_label("non_json_then_prompt.jsonl").as_deref(),
        Some("Typed after the junk"),
        "an unparsable line is skipped, never an error (C2.3)"
    );
}

#[test]
fn a_command_record_with_no_parsable_name_is_not_a_turn() {
    let orphan = user("<command-message>orphan</command-message>".into());
    assert_eq!(
        claude_first_turn(&transcript(&[orphan, user(THE_PROMPT.into())])).as_deref(),
        Some(THE_PROMPT),
        "a command record without `<command-name>` cannot be labelled by its name (C3.5d)"
    );
}

#[test]
fn a_conversation_with_nothing_typed_in_it_has_no_label() {
    // Every record here was written by `claude`, a tool, or the application (FR-004): the row
    // still reads "New session".
    assert_eq!(claude_label("injected_only.jsonl"), None);
}

#[test]
fn a_last_record_still_being_written_is_not_read() {
    // The only typed prompt is on a final line with no `\n` (C2.2).
    assert_eq!(claude_label("truncated_last_line.jsonl"), None);
}

// ---------------------------------------------------------------------------------------
// C4 — Copilot's first typed turn
//
// Copilot's records are flatter than `claude`'s: one `user.message` record per turn, with no
// command wrappers to interpret. What has to be recognised instead is the text Copilot itself put
// into the conversation — skill context, instruction discovery, autopilot continuations — which it
// marks in the record rather than in the text.
// ---------------------------------------------------------------------------------------

use micold_core::first_turn::copilot_first_turn;

/// A synthetic Copilot event log from `tests/fixtures/first_turn/copilot/`.
fn copilot_label(fixture: &str) -> Option<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/first_turn/copilot")
        .join(fixture);
    let bytes =
        std::fs::read(&path).unwrap_or_else(|err| panic!("fixture {}: {err}", path.display()));
    copilot_first_turn(&bytes)
}

#[test]
fn the_first_user_message_copilot_did_not_source_itself_is_the_label() {
    assert_eq!(
        copilot_label("plain_first_turn.jsonl").as_deref(),
        Some("Add the login page"),
        "the first turn, not the second, and not the `system.message` before it (C4.1, C4.3)"
    );
}

#[test]
fn a_user_message_copilot_sourced_itself_is_never_a_turn() {
    // 12 of the 972 `user.message` records surveyed carry a `source` — skill context and
    // instruction discovery. Copilot wrote them, so they are not what the user typed (C4.1).
    assert_eq!(
        copilot_label("sourced_then_prompt.jsonl").as_deref(),
        Some("Explain the release workflow"),
        "a `data.source` marks a record Copilot inserted; an explicit null does not (C4.1)"
    );
}

#[test]
fn an_autopilot_continuation_is_never_a_turn() {
    // 28 of the surveyed records are continuations with empty `content`: Copilot prompting itself
    // to keep going, not the user typing (C4.1).
    assert_eq!(
        copilot_label("autopilot_continuation_then_prompt.jsonl").as_deref(),
        Some("Rename the worktree column")
    );
}

#[test]
fn the_label_is_the_recorded_content_never_the_transformed_one() {
    // `transformedContent` is what Copilot sends the model: the same text wrapped in a datetime
    // header and system reminders. Reading it would put `<current_datetime>` on the row (C4.2).
    let label = copilot_label("plain_first_turn.jsonl").expect("the fixture has a first turn");
    assert!(
        !label.contains("current_datetime"),
        "the row shows what the user typed, not what was sent to the model (C4.2): {label:?}"
    );
    assert_eq!(label, "Add the login page");
}

#[test]
fn a_whitespace_only_turn_is_skipped_for_the_next_one() {
    assert_eq!(
        copilot_label("whitespace_then_prompt.jsonl").as_deref(),
        Some("The real first turn"),
        "an empty turn is no label source; the next non-empty one is (C4.3, C5.2)"
    );
}

#[test]
fn a_fleet_command_is_kept_exactly_as_copilot_recorded_it() {
    // Copilot records the text *after* a slash command, and `/fleet` with its own prefix. There is
    // no command syntax left in the record to strip, so none is stripped (C4.4).
    assert_eq!(
        copilot_label("fleet_command.jsonl").as_deref(),
        Some("Fleet deployed: audit the settings screen")
    );
}

#[test]
fn a_first_turn_at_the_tenth_record_is_still_found() {
    // The survey put the first qualifying turn at record 2–10 in all 44 sessions (FR-014, SC-009).
    // This fixture is the worst of them: nine records Copilot wrote, then the turn.
    assert_eq!(
        copilot_label("first_turn_at_record_ten.jsonl").as_deref(),
        Some("The tenth record is the first turn")
    );
}

#[test]
fn a_copilot_log_with_nothing_typed_in_it_has_no_label() {
    let sourced_only = b"{\"type\":\"session.start\",\"data\":{}}\n\
        {\"type\":\"user.message\",\"data\":{\"content\":\"skill context\",\"source\":\"skill-context\"}}\n";
    assert_eq!(
        copilot_first_turn(sourced_only),
        None,
        "nothing the user typed is nothing to show but \"New session\" (FR-004)"
    );
}
