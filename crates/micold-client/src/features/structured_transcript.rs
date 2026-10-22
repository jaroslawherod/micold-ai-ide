//! The structured sessions' transcripts this window holds (feature 939, data-model.md *View
//! state*): one [`Transcript`] per session, with the cards the user unfolded and whether the view
//! follows the end.
//!
//! Render-free: this holds the state, `ui/transcript/` draws it and the
//! shell turns a [`Scroll`] into a scroll operation on `ui::transcript::transcript_scroll_id`
//! (research R9).

use crate::features::transcript_rich::{HighlightJob, Part, RichCache, Ticket, COPY_CONFIRM};
use crate::features::{OpenRequest, Outcome};
use micold_core::link::address::{classify, Address};
use micold_core::markdown::CodeId;
use micold_core::review::diff::Spans;
use micold_core::session::SessionId;
use micold_core::structured::transcript::{EntryKey, Transcript};
use micold_core::structured::{SequencedEvent, MAX_EVENTS};
use std::collections::{HashMap, HashSet};

/// How close to the end, in whole pixels, a scroll position still counts as at the end (R9), so
/// momentum and fractional trackpad steps land on it on every platform.
pub const END_SLACK: u32 = 4;

/// Whether the view keeps the end in sight (FR-017, FR-018).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Follow {
    /// At the end: growth keeps the end in view. Where a transcript starts (FR-018).
    #[default]
    Following,
    /// Scrolled away from the end, at `offset` pixels from the top: growth moves nothing.
    Detached {
        /// The scroll offset the user left the view at.
        offset: u32,
    },
}

impl Follow {
    /// The follow state after the view reported `offset` within `content` pixels, `viewport` of
    /// them visible, and what changed (R9): at the end iff `offset + viewport + END_SLACK >=
    /// content`.
    pub fn scrolled(self, offset: u32, viewport: u32, content: u32) -> (Self, FollowChange) {
        let at_end = offset.saturating_add(viewport).saturating_add(END_SLACK) >= content;
        match (self, at_end) {
            (Self::Following, true) => (self, FollowChange::None),
            (Self::Detached { .. }, true) => (Self::Following, FollowChange::Resumed),
            (Self::Following, false) => (Self::Detached { offset }, FollowChange::Detached),
            (Self::Detached { .. }, false) => (Self::Detached { offset }, FollowChange::None),
        }
    }
}

/// What a scroll report changed (R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FollowChange {
    /// Following stayed as it was.
    None,
    /// The user scrolled away from the end: following stopped.
    Detached,
    /// The user scrolled back to the end: following resumed, and the shell snaps to the end.
    Resumed,
}

/// The scroll operation a transcript message asks the shell for (R9).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scroll {
    /// `snap_to(RelativeOffset::END)`: keep the end in view as content grows.
    End,
    /// `scroll_to(AbsoluteOffset { y })`: back where the user left a detached view.
    To(u32),
}

/// What the transcript view says (data-model.md *Messages*), carried by the session message
/// family as `session::Msg::Transcript`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptMsg {
    /// A card's header was activated: unfold or fold it (FR-010).
    Toggle {
        /// The session whose transcript holds the card.
        session: SessionId,
        /// The card.
        key: EntryKey,
    },
    /// The view scrolled, or its content or viewport changed size; whole pixels.
    Scrolled {
        /// The session the view shows.
        session: SessionId,
        /// The offset from the top.
        offset: u32,
        /// The visible height.
        viewport: u32,
        /// The content's height.
        content: u32,
    },
    /// "Jump to latest" was activated (FR-017).
    JumpToLatest {
        /// The session the view shows.
        session: SessionId,
    },
    /// The view's scrollable was laid out for the first time: a transcript was shown (R9).
    Shown {
        /// The session the view shows.
        session: SessionId,
    },
    /// A link in a rich block was activated (feature 940, FR-004, RR-6).
    OpenLink {
        /// Its address, re-classified before anything opens.
        url: String,
    },
    /// A code block's copy control was activated (feature 940, FR-011).
    CopyCode {
        session: SessionId,
        key: EntryKey,
        code: CodeId,
    },
    /// The clipboard read back after a copy's write (feature 940, FR-012, research R5).
    CopyChecked {
        ticket: Ticket,
        /// What the clipboard held; `None` when it could not be read.
        read: Option<String>,
    },
    /// A copy's confirmation is over (feature 940, FR-012).
    CopyExpired { ticket: Ticket },
    /// "Show in full" was activated on a part over the size limits (feature 940, FR-019).
    ShowInFull {
        session: SessionId,
        key: EntryKey,
        part: Part,
    },
    /// A highlight job's colours, in both schemes (feature 940, research R4).
    Highlighted {
        session: SessionId,
        key: EntryKey,
        part: Part,
        spans: Spans,
    },
    /// A message of what feature 944 draws in the transcript (a thread, a card inside one, the
    /// background-task list): the insights feature's, which the window's view hands to it as
    /// `session::Msg::Insights`; this feature does nothing with it.
    Insights(crate::features::structured_insights::InsightsMsg),
}

