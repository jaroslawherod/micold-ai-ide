//! `PiProvider` — the Pi Coding Agent profile of the AI CLI seam (feature 029, T012–T015).
//!
//! The third provider, and the first the seam was not designed around. Every derivation here is
//! pure, so the whole provider is testable **without `pi` installed** — the same property
//! `copilot_provider.rs` relies on, and the reason CI on three platforms can hold this file to the
//! contract without a Node runtime anywhere near it. The corpus comes from `support::pi_home()`
//! rather than a real `~/.pi/agent`; a test that read the developer's own store would be a defect
//! even on the runs where it passed.
//!
//! Contract: `specs/029-pi-cli-provider/contracts/pi-cli.md`.

mod support;

use micold_core::provider::{AiCliProvider, PiProvider};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::path::PathBuf;
use support::{fixture_id, pi_home, FIXTURE_SESSION_A, FIXTURE_SESSION_B};
use uuid::Uuid;

fn fixed_id() -> Uuid {
    Uuid::parse_str("44444444-5555-4555-8555-666666666666").unwrap()
}

// ---------------------------------------------------------------------------------------
// T012 — identity and the launch vector
// ---------------------------------------------------------------------------------------

#[test]
fn identity_is_the_pi_coding_agent() {
    // Both registers side by side, as for the other two: `pi` labels a sidebar row and the
    // terminal bar, "Pi Coding Agent" fills a menu entry and a failure sentence. Letting either
    // leak into the other is the likeliest drift in this seam, so they are pinned together.
    assert_eq!(PiProvider.command(), "pi");
    assert_eq!(PiProvider.display_name(), "Pi Coding Agent");
    assert_eq!(PiProvider.id(), AiCli::Pi);
}

#[test]
fn a_fresh_launch_names_the_id_we_chose() {
    let id = fixed_id();
    assert_eq!(
        PiProvider.launch_args(id, LaunchMode::Fresh),
        vec!["--session-id".to_string(), id.to_string()],
        "the application owns the id; `pi --session-id` creates it when it is missing"
    );
}

#[test]
fn resuming_is_the_same_vector_as_starting() {
    // The fact that made FR-005b's correspondence store unnecessary, and the first time two launch
    // modes agree. `--session-id` is documented as *"Use exact project session ID, creating it if
    // missing"*, so one vector covers both: Pi looks the id up among this cwd's conversations,
    // opens it if found, and otherwise starts a new one carrying it.
    //
    // Asserting the equality rather than the two literals is the point. A future change that gave
    // resume its own flag would have to come here and say so, instead of quietly diverging in a
    // place only a live `pi` would catch.
    let id = fixed_id();
    assert_eq!(
        PiProvider.launch_args(id, LaunchMode::Resume),
        PiProvider.launch_args(id, LaunchMode::Fresh),
        "one argument vector for both modes (research R1, FR-005a)"
    );
}

#[test]
fn no_launch_writes_an_identity_into_the_users_conversation_name() {
    // `--name`/`-n` would put a UUID on every row this application started, and the name is what
    // the sidebar reads. It is the user's, or Pi's, and never ours (FR-005a, FR-011).
    //
    // The same assertion covers the rest of the "Not used" list, which is why it reads over the
    // whole vector rather than for one flag: `--no-session` would keep the conversation out of
    // Pi's store and break resuming it with a bare `pi`; `--session-dir` would relocate the store
    // FR-005a says stays where Pi put it; `--session` resolves an existing file and cannot create,
    // so it would need the mode-dependent vector the test above just refused.
    for mode in [LaunchMode::Fresh, LaunchMode::Resume] {
        let args = PiProvider.launch_args(fixed_id(), mode);
        for forbidden in [
            "--name",
            "-n",
            "--no-session",
            "--session-dir",
            "--session",
            "--fork",
            "--continue",
            "-p",
            "--print",
        ] {
            assert!(
                !args.iter().any(|arg| arg == forbidden),
                "{mode:?} passed {forbidden}, which the contract's \"Not used\" list excludes: {args:?}"
            );
        }
    }
}

