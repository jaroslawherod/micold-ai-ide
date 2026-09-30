//! Spec 035: the stored environment-include script path is checked, never run
//! (contracts/script-path-check.md, rows C1–C8 and P1–P8).
//!
//! `classify` is tested with [`FakeScriptPathProbe`], so each row states both what was found and
//! whether anything was examined at all. `StdScriptPathProbe` is tested on real temp files.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use micold_core::script_path_check::{
    check_bounded, classify, FakeScriptPathProbe, ProbeAnswer, ScriptPathProbe, ScriptPathState,
    StdScriptPathProbe, SCRIPT_PATH_CHECK_BOUND,
};

/// An absolute path on every OS (`/tmp/...` is not absolute on Windows), naming nothing.
fn absolute_script() -> PathBuf {
    std::env::temp_dir().join("micold-035-env.sh")
}

// --- classify: C1, blank ---------------------------------------------------------------------

#[test]
fn an_empty_path_has_no_check_and_examines_nothing() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);

    assert_eq!(
        classify("", &probe),
        None,
        "a blank path is not a missing script: there is nothing to report (FR-011)"
    );
    assert!(probe.calls().is_empty(), "a blank path must not be probed");
}

#[test]
fn a_whitespace_only_path_has_no_check_and_examines_nothing() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);

    assert_eq!(
        classify("   ", &probe),
        None,
        "whitespace is blank, as 011's resolver treats it (FR-011)"
    );
    assert!(probe.calls().is_empty(), "a blank path must not be probed");
}

// --- classify: C2, `~` -----------------------------------------------------------------------

#[test]
fn a_tilde_path_is_not_found_without_a_probe() {
    for path in ["~", "~/env.sh"] {
        let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);

        assert_eq!(
            classify(path, &probe),
            Some(ScriptPathState::NotFound { tilde: true }),
            "{path:?}: resolution does not expand ~, so the path as written names nothing"
        );
        assert!(
            probe.calls().is_empty(),
            "{path:?}: a ~ path is reported from the string alone"
        );
    }
}

#[test]
fn a_backslash_tilde_path_is_not_found_on_every_os() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);

    assert_eq!(
        classify("~\\env.ps1", &probe),
        Some(ScriptPathState::NotFound { tilde: true }),
        "~\\ is a string test, so a Windows-style ~ path reads the same on Unix (FR-012)"
    );
    assert!(probe.calls().is_empty());
}

#[test]
fn a_name_that_merely_starts_with_a_tilde_is_relative() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);

    assert_eq!(
        classify("~env.sh", &probe),
        Some(ScriptPathState::Relative),
        "~env.sh is a file named ~env.sh, not a home-directory path"
    );
    assert!(
        probe.calls().is_empty(),
        "a relative path must not be probed"
    );
}

// --- classify: C3, relative ------------------------------------------------------------------

#[test]
fn a_relative_path_is_not_checked() {
    for path in ["env.sh", "./env.sh", "scripts/env.sh"] {
        let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);

        assert_eq!(
            classify(path, &probe),
            Some(ScriptPathState::Relative),
            "{path:?}: its answer depends on each session's directory"
        );
        assert!(
            probe.calls().is_empty(),
            "{path:?}: a relative path must not be probed"
        );
    }
}

// --- classify: C4–C6, absolute ---------------------------------------------------------------

#[test]
fn an_absolute_path_that_is_a_file_is_present_and_probed_once_as_stored() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::File);
    let path = absolute_script();

    assert_eq!(
        classify(path.to_str().expect("utf-8 temp dir"), &probe),
        Some(ScriptPathState::Present)
    );
    assert_eq!(
        probe.calls(),
        vec![path],
        "the path is probed once, exactly as stored: no trim, no expansion"
    );
}

#[test]
fn an_absolute_path_with_nothing_there_is_not_found() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::Missing);

    assert_eq!(
        classify(absolute_script().to_str().expect("utf-8"), &probe),
        Some(ScriptPathState::NotFound { tilde: false })
    );
}

#[test]
fn an_absolute_path_that_is_not_a_file_is_not_readable() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::NotAFile);

    assert_eq!(
        classify(absolute_script().to_str().expect("utf-8"), &probe),
        Some(ScriptPathState::NotReadable),
        "a directory exists but cannot be sourced"
    );
}

#[test]
fn an_absolute_path_that_cannot_be_opened_is_not_readable() {
    let probe = FakeScriptPathProbe::answering(ProbeAnswer::Unreadable);

    assert_eq!(
        classify(absolute_script().to_str().expect("utf-8"), &probe),
        Some(ScriptPathState::NotReadable),
        "a fast error other than absence is never reported as present or as missing"
    );
}

// --- StdScriptPathProbe: P1–P8, on real files -------------------------------------------------

#[test]
fn a_regular_file_answers_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("env.sh");
    std::fs::write(&script, "export A=1\n").expect("write");

    assert_eq!(StdScriptPathProbe.probe(&script), ProbeAnswer::File);
}

#[test]
fn nothing_at_the_path_answers_missing() {
    let dir = tempfile::tempdir().expect("tempdir");

    assert_eq!(
        StdScriptPathProbe.probe(&dir.path().join("no-such-script.sh")),
        ProbeAnswer::Missing
    );
}

