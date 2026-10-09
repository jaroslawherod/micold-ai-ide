//! `OpenCodeProvider` — the OpenCode profile of the AI CLI seam, fresh start only (feature 488, M1, T003).
//!
//! Every derivation is pure, so the provider is testable without the CLI installed.

use micold_core::provider::{
    ActivitySource, AiCliProvider, FolderTrust, InputReadiness, OpenCodeProvider, ToolServerSupport,
};
use micold_core::session::AiCli;
use micold_core::terminal::LaunchMode;
use std::path::Path;
use uuid::Uuid;

fn id() -> Uuid {
    Uuid::parse_str("44444444-5555-4555-8555-666666666666").unwrap()
}

#[test]
fn identity_is_opencode() {
    assert_eq!(OpenCodeProvider.command(), "opencode");
    assert_eq!(OpenCodeProvider.display_name(), "OpenCode");
    assert_eq!(OpenCodeProvider.id(), AiCli::OpenCode);
    assert_eq!(AiCli::OpenCode.tool_name(), "opencode");
    assert_eq!(AiCli::OpenCode.provider().command(), "opencode");
}

#[test]
fn a_fresh_start_passes_no_arguments() {
    assert!(OpenCodeProvider
        .launch_args(id(), LaunchMode::Fresh)
        .is_empty());
}

#[test]
fn availability_follows_the_path_it_is_given() {
    let dir = tempfile::tempdir().unwrap();
    let path = std::env::join_paths([dir.path()]).unwrap();
    assert!(!OpenCodeProvider.is_available(&path));
    std::fs::write(dir.path().join("opencode"), "").unwrap();
    assert!(OpenCodeProvider.is_available(&path));
    assert!(!OpenCodeProvider.is_available(std::ffi::OsStr::new("")));
}

#[test]
fn an_empty_store_contributes_nothing() {
    let cwd = Path::new("/work");
    let base = Path::new("/nowhere");
    assert!(OpenCodeProvider.recorded_session_ids(base, cwd).is_empty());
    assert!(!OpenCodeProvider.has_recorded_conversation(base, cwd, id()));
    assert_eq!(OpenCodeProvider.read_title(base, cwd, id()), None);
    assert_eq!(OpenCodeProvider.read_label(base, cwd, id()), None);
    assert_eq!(
        OpenCodeProvider.name_in_terminal_title("OpenCode", cwd),
        None
    );
}

#[test]
fn the_seam_answers_for_activity_tools_readiness_and_trust() {
    assert_eq!(
        OpenCodeProvider.activity_source(Path::new("/b"), Path::new("/w"), id()),
        ActivitySource::None
    );
    assert!(matches!(
        OpenCodeProvider.tool_server_support(),
        ToolServerSupport::Unsupported { .. }
    ));
    assert_eq!(
        OpenCodeProvider.input_readiness(),
        InputReadiness::OutputSettled
    );
    assert_eq!(OpenCodeProvider.folder_trust(), FolderTrust::NeverAsks);
    assert_eq!(
        OpenCodeProvider.launch_env(),
        vec![("OPENCODE_DISABLE_AUTOUPDATE".to_string(), "1".to_string())]
    );
}

#[test]
fn closing_a_session_leaves_a_marker_outside_the_cli_store() {
    let base = tempfile::tempdir().unwrap();
    let cwd = Path::new("/work");
    assert!(!OpenCodeProvider.is_archived(base.path(), cwd, id()));
    OpenCodeProvider
        .mark_archived(base.path(), cwd, id())
        .unwrap();
    assert!(OpenCodeProvider.is_archived(base.path(), cwd, id()));
    assert!(!OpenCodeProvider.is_archived(base.path(), cwd, Uuid::from_u128(7)));
}

// ---------------------------------------------------------------------------------------
// M4 (T023): resume, binding and naming, read through a stand-in `opencode` CLI.
// ---------------------------------------------------------------------------------------

/// The stand-in is a `#!/bin/sh` script, so these run on unix only.
#[cfg(unix)]
mod resume {
    use super::id;
    use micold_core::provider::{
        sole_candidate, AiCliProvider, ConversationIdentity, ConversationRef, OpenCodeProvider,
    };
    use micold_core::terminal::LaunchMode;
    use std::ffi::OsString;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use uuid::Uuid;

    /// The tests change `PATH`; one at a time.
    static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

