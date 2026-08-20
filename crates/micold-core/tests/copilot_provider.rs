//! `CopilotProvider` — the GitHub Copilot profile of the AI CLI seam (feature 026, T017/T017a/T018).
//!
//! Every path derivation here is pure, so the whole provider is testable **without `copilot`
//! installed**. That property is load-bearing for CI on all three platforms, and it is why every
//! test in this file reads the captured corpus through `support::copilot_home()` rather than a real
//! `~/.copilot`. A test that read the developer's own store would be a defect even on the runs
//! where it passed.
//!
//! Contract: `specs/026-multi-provider-sessions/contracts/copilot-cli.md`.

mod support;

use micold_core::protocol::hashing::sha256_hex;
use micold_core::provider::{AiCliProvider, CopilotProvider};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use support::{copilot_home, fixture_id, FIXTURE_SESSION_A};
use uuid::Uuid;

fn fixed_id() -> Uuid {
    Uuid::parse_str("11111111-2222-4222-8222-333333333333").unwrap()
}

// ---------------------------------------------------------------------------------------
// T017 — launch
// ---------------------------------------------------------------------------------------

#[test]
fn identity_is_github_copilot() {
    // The two registers again: `copilot` labels a row, "GitHub Copilot" fills a menu entry. Both
    // strings hang off this one type, which is exactly why they are asserted side by side.
    assert_eq!(CopilotProvider.command(), "copilot");
    assert_eq!(CopilotProvider.display_name(), "GitHub Copilot");
    assert_eq!(CopilotProvider.id(), AiCli::Copilot);
}

#[test]
fn a_fresh_launch_names_the_id_we_chose_and_refuses_remote_access() {
    let id = fixed_id();
    assert_eq!(
        CopilotProvider.launch_args(id, LaunchMode::Fresh),
        vec![
            "--session-id".to_string(),
            id.to_string(),
            "--no-remote".to_string(),
        ],
        "the application owns the id, and `--no-remote` is on every launch"
    );
}

#[test]
fn a_resume_targets_one_conversation_and_glues_the_id_to_the_flag() {
    let id = fixed_id();
    assert_eq!(
        CopilotProvider.launch_args(id, LaunchMode::Resume),
        vec![format!("--resume={id}"), "--no-remote".to_string()],
        "`--resume=<uuid>` as one argument: a bare `--resume` opens Copilot's interactive picker, \
         and this application always targets a specific conversation"
    );
}

#[test]
fn no_launch_ever_hands_copilot_blanket_permission() {
    // The session is interactive. The user answers Copilot's permission prompts in its own
    // terminal, exactly as they answer `claude`'s — so these flags must never appear, on either
    // mode. Stated as a negative because that is how it would regress: someone adds
    // `--allow-all-tools` to quiet a prompt in a probe and it survives into the launch path.
    for mode in [LaunchMode::Fresh, LaunchMode::Resume] {
        let args = CopilotProvider.launch_args(fixed_id(), mode);
        for forbidden in ["--allow-all-tools", "--allow-all", "--allow-all-paths"] {
            assert!(
                !args.iter().any(|arg| arg.starts_with(forbidden)),
                "{mode:?} passed {forbidden}: {args:?}"
            );
        }
    }
}

#[test]
fn every_launch_refuses_remote_steering() {
    // Principle IV, and the one flag in this feature that is a safety decision rather than a
    // mechanism: without `--no-remote` Copilot logs `Remote session access enabled` and a session
    // this application spawned on the user's behalf becomes remotely steerable.
    for mode in [LaunchMode::Fresh, LaunchMode::Resume] {
        assert!(
            CopilotProvider
                .launch_args(fixed_id(), mode)
                .iter()
                .any(|arg| arg == "--no-remote"),
            "{mode:?} launched without --no-remote"
        );
    }
}

// ---------------------------------------------------------------------------------------
// T017a — FR-011: a launch modifies no user configuration
// ---------------------------------------------------------------------------------------

