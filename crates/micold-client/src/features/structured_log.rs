//! The read-only log of structured sessions (feature 938, FR-022a): one plain line per event,
//! until #939 replaces it with the transcript view.
//!
//! Render-free: this holds the lines, `ui/terminal.rs` draws them. The service sends a session's
//! history as `live: false` messages and each new event as a `live: true` message
//! (`DaemonMsg::StructuredEvents`); [`State::apply`] turns them into lines.

use micold_core::session::SessionId;
use micold_core::structured::{log_line, omitted_line, SequencedEvent, MAX_EVENTS};
use std::collections::HashMap;

/// The structured sessions' logs this window holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    logs: HashMap<SessionId, Log>,
}

/// One session's lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Log {
    /// How many earlier events are not in `lines`: the service's history no longer holds them, or
    /// this log dropped them to stay bounded.
    omitted: u64,
    /// The marker for `omitted`, when it is above zero.
    marker: Option<String>,
    lines: Vec<String>,
    /// The last message was a live event: the next history message starts over.
    after_live: bool,
}

impl State {
    /// Take one `StructuredEvents` message for `session`.
    ///
    /// A history message (`live: false`) replaces the lines when it is the session's first, or
    /// follows a live event (the window viewed the session again); otherwise it, and every live
    /// event, appends.
    pub fn apply(
        &mut self,
        session: SessionId,
        omitted: u64,
        events: &[SequencedEvent],
        live: bool,
    ) {
        let log = self.logs.entry(session).or_default();
        let replaces = !live && (log.lines.is_empty() && log.omitted == 0 || log.after_live);
        if replaces {
            *log = Log::default();
            log.omitted = omitted;
        }
        log.after_live = live;
        log.lines.extend(events.iter().map(log_line));
        // The log stays as bounded as the service's history is.
        let excess = log.lines.len().saturating_sub(MAX_EVENTS);
        if excess > 0 {
            log.lines.drain(..excess);
            log.omitted += excess as u64;
        }
        log.marker = (log.omitted > 0).then(|| omitted_line(log.omitted));
    }

    /// The rows to draw for `session`, in order: the omitted marker first when events are
    /// omitted. Empty for a session this window has heard nothing of.
    pub fn rows(&self, session: SessionId) -> impl Iterator<Item = &str> {
        self.logs.get(&session).into_iter().flat_map(|log| {
            log.marker
                .iter()
                .chain(log.lines.iter())
                .map(String::as_str)
        })
    }

    /// Forget `session`'s log.
    pub fn forget(&mut self, session: SessionId) {
        self.logs.remove(&session);
    }
}
