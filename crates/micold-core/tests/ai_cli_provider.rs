//! T063 (bugfix BUG-002) — the AI CLI provider abstraction (FR-024), and the regression gate on
//! not breaking provider one while adding provider two (feature 026, T006).
//!
//! Verifies `ClaudeProvider` consolidates every provider-specific detail: launch command + args,
//! conversation-transcript location, recorded-conversation detection, session-title extraction and
//! the durable archived marker. Pure + I/O-boundary behaviour is exercised headlessly (a real temp
//! transcript file — no `claude`, no GUI).
//!
//! # What feature 026 changed here, and what it must not have changed
//!
//! `transcript_path`, `transcript_dir`, `parse_title` and `archived_marker_path` are no longer on
//! the seam — they encode `claude`'s one-file-per-session-inside-a-per-cwd-directory layout, which
//! is exactly the assumption that made the trait un-substitutable, so they became private helpers
//! of this impl. `discover_transcript_session_ids` became the required `recorded_session_ids`.
//!
//! **None of that may change a single byte of what `claude` sees on disk**, and this file is what
//! says so. The path encoding is therefore still asserted byte-for-byte — from the outside, by
//! writing a transcript at the path the old implementation produced and requiring the provider to
//! find it. That is a stronger statement than calling a getter would be: it fails if the derivation
//! moves *in either direction*, and it keeps working when the helper is private.

use micold_core::provider::{ActivitySource, AiCliProvider, ClaudeProvider};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn fixed_id() -> Uuid {
    Uuid::parse_str("00000000-0000-4000-8000-000000000abc").unwrap()
}

/// The cwd every path assertion below uses.
const CWD: &str = "/home/u/proj/.claude/worktrees/feat-x";

/// `<config>/projects/<encoded-cwd>/`, spelled out here rather than asked of the provider.
///
/// This restates the layout on purpose: it is the *fixture*, not the code under test. If the
/// provider's own derivation drifts, these two stop agreeing and every path test below fails —
/// which is precisely the regression the reshape could have introduced silently.
fn expected_transcript_dir(config: &Path, cwd: &str) -> PathBuf {
    let encoded: String = cwd
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    // Sanity: no path separators survived inside the encoded segment.
    assert!(!encoded.contains('/'));
    config.join("projects").join(encoded)
}

/// Write `contents` where `claude` would have written this session's transcript.
fn write_transcript(config: &Path, cwd: &str, id: Uuid, contents: &str) -> PathBuf {
    let dir = expected_transcript_dir(config, cwd);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{id}.jsonl"));
    std::fs::write(&path, contents).unwrap();
    path
}

#[test]
fn identity_is_claude_code() {
    // Two registers, one provider (feature 026): `command()` is what a sidebar row and the terminal
    // bar carry, `display_name()` is what a menu or a failure message says. Asserted together so a
    // change that swaps them has to change this line.
    assert_eq!(ClaudeProvider.command(), "claude");
    assert_eq!(ClaudeProvider.display_name(), "Claude Code");
    assert_eq!(ClaudeProvider.id(), AiCli::ClaudeCode);
}

#[test]
fn launch_args_fresh_uses_session_id_flag() {
    let id = fixed_id();
    assert_eq!(
        ClaudeProvider.launch_args(id, LaunchMode::Fresh),
        vec!["--session-id".to_string(), id.to_string()]
    );
}

#[test]
fn launch_args_resume_uses_resume_flag() {
    let id = fixed_id();
    assert_eq!(
        ClaudeProvider.launch_args(id, LaunchMode::Resume),
        vec!["--resume".to_string(), id.to_string()],
        "two arguments, and `--resume` unglued from its value — Copilot's is `--resume=<id>`, and \
         the reshape must not have quietly harmonised them"
    );
}

#[test]
fn config_dir_comes_from_the_environment_override() {
    // `CLAUDE_CONFIG_DIR` is process-global, so this test owns it for its own duration. It is the
    // only test in this file that touches the environment; every other one is handed a directory.
    let previous = std::env::var("CLAUDE_CONFIG_DIR").ok();
    std::env::set_var("CLAUDE_CONFIG_DIR", "/somewhere/else");
    let overridden = ClaudeProvider.config_dir();
    // An empty value is "absent", not "the empty path" — the convention every provider follows.
    std::env::set_var("CLAUDE_CONFIG_DIR", "");
    let empty = ClaudeProvider.config_dir();
    match previous {
        Some(value) => std::env::set_var("CLAUDE_CONFIG_DIR", value),
        None => std::env::remove_var("CLAUDE_CONFIG_DIR"),
    }

    assert_eq!(overridden, Some(PathBuf::from("/somewhere/else")));
    assert_ne!(
        empty,
        Some(PathBuf::new()),
        "an empty override falls back to the home directory"
    );
}

