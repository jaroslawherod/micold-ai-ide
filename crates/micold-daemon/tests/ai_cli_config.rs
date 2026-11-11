//! #1532 (contract "`[[ai.cli]]`"): a configured `[[ai.cli]]` entry's `args`/`env`/`model` add to
//! the CLI's own launch — never a replacement — and a configured `api_key_file` reaches the
//! session through its environment, never on the command line.
//!
//! Unix only, like the other tests that run a real spawn through a `PATH` stand-in
//! (`crates/micold-daemon/tests/one_session_per_worktree.rs`): the stand-in is a `#!/bin/sh`
//! script made executable with `std::os::unix::fs::PermissionsExt`; a Windows stand-in needs its
//! own compiled binary (see `session_identity_env.rs`'s `write_stand_in`), not worth it here.
// unix-only: the stand-in is a `#!/bin/sh` script, made executable with `PermissionsExt`.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::project::{Availability, Project};
use micold_core::session::{
    AiCli, Session, SessionId, SessionLabel, SessionLocation, TerminalMode,
};
use micold_core::settings::JsonFileSettingsStore;
use micold_core::store::{JsonFileStore, ProjectStore};
use micold_core::terminal::LaunchMode;
use micold_core::workspace::Workspace;
use micold_daemon::catalog::Catalog;
use micold_daemon::config_load::LoadedFile;
use micold_daemon::config_watch::ConfigRuntime;
use micold_daemon::state::DaemonState;
use uuid::Uuid;

/// `PATH` is process-global; this binary runs one test, so there is nothing else to race with.
static PATH_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn wait_until(timeout: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if cond() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    cond()
}