impl TranscriptMsg {
    /// The session whose view sent it; `None` for the window-wide answers (links, copy tickets).
    pub fn session(&self) -> Option<SessionId> {
        match self {
            Self::Toggle { session, .. }
            | Self::Scrolled { session, .. }
            | Self::JumpToLatest { session }
            | Self::Shown { session }
            | Self::CopyCode { session, .. }
            | Self::ShowInFull { session, .. }
            | Self::Highlighted { session, .. } => Some(*session),
            Self::Insights(msg) => Some(msg.session()),
            Self::OpenLink { .. } | Self::CopyChecked { .. } | Self::CopyExpired { .. } => None,
        }
    }
}

/// What a transcript message asks of the shell: a scroll (R9), effect requests (a link to open, a
/// checked clipboard write, a confirmation timer) and highlight jobs (feature 940).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Effect {
    pub scroll: Option<Scroll>,
    pub outcomes: Vec<Outcome>,
    pub jobs: Vec<HighlightJob>,
}

impl Effect {
    fn scroll(scroll: Option<Scroll>) -> Self {
        Self {
            scroll,
            ..Self::default()
        }
    }

    fn outcome(outcome: Option<Outcome>) -> Self {
        Self {
            outcomes: outcome.into_iter().collect(),
            ..Self::default()
        }
    }
}

/// The structured sessions' transcripts this window holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    sessions: HashMap<SessionId, SessionTranscript>,
}

/// One session's transcript and its view state.
#[derive(Debug, Clone, Default)]
struct SessionTranscript {
    transcript: Transcript,
    /// The unfolded cards; a key absent is folded (FR-010).
    open: HashSet<EntryKey>,
    follow: Follow,
}

/// Two session transcripts are equal when they hold the same entries at the same revisions, the
/// same cards open and the same follow state. A revision names one content for as long as the
/// transcript lives (data-model.md *Entry*), so this compares what the view would draw without
/// comparing every event.
impl PartialEq for SessionTranscript {
    fn eq(&self, other: &Self) -> bool {
        let shown = |t: &Transcript| {
            (
                t.rebuilds(),
                t.held(),
                t.omitted_notice(),
                t.entries()
                    .map(|e| (e.key().clone(), e.revision()))
                    .collect::<Vec<_>>(),
            )
        };
        self.open == other.open
            && self.follow == other.follow
            && shown(&self.transcript) == shown(&other.transcript)
    }
}

impl Eq for SessionTranscript {}

impl State {
    /// Take one `StructuredEvents` message for `session` (#938 wire.md) into its transcript.
    /// After a rebuild, or once the bound may have dropped entries from the front (FR-020), only
    /// the open cards still present stay open (FR-013, FR-021).
    pub fn apply(
        &mut self,
        session: SessionId,
        omitted: u64,
        events: &[SequencedEvent],
        live: bool,
    ) {
        let s = self.sessions.entry(session).or_default();
        let rebuilds = s.transcript.rebuilds();
        s.transcript.apply(omitted, events, live);
        let bounded = s.transcript.held() >= MAX_EVENTS;
        if s.transcript.rebuilds() != rebuilds || bounded && !s.open.is_empty() {
            let transcript = &s.transcript;
            s.open.retain(|key| transcript.entry(key).is_some());
        }
    }

    /// `session`'s transcript, when this window has heard of it.
    pub fn transcript(&self, session: SessionId) -> Option<&Transcript> {
        self.sessions.get(&session).map(|s| &s.transcript)
    }