#[test]
fn a_transcript_is_found_at_the_encoded_per_cwd_path() {
    // The byte-for-byte path assertion, made from the outside. Nothing here calls a path getter —
    // the transcript is placed at the address the pre-reshape implementation produced, and the
    // provider is required to find it there.
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        id,
        r#"{"type":"ai-title","aiTitle":"From disk"}"#,
    );

    assert!(ClaudeProvider.has_recorded_conversation(config.path(), Path::new(CWD), id));
    assert_eq!(
        ClaudeProvider.read_title(config.path(), Path::new(CWD), id),
        Some("From disk".to_string())
    );
    assert!(
        !ClaudeProvider.has_recorded_conversation(config.path(), Path::new("/another/place"), id),
        "the transcript is scoped to its working directory — a different cwd is a different \
         encoded segment and finds nothing"
    );
}

#[test]
fn the_latest_ai_title_record_wins() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        id,
        &[
            r#"{"type":"user","text":"hi"}"#,
            r#"{"type":"ai-title","aiTitle":"First title"}"#,
            r#"{"type":"assistant","text":"..."}"#,
            r#"{"type":"ai-title","aiTitle":"Second title"}"#,
        ]
        .join("\n"),
    );

    assert_eq!(
        ClaudeProvider.read_title(config.path(), Path::new(CWD), id),
        Some("Second title".to_string()),
        "the title grows and changes with the conversation, so the last record is the current one"
    );
}

#[test]
fn a_title_read_never_errors() {
    // Three ways to have no title, all of which must be `None` rather than a failure: no record at
    // all, an empty one, and lines that are not JSON. A conversation is not failed by a bad title.
    let config = tempfile::tempdir().unwrap();

    let no_record = Uuid::from_u128(1);
    write_transcript(
        config.path(),
        CWD,
        no_record,
        "{\"type\":\"user\",\"text\":\"hi\"}\n{\"type\":\"assistant\",\"text\":\"there\"}\n",
    );

    let empty_title = Uuid::from_u128(2);
    write_transcript(
        config.path(),
        CWD,
        empty_title,
        r#"{"type":"ai-title","aiTitle":""}"#,
    );

    let malformed = Uuid::from_u128(3);
    write_transcript(
        config.path(),
        CWD,
        malformed,
        "\nnot json at all\n{\"type\":\"ai-title\",\"aiTitle\":\"Good\"}\n{ broken json\n",
    );

    let missing = Uuid::from_u128(4);

    let read = |id| ClaudeProvider.read_title(config.path(), Path::new(CWD), id);
    assert_eq!(read(no_record), None);
    assert_eq!(read(empty_title), None, "an empty `aiTitle` is not a title");
    assert_eq!(
        read(malformed),
        Some("Good".to_string()),
        "blank and unparseable lines are skipped, not fatal"
    );
    assert_eq!(read(missing), None);
    assert!(!ClaudeProvider.has_recorded_conversation(config.path(), Path::new(CWD), missing));
}

#[test]
fn recorded_session_ids_lists_the_working_directorys_transcripts() {
    // What `discover_transcript_session_ids` used to do, under its required name. The behaviour is
    // unchanged: list the per-cwd directory, take each `*.jsonl` stem as an id.
    let config = tempfile::tempdir().unwrap();
    let one = Uuid::from_u128(1);
    let two = Uuid::from_u128(2);
    write_transcript(config.path(), CWD, one, "{}");
    write_transcript(config.path(), CWD, two, "{}");
    // Not a transcript: an unparseable stem, which contributes nothing rather than erroring.
    let dir = expected_transcript_dir(config.path(), CWD);
    std::fs::write(dir.join("not-a-uuid.jsonl"), "{}").unwrap();

    let mut found = ClaudeProvider.recorded_session_ids(config.path(), Path::new(CWD));
    found.sort();
    assert_eq!(found, vec![one, two]);

    assert!(
        ClaudeProvider
            .recorded_session_ids(config.path(), Path::new("/never/used"))
            .is_empty(),
        "a missing directory contributes nothing — never an error, so a project open cannot fail"
    );
}