#[test]
fn the_id_is_a_shape_pi_will_accept() {
    // Pi validates the id against `^[A-Za-z0-9](?:[A-Za-z0-9._-]*[A-Za-z0-9])?$` and refuses to
    // start on anything else. A hyphenated UUID passes, but that is a fact about *both* sides, and
    // an application that generated ids some other way would find out at the user's terminal
    // rather than here.
    let id = fixed_id().to_string();
    let accepted = |c: char| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-');
    assert!(id.chars().all(accepted), "unexpected character in {id}");
    assert!(id.starts_with(|c: char| c.is_ascii_alphanumeric()));
    assert!(id.ends_with(|c: char| c.is_ascii_alphanumeric()));
}

// ---------------------------------------------------------------------------------------
// T013 — the base directory
// ---------------------------------------------------------------------------------------

#[test]
fn config_dir_resolves_the_environment_override() {
    let home = pi_home();
    assert_eq!(
        PiProvider.config_dir(),
        Some(home.path().to_path_buf()),
        "`PI_CODING_AGENT_DIR` relocates the store, which is what makes every test here safe"
    );
}

#[test]
fn an_empty_override_is_absent_not_the_empty_path() {
    // The convention both existing providers follow, held for the third so they cannot drift. An
    // empty `PI_CODING_AGENT_DIR` means "unset", not "the store is at `/`".
    let _home = pi_home();
    let previous = std::env::var("PI_CODING_AGENT_DIR").ok();
    std::env::set_var("PI_CODING_AGENT_DIR", "");
    let resolved = PiProvider.config_dir();
    match previous {
        Some(value) => std::env::set_var("PI_CODING_AGENT_DIR", value),
        None => std::env::remove_var("PI_CODING_AGENT_DIR"),
    }

    assert_ne!(resolved, Some(PathBuf::new()));
    if let Some(path) = resolved {
        // `ends_with` on a `Path` compares whole components, so this is `.pi/agent` and not a
        // string suffix that `x.pi/agent` would also satisfy. No platform branch: Pi is a
        // JavaScript bundle whose default is `homedir()` joined with `.pi/agent` everywhere,
        // Windows included, so one assertion is the honest one on all three (Principle VI).
        assert!(
            path.ends_with(PathBuf::from(".pi").join("agent")),
            "the fallback is `~/.pi/agent`, got {}",
            path.display()
        );
    }
    // The `None` arm — an unresolvable home directory — is "uncertain", not "absent". It cannot be
    // provoked portably from a test (it needs `UserDirs::new()` to fail), so it is asserted where
    // it is reachable, through a fake: the client's boot prune and the daemon's set-wide decisions
    // both drive it.
}

#[test]
fn the_store_is_never_relocated_by_pis_own_session_dir_variable() {
    // `PI_CODING_AGENT_SESSION_DIR` and `--session-dir` split the session store off from the
    // config directory. Honouring the read side is a recorded open item, not an accident, so this
    // pins today's answer: the variable is not consulted, and setting it changes nothing here.
    let home = pi_home();
    let previous = std::env::var("PI_CODING_AGENT_SESSION_DIR").ok();
    std::env::set_var("PI_CODING_AGENT_SESSION_DIR", "/somewhere/else");
    let resolved = PiProvider.config_dir();
    match previous {
        Some(value) => std::env::set_var("PI_CODING_AGENT_SESSION_DIR", value),
        None => std::env::remove_var("PI_CODING_AGENT_SESSION_DIR"),
    }
    assert_eq!(resolved, Some(home.path().to_path_buf()));
}

// ---------------------------------------------------------------------------------------
// T014 — recorded-conversation detection
// ---------------------------------------------------------------------------------------

#[test]
fn a_conversation_is_recorded_exactly_when_its_file_is_in_the_cwds_directory() {
    let cwd = PathBuf::from("/fixture/worktree");
    let recorded = fixture_id(FIXTURE_SESSION_A);
    let never_started = fixture_id(FIXTURE_SESSION_B);
    let home = pi_home().with_conversation(&cwd, recorded, "conversation-named.jsonl");

    assert!(
        PiProvider.has_recorded_conversation(home.path(), &cwd, recorded),
        "Pi writes the file with its header at session creation, so there is no \
         started-but-not-yet-recorded window to be generous about"
    );
    assert!(
        !PiProvider.has_recorded_conversation(home.path(), &cwd, never_started),
        "an id nothing recorded is absent, not merely unread"
    );
}

