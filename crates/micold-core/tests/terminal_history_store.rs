//! T015 (feature 041; U44–U54): the directory of saved-history files (data-model §5,
//! contracts/saved-history-file.md §1, §3) — one owner-only file per session, written whole and
//! only when it changed, read back as the snapshot, as nothing, or as the reason it cannot be
//! shown. Every test works in a temporary directory.

use std::path::{Path, PathBuf};

use micold_core::session::SessionId;
use micold_core::terminal_history::{
    encode, history_dir, DamageReason, HistorySnapshot, HistoryStore, HistoryStyle, LoadOutcome,
    LogicalLine, SaveOutcome, SkipReason, StyleRun,
};
use uuid::Uuid;

const SESSION: Uuid = Uuid::from_u128(0x0a1b_2c3d_4e5f_4a6b_8c7d_9e0f_1a2b_3c4d);
const SESSION_FILE: &str = "0a1b2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d.history";
const OTHER_SESSION: Uuid = Uuid::from_u128(0xffff_2c3d_4e5f_4a6b_8c7d_9e0f_1a2b_3c4d);
const OTHER_SESSION_FILE: &str = "ffff2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d.history";

fn session() -> SessionId {
    SessionId::from_uuid(SESSION)
}

fn other_session() -> SessionId {
    SessionId::from_uuid(OTHER_SESSION)
}

/// One line per text, each in the default style.
fn snapshot(texts: &[&str]) -> HistorySnapshot {
    let lines = texts
        .iter()
        .map(|text| LogicalLine {
            text: text.to_string(),
            runs: vec![StyleRun {
                chars: text.chars().count() as u32,
                style: HistoryStyle::default(),
            }],
        })
        .collect();
    HistorySnapshot { lines }
}

/// A store that creates its directory, `terminal-history` under `root`, which is not there yet.
fn store_in(root: &Path) -> HistoryStore {
    HistoryStore::new(root.join("terminal-history"), true)
}

/// Every entry of `dir`, sorted.
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    found.sort();
    found
}

// U44.
#[test]
fn a_saved_snapshot_loads_from_the_file_named_after_the_session() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    let saved = snapshot(&["$ cargo test", "ok"]);

    assert_eq!(store.save(session(), &saved).unwrap(), SaveOutcome::Saved);

    assert_eq!(
        entries(store.dir()),
        [store.dir().join(SESSION_FILE)],
        "one file, <session uuid>.history, hyphenated and lower-case"
    );
    assert_eq!(
        std::fs::read(store.dir().join(SESSION_FILE)).unwrap(),
        encode(&saved),
        "the file is the encoding of the snapshot"
    );
    assert_eq!(store.load(session()), LoadOutcome::History(saved));
}

// U44: what a service start reads is what the run before it wrote.
#[test]
fn another_store_on_the_same_directory_loads_what_the_first_saved() {
    let root = tempfile::tempdir().unwrap();
    let saved = snapshot(&["before the restart"]);
    store_in(root.path()).save(session(), &saved).unwrap();

    assert_eq!(
        store_in(root.path()).load(session()),
        LoadOutcome::History(saved)
    );
}

// Data-model §3: a history with no lines is a history, not an absent one.
#[test]
fn an_empty_snapshot_loads_as_an_empty_history() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    store.save(session(), &HistorySnapshot::default()).unwrap();

    assert_eq!(
        store.load(session()),
        LoadOutcome::History(HistorySnapshot::default())
    );
}

// U45.
#[test]
fn a_session_with_no_file_loads_as_none() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());

    assert_eq!(
        store.load(session()),
        LoadOutcome::None,
        "no directory at all"
    );

    store.save(other_session(), &snapshot(&["theirs"])).unwrap();
    assert_eq!(
        store.load(session()),
        LoadOutcome::None,
        "a directory with another session's file"
    );
    assert!(
        matches!(store.load(other_session()), LoadOutcome::History(_)),
        "and that other session's file loads"
    );
}

// U46.
#[test]
fn a_damaged_file_loads_as_damaged_with_its_reason() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    let path = store.dir().join(SESSION_FILE);
    let whole = encode(&snapshot(&["cut here"]));
    std::fs::create_dir_all(store.dir()).unwrap();

    std::fs::write(&path, &whole[..whole.len() - 1]).unwrap();
    assert_eq!(
        store.load(session()),
        LoadOutcome::Damaged(DamageReason::Truncated),
        "a file that lost its last byte"
    );

    std::fs::write(
        &path,
        b"not a history at all, only some text of a fair length",
    )
    .unwrap();
    assert_eq!(
        store.load(session()),
        LoadOutcome::Damaged(DamageReason::NotAHistory)
    );

    std::fs::write(&path, b"").unwrap();
    assert_eq!(
        store.load(session()),
        LoadOutcome::Damaged(DamageReason::NotAHistory),
        "an empty file is a damaged one, not an absent one"
    );
}

