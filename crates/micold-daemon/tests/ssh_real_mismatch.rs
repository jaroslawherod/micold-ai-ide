//! A remote daemon of another version over a real `ssh` (feature 489, T069, part 1, FR-011, SC-005):
//! in each direction the connection is refused with both versions, the explanation names which side
//! is older and the way out, no session can start on it, and the refusal ends nothing on the host.
//!
//! The stand-in daemon is a real `micold-daemon` built as another version (`stale_daemon`). Off by
//! default (`ssh-real-runtime`); `mise run test-ssh` runs it. Skips with a printed reason when
//! `sshd` is not installed or the stand-in cannot be built.

#![cfg(all(feature = "ssh-real-runtime", unix))]

#[path = "ssh_real_support/mod.rs"]
mod ssh_real_support;

use micold_core::connect::Connected;
use micold_core::protocol::messages::RefusalReason;
use micold_core::ssh::mismatch::{MismatchExplanation, ReplaceOffer, Side, WayOut};
use ssh_real_support::stale_daemon::{self, NEWER, OLDER};

/// Dial a daemon built with `suffix` and return the versions the refusal names and the host's pid.
async fn refused_by(test: &str, suffix: &str) -> Option<(MismatchExplanation, Option<u32>)> {
    let host = ssh_real_support::host_or_skip(test)?;
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    if micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        eprintln!(
            "SKIP {test}: a daemon already listens on {}",
            endpoint.socket_path.display()
        );
        return None;
    }
    let stale = match stale_daemon::build(suffix) {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("SKIP {test}: {reason}");
            return None;
        }
    };
    let dialer = stale_daemon::dialer(&host, stale, suffix);
    let connected = dialer.connect(None).await;
    let guard = stale_daemon::cleanup_guard();
    let pid = guard.0;
    let Ok(Connected::Refused(reason)) = connected else {
        panic!(
            "the mismatched daemon must refuse the handshake, not connect: {:?}",
            connected.map(|_| ())
        );
    };
    let RefusalReason::BuildMismatch {
        client_build,
        daemon_build,
    } = reason
    else {
        panic!("a package-version difference is a BuildMismatch: {reason:?}");
    };
    // A second dial is refused the same way: nothing replaced or ended the daemon.
    let again = dialer.connect(None).await;
    assert!(
        matches!(again, Ok(Connected::Refused(_))),
        "the daemon is still the stale one after a refusal"
    );
    assert_eq!(
        pid,
        micold_core::spawn::running_daemon_pid(&endpoint),
        "a refusal ends no process on the host"
    );
    drop(guard);
    Some((
        MismatchExplanation::new(&client_build, &daemon_build, ReplaceOffer::Unknown),
        pid,
    ))
}

#[tokio::test]
async fn ssh_real_mismatch_an_older_daemon_is_refused_and_explained() {
    let name = "ssh_real_mismatch_an_older_daemon_is_refused_and_explained";
    let Some((explanation, pid)) = refused_by(name, OLDER).await else {
        return;
    };
    assert!(pid.is_some(), "the stale daemon was started on the host");
    assert_eq!(
        explanation.client,
        micold_core::protocol::version::PACKAGE_VERSION
    );
    assert_eq!(explanation.daemon, stale_daemon::version_with(OLDER));
    assert_eq!(explanation.older, Side::Daemon);
    assert_eq!(explanation.way_out, WayOut::EndDaemonByHand);
    let text = explanation.text("Dev", "stand-in");
    assert!(
        text.contains(&format!("client {}", explanation.client)),
        "{text}"
    );
    assert!(
        text.contains(&format!("daemon {}", explanation.daemon)),
        "{text}"
    );
    assert!(text.contains("(daemon is older)"), "{text}");
}

#[tokio::test]
async fn ssh_real_mismatch_a_newer_daemon_is_refused_and_explained() {
    let name = "ssh_real_mismatch_a_newer_daemon_is_refused_and_explained";
    let Some((explanation, _)) = refused_by(name, NEWER).await else {
        return;
    };
    assert_eq!(explanation.daemon, stale_daemon::version_with(NEWER));
    assert_eq!(explanation.older, Side::Client);
    assert_eq!(explanation.way_out, WayOut::UpdateApp);
    let text = explanation.text("Dev", "stand-in");
    assert!(
        text.contains("(daemon is newer). Update this app."),
        "{text}"
    );
}