#[test]
fn the_same_id_under_a_different_working_directory_is_a_different_conversation() {
    // Pi's store is keyed on the cwd, which is what makes worktree scoping Pi's own arithmetic
    // rather than something this application layers on (Principle III).
    let here = PathBuf::from("/fixture/worktree");
    let elsewhere = PathBuf::from("/fixture/other-worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home().with_conversation(&here, id, "conversation-named.jsonl");

    assert!(PiProvider.has_recorded_conversation(home.path(), &here, id));
    assert!(
        !PiProvider.has_recorded_conversation(home.path(), &elsewhere, id),
        "the same id in another location is another conversation, or none"
    );
}

#[test]
fn the_timestamp_in_the_filename_is_not_something_the_reader_has_to_know() {
    // The id is the part after the *first* underscore, and the timestamp Pi writes carries its own
    // `-` separators and no underscore. A reader that reconstructed the filename would have to
    // know when the conversation was created, which it never does.
    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home().with_conversation_at(
        &cwd,
        "2019-01-02T03-04-05-006Z",
        id,
        "conversation-named.jsonl",
    );
    assert!(PiProvider.has_recorded_conversation(home.path(), &cwd, id));
}

#[test]
fn an_absent_or_unreadable_store_answers_false_and_never_fails() {
    // Two shapes of "nothing here", both of which a project open must survive: the store has never
    // been written, and the path where it would be is occupied by something that is not a
    // directory. Neither is an error — a Pi session's absence cannot fail a project open for the
    // other two CLIs' sessions (Edge Cases).
    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);

    // Scoped: each `PiHome` holds the process-wide environment lock, which is not re-entrant, so
    // the second would wait on the first forever.
    {
        let empty = pi_home();
        assert!(!PiProvider.has_recorded_conversation(empty.path(), &cwd, id));
    }

    let blocked = pi_home();
    let dir = blocked.sessions_dir(&cwd);
    std::fs::create_dir_all(dir.parent().expect("sessions root")).expect("sessions root");
    std::fs::write(&dir, "not a directory").expect("occupy the path");
    assert!(
        !PiProvider.has_recorded_conversation(blocked.path(), &cwd, id),
        "an unreadable store contributes nothing rather than propagating an error"
    );
}

// ---------------------------------------------------------------------------------------
// T015 — the launch environment
// ---------------------------------------------------------------------------------------

#[test]
fn a_launch_disables_pis_own_calls_home() {
    // Principle IV, by the same judgement that made `--no-remote` deliberate for Copilot. Pi's
    // documented startup does an update check, a `pi.dev` version request and install telemetry;
    // a session this application started on the user's behalf does none of the three.
    //
    // Model traffic is untouched and deliberately so: that is the user's own configured provider
    // and the reason they started the session.
    let env: Vec<(String, String)> = PiProvider.launch_env();
    for (key, value) in [
        ("PI_OFFLINE", "1"),
        ("PI_SKIP_VERSION_CHECK", "1"),
        ("PI_TELEMETRY", "0"),
    ] {
        assert!(
            env.iter().any(|(k, v)| k == key && v == value),
            "{key}={value} is missing from {env:?}"
        );
    }
}

#[test]
fn the_launch_environment_is_the_same_for_every_session() {
    // It takes no id, no working directory and no mode, so the daemon merges one answer for every
    // launch of this CLI in one place. Asking twice and getting the same answer is what lets the
    // spawn site stay free of a `match` on which CLI a session runs (FR-020).
    assert_eq!(PiProvider.launch_env(), PiProvider.launch_env());
}

#[test]
fn nothing_in_the_launch_environment_relocates_the_users_store() {
    // FR-007: a launch must not modify or redirect the user's own Pi configuration. The three
    // variables above are per-launch posture; anything that pointed Pi at another directory would
    // make a conversation this application started unreachable from the user's own `pi`.
    for (key, value) in PiProvider.launch_env() {
        assert!(
            !matches!(
                key.as_str(),
                "PI_CODING_AGENT_DIR" | "PI_CODING_AGENT_SESSION_DIR" | "HOME" | "USERPROFILE"
            ),
            "{key}={value} would move the store out from under the user's own `pi`"
        );
    }
}

