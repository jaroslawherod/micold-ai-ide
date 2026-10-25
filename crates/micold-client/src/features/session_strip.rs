//! The session strip's rules, render-free (feature 1126, research R7): which tab is marked, whether
//! a member's process is stopped, the shown session's status words and restart, and the strip's
//! overflow and reveal arithmetic. Moved from `ui/terminal.rs`, which re-exports them; the view
//! only lays these answers out. Like every feature module it names no rendering framework
//! (`tests/features_are_render_free.rs`).

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::ui::material::{tab, TAB_WIDTH};
use micold_core::review_tabs::ReviewTabFace;
use micold_core::session::{
    SessionId, SessionLifecycle, SessionLocation, ShellInstanceId, ShellLifecycle, TerminalMode,
};
use micold_core::tokens::spacing;
use std::path::PathBuf;

pub(crate) fn session_title(state: &State, id: SessionId) -> String {
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| s.label.display().to_string())
        .unwrap_or_else(|| "Session".to_string())
}

/// Status text for the process currently attached to the pane (feature 010) — the AI CLI's
/// `SessionLifecycle` in `AiCli` mode, the shell's `ShellLifecycle` in `Regular` mode
/// (contracts/terminal-mode-lifecycle.md: `TerminalMode` never determines *running*, only
/// *displayed* — so the status shown must follow the same split).
///
/// Owned rather than `&'static str` because one state is no longer a word: a give-up names the
/// number of attempts it spent (`010` BUG-017, FR-005/SC-012). Every other arm is still a literal
/// and allocates a small one per render, which is what [`session_title`] beside it already does.
pub(crate) fn session_status(state: &State, id: SessionId) -> String {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return String::new();
    };
    match session.mode {
        TerminalMode::AiCli => match &session.lifecycle {
            SessionLifecycle::Running => "running".to_string(),
            SessionLifecycle::Starting => "starting…".to_string(),
            SessionLifecycle::Restarting { .. } => "restarting…".to_string(),
            // The count, not the sentence (`010` BUG-017 step 3). The sentence goes to the pane,
            // which has room for it; this row is content-sized labels sharing its width with the
            // tab strip, and the strip is what gives that width up (FR-002c).
            //
            // `0` names no count because there is none to name: the wire's `Failed` also covers a
            // spawn that never reached a first process, which spends no budget at all.
            SessionLifecycle::Failed { attempts, .. } => match attempts {
                0 => "failed".to_string(),
                1 => "failed after 1 attempt".to_string(),
                n => format!("failed after {n} attempts"),
            },
            SessionLifecycle::Idle => "idle".to_string(),
            SessionLifecycle::InterruptedResumable => "interrupted".to_string(),
        },
        TerminalMode::Regular => match session.active_shell_lifecycle() {
            Some(ShellLifecycle::Running) => "running".to_string(),
            Some(ShellLifecycle::Starting) => "starting…".to_string(),
            Some(ShellLifecycle::Exited) => "exited".to_string(),
            Some(ShellLifecycle::NotStarted) | None => "idle".to_string(),
        },
    }
}

/// What the bottom-bar restart control must ask for (`012` FR-010, BUG-004).
///
/// The same question [`attached_process_restartable`] asks to decide *whether* to offer a restart —
/// which process is attached — asked again to decide *what* the restart acts on. It used to press
/// `TerminalRestartRequested` unconditionally, which restarts the **session**; in Regular mode the
/// session's AI CLI primary is still alive, so `start_session` took its already-live early return
/// and the control did nothing. With a single instance there is no tab strip either, so FR-010's
/// affordance had no reachable route at all.
///
/// A session with no instance yet keeps the session-level request: there is nothing instance-shaped
/// to name, and that path lazily opens the first one.
pub(crate) fn restart_message(state: &State, id: SessionId) -> Message {
    let instance = state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .filter(|s| s.mode == TerminalMode::Regular)
        .and_then(|s| s.active_shell);
    match instance {
        Some(instance) => Message::Session(SessionMsg::ShellInstanceRestartRequested(id, instance)),
        None => Message::Session(SessionMsg::TerminalRestartRequested),
    }
}

/// Which member of the strip something refers to (feature 026, `data-model.md`).
///
/// **A closed two-variant enum, not an `Option<ShellInstanceId>`.** Principle V asks that invalid
/// states be unrepresentable, and this is where FR-005's "never zero, never two" is either
/// structural or a rule somebody has to keep. `None` already means something else in this file —
/// "this session has no active instance" — so overloading it to also mean "the AI tab" gives one
/// value two meanings and makes [`marked_tab`] unanswerable in the one case that matters. A closed
/// enum makes the marked tab a **total function** of `(TerminalMode, Option<ShellInstanceId>)`, so
/// exactly one tab is marked because there is nowhere else for the answer to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StripTab {
    /// One open Regular Terminal instance.
    Instance(ShellInstanceId),
    /// The session's single AI CLI process.
    Ai,
}

/// Which tab the strip marks — the one whose content the pane is displaying (FR-005).
///
/// Total by construction, which is the whole of FR-005's "never zero, never two". The `Regular`
/// session with no active instance falls to the AI tab rather than to nothing: that is what the
/// pane shows in that state, and a strip that marked no tab there would be the exact defect this
/// feature exists to remove, arriving by a different route.
///
/// It reads `mode`, and since feature 027 nothing else writes it but the tabs themselves, so
/// FR-008's "two routes to this pane must not be able to disagree" is structural rather than a
/// synchronisation effort — there is no second selection to keep in step.
pub fn marked_tab(state: &State, id: SessionId) -> StripTab {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return StripTab::Ai;
    };
    match session.mode {
        TerminalMode::AiCli => StripTab::Ai,
        TerminalMode::Regular => session
            .active_shell
            .map(StripTab::Instance)
            .unwrap_or(StripTab::Ai),
    }
}

/// Whether the process behind `tab` is **stopped** — not running, and restartable (feature 026
/// FR-012d, research R1/R2).
///
/// One predicate for both lifecycle vocabularies, and three things are derived from it: whether a
/// tab wears the stopped mark (FR-012d), whether that tab's menu carries a restart item (FR-006a),
/// and therefore whether the menu opens at all (FR-006b). FR-012d asks the mark and the menu to
/// *agree*, and deriving both from one function is what makes that true by construction rather than
/// by two matches happening to say the same thing.
///
/// This file has already paid twice for the alternative. `empty_terminal_message`'s own comment
/// records that the pane and the bar disagreed "for exactly as long as they were two readings of
/// one fact"; BUG-004 was `restart_message` re-deriving something the predicate beside it already
/// had. [`attached_process_restartable`] is a call into this one for the same reason.
///
/// **The rule is translated, not the names.** FR-012d names `NotStarted` and `Exited`, which are a
/// shell's vocabulary; the AI process has a larger lifecycle of its own. The rule underneath is
/// FR-012d's: the mark appears exactly where a restart can act. `Starting` and `Restarting` are
/// excluded in both rows by FR-012e — they are in progress, and a mark on a state nobody can act on
/// sends a user to a press that does nothing (FR-006b), which is the dead end the mark exists to
/// prevent.
pub fn process_stopped(state: &State, id: SessionId, tab: StripTab) -> bool {
    let Some(session) = state.active_sessions().iter().find(|s| s.id == id) else {
        return false;
    };
    match tab {
        StripTab::Ai => matches!(
            session.lifecycle,
            SessionLifecycle::Idle
                | SessionLifecycle::Failed { .. }
                | SessionLifecycle::InterruptedResumable
        ),
        StripTab::Instance(instance) => matches!(
            session
                .shells
                .iter()
                .find(|s| s.id == instance)
                .map(|s| s.lifecycle),
            None | Some(ShellLifecycle::NotStarted | ShellLifecycle::Exited)
        ),
    }
}

/// Whether the bottom-bar restart control should show (FR-013): the currently-attached process
/// (per `Session.mode`) is not running (contracts/terminal-mode-lifecycle.md's predicate).
///
/// A thin call into [`process_stopped`] since feature 026 (research R2), asking it about whichever
/// tab the pane is currently showing. Keeping it as its own name rather than inlining the call: the
/// bar's question is "is *the* attached process restartable", which is a different question from
/// the strip's "is *this* tab's process restartable", and it happens to be the strip's question
/// asked about the marked tab.
pub(crate) fn attached_process_restartable(state: &State, id: SessionId) -> bool {
    process_stopped(state, id, marked_tab(state, id))
}