    struct Cli {
        bin: tempfile::TempDir,
        base: tempfile::TempDir,
        cwd: PathBuf,
        saved: Option<OsString>,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl Cli {
        /// `PATH` holds a stand-in `opencode`. `session list` prints `list.json` when run in `cwd`; `export <id>`
        /// prints `export-<id>.json`; `sleep` in `list.json` makes it hang instead.
        fn new() -> Self {
            let guard = ENV.lock().unwrap_or_else(|p| p.into_inner());
            let bin = tempfile::tempdir().unwrap();
            let base = tempfile::tempdir().unwrap();
            let script = format!(
                "#!/bin/sh\n\
                 dir='{dir}'\n\
                 if [ -f \"$dir/hang\" ]; then exec sleep 30; fi\n\
                 if [ -f \"$dir/fail\" ]; then exit 3; fi\n\
                 if [ \"$1 $2\" = 'session list' ]; then [ \"$(pwd -P)\" = \"$(cat \"$dir/cwd\")\" ] || {{ echo '[]'; exit 0; }}; cat \"$dir/list.json\"; exit 0; fi\n\
                 if [ \"$1\" = export ]; then cat \"$dir/export-$2.json\"; exit 0; fi\n\
                 exit 9\n",
                dir = bin.path().display()
            );
            let program = bin.path().join("opencode");
            std::fs::write(&program, script).unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            let saved = std::env::var_os("PATH");
            let path = std::env::join_paths([bin.path(), Path::new("/usr/bin"), Path::new("/bin")])
                .unwrap();
            std::env::set_var("PATH", path);
            let cwd = std::fs::canonicalize(tempfile::tempdir().unwrap().keep()).unwrap();
            // The stand-in lists sessions only when run in this folder, as the real CLI does.
            std::fs::write(bin.path().join("cwd"), cwd.display().to_string()).unwrap();
            Self {
                bin,
                base,
                cwd,
                saved,
                _guard: guard,
            }
        }

        fn say(&self, name: &str, text: &str) {
            std::fs::write(self.bin.path().join(name), text).unwrap();
        }

        fn list(&self, entries: &[String]) {
            self.say("list.json", &format!("[{}]", entries.join(",")));
        }

        fn entry(&self, id: &str, directory: &Path, created_ms: u128) -> String {
            format!(
                r#"{{"id":"{id}","title":"t","updated":{created_ms},"created":{created_ms},"projectId":"p","directory":"{}"}}"#,
                directory.display()
            )
        }

        fn found(&self, since: SystemTime) -> Vec<ConversationRef> {
            OpenCodeProvider.new_conversations(self.base.path(), &self.cwd, since)
        }
    }

    impl Drop for Cli {
        fn drop(&mut self) {
            match self.saved.take() {
                Some(path) => std::env::set_var("PATH", path),
                None => std::env::remove_var("PATH"),
            }
        }
    }

    fn ms(time: SystemTime) -> u128 {
        time.duration_since(UNIX_EPOCH).unwrap().as_millis()
    }

    fn bind(cli: &Cli, session: Uuid, conversation: &str) {
        let found = ConversationRef {
            id: conversation.to_string(),
            cwd: cli.cwd.clone(),
            created: SystemTime::now(),
        };
        OpenCodeProvider
            .bind(cli.base.path(), session, &found)
            .unwrap();
    }

    #[test]
    fn opencode_mints_its_own_conversation_ids() {
        assert_eq!(OpenCodeProvider.identity(), ConversationIdentity::Minted);
    }

    #[test]
    fn a_bound_session_resumes_its_own_conversation_and_an_unbound_one_starts_fresh() {
        let cli = Cli::new();
        let base = Some(cli.base.path());
        assert!(OpenCodeProvider
            .launch_args_in(base, id(), LaunchMode::Resume)
            .is_empty());
        bind(&cli, id(), "ses_abc123");
        assert_eq!(
            OpenCodeProvider.launch_args_in(base, id(), LaunchMode::Resume),
            vec!["--session".to_string(), "ses_abc123".to_string()]
        );
        // Never another session's: a different session is still fresh, and so is a fresh start.
        assert!(OpenCodeProvider
            .launch_args_in(base, Uuid::from_u128(9), LaunchMode::Resume)
            .is_empty());
        assert!(OpenCodeProvider
            .launch_args_in(base, id(), LaunchMode::Fresh)
            .is_empty());
        assert!(OpenCodeProvider.has_recorded_conversation(cli.base.path(), &cli.cwd, id()));
    }

    #[test]
    fn candidates_are_this_folders_conversations_created_after_the_spawn() {
        let cli = Cli::new();
        let since = SystemTime::now();
        let after = ms(since) + 500;
        let long_before = ms(since) - 60_000;
        let elsewhere = cli.cwd.join("other");
        cli.list(&[
            cli.entry("ses_new", &cli.cwd, after),
            cli.entry("ses_old", &cli.cwd, long_before),
            cli.entry("ses_there", &elsewhere, after),
        ]);
        let ids: Vec<String> = cli.found(since).into_iter().map(|c| c.id).collect();
        assert_eq!(ids, vec!["ses_new".to_string()]);
    }

