//! Regression guard (#1526, M1a, T025/U23): `spawn_detached_daemon` adds no env beyond what the
//! test process already had — proves the "no `MICOLD_*` env" requirement holds, independent of
//! the config-delivery write step T027 adds ahead of it. Sets process env vars, so it is its own
//! test target (`tests/main.rs`'s header comment): it must never run concurrently with other tests
//! that read the daemon-binary environment.

#[test]
#[cfg(unix)]
fn spawn_detached_daemon_adds_no_env() {
    use std::io::Write as _;

    let dir = tempfile::tempdir().unwrap();
    let script_path = dir.path().join("fake-daemon.sh");

    {
        let mut f = std::fs::File::create(&script_path).unwrap();
        writeln!(f, "#!/bin/sh").unwrap();
        writeln!(f, "env > \"$SOME_OUT_PATH\"").unwrap();
        writeln!(f, "echo DONE >> \"$SOME_OUT_PATH\"").unwrap();
        writeln!(f, "sleep 5").unwrap();
    }
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o700)).unwrap();
    }

    // Both runs execute the same `/bin/sh` script, which on this system is `dash`: it drops any
    // inherited variable whose name is not a valid shell identifier (seen here with
    // `CARGO_BIN_EXE_micold-ai-ide` — the hyphen comes from the binary's own name) before it execs
    // the command inside. Comparing two dumps taken through that same filter, rather than one of
    // them against `std::env::vars()` straight from this process, is what makes the comparison
    // catch a real regression instead of that filtering artifact.
    // `MICOLD_DAEMON_BIN` is set before both runs, so it cancels out in the comparison: it is
    // this test's own override (how it makes `spawn_detached_daemon` run the script above), not
    // something the production code adds.
    std::env::set_var(micold_core::spawn::DAEMON_BIN_ENV, script_path.as_os_str());

    let baseline_out = dir.path().join("baseline.out");
    std::env::set_var("SOME_OUT_PATH", &baseline_out);
    let mut baseline_child = std::process::Command::new(&script_path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let baseline = wait_and_parse(&baseline_out);
    let _ = baseline_child.kill();
    let _ = baseline_child.wait();

    let after_out = dir.path().join("after.out");
    std::env::set_var("SOME_OUT_PATH", &after_out);
    // Deliberately not waited on: `spawn_detached_daemon` returns before the daemon it starts
    // exits, by design (`spawn.rs`'s own doc, FR-003) — this test exercises exactly that path, and
    // the short-lived script it starts here is reaped by `init` once this test process exits.
    let _pid = micold_core::spawn::spawn_detached_daemon().unwrap();
    let after = wait_and_parse(&after_out);

    std::env::remove_var(micold_core::spawn::DAEMON_BIN_ENV);
    std::env::remove_var("SOME_OUT_PATH");

    assert_eq!(after, baseline, "spawn_detached_daemon must add no env");
}

/// Polls for the `DONE` marker the script writes only after the full `env` dump landed —
/// `is_file()` alone would race the shell's write (the file can exist, opened by the redirect,
/// before `env` finishes writing it).
#[cfg(unix)]
fn wait_and_parse(out_path: &std::path::Path) -> std::collections::BTreeSet<String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut content;
    loop {
        content = std::fs::read_to_string(out_path).unwrap_or_default();
        if content.lines().any(|l| l == "DONE") || std::time::Instant::now() >= deadline {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    content
        .lines()
        .filter_map(|l| l.split_once('=').map(|(k, _)| k.to_string()))
        .collect()
}
