//! Which session a window has in view (feature 039, FR-002, FR-016, spec Terms).
//!
//! Render-free. The client fills [`ViewFacts`] from its own state and reports [`in_view`]'s answer
//! to the session service, which counts an attention event only for a session that no window has
//! in view.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::clock::Uptime;
use crate::project::Availability;
use crate::protocol::messages::{ActivitySignal, SessionSummary};
use crate::session::{Session, SessionId};
use crate::workspace::Workspace;

/// What decides whether a window has a session in view.
///
/// There is no field for the tab the window shows: a regular terminal tab of the selected session
/// leaves that session in view (US1 scenario 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFacts {
    /// The window has keyboard focus.
    pub window_focused: bool,
    /// A screen fills the main area instead of the session (Settings, a Changes view).
    pub main_area_taken: bool,
    /// The active project's selected session.
    pub selected: Option<SessionId>,
}

/// The session the window has in view: the selected one, while the window is focused and its main
/// area shows the session.
pub fn in_view(facts: ViewFacts) -> Option<SessionId> {
    if facts.window_focused && !facts.main_area_taken {
        facts.selected
    } else {
        None
    }
}

/// What the tracker last saw of one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Seen {
    /// The `attention_seq` last seen.
    pub seq: u64,
    /// Whether the activity last seen was `AwaitingInput`.
    pub awaiting: bool,
}

/// A request to be the window that raises the notification for one attention event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Claim {
    /// The session that changed.
    pub session: SessionId,
    /// The `attention_seq` being claimed.
    pub seq: u64,
}

/// Whether the connection was unbroken since the last snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Unbroken: every higher sequence is a change this window watched happen.
    Live,
    /// The first snapshot after a lost connection.
    Reconnected,
}

/// Per process and in memory: what this window last saw of each session (research R3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttentionTracker {
    seen: HashMap<SessionId, Seen>,
}

impl AttentionTracker {
    /// A tracker that has seen nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// The sessions of one full catalog snapshot, and the claims they give rise to.
    pub fn observe(
        &mut self,
        sessions: &[SessionSummary],
        phase: Phase,
        in_view: Option<SessionId>,
    ) -> Vec<Claim> {
        let mut claims = Vec::new();
        let mut next = HashMap::with_capacity(sessions.len());
        for summary in sessions {
            let now = Seen {
                seq: summary.attention_seq,
                awaiting: summary.activity == ActivitySignal::AwaitingInput,
            };
            if let Some(before) = self.seen.get(&summary.id) {
                let higher = now.seq > before.seq;
                let claimed = match phase {
                    Phase::Live => higher,
                    Phase::Reconnected => {
                        higher && !before.awaiting && now.awaiting && in_view != Some(summary.id)
                    }
                };
                if claimed {
                    claims.push(Claim {
                        session: summary.id,
                        seq: now.seq,
                    });
                }
            }
            next.insert(summary.id, now);
        }
        self.seen = next;
        claims
    }

    /// What was last seen of `session`.
    pub fn seen(&self, session: SessionId) -> Option<Seen> {
        self.seen.get(&session).copied()
    }

    /// How many sessions are remembered.
    pub fn len(&self) -> usize {
        self.seen.len()
    }

    /// Whether nothing is remembered.
    pub fn is_empty(&self) -> bool {
        self.seen.is_empty()
    }
}

/// How long a turn must last for its end to be **Long task finished** by default (FR-002, FR-025).
/// The only definition of this default (contract C6); the setting
/// `Settings::long_task_threshold_secs` overrides it.
pub const LONG_TASK_THRESHOLD: Duration = Duration::from_secs(60);

/// What kind of event a desktop notification is for (feature 613, FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    /// The session stopped mid-turn to ask for a permission or an answer.
    NeedsPermission,
    /// The session stopped because of an error.
    SessionError,
    /// The session finished a turn of at least [`LONG_TASK_THRESHOLD`].
    LongTaskFinished,
    /// The session finished a shorter turn.
    TurnFinished,
}

impl NotificationKind {
    /// Every kind, in the order Settings lists them (FR-009).
    pub const ALL: [NotificationKind; 4] = [
        NotificationKind::NeedsPermission,
        NotificationKind::SessionError,
        NotificationKind::LongTaskFinished,
        NotificationKind::TurnFinished,
    ];

    /// The kind's name, as Settings shows it.
    pub fn name(self) -> &'static str {
        match self {
            NotificationKind::NeedsPermission => "Needs permission",
            NotificationKind::SessionError => "Session error",
            NotificationKind::LongTaskFinished => "Long task finished",
            NotificationKind::TurnFinished => "Turn finished",
        }
    }

    /// The kind's note in Settings.
    pub fn description(self) -> String {
        match self {
            NotificationKind::NeedsPermission => {
                "A session stopped to ask for a permission or an answer.".to_string()
            }
            NotificationKind::SessionError => "A session stopped because of an error.".to_string(),
            NotificationKind::LongTaskFinished => {
                "A session finished a turn at least as long as the long-task threshold.".to_string()
            }
            NotificationKind::TurnFinished => "A session finished a shorter turn.".to_string(),
        }
    }

    /// Whether the kind is on in a fresh installation (FR-010).
    pub fn default_on(self) -> bool {
        self != NotificationKind::TurnFinished
    }
}