// U46, HF §4 check 2: the size is judged from the file's length, before its content.
#[test]
fn a_file_over_the_size_cap_loads_as_too_large() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    std::fs::create_dir_all(store.dir()).unwrap();
    let file = std::fs::File::create(store.dir().join(SESSION_FILE)).unwrap();
    // Sparse: a length, no content.
    file.set_len(micold_core::terminal_history::MAX_FILE_BYTES + 1)
        .unwrap();

    assert_eq!(
        store.load(session()),
        LoadOutcome::Damaged(DamageReason::TooLarge)
    );
}

// U46: a damaged file is replaced by the next save (FR-018).
#[test]
fn a_save_replaces_a_damaged_file() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    std::fs::create_dir_all(store.dir()).unwrap();
    std::fs::write(store.dir().join(SESSION_FILE), b"damaged").unwrap();
    let saved = snapshot(&["after the damage"]);

    assert_eq!(store.save(session(), &saved).unwrap(), SaveOutcome::Saved);

    assert_eq!(store.load(session()), LoadOutcome::History(saved));
}

// U48.
#[test]
fn a_save_over_an_existing_file_leaves_no_temporary_file() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    let second = snapshot(&["first", "second"]);
    store.save(session(), &snapshot(&["first"])).unwrap();

    assert_eq!(store.save(session(), &second).unwrap(), SaveOutcome::Saved);

    assert_eq!(
        entries(store.dir()),
        [store.dir().join(SESSION_FILE)],
        "only the session's file is in the directory"
    );
    assert_eq!(store.load(session()), LoadOutcome::History(second));
}

// U49 (FR-006): a service killed between the creation of the temporary file and the rename leaves
// part of a file beside the previous whole one.
#[test]
fn a_temporary_file_left_behind_leaves_the_previous_file_loadable() {
    let root = tempfile::tempdir().unwrap();
    let previous = snapshot(&["the save before the kill"]);
    store_in(root.path()).save(session(), &previous).unwrap();
    let dir = root.path().join("terminal-history");
    let interrupted = encode(&snapshot(&["the save that was killed", "half written"]));
    let left_behind = dir.join(format!(".{SESSION_FILE}.tmp"));
    std::fs::write(&left_behind, &interrupted[..interrupted.len() / 2]).unwrap();

    let store = store_in(root.path());

    assert_eq!(
        store.load(session()),
        LoadOutcome::History(previous),
        "the previous whole file"
    );

    let next = snapshot(&["the first save of the next run"]);
    assert_eq!(store.save(session(), &next).unwrap(), SaveOutcome::Saved);
    assert_eq!(
        entries(&dir),
        [dir.join(SESSION_FILE)],
        "the next save clears what was left behind"
    );
    assert_eq!(store.load(session()), LoadOutcome::History(next));
}

// U50 (FR-025).
#[test]
fn two_sessions_have_two_files_and_never_each_others_content() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    let mine = snapshot(&["mine"]);
    let theirs = snapshot(&["theirs"]);

    store.save(session(), &mine).unwrap();
    store.save(other_session(), &theirs).unwrap();

    assert_eq!(
        entries(store.dir()),
        [
            store.dir().join(SESSION_FILE),
            store.dir().join(OTHER_SESSION_FILE)
        ]
    );
    assert_eq!(store.load(session()), LoadOutcome::History(mine.clone()));
    assert_eq!(store.load(other_session()), LoadOutcome::History(theirs));

    // A new save of one leaves the other as it was.
    let theirs_now = snapshot(&["theirs", "and more"]);
    store.save(other_session(), &theirs_now).unwrap();
    assert_eq!(store.load(session()), LoadOutcome::History(mine));
    assert_eq!(
        store.load(other_session()),
        LoadOutcome::History(theirs_now)
    );
}

// U51 (FR-004), the part every platform can see: the answer, and that a changed snapshot and a
// second session are still written.
#[test]
fn a_second_save_of_an_equal_snapshot_is_unchanged() {
    let root = tempfile::tempdir().unwrap();
    let store = store_in(root.path());
    let first = snapshot(&["nothing new"]);
    assert_eq!(store.save(session(), &first).unwrap(), SaveOutcome::Saved);

    assert_eq!(
        store.save(session(), &first.clone()).unwrap(),
        SaveOutcome::Unchanged
    );
    assert_eq!(
        store.save(other_session(), &first).unwrap(),
        SaveOutcome::Saved,
        "an equal snapshot of another session is that session's first save"
    );

    let changed = snapshot(&["nothing new", "something new"]);
    assert_eq!(store.save(session(), &changed).unwrap(), SaveOutcome::Saved);
    assert_eq!(
        store.save(session(), &first).unwrap(),
        SaveOutcome::Saved,
        "back to the earlier content is a change against the file last written"
    );
    assert_eq!(store.load(session()), LoadOutcome::History(first));
}

