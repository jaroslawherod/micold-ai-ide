//! Live reload of the daemon's configuration (#1524, US1, data-model *Config file on a running
//! daemon*, research D5, contract R7 and R10).
//!
//! The state machine runs through `ConfigRuntime::check` with an injected clock; what needs a real
//! watcher and a real clock (SC-004: in use within 5 s) runs `ConfigRuntime::spawn`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_daemon::config_watch::{ConfigRuntime, Outcome, WatchMode};

fn host(env_value: &str, log: &str, extra: &str) -> String {
    format!(
        "schema = 1\n[daemon]\nname = \"t\"\nkind = \"host\"\nlog = \"{log}\"\n\
         [session.env]\nA = \"{env_value}\"\n{extra}\n"
    )
}

/// What the application does: a temporary file in the same directory, then a rename.
fn replace(path: &Path, text: &str) {
    let part = path.with_extension("part");
    std::fs::write(&part, text).unwrap();
    std::fs::rename(&part, path).unwrap();
}

fn setup(text: &str) -> (tempfile::TempDir, PathBuf, Arc<ConfigRuntime>) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, text).unwrap();
    let runtime = Arc::new(ConfigRuntime::open(&file).expect("a valid file opens"));
    (dir, file, runtime)
}

fn a_of(runtime: &ConfigRuntime) -> String {
    runtime.current().session.env["A"].clone()
}

#[cfg(unix)]
fn mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// A secret file `dir/<relative>` holding `content` (owner-only on Unix), and the `[[github.tokens]]`
/// section that names it.
fn secret_fixture(dir: &Path, relative: &str, content: &str) -> (PathBuf, String) {
    let secret = dir.join(relative);
    std::fs::write(&secret, content).unwrap();
    #[cfg(unix)]
    mode(&secret, 0o600);
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
    (secret, named)
}

// --- T016: the state machine ---

#[test]
fn a_live_key_change_applies_without_a_restart() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let before = runtime.status().applied_hash;
    assert_eq!(runtime.check(Instant::now()), Outcome::Unchanged);
    replace(&file, &host("2", "info", ""));
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    assert_eq!(a_of(&runtime), "2");
    let status = runtime.status();
    assert!(!status.restart_needed && status.changed_restart_keys.is_empty());
    assert_eq!(status.last_error, None);
    assert_ne!(status.applied_hash, before);
    assert_eq!(runtime.check(Instant::now()), Outcome::Unchanged);
}

#[test]
fn a_restart_key_change_sets_restart_needed_and_names_the_key() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    replace(&file, &host("1", "debug", ""));
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    let status = runtime.status();
    assert!(status.restart_needed);
    assert_eq!(status.changed_restart_keys, vec!["daemon.log".to_string()]);
    // Changing it back is no longer a difference from what this process started with.
    replace(&file, &host("1", "info", ""));
    runtime.check(Instant::now());
    let status = runtime.status();
    assert!(!status.restart_needed && status.changed_restart_keys.is_empty());
}

/// A minimal host file; `daemon_lines` go in `[daemon]`, `sections` follow it.
fn file_with(name: &str, kind: &str, daemon_lines: &str, sections: &str) -> String {
    format!(
        "schema = 1\n[daemon]\nname = \"{name}\"\nkind = \"{kind}\"\n{daemon_lines}\n{sections}\n"
    )
}