    /// Unfold the card `key`, or fold it when it is open (FR-010).
    pub fn toggle(&mut self, session: SessionId, key: EntryKey) {
        let Some(s) = self.sessions.get_mut(&session) else {
            return;
        };
        // A late toggle of a card that has left opens nothing.
        if !s.open.remove(&key) && s.transcript.entry(&key).is_some() {
            s.open.insert(key);
        }
    }

    /// Whether the card `key` is unfolded.
    pub fn is_open(&self, session: SessionId, key: &EntryKey) -> bool {
        self.sessions
            .get(&session)
            .is_some_and(|s| s.open.contains(key))
    }

    /// The view of `session` reported a scroll position (R9).
    pub fn scrolled(
        &mut self,
        session: SessionId,
        offset: u32,
        viewport: u32,
        content: u32,
    ) -> FollowChange {
        let Some(s) = self.sessions.get_mut(&session) else {
            return FollowChange::None;
        };
        let (follow, change) = s.follow.scrolled(offset, viewport, content);
        s.follow = follow;
        change
    }

    /// "Jump to latest": follow the end again (FR-017).
    pub fn jump_to_latest(&mut self, session: SessionId) {
        if let Some(s) = self.sessions.get_mut(&session) {
            s.follow = Follow::Following;
        }
    }

    /// Whether the view of `session` follows the end; a transcript not yet seen starts following
    /// (FR-018).
    pub fn follow(&self, session: SessionId) -> Follow {
        self.sessions
            .get(&session)
            .map_or(Follow::Following, |s| s.follow)
    }

    /// Forget `session`'s transcript and view state (the session was closed or removed).
    pub fn forget(&mut self, session: SessionId) {
        self.sessions.remove(&session);
    }

    /// Apply a message of the view, and say what the shell performs: the scroll (R9), and for the
    /// rich blocks' messages what `rich`, the window's render cache, asks for (feature 940).
    pub fn update(&mut self, rich: &mut RichCache, msg: TranscriptMsg) -> Effect {
        match msg {
            TranscriptMsg::Toggle { session, key } => {
                self.toggle(session, key);
                Effect::default()
            }
            TranscriptMsg::Scrolled {
                session,
                offset,
                viewport,
                content,
            } => Effect::scroll(
                (self.scrolled(session, offset, viewport, content) == FollowChange::Resumed)
                    .then_some(Scroll::End),
            ),
            TranscriptMsg::JumpToLatest { session } => {
                self.jump_to_latest(session);
                Effect::scroll(Some(Scroll::End))
            }
            TranscriptMsg::Shown { session } => Effect::scroll(Some(match self.follow(session) {
                Follow::Following => Scroll::End,
                Follow::Detached { offset } => Scroll::To(offset),
            })),
            // RR-6: a renderer sends only activatable links; this re-checks a message built
            // elsewhere.
            TranscriptMsg::OpenLink { url } => Effect::outcome(match classify(&url) {
                Address::Web(url) | Address::Mail(url) => {
                    Some(Outcome::OpenLink(OpenRequest::Url(url)))
                }
                Address::File { .. } | Address::NotFollowable => None,
            }),
            TranscriptMsg::CopyCode { session, key, code } => Effect::outcome(
                rich.copy(session, &key, code)
                    .map(|(text, ticket)| Outcome::ClipboardWriteChecked { text, ticket }),
            ),
            TranscriptMsg::CopyChecked { ticket, read } => Effect::outcome(
                rich.copy_checked(ticket, read)
                    .then_some(Outcome::CopyConfirmTimer {
                        ticket,
                        after: COPY_CONFIRM,
                    }),
            ),
            TranscriptMsg::CopyExpired { ticket } => {
                rich.copy_expired(ticket);
                Effect::default()
            }
            TranscriptMsg::ShowInFull { session, key, part } => Effect {
                jobs: rich.show_in_full(session, &key, part).into_iter().collect(),
                ..Effect::default()
            },
            TranscriptMsg::Highlighted {
                session,
                key,
                part,
                spans,
            } => {
                rich.highlighted(session, &key, part, spans);
                Effect::default()
            }
            TranscriptMsg::Insights(_) => Effect::default(),
        }
    }
}