/// Every file under `root`, keyed by its path relative to `root`, with contents.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if let Ok(bytes) = std::fs::read(&path) {
                out.insert(path.strip_prefix(root).unwrap().to_path_buf(), bytes);
            }
        }
    }
    out
}

#[test]
fn preparing_a_launch_writes_nothing_into_copilots_configuration() {
    // FR-011. Every per-launch need is an *argument*, not a file: no `config.json` write, no
    // `trustedFolders` edit, no user-level state touched. The provider has no verb that could do
    // otherwise, and this is what keeps it that way — the obvious "fix" for Copilot's trust prompt
    // is to append the worktree to `trustedFolders`, which is exactly what FR-011 forbids.
    let home = copilot_home().with_corpus(Path::new("/fixture/worktree"));
    std::fs::write(home.path().join("config.json"), "{\"trustedFolders\":[]}").unwrap();
    let before = snapshot(home.path());

    let id = fixed_id();
    let _fresh = CopilotProvider.launch_args(id, LaunchMode::Fresh);
    let _resume = CopilotProvider.launch_args(id, LaunchMode::Resume);
    let _config = CopilotProvider.config_dir();
    let _recorded =
        CopilotProvider.recorded_session_ids(home.path(), Path::new("/fixture/worktree"));
    let _has = CopilotProvider.has_recorded_conversation(
        home.path(),
        Path::new("/fixture/worktree"),
        fixture_id(FIXTURE_SESSION_A),
    );
    let _title =
        CopilotProvider.read_title(home.path(), Path::new("/fixture/worktree"), fixed_id());

    assert_eq!(
        snapshot(home.path()),
        before,
        "preparing and reading around a launch left Copilot's store byte-identical"
    );
}

#[test]
fn the_only_thing_this_application_ever_writes_there_is_its_own_sentinel() {
    // The single exception FR-011 allows, and it is app-owned: `micold.archived`. Asserting the
    // *difference* rather than "nothing changed" is the point — it names exactly what we add.
    let home = copilot_home().with_corpus(Path::new("/fixture/worktree"));
    let before = snapshot(home.path());

    let id = fixture_id(FIXTURE_SESSION_A);
    CopilotProvider
        .mark_archived(home.path(), Path::new("/fixture/worktree"), id)
        .unwrap();

    let after = snapshot(home.path());
    let added: Vec<&PathBuf> = after.keys().filter(|k| !before.contains_key(*k)).collect();
    assert_eq!(
        added,
        vec![&PathBuf::from("session-state")
            .join(id.to_string())
            .join("micold.archived")],
        "exactly one file appeared, and it is ours"
    );
    for (path, bytes) in &before {
        assert_eq!(
            after.get(path),
            Some(bytes),
            "{} changed, and nothing of Copilot's may",
            path.display()
        );
    }
    assert!(
        after[added[0]].is_empty(),
        "the sentinel is present/absent and has no shape beyond that"
    );
}

// ---------------------------------------------------------------------------------------
// T018 — the base directory
// ---------------------------------------------------------------------------------------

#[test]
fn config_dir_resolves_the_environment_override() {
    let home = copilot_home();
    assert_eq!(
        CopilotProvider.config_dir(),
        Some(home.path().to_path_buf()),
        "`COPILOT_HOME` relocates the whole store, which is what makes every test here safe"
    );
}

