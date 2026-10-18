//! FR-023a, as feature 806 reshaped it: the basic image carries no AI CLI; provisioning installs
//! every CLI the application offers (`AiCli::ALL`) into the state directory, and the daemon offers
//! the ones `offered.json` lists.
//!
//! # What makes it a gate rather than a spot-check
//!
//! It iterates [`AiCli::ALL`] and asks the container for each provider's own
//! [`command`](micold_core::provider::AiCliProvider::command). Neither the CLI names nor their count
//! is written here. So a third provider added to the enum fails this test until the install step
//! installs it — which is the failure mode worth preventing, since the surface will happily *offer*
//! a CLI the sandbox cannot run.
//!
//! # And it runs them, rather than looking for them
//!
//! A file on disk does not prove the file executes. So each CLI is asked for its version, in a
//! session, as the uid the sandbox actually runs as. The CLIs are the stand-in packages
//! (`standin_table`): no registry, no sign-in (FR-025). That the *real* CLIs install and run is
//! `scripts/tests` territory and the install step's own unit tests; this probe is about the
//! provisioning seam inside a real container.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_ai_cli -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use micold_core::session::AiCli;

use sandbox_real_support::{
    input_serial, open_session, seed, start_sandbox, wait_for_accept, SandboxSpec, Terminal,
};

const CONTAINER: &str = "micold-ai-cli-probe";
const NETWORK: &str = "micold-ai-cli-probe-net";
const PORT: u16 = 17740;

/// Pinned for the reason the other probes pin it: an inherited `SHELL` is the developer's, and the
/// difference would be reported as the sandbox's.
const SESSION_SHELL: &str = "/bin/sh";

#[tokio::test]
async fn sandbox_real_the_image_ships_every_ai_cli_the_application_offers() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("data");
    let project = dir.path().join("project");
    std::fs::create_dir_all(&project).unwrap();
    let session = seed(&data, &project, "ai-cli-probe");
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
        // Off, as in every probe but the one that is about it: nothing here concerns survival.
        survive_logout: false,
        extra: &[],
    });

    let (mut conn, catalog) = wait_for_accept(PORT).await;
    let serial = input_serial(&catalog, session);
    let log = data.join("micold-ai-ide").join("micold-daemon.log");
    let screen = open_session(&mut conn, &project, session, &log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &log, serial);

    // Where the install step recorded each CLI (`<state>/ai-clis/offered.json`, on the host side
    // of the state mount).
    let offered =
        micold_core::provision::ai_cli::read_offered(&data.join("micold-ai-ide").join("ai-clis"))
            .expect("offered.json is readable")
            .expect("provisioning wrote offered.json");

    // Deliberately not one assertion per CLI: a developer who has just added a provider wants the
    // whole list, not the first name alphabetically. Collected, then reported together.
    let mut missing = Vec::new();
    let mut unrunnable = Vec::new();

    for which in AiCli::ALL {
        let command = which.provider().command();
        let Some(entry) = offered.clis.iter().find(|c| c.cli == which) else {
            missing.push(command);
            continue;
        };
        let path = entry.bin.join(command);

        // `--version` is the cheapest thing every CLI answers, and the exit status is what
        // separates "the file is there" from "the file runs". Stdin is closed because the stand-in
        // reads lines until it is.
        let version = term
            .run(&format!(
                "{} --version </dev/null >/dev/null 2>&1; echo rc=$?",
                path.display()
            ))
            .await;
        if !version.contains("rc=0") {
            unrunnable.push((command, version.trim().to_string()));
        }
    }

    assert!(
        missing.is_empty(),
        "provisioning did not install {missing:?}, which the application offers as a session \
         provider. A user who picks one gets a session that cannot start its agent — the choice is \
         accepted, the failure arrives later, and nothing in the surface connected the two. \
         FR-023a puts every `AiCli::ALL` variant in the install step's table."
    );
    assert!(
        unrunnable.is_empty(),
        "these CLIs were installed in the sandbox but do not run: {unrunnable:?}. Being present is \
         not the requirement. The likeliest cause is the interpreter rather than the CLI: the \
         basic image's Node must satisfy what the CLIs declare."
    );
}