// --- Bugfix BUG-003: durable close/remove suppression marker (T068) ---

#[test]
fn mark_archived_then_is_archived_reflects_it() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();

    assert!(
        !ClaudeProvider.is_archived(config.path(), Path::new(CWD), id),
        "no marker written yet"
    );

    ClaudeProvider
        .mark_archived(config.path(), Path::new(CWD), id)
        .unwrap();

    assert!(
        ClaudeProvider.is_archived(config.path(), Path::new(CWD), id),
        "marker written by mark_archived must be reflected by is_archived"
    );
    assert!(
        expected_transcript_dir(config.path(), CWD)
            .join(format!("{id}.archived"))
            .exists(),
        "and it is at the address the pre-reshape `archived_marker_path` produced — beside the \
         transcript, in `claude`'s own storage, so it survives the loss of our record"
    );
}

#[test]
fn archived_marker_is_never_discovered_as_a_transcript() {
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();

    // Only a marker exists for this id — no `.jsonl` transcript was ever written.
    ClaudeProvider
        .mark_archived(config.path(), Path::new(CWD), id)
        .unwrap();

    assert!(
        ClaudeProvider
            .recorded_session_ids(config.path(), Path::new(CWD))
            .is_empty(),
        "a `.archived` marker must never be misread as a `.jsonl` transcript"
    );
}

#[test]
fn activity_comes_from_the_hook_receiver_and_carries_no_path() {
    // Feature 010's mechanism, named rather than described. The variant is payload-free because
    // the settings file is *written* by the daemon from a port and a token chosen at runtime — no
    // pure `(config_dir, cwd, id)` derivation in this crate could produce it.
    let config = tempfile::tempdir().unwrap();
    assert_eq!(
        ClaudeProvider.activity_source(config.path(), Path::new(CWD), fixed_id()),
        ActivitySource::Hooks
    );
}

// ---------------------------------------------------------------------------------------
// T005 (feature 029) — a per-launch environment is a seam capability
// ---------------------------------------------------------------------------------------

#[test]
fn every_provider_answers_what_a_launch_needs_in_the_environment() {
    // Pi's offline posture (research R9) is three environment variables set per launch. The
    // daemon builds a session's environment from one shared derivation with no per-provider slot,
    // so before this method the only way to set them was to ask "is this session Pi?" at spawn —
    // the conditional on which CLI a session runs that FR-019/FR-020 forbid by name.
    //
    // Asking every provider instead costs the two existing ones one honest empty answer each,
    // which is the assertion below. It is worth writing down rather than leaving implied: an
    // implementation that returned `claude`'s or Copilot's own variables here would change what
    // those two CLIs see in their environment, and this feature must not.
    for which in [AiCli::ClaudeCode, AiCli::Copilot] {
        assert!(
            which.provider().launch_env().is_empty(),
            "{which:?} asks for nothing in the environment and must keep asking for nothing"
        );
    }
}

#[test]
fn the_launch_environment_is_a_property_of_the_provider_not_of_a_session() {
    // No session id, no working directory, no mode: the answer is the same for every launch of
    // this CLI. That is what lets the daemon merge it in one place for all three providers.
    let port: &dyn AiCliProvider = &ClaudeProvider;
    assert_eq!(port.launch_env(), Vec::new());
}

// ---------------------------------------------------------------------------------------
// Feature 032 (T019) — the first-turn label, read from the transcript at its derived path
// ---------------------------------------------------------------------------------------

/// One `claude` user record whose content is `text`, `\n`-terminated.
fn typed(text: &str) -> String {
    format!(
        "{}\n",
        serde_json::json!({"type": "user", "message": {"role": "user", "content": text}})
    )
}

#[test]
fn the_label_is_the_shaped_first_turn_of_the_sessions_own_transcript() {
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        fixed_id(),
        &format!(
            "{}{}",
            typed("Fix the\n\n  flaky login test"),
            typed("and then this")
        ),
    );
    assert_eq!(
        ClaudeProvider
            .read_label(config.path(), Path::new(CWD), fixed_id())
            .as_deref(),
        Some("Fix the flaky login test"),
        "the first typed turn, as one line (FR-002, FR-003)"
    );
}

#[test]
fn a_missing_transcript_has_no_label_and_does_not_fail() {
    let config = tempfile::tempdir().unwrap();
    assert_eq!(
        ClaudeProvider.read_label(config.path(), Path::new(CWD), fixed_id()),
        None
    );
}