#[test]
fn an_empty_override_is_absent_not_the_empty_path() {
    // The convention `ClaudeProvider` already follows, held for the second provider so the two
    // cannot drift. An empty `COPILOT_HOME` means "unset", not "the store is at `/`".
    let _home = copilot_home();
    let previous = std::env::var("COPILOT_HOME").ok();
    std::env::set_var("COPILOT_HOME", "");
    let resolved = CopilotProvider.config_dir();
    match previous {
        Some(value) => std::env::set_var("COPILOT_HOME", value),
        None => std::env::remove_var("COPILOT_HOME"),
    }

    assert_ne!(resolved, Some(PathBuf::new()));
    assert_ne!(
        resolved,
        Some(PathBuf::from("")),
        "an empty value falls back to the home directory rather than becoming a path"
    );
    if let Some(path) = resolved {
        assert!(
            path.ends_with(".copilot"),
            "the fallback is `~/.copilot`, got {}",
            path.display()
        );
    }
    // The `None` arm — an unresolvable home directory — is "uncertain", not "absent". It cannot be
    // provoked portably from a test (it needs `UserDirs::new()` to fail), so it is asserted where
    // it is reachable: `boot_keeps_every_session_when_the_provider_cannot_locate_its_config` and
    // the daemon's `set_wide_provider_decisions` drive that arm through a fake.
}

// ---------------------------------------------------------------------------------------
// T003 — the recorded hash vector
// ---------------------------------------------------------------------------------------

#[test]
fn the_index_filename_is_the_hash_copilot_itself_wrote() {
    // Not a self-consistency check: this vector was produced by Copilot 1.0.80, which named the
    // index file for a session started in `/tmp/copilot-hash-probe`. It is recorded in
    // `tests/fixtures/copilot/README.md` and cross-referenced from the contract.
    //
    // Without it, a change to `sha256_hex` would leave every derivation agreeing with itself and
    // silently orphan every session Copilot has recorded — the application would look in a file
    // that does not exist and report a working directory as having no conversations.
    assert_eq!(
        sha256_hex(b"/tmp/copilot-hash-probe"),
        "75980abda9809593b6cc1c6005b85aca235c3f973c6afac4f9a2ea707710dd98"
    );
}

// ---------------------------------------------------------------------------------------
// T038/T039 — the per-working-directory index
// ---------------------------------------------------------------------------------------

#[test]
fn the_index_is_read_in_the_order_copilot_wrote_it() {
    // T038. Order matters because it is Copilot's own — the ids come back as listed, not sorted,
    // so a reader that "tidied" them would be reporting something the file does not say. The
    // fixture lists A, C, B deliberately, so "in order" is a real assertion rather than one that
    // happens to hold.
    let cwd = Path::new("/fixture/worktree");
    let home = copilot_home().with_index(cwd, "index-well-formed.json");

    assert_eq!(
        CopilotProvider.recorded_session_ids(home.path(), cwd),
        vec![
            fixture_id(FIXTURE_SESSION_A),
            fixture_id(support::FIXTURE_SESSION_C),
            fixture_id(support::FIXTURE_SESSION_B),
        ]
    );
}

#[test]
fn an_index_this_application_cannot_read_contributes_nothing_and_never_an_error() {
    // T038's other half, and the rule the whole seam runs on: everything read in another vendor's
    // store is best-effort. Four ways to be unreadable, one answer to all of them.
    //
    // `schemaVersion: 2` is the interesting one. It is *parseable* — the file is valid JSON with a
    // plausible `sessionIds` array — and it is still refused, because the version is Copilot's
    // statement about its own format and reading a shape we have not seen is guessing.
    for (fixture, why) in [
        ("index-schema-version-2.json", "a version we do not know"),
        ("index-truncated.json", "cut off mid-array"),
        ("index-empty.json", "zero bytes"),
    ] {
        let cwd = Path::new("/fixture/worktree");
        let home = copilot_home().with_index(cwd, fixture);
        assert_eq!(
            CopilotProvider.recorded_session_ids(home.path(), cwd),
            Vec::<Uuid>::new(),
            "{fixture}: {why}"
        );
    }

    // And the file that is not there at all — the ordinary case for a directory Copilot has never
    // been run in.
    let home = copilot_home();
    assert_eq!(
        CopilotProvider.recorded_session_ids(home.path(), Path::new("/never/used")),
        Vec::<Uuid>::new()
    );
}

