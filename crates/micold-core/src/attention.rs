//! Which session a window has in view (feature 039, FR-002, FR-016, spec Terms).
//!
//! Render-free. The client fills [`ViewFacts`] from its own state and reports [`in_view`]'s answer
//! to the session service, which counts an attention event only for a session that no window has
//! in view.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::project::Availability;
use crate::protocol::messages::{ActivitySignal, SessionSummary};
use crate::session::SessionId;
use crate::workspace::Workspace;

/// What decides whether a window has a session in view.
///
/// There is no field for the tab the window shows: a regular terminal tab of the selected session
/// leaves that session in view (US1 scenario 10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewFacts {
    /// The window has keyboard focus.
    pub window_focused: bool,
    /// A screen fills the main area instead of the session (Settings).
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

/// The text of a desktop notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotificationText {
    /// The title.
    pub title: String,
    /// The body.
    pub body: String,
}

/// The text for a session waiting for input: the three names and the fixed words, nothing else.
pub fn notification_text(project: &str, worktree: &str, session: &str) -> NotificationText {
    NotificationText {
        title: format!("{session} is waiting for input"),
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

    // --- notification_text (U28–U31) ---

    #[test]
    fn u28_the_title_names_the_session_and_the_body_the_project_and_worktree() {
        let text = notification_text("micold", "feat-x", "Fix the parser");
        assert_eq!(text.title, "Fix the parser is waiting for input");
        assert_eq!(text.body, "micold \u{2014} feat-x");
    }

    #[test]
    fn u29_the_default_entry_name_is_passed_through_unchanged() {
        let text = notification_text("micold", "Default", "Fix the parser");
        assert_eq!(text.body, "micold \u{2014} Default");
    }

    #[test]
    fn u30_a_placeholder_session_label_is_passed_through_unchanged() {
        let text = notification_text("micold", "feat-x", "New session");
        assert_eq!(text.title, "New session is waiting for input");
    }

    #[test]
    fn u31_neither_string_holds_text_besides_the_three_names_and_the_fixed_words() {
        let text = notification_text("PROJ", "TREE", "SESS");
        assert_eq!(
            text.title.replace("SESS", ""),
            " is waiting for input",
            "the title is the session name and fixed words only"
        );
        assert_eq!(
            text.body.replace("PROJ", "").replace("TREE", ""),
            " \u{2014} ",
            "the body is the project, the worktree and a dash only"
        );
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