// ---------------------------------------------------------------------------------------
// T034 — the conversation's own label, from a bounded prefix
// ---------------------------------------------------------------------------------------

/// The label for one fixture conversation, materialised under a fresh id.
fn title_of(fixture: &str) -> Option<String> {
    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home().with_conversation(&cwd, id, fixture);
    PiProvider.read_title(home.path(), &cwd, id)
}

#[test]
fn the_latest_name_in_the_prefix_is_the_label() {
    // Pi's own rule is "latest `session_info` wins"; two names in one file must not read as the
    // first, which is what a reader that stopped at its first match would show.
    assert_eq!(
        title_of("conversation-named.jsonl").as_deref(),
        Some("Rename the encoder and its callers")
    );
}

#[test]
fn an_unnamed_conversation_is_labelled_by_its_first_user_message() {
    // The same fallback Pi's own session picker uses, so a row reads what `pi --resume` would show.
    // This fixture's `content` is a bare string rather than a parts array; both shapes occur.
    assert_eq!(
        title_of("conversation-unnamed.jsonl").as_deref(),
        Some("Why does the sidebar show two rows for one worktree?")
    );
}

#[test]
fn a_name_past_the_bound_falls_back_rather_than_paying_for_the_whole_file() {
    // FR-011 and SC-006b: labelling costs the same for a conversation a minute old and a year old.
    // The name here is ~256 KiB in; the row reads the first user message instead, and does not fail.
    assert_eq!(
        title_of("conversation-named-past-bound.jsonl").as_deref(),
        Some("Explain the provider seam")
    );
}

#[test]
fn a_conversation_nobody_has_spoken_in_yet_has_no_label() {
    // `None` is what the caller renders as the neutral placeholder — never an empty string, which
    // would render as a blank row.
    assert_eq!(title_of("conversation-header-only.jsonl"), None);
}

#[test]
fn junk_lines_and_empty_names_are_skipped_not_fatal() {
    // A non-JSON line, a blank line, an off-contract type, and names that are empty or whitespace.
    // None of it is an error, and none of it is a label.
    assert_eq!(
        title_of("conversation-unparseable-lines.jsonl").as_deref(),
        Some("Where does the encoder live?")
    );
}

#[test]
fn a_line_still_being_written_is_not_read_as_a_name() {
    // The file ends mid-`session_info`, the way one being appended to while it is read does. The
    // incomplete line is dropped, so the row does not flicker to a half-written name.
    assert_eq!(
        title_of("conversation-truncated.jsonl").as_deref(),
        Some("Summarise the release notes")
    );
}

#[test]
fn a_missing_conversation_has_no_label_and_does_not_fail() {
    let cwd = PathBuf::from("/fixture/worktree");
    let home = pi_home();
    assert_eq!(
        PiProvider.read_title(home.path(), &cwd, fixture_id(FIXTURE_SESSION_A)),
        None
    );
}

// ---------------------------------------------------------------------------------------
// T035 — where activity is reported
// ---------------------------------------------------------------------------------------

#[test]
fn activity_arrives_through_a_component_log_outside_the_session_store() {
    use micold_core::provider::ActivitySource;

    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home();
    let source = PiProvider.activity_source(home.path(), &cwd, id);
    assert_eq!(
        source,
        ActivitySource::Extension {
            log: home
                .path()
                .join("micold-activity")
                .join(format!("{id}.jsonl")),
        },
        "Pi reports busy/idle only to code loaded into it, so the source names the log that code \
         writes — beside `sessions/`, never inside it"
    );
    let ActivitySource::Extension { log } = source else {
        unreachable!()
    };
    assert!(
        !log.starts_with(home.path().join("sessions")),
        "a log inside `sessions/` could be listed as a conversation, by us or by Pi"
    );
}

#[test]
fn the_activity_source_does_not_depend_on_whether_the_component_is_enabled() {
    // The FR-012e switch is honoured at spawn, by the daemon. The provider is pure: it has no way
    // to see the setting, so it answers the same whether or not the log will ever be written — and
    // the same whether or not it exists yet. There is one code path, not a second for "off".
    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home();
    let before = PiProvider.activity_source(home.path(), &cwd, id);
    let log = home
        .path()
        .join("micold-activity")
        .join(format!("{id}.jsonl"));
    std::fs::create_dir_all(log.parent().unwrap()).unwrap();
    std::fs::write(
        &log,
        "{\"type\":\"turn_start\",\"at\":\"2026-09-12T08:15:04Z\"}\n",
    )
    .unwrap();
    assert_eq!(PiProvider.activity_source(home.path(), &cwd, id), before);
}