#[test]
fn the_index_path_is_derived_purely_from_the_working_directory() {
    // T039. No I/O in the derivation, which is what makes the whole provider testable without
    // `copilot` installed — and the property `ClaudeProvider` already has.
    //
    // Asserted from the outside: an index written at `sha256_hex(cwd)` is found, and the same
    // index is invisible from any other working directory. Nothing calls a path getter, so the
    // derivation stays private and the assertion still fails if it moves.
    let here = Path::new("/fixture/worktree");
    let elsewhere = Path::new("/fixture/other-worktree");
    let home = copilot_home().with_index(here, "index-well-formed.json");

    assert_eq!(
        CopilotProvider
            .recorded_session_ids(home.path(), here)
            .len(),
        3
    );
    assert!(
        CopilotProvider
            .recorded_session_ids(home.path(), elsewhere)
            .is_empty(),
        "conversations are scoped to the directory they were had in"
    );

    // The filename itself, against the hash the workspace's own helper computes — the link between
    // `sha256_hex` and this derivation, which the recorded vector above then ties to Copilot.
    let expected = home.path().join("sidebar-sessions-state").join(format!(
        "{}.json",
        sha256_hex(here.to_string_lossy().as_bytes())
    ));
    assert!(expected.exists(), "{} was not written", expected.display());
}

// ---------------------------------------------------------------------------------------
// T040/T041 — recorded conversations and the durable marker
// ---------------------------------------------------------------------------------------

#[test]
fn a_conversation_is_recorded_exactly_when_the_event_log_exists() {
    // T040. `events.jsonl` is created lazily on the **first user message**, not at session start,
    // so its existence is precisely "this session was used". A session directory without one was
    // opened and never used — which is exactly what the prune paths need to tell apart, and
    // fixture C is that session.
    let cwd = Path::new("/fixture/worktree");
    let home = copilot_home().with_corpus(cwd);

    assert!(CopilotProvider.has_recorded_conversation(
        home.path(),
        cwd,
        fixture_id(FIXTURE_SESSION_A)
    ));
    assert!(
        !CopilotProvider.has_recorded_conversation(
            home.path(),
            cwd,
            fixture_id(support::FIXTURE_SESSION_C)
        ),
        "a session directory with a `workspace.yaml` and no `events.jsonl` is an unused session, \
         not a conversation"
    );
    assert!(
        !CopilotProvider.has_recorded_conversation(home.path(), cwd, Uuid::from_u128(0xAB5E7)),
        "and an id Copilot has never seen has nothing recorded"
    );
}

#[test]
fn the_archived_marker_is_written_read_and_never_fatal() {
    // T041, FR-015. Our sentinel, in Copilot's storage, so it outlives the loss of our own store.
    let cwd = Path::new("/fixture/worktree");
    let home = copilot_home().with_corpus(cwd);
    let id = fixture_id(FIXTURE_SESSION_A);

    assert!(!CopilotProvider.is_archived(home.path(), cwd, id));
    CopilotProvider.mark_archived(home.path(), cwd, id).unwrap();
    assert!(CopilotProvider.is_archived(home.path(), cwd, id));

    // A session directory Copilot has not created yet: the marker's parent is made on the way, so
    // closing a session the application knows about but Copilot does not is not a failure.
    let unknown = Uuid::from_u128(0x9999);
    CopilotProvider
        .mark_archived(home.path(), cwd, unknown)
        .expect("the marker's directory is created if absent");
    assert!(CopilotProvider.is_archived(home.path(), cwd, unknown));

    // And a marker is never mistaken for a session. Copilot's discovery reads the *index*, not a
    // directory listing, so the sentinel can live inside the session directory — the
    // `.jsonl`-extension filter `claude` needs has no counterpart to need here.
    assert!(
        !CopilotProvider
            .recorded_session_ids(home.path(), cwd)
            .contains(&unknown),
        "writing a marker did not invent a session"
    );
}

