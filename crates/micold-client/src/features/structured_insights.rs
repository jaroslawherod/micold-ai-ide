//! The structured sessions' insights this window holds (feature 944, data-model.md *View state*):
//! per session, the [`SessionInsights`] derived from its events, the subagent threads the user
//! opened (each a nested [`Transcript`]) and whether the background-task list is open.
//!
//! Render-free: this holds the state, `ui/transcript/` draws it. It is applied after #939's
//! [`crate::features::structured_transcript`] for the same message and reads the main transcript
//! that feature holds; it never changes it (contracts/insights-model.md *Client seam*). Fold state
//! lives here, per window, and never on the wire (FR-019).

use micold_core::session::SessionId;
use micold_core::structured::insights::{SessionInsights, SubagentState, ThreadView, TurnUsage};
use micold_core::structured::transcript::{is_subagent_event, EntryKey, ToolStatus, Transcript};
use micold_core::structured::SequencedEvent;
use std::collections::HashMap;

/// What the insights shown with a transcript say (data-model.md *Messages*), carried by the
/// session message family as `session::Msg::Insights`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsightsMsg {
    /// A subagent thread's header was activated: unfold or fold it (FR-015).
    ToggleThread {
        /// The session whose transcript holds the thread.
        session: SessionId,
        /// The tool use id of the call that started the subagent.
        parent: String,
    },
    /// The background-task list's header was activated: unfold or fold it.
    ToggleBackgroundTasks {
        /// The session whose list it is.
        session: SessionId,
    },
}

impl InsightsMsg {
    /// The session the message is about.
    pub fn session(&self) -> SessionId {
        match self {
            Self::ToggleThread { session, .. } | Self::ToggleBackgroundTasks { session } => {
                *session
            }
        }
    }
}

/// The structured sessions' insights this window holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    sessions: HashMap<SessionId, SessionExtras>,
}

/// One session's insights and this window's folds of them.
#[derive(Debug, Clone, Default)]
struct SessionExtras {
    insights: SessionInsights,
    /// The nested transcripts of the open threads, by parent tool use id; absent is folded.
    open_threads: HashMap<String, Transcript>,
    /// The background-task list is unfolded.
    tasks_open: bool,
    /// The main transcript's `(rebuilds, omitted)` at the last apply: a rebuild builds the open
    /// threads again from its held events; an eviction rebuilds those it cut and folds those gone.
    seen: (u64, Option<u64>),
}

/// Equal when the insights, the open threads' entries at their revisions and the list's fold are;
/// as #939's transcript state compares, without comparing every event.
impl PartialEq for SessionExtras {
    fn eq(&self, other: &Self) -> bool {
        let same_thread = |a: &Transcript, b: &Transcript| {
            a.entries().len() == b.entries().len()
                && a.entries()
                    .zip(b.entries())
                    .all(|(x, y)| x.key() == y.key() && x.revision() == y.revision())
        };
        self.insights == other.insights
            && self.tasks_open == other.tasks_open
            && self.seen == other.seen
            && self.open_threads.len() == other.open_threads.len()
            && self.open_threads.iter().all(|(parent, a)| {
                other
                    .open_threads
                    .get(parent)
                    .is_some_and(|b| same_thread(a, b))
            })
    }
}

impl Eq for SessionExtras {}

/// The main transcript's marks that say its held events were replaced or cut at the front.
fn marks(main: &Transcript) -> (u64, Option<u64>) {
    (main.rebuilds(), main.omitted_notice())
}

/// The thread of `parent` built from the events `main` holds (research R7).
fn nested(parent: &str, main: &Transcript) -> Transcript {
    let events: Vec<SequencedEvent> = main.subagent_events(parent).cloned().collect();
    let mut thread = Transcript::nested(parent);
    thread.apply(0, &events, false);
    thread
}