// U51: "last written" is what this store wrote, not what the file holds. A store that has written
// nothing yet writes, whatever is on disk.
#[test]
fn the_first_save_of_a_store_writes_even_over_an_equal_file() {
    let root = tempfile::tempdir().unwrap();
    let saved = snapshot(&["from the run before"]);
    store_in(root.path()).save(session(), &saved).unwrap();

    assert_eq!(
        store_in(root.path()).save(session(), &saved).unwrap(),
        SaveOutcome::Saved
    );
}

// U52 (R15): a service in a container never creates the directory.
#[test]
fn a_store_that_does_not_create_its_directory_skips_until_it_exists() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("terminal-history");
    let store = HistoryStore::new(dir.clone(), false);
    let saved = snapshot(&["in a container"]);

    assert_eq!(
        store.save(session(), &saved).unwrap(),
        SaveOutcome::Skipped(SkipReason::NoDirectory)
    );
    assert_eq!(
        entries(root.path()),
        Vec::<PathBuf>::new(),
        "nothing was created"
    );
    assert_eq!(store.load(session()), LoadOutcome::None);

    // The launcher made it on the host.
    std::fs::create_dir(&dir).unwrap();
    assert_eq!(
        store.save(session(), &saved).unwrap(),
        SaveOutcome::Saved,
        "the skipped save was not counted as written"
    );
    assert_eq!(store.load(session()), LoadOutcome::History(saved));
}

// Data-model §5: the daemon holds one store in an `Arc` and saves on the blocking pool.
#[test]
fn a_store_is_shared_between_threads() {
    fn shared<T: Send + Sync + 'static>() {}
    shared::<HistoryStore>();

    let root = tempfile::tempdir().unwrap();
    let store = std::sync::Arc::new(store_in(root.path()));
    let saved = snapshot(&["from another thread"]);
    let saver = {
        let (store, saved) = (store.clone(), saved.clone());
        std::thread::spawn(move || store.save(session(), &saved).unwrap())
    };

    assert_eq!(saver.join().unwrap(), SaveOutcome::Saved);
    assert_eq!(store.load(session()), LoadOutcome::History(saved));
}

#[cfg(unix)]
mod unix {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::time::Duration;

    use super::*;

    fn mode(path: &Path) -> u32 {
        std::fs::metadata(path).unwrap().mode() & 0o7777
    }

    fn set_mode(path: &Path, mode: u32) {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    // U47 (story 3 scenario 3).
    #[test]
    fn a_file_that_cannot_be_opened_loads_as_unreadable() {
        // SAFETY: `geteuid` reads the process's credentials and cannot fail.
        if unsafe { libc::geteuid() } == 0 {
            return; // root reads a file of mode 000
        }
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save(session(), &snapshot(&["locked away"])).unwrap();
        set_mode(&store.dir().join(SESSION_FILE), 0o000);

        assert_eq!(
            store.load(session()),
            LoadOutcome::Damaged(DamageReason::Unreadable(
                std::io::ErrorKind::PermissionDenied
            ))
        );
    }

    // U51 (FR-004): nothing touches the file.
    #[test]
    fn an_unchanged_save_leaves_the_file_as_it_is() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        let saved = snapshot(&["nothing new"]);
        store.save(session(), &saved).unwrap();
        let path = store.dir().join(SESSION_FILE);
        let before = std::fs::metadata(&path).unwrap();
        // Longer than the coarsest modification-time step a rewrite could hide in.
        std::thread::sleep(Duration::from_millis(20));

        assert_eq!(
            store.save(session(), &saved).unwrap(),
            SaveOutcome::Unchanged
        );

        let after = std::fs::metadata(&path).unwrap();
        assert_eq!(after.ino(), before.ino(), "the same file under the name");
        assert_eq!(
            (after.mtime(), after.mtime_nsec()),
            (before.mtime(), before.mtime_nsec()),
            "not written to"
        );
        assert_eq!(entries(store.dir()), [path]);
    }

    // U54, on Linux and macOS. The per-user directories are redirected first: this is the one
    // test of this file that resolves them, and no other reads the two variables.
    #[test]
    fn the_history_directory_is_terminal_history_under_the_local_data_directory() {
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("XDG_DATA_HOME", home.path());
        std::env::set_var("HOME", home.path());
        let data_local_dir = if cfg!(target_os = "macos") {
            home.path()
                .join("Library/Application Support/micold-ai-ide")
        } else {
            home.path().join("micold-ai-ide")
        };

        assert_eq!(history_dir(), Some(data_local_dir.join("terminal-history")));
    }

