//! The client does not answer "which AI CLIs exist" from its own environment (feature 027,
//! FR-023c).
//!
//! # The bug this exists to prevent coming back
//!
//! Until feature 027 the client walked its own `PATH` — `Capabilities::available_providers()` —
//! and put the result on `State`. That was correct for as long as the session service was always a
//! child of this process. FR-021 made the service a container the client merely talks to, and the
//! same code then answered a question about the *host* and presented it as an answer about where
//! sessions run.
//!
//! What makes that worth a gate rather than a comment is how it fails. It does not crash and it
//! does not look wrong: the developer's own machine has every CLI installed, so the host's answer
//! and the container's agree on every machine the change would be tested on. It is only wrong for
//! the user who substituted an image (FR-023b) — and for them it is wrong in the most expensive
//! direction, offering a CLI that no session can start.
//!
//! # What is asserted
//!
//! Two halves of one claim, because either alone passes while wrong:
//!
//! 1. **No client source calls the probe.** `micold_core::provider::available_here` and the
//!    `is_available()` it is built from are the daemon's to call. A client that reintroduces
//!    either has reintroduced the bug whatever it names the field.
//! 2. **The field is fed by the protocol.** A scan for an absence passes trivially if the feature
//!    was deleted rather than moved, so the shell must still be seen asking the service and
//!    recording its reply.
//!
//! The far end — that the service's answer is its own environment and not a constant — is
//! `crates/micold-daemon/tests/ai_cli_availability.rs`, and the probe's own behaviour is
//! `crates/micold-core/tests/available_here.rs`.

mod inventory;

use std::path::Path;

/// Every client source, comments stripped, keyed by path relative to `src/`.
fn sources() -> Vec<(String, String)> {
    inventory::sources_under(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
        .into_iter()
        .map(|(path, text)| (path, inventory::code_only(&text)))
        .collect()
}

/// The spellings that would resolve a CLI on *this* process's `PATH`.
///
/// `available_here` is the whole probe; `provider().is_available()` is the per-provider predicate
/// it is built from, and calling that in a loop is the same bug written out longhand. Both are
/// listed so that re-implementing the function under another name is caught as well as calling it.
///
/// The predicate is spelled with its receiver on purpose. A bare `.is_available()` also matches
/// `worktree_form`'s branch candidates, which are about a git ref and have nothing to do with any
/// of this — a guard that fails for an unrelated reason gets relaxed, and a relaxed guard is
/// worth nothing.
const PROBES: [&str; 2] = ["available_here(", "provider().is_available()"];

#[test]
fn no_client_source_probes_this_process_for_a_cli() {
    let offenders: Vec<String> = sources()
        .into_iter()
        .flat_map(|(path, code)| {
            PROBES
                .into_iter()
                .filter(|probe| code.contains(probe))
                .map(|probe| format!("`{path}` calls `{probe}…)`"))
                .collect::<Vec<_>>()
        })
        .collect();

    assert!(
        offenders.is_empty(),
        "the client is answering CLI availability from its own environment (FR-023c):\n  - {}\n\n\
         Under the sandboxed placement this process is on the host and the sessions are in a \
         container, so a `PATH` probe here describes a different machine — plausibly, which is \
         what makes it expensive. Send `ClientMsg::AiCliAvailabilityRequest` and read \
         `DaemonMsg::AiCliAvailability` instead; `shell/daemon_sync.rs` already does both.",
        offenders.join("\n  - ")
    );
}

#[test]
fn the_shell_asks_the_service_and_records_the_answer() {
    // The vacuity check for the test above, which would also pass if availability had simply
    // stopped being tracked. Three spellings, because the three are what make the answer usable:
    // the request going out, the reply coming back, and the reply reaching the field the pickers
    // read. Any one of them missing is a picker that offers nothing, for ever.
    let sources = sources();
    let shell: String = sources
        .iter()
        .filter(|(path, _)| path.starts_with("shell/"))
        .map(|(_, code)| code.as_str())
        .collect();

    for expected in [
        "ClientMsg::AiCliAvailabilityRequest",
        "DaemonMsg::AiCliAvailability",
        "available_providers = Some(",
    ] {
        assert!(
            shell.contains(expected),
            "nothing under `shell/` mentions `{expected}` — the guard above is passing because \
             the client no longer tracks CLI availability at all, not because it asks the service \
             for it (FR-023c)."
        );
    }
}
