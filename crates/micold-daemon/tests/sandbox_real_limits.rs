//! §B.4 of the quickstart — the limits, enforced against a **real session** (FR-016).
//!
//! `argv.rs` already asserts that the right flag with the right unit is produced for each supported
//! limit, and that no flag is produced for one the runtime cannot enforce. What it cannot assert is
//! the thing the limit exists for: that a session which asks for more than its share is stopped, and
//! that stopping it does not take the daemon — or the machine — with it.
//!
//! That distinction is not pedantic. A memory limit is enforced by the kernel's OOM killer against
//! the whole cgroup, and the process it picks is *its* choice, not ours. "The flag was passed" and
//! "the runaway process died rather than the daemon" are different claims, and only the second is
//! FR-016.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_limits -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use std::time::Duration;

use sandbox_real_support::{
    cli, cli_out, input_serial, open_session, seed, start_sandbox, wait_for_accept, SandboxSpec,
    Terminal,
};

const CONTAINER: &str = "micold-limits-probe";
const NETWORK: &str = "micold-limits-probe-net";
/// Not 7727, and not any other probe's port: these tests may run beside each other.
const PORT: u16 = 17732;

/// Small enough that a single allocation passes it in well under a second, large enough that the
/// daemon and a shell fit inside it comfortably — if the daemon itself could not run in this much,
/// the test would be measuring the wrong death.
const MEMORY_MIB: u64 = 256;
/// What the session tries to allocate. Twice the cap, in one string, so there is no gradual growth
/// for the kernel to reclaim its way out of.
const ASK_MIB: u64 = 512;

/// What `cli update` applies for a memory limit, plus the swap flag `argv::budget_args` does not
/// pass.
///
/// `--memory-swap` equal to `--memory` is what makes this a *memory* limit rather than a swap
/// limit: Docker's default is twice the memory, so without it a 512 MiB allocation under a 256 MiB
/// cap succeeds — slowly, on disk — and the test proves nothing while looking like it passed.
/// The production argv does not pass it, so the failure it prevents is the test's, not the
/// application's: see the note in `evidence/us4-limits.md`.
fn memory_flags() -> Vec<String> {
    vec![
        "--memory".into(),
        format!("{MEMORY_MIB}m"),
        "--memory-swap".into(),
        format!("{MEMORY_MIB}m"),
    ]
}

#[tokio::test]
async fn sandbox_real_limits_stop_the_session_not_the_daemon() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed(&data, &project, "limits");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let daemon_log = data.join("micold-ai-ide").join("micold-daemon.log");

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &[],
    });

    // Provisioning runs `npm install` in the container, which does not fit in this cap, so the
    // daemon is provisioned first and the cap applied to the running container. That the limit is
    // fixed at creation is the next test's subject, and the one after it.
    let (mut conn, catalog) = wait_for_accept(PORT).await;
    let mut update = vec!["update".to_string()];
    update.extend(memory_flags());
    update.push(CONTAINER.to_string());
    cli(&update.iter().map(String::as_str).collect::<Vec<_>>());

    // The limit reached the container, not just the argv (FR-015) -------------------------------
    let enforced = cli_out(&["inspect", "-f", "{{.HostConfig.Memory}}", CONTAINER]);
    assert_eq!(
        enforced,
        (MEMORY_MIB * 1024 * 1024).to_string(),
        "the runtime must report the limit it was asked for, in bytes"
    );

    let serial = input_serial(&catalog, session);
    let screen = open_session(&mut conn, &project, session, &daemon_log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &daemon_log, serial);

    // A session cannot exceed the memory it was given (FR-016) ----------------------------------
    //
    // Perl rather than a shell loop: one allocation of a known size, refused or killed at once,
    // instead of a growth curve whose timing depends on the machine.
    //
    // `$x`, not `\$x`. The daemon writes the typed bytes to a PTY and the shell is the one
    // expanding, but the program is inside single quotes, where `sh` expands nothing — so the
    // escape did not protect `$x`, it handed perl a literal backslash, and `\$x = ...` is a fatal
    // compile error. This passed for the whole of feature 027 because perl *constant-folds*
    // `"A" x 536870912` while compiling: the 512 MiB allocation happens before the error can be
    // reported, so under a working limit the process is killed and the screen shows exactly what
    // the test asks for. It surfaced the moment the limit was not enforced (T098), which is the
    // one run where this probe had to be trustworthy.
    let out = term
        .run_within(
            &format!("perl -e '$x = \"A\" x ({ASK_MIB} * 1024 * 1024); print \"ALLOCATED\\n\"'"),
            Duration::from_secs(60),
        )
        .await;
    assert!(
        !out.contains("ALLOCATED"),
        "a session allocated {ASK_MIB} MiB under a {MEMORY_MIB} MiB limit; got:\n{out}"
    );
    // Named, not merely non-empty. `!contains("ALLOCATED")` on its own is satisfied by a command
    // that never ran at all — a missing interpreter, a quoting mistake, a session that dropped the
    // input — so the probe has to show the shape of a death for its passing to mean anything.
    eprintln!("the session's account of the limit being reached:\n{out}");
    let died = out.contains("Killed") || out.to_lowercase().contains("out of memory");
    assert!(
        died,
        "the session must show the process being stopped, not merely fail to print its success \
         line — a silent absence is indistinguishable from a command that never ran; got:\n{out}"
    );

    // …and the session it happened in still works -----------------------------------------------
    let alive = term.run("echo still-here").await;
    assert!(
        alive.contains("still-here"),
        "the session must survive one of its processes being stopped; got:\n{alive}"
    );

    // …and so does the daemon, on its own connection (FR-035's premise) -------------------------
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.State.Running}}", CONTAINER]),
        "true",
        "the container must still be running after the limit was reached"
    );
    let (_second, _) = wait_for_accept(PORT).await;
}

