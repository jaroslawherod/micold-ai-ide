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

    std::fs::write(&path, b"not a history at all, only some text of a fair length").unwrap();
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

// U54 (FR-019), the part every platform has.
#[test]
fn the_history_directory_is_terminal_history_under_the_local_data_directory() {
    let dirs = directories::ProjectDirs::from("", "", "micold-ai-ide").unwrap();

    assert_eq!(
        history_dir(),
        Some(dirs.data_local_dir().join("terminal-history"))
    );
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
// one it was written on.
#[cfg(windows)]
#[test]
fn on_windows_the_history_directory_is_not_in_the_roaming_profile() {
    let dirs = directories::ProjectDirs::from("", "", "micold-ai-ide").unwrap();
    let dir = history_dir().unwrap();

    assert!(
        dir.starts_with(dirs.data_local_dir()),
        "{dir:?} is under the local data directory"
    );
    assert!(
        !dir.starts_with(dirs.data_dir()),
        "{dir:?} is not under the roaming data directory"
    );
}