// ---------------------------------------------------------------------------------------
// T049 — discovering conversations the application did not start
// ---------------------------------------------------------------------------------------

#[test]
fn every_conversation_for_a_location_is_listed_with_no_cap() {
    // FR-015: no cap by count, no cut-off by age. Two hundred is well past any page size a lister
    // might be tempted to adopt, and the timestamps span whatever `with_conversations` gives them.
    let cwd = PathBuf::from("/fixture/worktree");
    let ids: Vec<Uuid> = (1..=200u128)
        .map(|n| Uuid::from_u128(0x0290_0000 + n))
        .collect();
    let home = pi_home().with_conversations(&cwd, &ids, "conversation-header-only.jsonl");

    let mut listed = PiProvider.recorded_session_ids(home.path(), &cwd);
    listed.sort();
    let mut expected = ids.clone();
    expected.sort();
    assert_eq!(listed, expected);
}

#[test]
fn a_pi_generated_id_is_listed_like_one_this_application_chose() {
    // Pi's own conversations carry UUIDv7 ids; ours carry v4. Both are one conversation each.
    let cwd = PathBuf::from("/fixture/worktree");
    let ours = fixture_id(FIXTURE_SESSION_A);
    let pis = Uuid::parse_str("01936f7e-2c4a-7b1d-9e3f-5a6b7c8d9e0f").unwrap();
    let home = pi_home()
        .with_conversation(&cwd, ours, "conversation-named.jsonl")
        .with_conversation_at(
            &cwd,
            "2026-09-12T09-00-00-000Z",
            pis,
            "conversation-unnamed.jsonl",
        );

    let mut listed = PiProvider.recorded_session_ids(home.path(), &cwd);
    listed.sort();
    let mut expected = vec![ours, pis];
    expected.sort();
    assert_eq!(listed, expected);
}

#[test]
fn a_forked_conversation_is_listed_without_reading_its_parentage() {
    // A fork is an ordinary file whose header names a parent. Discovery opens no file, so the
    // header cannot matter — this pins that a parent that no longer exists changes nothing.
    let cwd = PathBuf::from("/fixture/worktree");
    let fork = fixture_id(FIXTURE_SESSION_B);
    let home = pi_home();
    let path = support::pi_conversation_path(home.path(), &cwd, "2026-09-12T10-00-00-000Z", fork);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        format!(
            "{{\"type\":\"session\",\"version\":3,\"id\":\"{fork}\",\"timestamp\":\"2026-09-12T10:00:00.000Z\",\"cwd\":\"{}\",\"parentSession\":\"/gone/2026-09-11T00-00-00-000Z_{}.jsonl\"}}\n",
            cwd.display(),
            fixture_id(FIXTURE_SESSION_A)
        ),
    )
    .unwrap();

    assert_eq!(
        PiProvider.recorded_session_ids(home.path(), &cwd),
        vec![fork]
    );
}

#[test]
fn only_conversation_files_are_listed() {
    // Our own `.archived` markers live in the same directory, and neither they nor anything else
    // that is not a `<timestamp>_<id>.jsonl` may come back as a session.
    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home().with_conversation(&cwd, id, "conversation-named.jsonl");
    let dir = home.sessions_dir(&cwd);
    std::fs::write(
        dir.join(format!("{}.archived", fixture_id(FIXTURE_SESSION_B))),
        "",
    )
    .unwrap();
    std::fs::write(dir.join("notes.txt"), "not a conversation").unwrap();
    std::fs::write(dir.join("no-underscore.jsonl"), "{}\n").unwrap();
    std::fs::write(
        dir.join("2026-09-12T08-15-04-123Z_not-a-uuid.jsonl"),
        "{}\n",
    )
    .unwrap();
    std::fs::create_dir_all(dir.join(format!(
        "2026-09-12T08-15-04-123Z_{}.jsonl",
        fixture_id(FIXTURE_SESSION_B)
    )))
    .ok();

    let listed = PiProvider.recorded_session_ids(home.path(), &cwd);
    assert!(listed.contains(&id));
    assert!(
        !listed.contains(&fixture_id(FIXTURE_SESSION_B)),
        "a marker, or a directory wearing a conversation's name, is not a conversation: {listed:?}"
    );
    assert_eq!(listed.len(), 1, "{listed:?}");
}