    #[test]
    fn the_nested_time_shape_is_read_too() {
        let cli = Cli::new();
        let since = SystemTime::now();
        cli.list(&[format!(
            r#"{{"id":"ses_n","directory":"{}","time":{{"created":{},"updated":{}}}}}"#,
            cli.cwd.display(),
            ms(since) + 100,
            ms(since) + 100
        )]);
        assert_eq!(cli.found(since).len(), 1);
    }

    #[test]
    fn a_conversation_another_session_is_bound_to_is_not_a_candidate() {
        let cli = Cli::new();
        let since = SystemTime::now();
        let after = ms(since) + 500;
        cli.list(&[
            cli.entry("ses_mine", &cli.cwd, after),
            cli.entry("ses_taken", &cli.cwd, after + 1),
        ]);
        bind(&cli, Uuid::from_u128(5), "ses_taken");
        let found = cli.found(since);
        assert_eq!(
            sole_candidate(&found).map(|c| c.id.as_str()),
            Some("ses_mine")
        );
    }

    #[test]
    fn two_unclaimed_conversations_are_never_guessed_between() {
        let cli = Cli::new();
        let since = SystemTime::now();
        let after = ms(since) + 500;
        cli.list(&[
            cli.entry("ses_a", &cli.cwd, after),
            cli.entry("ses_b", &cli.cwd, after + 1),
        ]);
        let found = cli.found(since);
        assert_eq!(found.len(), 2);
        assert_eq!(sole_candidate(&found), None);
    }

    #[test]
    fn a_cli_that_fails_hangs_prints_junk_or_is_missing_yields_nothing() {
        let cli = Cli::new();
        let since = SystemTime::now() - Duration::from_secs(1);
        let after = ms(SystemTime::now()) + 500;
        cli.list(&[cli.entry("ses_a", &cli.cwd, after)]);
        assert_eq!(cli.found(since).len(), 1, "the control case finds it");

        cli.say("list.json", "this is not json");
        assert!(cli.found(since).is_empty(), "bad JSON");
        cli.say("list.json", r#"{"id":"ses_a"}"#);
        assert!(cli.found(since).is_empty(), "wrong shape");

        cli.list(&[cli.entry("ses_a", &cli.cwd, after)]);
        cli.say("fail", "");
        assert!(cli.found(since).is_empty(), "non-zero exit");
        std::fs::remove_file(cli.bin.path().join("fail")).unwrap();

        cli.say("hang", "");
        let started = std::time::Instant::now();
        assert!(cli.found(since).is_empty(), "timeout");
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the read is bounded"
        );
        std::fs::remove_file(cli.bin.path().join("hang")).unwrap();

        std::fs::remove_file(cli.bin.path().join("opencode")).unwrap();
        assert!(cli.found(since).is_empty(), "no CLI");
    }

    #[test]
    fn a_session_is_named_from_its_first_typed_turn() {
        let cli = Cli::new();
        bind(&cli, id(), "ses_abc");
        cli.say(
            "export-ses_abc.json",
            r#"{"info":{"id":"ses_abc","title":"New session - 2026"},"messages":[
              {"info":{"role":"assistant"},"parts":[{"type":"text","text":"Hello, how can I help?"}]},
              {"info":{"role":"user"},"parts":[{"type":"text","text":"ignore me","synthetic":true},{"type":"text","text":"Fix the flaky   login test"}]},
              {"info":{"role":"user"},"parts":[{"type":"text","text":"second turn"}]}]}"#,
        );
        assert_eq!(
            OpenCodeProvider.read_title(cli.base.path(), &cli.cwd, id()),
            Some("Fix the flaky login test".to_string())
        );
    }

    #[test]
    fn an_unbound_session_or_an_unreadable_export_has_no_name() {
        let cli = Cli::new();
        assert_eq!(
            OpenCodeProvider.read_title(cli.base.path(), &cli.cwd, id()),
            None
        );
        bind(&cli, id(), "ses_abc");
        assert_eq!(
            OpenCodeProvider.read_title(cli.base.path(), &cli.cwd, id()),
            None,
            "no export"
        );
        cli.say("export-ses_abc.json", "{not json");
        assert_eq!(
            OpenCodeProvider.read_title(cli.base.path(), &cli.cwd, id()),
            None,
            "bad JSON"
        );
        cli.say("export-ses_abc.json", r#"{"messages":[]}"#);
        assert_eq!(
            OpenCodeProvider.read_title(cli.base.path(), &cli.cwd, id()),
            None,
            "no turn"
        );
    }
}