#[test]
fn a_first_turn_past_the_read_bound_yields_no_label_even_with_a_later_prompt() {
    // One record longer than the whole 1 MiB bound comes first; the typed prompt starts after it.
    // Reading on to find it would cost the whole file, and the bound is what FR-014 asks for
    // (C2.4).
    let config = tempfile::tempdir().unwrap();
    let padding = "x".repeat(micold_core::first_turn::LABEL_BUDGET_BYTES as usize);
    let huge = format!(
        "{}\n",
        serde_json::json!({"type": "assistant", "message": {"role": "assistant", "content": padding}})
    );
    write_transcript(
        config.path(),
        CWD,
        fixed_id(),
        &format!("{huge}{}", typed("A prompt past the bound")),
    );
    assert_eq!(
        ClaudeProvider.read_label(config.path(), Path::new(CWD), fixed_id()),
        None
    );
}

#[test]
fn the_title_and_the_label_are_read_from_the_same_file_and_never_mixed() {
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        fixed_id(),
        &format!(
            "{}{{\"type\":\"ai-title\",\"aiTitle\":\"First title\"}}\n{}{{\"type\":\"ai-title\",\"aiTitle\":\"Latest title\"}}\n",
            typed("The first typed turn"),
            typed("A second turn")
        ),
    );
    assert_eq!(
        ClaudeProvider
            .read_title(config.path(), Path::new(CWD), fixed_id())
            .as_deref(),
        Some("Latest title"),
        "`read_title` is unchanged: the latest `ai-title` (C1.4)"
    );
    assert_eq!(
        ClaudeProvider
            .read_label(config.path(), Path::new(CWD), fixed_id())
            .as_deref(),
        Some("The first typed turn"),
        "and `read_label` never answers a title (C1.4)"
    );
}

/// The three name records `claude` re-emits together on every turn after a `/rename`, in the order
/// the real transcript writes them: `custom-title`, then the **pre-rename** `ai-title`, then
/// `agent-name` (BUG-002 *What `claude` writes after a `/rename`*, `c4cc3dca…` lines 12058-12060,
/// 12080-12082).
fn renamed_turn() -> String {
    [
        r#"{"type":"custom-title","customTitle":"Windows package"}"#,
        r#"{"type":"ai-title","aiTitle":"PR 284 macOS feature"}"#,
        r#"{"type":"agent-name","agentName":"Windows package"}"#,
        "",
    ]
    .join("\n")
}

#[test]
fn a_rename_is_the_current_name_not_the_ai_title_it_replaced() {
    // BUG-002's regression gate (T031). `claude`'s `/rename` writes a `custom-title` and keeps
    // re-emitting the `ai-title` it replaced on every later turn, so "the latest record written" is
    // the *old* name. The rename is the conversation's current name and must win (FR-005, C16.1).
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        id,
        &format!(
            "{}{}{}{}",
            typed("Package it for Windows"),
            renamed_turn(),
            typed("And a second turn after the rename"),
            renamed_turn(),
        ),
    );

    assert_eq!(
        ClaudeProvider.read_title(config.path(), Path::new(CWD), id),
        Some("Windows package".to_string()),
        "the user's `/rename` is the conversation's current name; the pre-rename `ai-title` beside \
         it is stale by construction and must not come back (C16.1)"
    );
}

#[test]
fn an_agent_name_is_a_name_record_and_not_a_kind_to_ignore() {
    // `agent-name` is `claude`'s resolved display name and one of the three name records C16.1
    // resolves. The fixture gives it *alone*, with no `ai-title` to fall back on, on purpose: on
    // every real transcript observed for BUG-002 the two carry the same string, so a fixture with
    // both cannot tell a read that honours `agent-name` from one that ignores it.
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        id,
        &[
            r#"{"type":"user","text":"hi"}"#,
            r#"{"type":"agent-name","agentName":"Token usage optimization"}"#,
            "",
        ]
        .join("\n"),
    );

    assert_eq!(
        ClaudeProvider.read_title(config.path(), Path::new(CWD), id),
        Some("Token usage optimization".to_string()),
        "`agent-name` is a name record like the others, not a record kind to ignore (C16.1)"
    );
}