/// The third §B.4 box: change a limit, restart the sandbox, confirm it took effect.
///
/// Restart as the application performs it — the container is *recreated*, because a limit is fixed
/// at creation and `docker start` on the old one would silently keep the old value. That is the
/// failure this asserts against: a settings change that appears to apply and does not.
#[tokio::test]
async fn sandbox_real_limits_change_only_by_recreating_the_container() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "limits-change");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    let container = "micold-limits-change";
    let network = "micold-limits-change-net";
    let port = 17733;
    let first = vec!["--memory".to_string(), "256m".to_string()];
    let sandbox = start_sandbox(&SandboxSpec {
        container,
        network,
        port,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &first,
    });
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.HostConfig.Memory}}", container]),
        (256u64 * 1024 * 1024).to_string()
    );
    drop(sandbox);

    let second = vec!["--memory".to_string(), "512m".to_string()];
    let _sandbox = start_sandbox(&SandboxSpec {
        container,
        network,
        port,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &second,
    });
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.HostConfig.Memory}}", container]),
        (512u64 * 1024 * 1024).to_string(),
        "a changed limit must be in force after the sandbox is restarted"
    );

    // The daemon still comes up under the new limit — a limit that takes effect by making the
    // sandbox unstartable is not an improvement.
    let (_conn, _) = wait_for_accept(port).await;
}

/// The profile budget of US2 #1 / SC-006: two CPUs and 4 GiB, as `argv::budget_args` passes them,
/// are what the runtime reports, and the daemon still comes up under them.
#[tokio::test]
async fn sandbox_real_limits_report_the_profiles_cpu_and_memory() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    seed(&data, &project, "limits-profile");

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    let container = "micold-limits-profile";
    let network = "micold-limits-profile-net";
    let port = 17734;
    let flags = vec![
        "--cpus".to_string(),
        "2.000".to_string(),
        "--memory".to_string(),
        "4096m".to_string(),
    ];
    let _sandbox = start_sandbox(&SandboxSpec {
        container,
        network,
        port,
        data_home: &data,
        project: &project,
        home: &home,
        survive_logout: false,
        extra: &flags,
    });
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.HostConfig.NanoCpus}}", container]),
        "2000000000"
    );
    assert_eq!(
        cli_out(&["inspect", "-f", "{{.HostConfig.Memory}}", container]),
        (4096u64 * 1024 * 1024).to_string()
    );
    let (_conn, _) = wait_for_accept(port).await;
}
