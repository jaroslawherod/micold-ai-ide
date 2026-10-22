//! The input driver (#941 research R2, data-model *Daemon*): one task that takes the turn ends and
//! output ends structured sessions' stdout readers report and, for each in order, delivers the
//! next queued message when the session is idle and tells every window
//! ([`DaemonState::turn_closed`]). A reader never takes the state's lock; this task does.
//!
//! It also writes the interrupts windows ask for and runs one overdue timer per ask (research R4,
//! FR-015): the 10 s are the daemon's, so every window, early or late, sees the same notice.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::state::{DaemonState, PendingInterrupt};
use crate::structured::TurnClosed;

/// Handle every report on `feed`, one at a time and in order, until the service stops. Each step
/// may write to a CLI's stdin, so it runs on a blocking thread.
pub async fn drive(state: Arc<DaemonState>, mut feed: mpsc::UnboundedReceiver<TurnClosed>) {
    while let Some(closed) = feed.recv().await {
        let state = Arc::clone(&state);
        let _ = tokio::task::spawn_blocking(move || state.turn_closed(closed)).await;
    }
}

/// Write `pending`, an interrupt a window asked for ([`DaemonState::ask_interrupt`]), off the async
/// threads, then, when it was written, arm its overdue timer: the ask goes `Overdue` when the bound
/// passes with the same ask still open ([`DaemonState::interrupt_went_overdue`]).
pub async fn interrupt(state: Arc<DaemonState>, pending: PendingInterrupt) {
    let asked = tokio::task::spawn_blocking(move || pending.write())
        .await
        .ok()
        .flatten();
    let Some(asked) = asked else {
        return;
    };
    tokio::time::sleep(state.interrupt_overdue()).await;
    let _ = tokio::task::spawn_blocking(move || state.interrupt_went_overdue(asked)).await;
}
