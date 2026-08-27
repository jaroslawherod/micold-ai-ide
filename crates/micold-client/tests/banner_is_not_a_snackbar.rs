//! The connection banner did not become a snackbar (feature 018, T053a — FR-032c).
//!
//! Material treats banners and snackbars as **different components**, and folding one into the
//! other is the specific mistake this requirement forbids. The temptation is real: after T053 the
//! application has a queue that shows one message at a time with a timeout, and the connection
//! strip is also "a message about something being wrong", so routing it through the same queue
//! looks like removing a duplicate.
//!
//! It would be a bug. The banner reports a *standing condition* — the daemon is unreachable, the
//! session was taken over — which is true until it stops being true. A snackbar is transient by
//! construction: it queues behind other messages, it times out, and it can be dismissed. Put the
//! connection state through it and the user is told once, for four seconds, that the application
//! cannot reach its daemon, and then the notice disappears while the condition persists.

use std::fs;
use std::path::Path;

use micold_client::app::State;

/// The source of the rendering layer, for the structural half.
fn source(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join(relative);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The banner is rendered from connection status, never from the notification queue.
///
/// A source check because the status is a *view parameter* rather than application state — it comes
/// from the daemon connection and is handed to `view`, so there is no reducer path to drive. What
/// can be asserted is that the function drawing it never reaches for the queue, which is the shape
/// the folding-in would take.
#[test]
fn the_banner_is_drawn_from_connection_status_and_not_from_the_queue() {
    let ui = source("ui/mod.rs");
    let start = ui.find("fn connection_banner").expect(
        "`connection_banner` is gone — if the banner was folded into the snackbar, that is \
                 exactly what FR-032c forbids",
    );
    let end = ui[start..]
        .find("\n}\n")
        .map(|o| start + o)
        .unwrap_or(ui.len());
    let body = &ui[start..end];

    assert!(
        body.contains("ConnectionStatus"),
        "the banner no longer reads the connection status it exists to report"
    );
    assert!(
        !body.contains("notify") && !body.contains("Snackbar"),
        "the connection banner is reaching into the notification queue. It would then time out and \
         be dismissible — telling the user once that the daemon is unreachable and hiding it while \
         it still is (FR-032c)"
    );
}

/// The banner is full-width; a snackbar is not.
///
/// The two are distinguishable in the layout as well as in the code, which is what makes "they are
/// different components" observable rather than merely stated.
#[test]
fn the_banner_spans_the_window_and_the_snackbar_does_not() {
    let ui = source("ui/mod.rs");
    let snackbar = source("ui/material/snackbar.rs");

    assert!(
        ui.contains("connection_banner(connection, roles),"),
        "the banner is no longer a full-width row in the window column"
    );
    assert!(
        snackbar.contains("max_width(anatomy::snackbar::MAX_WIDTH)"),
        "the snackbar lost its max width, so it is now as wide as the banner and the two are no \
         longer distinguishable on screen (§7.8)"
    );
}

/// Each has its own vocabulary, and only one of them carries a duration.
///
/// `NoticeLevel` is the banner's severity — it feeds `SurfaceKind::Notification`, whose fill is
/// chosen by it. `micold_core::notify::Level` additionally decides *how long* a message stays. A
/// standing condition has no duration, so giving the banner that vocabulary would invite one.
#[test]
fn only_the_queue_s_level_carries_a_duration() {
    assert!(
        micold_core::notify::Level::Error.duration() > micold_core::notify::Level::Info.duration(),
        "the queue's levels no longer differ in duration, which is the property that makes them \
         the wrong vocabulary for a standing condition"
    );
}

/// The queue still works — so the separation above is not passing because nothing does.
#[test]
fn an_ordinary_notification_still_reaches_the_queue() {
    let mut state = State::default();
    state.notify_error("could not create the worktree");

    assert!(
        state.notifications.queue.visible().is_some(),
        "no notification reached the queue at all, so every assertion above is vacuous"
    );
    assert_eq!(
        state.notifications.queue.pending(),
        0,
        "one message should not queue behind itself"
    );
}

// -------------------------------------------------------------------------------------------
// T102 — the sandbox's standing conditions belong to the banner too (feature 027, FR-035b, S-3)
// -------------------------------------------------------------------------------------------

use micold_client::features::sandbox::Sandbox;
use micold_core::sandbox::lifecycle::{Failure, Stage};
use micold_core::sandbox::placement::{ConsentedFallback, PlacementKind};
use micold_core::sandbox::runtime::{RuntimeError, RuntimeKind};

fn failed_sandbox() -> Sandbox {
    let mut s = Sandbox::for_placement(PlacementKind::LocalSandbox);
    s.failed(Failure {
        stage: Stage::Probing,
        error: RuntimeError::NotInstalled {
            kind: RuntimeKind::Docker,
        },
    });
    s
}

/// A broken sandbox and a session running outside one are conditions, not events.
///
/// The spec's own edge case is a user who takes the one-occurrence fallback on every launch and
/// never notices sandboxing has been off for weeks. A four-second toast is how that happens: it is
/// shown once, while the condition it describes lasts indefinitely.
#[test]
fn the_failed_and_unsandboxed_states_are_standing_conditions() {
    let failed = failed_sandbox();
    assert!(failed.state.is_persistent());
    assert!(failed.persistent_notice().is_some());

    let mut unsandboxed = failed_sandbox();
    unsandboxed.accept_fallback(ConsentedFallback {
        because: "Docker is not installed.".into(),
    });
    assert!(
        unsandboxed.persistent_notice().is_some(),
        "the fallback is where FR-035b bites hardest — the sandbox is no longer failing, it is \
         simply not there, and nothing else on screen says so"
    );
}

/// And they are drawn by the banner rather than pushed through the queue.
///
/// A source check for the same reason the connection one above is: the sandbox lives on the
/// binary's `App` beside the daemon connection, so there is no reducer path to drive from a test.
/// What can be asserted is the shape the mistake would take.
#[test]
fn the_sandbox_banner_is_drawn_from_the_sandbox_and_not_from_the_queue() {
    let ui = source("ui/mod.rs");
    let start = ui
        .find("fn sandbox_banner")
        .expect("`sandbox_banner` is gone — a failed sandbox has nowhere persistent to be shown");
    let end = ui[start..]
        .find("\n}\n")
        .map(|o| start + o)
        .unwrap_or(ui.len());
    let body = &ui[start..end];

    assert!(
        body.contains("persistent_notice"),
        "the banner no longer reads the notice it exists to show"
    );
    assert!(
        !body.contains("notify") && !body.contains("Snackbar"),
        "the sandbox banner is reaching into the notification queue, which times out and is \
         dismissible (FR-035b)"
    );
}

/// The failure must not *also* be announced as a toast.
///
/// It was, and the comment beside it said the opposite — which is the failure mode this file
/// exists for: the queue is right there, `notify_error` is one line, and the result tells the user
/// once about a condition that outlives the telling.
#[test]
fn a_failed_sandbox_is_not_announced_through_the_notification_queue() {
    // Feature 028 folded the six flat `Message::Sandbox*` arms behind one wrapper and moved the
    // reducer to the shell half, so the arm this reads is `Msg::Failed` in `shell/sandbox.rs`
    // rather than `Message::SandboxFailed` in `main.rs`. Same arm, same claim about it.
    let main = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/sandbox.rs"),
    )
    .expect("read shell/sandbox.rs");

    let start = main
        .find("Msg::Failed(failure) =>")
        .expect("the failure is no longer handled at all");
    let end = main[start..]
        .find("Task::none()")
        .map(|o| start + o)
        .unwrap_or(main.len());

    assert!(
        !main[start..end].contains("notify"),
        "a failed sandbox is being queued as a notification. It would time out and be dismissible \
         while the sandbox is still broken (FR-035b, S-3)"
    );
}
