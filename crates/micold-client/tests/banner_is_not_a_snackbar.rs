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
        state.notify.visible().is_some(),
        "no notification reached the queue at all, so every assertion above is vacuous"
    );
    assert_eq!(
        state.notify.pending(),
        0,
        "one message should not queue behind itself"
    );
}