/// Which edges of the scrolling region have tabs beyond them (FR-002e).
///
/// Two independent facts, not one enum, because a strip scrolled to the middle has content beyond
/// **both** edges and each fade is drawn on its own side.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Beyond {
    /// Tabs lie before the viewport's leading edge.
    pub leading: bool,
    /// Tabs lie after its trailing edge.
    pub trailing: bool,
}

/// Half a pixel — the same tolerance the layout gates use, and for the same reason: a strip
/// scrolled exactly to one end must read as *at* that end rather than a hair short of it.
const EDGE_TOLERANCE: f32 = 0.5;

/// Whether anything lies beyond either edge of a viewport `viewport` wide, holding `content`, and
/// currently scrolled `offset` from the leading end (FR-002e, research R6).
///
/// Pure arithmetic, deliberately. The *fade* is appearance and no layout gate can see it — a
/// gradient occupies the same box whether it is opaque or invisible, which is the family of defect
/// the `visual-pass` skill exists for. What can be gated is the fact behind it, and this is it.
pub(crate) fn overflowing(offset: f32, viewport: f32, content: f32) -> Beyond {
    Beyond {
        leading: offset > EDGE_TOLERANCE,
        trailing: offset + viewport + EDGE_TOLERANCE < content,
    }
}

/// One member of the tab strip above the terminal, in order (feature 1126, data-model
/// *StripModel*, contract S1): a session tab per session of the shown location, and right after
/// the shown session's own tab its shell tabs and the "+"; last, the location's Review tab when it
/// has one (feature 1269, U13).
#[derive(Debug, Clone, PartialEq)]
pub enum Member {
    /// A session's tab: `<command> · <session name>`; the shown session's tab is its AI
    /// conversation.
    Session {
        id: SessionId,
        label: String,
        tooltip: String,
        shown: bool,
        marked: bool,
        stopped: bool,
    },
    /// One of the shown session's shell instances, labelled by its ordinal.
    Shell {
        session: SessionId,
        instance: ShellInstanceId,
        label: String,
        marked: bool,
        stopped: bool,
    },
    /// Open another shell instance of the shown session.
    Plus,
    /// The shown location's Review tab (feature 1269, U13, R10): `Review`, its draft count and
    /// changed mark, and its ×.
    Review {
        project: PathBuf,
        entry: SessionLocation,
        face: ReviewTabFace,
        /// The review is what the main area shows; then it is the marked member.
        shown: bool,
    },
}

/// The key a member's widget state is kept under in the strip's keyed row (R10): a press in flight
/// follows its member when another is added before it (FR-011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemberKey {
    Session(SessionId),
    Shell(SessionId, ShellInstanceId),
    Plus,
    /// The Review tab: a strip shows one location, so it holds at most one (feature 1269, U14).
    Review,
}

/// The "+" member's width: its icon button's touch target (R8).
pub const PLUS_WIDTH: f32 = micold_core::tokens::anatomy::button::MIN_TOUCH_TARGET;

impl Member {
    /// The key its widget state is kept under (R10).
    pub fn key(&self) -> MemberKey {
        match self {
            Self::Session { id, .. } => MemberKey::Session(*id),
            Self::Shell {
                session, instance, ..
            } => MemberKey::Shell(*session, *instance),
            Self::Plus => MemberKey::Plus,
            Self::Review { .. } => MemberKey::Review,
        }
    }

    /// Whether this member is the marked one (S2).
    pub fn marked(&self) -> bool {
        match self {
            Self::Session { marked, .. } | Self::Shell { marked, .. } => *marked,
            Self::Review { shown, .. } => *shown,
            Self::Plus => false,
        }
    }

    /// Whether this member's own process is stopped (S3).
    pub fn stopped(&self) -> bool {
        match self {
            Self::Session { stopped, .. } | Self::Shell { stopped, .. } => *stopped,
            Self::Plus | Self::Review { .. } => false,
        }
    }

    /// What a primary press sends (S5; feature 1269, U14: the Review tab asks for its location's
    /// review, as **Review changes** does, which shows the kept view).
    pub fn press(&self) -> Message {
        let session = |msg| Message::Session(msg);
        match self {
            Self::Session {
                id, shown: true, ..
            } => session(SessionMsg::TerminalAiCliSelected(*id)),
            // As its sidebar row does: the session is shown, its terminal takes the keyboard.
            Self::Session { id, .. } => session(SessionMsg::Selected(*id)),
            Self::Shell {
                session: id,
                instance,
                ..
            } => session(SessionMsg::ShellInstanceSelected(*id, *instance)),
            Self::Plus => session(SessionMsg::ShellInstanceOpenRequested),
            Self::Review { entry, .. } => Message::Sidebar(
                crate::features::sidebar::Msg::ReviewChangesRequested(entry.clone()),
            ),
        }
    }

    /// What its close control sends: shell tabs (S5) and the Review tab, whose × closes it
    /// (feature 1269, U4, U14).
    pub fn close(&self) -> Option<Message> {
        match self {
            Self::Shell {
                session, instance, ..
            } => Some(Message::Session(SessionMsg::ShellInstanceCloseRequested(
                *session, *instance,
            ))),
            Self::Review { project, entry, .. } => {
                Some(Message::Changes(crate::features::changes::Msg::TabClosed {
                    project: project.clone(),
                    entry: entry.clone(),
                }))
            }
            _ => None,
        }
    }

    /// The tab a right-click opens the menu for (S5): the shown session's tab and its shell tabs;
    /// another session's tab and the "+" open none, since the menu's restart acts on the shown
    /// session (`TerminalRestartRequested`).
    pub fn menu_tab(&self) -> Option<StripTab> {
        match self {
            Self::Session { shown: true, .. } => Some(StripTab::Ai),
            Self::Shell { instance, .. } => Some(StripTab::Instance(*instance)),
            _ => None,
        }
    }

    /// Its fixed width (R8): a session tab's, a shell tab's, the "+"'s, or the Review tab's
    /// (feature 1269, U15), whatever its badge or mark.
    pub fn width(&self) -> f32 {
        match self {
            Self::Session { .. } => tab::SESSION_WIDTH,
            Self::Review { .. } => tab::REVIEW_WIDTH,
            Self::Shell { .. } => TAB_WIDTH,
            Self::Plus => PLUS_WIDTH,
        }
    }
}

/// What the strip above the terminal shows (data-model *StripModel*; contract S1–S4).
#[derive(Debug, Clone, PartialEq)]
pub struct StripModel {
    /// In order: session tabs; the shown session's shell tabs and `Plus` right after its own tab.
    pub members: Vec<Member>,
    /// The shown session's status words (S4).
    pub status: String,
    /// The restart press, while the shown session's attached process is stopped (S3).
    pub restart: Option<Message>,
    /// The shown location, for **New session**; `None` when it no longer resolves.
    pub location: Option<SessionLocation>,
}

impl StripModel {
    /// The strip for the shown location (feature 1269, U13): its sessions' members as for the shown
    /// session, then its Review tab when it has one; a sessionless location's shown review alone.
    /// `None` with neither a session nor a review shown.
    pub fn of(state: &State) -> Option<Self> {
        let review = review_member(state);
        let review_shown = review.as_ref().is_some_and(Member::marked);
        // A shown review of another location than the current session's (a sessionless one)
        // draws no session member: the strip is that location's.
        let shown = state.session.active.filter(|id| {
            !review_shown
                || state
                    .active_sessions()
                    .iter()
                    .find(|s| s.id == *id)
                    .map(|s| &s.location)
                    == state.review_shown().map(|(_, entry)| entry)
        });
        let Some(shown) = shown else {
            return review.map(|review| Self {
                members: vec![review],
                status: String::new(),
                restart: None,
                location: state.shown_location(),
            });
        };
        let location = state.current_session_location();
        let node = location
            .as_ref()
            .and_then(|at| crate::features::navigation::location_node(state, at));
        let main = node.as_ref().and_then(|n| n.main_session());
        // The order the previous/next-session chords step through, folds ignored (R7), so a session
        // under a folded origin keeps its tab while it is shown.
        let order: Vec<SessionId> = node
            .as_ref()
            .map(|n| n.unfolded_sessions().iter().map(|s| s.id).collect())
            .filter(|ids: &Vec<SessionId>| ids.contains(&shown))
            .unwrap_or_else(|| vec![shown]);
        // Feature 1269 (U13): while the review is shown it is the marked member, and no session or
        // shell member is.
        let marked = (!review_shown).then(|| marked_tab(state, shown));
        let mut members = Vec::with_capacity(order.len() + 3);
        for id in order {
            let is_shown = id == shown;
            members.push(session_member(
                state,
                id,
                is_shown,
                main == Some(id),
                marked,
            ));
            if is_shown {
                let shells = state
                    .active_sessions()
                    .iter()
                    .find(|s| s.id == id)
                    .map(|s| s.shells.iter().map(|i| i.id).collect::<Vec<_>>())
                    .unwrap_or_default();
                members.extend(shells.into_iter().map(|instance| Member::Shell {
                    session: id,
                    instance,
                    label: instance.0.to_string(),
                    marked: marked == Some(StripTab::Instance(instance)),
                    stopped: process_stopped(state, id, StripTab::Instance(instance)),
                }));
                members.push(Member::Plus);
            }
        }
        members.extend(review);
        Some(Self {
            members,
            status: session_status(state, shown),
            restart: attached_process_restartable(state, shown)
                .then(|| restart_message(state, shown)),
            location,
        })
    }