#[test]
fn a_directory_answers_not_a_file() {
    let dir = tempfile::tempdir().expect("tempdir");

    assert_eq!(
        StdScriptPathProbe.probe(dir.path()),
        ProbeAnswer::NotAFile,
        "a directory exists, and still cannot be sourced"
    );
}

#[cfg(unix)]
#[test]
fn a_file_the_user_cannot_open_answers_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let script = dir.path().join("env.sh");
    std::fs::write(&script, "export A=1\n").expect("write");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    if std::fs::File::open(&script).is_ok() {
        // Running as root: mode bits do not stop the open, so there is no unreadable file to test.
        eprintln!(
            "SKIPPED a_file_the_user_cannot_open_answers_unreadable: running as root, mode 000 \
             does not stop the open"
        );
        return;
    }

    assert_eq!(
        StdScriptPathProbe.probe(&script),
        ProbeAnswer::Unreadable,
        "readable is what the OS reports for the current user, not what the mode bits say"
    );
}

#[cfg(unix)]
#[test]
fn a_file_under_a_directory_without_search_permission_answers_unreadable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let locked = dir.path().join("locked");
    std::fs::create_dir(&locked).expect("mkdir");
    let script = locked.join("env.sh");
    std::fs::write(&script, "export A=1\n").expect("write");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).expect("chmod");
    // Restores search permission however the test ends, so the temp dir can be removed.
    struct Unlock<'a>(&'a std::path::Path);
    impl Drop for Unlock<'_> {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(self.0, std::fs::Permissions::from_mode(0o700));
        }
    }
    let _unlock = Unlock(&locked);
    if std::fs::metadata(&script).is_ok() {
        // Running as root: the parent's mode does not stop the lookup.
        eprintln!(
            "SKIPPED a_file_under_a_directory_without_search_permission_answers_unreadable: \
             running as root, the parent's mode does not stop the lookup"
        );
        return;
    }

    let answer = StdScriptPathProbe.probe(&script);

    assert_eq!(
        answer,
        ProbeAnswer::Unreadable,
        "a permissions error is not absence: the file may well be there"
    );
}

#[test]
fn probing_a_script_never_runs_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("ran");
    let script = dir.path().join("env.sh");
    std::fs::write(
        &script,
        format!("#!/bin/sh\ntouch '{}'\n", marker.display()),
    )
    .expect("write");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    }

    assert_eq!(StdScriptPathProbe.probe(&script), ProbeAnswer::File);
    assert!(
        !marker.exists(),
        "the check examines the script and must never run it (FR-003)"
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_regular_file_answers_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("real.sh");
    std::fs::write(&target, "export A=1\n").expect("write");
    let link = dir.path().join("env.sh");
    std::os::unix::fs::symlink(&target, &link).expect("symlink");

    assert_eq!(
        StdScriptPathProbe.probe(&link),
        ProbeAnswer::File,
        "a link is followed, as `source` follows it"
    );
}

#[cfg(unix)]
#[test]
fn a_dangling_symlink_answers_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let link = dir.path().join("env.sh");
    std::os::unix::fs::symlink(dir.path().join("gone.sh"), &link).expect("symlink");

    assert_eq!(
        StdScriptPathProbe.probe(&link),
        ProbeAnswer::Missing,
        "a link to nothing names no script"
    );
}

// --- check_bounded: C7–C8 --------------------------------------------------------------------

/// Short, so the hang case costs the suite a tenth of a second rather than the real 2 s.
const TEST_BOUND: Duration = Duration::from_millis(100);

#[test]
fn a_probe_that_never_answers_is_reported_as_unchecked_within_the_bound() {
    let probe = Arc::new(FakeScriptPathProbe::blocking());
    let path = absolute_script().to_str().expect("utf-8").to_string();

    let started = Instant::now();
    let result = check_bounded(probe, path, TEST_BOUND);

    assert_eq!(
        result,
        Some(ScriptPathState::Unchecked),
        "a hung stat must become *could not be checked*, not a wait (FR-006)"
    );
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "the check must return at the bound plus scheduling slack, took {:?}",
        started.elapsed()
    );
}

#[test]
fn a_probe_that_answers_at_once_gives_the_same_result_as_classify() {
    let path = absolute_script().to_str().expect("utf-8").to_string();
    let probe = Arc::new(FakeScriptPathProbe::answering(ProbeAnswer::Missing));

    assert_eq!(
        check_bounded(probe, path.clone(), TEST_BOUND),
        classify(&path, &FakeScriptPathProbe::answering(ProbeAnswer::Missing)),
        "an answer inside the bound is the answer"
    );
}

#[test]
fn a_bounded_check_of_a_blank_path_is_none_without_a_probe() {
    let probe = Arc::new(FakeScriptPathProbe::answering(ProbeAnswer::File));

    assert_eq!(
        check_bounded(probe.clone(), "  ".to_string(), TEST_BOUND),
        None
    );
    assert!(probe.calls().is_empty(), "a blank path must not be probed");
}

#[test]
fn the_bound_is_two_seconds() {
    assert_eq!(
        SCRIPT_PATH_CHECK_BOUND,
        Duration::from_secs(2),
        "FR-006 and SC-004 name 2 s"
    );
}