    // U53 (FR-020, SC-009).
    #[test]
    fn the_directory_is_0700_and_the_file_0600() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());

        store.save(session(), &snapshot(&["private"])).unwrap();

        assert_eq!(mode(store.dir()), 0o700);
        assert_eq!(mode(&store.dir().join(SESSION_FILE)), 0o600);
    }
}

// U54 (FR-019): the roaming profile is copied between machines; a terminal's history stays on the
// one it was written on. Windows resolves both folders from the user's profile, not from a
// variable a test could redirect; nothing is created here.
#[cfg(windows)]
#[test]
fn on_windows_the_history_directory_is_local_and_not_in_the_roaming_profile() {
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let roaming = PathBuf::from(std::env::var_os("APPDATA").unwrap());

    assert_eq!(
        history_dir(),
        Some(local.join("micold-ai-ide\\data\\terminal-history")),
        "under the local data directory"
    );
    assert!(
        !history_dir().unwrap().starts_with(&roaming),
        "not under the roaming data directory {roaming:?}"
    );
}

// ---------------------------------------------------------------------------------------
// Feature 041 M5 — the setting (T039; U92–U97; data-model §5, contracts/setting.md §3)
// ---------------------------------------------------------------------------------------

/// A store with two sessions saved.
fn two_saved(root: &Path) -> (HistoryStore, PathBuf) {
    let store = store_in(root);
    store.save(session(), &snapshot(&["one"])).unwrap();
    store.save(other_session(), &snapshot(&["two"])).unwrap();
    (store, root.join("terminal-history"))
}

// U92.
#[test]
fn with_saving_off_a_save_is_skipped_and_a_load_finds_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    assert!(store.set_enabled(false).is_empty());

    assert_eq!(
        store.save(session(), &snapshot(&["secret"])).unwrap(),
        SaveOutcome::Skipped(SkipReason::Disabled)
    );
    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
    assert_eq!(store.load(session()), LoadOutcome::None);
    assert!(!store.enabled());
}

// U93.
#[test]
fn turning_saving_off_deletes_every_saved_and_temporary_file() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    std::fs::write(dir.join(format!(".{SESSION_FILE}.tmp")), b"half").unwrap();

    let failures = store.set_enabled(false);

    assert!(failures.is_empty(), "none left: {failures:?}");
    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
}

// U94, U95.
#[cfg(unix)]
#[test]
fn a_file_that_cannot_be_deleted_is_reported_retried_and_never_loaded() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    let locked = |on: bool| {
        let mode = if on { 0o500 } else { 0o700 };
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode)).unwrap();
    };
    locked(true);
    if std::fs::File::create(dir.join("probe")).is_ok() {
        // Running as root: a read-only directory stops nothing, so there is nothing to test.
        locked(false);
        return;
    }

    let failures = store.set_enabled(false);
    assert_eq!(failures.len(), 2, "both files stay: {failures:?}");
    assert!(failures.iter().any(|f| f.session == Some(session())));
    assert_eq!(store.retry_deletions().len(), 2, "still not deletable");

    // Turned on again: the file is still there, and is not shown.
    assert!(store.set_enabled(true).is_empty());
    assert_eq!(store.load(session()), LoadOutcome::None);
    assert_eq!(entries(&dir).len(), 2);

    locked(false);
    assert!(store.retry_deletions().is_empty());
    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
}

// U95: a save replaces the file whose deletion failed and it is shown again.
#[cfg(unix)]
#[test]
fn a_new_save_replaces_a_file_whose_deletion_failed() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o500)).unwrap();
    let rooted = std::fs::File::create(dir.join("probe")).is_ok();
    let failures = store.set_enabled(false);
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    if rooted {
        return;
    }
    assert_eq!(failures.len(), 2);
    store.set_enabled(true);

    let fresh = snapshot(&["after"]);
    assert_eq!(store.save(session(), &fresh).unwrap(), SaveOutcome::Saved);

    assert_eq!(store.load(session()), LoadOutcome::History(fresh));
    // The other session's file is still to delete, and now can be; this session's is kept.
    assert!(store.retry_deletions().is_empty());
    assert_eq!(entries(&dir).len(), 1);
    assert_eq!(
        store.load(session()),
        LoadOutcome::History(snapshot(&["after"]))
    );
}

// U96.
#[test]
fn purge_deletes_everything_and_leaves_saving_off() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());

    assert!(store.purge().is_empty());

    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
    assert!(!store.enabled());
}