impl State {
    /// Take one `StructuredEvents` message for `session` (#938 wire.md), after `main`, the
    /// session's transcript, took it. An open thread follows it: its own events are appended,
    /// and after a rebuild of `main`, or an eviction that dropped some of them, it is built again
    /// from `main`'s held events. A thread whose events all left folds, as #939 folds a card that
    /// left (FR-013 of #939).
    pub fn apply(
        &mut self,
        session: SessionId,
        omitted: u64,
        events: &[SequencedEvent],
        live: bool,
        main: &Transcript,
    ) {
        let s = self.sessions.entry(session).or_default();
        s.insights.apply(omitted, events, live);
        let seen = marks(main);
        let rebuilt = seen.0 != s.seen.0;
        let evicted = seen.1 != s.seen.1;
        s.seen = seen;
        if rebuilt || evicted {
            let insights = &s.insights;
            s.open_threads
                .retain(|parent, _| insights.thread(parent).is_some());
        }
        for (parent, thread) in &mut s.open_threads {
            if rebuilt {
                *thread = nested(parent, main);
                continue;
            }
            // `main` continued: append as live, whatever `live` says, so a history split over
            // several messages does not replace a thread built from fewer events.
            let own: Vec<SequencedEvent> = events
                .iter()
                .filter(|e| is_subagent_event(&e.event, parent))
                .cloned()
                .collect();
            if !own.is_empty() {
                thread.apply(0, &own, true);
            }
            if evicted && thread.held() != main.subagent_events(parent).count() {
                *thread = nested(parent, main);
            }
        }
    }

    /// `session`'s insights, when this window has heard of it.
    pub fn insights(&self, session: SessionId) -> Option<&SessionInsights> {
        self.sessions.get(&session).map(|s| &s.insights)
    }

    /// The figures of the turn whose footer is `footer` (FR-006): a footer is keyed by its
    /// result event.
    pub fn turn(&self, session: SessionId, footer: &EntryKey) -> Option<&TurnUsage> {
        let EntryKey::At { seq, .. } = footer else {
            return None;
        };
        self.insights(session)?.turn(*seq)
    }

    /// Unfold the thread of `parent`, built from `main`'s held events, or fold it when it is
    /// open (FR-015). A thread no held event names opens nothing.
    pub fn toggle_thread(&mut self, session: SessionId, parent: &str, main: &Transcript) {
        let Some(s) = self.sessions.get_mut(&session) else {
            return;
        };
        if s.open_threads.remove(parent).is_none() && s.insights.thread(parent).is_some() {
            s.open_threads
                .insert(parent.to_string(), nested(parent, main));
        }
    }

    /// The open thread of `parent`; `None` while it is folded.
    pub fn thread(&self, session: SessionId, parent: &str) -> Option<&Transcript> {
        self.sessions.get(&session)?.open_threads.get(parent)
    }

    /// The thread of `parent` with its summary state (FR-015), `card` being its parent card's
    /// status; a running subagent of a session whose CLI is not running shows as ended
    /// (FR-018).
    pub fn thread_view(
        &self,
        session: SessionId,
        parent: &str,
        card: Option<ToolStatus>,
        cli_running: bool,
    ) -> Option<ThreadView<'_>> {
        let mut view = self.insights(session)?.thread_view(parent, card)?;
        if !cli_running && view.state == SubagentState::Running {
            view.state = SubagentState::EndedWithoutResult;
        }
        Some(view)
    }

    /// How many of `session`'s background tasks are running (FR-020, FR-022).
    pub fn running_background(&self, session: SessionId, cli_running: bool) -> usize {
        self.insights(session)
            .map_or(0, |i| i.running_background(cli_running))
    }

    /// Unfold the background-task list, or fold it.
    pub fn toggle_tasks(&mut self, session: SessionId) {
        if let Some(s) = self.sessions.get_mut(&session) {
            s.tasks_open = !s.tasks_open;
        }
    }

    /// Whether the background-task list is unfolded; folded by default.
    pub fn tasks_open(&self, session: SessionId) -> bool {
        self.sessions.get(&session).is_some_and(|s| s.tasks_open)
    }

    /// Forget `session`'s insights and folds (the session was closed or removed).
    pub fn forget(&mut self, session: SessionId) {
        self.sessions.remove(&session);
    }

    /// Apply a message of the view; `main` is the session's transcript.
    pub fn update(&mut self, msg: InsightsMsg, main: &Transcript) {
        match msg {
            InsightsMsg::ToggleThread { session, parent } => {
                self.toggle_thread(session, &parent, main)
            }
            InsightsMsg::ToggleBackgroundTasks { session } => self.toggle_tasks(session),
        }
    }
}