/// Each restart-only key (contract R7), changed alone from the minimal file, is named and flags a
/// restart; and changing it back clears the flag.
#[test]
fn every_restart_key_changed_alone_sets_restart_needed_and_names_itself() {
    let tmp = tempfile::tempdir().unwrap();
    let (token, _) = secret_fixture(tmp.path(), "client-token", "t\n");
    let script = tmp.path().join("setup.sh");
    std::fs::write(&script, "#!/bin/sh\n").unwrap();
    let t = tmp.path().display();
    let cases: Vec<(&str, String)> = vec![
        ("daemon.id", file_with("t", "host", "id = \"edge-1\"", "")),
        ("daemon.name", file_with("other", "host", "", "")),
        ("daemon.kind", file_with("t", "ssh", "", "")),
        (
            "daemon.data_dir",
            file_with("t", "host", "data_dir = \"/tmp/d\"", ""),
        ),
        (
            "daemon.home",
            file_with("t", "host", "home = \"/tmp/h\"", ""),
        ),
        (
            "daemon.time_zone",
            file_with("t", "host", "time_zone = \"UTC\"", ""),
        ),
        (
            "daemon.image",
            file_with("t", "host", "image = \"img:1\"", ""),
        ),
        (
            "daemon.idle_stop",
            file_with("t", "host", "idle_stop = \"off\"", ""),
        ),
        ("daemon.log", file_with("t", "host", "log = \"debug\"", "")),
        (
            "listen",
            file_with("t", "host", "", &format!("[listen]\nsocket = '{t}/l.sock'")),
        ),
        (
            "auth",
            file_with(
                "t",
                "host",
                "",
                &format!("[auth]\nclient_token_file = '{}'", token.display()),
            ),
        ),
        (
            "ssh",
            file_with("t", "host", "", &format!("[ssh]\ndir = '{t}'")),
        ),
        (
            "storage.clone_root",
            file_with("t", "host", "", "[storage]\nclone_root = \"/tmp/c\""),
        ),
        (
            "startup",
            file_with(
                "t",
                "host",
                "",
                &format!("[startup]\nscript_file = '{}'", script.display()),
            ),
        ),
    ];
    for (key, changed) in cases {
        let base = file_with("t", "host", "", "");
        let (_dir, file, runtime) = setup(&base);
        replace(&file, &changed);
        assert_eq!(
            runtime.check(Instant::now()),
            Outcome::Applied,
            "{key}: the changed file must be valid: {:?}",
            runtime.status().last_error
        );
        let status = runtime.status();
        assert!(status.restart_needed, "{key} changed: a restart is needed");
        assert_eq!(
            status.changed_restart_keys,
            vec![key.to_string()],
            "{key} changed alone is the only key named"
        );
        replace(&file, &base);
        runtime.check(Instant::now());
        assert!(
            !runtime.status().restart_needed,
            "{key} changed back is no longer a difference"
        );
    }
}

/// Live keys (contract R7) never ask for a restart.
#[test]
fn live_keys_changed_do_not_set_restart_needed() {
    let tmp = tempfile::tempdir().unwrap();
    let (_secret, token) = secret_fixture(tmp.path(), "gh-token", "t\n");
    let live = [
        ("session.env", "[session.env]\nB = \"2\"".to_string()),
        ("github", token),
        ("ai", "[ai]\ndefault = \"claude_code\"".to_string()),
        ("git", "[git]\nssh_to_https = true".to_string()),
    ];
    for (key, section) in live {
        let (_dir, file, runtime) = setup(&file_with("t", "host", "", ""));
        replace(&file, &file_with("t", "host", "", &section));
        assert_eq!(
            runtime.check(Instant::now()),
            Outcome::Applied,
            "{key}: the changed file must be valid: {:?}",
            runtime.status().last_error
        );
        let status = runtime.status();
        assert!(
            !status.restart_needed && status.changed_restart_keys.is_empty(),
            "{key} is live and must not ask for a restart: {:?}",
            status.changed_restart_keys
        );
    }
}

#[test]
fn a_parse_error_keeps_the_last_configuration_and_reports_it() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let hash = runtime.status().applied_hash;
    replace(&file, "schema = 1\n[daemon\n");
    assert_eq!(runtime.check(Instant::now()), Outcome::Rejected);
    assert_eq!(a_of(&runtime), "1");
    let status = runtime.status();
    assert_eq!(status.applied_hash, hash);
    let error = status.last_error.expect("the rejection is reported");
    assert!(
        error.contains("daemon.toml:2:8:"),
        "the error names file, line and column of the break: {error}"
    );
    // The same bad bytes are not judged again; fixing the file clears the error.
    assert_eq!(runtime.check(Instant::now()), Outcome::Unchanged);
    replace(&file, &host("3", "info", ""));
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    assert_eq!(runtime.status().last_error, None);
    assert_eq!(a_of(&runtime), "3");
}