    /// Whether the strip's Review tab is the shown one (feature 1269, U15): the status cluster and
    /// the split buttons then keep their place, disabled.
    pub fn review_shown(&self) -> bool {
        self.members
            .iter()
            .any(|m| matches!(m, Member::Review { shown: true, .. }))
    }

    /// Where the marked member sits in [`Self::members`].
    pub fn marked_index(&self) -> Option<usize> {
        self.members.iter().position(Member::marked)
    }
}

/// Each member's (leading, trailing) edge along the strip, from the members' fixed widths and the
/// strip's `spacing::SM` gaps (S6). Derived, never measured: a measured content width is stale
/// until the first scroll, which drew a fade over nothing (feature 027).
pub fn member_offsets(members: &[Member]) -> Vec<(f32, f32)> {
    let mut at = 0.0;
    members
        .iter()
        .map(|member| {
            let edges = (at, at + member.width());
            at = edges.1 + spacing::SM;
            edges
        })
        .collect()
}

/// What lies beyond the strip's edges, for a viewport `viewport` wide holding `members` and
/// scrolled `offset` from the leading end (FR-010; feature 027 FR-002e). A viewport of zero means
/// "not measured yet": no edge is beyond.
pub(crate) fn strip_overflow(offset: f32, viewport: f32, members: &[Member]) -> Beyond {
    if viewport <= 0.0 {
        return Beyond {
            leading: false,
            trailing: false,
        };
    }
    let content = member_offsets(members).last().map_or(0.0, |&(_, end)| end);
    overflowing(offset, viewport, content)
}

/// Where to scroll a viewport `viewport` wide, currently at `offset`, to bring member `index` fully
/// into view — or `None` if it already is (feature 026 FR-002d).
///
/// `None` rather than "scroll to where you already are" is the requirement: a user may scroll away
/// from the marked tab by hand, and a reveal that fired on every selection would yank them back.
/// It scrolls the member **just** into view rather than centring it.
///
/// A member wider than the viewport cannot be brought wholly into view, so its **leading** edge is
/// (feature 1269, visual pass): that is where a tab's mark — the Review tab's changed dot — is
/// drawn, and showing its trailing end instead left the mark under the leading fade.
pub fn scroll_into_view(
    index: usize,
    offset: f32,
    viewport: f32,
    members: &[Member],
) -> Option<f32> {
    let (leading, trailing) = *member_offsets(members).get(index)?;
    let target = if leading + EDGE_TOLERANCE < offset || trailing - leading > viewport {
        leading
    } else if trailing > offset + viewport + EDGE_TOLERANCE {
        trailing - viewport
    } else {
        return None;
    };
    ((target - offset).abs() > EDGE_TOLERANCE).then_some(target)
}

/// Where the drained tab reveal scrolls the strip, or `None` to leave it (feature 026 FR-002d,
/// feature 1269 visual pass).
///
/// `from_start` is set after a location change: the offset held is then the left strip's, so the
/// reveal measures from `0` instead and always scrolls — to the start when the marked tab fits
/// there, else to where [`scroll_into_view`] puts it. Otherwise it measures from `offset`, and a
/// tab already in view is left alone, so a strip the user scrolled by hand stays put.
pub fn tab_reveal_target(
    from_start: bool,
    offset: f32,
    viewport: f32,
    index: usize,
    members: &[Member],
) -> Option<f32> {
    if from_start {
        Some(scroll_into_view(index, 0.0, viewport, members).unwrap_or(0.0))
    } else {
        scroll_into_view(index, offset, viewport, members)
    }
}

/// A session's tab (S1, R8): `<command> · <session name>`, its role and status on hover.
fn session_member(
    state: &State,
    id: SessionId,
    shown: bool,
    main: bool,
    marked: Option<StripTab>,
) -> Member {
    let (provider, kind) = state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)
        .map(|s| (s.provider, s.kind))
        .unwrap_or_default();
    let label = format!(
        "{} · {}",
        crate::features::session::command_caption(provider, kind),
        session_title(state, id)
    );
    let role = if main {
        "main session"
    } else {
        "helper session"
    };
    let tooltip = format!("{label}\n{role} · {}", session_status(state, id));
    Member::Session {
        id,
        label,
        tooltip,
        shown,
        marked: shown && marked == Some(StripTab::Ai),
        stopped: process_stopped(state, id, StripTab::Ai),
    }
}

/// The shown location's Review tab, if it has one (feature 1269, U13): its face from the draft
/// count of the location's comments (`None` while unknown) and its freshness; shown while it is
/// the shown review.
fn review_member(state: &State) -> Option<Member> {
    // No active project reads as the empty path, as `review_shown` and `ChangesRequested` do.
    let project = state.workspace.active.clone().unwrap_or_default();
    let entry = state.shown_location()?;
    let key = (project, entry);
    let tab = state.changes.tabs.get(&key)?;
    let drafts = state
        .changes
        .reviews
        .get(&(key.0.clone(), crate::features::changes::entry_dir(&key.1)))
        .map(|review| {
            review
                .comments
                .iter()
                .filter(|c| crate::features::changes::is_pending(c))
                .count()
        });
    let shown = state.review_shown() == Some(&key);
    let (project, entry) = key;
    Some(Member::Review {
        project,
        entry,
        face: ReviewTabFace::of(drafts, tab.freshness(), shown),
        shown,
    })
}

/// The marked member's position in the shown session's strip, or `None` without a session.
pub fn marked_tab_index(state: &State) -> Option<usize> {
    StripModel::of(state)?.marked_index()
}

#[cfg(test)]
pub(crate) mod tests {
    //! Moved with the rules from `ui/terminal.rs` (feature 1126, T003), unchanged. The fixtures are
    //! shared with that file's own tests.
    use super::*;
    use micold_core::project::{Availability, Project};
    use micold_core::session::{Session, SessionLocation};

    /// A `Failed` in the shape supervision produces one. These cases are about the *variant* — the
    /// tint, the status word, whether restart is offered — not about the sentence, but the variant
    /// has carried one since `010` BUG-017 and something has to fill it.
    pub(crate) fn failed() -> SessionLifecycle {
        SessionLifecycle::Failed {
            reason: "Gave up after 3 restart attempts — last exit: exit status 1.".into(),
            attempts: 3,
        }
    }