#[test]
fn turning_saving_on_restores_nothing() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    store.set_enabled(false);
    store.set_enabled(true);

    assert_eq!(store.load(session()), LoadOutcome::None);
    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
    // An equal snapshot is written again: the record of the last write went with the files.
    assert_eq!(
        store.save(session(), &snapshot(&["one"])).unwrap(),
        SaveOutcome::Saved
    );
}

// A directory that could not be listed when saving went off is deleted from when it goes on.
#[cfg(unix)]
#[test]
fn turning_saving_on_after_an_unlistable_directory_deletes_what_is_there() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    let set = |mode: u32| std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(mode));
    set(0o000).unwrap();
    let rooted = std::fs::read_dir(&dir).is_ok();
    let failures = store.set_enabled(false);
    set(0o700).unwrap();
    if rooted {
        return; // running as root: nothing stops the listing
    }
    assert_eq!(failures.len(), 1, "the directory could not be listed");
    assert_eq!(entries(&dir).len(), 2, "nothing was deleted yet");

    assert!(store.set_enabled(true).is_empty());

    assert_eq!(
        entries(&dir),
        Vec::<PathBuf>::new(),
        "turning saving on deletes what could not be listed before"
    );
    assert_eq!(store.load(session()), LoadOutcome::None);
}

// U97 (FR-033): a save racing the setting being turned off leaves no file.
#[test]
fn a_save_racing_turning_saving_off_leaves_no_file() {
    for round in 0..50 {
        let root = tempfile::tempdir().unwrap();
        let store = std::sync::Arc::new(store_in(root.path()));
        let dir = root.path().join("terminal-history");
        let saver = {
            let store = std::sync::Arc::clone(&store);
            std::thread::spawn(move || {
                for n in 0..20 {
                    let _ = store.save(session(), &snapshot(&[&format!("{round}-{n}")]));
                }
            })
        };
        let failures = store.set_enabled(false);
        saver.join().unwrap();
        assert!(failures.is_empty());
        let left: Vec<_> = if dir.is_dir() { entries(&dir) } else { vec![] };
        assert_eq!(left, Vec::<PathBuf>::new(), "round {round}");
    }
}

// FR-033: a directory that cannot be listed when saving is turned off keeps its files from
// being shown, and they are deleted once it can be listed.
#[cfg(unix)]
#[test]
fn an_unlistable_directory_is_retried_and_its_files_are_never_loaded() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o000)).unwrap();
    if std::fs::read_dir(&dir).is_ok() {
        // Running as root: nothing stops the listing, so there is nothing to test.
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
        return;
    }

    let failures = store.set_enabled(false);
    assert_eq!(
        failures.len(),
        1,
        "one failure for the directory: {failures:?}"
    );
    assert_eq!(store.retry_deletions().len(), 1, "still not listable");
    assert!(store.set_enabled(true).len() <= 1);
    assert_eq!(store.load(session()), LoadOutcome::None);

    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700)).unwrap();
    store.set_enabled(false);
    assert!(store.retry_deletions().is_empty());
    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
}

// ---------------------------------------------------------------------------------------
// Feature 041 M7 — removal (T056; U108–U111; data-model §5, FR-023, FR-024)
// ---------------------------------------------------------------------------------------

fn ids(list: &[SessionId]) -> std::collections::HashSet<SessionId> {
    list.iter().copied().collect()
}

// U108.
#[test]
fn forget_deletes_the_files_of_those_sessions_and_no_other() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    std::fs::write(dir.join(format!(".{SESSION_FILE}.tmp")), b"half").unwrap();

    assert!(store.forget(&[session()]).is_empty());

    assert_eq!(entries(&dir), vec![dir.join(OTHER_SESSION_FILE)]);
    assert_eq!(store.load(session()), LoadOutcome::None);
    assert!(matches!(
        store.load(other_session()),
        LoadOutcome::History(_)
    ));
}

// U109.
#[test]
fn a_save_for_a_forgotten_session_is_skipped_and_leaves_no_file() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    store.forget(&[session()]);

    assert_eq!(
        store.save(session(), &snapshot(&["late"])).unwrap(),
        SaveOutcome::Skipped(SkipReason::Forgotten)
    );
    assert_eq!(entries(&dir), vec![dir.join(OTHER_SESSION_FILE)]);
}

