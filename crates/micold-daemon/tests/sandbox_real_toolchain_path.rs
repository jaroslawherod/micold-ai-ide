//! Feature 1289, M1 (FR-001 to FR-003, US1): in a real app-owned sandbox, a terminal's `PATH` is
//! the offered CLI bins, then the sandbox home's `.local/bin`, then mise's shims folder, then the
//! image's own `PATH`. The unit tests prove the order from fixed inputs; this proves the inputs the
//! daemon reads inside the container are the ones it should, and that the order reaches a shell.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_toolchain_path -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use sandbox_real_support::{
    input_serial, open_session, seed, start_sandbox, wait_for_accept, SandboxSpec, Terminal,
};

const CONTAINER: &str = "micold-toolchain-path-probe";
const NETWORK: &str = "micold-toolchain-path-probe-net";
const PORT: u16 = 17790;

/// Pinned for the reason the other probes pin it: an inherited `SHELL` is the developer's.
const SESSION_SHELL: &str = "/bin/sh";

/// One value per line, so a long `PATH` is never wrapped across screen lines.
async fn entries(term: &mut Terminal<'_>, var: &str) -> Vec<String> {
    term.run(&format!("printf '%s\\n' \"${var}\" | tr ':' '\\n'"))
        .await
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[tokio::test]
async fn sandbox_real_toolchain_path_puts_the_home_folders_after_the_offered_bins() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed(&data, &project, "toolchain-path-probe");
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // SAFETY: set before any spawn in this single-test binary.
    std::env::set_var("SHELL", SESSION_SHELL);

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

    let (mut conn, catalog) = wait_for_accept(PORT).await;
    let serial = input_serial(&catalog, session);
    let log = data.join("micold-ai-ide").join("micold-daemon.log");
    let screen = open_session(&mut conn, &project, session, &log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &log, serial);

    let offered =
        micold_core::provision::ai_cli::read_offered(&data.join("micold-ai-ide").join("ai-clis"))
            .expect("offered.json is readable")
            .expect("provisioning wrote offered.json");
    let bins: Vec<String> = offered
        .clis
        .iter()
        .map(|c| c.bin.to_string_lossy().into_owned())
        .collect();
    assert!(!bins.is_empty(), "provisioning offered no CLI");

    let sandbox_home = entries(&mut term, "HOME").await.join("");
    let mise = entries(&mut term, "MISE_DATA_DIR").await.join("");
    let xdg = entries(&mut term, "XDG_DATA_HOME").await.join("");
    let shims = if !mise.is_empty() {
        format!("{mise}/shims")
    } else if !xdg.is_empty() {
        format!("{xdg}/mise/shims")
    } else {
        format!("{sandbox_home}/.local/share/mise/shims")
    };
    let local_bin = format!("{sandbox_home}/.local/bin");

    let path = entries(&mut term, "PATH").await;
    let mut want: Vec<String> = bins.clone();
    want.push(local_bin.clone());
    want.push(shims.clone());
    assert_eq!(
        &path[..want.len().min(path.len())],
        &want[..],
        "the sandbox PATH does not start with the offered bins, {local_bin}, then {shims}: {path:?}"
    );
    let image_rest = &path[want.len()..];
    assert!(
        !image_rest.is_empty(),
        "the image's own PATH is missing after the home folders: {path:?}"
    );
    assert!(
        image_rest.iter().all(|entry| !want.contains(entry)),
        "a folder appears twice on the sandbox PATH: {path:?}"
    );
}
