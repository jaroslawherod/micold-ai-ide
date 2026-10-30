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
    assert!(error.contains("daemon.toml:"), "{error}");
    // The same bad bytes are not judged again; fixing the file clears the error.
    assert_eq!(runtime.check(Instant::now()), Outcome::Unchanged);
    replace(&file, &host("3", "info", ""));
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    assert_eq!(runtime.status().last_error, None);
    assert_eq!(a_of(&runtime), "3");
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
        return; // root reads anything
    }
    let (dir, file, runtime) = setup(&host("1", "info", ""));
    let secret = dir.path().join("token");
    std::fs::write(&secret, "s3cr3t\n").unwrap();
    mode(&secret, 0o000);
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
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
    let secret = dir.path().join("token");
    std::fs::write(&secret, "one\n").unwrap();
    #[cfg(unix)]
    mode(&secret, 0o600);
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    let runtime = ConfigRuntime::open(&file).unwrap();
    let before = runtime.status().applied_hash;
    replace(&secret, "two\n");
    assert_eq!(runtime.check(Instant::now()), Outcome::Applied);
    assert_ne!(runtime.status().applied_hash, before);
}

// --- T016a: the diagnostic section ---

#[test]
fn the_status_carries_the_file_the_hash_and_never_a_secret() {
    let dir = tempfile::tempdir().unwrap();
    let secret = dir.path().join("token");
    std::fs::write(&secret, "s3cr3t-value\n").unwrap();
    #[cfg(unix)]
    mode(&secret, 0o600);
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
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

// --- SC-004: in use within 5 s, through the real watcher and poll ---

/// Lets the task's immediate first poll pass, so a later change can only be found by a real source.
async fn settled(runtime: &Arc<ConfigRuntime>, mode: WatchMode) -> tokio::task::JoinHandle<()> {
    let task = runtime.spawn(mode);
    tokio::time::sleep(Duration::from_millis(300)).await;
    task
}

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
async fn a_rename_replaced_file_is_in_use_within_5_seconds() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let _task = settled(&runtime, WatchMode::Full).await;
    replace(&file, &host("2", "info", ""));
    assert!(applied_within(&runtime, "2", Duration::from_secs(5)).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_change_with_no_watch_event_is_found_by_the_poll_within_5_seconds() {
    let (_dir, file, runtime) = setup(&host("1", "info", ""));
    let _task = settled(&runtime, WatchMode::PollOnly).await;
    replace(&file, &host("2", "info", ""));
    assert!(applied_within(&runtime, "2", Duration::from_secs(5)).await);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_changed_secret_is_in_use_within_5_seconds() {
    let dir = tempfile::tempdir().unwrap();
    let secrets = dir.path().join("secrets");
    std::fs::create_dir(&secrets).unwrap();
    let secret = secrets.join("token");
    std::fs::write(&secret, "one\n").unwrap();
    #[cfg(unix)]
    mode(&secret, 0o600);
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
    let file = dir.path().join("daemon.toml");
    std::fs::write(&file, host("1", "info", &named)).unwrap();
    let runtime = Arc::new(ConfigRuntime::open(&file).unwrap());
    let before = runtime.status().applied_hash;
    let _task = settled(&runtime, WatchMode::Full).await;
    replace(&secret, "two\n");
    let deadline = Instant::now() + Duration::from_secs(5);
    while runtime.status().applied_hash == before && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_ne!(runtime.status().applied_hash, before);
}

/// A Kubernetes ConfigMap volume: `daemon.toml -> ..data/daemon.toml`, `..data -> ..<stamp>`, and
/// an update writes a new stamped directory and swaps the `..data` link.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn a_kubernetes_data_symlink_swap_is_in_use_within_5_seconds() {
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
    assert!(applied_within(&runtime, "2", Duration::from_secs(5)).await);
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
    let secret = dir.path().join("token");
    std::fs::write(&secret, "s3cr3t\n").unwrap();
    mode(&secret, 0o400);
    let named = format!(
        "[[github.tokens]]\nname = \"a\"\ntoken_file = \"{}\"\n",
        secret.display()
    );
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
        assert_eq!(runtime.check(Instant::now()), Outcome::Unchanged);
    }
    let _ = runtime.status();
    let _ = micold_core::daemon_config::resolved_view(&runtime.current());
    // The credential helper's root decision does not touch them either.
    let settings =
        micold_daemon::settings::DaemonSettings::resolve(Some(&runtime.current()), &|_| None);
    let _ = micold_daemon::git_credential::credential_root_of(&settings);

    assert_eq!(before, (stamp(&file), stamp(&secret)));
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