#[test]
fn a_location_with_no_store_lists_nothing_and_does_not_fail() {
    let home = pi_home();
    assert!(PiProvider
        .recorded_session_ids(home.path(), &PathBuf::from("/never/used"))
        .is_empty());
}

// ---------------------------------------------------------------------------------------
// T050 — a closed session stays closed, and nothing of the user's is removed
// ---------------------------------------------------------------------------------------

#[test]
fn closing_writes_a_marker_beside_the_conversation_and_nothing_else() {
    let cwd = PathBuf::from("/fixture/worktree");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home().with_conversation(&cwd, id, "conversation-named.jsonl");
    let conversation = home.conversation_path(&cwd, id).unwrap();
    let before = std::fs::read(&conversation).unwrap();

    assert!(!PiProvider.is_archived(home.path(), &cwd, id));
    PiProvider.mark_archived(home.path(), &cwd, id).unwrap();
    assert!(PiProvider.is_archived(home.path(), &cwd, id));

    let marker = home.sessions_dir(&cwd).join(format!("{id}.archived"));
    assert_eq!(
        std::fs::read(&marker).unwrap(),
        Vec::<u8>::new(),
        "the marker is empty"
    );
    assert_eq!(
        std::fs::read(&conversation).unwrap(),
        before,
        "the store is shared with the user's own `pi`; closing a row here is not permission to \
         remove or truncate their history"
    );
    assert!(
        PiProvider.has_recorded_conversation(home.path(), &cwd, id),
        "and the conversation is still resumable by a bare `pi`"
    );
}

#[test]
fn a_marker_is_scoped_to_its_location_and_its_id() {
    let here = PathBuf::from("/fixture/worktree");
    let elsewhere = PathBuf::from("/fixture/other-worktree");
    let closed = fixture_id(FIXTURE_SESSION_A);
    let open = fixture_id(FIXTURE_SESSION_B);
    let home = pi_home();

    PiProvider
        .mark_archived(home.path(), &here, closed)
        .unwrap();
    assert!(PiProvider.is_archived(home.path(), &here, closed));
    assert!(!PiProvider.is_archived(home.path(), &here, open));
    assert!(!PiProvider.is_archived(home.path(), &elsewhere, closed));
}

#[test]
fn a_marker_can_be_written_for_a_location_pi_never_wrote_to() {
    // Best-effort, and it creates its own directory: a session closed before its first launch
    // must still stay closed.
    let cwd = PathBuf::from("/fixture/fresh");
    let id = fixture_id(FIXTURE_SESSION_A);
    let home = pi_home();
    PiProvider.mark_archived(home.path(), &cwd, id).unwrap();
    assert!(PiProvider.is_archived(home.path(), &cwd, id));
}

// ---------------------------------------------------------------------------------------
// Feature 029 (persistent session names), FR-004 — the name inside pi's terminal title
// ---------------------------------------------------------------------------------------

#[test]
fn only_the_name_inside_pis_terminal_title_is_a_name() {
    // `updateTerminalTitle`: `π - <folder>` while the conversation is unnamed, `π - <name> -
    // <folder>` once named. The folder is the basename of the directory pi runs in.
    let cwd = PathBuf::from("/fixture/.claude/worktrees/proj");
    let name = |title: &str| PiProvider.name_in_terminal_title(title, &cwd);

    assert_eq!(name("π - proj"), None, "an unnamed conversation");
    assert_eq!(
        name("π - Fixing the parser - proj"),
        Some("Fixing the parser".into())
    );
    assert_eq!(
        name("π - a - b - proj"),
        Some("a - b".into()),
        "a name may itself contain the separator"
    );
    assert_eq!(
        name("π - Fixing the parser - elsewhere"),
        None,
        "another folder's title"
    );
    assert_eq!(
        name("Some extension's title"),
        None,
        "a title pi did not shape"
    );
}