    /// A state with one project and one session at `lifecycle`, made current.
    pub(crate) fn state_showing(lifecycle: SessionLifecycle) -> (State, SessionId) {
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state.workspace.projects.push(Project {
            path: path.clone(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        state.workspace.active = Some(path.clone());
        let mut session = Session::start_new(
            SessionLocation::Default,
            micold_core::session::AiCli::ClaudeCode,
        );
        session.lifecycle = lifecycle;
        let id = session.id;
        state.workspace.sessions.insert(path, vec![session]);
        state.session.active = Some(id);
        (state, id)
    }

    /// A state with one project and one session in `mode`, holding `shells` instances at the given
    /// lifecycles and marking `active` (an index into `shells`) as the displayed one.
    pub(crate) fn state_with_instances(
        mode: TerminalMode,
        lifecycle: SessionLifecycle,
        shells: &[ShellLifecycle],
        active: Option<usize>,
    ) -> (State, SessionId) {
        let mut state = State::default();
        let path = std::path::PathBuf::from("/repo");
        state.workspace.projects.push(Project {
            path: path.clone(),
            display_name: "repo".to_string(),
            is_git_repo: true,
            availability: Availability::Available,
        });
        state.workspace.active = Some(path.clone());
        let mut session = Session::start_new(
            SessionLocation::Default,
            micold_core::session::AiCli::ClaudeCode,
        );
        session.lifecycle = lifecycle;
        session.mode = mode;
        let mut opened = Vec::new();
        for want in shells {
            let id = session.open_shell_instance();
            // Written rather than driven through the transitions: `open_shell_instance` leaves an
            // instance `Starting`, so `NotStarted` — a state a *restored* session's instances are
            // in, and one of the two FR-012d names — is not reachable by any public transition.
            // A table-driven test has to be able to state its inputs.
            if let Some(instance) = session.shells.iter_mut().find(|s| s.id == id) {
                instance.lifecycle = *want;
            }
            opened.push(id);
        }
        session.active_shell = active.map(|i| opened[i]);
        let id = session.id;
        state.workspace.sessions.insert(path, vec![session]);
        state.session.active = Some(id);
        (state, id)
    }

    /// FR-002e, research R6: "content lies beyond this edge" is a pure function of the viewport
    /// offset and the content width.
    ///
    /// The *fade* is appearance and no gate can see it — a gradient occupies the same box whether
    /// it is opaque or invisible, which is what the visual pass is for. The **fact behind it** is
    /// arithmetic, and this is where it is held. Four cases, because a strip can be scrolled to
    /// either end, to the middle, or not be scrollable at all, and the last one is the case a naive
    /// `offset > 0` would get wrong in the direction that matters: a fade on a strip with nothing
    /// beyond it tells the user to scroll toward nothing.
    #[test]
    fn an_edge_knows_whether_anything_lies_beyond_it() {
        // Written as literals rather than as named constants on `Beyond` itself. The four names
        // would have to be `#[cfg(test)]` — nothing in the drawing path wants a whole value, since
        // each edge is drawn on its own side and asking "is this exactly BOTH" would couple the two
        // together — and a second `#[cfg(test)]` block in this file breaks
        // `tests/anatomy_call_sites.rs`, which truncates a source at the first one.
        let beyond = |leading, trailing| Beyond { leading, trailing };

        // Nothing to scroll: the content fits, so neither edge says anything.
        assert_eq!(overflowing(0.0, 500.0, 300.0), beyond(false, false));
        // Scrolled hard to the leading end, with more to come.
        assert_eq!(overflowing(0.0, 300.0, 900.0), beyond(false, true));
        // Somewhere in the middle: both.
        assert_eq!(overflowing(300.0, 300.0, 900.0), beyond(true, true));
        // Scrolled hard to the trailing end.
        assert_eq!(overflowing(600.0, 300.0, 900.0), beyond(true, false));
        // Exactly at the trailing end, to the pixel — not "a little more that way".
        assert!(
            !overflowing(600.0, 300.0, 900.0).trailing,
            "an edge with nothing past it must not be faded; a cue that points at nothing is \
             worse than none, because it is the same cue that means something elsewhere"
        );
    }

    /// A location with main `M` (two shell instances) and helpers `H1`, `H2` at the project root,
    /// session `shown` (0 = `M`) shown, every session at `Running` (contract S1's fixture).
    pub(crate) fn strip_fixture(shown: usize) -> (State, [SessionId; 3]) {
        let (mut state, main) = state_with_instances(
            TerminalMode::AiCli,
            SessionLifecycle::Running,
            &[ShellLifecycle::Running, ShellLifecycle::Running],
            None,
        );
        let path = std::path::PathBuf::from("/repo");
        let sessions = state.workspace.sessions.get_mut(&path).expect("fixture");
        sessions[0].main = true;
        let mut ids = [main; 3];
        for id in ids.iter_mut().skip(1) {
            let helper = Session::start_new(
                SessionLocation::Default,
                micold_core::session::AiCli::ClaudeCode,
            );
            *id = helper.id;
            sessions.push(helper);
        }
        for s in sessions.iter_mut() {
            s.lifecycle = SessionLifecycle::Running;
        }
        state.session.active = Some(ids[shown]);
        (state, ids)
    }

    fn keys(model: &StripModel) -> Vec<MemberKey> {
        model.members.iter().map(Member::key).collect()
    }

    fn shells_of(state: &State, id: SessionId) -> Vec<ShellInstanceId> {
        state
            .active_sessions()
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.shells.iter().map(|i| i.id).collect())
            .unwrap_or_default()
    }

    /// S1: session tabs in the location's order, the shown session's shells and "+" right after its
    /// own tab, and no other session's shells.
    #[test]
    fn the_strip_holds_each_session_then_the_shown_ones_shells_and_plus() {
        let (state, [m, h1, h2]) = strip_fixture(0);
        let shells = shells_of(&state, m);
        let model = StripModel::of(&state).expect("a session is shown");
        assert_eq!(
            keys(&model),
            vec![
                MemberKey::Session(m),
                MemberKey::Shell(m, shells[0]),
                MemberKey::Shell(m, shells[1]),
                MemberKey::Plus,
                MemberKey::Session(h1),
                MemberKey::Session(h2),
            ]
        );

        let (state, [m, h1, h2]) = strip_fixture(1);
        assert_eq!(
            keys(&StripModel::of(&state).expect("shown")),
            vec![
                MemberKey::Session(m),
                MemberKey::Session(h1),
                MemberKey::Plus,
                MemberKey::Session(h2),
            ],
            "another session's shell tabs are not members while it is not shown (US2 sc. 7)"
        );

        let (state, id) = state_showing(SessionLifecycle::Running);
        let model = StripModel::of(&state).expect("shown");
        assert_eq!(keys(&model), vec![MemberKey::Session(id), MemberKey::Plus]);
        assert!(matches!(
            model.members[0],
            Member::Session {
                shown: true,
                marked: true,
                ..
            }
        ));

        assert_eq!(StripModel::of(&State::default()), None, "no session shown");
    }

    /// S1: a session an agent started follows the session that started it, folded or not.
    #[test]
    fn an_agent_started_session_follows_its_origin_folded_or_not() {
        let (mut state, [m, h1, h2]) = strip_fixture(0);
        let mut child = Session::start_new(
            SessionLocation::Default,
            micold_core::session::AiCli::ClaudeCode,
        );
        child.origin = Some(micold_core::origin::SessionOrigin {
            id: h1,
            status: micold_core::origin::OriginStatus::Known,
        });
        let c = child.id;
        state
            .workspace
            .sessions
            .get_mut(&std::path::PathBuf::from("/repo"))
            .expect("fixture")
            .push(child);
        let sessions = |state: &State| -> Vec<MemberKey> {
            keys(&StripModel::of(state).expect("shown"))
                .into_iter()
                .filter(|k| matches!(k, MemberKey::Session(_)))
                .collect()
        };
        let want = vec![
            MemberKey::Session(m),
            MemberKey::Session(h1),
            MemberKey::Session(c),
            MemberKey::Session(h2),
        ];
        assert_eq!(sessions(&state), want);
        state.sidebar.folded.insert(h1);
        assert_eq!(sessions(&state), want, "a fold hides no tab");
    }

    /// S2: exactly one member is marked, and never one of another session.
    #[test]
    fn exactly_one_member_is_marked_and_only_of_the_shown_session() {
        let marked = |state: &State| -> Vec<MemberKey> {
            StripModel::of(state)
                .expect("shown")
                .members
                .iter()
                .filter(|m| m.marked())
                .map(Member::key)
                .collect()
        };
        let (mut state, [m, _, _]) = strip_fixture(0);
        assert_eq!(marked(&state), vec![MemberKey::Session(m)], "AI mode");
        let shells = shells_of(&state, m);
        {
            let (_, s) = state.workspace.find_session_mut(m).expect("session");
            s.mode = TerminalMode::Regular;
            s.active_shell = Some(shells[1]);
        }
        assert_eq!(marked(&state), vec![MemberKey::Shell(m, shells[1])]);
        {
            let (_, s) = state.workspace.find_session_mut(m).expect("session");
            s.active_shell = None;
        }
        assert_eq!(
            marked(&state),
            vec![MemberKey::Session(m)],
            "Regular with no active instance falls to the session tab"
        );
        let (state, [_, h, _]) = strip_fixture(1);
        assert_eq!(marked(&state), vec![MemberKey::Session(h)]);
    }

    /// S3: each member's stopped mark is its own process's, never across sessions; `restart` is
    /// offered exactly when the shown marked tab is stopped, aimed at that process.
    #[test]
    fn a_stopped_mark_is_each_members_own_and_restart_follows_the_shown_one() {
        let stopped = |state: &State| -> Vec<(MemberKey, bool)> {
            StripModel::of(state)
                .expect("shown")
                .members
                .iter()
                .map(|m| (m.key(), m.stopped()))
                .collect()
        };
        let (mut state, [m, h1, h2]) = strip_fixture(0);
        {
            let (_, s) = state.workspace.find_session_mut(h1).expect("session");
            s.lifecycle = SessionLifecycle::Idle;
        }
        let shells = shells_of(&state, m);
        {
            let (_, s) = state.workspace.find_session_mut(m).expect("session");
            s.shells[1].lifecycle = ShellLifecycle::Exited;
        }
        assert_eq!(
            stopped(&state),
            vec![
                (MemberKey::Session(m), false),
                (MemberKey::Shell(m, shells[0]), false),
                (MemberKey::Shell(m, shells[1]), true),
                (MemberKey::Plus, false),
                (MemberKey::Session(h1), true),
                (MemberKey::Session(h2), false),
            ],
            "a stopped helper never marks the main tab"
        );
        assert_eq!(
            StripModel::of(&state).expect("shown").restart,
            None,
            "the shown marked tab (M's AI) runs"
        );

        {
            let (_, s) = state.workspace.find_session_mut(m).expect("session");
            s.mode = TerminalMode::Regular;
            s.active_shell = Some(shells[1]);
        }
        assert_eq!(
            StripModel::of(&state).expect("shown").restart,
            Some(restart_message(&state, m)),
            "the marked exited shell offers its restart"
        );
        assert_eq!(
            restart_message(&state, m),
            Message::Session(SessionMsg::ShellInstanceRestartRequested(m, shells[1]))
        );

        let (mut state, [m, _, _]) = strip_fixture(0);
        {
            let (_, s) = state.workspace.find_session_mut(m).expect("session");
            s.lifecycle = failed();
        }
        assert_eq!(
            StripModel::of(&state).expect("shown").restart,
            Some(Message::Session(SessionMsg::TerminalRestartRequested))
        );
    }

    /// S4: the status words are the shown session's, unchanged; `location` is the shown one, `None`
    /// when it no longer resolves.
    #[test]
    fn the_status_is_the_shown_sessions_words_and_the_location_its_own() {
        for (lifecycle, words) in [
            (SessionLifecycle::Running, "running"),
            (SessionLifecycle::Starting, "starting…"),
            (SessionLifecycle::Idle, "idle"),
            (SessionLifecycle::InterruptedResumable, "interrupted"),
            (failed(), "failed after 3 attempts"),
        ] {
            let (state, _) = state_showing(lifecycle.clone());
            let model = StripModel::of(&state).expect("shown");
            assert_eq!(model.status, words, "{lifecycle:?}");
            assert_eq!(model.location, Some(SessionLocation::Default));
        }
        let (mut state, id) = state_showing(SessionLifecycle::Running);
        {
            let (_, s) = state.workspace.find_session_mut(id).expect("session");
            s.location = SessionLocation::Worktree("gone".into());
        }
        let model = StripModel::of(&state).expect("still shown");
        assert_eq!(model.location, None);
        assert_eq!(keys(&model), vec![MemberKey::Session(id), MemberKey::Plus]);
    }

    /// S1, R8: a session tab reads `<command> · <session name>`; its tooltip adds the role and the
    /// status.
    #[test]
    fn a_session_tab_names_its_cli_and_session_and_says_its_role_on_hover() {
        let (state, [m, h1, _]) = strip_fixture(0);
        let model = StripModel::of(&state).expect("shown");
        let tab = |id| {
            model.members.iter().find_map(|member| match member {
                Member::Session {
                    id: at,
                    label,
                    tooltip,
                    ..
                } if *at == id => Some((label.clone(), tooltip.clone())),
                _ => None,
            })
        };
        let (label, tooltip) = tab(m).expect("main tab");
        assert_eq!(label, format!("claude · {}", session_title(&state, m)));
        assert!(
            tooltip.contains(&label) && tooltip.contains("main"),
            "{tooltip:?}"
        );
        assert!(tooltip.contains("running"), "{tooltip:?}");
        let (_, tooltip) = tab(h1).expect("helper tab");
        assert!(tooltip.contains("helper"), "{tooltip:?}");
    }

    /// S5: each member's press, close and menu.
    #[test]
    fn each_member_presses_what_its_bar_control_pressed() {
        let (state, [m, h1, _]) = strip_fixture(0);
        let shell = shells_of(&state, m)[0];
        let model = StripModel::of(&state).expect("shown");
        let member = |key| {
            model
                .members
                .iter()
                .find(|x| x.key() == key)
                .expect("member")
                .clone()
        };
        let shown = member(MemberKey::Session(m));
        assert_eq!(
            shown.press(),
            Message::Session(SessionMsg::TerminalAiCliSelected(m))
        );
        assert_eq!(shown.close(), None);
        assert_eq!(shown.menu_tab(), Some(StripTab::Ai));

        let other = member(MemberKey::Session(h1));
        assert_eq!(other.press(), Message::Session(SessionMsg::Selected(h1)));
        assert_eq!(
            other.menu_tab(),
            None,
            "another session's tab yields no StripTabMenuRequested"
        );

        let tab = member(MemberKey::Shell(m, shell));
        assert_eq!(
            tab.press(),
            Message::Session(SessionMsg::ShellInstanceSelected(m, shell))
        );
        assert_eq!(
            tab.close(),
            Some(Message::Session(SessionMsg::ShellInstanceCloseRequested(
                m, shell
            )))
        );
        assert_eq!(tab.menu_tab(), Some(StripTab::Instance(shell)));

        let plus = member(MemberKey::Plus);
        assert_eq!(
            plus.press(),
            Message::Session(SessionMsg::ShellInstanceOpenRequested)
        );
        assert_eq!((plus.close(), plus.menu_tab()), (None, None));
    }

    /// S5, FR-009: a member's menu offers *Restart* exactly when its stopped mark shows, for every
    /// member of the S1 fixtures; another session's tab opens no menu at all.
    #[test]
    fn a_members_menu_offers_restart_exactly_when_it_wears_the_stopped_mark() {
        let (mut state, [m, h1, _]) = strip_fixture(0);
        let shells = shells_of(&state, m);
        {
            let (_, s) = state.workspace.find_session_mut(m).expect("session");
            s.shells[0].lifecycle = ShellLifecycle::Exited;
        }
        {
            let (_, s) = state.workspace.find_session_mut(h1).expect("session");
            s.lifecycle = SessionLifecycle::Idle;
        }
        for ai in [SessionLifecycle::Running, SessionLifecycle::Idle] {
            {
                let (_, s) = state.workspace.find_session_mut(m).expect("session");
                s.lifecycle = ai.clone();
            }
            let model = StripModel::of(&state).expect("shown");
            let mut menus = 0;
            for member in &model.members {
                match member.menu_tab() {
                    Some(tab) => {
                        menus += 1;
                        let labels = crate::ui::strip_tab_menu_labels(&state, m, tab);
                        assert_eq!(
                            labels.contains(&"Restart"),
                            member.stopped(),
                            "{ai:?} {:?}: menu {labels:?}",
                            member.key()
                        );
                    }
                    None => assert!(
                        matches!(member, Member::Plus | Member::Session { shown: false, .. }),
                        "{:?} opens no menu",
                        member.key()
                    ),
                }
            }
            assert_eq!(menus, 1 + shells.len(), "the shown tab and its shells");
        }
    }

    /// The shells among [`twelve`]'s members.
    const SHELLS_IN_TWELVE: f32 = 9.0;

    /// Twelve members: two sessions, the shown one's nine shells and "+".
    fn twelve() -> Vec<Member> {
        let session = |shown| Member::Session {
            id: SessionId::new(),
            label: String::new(),
            tooltip: String::new(),
            shown,
            marked: shown,
            stopped: false,
        };
        let main = session(true);
        let MemberKey::Session(id) = main.key() else {
            unreachable!()
        };
        let mut members = vec![main];
        members.extend((1..=9).map(|n| Member::Shell {
            session: id,
            instance: ShellInstanceId(n),
            label: n.to_string(),
            marked: false,
            stopped: false,
        }));
        members.push(Member::Plus);
        members.push(session(false));
        members
    }

    /// S6: offsets from the fixed widths and `SM` gaps.
    #[test]
    fn member_offsets_add_fixed_widths_and_gaps() {
        let members = &twelve()[..];
        assert_eq!(members.len(), 12);
        let offsets = member_offsets(members);
        let sw = tab::SESSION_WIDTH;
        // Accumulated float sums: equal within the edge tolerance, not bit for bit.
        let near = |got: (f32, f32), want: (f32, f32)| {
            assert!(
                (got.0 - want.0).abs() <= EDGE_TOLERANCE
                    && (got.1 - want.1).abs() <= EDGE_TOLERANCE,
                "offsets {got:?}, wanted {want:?}"
            );
        };
        near(offsets[0], (0.0, sw));
        near(offsets[1], (sw + spacing::SM, sw + spacing::SM + TAB_WIDTH));
        let plus = offsets[10];
        let lead = sw + SHELLS_IN_TWELVE * (TAB_WIDTH + spacing::SM) + spacing::SM;
        near(plus, (lead, lead + PLUS_WIDTH));
        near(
            offsets[11],
            (plus.1 + spacing::SM, plus.1 + spacing::SM + sw),
        );
        assert!(member_offsets(&[]).is_empty());
    }

    /// S6's table: which edges have members beyond them; viewport 0 means none.
    #[test]
    fn the_overflow_rows_of_the_s6_table() {
        let beyond = |leading, trailing| Beyond { leading, trailing };
        let (state, _) = state_showing(SessionLifecycle::Running);
        let one = StripModel::of(&state).expect("shown").members;
        assert_eq!(strip_overflow(0.0, 340.0, &one), beyond(false, false));
        let members = twelve();
        let content = member_offsets(&members).last().expect("members").1;
        assert_eq!(strip_overflow(0.0, 340.0, &members), beyond(false, true));
        assert_eq!(
            strip_overflow(content - 340.0, 340.0, &members),
            beyond(true, false)
        );
        assert_eq!(strip_overflow(0.0, 0.0, &members), beyond(false, false));
    }

    /// S6: the reveal brings a marked member beyond either edge just into view, and leaves one in
    /// view alone (feature 026 FR-002d).
    #[test]
    fn a_member_beyond_either_edge_scrolls_just_into_view() {
        let members = twelve();
        let offsets = member_offsets(&members);
        assert_eq!(scroll_into_view(0, 0.0, 340.0, &members), None);
        let (_, trailing) = offsets[11];
        assert_eq!(
            scroll_into_view(11, 0.0, 340.0, &members),
            Some(trailing - 340.0)
        );
        let (leading, _) = offsets[2];
        assert_eq!(
            scroll_into_view(2, leading + 50.0, 340.0, &members),
            Some(leading)
        );
        assert_eq!(
            scroll_into_view(12, 0.0, 340.0, &members),
            None,
            "no such member"
        );
        // Feature 1269 (visual pass): a member wider than the viewport shows its leading edge,
        // where a tab's mark is drawn, and one already scrolled there is left alone.
        let (leading, trailing) = offsets[11];
        let narrow = (trailing - leading) - 10.0;
        assert_eq!(scroll_into_view(11, 0.0, narrow, &members), Some(leading));
        assert_eq!(scroll_into_view(11, leading, narrow, &members), None);
        assert_eq!(scroll_into_view(0, 0.0, narrow, &members), None);
        // Deliberate: a hand-scrolled offset in the middle of an oversized marked tab snaps back to
        // its leading edge, where the mark is, rather than being left mid-tab.
        assert_eq!(
            scroll_into_view(11, leading + narrow / 2.0, narrow, &members),
            Some(leading)
        );
        // Boundary: a member exactly the viewport's width fits — scrolled to, it is left alone.
        let exact = trailing - leading;
        assert_eq!(scroll_into_view(11, leading, exact, &members), None);
        assert_eq!(
            scroll_into_view(11, 0.0, exact, &members),
            Some(trailing - exact),
            "it fits, so the trailing-edge rule applies, which is its leading edge"
        );
    }

    /// The drained reveal's target (feature 1269, visual pass): after a location change it
    /// measures from the start and always scrolls; otherwise a tab in view is left alone.
    #[test]
    fn the_tab_reveal_target_measures_from_the_start_after_a_location_change() {
        let members = twelve();
        let offsets = member_offsets(&members);
        assert_eq!(
            tab_reveal_target(true, 500.0, 340.0, 0, &members),
            Some(0.0),
            "from the start, a tab that fits there scrolls to the start"
        );
        let (leading, trailing) = offsets[11];
        assert_eq!(
            tab_reveal_target(true, leading, 340.0, 11, &members),
            Some(trailing - 340.0),
            "from the start, an overflowing tab scrolls to its position, whatever the offset held"
        );
        let (leading, _) = offsets[2];
        assert_eq!(
            tab_reveal_target(false, leading, 340.0, 2, &members),
            None,
            "not from the start, a visible tab is left alone"
        );
        assert_eq!(
            tab_reveal_target(false, 0.0, 340.0, 11, &members),
            Some(trailing - 340.0)
        );
    }

    /// FR-005: exactly one tab is marked, for **every** combination of mode and active instance.
    ///
    /// "Never zero, never two" is a claim about **totality**, so this is where it is proved: the
    /// function returns one `StripTab` and there is nowhere else for the answer to go. The case
    /// that does the work is the last one — a `Regular` session whose `active_shell` is `None`,
    /// which is what an `Option<ShellInstanceId>` would have had to answer twice for (Principle V).
    #[test]
    fn exactly_one_tab_is_marked_in_every_state() {
        // AI CLI mode: the AI tab is marked whatever the instances are doing.
        for active in [None, Some(0), Some(1)] {
            let (state, id) = state_with_instances(
                TerminalMode::AiCli,
                SessionLifecycle::Running,
                &[ShellLifecycle::Running, ShellLifecycle::Running],
                active,
            );
            assert_eq!(
                marked_tab(&state, id),
                StripTab::Ai,
                "in AiCli mode the AI tab is the displayed one regardless of active_shell \
                 ({active:?})"
            );
        }

        // Regular mode with an instance selected: that instance's tab, and not the AI tab.
        let (state, id) = state_with_instances(
            TerminalMode::Regular,
            SessionLifecycle::Running,
            &[ShellLifecycle::Running, ShellLifecycle::Running],
            Some(1),
        );
        let selected = state.active_sessions()[0].shells[1].id;
        assert_eq!(marked_tab(&state, id), StripTab::Instance(selected));

        // Regular mode with **no** instance selected. `None` already means "this session has no
        // active instance" in this file, so overloading it to also mean "the AI tab" would make
        // this case unanswerable. The AI tab is what the pane shows, so the AI tab is marked.
        let (state, id) =
            state_with_instances(TerminalMode::Regular, SessionLifecycle::Running, &[], None);
        assert_eq!(
            marked_tab(&state, id),
            StripTab::Ai,
            "a Regular session with nothing to show falls back to the AI tab — never to no tab \
             at all, which is the state FR-005 exists to forbid"
        );

        // A session that is not in the workspace at all still answers, because the return type
        // leaves no room not to.
        let (state, _) =
            state_with_instances(TerminalMode::Regular, SessionLifecycle::Running, &[], None);
        assert_eq!(marked_tab(&state, SessionId::new()), StripTab::Ai);
    }

    /// FR-012d / FR-012e, research R1 and R2: **one** predicate answers "this process is stopped"
    /// for both lifecycle vocabularies.
    ///
    /// Asserted for every variant of both enums **by name**, so a variant added later fails here
    /// rather than silently defaulting into one answer or the other. The rule is the same in both
    /// rows and it is FR-012d's own: the mark appears exactly where a restart can act.
    #[test]
    fn one_predicate_calls_a_process_stopped_in_both_vocabularies() {
        let (state, id) = state_with_instances(
            TerminalMode::AiCli,
            SessionLifecycle::Running,
            &[
                ShellLifecycle::NotStarted,
                ShellLifecycle::Starting,
                ShellLifecycle::Running,
                ShellLifecycle::Exited,
            ],
            None,
        );
        let shells: Vec<_> = state.active_sessions()[0]
            .shells
            .iter()
            .map(|s| s.id)
            .collect();
        for (i, (want, why)) in [
            (true, "a shell that was never started is restartable"),
            (
                false,
                "a starting shell is in progress, not stopped (FR-012e)",
            ),
            (false, "a running shell has nothing to restart"),
            (true, "an exited shell is the case FR-012 exists for"),
        ]
        .iter()
        .enumerate()
        .map(|(i, (w, why))| (i, (*w, *why)))
        {
            assert_eq!(
                process_stopped(&state, id, StripTab::Instance(shells[i])),
                want,
                "{why}"
            );
        }

        for (lifecycle, want, why) in [
            (SessionLifecycle::Idle, true, "a persisted-but-stopped session resumes on request"),
            (SessionLifecycle::Starting, false, "starting is in progress, not stopped (FR-012e)"),
            (SessionLifecycle::Running, false, "a running AI process has nothing to restart"),
            (
                SessionLifecycle::Restarting { attempts: 1 },
                false,
                "an auto-restart is already under way; a mark here would point at an action                  nobody needs to take",
            ),
            (failed(), true, "auto-restart gave up; a manual one is the offer"),
            (
                SessionLifecycle::InterruptedResumable,
                true,
                "the service restarted and found a conversation — resuming it is the one action",
            ),
        ] {
            let (state, id) =
                state_with_instances(TerminalMode::AiCli, lifecycle.clone(), &[], None);
            assert_eq!(
                process_stopped(&state, id, StripTab::Ai),
                want,
                "{lifecycle:?}: {why}"
            );
        }
    }

    /// R2's other half: the strip's restart is offered for the process the pane shows. In AI CLI
    /// mode, or in Regular mode with no instance shown, that is the AI process and its lifecycle
    /// decides; with an instance shown, that instance's lifecycle does, whatever the AI process
    /// is doing.
    #[test]
    fn the_strips_restart_is_offered_for_the_process_the_pane_shows() {
        const IDLE: SessionLifecycle = SessionLifecycle::Idle;
        const RUNNING: SessionLifecycle = SessionLifecycle::Running;
        for (mode, shells, active, lifecycle, offered) in [
            (TerminalMode::AiCli, &[][..], None, IDLE, true),
            (TerminalMode::AiCli, &[][..], None, RUNNING, false),
            (TerminalMode::AiCli, &[][..], None, failed(), true),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Exited][..],
                Some(0),
                RUNNING,
                true,
            ),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Exited][..],
                Some(0),
                IDLE,
                true,
            ),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Running][..],
                Some(0),
                IDLE,
                false,
            ),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Running][..],
                Some(0),
                failed(),
                false,
            ),
            (TerminalMode::Regular, &[][..], None, IDLE, true),
            (TerminalMode::Regular, &[][..], None, RUNNING, false),
        ] {
            let (state, id) = state_with_instances(mode, lifecycle.clone(), shells, active);
            assert_eq!(
                attached_process_restartable(&state, id),
                offered,
                "{mode:?}/{shells:?}/{lifecycle:?}"
            );
        }
    }

    /// `012` BUG-004 / FR-010. The bar's `restart` control restarted the **session**, whose AI CLI
    /// primary is still alive in Regular mode — so `start_session` took its already-live early
    /// return and pressing the control did nothing at all. With a single instance there is no tab
    /// strip either, so FR-010 had no reachable route in the commonest case.
    ///
    /// Asserted on the message rather than on the presence of a control: `attached_process_restartable`
    /// already decides *whether* to offer a restart by asking which process is attached, and the
    /// defect was the button not asking the same question about *what to restart*. Both now come
    /// from one reading, which is what stops them disagreeing again (this feature's BUG-001, and
    /// `025`'s, are the same shape).
    #[test]
    fn the_bars_restart_targets_the_process_the_bar_is_describing() {
        // AI CLI mode: the session is the attached process, so the session-level restart is right.
        let (state, id) = state_showing(SessionLifecycle::Idle);
        assert_eq!(
            restart_message(&state, id),
            Message::Session(SessionMsg::TerminalRestartRequested)
        );

        // Regular mode with an instance: that instance is what the bar reports on, and what
        // `restart` must act on.
        let (mut state, id) = state_showing(SessionLifecycle::Running);
        let instance = {
            let (_, session) = state.workspace.find_session_mut(id).expect("session");
            session.mode = TerminalMode::Regular;
            session.open_shell_instance()
        };
        assert_eq!(
            restart_message(&state, id),
            Message::Session(SessionMsg::ShellInstanceRestartRequested(id, instance)),
            "in Regular mode the bar describes the attached shell instance, so its restart must \
             name that instance — restarting the session leaves the dead shell dead"
        );

        // Regular mode with no instance yet: nothing instance-shaped to name, so the session-level
        // request stands and lazily opens the first one.
        let (mut state, id) = state_showing(SessionLifecycle::Running);
        {
            let (_, session) = state.workspace.find_session_mut(id).expect("session");
            session.mode = TerminalMode::Regular;
        }
        assert_eq!(
            restart_message(&state, id),
            Message::Session(SessionMsg::TerminalRestartRequested)
        );

        // An unknown session must not panic the render path.
        assert_eq!(
            restart_message(&state, SessionId::new()),
            Message::Session(SessionMsg::TerminalRestartRequested)
        );
    }

    /// `010` BUG-017 step 3, the display. FR-005 / SC-012, quickstart S4: *"on the next attach it
    /// shows the failed state with the reason **and attempt count**"*.
    ///
    /// Steps 1 and 2 carried both fields the whole way from the FSM to the client's `State`, and
    /// then nothing read them — the bar said the bare word `failed` for every give-up, whatever it
    /// had cost and whatever had killed it.
    ///
    /// Driven at three different counts because `attempts` was a **constant** for the whole of this
    /// bug's life, and a constant that happened to be right for the crash loop. One value cannot
    /// tell a measurement from `MAX_RESTART_ATTEMPTS` spelled out; three can.
    #[test]
    fn the_bar_names_the_attempts_a_give_up_actually_spent() {
        for attempts in [1u8, 2, 3] {
            let (state, id) = state_showing(SessionLifecycle::Failed {
                reason: "Gave up after 1 restart attempt — last exit: exit status 1.".into(),
                attempts,
            });
            let status = session_status(&state, id);
            assert!(
                status.contains("failed"),
                "a give-up is still a failure and the bar still says so; got {status:?}"
            );
            assert!(
                status.contains(&attempts.to_string()),
                "the bar must name the budget this give-up actually spent — a spawn that failed                  once reads as three attempts the moment this is a constant ({attempts} spent,                  got {status:?})"
            );
        }
    }

    // ---- Feature 1269 (M4): the Review tab in the strip (U13–U15) ----

    use crate::features::sidebar::Msg as SidebarMsg;
    use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
    use micold_core::review::{LineRange, RelPath, Side};
    use micold_core::review_tabs::Freshness;

    /// The S1 fixture with its location's Review tab made with session `shown` current, then hidden
    /// by selecting that session again when `hide`.
    fn with_review(shown: usize, hide: bool) -> (State, [SessionId; 3]) {
        let (mut state, ids) = strip_fixture(shown);
        state.update(Message::Sidebar(SidebarMsg::ReviewChangesRequested(
            SessionLocation::Default,
        )));
        let _ = state.take_changes_effect();
        if hide {
            state.update(Message::Session(SessionMsg::Selected(ids[shown])));
        }
        (state, ids)
    }

    fn review_key() -> micold_core::review_tabs::EntryKey {
        (std::path::PathBuf::from("/repo"), SessionLocation::Default)
    }

    /// The strip's Review member, if any.
    fn review_of(model: &StripModel) -> Option<(ReviewTabFace, bool)> {
        model.members.iter().find_map(|m| match m {
            Member::Review { face, shown, .. } => Some((face.clone(), *shown)),
            _ => None,
        })
    }

    fn comment(state: CommentState) -> ReviewComment {
        ReviewComment {
            id: CommentId::new(),
            path: RelPath::from_native("src/lib.rs").expect("relative path"),
            side: Side::New,
            range: LineRange::new(1, 1).expect("a range"),
            quote: vec!["line 1".into()],
            text: "about it".into(),
            state,
            created: 1,
            pr: None,
        }
    }

    /// T033 (U13, FR-001): with the Review tab hidden it is the last member, after the sessions,
    /// the shown session's shells and "+", and the session's own member stays marked.
    #[test]
    fn a_hidden_review_tab_is_the_last_member_and_marks_nothing() {
        let (state, [m, h1, h2]) = with_review(0, true);
        assert_eq!(
            state.review_shown(),
            None,
            "the premise: the review is hidden"
        );
        let shells = shells_of(&state, m);
        let model = StripModel::of(&state).expect("a session is shown");
        assert_eq!(
            keys(&model),
            vec![
                MemberKey::Session(m),
                MemberKey::Shell(m, shells[0]),
                MemberKey::Shell(m, shells[1]),
                MemberKey::Plus,
                MemberKey::Session(h1),
                MemberKey::Session(h2),
                MemberKey::Review,
            ]
        );
        let marked: Vec<_> = model
            .members
            .iter()
            .filter(|x| x.marked())
            .map(Member::key)
            .collect();
        assert_eq!(marked, vec![MemberKey::Session(m)]);
        assert_eq!(review_of(&model).map(|(_, shown)| shown), Some(false));
        assert!(!model.review_shown());
    }

    /// T033 (U13): with the Review tab shown the members are the same, the Review member is the
    /// one marked, and the current session's member is still its shown one.
    #[test]
    fn a_shown_review_tab_is_the_one_marked_member() {
        let (state, [m, _, _]) = with_review(0, false);
        assert!(
            state.review_shown().is_some(),
            "the premise: the review is shown"
        );
        let mut hidden = state.clone();
        hidden.update(Message::Session(SessionMsg::Selected(m)));
        assert_eq!(hidden.review_shown(), None);
        let model = StripModel::of(&state).expect("a session is current");
        assert_eq!(
            keys(&model),
            keys(&StripModel::of(&hidden).expect("shown")),
            "showing the tab changes no member"
        );
        let marked: Vec<_> = model
            .members
            .iter()
            .filter(|x| x.marked())
            .map(Member::key)
            .collect();
        assert_eq!(
            marked,
            vec![MemberKey::Review],
            "no session or shell member"
        );
        assert!(matches!(
            model.members[0],
            Member::Session {
                shown: true,
                marked: false,
                ..
            }
        ));
        assert_eq!(model.marked_index(), Some(model.members.len() - 1));
        assert!(model.review_shown());
        assert_eq!(model.status, "running", "the status keeps its place");
    }

    /// T033 (U13, US2.6): a sessionless worktree's shown review is the strip's only member, with no
    /// status and no restart; New session still starts one there.
    #[test]
    fn a_sessionless_locations_strip_is_its_review_alone() {
        let (mut state, _) = strip_fixture(0);
        state.worktree.worktrees = vec![micold_core::worktree::Worktree {
            dir_name: "z".into(),
            path: "/repo/.claude/worktrees/z".into(),
            branch: Some("z".into()),
            status: micold_core::worktree::WorktreeStatus::Valid,
            included: false,
        }];
        let z = SessionLocation::Worktree("z".into());
        state.update(Message::Sidebar(SidebarMsg::ReviewChangesRequested(
            z.clone(),
        )));
        let _ = state.take_changes_effect();
        assert_eq!(
            state.shown_location(),
            Some(z.clone()),
            "the premise: Z's review is shown"
        );
        let model = StripModel::of(&state).expect("the review's strip");
        assert_eq!(keys(&model), vec![MemberKey::Review]);
        assert!(model.members[0].marked());
        assert_eq!(model.status, "");
        assert_eq!(model.restart, None);
        assert_eq!(model.location, Some(z));
    }

    /// T033 (U13, FR-009): the badge counts the location's drafts, sent comments not; `None` while
    /// its comments are unknown.
    #[test]
    fn the_badge_counts_the_locations_drafts_and_none_while_unknown() {
        let (mut state, _) = with_review(0, true);
        let badge = |state: &State| {
            review_of(&StripModel::of(state).expect("shown")).and_then(|(face, _)| face.badge)
        };
        assert_eq!(badge(&state), None, "comments unknown");
        state.changes.reviews.insert(
            (std::path::PathBuf::from("/repo"), String::new()),
            crate::features::changes::ReviewView {
                comments: vec![
                    comment(CommentState::Draft),
                    comment(CommentState::Draft),
                    comment(CommentState::Sent { at: 1 }),
                ],
                sending: false,
            },
        );
        assert_eq!(badge(&state).map(|n| n.get()), Some(2));
        state.changes.reviews.insert(
            (std::path::PathBuf::from("/repo"), "elsewhere".into()),
            crate::features::changes::ReviewView {
                comments: vec![comment(CommentState::Draft); 5],
                sending: false,
            },
        );
        assert_eq!(
            badge(&state).map(|n| n.get()),
            Some(2),
            "never another location's drafts"
        );
    }

    /// T033 (U13, FR-010): a hidden tab whose files changed carries the changed mark; a shown one
    /// never does.
    #[test]
    fn only_a_hidden_review_tab_carries_the_changed_mark() {
        let changed = |state: &State| {
            review_of(&StripModel::of(state).expect("shown")).map(|(face, _)| face.changed)
        };
        let (mut shown, _) = with_review(0, false);
        let _ = shown.changes.tabs.files_changed(&review_key());
        assert_eq!(changed(&shown), Some(false), "shown: re-read, no mark");
        let (mut hidden, _) = with_review(0, true);
        let _ = hidden.changes.tabs.files_changed(&review_key());
        assert_eq!(
            hidden
                .changes
                .tabs
                .get(&review_key())
                .map(|t| t.freshness()),
            Some(Freshness::Changed)
        );
        assert_eq!(changed(&hidden), Some(true));
    }

    /// T034 (U14, US5.4): the Review tab's press asks for its location's review; its × closes it.
    #[test]
    fn the_review_tab_presses_review_changes_and_its_close_closes_it() {
        let (state, _) = with_review(0, true);
        let model = StripModel::of(&state).expect("shown");
        let review = model
            .members
            .iter()
            .find(|m| m.key() == MemberKey::Review)
            .expect("the Review member");
        assert_eq!(
            review.press(),
            Message::Sidebar(SidebarMsg::ReviewChangesRequested(SessionLocation::Default))
        );
        assert_eq!(
            review.close(),
            Some(Message::Changes(crate::features::changes::Msg::TabClosed {
                project: std::path::PathBuf::from("/repo"),
                entry: SessionLocation::Default,
            }))
        );
        assert_eq!(review.menu_tab(), None, "no menu: nothing to restart");
        assert!(!review.stopped());
    }

    /// Review A M4 F1 (US1.2, U2): with the Review tab shown, pressing the current session's tab,
    /// one of its shell tabs or the "+" shows the terminal: the review is hidden, kept.
    #[test]
    fn a_session_shell_or_plus_press_hides_a_shown_review() {
        let (state, [m, _, _]) = with_review(0, false);
        let model = StripModel::of(&state).expect("shown");
        for key in [
            MemberKey::Session(m),
            MemberKey::Shell(m, shells_of(&state, m)[0]),
            MemberKey::Plus,
        ] {
            let mut pressed = state.clone();
            assert!(pressed.review_shown().is_some(), "the premise");
            let member = model
                .members
                .iter()
                .find(|x| x.key() == key)
                .expect("member");
            pressed.update(member.press());
            assert_eq!(pressed.review_shown(), None, "{key:?} hides the review");
            assert!(
                pressed.changes.tabs.has(&review_key()),
                "{key:?} keeps the Review tab"
            );
        }
    }

    fn review_member_with(drafts: Option<usize>, freshness: Freshness) -> Member {
        Member::Review {
            project: std::path::PathBuf::from("/repo"),
            entry: SessionLocation::Default,
            face: ReviewTabFace::of(drafts, freshness, false),
            shown: false,
        }
    }

    /// T034 (U15, FR-015): the Review member is `REVIEW_WIDTH` wide whatever its badge or mark, and
    /// the offsets and the reveal count it.
    #[test]
    fn the_review_member_has_one_width_and_the_offsets_and_reveal_count_it() {
        for member in [
            review_member_with(None, Freshness::Fresh),
            review_member_with(Some(1), Freshness::Fresh),
            review_member_with(Some(150), Freshness::Fresh),
            review_member_with(Some(150), Freshness::Changed),
        ] {
            assert_eq!(member.width(), tab::REVIEW_WIDTH, "{member:?}");
        }
        let mut members = twelve();
        members.push(review_member_with(Some(150), Freshness::Changed));
        let offsets = member_offsets(&members);
        let (before, last) = (offsets[11], offsets[12]);
        assert!(
            (last.0 - (before.1 + spacing::SM)).abs() <= EDGE_TOLERANCE
                && (last.1 - last.0 - tab::REVIEW_WIDTH).abs() <= EDGE_TOLERANCE,
            "{last:?} after {before:?}"
        );
        assert_eq!(
            scroll_into_view(12, 0.0, 340.0, &members),
            Some(last.1 - 340.0),
            "showing it brings it into view"
        );
    }
}
