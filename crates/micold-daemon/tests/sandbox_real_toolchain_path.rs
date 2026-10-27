//! Feature 1289, M1 (FR-001 to FR-003, US1): in a real app-owned sandbox, a terminal's `PATH` is
//! the offered CLI bins, then the sandbox home's `.local/bin`, then mise's shims folder, then the
//! image's own `PATH`. The unit tests prove the order from fixed inputs; this proves the inputs the
//! daemon reads inside the container are the ones it should, and that the order reaches a shell.
//!
//! And #1439 (FR-011, US3): the basic image's entrypoint creates both home folders before the first
//! session, and a `mise` the user put in `~/.local/bin` wins over the image's `/usr/local/bin/mise`.
//! The daemon also writes the agent's guide and its block in `~/.claude/CLAUDE.md` (FR-019).
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

    // #1439 FR-011: before any setup, both folders exist (the entrypoint made them), so a session
    // listing its PATH sees what is there.
    let folders = term
        .run(&format!(
            "test -d '{local_bin}' && test -d '{shims}' && echo FOLDERS-EXIST"
        ))
        .await;
    assert!(
        folders.lines().any(|l| l.trim() == "FOLDERS-EXIST"),
        "{local_bin} or {shims} does not exist in a new sandbox: {folders}"
    );

    // #1439 FR-019 (BUG-1500): the daemon wrote the agent's guide at startup and pointed Claude
    // Code's user-level instructions at it. The tag is split so the wrapped echo of the typed
    // command never carries it joined.
    // The block goes only into an offered CLI's file.
    let begin = micold_core::sandbox::agent_guide::BEGIN;
    let claude = offered
        .clis
        .iter()
        .any(|c| c.cli == micold_core::session::AiCli::ClaudeCode);
    let block_check = if claude {
        format!("grep -qF '{begin}' '{sandbox_home}/.claude/CLAUDE.md' && ")
    } else {
        String::new()
    };
    let guide = term
        .run(&format!(
            "test -s '{sandbox_home}/.micold/AGENTS.md' && {block_check}echo GUIDE\"-WRITTEN\""
        ))
        .await;
    assert!(
        guide.lines().any(|l| l.trim() == "GUIDE-WRITTEN"),
        "no agent guide in {sandbox_home}/.micold/AGENTS.md or no marked block in \
         {sandbox_home}/.claude/CLAUDE.md: {guide}"
    );

    // Edge case "A `mise` already in `~/.local/bin`": the user's copy comes first on PATH, ahead of
    // the image's own.
    let stub = format!("{local_bin}/mise");
    let resolved = term
        .run(&format!(
            "test -x /usr/local/bin/mise && echo IMAGE-MISE; \
             printf '#!/bin/sh\\necho stub\\n' > '{stub}' && chmod 0755 '{stub}' && \
             hash -r 2>/dev/null; command -v mise"
        ))
        .await;
    let lines: Vec<&str> = resolved.lines().map(str::trim).collect();
    assert!(
        lines.contains(&"IMAGE-MISE"),
        "the image has no /usr/local/bin/mise: {resolved}"
    );
    assert!(
        lines.contains(&stub.as_str()),
        "`command -v mise` does not resolve to {stub}: {resolved}"
    );
}