#[test]
fn a_failed_marker_write_is_swallowed_by_the_caller_not_thrown_at_the_user() {
    // FR-015's "best-effort" clause, asserted at the boundary where it is decidable. The provider
    // returns `io::Result` — it reports what happened rather than deciding what to do about it —
    // and every caller in the workspace logs and continues. `catalog.rs::mark_archived_durable` is
    // the production one, and it is a `tracing::warn!` with no `?`.
    //
    // What this can hold here is the shape: the failure arrives as an `Err`, not a panic, so a
    // caller *can* swallow it.
    let home = copilot_home();
    let unwritable = home.path().join("session-state");
    std::fs::create_dir_all(&unwritable).unwrap();
    // A file where the session directory should be: creating the directory fails.
    std::fs::write(unwritable.join("00000000-0000-4000-8000-000000000001"), "").unwrap();

    let result = CopilotProvider.mark_archived(
        home.path(),
        Path::new("/fixture/worktree"),
        Uuid::parse_str("00000000-0000-4000-8000-000000000001").unwrap(),
    );
    assert!(result.is_err(), "the failure is reported, not hidden");
}

// ---------------------------------------------------------------------------------------
// T052/T053 — the title, and where activity comes from
// ---------------------------------------------------------------------------------------

#[test]
fn a_title_is_read_from_the_sessions_own_workspace_file() {
    // T052, FR-017. Three forms, and the quoted-with-a-colon one is the reason this is not a
    // `split(':')`: `name: 'Reply with the single word: hello'` is a title Copilot actually wrote,
    // and a naive reader truncates it to `'Reply with the single word` while passing every test
    // whose fixture has no colon in it.
    let cwd = Path::new("/fixture/worktree");
    let home = copilot_home().with_corpus(cwd);

    let title = |id: &str| CopilotProvider.read_title(home.path(), cwd, fixture_id(id));
    assert_eq!(
        title(FIXTURE_SESSION_A),
        Some("Fixture session A".to_string()),
        "the plain form"
    );
    assert_eq!(
        title(support::FIXTURE_SESSION_B),
        Some("Reply with the single word: hello".to_string()),
        "single-quoted, with a colon inside"
    );
    assert_eq!(
        title(support::FIXTURE_SESSION_D),
        Some("Double quoted: with a colon".to_string()),
        "double-quoted"
    );
}

#[test]
fn no_title_is_none_and_never_an_error() {
    // The label stays `Pending`, which is a state the sidebar already renders. Two ways to have no
    // title — Copilot has not summarised the conversation yet, or the file is not there at all —
    // and neither may fail the session (FR-017).
    let cwd = Path::new("/fixture/worktree");
    let home = copilot_home().with_corpus(cwd);

    assert_eq!(
        CopilotProvider.read_title(home.path(), cwd, fixture_id(support::FIXTURE_SESSION_C)),
        None,
        "a session Copilot has not summarised has no `name:` key yet"
    );
    assert_eq!(
        CopilotProvider.read_title(home.path(), cwd, Uuid::from_u128(0x404)),
        None,
        "and a session directory that does not exist reads as no title, not as a failure"
    );

    // An unreadable file: present, but not something the reader can make sense of.
    let broken = Uuid::from_u128(0x1234);
    let dir = home.path().join("session-state").join(broken.to_string());
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("workspace.yaml"),
        "\u{0}\u{1}\u{2}not: yaml: at: all\n",
    )
    .unwrap();
    assert_eq!(CopilotProvider.read_title(home.path(), cwd, broken), None);
}

#[test]
fn activity_comes_from_the_sessions_own_event_log() {
    // T053. `EventLog { path }` carries a path because it *is* arithmetic; `ClaudeProvider`'s
    // `Hooks` carries none because the equivalent is a runtime secret — the daemon writes that
    // settings file itself, embedding a port chosen at daemon start and a per-session bearer
    // token, so no pure `(config_dir, cwd, id)` derivation in this crate could produce it.
    //
    // The asymmetry is asserted deliberately, because it is the part a reader will want to tidy
    // into one uniform shape. A variant that looked uniform at the cost of being unimplementable
    // would be worse than one that admits the two mechanisms differ.
    let cwd = Path::new("/fixture/worktree");
    let home = copilot_home().with_corpus(cwd);
    let id = fixture_id(FIXTURE_SESSION_A);

    assert_eq!(
        CopilotProvider.activity_source(home.path(), cwd, id),
        micold_core::provider::ActivitySource::EventLog {
            path: home
                .path()
                .join("session-state")
                .join(id.to_string())
                .join("events.jsonl")
        }
    );
    assert_eq!(
        micold_core::provider::ClaudeProvider.activity_source(home.path(), cwd, id),
        micold_core::provider::ActivitySource::Hooks,
        "payload-free, and that is the point"
    );
}