// U109 (story 4 scenario 5): a save that began before the forget, on another thread.
#[test]
fn a_save_racing_a_forget_leaves_no_file() {
    for round in 0..50 {
        let root = tempfile::tempdir().unwrap();
        let store = std::sync::Arc::new(store_in(root.path()));
        let dir = root.path().join("terminal-history");
        let saver = {
            let store = std::sync::Arc::clone(&store);
            std::thread::spawn(move || {
                for n in 0..20 {
                    let _ = store.save(session(), &snapshot(&[&format!("{round}-{n}")]));
                }
            })
        };
        assert!(store.forget(&[session()]).is_empty());
        saver.join().unwrap();
        let left: Vec<_> = if dir.is_dir() { entries(&dir) } else { vec![] };
        assert_eq!(left, Vec::<PathBuf>::new(), "round {round}");
    }
}

// U110.
#[test]
fn forget_deletes_while_saving_is_off() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    // A file left by a failed deletion: the store is off, and the file is still there.
    store.set_enabled(false);
    std::fs::write(dir.join(SESSION_FILE), b"left").unwrap();

    assert!(store.forget(&[session()]).is_empty());

    assert_eq!(entries(&dir), Vec::<PathBuf>::new());
}

#[test]
fn forget_drops_the_record_of_the_last_write_of_a_session_it_does_not_forget_for_good() {
    let root = tempfile::tempdir().unwrap();
    let (store, _dir) = two_saved(root.path());
    // The other session's record stays: an equal save is still unchanged.
    store.forget(&[session()]);
    assert_eq!(
        store.save(other_session(), &snapshot(&["two"])).unwrap(),
        SaveOutcome::Unchanged
    );
}

// U111.
#[test]
fn sweep_keeps_the_listed_sessions_and_deletes_everything_else() {
    let root = tempfile::tempdir().unwrap();
    let (store, dir) = two_saved(root.path());
    store
        .save(
            SessionId::from_uuid(Uuid::from_u128(7)),
            &snapshot(&["unknown"]),
        )
        .unwrap();
    std::fs::write(dir.join(format!(".{SESSION_FILE}.tmp")), b"half").unwrap();
    std::fs::write(dir.join("notes.txt"), b"other").unwrap();
    std::fs::write(dir.join("not-a-uuid.history"), b"other").unwrap();

    let failures = store.sweep(&ids(&[session()]));

    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(entries(&dir), vec![dir.join(SESSION_FILE)]);
    assert!(matches!(store.load(session()), LoadOutcome::History(_)));
}

#[test]
fn sweep_of_a_directory_that_is_not_there_does_nothing() {
    let root = tempfile::tempdir().unwrap();
    assert!(store_in(root.path()).sweep(&ids(&[])).is_empty());
}

// ---- The events file kind (feature 938, M7; research R14): `<uuid>.events` beside `.history`.

mod events_kind {
    use super::*;
    use micold_core::structured::{encode_events, parse_line, SequencedEvent, StructuredEvent};
    use micold_core::terminal_history::EventsLoad;

    const EVENTS_FILE: &str = "0a1b2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d.events";
    const OTHER_EVENTS_FILE: &str = "ffff2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d.events";

    fn history(texts: &[&str]) -> Vec<SequencedEvent> {
        texts
            .iter()
            .enumerate()
            .map(|(i, text)| SequencedEvent {
                seq: 4 + i as u64,
                event: StructuredEvent::Unreadable {
                    text: (*text).to_owned(),
                },
            })
            .collect()
    }

    fn parsed(events: &[SequencedEvent]) -> Vec<StructuredEvent> {
        events.iter().map(|e| parse_line_of(&e.event)).collect()
    }

    fn parse_line_of(event: &StructuredEvent) -> StructuredEvent {
        match event {
            StructuredEvent::Unreadable { text } => parse_line(text),
            other => other.clone(),
        }
    }

