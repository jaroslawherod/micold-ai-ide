//! The input driver (#941 research R2, data-model *Daemon*): one task that takes the turn ends and
//! output ends structured sessions' stdout readers report and, for each in order, delivers the
//! next queued message when the session is idle and tells every window
//! ([`DaemonState::turn_closed`]). A reader never takes the state's lock; this task does.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::state::DaemonState;
use crate::structured::TurnClosed;

/// Handle every report on `feed`, one at a time and in order, until the service stops. Each step
/// may write to a CLI's stdin, so it runs on a blocking thread.
pub async fn drive(state: Arc<DaemonState>, mut feed: mpsc::UnboundedReceiver<TurnClosed>) {
    while let Some(closed) = feed.recv().await {
        let state = Arc::clone(&state);
        let _ = tokio::task::spawn_blocking(move || state.turn_closed(closed)).await;
    }
}