// ---------------------------------------------------------------------------------------
// T069 — availability is computed, never remembered
// ---------------------------------------------------------------------------------------

/// A scratch `PATH` holding exactly the named executables, restored on drop.
///
/// Process-global, like `COPILOT_HOME`, so the two availability tests below share one `#[test]`
/// function and never race each other.
struct ScopedPath {
    _dir: tempfile::TempDir,
    bin: PathBuf,
    previous: Option<std::ffi::OsString>,
}

impl ScopedPath {
    fn empty() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let previous = std::env::var_os("PATH");
        std::env::set_var("PATH", &bin);
        Self {
            _dir: dir,
            bin,
            previous,
        }
    }

    fn install(&self, command: &str) {
        let path = self.bin.join(command);
        std::fs::write(&path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }

    fn uninstall(&self, command: &str) {
        let _ = std::fs::remove_file(self.bin.join(command));
    }
}

impl Drop for ScopedPath {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(value) => std::env::set_var("PATH", value),
            None => std::env::remove_var("PATH"),
        }
    }
}

#[test]
fn availability_is_a_live_path_lookup_that_remembers_nothing() {
    // T069, research R11. The **method** memoises nothing: install the binary and it becomes
    // available; remove it and it stops. That is what makes "installed" impossible to go stale.
    //
    // Keep the claim scoped to the method. The client legitimately holds an in-memory snapshot of
    // the answer (`State::available_providers`), refreshed when the choice is offered, because a
    // view cannot run a `PATH` probe per frame and SC-006 forbids scheduling one. R11's rule is
    // "never *persisted*"; "never held in memory anywhere" is not, and asserting the latter here
    // would contradict the design.
    let path = ScopedPath::empty();

    assert!(
        !CopilotProvider.is_available(),
        "an empty PATH means neither CLI is installed"
    );
    assert!(!micold_core::provider::ClaudeProvider.is_available());

    path.install("copilot");
    assert!(
        CopilotProvider.is_available(),
        "the answer changed with the machine, without anything being told to re-check"
    );
    assert!(
        !micold_core::provider::ClaudeProvider.is_available(),
        "and it is per provider — installing one says nothing about the other"
    );

    path.install("claude");
    assert!(micold_core::provider::ClaudeProvider.is_available());

    // Called twice in a row: still live, so a memoising implementation is caught here rather than
    // by a user whose fresh install is not noticed until they restart.
    assert!(CopilotProvider.is_available());
    path.uninstall("copilot");
    assert!(
        !CopilotProvider.is_available(),
        "uninstalling is noticed too — a cache would still be answering `true`"
    );

    // Nothing was written anywhere. The provider has no state and no store; the only input is the
    // environment, and the only output is the return value.
    assert!(
        std::fs::read_dir(path.bin.parent().unwrap())
            .unwrap()
            .flatten()
            .all(|e| e.file_name() == "bin"),
        "asking about availability created no file of its own"
    );
}

#[test]
fn a_directory_named_like_the_cli_is_not_an_installed_cli() {
    // The lookup asks for a *file*. A directory called `copilot` on `PATH` — a source checkout, a
    // vendored tree — would otherwise report the CLI as installed and the spawn would fail with
    // something the user cannot connect to what they did.
    let path = ScopedPath::empty();
    std::fs::create_dir_all(path.bin.join("copilot")).unwrap();
    assert!(!CopilotProvider.is_available());
}