    #[test]
    fn saved_events_load_from_the_events_file_named_after_the_session() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        let saved = history(&["a", "b"]);
        assert_eq!(
            store.save_events(session(), 4, &saved).unwrap(),
            SaveOutcome::Saved
        );
        assert_eq!(entries(store.dir()), [store.dir().join(EVENTS_FILE)]);
        assert_eq!(
            std::fs::read(store.dir().join(EVENTS_FILE)).unwrap(),
            encode_events(4, &saved)
        );
        assert_eq!(
            store.load_events(session()),
            EventsLoad::Events {
                omitted: 4,
                events: parsed(&saved)
            }
        );
        assert_eq!(store.load(session()), LoadOutcome::None);
    }

    #[test]
    fn the_same_bytes_again_are_unchanged() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        let saved = history(&["a"]);
        store.save_events(session(), 4, &saved).unwrap();
        assert_eq!(
            store.save_events(session(), 4, &saved).unwrap(),
            SaveOutcome::Unchanged
        );
        assert_eq!(
            store
                .save_events(session(), 4, &history(&["a", "b"]))
                .unwrap(),
            SaveOutcome::Saved
        );
    }

    #[test]
    fn a_history_and_an_events_save_do_not_mark_each_other_unchanged() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_events(session(), 0, &[]).unwrap();
        assert_eq!(
            store.save(session(), &snapshot(&["x"])).unwrap(),
            SaveOutcome::Saved
        );
        assert_eq!(
            store.save(session(), &snapshot(&["x"])).unwrap(),
            SaveOutcome::Unchanged
        );
        assert_eq!(
            store.save_events(session(), 0, &[]).unwrap(),
            SaveOutcome::Unchanged
        );
        assert_eq!(entries(store.dir()).len(), 2);
    }

    #[test]
    fn a_save_is_skipped_when_off_forgotten_or_with_no_directory() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path()).with_enabled(false);
        assert_eq!(
            store.save_events(session(), 0, &[]).unwrap(),
            SaveOutcome::Skipped(SkipReason::Disabled)
        );
        let store = store_in(root.path());
        store.forget(&[session()]);
        assert_eq!(
            store.save_events(session(), 0, &[]).unwrap(),
            SaveOutcome::Skipped(SkipReason::Forgotten)
        );
        let absent = HistoryStore::new(root.path().join("absent"), false);
        assert_eq!(
            absent.save_events(session(), 0, &[]).unwrap(),
            SaveOutcome::Skipped(SkipReason::NoDirectory)
        );
    }

    #[test]
    fn a_load_is_none_when_off_forgotten_or_with_no_file() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        assert_eq!(store.load_events(session()), EventsLoad::None);
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        store.set_enabled(false);
        store.set_enabled(true);
        assert_eq!(store.load_events(session()), EventsLoad::None, "deleted");
        store.save_events(other_session(), 0, &[]).unwrap();
        store.forget(&[other_session()]);
        assert_eq!(store.load_events(other_session()), EventsLoad::None);
    }

    #[test]
    fn a_damaged_file_loads_as_damaged_with_its_reason() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        let path = store.dir().join(EVENTS_FILE);
        let mut bytes = std::fs::read(&path).unwrap();
        let end = bytes.len() - 1;
        bytes[end] ^= 1;
        std::fs::write(&path, &bytes).unwrap();
        assert_eq!(
            store.load_events(session()),
            EventsLoad::Damaged(DamageReason::Checksum)
        );
        std::fs::write(&path, encode(&snapshot(&["x"]))).unwrap();
        assert_eq!(
            store.load_events(session()),
            EventsLoad::Damaged(DamageReason::NotAHistory)
        );
        assert_eq!(
            store
                .save_events(session(), 0, &history(&["a", "b"]))
                .unwrap(),
            SaveOutcome::Saved,
            "the next save replaces it"
        );
    }

    #[test]
    fn turning_saving_off_and_purge_delete_events_and_their_temporary_files() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        store.save(other_session(), &snapshot(&["x"])).unwrap();
        std::fs::write(store.dir().join(format!(".{EVENTS_FILE}.tmp")), b"x").unwrap();
        assert!(store.set_enabled(false).is_empty());
        assert!(entries(store.dir()).is_empty());
        store.set_enabled(true);
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        std::fs::write(store.dir().join(format!(".{EVENTS_FILE}.tmp")), b"x").unwrap();
        assert!(store.purge().is_empty());
        assert!(entries(store.dir()).is_empty());
    }

    #[test]
    fn forget_deletes_both_kinds_of_the_session_and_no_other() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        store.save(session(), &snapshot(&["x"])).unwrap();
        std::fs::write(store.dir().join(format!(".{EVENTS_FILE}.tmp")), b"x").unwrap();
        store
            .save_events(other_session(), 0, &history(&["a"]))
            .unwrap();
        assert!(store.forget(&[session()]).is_empty());
        assert_eq!(entries(store.dir()), [store.dir().join(OTHER_EVENTS_FILE)]);
    }

    #[test]
    fn sweep_keeps_the_events_of_kept_sessions_and_deletes_the_rest() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        store
            .save_events(other_session(), 0, &history(&["a"]))
            .unwrap();
        std::fs::write(store.dir().join(format!(".{EVENTS_FILE}.tmp")), b"x").unwrap();
        assert!(store.sweep(&ids(&[session()])).is_empty());
        assert_eq!(entries(store.dir()), [store.dir().join(EVENTS_FILE)]);
    }

    #[test]
    fn a_save_for_a_forgotten_session_after_a_racing_forget_leaves_no_events_file() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.forget(&[session()]);
        store.save_events(session(), 0, &history(&["a"])).unwrap();
        assert!(!store.dir().join(EVENTS_FILE).exists());
    }
}

// ---- The queue file kind (feature 941, research R9): `<uuid>.queue` beside `.events`.

