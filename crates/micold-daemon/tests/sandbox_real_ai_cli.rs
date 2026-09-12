//! FR-023a: the image we publish ships every AI CLI the application offers.
//!
//! # Why this test exists
//!
//! It did not, and the image shipped without any AI CLI at all — `bash`, `git` and `ssh` were
//! there, `claude` and `copilot` were not. A user enabling sandboxed mode got working shells and a
//! session that could never start its agent. Nothing caught it:
//!
//! - the twenty-two real-runtime probes all drive a **shell**, so a green suite said nothing about
//!   the tooling a session actually needs;
//! - `mise run image`'s smoke check verifies that *the daemon* can execute in the image, which is a
//!   different claim;
//! - the requirement was written down (FR-023, and `plan.md` called the file "daemon + shell + git
//!   + AI CLI"), and being written down is not being checked.
//!
//! # What makes it a gate rather than a spot-check
//!
//! It iterates [`AiCli::ALL`] and asks the container for each provider's own
//! [`command`](micold_core::provider::AiCliProvider::command). Neither the CLI names nor their count
//! is written here. So a third provider added to the enum fails this test until the image ships it
//! — which is the failure mode worth preventing, since the surface will happily *offer* a CLI the
//! sandbox cannot run.
//!
//! # And it runs them, rather than looking for them
//!
//! `command -v` proves a file is on `PATH`. It does not prove the file executes, and the two came
//! apart here in a way that is easy to ship: these CLIs are Node programs, Debian trixie packages
//! Node 20, and `@anthropic-ai/claude-code` declares `engines: node >=22.0.0`. An image built with
//! the distribution's Node would pass a `command -v` check and fail on first use. So each CLI is
//! asked for its version, in a session, as the uid the sandbox actually runs as.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).
//!
//! ```text
//! cargo test -p micold-daemon --features sandbox-real-runtime sandbox_real_ai_cli -- --nocapture
//! ```

#![cfg(all(feature = "sandbox-real-runtime", unix))]

mod sandbox_real_support;

use micold_core::protocol::auth::Token;
use micold_core::session::AiCli;

use sandbox_real_support::{
    credentials, input_serial, open_session, seed, start_sandbox, wait_for_accept, SandboxSpec,
    Terminal,
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

    let token = Token::generate();
    let token_path = data.join("micold-ai-ide").join("sandbox.token");
    token.write_to(&token_path).unwrap();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());

    // SAFETY: set before any spawn in this single-test binary.
    std::env::set_var("SHELL", SESSION_SHELL);

    let _sandbox = start_sandbox(&SandboxSpec {
        container: CONTAINER,
        network: NETWORK,
        port: PORT,
        data_home: &data,
        project: &project,
        token_path: &token_path,
        home: &home,
        // Off, as in every probe but the one that is about it: nothing here concerns survival.
        survive_logout: false,
        extra: &[],
    });

    let (mut conn, catalog) = wait_for_accept(PORT, &credentials(&token)).await;
    let serial = input_serial(&catalog, session);
    let log = data.join("micold-ai-ide").join("micold-daemon.log");
    let screen = open_session(&mut conn, &project, session, &log).await;
    let mut term = Terminal::new(&mut conn, session, screen, CONTAINER, &log, serial);

    // Deliberately not one assertion per CLI: a developer who has just added a provider wants the
    // whole list, not the first name alphabetically. Collected, then reported together.
    let mut missing = Vec::new();
    let mut unrunnable = Vec::new();

    for which in AiCli::ALL {
        let command = which.provider().command();

        let resolved = term
            .run(&format!("command -v {command} || echo NOT-ON-PATH"))
            .await;
        if resolved.contains("NOT-ON-PATH") {
            missing.push(command);
            continue;
        }

        // `--version` is the cheapest thing every CLI answers, and the exit status is what
        // separates "the file is there" from "the file runs". A Node program whose interpreter is
        // too old fails here and nowhere earlier.
        let version = term
            .run(&format!("{command} --version >/dev/null 2>&1; echo rc=$?"))
            .await;
        if !version.contains("rc=0") {
            unrunnable.push((command, version.trim().to_string()));
        }
    }

    assert!(
        missing.is_empty(),
        "the sandbox image does not provide {missing:?}, which the application offers as a session \
         provider. A user who picks one gets a session that cannot start its agent — the choice is \
         accepted, the failure arrives later, and nothing in the surface connected the two. \
         FR-023a puts every `AiCli::ALL` variant in `packaging/sandbox/Containerfile`."
    );
    assert!(
        unrunnable.is_empty(),
        "these CLIs are on `PATH` inside the sandbox but do not run: {unrunnable:?}. Being present \
         is not the requirement. The likeliest cause is the interpreter rather than the CLI — \
         `@anthropic-ai/claude-code` needs Node >=22 and Debian trixie packages Node 20, so an \
         image built on the distribution's Node passes a `command -v` check and fails on first use."
    );
}