/// Which kinds notify (FR-010, FR-011). A field per kind, so a settings file that lacks one gets
/// that one's default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotificationKinds {
    /// **Needs permission**.
    #[serde(default = "on")]
    pub needs_permission: bool,
    /// **Session error**.
    #[serde(default = "on")]
    pub session_error: bool,
    /// **Long task finished**.
    #[serde(default = "on")]
    pub long_task_finished: bool,
    /// **Turn finished**.
    #[serde(default)]
    pub turn_finished: bool,
}

fn on() -> bool {
    true
}

impl Default for NotificationKinds {
    fn default() -> Self {
        Self {
            needs_permission: NotificationKind::NeedsPermission.default_on(),
            session_error: NotificationKind::SessionError.default_on(),
            long_task_finished: NotificationKind::LongTaskFinished.default_on(),
            turn_finished: NotificationKind::TurnFinished.default_on(),
        }
    }
}

impl NotificationKinds {
    /// Whether `kind` is on.
    pub fn is_on(&self, kind: NotificationKind) -> bool {
        match kind {
            NotificationKind::NeedsPermission => self.needs_permission,
            NotificationKind::SessionError => self.session_error,
            NotificationKind::LongTaskFinished => self.long_task_finished,
            NotificationKind::TurnFinished => self.turn_finished,
        }
    }

    /// Turn `kind` on or off, leaving the others as they are.
    pub fn set(&mut self, kind: NotificationKind, on: bool) {
        let field = match kind {
            NotificationKind::NeedsPermission => &mut self.needs_permission,
            NotificationKind::SessionError => &mut self.session_error,
            NotificationKind::LongTaskFinished => &mut self.long_task_finished,
            NotificationKind::TurnFinished => &mut self.turn_finished,
        };
        *field = on;
    }
}

/// What the service saw a session do, as far as its turn is concerned (data-model "TurnClock").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnChange {
    /// The user sent a prompt: a turn starts.
    PromptSubmitted,
    /// The session worked.
    Working,
    /// The session stopped mid-turn to ask for a permission or an answer.
    AskedUser,
    /// The session finished its turn.
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum TurnState {
    #[default]
    NotInTurn,
    Working {
        since: Uptime,
    },
    Paused {
        since: Uptime,
    },
}

/// One live session's turn: when it started, and whether it is paused on the user (FR-002, FR-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TurnClock {
    state: TurnState,
}

impl TurnClock {
    /// A clock not in a turn.
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply `change` at `now` (data-model transition table); the kind an attention event at this
    /// change would have. The caller uses it only when the change counts as an attention event.
    pub fn change(
        &mut self,
        change: TurnChange,
        now: Uptime,
        threshold: Duration,
    ) -> Option<NotificationKind> {
        let (next, kind) = match (self.state, change) {
            (_, TurnChange::PromptSubmitted) => (TurnState::Working { since: now }, None),
            (TurnState::NotInTurn, TurnChange::Working) => {
                (TurnState::Working { since: now }, None)
            }
            (TurnState::Working { since } | TurnState::Paused { since }, TurnChange::Working) => {
                (TurnState::Working { since }, None)
            }
            (TurnState::NotInTurn, TurnChange::AskedUser | TurnChange::Finished) => {
                (TurnState::NotInTurn, Some(NotificationKind::TurnFinished))
            }
            (TurnState::Working { since }, TurnChange::AskedUser) => (
                TurnState::Paused { since },
                Some(NotificationKind::NeedsPermission),
            ),
            (TurnState::Paused { since }, TurnChange::AskedUser) => {
                (TurnState::Paused { since }, None)
            }
            (TurnState::Working { since } | TurnState::Paused { since }, TurnChange::Finished) => {
                let kind = if now.saturating_sub(since) >= threshold {
                    NotificationKind::LongTaskFinished
                } else {
                    NotificationKind::TurnFinished
                };
                (TurnState::NotInTurn, Some(kind))
            }
        };
        self.state = next;
        kind
    }
}

/// The text of a desktop notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationText {
    /// The title.
    pub title: String,
    /// The body.
    pub body: String,
}

/// The text of a notification of `kind`: the three names and the fixed words, nothing else
/// (FR-008, contract T1, T2).
pub fn notification_text(
    kind: NotificationKind,
    project: &str,
    worktree: &str,
    session: &str,
) -> NotificationText {
    let what = match kind {
        NotificationKind::NeedsPermission => "needs permission",
        NotificationKind::SessionError => "stopped with an error",
        NotificationKind::LongTaskFinished => "finished a long task",
        NotificationKind::TurnFinished => "finished its turn",
    };
    NotificationText {
        title: format!("{session} {what}"),
        body: format!("{project} \u{2014} {worktree}"),
    }
}