#[test]
fn an_ai_title_written_after_the_last_agent_name_wins_by_position() {
    // The reason `agent-name` is *not* ranked above `ai-title` by kind: a later `claude` that
    // re-titles a conversation without re-emitting `agent-name` must still read the new title. A
    // kind ranking would pin the first `agent-name` for ever, which is BUG-002 again (C16.1, D4).
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        id,
        &[
            r#"{"type":"ai-title","aiTitle":"An early guess"}"#,
            r#"{"type":"agent-name","agentName":"An early guess"}"#,
            r#"{"type":"ai-title","aiTitle":"What it turned out to be"}"#,
            "",
        ]
        .join("\n"),
    );

    assert_eq!(
        ClaudeProvider.read_title(config.path(), Path::new(CWD), id),
        Some("What it turned out to be".to_string()),
        "with neither kind renamed, the latest of `agent-name` and `ai-title` by position is the \
         current name (C16.1)"
    );
}

#[test]
fn the_latest_custom_title_wins_however_early_it_sits() {
    // Only `custom-title` ranks above position, and it does so from wherever it sits: the rename is
    // sticky in `claude`'s records, so no number of later `ai-title`/`agent-name` lines unseats it.
    // Two renames, and the later one is the name (C16.1).
    let id = fixed_id();
    let config = tempfile::tempdir().unwrap();
    write_transcript(
        config.path(),
        CWD,
        id,
        &[
            r#"{"type":"custom-title","customTitle":"First choice"}"#,
            r#"{"type":"custom-title","customTitle":"What I settled on"}"#,
            r#"{"type":"ai-title","aiTitle":"The CLI's own guess"}"#,
            r#"{"type":"agent-name","agentName":"What I settled on"}"#,
            r#"{"type":"ai-title","aiTitle":"A later guess still"}"#,
            "",
        ]
        .join("\n"),
    );

    assert_eq!(
        ClaudeProvider.read_title(config.path(), Path::new(CWD), id),
        Some("What I settled on".to_string()),
        "the latest `custom-title` outranks every later `ai-title`, and an earlier rename does not \
         outrank a later one (C16.1)"
    );
}

#[test]
fn an_empty_name_in_any_record_kind_is_not_a_name() {
    // C17: an empty value is a missing name, not a name — in all three kinds. Each falls through to
    // whatever the remaining records say, and a transcript of nothing but empties reads `None`.
    let config = tempfile::tempdir().unwrap();

    let empty_custom = Uuid::from_u128(11);
    write_transcript(
        config.path(),
        CWD,
        empty_custom,
        &[
            r#"{"type":"custom-title","customTitle":""}"#,
            r#"{"type":"ai-title","aiTitle":"The CLI's name"}"#,
            "",
        ]
        .join("\n"),
    );

    let empty_agent = Uuid::from_u128(12);
    write_transcript(
        config.path(),
        CWD,
        empty_agent,
        &[
            r#"{"type":"ai-title","aiTitle":"The CLI's name"}"#,
            r#"{"type":"agent-name","agentName":""}"#,
            "",
        ]
        .join("\n"),
    );

    let all_empty = Uuid::from_u128(13);
    write_transcript(
        config.path(),
        CWD,
        all_empty,
        &[
            r#"{"type":"custom-title","customTitle":""}"#,
            r#"{"type":"ai-title","aiTitle":""}"#,
            r#"{"type":"agent-name","agentName":""}"#,
            "",
        ]
        .join("\n"),
    );

    let unparsable_beside_a_rename = Uuid::from_u128(14);
    write_transcript(
        config.path(),
        CWD,
        unparsable_beside_a_rename,
        &[
            "not json at all",
            r#"{"type":"custom-title","customTitle":"Survives the noise"}"#,
            "{ broken json",
            r#"{"type":"ai-title","aiTitle":"The replaced name"}"#,
            "",
        ]
        .join("\n"),
    );

    let read = |id| ClaudeProvider.read_title(config.path(), Path::new(CWD), id);
    assert_eq!(
        read(empty_custom),
        Some("The CLI's name".to_string()),
        "an empty `customTitle` is not a rename, so it does not mask the CLI's own name"
    );
    assert_eq!(
        read(empty_agent),
        Some("The CLI's name".to_string()),
        "an empty `agentName` is not a name, so the positional read falls through to the `ai-title`"
    );
    assert_eq!(read(all_empty), None, "nothing but empty values is no name");
    assert_eq!(
        read(unparsable_beside_a_rename),
        Some("Survives the noise".to_string()),
        "a read still never errors on unparsable lines, and still finds the rename (C17)"
    );
}