#[test]
fn an_invalid_orchestrator_table_keeps_the_last_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let token = dir.path().join("enroll");
    std::fs::write(&token, "x").unwrap();
    #[cfg(unix)]
    mode(&token, 0o600);
    let valid = format!(
        "[orchestrator]\nurl = \"https://orch.example\"\nenrollment_token_file = \"{}\"\n",
        token.display()
    );
    let (_dir, file, runtime) = setup(&host("1", "info", &valid));
    let hash = runtime.status().applied_hash;
    // O2 (http) next to a valid live change.
    let bad = valid.replace("https://", "http://");
    replace(&file, &host("2", "info", &bad));
    assert_eq!(runtime.check(Instant::now()), Outcome::Rejected);
    assert_eq!(a_of(&runtime), "1", "no key of the new file is in force");
    let current = runtime.current();
    let o = current.orchestrator.as_ref().expect("the last table stays");
    assert_eq!(o.url.as_deref(), Some("https://orch.example"));
    let status = runtime.status();
    assert_eq!(status.applied_hash, hash);
    assert!(status.last_error.unwrap().contains("https"));
}

#[test]
fn a_removed_file_keeps_the_last_configuration() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    std::fs::remove_file(&file).unwrap();
    assert_eq!(runtime.check(Instant::now()), Outcome::Rejected);
    assert_eq!(a_of(&runtime), "1");
    let error = runtime.status().last_error.unwrap();
    assert!(
        error.contains("file removed; keeping the last configuration"),
        "{error}"
    );
    replace(&file, &host("4", "info", ""));
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    assert_eq!(a_of(&runtime), "4");
    assert_eq!(runtime.status().last_error, None);
}

#[test]
fn an_invalid_new_file_applies_none_of_its_keys() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    // A valid live change next to a failing rule (V16: a reserved variable name).
    replace(&file, &host("2", "info", "PATH = \"/x\""));
    assert_eq!(runtime.check(Instant::now()), Outcome::Rejected);
    assert_eq!(a_of(&runtime), "1", "no key of the new file is in force");
    assert!(!runtime.current().session.env.contains_key("PATH"));
    assert!(runtime.status().last_error.unwrap().contains("PATH"));
}

#[cfg(unix)]
#[test]
fn an_unreadable_secret_rejects_the_whole_new_file() {
    if unsafe { libc::geteuid() } == 0 {
        // Root reads anything, so the case cannot be built; say so rather than pass silently.
        eprintln!("SKIPPED an_unreadable_secret_rejects_the_whole_new_file: running as root");
        return;
    }
    let (dir, file, runtime) = setup(&host("1", "info", ""));
    let (secret, named) = secret_fixture(dir.path(), "token", "s3cr3t\n");
    mode(&secret, 0o000);
    replace(&file, &host("2", "info", &named));
    assert_eq!(runtime.check(Instant::now()), Outcome::Rejected);
    assert_eq!(a_of(&runtime), "1");
    let error = runtime.status().last_error.unwrap();
    assert!(
        error.contains("token") && !error.contains("s3cr3t"),
        "{error}"
    );
    mode(&secret, 0o600);
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    assert_eq!(a_of(&runtime), "2");
}