/// What a window does with a request to show a session (research R6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reveal {
    /// Make `project` the active one, when it is not, and select `session`.
    Show {
        /// The project that holds the session.
        project: PathBuf,
        /// The session to select.
        session: SessionId,
    },
    /// The session cannot be shown: change no selection and say so (FR-013).
    Unavailable,
}

/// What to do with a request to show `session` of `project` (FR-011, FR-013): [`Reveal::Show`]
/// when the project is known, its folder is available and it holds the session, not closed;
/// otherwise [`Reveal::Unavailable`] — the session was closed or removed, or the project was
/// forgotten or its folder is gone. A closed session keeps its record, flagged `archived`, and has
/// no row.
pub fn resolve_reveal(workspace: &Workspace, project: &Path, session: SessionId) -> Reveal {
    let known_and_available = workspace
        .projects
        .iter()
        .any(|p| p.path == project && p.availability == Availability::Available);
    let holds_the_session = workspace
        .sessions
        .get(project)
        .is_some_and(|sessions| sessions.iter().any(|s| s.id == session && !s.archived));
    if known_and_available && holds_the_session {
        Reveal::Show {
            project: project.to_path_buf(),
            session,
        }
    } else {
        Reveal::Unavailable
    }
}

/// Whether `session` counts as unread for a location's or a project's attention mark (feature
/// 575, data-model "Counted session").
///
/// Unread in 039's sense (FR-016), not the session the window has in view (039 FR-019: a session
/// in view is read the moment it is), and not closed (039 US1 scenario 9: a closed session is out
/// of the user's way and must not call them back to it).
pub fn counts_as_unread(session: &Session, in_view: Option<SessionId>) -> bool {
    session.unread && !session.archived && in_view != Some(session.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(selected: Option<SessionId>) -> ViewFacts {
        ViewFacts {
            window_focused: true,
            main_area_taken: false,
            selected,
        }
    }

    #[test]
    fn a_focused_window_showing_its_session_has_the_selected_session_in_view() {
        let session = SessionId::new();
        assert_eq!(
            in_view(facts(Some(session))),
            Some(session),
            "a focused window whose main area shows the session has that session in view"
        );
    }

    #[test]
    fn an_unfocused_window_has_no_session_in_view() {
        let unfocused = ViewFacts {
            window_focused: false,
            ..facts(Some(SessionId::new()))
        };
        assert_eq!(
            in_view(unfocused),
            None,
            "a window without keyboard focus shows its session to nobody (US1 scenario 3)"
        );
    }

    #[test]
    fn a_window_whose_main_area_is_taken_has_no_session_in_view() {
        let settings_open = ViewFacts {
            main_area_taken: true,
            ..facts(Some(SessionId::new()))
        };
        assert_eq!(
            in_view(settings_open),
            None,
            "a screen that fills the main area hides the selected session (US1 scenario 9)"
        );
    }

    #[test]
    fn a_window_with_no_selected_session_has_no_session_in_view() {
        assert_eq!(
            in_view(facts(None)),
            None,
            "with nothing selected there is nothing to have in view"
        );
    }

    #[test]
    fn the_shown_tab_is_not_one_of_the_facts() {
        // US1 scenario 10: the session stays in view whichever of its tabs is shown. The rule
        // holds by shape, so this names every field: a field added for the tab fails to compile
        // here, and the answer below is decided by these three alone.
        let session = SessionId::new();
        let ViewFacts {
            window_focused,
            main_area_taken,
            selected,
        } = facts(Some(session));
        assert_eq!(
            in_view(ViewFacts {
                window_focused,
                main_area_taken,
                selected,
            }),
            Some(session),
            "the three facts decide the answer; which tab is shown is not among them"
        );
    }

    // --- AttentionTracker (U18–U27) ---

    use crate::protocol::messages::WireLifecycle;
    use crate::session::{AiCli, SessionLabel};

    fn id(n: u128) -> SessionId {
        SessionId::from_uuid(uuid::Uuid::from_u128(n))
    }

    fn summary(session: SessionId, seq: u64, activity: ActivitySignal) -> SessionSummary {
        SessionSummary {
            id: session,
            worktree_dir: None,
            title: SessionLabel::Pending,
            lifecycle: WireLifecycle::Running,
            activity,
            provider: AiCli::ClaudeCode,
            input_serial: 0,
            live_shells: Vec::new(),
            attention_seq: seq,
            unread: false,
        }
    }

    fn claim(session: SessionId, seq: u64) -> Claim {
        Claim { session, seq }
    }

    /// A tracker that has adopted `session` at `seq` with `activity`.
    fn adopted(session: SessionId, seq: u64, activity: ActivitySignal) -> AttentionTracker {
        let mut tracker = AttentionTracker::new();
        let first = tracker.observe(&[summary(session, seq, activity)], Phase::Live, None);
        assert!(first.is_empty(), "precondition: adopting claims nothing");
        tracker
    }

    #[test]
    fn u18_a_session_not_seen_before_is_adopted_with_no_claim_even_when_awaiting() {
        let session = id(1);
        let mut tracker = AttentionTracker::new();
        let claims = tracker.observe(
            &[summary(session, 3, ActivitySignal::AwaitingInput)],
            Phase::Live,
            None,
        );
        assert_eq!(
            claims,
            vec![],
            "a session first seen is never claimed (FR-005)"
        );
        assert_eq!(
            tracker.seen(session),
            Some(Seen {
                seq: 3,
                awaiting: true
            }),
            "its values are adopted"
        );
    }

    #[test]
    fn u19_a_higher_sequence_in_live_phase_is_claimed() {
        let session = id(1);
        let mut tracker = adopted(session, 1, ActivitySignal::Working);
        let claims = tracker.observe(
            &[summary(session, 2, ActivitySignal::AwaitingInput)],
            Phase::Live,
            None,
        );
        assert_eq!(
            claims,
            vec![claim(session, 2)],
            "the new sequence is claimed"
        );
    }

    #[test]
    fn u20_the_same_sequence_again_is_not_claimed() {
        let session = id(1);
        let mut tracker = adopted(session, 1, ActivitySignal::Working);
        let snapshot = [summary(session, 2, ActivitySignal::AwaitingInput)];
        assert_eq!(
            tracker.observe(&snapshot, Phase::Live, None).len(),
            1,
            "precondition: the first sight of sequence 2 is claimed"
        );
        assert_eq!(
            tracker.observe(&snapshot, Phase::Live, None),
            vec![],
            "the same sequence is claimed once"
        );
    }

    #[test]
    fn u21_reconnected_claims_a_change_into_awaiting_that_is_not_in_view() {
        let session = id(1);
        let mut tracker = adopted(session, 1, ActivitySignal::Working);
        let claims = tracker.observe(
            &[summary(session, 2, ActivitySignal::AwaitingInput)],
            Phase::Reconnected,
            None,
        );
        assert_eq!(
            claims,
            vec![claim(session, 2)],
            "a change found after reconnecting is claimed (FR-006)"
        );
    }

    #[test]
    fn u22_reconnected_adopts_when_the_last_seen_activity_was_awaiting() {
        let session = id(1);
        let mut tracker = adopted(session, 1, ActivitySignal::AwaitingInput);
        let claims = tracker.observe(
            &[summary(session, 2, ActivitySignal::AwaitingInput)],
            Phase::Reconnected,
            None,
        );
        assert_eq!(claims, vec![], "the user had already been told");
        assert_eq!(
            tracker.seen(session).map(|seen| seen.seq),
            Some(2),
            "the sequence is adopted"
        );
    }

    #[test]
    fn u23_reconnected_does_not_claim_a_session_in_view() {
        let session = id(1);
        let mut tracker = adopted(session, 1, ActivitySignal::Working);
        let claims = tracker.observe(
            &[summary(session, 2, ActivitySignal::AwaitingInput)],
            Phase::Reconnected,
            Some(session),
        );
        assert_eq!(claims, vec![], "the user is looking at it (FR-002)");
    }

    #[test]
    fn u24_reconnected_does_not_claim_a_session_no_longer_awaiting() {
        let session = id(1);
        let mut tracker = adopted(session, 1, ActivitySignal::Working);
        let claims = tracker.observe(
            &[summary(session, 2, ActivitySignal::Working)],
            Phase::Reconnected,
            None,
        );
        assert_eq!(claims, vec![], "it is not waiting now");
    }

    #[test]
    fn u25_a_lower_sequence_is_adopted_with_no_claim() {
        let session = id(1);
        let mut tracker = adopted(session, 5, ActivitySignal::Working);
        let claims = tracker.observe(
            &[summary(session, 2, ActivitySignal::AwaitingInput)],
            Phase::Live,
            None,
        );
        assert_eq!(claims, vec![], "a recovered catalog claims nothing");
        assert_eq!(
            tracker.seen(session).map(|seen| seen.seq),
            Some(2),
            "the lower sequence is adopted"
        );
    }

    #[test]
    fn u26_ten_sessions_with_higher_sequences_give_ten_claims() {
        let ids: Vec<SessionId> = (1..=10).map(id).collect();
        let at = |seq: u64| -> Vec<SessionSummary> {
            ids.iter()
                .map(|&s| summary(s, seq, ActivitySignal::AwaitingInput))
                .collect()
        };
        let mut tracker = AttentionTracker::new();
        tracker.observe(&at(0), Phase::Live, None);
        let claims = tracker.observe(&at(1), Phase::Live, None);
        let expected: Vec<Claim> = ids.iter().map(|&s| claim(s, 1)).collect();
        assert_eq!(
            claims, expected,
            "each session is claimed under its own id (FR-009)"
        );
    }

    #[test]
    fn u27_a_session_absent_from_the_snapshot_is_dropped() {
        let (kept, gone) = (id(1), id(2));
        let mut tracker = AttentionTracker::new();
        tracker.observe(
            &[
                summary(kept, 0, ActivitySignal::Working),
                summary(gone, 0, ActivitySignal::Working),
            ],
            Phase::Live,
            None,
        );
        tracker.observe(
            &[summary(kept, 0, ActivitySignal::Working)],
            Phase::Live,
            None,
        );
        assert_eq!(tracker.seen(gone), None, "the absent session is forgotten");
        assert_eq!(tracker.len(), 1, "only the present session remains");
    }

    // --- notification_text (U28–U31; 613 T1, T2) ---

    const KIND_TITLES: [(NotificationKind, &str); 4] = [
        (
            NotificationKind::NeedsPermission,
            "Fix the parser needs permission",
        ),
        (
            NotificationKind::SessionError,
            "Fix the parser stopped with an error",
        ),
        (
            NotificationKind::LongTaskFinished,
            "Fix the parser finished a long task",
        ),
        (
            NotificationKind::TurnFinished,
            "Fix the parser finished its turn",
        ),
    ];

    #[test]
    fn the_title_names_the_session_and_states_the_kind() {
        for (kind, title) in KIND_TITLES {
            let text = notification_text(kind, "micold", "feat-x", "Fix the parser");
            assert_eq!(text.title, title, "the title states {kind:?} in words (T1)");
        }
    }

    #[test]
    fn u28_the_body_names_the_project_and_worktree_for_every_kind() {
        for kind in NotificationKind::ALL {
            let text = notification_text(kind, "micold", "feat-x", "Fix the parser");
            assert_eq!(
                text.body, "micold \u{2014} feat-x",
                "the body is the same for {kind:?}"
            );
        }
    }

    #[test]
    fn u29_the_default_entry_name_is_passed_through_unchanged() {
        let text = notification_text(
            NotificationKind::LongTaskFinished,
            "micold",
            "Default",
            "Fix the parser",
        );
        assert_eq!(text.body, "micold \u{2014} Default");
    }

    #[test]
    fn u30_a_placeholder_session_label_is_passed_through_unchanged() {
        let text = notification_text(
            NotificationKind::NeedsPermission,
            "micold",
            "feat-x",
            "New session",
        );
        assert_eq!(text.title, "New session needs permission");
    }

    #[test]
    fn u31_neither_string_holds_text_besides_the_three_names_and_the_fixed_words() {
        let fixed = [
            (NotificationKind::NeedsPermission, " needs permission"),
            (NotificationKind::SessionError, " stopped with an error"),
            (NotificationKind::LongTaskFinished, " finished a long task"),
            (NotificationKind::TurnFinished, " finished its turn"),
        ];
        for (kind, words) in fixed {
            let text = notification_text(kind, "PROJ", "TREE", "SESS");
            assert_eq!(
                text.title.replace("SESS", ""),
                words,
                "the title is the session name and fixed words only (T2)"
            );
            assert_eq!(
                text.body.replace("PROJ", "").replace("TREE", ""),
                " \u{2014} ",
                "the body is the project, the worktree and a dash only (T2)"
            );
        }
    }

    // --- resolve_reveal (U32–U35) ---

    use crate::project::Project;
    use crate::session::{Session, SessionLocation, TerminalMode};

    const REPO: &str = "/repo";

    /// A workspace holding `REPO` with `availability` and one session, `held`.
    fn workspace_with(availability: Availability, held: SessionId) -> Workspace {
        let mut workspace = Workspace::default();
        workspace
            .projects
            .push(Project::new(PathBuf::from(REPO), true, availability));
        workspace.sessions.insert(
            PathBuf::from(REPO),
            vec![Session::restored(
                held,
                SessionLocation::Default,
                SessionLabel::Pending,
                TerminalMode::AiCli,
                AiCli::ClaudeCode,
            )],
        );
        workspace
    }

    #[test]
    fn u32_a_known_available_project_holding_the_session_resolves_to_show() {
        let workspace = workspace_with(Availability::Available, id(1));
        assert_eq!(
            resolve_reveal(&workspace, Path::new(REPO), id(1)),
            Reveal::Show {
                project: PathBuf::from(REPO),
                session: id(1),
            }
        );
    }

    #[test]
    fn u33_a_session_that_was_removed_resolves_to_unavailable() {
        let workspace = workspace_with(Availability::Available, id(1));
        assert_eq!(
            resolve_reveal(&workspace, Path::new(REPO), id(2)),
            Reveal::Unavailable,
            "the project holds no session with that id"
        );
    }

    #[test]
    fn u33_a_session_the_user_closed_resolves_to_unavailable() {
        // M6 review A F1: closing keeps the record and flags it `archived`; it has no row.
        let mut workspace = workspace_with(Availability::Available, id(1));
        workspace
            .sessions
            .get_mut(Path::new(REPO))
            .expect("the project's sessions")[0]
            .archive();
        assert_eq!(
            resolve_reveal(&workspace, Path::new(REPO), id(1)),
            Reveal::Unavailable,
            "a closed session has no row to select"
        );
    }

    #[test]
    fn u33_a_session_held_by_another_project_resolves_to_unavailable() {
        let mut workspace = workspace_with(Availability::Available, id(1));
        workspace.projects.push(Project::new(
            PathBuf::from("/other"),
            true,
            Availability::Available,
        ));
        assert_eq!(
            resolve_reveal(&workspace, Path::new("/other"), id(1)),
            Reveal::Unavailable,
            "the session is not one of the project the click names"
        );
    }

    #[test]
    fn u34_a_forgotten_project_resolves_to_unavailable() {
        let mut workspace = workspace_with(Availability::Available, id(1));
        // Forgotten, with its sessions still listed: the project list decides.
        workspace.projects.clear();
        assert_eq!(
            resolve_reveal(&workspace, Path::new(REPO), id(1)),
            Reveal::Unavailable
        );
    }

    #[test]
    fn u35_a_project_whose_folder_is_unavailable_resolves_to_unavailable() {
        let workspace = workspace_with(Availability::Unavailable, id(1));
        assert_eq!(
            resolve_reveal(&workspace, Path::new(REPO), id(1)),
            Reveal::Unavailable
        );
    }
}

#[cfg(test)]
mod counted_session_tests {
    use super::*;
    use crate::session::{AiCli, SessionLocation};

    fn unread_session() -> Session {
        let mut session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        session.unread = true;
        session
    }

    #[test]
    fn an_unread_session_out_of_view_counts() {
        assert!(
            counts_as_unread(&unread_session(), None),
            "an unread, not closed session no window has in view counts (575 FR-002)"
        );
    }

    #[test]
    fn the_session_in_view_does_not_count() {
        let session = unread_session();
        assert!(
            !counts_as_unread(&session, Some(session.id)),
            "the session the window has in view is not counted (039 FR-019)"
        );
    }

    #[test]
    fn a_read_session_does_not_count() {
        let mut session = unread_session();
        session.unread = false;
        assert!(
            !counts_as_unread(&session, None),
            "a session that is not unread is not counted (575 FR-002)"
        );
    }

    #[test]
    fn a_closed_unread_session_does_not_count() {
        let mut session = unread_session();
        session.archived = true;
        assert!(
            !counts_as_unread(&session, None),
            "a closed session has no row and adds to no mark (575 FR-002, US1 scenario 5)"
        );
    }

    #[test]
    fn an_unread_session_counts_while_another_is_in_view() {
        let session = unread_session();
        assert!(
            counts_as_unread(&session, Some(SessionId::new())),
            "another session in view leaves this one counted (575 FR-002)"
        );
    }
}

#[cfg(test)]
mod notification_kind_tests {
    //! Feature 613: the kinds and their switches (FR-009, FR-010, data-model).

    use super::*;

    #[test]
    fn the_kinds_are_listed_in_settings_order() {
        assert_eq!(
            NotificationKind::ALL,
            [
                NotificationKind::NeedsPermission,
                NotificationKind::SessionError,
                NotificationKind::LongTaskFinished,
                NotificationKind::TurnFinished,
            ],
            "Settings lists the kinds in this order (FR-009)"
        );
    }

    #[test]
    fn each_kind_has_its_name() {
        let names: Vec<&str> = NotificationKind::ALL.iter().map(|k| k.name()).collect();
        assert_eq!(
            names,
            [
                "Needs permission",
                "Session error",
                "Long task finished",
                "Turn finished"
            ],
            "the names are the data-model's"
        );
    }

    #[test]
    fn each_kind_has_its_description() {
        assert_eq!(
            NotificationKind::NeedsPermission.description(),
            "A session stopped to ask for a permission or an answer."
        );
        assert_eq!(
            NotificationKind::SessionError.description(),
            "A session stopped because of an error."
        );
        assert_eq!(
            NotificationKind::TurnFinished.description(),
            "A session finished a shorter turn."
        );
    }

    #[test]
    fn the_long_task_description_names_the_threshold_not_a_duration() {
        let note = NotificationKind::LongTaskFinished.description();
        assert_eq!(
            note, "A session finished a turn at least as long as the long-task threshold.",
            "the threshold is a setting (D4 = B), so the note names it rather than a duration"
        );
        assert!(
            !note.chars().any(|c| c.is_ascii_digit()) && !note.contains("minute"),
            "no duration in the note: {note}"
        );
    }

    #[test]
    fn only_turn_finished_is_off_by_default() {
        let defaults: Vec<bool> = NotificationKind::ALL
            .iter()
            .map(|k| k.default_on())
            .collect();
        assert_eq!(
            defaults,
            [true, true, true, false],
            "out of the box everything but a short turn notifies (FR-010)"
        );
    }

    #[test]
    fn the_kinds_are_snake_case_on_the_wire() {
        let encoded: Vec<String> = NotificationKind::ALL
            .iter()
            .map(|k| serde_json::to_string(k).unwrap())
            .collect();
        assert_eq!(
            encoded,
            [
                "\"needs_permission\"",
                "\"session_error\"",
                "\"long_task_finished\"",
                "\"turn_finished\""
            ],
            "the kinds travel as snake-case strings (W5)"
        );
    }

    #[test]
    fn the_default_switches_are_the_kinds_defaults() {
        let kinds = NotificationKinds::default();
        for kind in NotificationKind::ALL {
            assert_eq!(
                kinds.is_on(kind),
                kind.default_on(),
                "{kind:?} starts at its default (FR-010)"
            );
        }
    }

    #[test]
    fn set_changes_only_the_named_kind() {
        for kind in NotificationKind::ALL {
            let mut kinds = NotificationKinds::default();
            kinds.set(kind, !kind.default_on());
            for other in NotificationKind::ALL {
                let expected = if other == kind {
                    !other.default_on()
                } else {
                    other.default_on()
                };
                assert_eq!(
                    kinds.is_on(other),
                    expected,
                    "setting {kind:?} leaves {other:?} as it was"
                );
            }
        }
    }

    #[test]
    fn is_on_reads_each_kinds_own_field() {
        let kinds = NotificationKinds {
            needs_permission: false,
            session_error: true,
            long_task_finished: false,
            turn_finished: true,
        };
        let on: Vec<bool> = NotificationKind::ALL
            .iter()
            .map(|k| kinds.is_on(*k))
            .collect();
        assert_eq!(
            on,
            [false, true, false, true],
            "each kind reads its own field"
        );
    }

    #[test]
    fn the_threshold_is_one_minute() {
        assert_eq!(LONG_TASK_THRESHOLD, Duration::from_secs(60), "D4");
    }
}

#[cfg(test)]
mod turn_clock_tests {
    //! Feature 613: the turn clock's transition table (data-model "TurnClock", C4, C5).

    use super::*;

    const THRESHOLD: Duration = Duration::from_secs(60);

    fn at(secs: u64) -> Uptime {
        Uptime::from_nanos(secs * 1_000_000_000)
    }

    fn not_in_turn() -> TurnClock {
        TurnClock::new()
    }

    fn working_since(secs: u64) -> TurnClock {
        let mut clock = TurnClock::new();
        clock.change(TurnChange::PromptSubmitted, at(secs), THRESHOLD);
        clock
    }

    fn paused_since(secs: u64) -> TurnClock {
        let mut clock = working_since(secs);
        clock.change(TurnChange::AskedUser, at(secs + 1), THRESHOLD);
        clock
    }

    /// The kind a `Finished` at `secs` returns: shows when the clock thinks the turn started.
    fn finish_at(mut clock: TurnClock, secs: u64) -> Option<NotificationKind> {
        clock.change(TurnChange::Finished, at(secs), THRESHOLD)
    }

    #[test]
    fn not_in_turn_prompt_starts_the_turn_at_now() {
        let mut clock = not_in_turn();
        assert_eq!(
            clock.change(TurnChange::PromptSubmitted, at(10), THRESHOLD),
            None
        );
        assert_eq!(
            finish_at(clock, 70),
            Some(NotificationKind::LongTaskFinished),
            "the turn counts from the prompt"
        );
    }

    #[test]
    fn not_in_turn_working_starts_the_turn_at_now() {
        let mut clock = not_in_turn();
        assert_eq!(clock.change(TurnChange::Working, at(10), THRESHOLD), None);
        assert_eq!(
            finish_at(clock, 69),
            Some(NotificationKind::TurnFinished),
            "an unseen start counts from the first work seen (C5)"
        );
        let mut clock = not_in_turn();
        clock.change(TurnChange::Working, at(10), THRESHOLD);
        assert_eq!(
            finish_at(clock, 70),
            Some(NotificationKind::LongTaskFinished)
        );
    }

    #[test]
    fn not_in_turn_asked_user_is_turn_finished_and_stays_out_of_turn() {
        let mut clock = not_in_turn();
        assert_eq!(
            clock.change(TurnChange::AskedUser, at(10), THRESHOLD),
            Some(NotificationKind::TurnFinished)
        );
        assert_eq!(clock, not_in_turn(), "no turn starts on a question");
    }

    #[test]
    fn not_in_turn_finished_is_turn_finished() {
        let mut clock = not_in_turn();
        assert_eq!(
            clock.change(TurnChange::Finished, at(1_000), THRESHOLD),
            Some(NotificationKind::TurnFinished),
            "a turn whose start was not seen is not long (C5)"
        );
        assert_eq!(clock, not_in_turn());
    }

    #[test]
    fn working_prompt_restarts_the_turn_at_now() {
        let mut clock = working_since(0);
        assert_eq!(
            clock.change(TurnChange::PromptSubmitted, at(50), THRESHOLD),
            None
        );
        assert_eq!(
            finish_at(clock, 100),
            Some(NotificationKind::TurnFinished),
            "the new turn counts from the second prompt"
        );
    }

    #[test]
    fn working_working_keeps_the_start() {
        let mut clock = working_since(0);
        assert_eq!(clock.change(TurnChange::Working, at(50), THRESHOLD), None);
        assert_eq!(
            finish_at(clock, 60),
            Some(NotificationKind::LongTaskFinished)
        );
    }

    #[test]
    fn working_asked_user_is_needs_permission_and_pauses() {
        let mut clock = working_since(0);
        assert_eq!(
            clock.change(TurnChange::AskedUser, at(5), THRESHOLD),
            Some(NotificationKind::NeedsPermission)
        );
        assert_eq!(clock, paused_since(0), "the turn pauses, keeping its start");
    }

    #[test]
    fn working_finished_ends_the_turn() {
        let mut clock = working_since(0);
        assert_eq!(
            clock.change(TurnChange::Finished, at(5), THRESHOLD),
            Some(NotificationKind::TurnFinished)
        );
        assert_eq!(clock, not_in_turn());
    }

    #[test]
    fn a_finish_at_exactly_the_threshold_is_long() {
        let start = Uptime::from_nanos(1_000_000);
        let mut clock = TurnClock::new();
        clock.change(TurnChange::PromptSubmitted, start, THRESHOLD);
        let exactly = Uptime::from_nanos(1_000_000 + THRESHOLD.as_nanos() as u64);
        assert_eq!(
            clock.change(TurnChange::Finished, exactly, THRESHOLD),
            Some(NotificationKind::LongTaskFinished),
            "at the threshold is long (C4)"
        );
    }

    #[test]
    fn a_finish_one_millisecond_short_of_the_threshold_is_not_long() {
        let start = Uptime::from_nanos(1_000_000);
        let mut clock = TurnClock::new();
        clock.change(TurnChange::PromptSubmitted, start, THRESHOLD);
        let short = Uptime::from_nanos(THRESHOLD.as_nanos() as u64);
        assert_eq!(
            clock.change(TurnChange::Finished, short, THRESHOLD),
            Some(NotificationKind::TurnFinished),
            "below the threshold is not long (C4)"
        );
    }

    /// SC-008: for T of 10 s, 60 s and 3600 s, a finish at `since + T` is long and one at
    /// `since + T - 1 ms` is not.
    #[test]
    fn the_threshold_passed_decides_long_from_short_for_each_allowed_value() {
        for secs in [10, 60, 3600] {
            let threshold = Duration::from_secs(secs);
            let start = Uptime::from_nanos(1_000_000);
            let finish = |elapsed: Duration| {
                let mut clock = TurnClock::new();
                clock.change(TurnChange::PromptSubmitted, start, threshold);
                let now = Uptime::from_nanos(1_000_000 + elapsed.as_nanos() as u64);
                clock.change(TurnChange::Finished, now, threshold)
            };
            assert_eq!(
                finish(threshold),
                Some(NotificationKind::LongTaskFinished),
                "a turn of exactly {secs} s under a {secs} s threshold is long (SC-008)"
            );
            assert_eq!(
                finish(threshold - Duration::from_millis(1)),
                Some(NotificationKind::TurnFinished),
                "a turn 1 ms short of {secs} s is not long (SC-008)"
            );
        }
    }

    /// FR-013: a turn begun under one threshold is classified by the one in force at its finish.
    #[test]
    fn a_turn_is_classified_by_the_threshold_passed_at_its_finish() {
        let begun_under = Duration::from_secs(60);
        let in_force_at_finish = Duration::from_secs(20);
        let mut clock = TurnClock::new();
        clock.change(TurnChange::PromptSubmitted, at(0), begun_under);
        assert_eq!(
            clock.change(TurnChange::Finished, at(30), in_force_at_finish),
            Some(NotificationKind::LongTaskFinished),
            "30 s is long under the 20 s threshold in force at the finish (FR-013)"
        );

        let mut clock = TurnClock::new();
        clock.change(TurnChange::PromptSubmitted, at(0), in_force_at_finish);
        assert_eq!(
            clock.change(TurnChange::Finished, at(30), begun_under),
            Some(NotificationKind::TurnFinished),
            "30 s is short under the 60 s threshold in force at the finish (FR-013)"
        );
    }

    #[test]
    fn paused_prompt_restarts_the_turn_at_now() {
        let mut clock = paused_since(0);
        assert_eq!(
            clock.change(TurnChange::PromptSubmitted, at(50), THRESHOLD),
            None
        );
        assert_eq!(finish_at(clock, 100), Some(NotificationKind::TurnFinished));
    }

    #[test]
    fn paused_working_resumes_keeping_the_start_so_the_wait_counts() {
        let mut clock = paused_since(0);
        assert_eq!(clock.change(TurnChange::Working, at(55), THRESHOLD), None);
        assert_eq!(clock, working_since(0), "the turn resumes from its start");
        assert_eq!(
            finish_at(clock, 61),
            Some(NotificationKind::LongTaskFinished),
            "the wait for the answer counts (FR-003, US1.7)"
        );
    }

    #[test]
    fn paused_asked_user_returns_nothing() {
        let mut clock = paused_since(0);
        assert_eq!(clock.change(TurnChange::AskedUser, at(5), THRESHOLD), None);
        assert_eq!(clock, paused_since(0), "still paused from the same start");
    }

    #[test]
    fn paused_finished_ends_the_turn_by_its_whole_length() {
        let mut clock = paused_since(0);
        assert_eq!(
            clock.change(TurnChange::Finished, at(60), THRESHOLD),
            Some(NotificationKind::LongTaskFinished)
        );
        assert_eq!(clock, not_in_turn());
        assert_eq!(
            finish_at(paused_since(0), 30),
            Some(NotificationKind::TurnFinished)
        );
    }
}