/// A stand-in for `command` that records its argv (joined by `$*`) and its whole environment to
/// `<bin>/<command>.argv` / `.env`, then reads its stdin until closed (as a real AI CLI's PTY
/// child would).
fn stand_in(bin: &Path, command: &str) {
    let argv_file = bin.join(format!("{command}.argv"));
    let env_file = bin.join(format!("{command}.env"));
    let script = format!(
        "#!/bin/sh\nprintf '%s' \"$*\" > '{a}'\nenv > '{e}'\nexec cat > /dev/null\n",
        a = argv_file.display(),
        e = env_file.display(),
    );
    let path = bin.join(command);
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A catalog holding one AI-CLI session of `provider` at the project root.
fn catalog_with_ai_cli_session(
    project_dir: &Path,
    store_dir: &Path,
    provider: AiCli,
) -> (Catalog, SessionId) {
    let id = SessionId::from_uuid(Uuid::from_u128(0xA1C11));
    let session = Session::restored(
        id,
        SessionLocation::Default,
        SessionLabel::Named("AI".into()),
        TerminalMode::AiCli,
        provider,
    );
    let mut sessions = BTreeMap::new();
    sessions.insert(project_dir.to_path_buf(), vec![session]);
    let workspace = Workspace {
        projects: vec![Project::new(
            project_dir.to_path_buf(),
            false,
            Availability::Available,
        )],
        active: Some(project_dir.to_path_buf()),
        sessions,
        ..Default::default()
    };
    let projects_path = store_dir.join("projects.json");
    JsonFileStore::at(projects_path.clone())
        .save(&workspace)
        .unwrap();
    // Environment-include off (#1774): its default sources the developer's own shell profile, and
    // the `PATH` that yields replaces the one the test set, so a machine with a real `claude`
    // launched it instead of the stand-in.
    {
        use micold_core::settings::{Settings, SettingsStore};
        JsonFileSettingsStore::at(store_dir.join("settings.json"))
            .save(&Settings {
                env_include_enabled: false,
                ..Settings::default()
            })
            .unwrap();
    }
    let catalog = Catalog::load(
        Box::new(JsonFileStore::at(projects_path)),
        Box::new(JsonFileSettingsStore::at(store_dir.join("settings.json"))),
    );
    (catalog, id)
}

/// T060: configured `args`/`env` add to Codex's own launch — never a replacement — and a
/// configured `api_key_file` reaches it as `OPENAI_API_KEY` in the environment, never on the
/// command line (contract rows `args`, `env`, `api_key_file`; FR-013, SC-008).
#[test]
fn configured_args_and_env_add_to_the_clis_own_launch_and_the_api_key_never_reaches_argv() {
    let _guard = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = tempfile::tempdir().unwrap();
    stand_in(bin.path(), "codex");
    let previous_path = std::env::var_os("PATH");
    std::env::set_var(
        "PATH",
        std::env::join_paths([
            bin.path().to_path_buf(),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ])
        .unwrap(),
    );

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (catalog, id) = catalog_with_ai_cli_session(project.path(), store.path(), AiCli::Codex);
    let state = DaemonState::new(catalog);

    let key_file = project.path().join("codex.key");
    std::fs::write(&key_file, "sk-codex-test\n").unwrap();
    let text = format!(
        "schema = 1\n[daemon]\nname = \"d\"\nkind = \"host\"\n\n[[ai.cli]]\nid = \"codex\"\nargs = [\"--extra-flag\"]\napi_key_file = \"{}\"\nenv = {{ FOO = \"bar\" }}\n",
        key_file.display(),
    );
    let config = micold_core::daemon_config::parse(&text, "test.toml").expect("a valid config");
    state.set_config(Arc::new(ConfigRuntime::new(LoadedFile {
        path: PathBuf::from("test.toml"),
        config,
        hash: "x".to_string(),
        warnings: Vec::new(),
    })));

    state
        .start_session(id, LaunchMode::Fresh)
        .expect("start the ai-cli session");
    let live = state.live_session(id).expect("registered");
    assert!(
        wait_until(Duration::from_secs(5), || bin
            .path()
            .join("codex.argv")
            .exists()),
        "the stand-in must have run and recorded its launch"
    );
    live.kill().expect("kill");

    let argv = std::fs::read_to_string(bin.path().join("codex.argv")).unwrap();
    assert!(
        argv.contains("--extra-flag"),
        "configured args must be appended to the app's own: {argv}"
    );
    assert!(
        !argv.contains("sk-codex-test"),
        "the api key must never be on the command line: {argv}"
    );

    let env = std::fs::read_to_string(bin.path().join("codex.env")).unwrap();
    assert!(
        env.lines().any(|l| l == "OPENAI_API_KEY=sk-codex-test"),
        "the configured api_key_file must reach Codex as OPENAI_API_KEY: {env}"
    );
    assert!(
        env.lines().any(|l| l == "FOO=bar"),
        "configured env must be added to the launch environment: {env}"
    );

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }
}

