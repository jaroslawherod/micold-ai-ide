//! The storage capability, checked against what the runtime **actually does** (FR-015, SC-009, C-2).
//!
//! This test exists because the classification it checks was wrong, and was wrong for a reason that
//! no amount of fake-runtime coverage could have caught. R5 recorded this measurement:
//!
//! ```text
//! $ docker run --rm --storage-opt size=1G alpine:latest true ; echo $?
//! 0
//! ```
//!
//! and concluded that Docker 29's `overlayfs` driver enforces a writable-storage limit. It does not.
//! It **accepts** the flag and ignores it — 700 MiB writes cleanly into a 512 MiB cap — which is
//! precisely the "silently accepted" outcome C-2 forbids and the one a user is least able to
//! discover for themselves, because everything looks like it worked.
//!
//! An exit code is not enforcement. So this asks the runtime for its capability the way the
//! application does, then tries to exceed the limit for real, and requires the two to agree.
//!
//! Behind `sandbox-real-runtime` (Principle VI: the default suite needs nothing installed).

#![cfg(feature = "sandbox-real-runtime")]

use std::process::Command;

use micold_core::sandbox::cli::CliRuntime;
use micold_core::sandbox::exec::SystemRunner;
use micold_core::sandbox::runtime::{ContainerRuntime, LimitSupport, RuntimeKind};

const IMAGE: &str = "micold-daemon:dev";
/// Comfortably above the image's own size — a cap below it makes the container unstartable, which
/// would answer a different question — and low enough that exceeding it costs under a second.
const CAP_MIB: u64 = 512;
/// Enough over the cap to leave no doubt, small enough to stay cheap on a CI runner's disk.
const WRITE_MIB: u64 = 700;

/// What happened when a container was asked to exceed its storage limit.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// The write was stopped. The limit is real.
    Enforced,
    /// The write succeeded past the cap. The flag was accepted and dropped.
    Ignored,
    /// The runtime refused the flag outright, so no container ran.
    Rejected(String),
}

fn exceed_the_cap() -> Outcome {
    let out = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--storage-opt",
            &format!("size={CAP_MIB}m"),
            "--entrypoint",
            "sh",
            IMAGE,
            "-c",
            &format!("dd if=/dev/zero of=/big bs=1M count={WRITE_MIB} 2>&1; echo rc=$?"),
        ])
        .output()
        .expect("docker run");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    if text.contains("No space left") {
        Outcome::Enforced
    } else if text.contains("rc=0") {
        Outcome::Ignored
    } else {
        Outcome::Rejected(text.trim().to_string())
    }
}

#[tokio::test]
async fn sandbox_real_storage_capability_matches_what_the_runtime_enforces() {
    let claimed = CliRuntime::new(RuntimeKind::Docker, SystemRunner)
        .probe()
        .expect("probe the real runtime")
        .storage;
    let actual = exceed_the_cap();

    match (&claimed, &actual) {
        (LimitSupport::Supported, Outcome::Enforced) => {}
        (LimitSupport::Unsupported { reason }, Outcome::Ignored | Outcome::Rejected(_)) => {
            // SC-009 renders this beside the disabled field. An empty one leaves the user with a
            // control they cannot use and no account of why.
            assert!(
                !reason.trim().is_empty(),
                "an unsupported limit must carry a reason the view can show"
            );
        }
        (LimitSupport::Supported, other) => panic!(
            "the application would offer a writable-storage limit this runtime does not enforce \
             — the failure C-2 exists to prevent, and the one a user cannot detect. Writing \
             {WRITE_MIB} MiB into a {CAP_MIB} MiB cap gave: {other:?}"
        ),
        (LimitSupport::Unsupported { reason }, Outcome::Enforced) => panic!(
            "the application would deny a limit this runtime does in fact enforce, saying: {reason}"
        ),
    }
}