#[test]
fn a_secret_that_is_not_there_yet_is_retried_and_reported_after_120_seconds() {
    let (dir, file, runtime) = setup(&host("1", "info", ""));
    let secret = dir.path().join("late-token");
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
    let start = Instant::now();
    replace(&file, &host("2", "info", &named));
    assert_eq!(runtime.check(start), Outcome::Pending);
    assert_eq!(a_of(&runtime), "1", "the old configuration stays in force");
    assert_eq!(runtime.status().last_error, None, "not yet an error");
    assert_eq!(
        runtime.check(start + Duration::from_secs(119)),
        Outcome::Pending
    );
    assert_eq!(runtime.status().last_error, None);
    assert_eq!(
        runtime.check(start + Duration::from_secs(121)),
        Outcome::Pending
    );
    let error = runtime.status().last_error.expect("reported after 120 s");
    assert!(error.contains("late-token"), "{error}");
    // The secret arrives: the next poll applies the file and clears the report.
    std::fs::write(&secret, "tok\n").unwrap();
    #[cfg(unix)]
    mode(&secret, 0o600);
    assert_eq!(
        runtime.check(start + Duration::from_secs(123)),
        Outcome::Applied
    );
    assert_eq!(a_of(&runtime), "2");
    assert_eq!(runtime.status().last_error, None);
    assert_eq!(runtime.current().github.tokens.len(), 1);
}

#[test]
fn a_changed_secret_alone_is_a_change() {
    let dir = tempfile::tempdir().unwrap();
    let (secret, named) = secret_fixture(dir.path(), "token", "one\n");
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    let runtime = ConfigRuntime::open(&file).unwrap();
    let before = runtime.status().applied_hash;
    replace(&secret, "two\n");
    assert_eq!(
        runtime.check(Instant::now()),
        Outcome::Applied,
        "a secret that changed under an unchanged file is a change"
    );
    assert_ne!(
        runtime.status().applied_hash,
        before,
        "the applied hash follows the secret's content"
    );
}

// --- T016a: the diagnostic section ---

#[test]
fn the_status_carries_the_file_the_hash_and_never_a_secret() {
    let dir = tempfile::tempdir().unwrap();
    let (secret, named) = secret_fixture(dir.path(), "token", "s3cr3t-value\n");
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    let runtime = ConfigRuntime::open(&file).unwrap();
    let status = runtime.status();
    assert_eq!(status.path, file);
    assert_eq!((status.schema, status.kind.as_str()), (1, "host"));
    assert_eq!(status.applied_hash.len(), 64);
    assert!(status.applied_hash.bytes().all(|b| b.is_ascii_hexdigit()));
    assert!(
        status.resolved.contains("daemon: t (host)"),
        "{}",
        status.resolved
    );
    assert!(status.resolved.contains(&secret.display().to_string()));
    assert!(!format!("{status:?}").contains("s3cr3t-value"));
}

#[test]
fn the_hash_is_the_sha256_of_the_config_and_what_it_names() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let bare = std::fs::read(&file).unwrap();
    let mut framed = (bare.len() as u64).to_be_bytes().to_vec();
    framed.extend_from_slice(&bare);
    let expected = micold_core::protocol::hashing::sha256(&framed);
    let hex: String = expected.iter().map(|b| format!("{b:02x}")).collect();
    assert_eq!(runtime.status().applied_hash, hex);
}

// --- SC-004: in use within 5 s, through the real watcher and poll (the watcher tests beat the poll) ---

/// Lets the task's immediate first poll pass, so a later change can only be found by a real source.
async fn settled(runtime: &Arc<ConfigRuntime>, mode: WatchMode) -> tokio::task::JoinHandle<()> {
    let task = runtime.spawn(mode);
    tokio::time::sleep(Duration::from_millis(300)).await;
    task
}

/// The poll runs every 2 s from the task's start and the change is made ~0.3 s in, so a change
/// applied within this budget was found by the directory watcher: the poll cannot have run yet.
const BEFORE_THE_POLL: Duration = Duration::from_millis(1500);

