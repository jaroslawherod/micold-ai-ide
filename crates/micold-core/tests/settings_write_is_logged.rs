//! Every settings write leaves a record naming who wrote it and what document they merged into
//! (T162, BUG-025).
//!
//! The reset in that report had to be attributed by elimination from the bytes left on disk:
//! neither `micold-client.log` nor `micold-daemon.log` records that a settings write happened at
//! all, so "which of the two writers did this, and what did it think it was merging into?" had no
//! answer anywhere. Both halves matter — the writer alone does not distinguish a normal save from
//! one that laid its fields over defaults, and that distinction is the whole of the second arm.
//!
//! Asserted here, in the library, for the reason `attach_log_line` is: a diagnostic nothing tests
//! is a diagnostic that quietly stops being written, which is the failure this task exists to fix.

use micold_core::settings::{JsonFileSettingsStore, Settings, SettingsStore};
use micold_core::store::LoadStatus;

#[test]
fn a_normal_write_records_the_writer_the_process_and_a_loaded_base() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path);
    store.save(&Settings::default()).unwrap();

    let write = store.update_reporting(&mut |settings| settings.scrollback_lines = 12_345);

    assert_eq!(write.base, LoadStatus::Loaded);
    assert!(write.result.is_ok());

    let line = write.log_line("client");
    assert!(line.contains("writer=client"), "{line}");
    assert!(
        line.contains(&format!("pid={}", std::process::id())),
        "the line does not say which process wrote: {line}"
    );
    assert!(line.contains("base=loaded"), "{line}");
    assert!(line.contains("result=ok"), "{line}");
}

#[test]
fn a_first_run_write_says_its_base_was_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = JsonFileSettingsStore::at(dir.path().join("settings.json"));

    let write = store.update_reporting(&mut |settings| settings.scrollback_lines = 12_345);

    assert_eq!(write.base, LoadStatus::Missing);
    let line = write.log_line("daemon");
    assert!(line.contains("writer=daemon"), "{line}");
    assert!(
        line.contains("base=missing"),
        "a write over defaults must say so — that is the line that would have named the second \
         arm of BUG-025 on the day it happened: {line}"
    );
}

#[cfg(unix)]
#[test]
fn a_refused_write_is_logged_as_a_failure_with_the_reason() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let store = JsonFileSettingsStore::at(path.clone());
    store.save(&Settings::default()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

    let write = store.update_reporting(&mut |settings| settings.scrollback_lines = 12_345);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();

    assert!(write.result.is_err());
    let line = write.log_line("client");
    assert!(line.contains("result=failed"), "{line}");
    assert!(
        line.contains("could not be read"),
        "a refusal must log why, or it looks like an ordinary failed write: {line}"
    );
}
