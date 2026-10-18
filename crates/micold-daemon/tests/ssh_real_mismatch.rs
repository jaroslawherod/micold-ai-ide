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
use ssh_real_support::remote::{request, Dialer, RemoteDaemon};
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

/// The endpoint of the stand-in host, or why this test must skip.
fn free_endpoint(test: &str) -> Option<micold_core::endpoint::Endpoint> {
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    if micold_core::spawn::running_daemon_pid(&endpoint).is_some() {
        eprintln!(
            "SKIP {test}: a daemon already listens on {}",
            endpoint.socket_path.display()
        );
        return None;
    }
    Some(endpoint)
}

async fn wait_for_offer(dialer: &Dialer, wanted: ReplaceOffer) -> ReplaceOffer {
    let mut offer = dialer.offer().await;
    for _ in 0..50 {
        if offer == wanted {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        offer = dialer.offer().await;
    }
    offer
}

/// T076, FR-011, US5 scenario 2: an older daemon with no running session is offered a replacement,
/// the replacement ends it, and the next connection reaches this build's daemon.
#[tokio::test]
async fn ssh_real_mismatch_replace_is_offered_with_no_sessions_and_then_connects() {
    let name = "ssh_real_mismatch_replace_is_offered_with_no_sessions_and_then_connects";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    let Some(endpoint) = free_endpoint(name) else {
        return;
    };
    let stale = match stale_daemon::build(OLDER) {
        Ok(path) => path,
        Err(reason) => {
            eprintln!("SKIP {name}: {reason}");
            return;
        }
    };
    let refused = stale_daemon::dialer(&host, stale, OLDER)
        .connect(None)
        .await;
    let stale_guard = stale_daemon::cleanup_guard();
    let stale_pid = stale_guard.0.expect("the stale daemon runs on the host");
    let Ok(Connected::Refused(RefusalReason::BuildMismatch {
        client_build,
        daemon_build,
    })) = refused
    else {
        panic!("the older daemon must refuse: {:?}", refused.map(|_| ()));
    };

    // As the client does: this build's helper is on the host and its bridge is refused.
    let dialer = Dialer::new(&host);
    assert!(matches!(
        dialer.connect(None).await,
        Ok(Connected::Refused(_))
    ));
    let offer = wait_for_offer(&dialer, ReplaceOffer::Available).await;
    assert_eq!(offer, ReplaceOffer::Available, "no session runs on it");
    let explanation = MismatchExplanation::new(&client_build, &daemon_build, offer);
    assert_eq!(explanation.way_out, WayOut::ReplaceDaemon);

    assert_eq!(dialer.replace().await, Ok(()));
    assert_eq!(
        micold_core::spawn::running_daemon_pid(&endpoint),
        None,
        "the stale daemon is gone"
    );
    let connected = dialer.connect(None).await;
    let fresh = RemoteDaemon(micold_core::spawn::running_daemon_pid(&endpoint));
    assert!(
        matches!(connected, Ok(Connected::Ready(..))),
        "the replacement connects: {:?}",
        connected.map(|_| ())
    );
    assert_ne!(
        fresh.0,
        Some(stale_pid),
        "this build's daemon took its place"
    );
}

/// T076, FR-011: a daemon with a running session is never offered a replacement and is not ended.
/// A client of another version cannot start a session on a mismatched daemon, so the session is
/// started on this build's daemon; `status` and `stop` do not depend on the daemon's version.
#[tokio::test]
async fn ssh_real_mismatch_replace_is_withheld_while_a_session_runs() {
    use micold_core::protocol::messages::ClientMsg;
    use micold_core::session::AiCli;

    let name = "ssh_real_mismatch_replace_is_withheld_while_a_session_runs";
    let Some(host) = ssh_real_support::host_or_skip(name) else {
        return;
    };
    let repo = ssh_real_support::remote::stand_in_repo(
        &host,
        "#!/bin/sh\necho STAND-IN-AI-READY\nexec cat\n",
    );
    let Some(mut started) = ssh_real_support::remote::start(&host, name).await else {
        return;
    };
    let endpoint = micold_core::endpoint::resolve_pinned().expect("pinned endpoint");
    let pid = micold_core::spawn::running_daemon_pid(&endpoint);
    assert!(pid.is_some());
    let mut seen = String::new();
    let conn = &mut *started.connection;
    request(
        conn,
        ClientMsg::ProjectAdd {
            req: 1,
            path: repo.clone(),
        },
        1,
        &mut seen,
    )
    .await;
    let micold_core::protocol::messages::OperationResult::SessionCreated { session } = request(
        conn,
        ClientMsg::SessionCreate {
            req: 2,
            project: repo.clone(),
            worktree_dir: String::new(),
            provider: AiCli::ClaudeCode,
            kind: Default::default(),
            allow_additional_session: true,
        },
        2,
        &mut seen,
    )
    .await
    else {
        panic!("a session is created");
    };
    use futures_util::SinkExt;
    conn.send(micold_core::protocol::codec::Frame::Control(
        ClientMsg::SessionStart {
            session,
            allow_additional_session: true,
        },
    ))
    .await
    .expect("start the session");

    let dialer = &started.dialer;
    let offer = wait_for_offer(dialer, ReplaceOffer::Blocked { running: 1 }).await;
    assert_eq!(offer, ReplaceOffer::Blocked { running: 1 });
    assert_eq!(
        dialer.replace().await,
        Err(ReplaceOffer::Blocked { running: 1 })
    );
    assert_eq!(
        micold_core::spawn::running_daemon_pid(&endpoint),
        pid,
        "the daemon with a session is not ended"
    );
    let text = MismatchExplanation::new("0.5.3", "0.5.2", offer).text("Dev", "stand-in");
    assert!(text.contains("1 session"), "{text}");
}