/// T065a: a configured `token_file`'s content reaches Claude Code as `CLAUDE_CODE_OAUTH_TOKEN` in
/// the environment, with any trailing `expires_at=` line stripped before it gets there, and never
/// on the command line (contract row `api_key_file`'s same rule, applied to a sign-in token).
#[test]
fn configured_token_file_reaches_claude_code_as_oauth_token_without_the_expiry_line() {
    let _guard = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = tempfile::tempdir().unwrap();
    stand_in(bin.path(), "claude");
    let previous_path = std::env::var_os("PATH");
    std::env::set_var(
        "PATH",
        std::env::join_paths([
            bin.path().to_path_buf(),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ])
        .unwrap(),
    );

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (catalog, id) =
        catalog_with_ai_cli_session(project.path(), store.path(), AiCli::ClaudeCode);
    let state = DaemonState::new(catalog);

    let token_file = project.path().join("claude.token");
    std::fs::write(&token_file, "sk-ant-oat-test\nexpires_at=1999999999\n").unwrap();
    let text = format!(
        "schema = 1\n[daemon]\nname = \"d\"\nkind = \"host\"\n\n[[ai.cli]]\nid = \"claude_code\"\ntoken_file = \"{}\"\n",
        token_file.display(),
    );
    let config = micold_core::daemon_config::parse(&text, "test.toml").expect("a valid config");
    state.set_config(Arc::new(ConfigRuntime::new(LoadedFile {
        path: PathBuf::from("test.toml"),
        config,
        hash: "x".to_string(),
        warnings: Vec::new(),
    })));

    state
        .start_session(id, LaunchMode::Fresh)
        .expect("start the ai-cli session");
    let live = state.live_session(id).expect("registered");
    assert!(
        wait_until(Duration::from_secs(5), || bin
            .path()
            .join("claude.argv")
            .exists()),
        "the stand-in must have run and recorded its launch"
    );
    live.kill().expect("kill");

    let argv = std::fs::read_to_string(bin.path().join("claude.argv")).unwrap();
    assert!(
        !argv.contains("sk-ant-oat-test"),
        "the sign-in token must never be on the command line: {argv}"
    );

    let env = std::fs::read_to_string(bin.path().join("claude.env")).unwrap();
    assert!(
        env.lines()
            .any(|l| l == "CLAUDE_CODE_OAUTH_TOKEN=sk-ant-oat-test"),
        "the configured token_file must reach Claude Code as CLAUDE_CODE_OAUTH_TOKEN, with the \
         expires_at line stripped: {env}"
    );

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }
}

/// #1532, contract row `enabled`: a CLI the configuration disables refuses the start with its own
/// reason, never "not on `PATH`" — the two causes are different and a user and the log must be
/// told which one actually happened, even when (as here) the binary is installed and would be
/// found just fine.
#[test]
fn a_disabled_cli_refuses_the_start_with_its_own_reason_not_not_on_path() {
    let _guard = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = tempfile::tempdir().unwrap();
    stand_in(bin.path(), "codex");
    let previous_path = std::env::var_os("PATH");
    std::env::set_var(
        "PATH",
        std::env::join_paths([
            bin.path().to_path_buf(),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ])
        .unwrap(),
    );

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (catalog, id) = catalog_with_ai_cli_session(project.path(), store.path(), AiCli::Codex);
    let state = DaemonState::new(catalog);

    let text = "schema = 1\n[daemon]\nname = \"d\"\nkind = \"host\"\n\n[[ai.cli]]\nid = \"codex\"\nenabled = false\n";
    let config = micold_core::daemon_config::parse(text, "test.toml").expect("a valid config");
    state.set_config(Arc::new(ConfigRuntime::new(LoadedFile {
        path: PathBuf::from("test.toml"),
        config,
        hash: "x".to_string(),
        warnings: Vec::new(),
    })));

    let err = state
        .start_session(id, LaunchMode::Fresh)
        .expect_err("a disabled CLI must refuse the start");
    let message = err.to_string();
    assert!(
        message.contains("disabled"),
        "the refusal must name the real reason: {message}"
    );
    assert!(
        !message.to_lowercase().contains("path"),
        "a disabled CLI must never be reported as \"not on PATH\": {message}"
    );
    assert!(
        !bin.path().join("codex.argv").exists(),
        "a disabled CLI must never actually launch"
    );

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }
}