mod queue_kind {
    use super::*;
    use micold_core::structured::attachment::admit;
    use micold_core::structured::{
        encode_queue, Message, MAX_QUEUE_FILE_BYTES, QUEUE_FORMAT_VERSION,
    };
    use micold_core::terminal_history::{DamageReason, QueueLoad};

    const QUEUE_FILE: &str = "0a1b2c3d-4e5f-4a6b-8c7d-9e0f1a2b3c4d.queue";

    fn queue() -> Vec<Message> {
        let picture = admit(&[], "shot.png", b"\x89PNG\r\n\x1a\nbytes".to_vec()).unwrap();
        vec![
            Message {
                text: "first\nwith two lines".into(),
                images: vec![picture],
            },
            Message {
                text: "second".into(),
                images: vec![],
            },
        ]
    }

    #[test]
    fn a_saved_queue_loads_back_with_its_pictures_from_the_queue_file() {
        assert_eq!(QUEUE_FORMAT_VERSION, 1);
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        assert_eq!(
            store.save_queue(session(), &queue()).unwrap(),
            SaveOutcome::Saved
        );
        assert_eq!(entries(store.dir()), [store.dir().join(QUEUE_FILE)]);
        assert_eq!(store.load_queue(session()), QueueLoad::Queue(queue()));
        assert_eq!(store.load_queue(other_session()), QueueLoad::None);
    }

    #[test]
    fn no_file_is_no_queue() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(store_in(root.path()).load_queue(session()), QueueLoad::None);
    }

    #[test]
    fn saving_an_empty_queue_removes_the_file() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_queue(session(), &queue()).unwrap();
        store.save_queue(session(), &[]).unwrap();
        assert!(entries(store.dir()).is_empty());
        assert_eq!(store.load_queue(session()), QueueLoad::None);
        // And the same queue again is written again, not "unchanged".
        assert_eq!(
            store.save_queue(session(), &queue()).unwrap(),
            SaveOutcome::Saved
        );
    }

    #[test]
    fn another_version_a_truncated_file_and_an_oversized_one_are_damaged() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_queue(session(), &queue()).unwrap();
        let path = store.dir().join(QUEUE_FILE);
        let good = std::fs::read(&path).unwrap();

        let mut other_version = good.clone();
        other_version[8..12].copy_from_slice(&2u32.to_le_bytes());
        std::fs::write(&path, &other_version).unwrap();
        assert!(matches!(
            store.load_queue(session()),
            QueueLoad::Damaged(DamageReason::OtherVersion(2))
        ));

        std::fs::write(&path, &good[..good.len() - 5]).unwrap();
        assert!(matches!(store.load_queue(session()), QueueLoad::Damaged(_)));

        let file = std::fs::File::create(&path).unwrap();
        file.set_len(MAX_QUEUE_FILE_BYTES + 1).unwrap();
        drop(file);
        assert_eq!(
            store.load_queue(session()),
            QueueLoad::Damaged(DamageReason::TooLarge)
        );
    }

    #[test]
    fn a_picture_that_is_not_its_format_makes_the_file_damaged() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        let mut bytes = encode_queue(&queue());
        // The PNG's first magic byte, inside the payload: the picture no longer sniffs as PNG.
        let at = bytes
            .windows(4)
            .position(|w| w == b"\x89PNG")
            .expect("the picture's bytes");
        bytes[at] = b'x';
        std::fs::create_dir_all(store.dir()).unwrap();
        // Re-frame with a valid checksum, so only the picture is wrong.
        let body = bytes.len() - 32;
        let checksum = micold_core::protocol::hashing::sha256(&bytes[..body]);
        bytes[body..].copy_from_slice(&checksum);
        std::fs::write(store.dir().join(QUEUE_FILE), &bytes).unwrap();
        assert_eq!(
            store.load_queue(session()),
            QueueLoad::Damaged(DamageReason::Malformed)
        );
    }

    #[test]
    fn removing_the_session_removes_its_queue_with_its_history_files() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_queue(session(), &queue()).unwrap();
        store.save(session(), &snapshot(&["a"])).unwrap();
        assert!(store.forget(&[session()]).is_empty());
        assert!(entries(store.dir()).is_empty());
        assert_eq!(store.load_queue(session()), QueueLoad::None);
    }

    #[test]
    fn the_sweep_keeps_a_kept_sessions_queue() {
        let root = tempfile::tempdir().unwrap();
        let store = store_in(root.path());
        store.save_queue(session(), &queue()).unwrap();
        store.save_queue(other_session(), &queue()).unwrap();
        assert!(store.sweep(&ids(&[session()])).is_empty());
        assert_eq!(entries(store.dir()), [store.dir().join(QUEUE_FILE)]);
    }
}