async fn applied_within(runtime: &ConfigRuntime, a: &str, budget: Duration) -> bool {
    let deadline = Instant::now() + budget;
    while Instant::now() < deadline {
        if a_of(runtime) == a {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rename_replaced_file_is_found_by_the_watcher_before_the_poll_runs() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let _task = settled(&runtime, WatchMode::Full).await;
    replace(&file, &host("2", "info", ""));
    assert!(
        applied_within(&runtime, "2", BEFORE_THE_POLL).await,
        "the directory watch, not the 2 s poll, must find a rename-replaced file"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_with_no_watch_event_is_found_by_the_poll_within_5_seconds() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let _task = settled(&runtime, WatchMode::PollOnly).await;
    replace(&file, &host("2", "info", ""));
    assert!(applied_within(&runtime, "2", Duration::from_secs(5)).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_secret_is_found_by_the_watcher_before_the_poll_runs() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    std::fs::create_dir(&secrets).unwrap();
    let (secret, named) = secret_fixture(&secrets, "token", "one\n");
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    let runtime = Arc::new(ConfigRuntime::open(&file).unwrap());
    let before = runtime.status().applied_hash;
    let _task = settled(&runtime, WatchMode::Full).await;
    replace(&secret, "two\n");
    let deadline = Instant::now() + BEFORE_THE_POLL;
    while runtime.status().applied_hash == before && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_ne!(
        runtime.status().applied_hash,
        before,
        "the watch on the secret's directory must find it before the poll runs"
    );
}

/// A Kubernetes ConfigMap volume: `daemon.toml -> ..data/daemon.toml`, `..data -> ..<stamp>`, and
/// an update writes a new stamped directory and swaps the `..data` link.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_kubernetes_data_symlink_swap_is_found_by_the_watcher_before_the_poll_runs() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir(root.join("..2024_a")).unwrap();
    std::fs::write(root.join("..2024_a/daemon.toml"), host("1", "info", "")).unwrap();
    symlink("..2024_a", root.join("..data")).unwrap();
    symlink("..data/daemon.toml", root.join("daemon.toml")).unwrap();
    let runtime = Arc::new(ConfigRuntime::open(&root.join("daemon.toml")).unwrap());
    let _task = settled(&runtime, WatchMode::Full).await;

    std::fs::create_dir(root.join("..2024_b")).unwrap();
    std::fs::write(root.join("..2024_b/daemon.toml"), host("2", "info", "")).unwrap();
    symlink("..2024_b", root.join("..data_tmp")).unwrap();
    std::fs::rename(root.join("..data_tmp"), root.join("..data")).unwrap();
    assert!(
        applied_within(&runtime, "2", BEFORE_THE_POLL).await,
        "the watch on the directory must see the `..data` link swap before the poll runs"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_rejected_file_is_pushed_to_the_listener() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    runtime.on_change(Box::new(move |status| {
        sink.lock().unwrap().push(status.last_error.clone());
    }));
    replace(&file, "not toml [");
    runtime.check(Instant::now());
    replace(&file, &host("2", "info", ""));
    runtime.check(Instant::now());
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].is_some() && seen[1].is_none());
}

// --- T019b: the daemon never opens the config or a secret for writing (FR-025, SC-008) ---

#[cfg(unix)]
#[test]
fn a_reload_never_writes_the_config_or_a_secret() {
    use std::os::unix::fs::MetadataExt;
    let dir = tempfile::tempdir().unwrap();
    let (secret, named) = secret_fixture(dir.path(), "token", "s3cr3t\n");
    mode(&secret, 0o400);
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    mode(&file, 0o400);
    let stamp = |p: &Path| {
        let m = std::fs::metadata(p).unwrap();
        (
            std::fs::read(p).unwrap(),
            m.mtime(),
            m.mtime_nsec(),
            m.ino(),
        )
    };
    let before = (stamp(&file), stamp(&secret));

    let runtime = ConfigRuntime::open(&file).unwrap();
    for _ in 0..3 {
        assert_eq!(
            runtime.check(Instant::now()),
            Outcome::Unchanged,
            "an untouched file is not a change"
        );
    }
    let _ = runtime.status();
    let _ = micold_core::daemon_config::resolved_view(&runtime.current());
    // The credential helper's root decision does not touch them either.
    let settings =
        micold_daemon::settings::DaemonSettings::resolve(Some(&runtime.current()), &|_| None);
    let _ = micold_daemon::git_credential::credential_root_of(&settings);

    assert_eq!(
        before,
        (stamp(&file), stamp(&secret)),
        "reading must leave content, mtime and inode as they were"
    );
}

/// Applying a changed file and a changed secret reads both; neither may be opened for writing, not
/// even without writing a byte (`IN_CLOSE_WRITE` fires on closing any descriptor opened to write,
/// which no file mode can show to a root user).
#[cfg(target_os = "linux")]
#[test]
fn applying_a_changed_file_and_secret_never_opens_either_for_writing() {
    use notify::{event::AccessKind, event::AccessMode, EventKind, RecursiveMode, Watcher};
    let dir = tempfile::tempdir().unwrap();
    let (secret, named) = secret_fixture(dir.path(), "token", "one\n");
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    let runtime = ConfigRuntime::open(&file).unwrap();
    // The replacements are written first, so the only events left are the daemon's own. They stay
    // writable on purpose: a read-only file would make a stray write-open fail instead of show.
    let next_secret = dir.path().join("token.next");
    let next_file = dir.path().join("daemon.toml.next");
    std::fs::write(&next_secret, "two\n").unwrap();
    std::fs::write(&next_file, host("2", "info", &named)).unwrap();

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let _ = tx.send(event);
    })
    .unwrap();
    watcher
        .watch(dir.path(), RecursiveMode::NonRecursive)
        .unwrap();
    std::fs::rename(&next_secret, &secret).unwrap();
    std::fs::rename(&next_file, &file).unwrap();
    assert_eq!(
        runtime.check(Instant::now()),
        Outcome::Applied,
        "the changed file and secret are applied"
    );
    assert_eq!(a_of(&runtime), "2");
    std::thread::sleep(Duration::from_millis(300));
    let written: Vec<_> = rx
        .try_iter()
        .filter_map(Result::ok)
        .filter(|e| {
            matches!(
                e.kind,
                EventKind::Access(AccessKind::Close(AccessMode::Write))
            )
        })
        .collect();
    assert!(
        written.is_empty(),
        "a file was opened for writing: {written:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_busy_directory_does_not_postpone_a_reload_past_5_seconds() {
    let (dir, file, runtime) = setup(&host("1", "info", ""));
    let _task = settled(&runtime, WatchMode::Full).await;
    let noise = dir.path().join("noise.log");
    let writer = tokio::spawn(async move {
        for i in 0..200 {
            let _ = std::fs::write(&noise, i.to_string());
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    });
    replace(&file, &host("2", "info", ""));
    assert!(applied_within(&runtime, "2", Duration::from_secs(5)).await);
    writer.abort();
}

// --- #1535: the start script's last run reaches the status ---

#[test]
fn the_status_carries_the_start_scripts_last_run() {
    use micold_core::protocol::messages::{StartupOutcome, StartupReport};
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", "")).unwrap();
    let runtime = ConfigRuntime::open(&file).expect("a valid file opens");
    let report = StartupReport {
        outcome: StartupOutcome::Ok,
        reason: None,
        tail: vec![],
        finished_at_secs: 7,
    };
    micold_daemon::startup_script::record_report(report.clone());
    assert_eq!(runtime.status().startup, Some(report));
}

/// T100 (FR-040): the diagnostic shows each delivered sign-in's `expiresAt`, and never the token.
#[cfg(unix)]
#[test]
fn the_status_shows_each_delivered_sign_ins_expiry_and_never_its_token() {
    let dir = tempfile::tempdir().unwrap();
    let sign_in = dir.path().join("claude.json");
    std::fs::write(
        &sign_in,
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-secret","expiresAt":1700000000000}}"#,
    )
    .unwrap();
    mode(&sign_in, 0o600);
    let extra = format!(
        "[[ai.cli]]\nid = \"claude_code\"\nsign_in_file = \"{}\"\n",
        sign_in.display()
    );
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &extra)).unwrap();
    let status = ConfigRuntime::open(&file).unwrap().status();
    assert_eq!(status.ai_cli_sign_ins.len(), 1);
    assert_eq!(status.ai_cli_sign_ins[0].cli, "claude_code");
    assert_eq!(status.ai_cli_sign_ins[0].expires_at_secs, 1_700_000_000);
    assert!(!format!("{status:?}").contains("sk-ant-secret"));
}