/// T059/T063: a configured `model` is Claude Code's default when the session's own choice is
/// still "default" (contract row `model`; `data-model` V7 refuses `model` for any other CLI, so
/// only this one CLI needs the end-to-end check).
#[test]
fn a_configured_model_is_claude_codes_default_launch_model() {
    let _guard = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = tempfile::tempdir().unwrap();
    stand_in(bin.path(), "claude");
    let previous_path = std::env::var_os("PATH");
    std::env::set_var(
        "PATH",
        std::env::join_paths([
            bin.path().to_path_buf(),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ])
        .unwrap(),
    );

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let (catalog, id) =
        catalog_with_ai_cli_session(project.path(), store.path(), AiCli::ClaudeCode);
    let state = DaemonState::new(catalog);

    let text = "schema = 1\n[daemon]\nname = \"d\"\nkind = \"host\"\n\n[[ai.cli]]\nid = \"claude_code\"\nmodel = \"opus\"\n";
    let config = micold_core::daemon_config::parse(text, "test.toml").expect("a valid config");
    state.set_config(Arc::new(ConfigRuntime::new(LoadedFile {
        path: PathBuf::from("test.toml"),
        config,
        hash: "x".to_string(),
        warnings: Vec::new(),
    })));

    state
        .start_session(id, LaunchMode::Fresh)
        .expect("start the ai-cli session");
    let live = state.live_session(id).expect("registered");
    assert!(
        wait_until(Duration::from_secs(5), || bin
            .path()
            .join("claude.argv")
            .exists()),
        "the stand-in must have run and recorded its launch"
    );
    live.kill().expect("kill");

    let argv = std::fs::read_to_string(bin.path().join("claude.argv")).unwrap();
    assert!(
        argv.contains("--model opus"),
        "the configured model must launch as the default model choice: {argv}"
    );

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }
}

/// T096 (research D15 row 4): a delivered Claude sign-in seeds `<data_dir>/ai/claude_code/
/// .claude.json` once from the user's own `~/.claude.json`, without `primaryApiKey`, and the
/// user's file is never written.
#[test]
fn a_delivered_claude_sign_in_seeds_claude_json_without_the_api_key() {
    let _guard = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = tempfile::tempdir().unwrap();
    stand_in(bin.path(), "claude");
    let previous_path = std::env::var_os("PATH");
    let previous_home = std::env::var_os("HOME");
    std::env::set_var(
        "PATH",
        std::env::join_paths([
            bin.path().to_path_buf(),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
        ])
        .unwrap(),
    );
    let home = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", home.path());
    let user_json = r#"{"primaryApiKey":"sk-x","projects":{"/p":{"hasTrustDialogAccepted":true}}}"#;
    std::fs::write(home.path().join(".claude.json"), user_json).unwrap();

    let project = tempfile::tempdir().unwrap();
    let store = tempfile::tempdir().unwrap();
    let data = tempfile::tempdir().unwrap();
    let (catalog, id) =
        catalog_with_ai_cli_session(project.path(), store.path(), AiCli::ClaudeCode);
    let state = DaemonState::new(catalog);
    state.set_pasted_data_dir(data.path().to_path_buf());

    let sign_in = project.path().join("sign-in.json");
    std::fs::write(&sign_in, "{}").unwrap();
    let text = format!(
        "schema = 1\n[daemon]\nname = \"d\"\nkind = \"host\"\n\n[[ai.cli]]\nid = \"claude_code\"\nsign_in_file = \"{}\"\n",
        sign_in.display(),
    );
    let config = micold_core::daemon_config::parse(&text, "test.toml").expect("a valid config");
    state.set_config(Arc::new(ConfigRuntime::new(LoadedFile {
        path: PathBuf::from("test.toml"),
        config,
        hash: "x".to_string(),
        warnings: Vec::new(),
    })));

    state
        .start_session(id, LaunchMode::Fresh)
        .expect("start the ai-cli session");
    let live = state.live_session(id).expect("registered");
    assert!(
        wait_until(Duration::from_secs(5), || bin
            .path()
            .join("claude.env")
            .exists()),
        "the stand-in must have run"
    );
    live.kill().expect("kill");

    let seeded =
        std::fs::read_to_string(data.path().join("ai/claude_code/.claude.json")).expect("seeded");
    assert!(!seeded.contains("primaryApiKey"), "{seeded}");
    assert!(seeded.contains("hasTrustDialogAccepted"), "{seeded}");
    assert_eq!(
        std::fs::read_to_string(home.path().join(".claude.json")).unwrap(),
        user_json,
        "the user's own file is never written"
    );

    for (name, value) in [("PATH", previous_path), ("HOME", previous_home)] {
        match value {
            Some(v) => std::env::set_var(name, v),
            None => std::env::remove_var(name),
        }
    }
}
