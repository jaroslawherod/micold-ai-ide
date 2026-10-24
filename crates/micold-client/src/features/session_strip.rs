//! The session strip's rules, render-free (feature 1126, research R7): which tab is marked, whether
//! a member's process is stopped, the shown session's status words and restart, and the strip's
//! overflow and reveal arithmetic. Moved from `ui/terminal.rs`, which re-exports them; the view
//! only lays these answers out. Like every feature module it names no rendering framework
//! (`tests/features_are_render_free.rs`).

use crate::app::{Message, State};
use crate::features::session::Msg as SessionMsg;
use crate::ui::material::TAB_WIDTH;
use micold_core::session::{
    SessionId, SessionLifecycle, ShellInstanceId, ShellLifecycle, TerminalMode,
};
use micold_core::tokens::spacing;

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

/// What lies beyond the strip's edges, for a viewport `viewport` wide holding `tabs` tabs and
/// scrolled `offset` from the leading end (FR-002e; feature 027 FR-003).
///
/// # Why the content width is derived here and not measured
///
/// `State::tab_strip_content_width` is only ever written by a scroll event, so between the strip's
/// first layout and the user's first scroll it holds whatever it last held — and since feature 027
/// the content width is a function of the viewport (the slack is laid out inside it), so a stale
/// pair can claim an overflow the strip does not have. It did: the first terminal instance opened
/// in a fresh session drew a trailing fade across its own tab, over the indicator, with nothing
/// beyond that edge at all. Found by the visual pass, invisible to every gate — a gradient occupies
/// the same box whether it is opaque or not.
///
/// Deriving it from the tab count is what makes the fade and the geometry incapable of disagreeing:
/// the strip lays out `leading_slack(..) + natural_strip_width(..)`, so that *is* the content width,
/// and it is known on the same frame the layout is rather than one scroll later.
///
/// A viewport of zero means "not measured yet" — nothing has been laid out, so nothing is known to
/// lie beyond anything, and the honest answer is neither edge.
pub(crate) fn strip_overflow(offset: f32, viewport: f32, tabs: usize) -> Beyond {
    if viewport <= 0.0 {
        return Beyond {
            leading: false,
            trailing: false,
        };
    }
    overflowing(
        offset,
        viewport,
        leading_slack(viewport, tabs) + natural_strip_width(tabs),
    )
}

/// The distance between one tab's leading edge and the next's.
fn tab_pitch() -> f32 {
    TAB_WIDTH + spacing::SM
}

/// Where to scroll a viewport `viewport` wide, currently at `offset`, to bring the tab at `index`
/// fully into view — or `None` if it already is (FR-002d).
///
/// `None` rather than "scroll to where you already are" is the requirement, not an optimisation.
/// FR-002d lets a user scroll away from the marked tab by hand, and a reveal that fires on every
/// selection would yank them back each time — including on selections made from the keyboard
/// rather than with the strip, which are the ones they were not looking at the strip for.
///
/// It scrolls the tab **just** into view rather than centring it: the tabs either side are context,
/// and a strip that recentres on every press moves more than the selection did.
pub fn scroll_into_view(index: usize, offset: f32, viewport: f32) -> Option<f32> {
    let pitch = tab_pitch();
    let leading = index as f32 * pitch;
    let trailing = leading + TAB_WIDTH;
    if leading + EDGE_TOLERANCE < offset {
        Some(leading)
    } else if trailing > offset + viewport + EDGE_TOLERANCE {
        Some(trailing - viewport)
    } else {
        None
    }
}

/// The marked tab's position **within the scrolling region**, or `None` when the marked tab is the
/// pinned AI one (FR-002b, FR-002d).
///
/// `None` is not a failure and not "nothing is marked" — FR-005 makes that unrepresentable. It is
/// the AI tab, which sits outside the viewport and therefore cannot be scrolled into it. There is
/// nothing to do, which is exactly the right answer.
pub fn marked_tab_index(state: &State) -> Option<usize> {
    let id = state.session.active?;
    let StripTab::Instance(instance) = marked_tab(state, id) else {
        return None;
    };
    state
        .active_sessions()
        .iter()
        .find(|s| s.id == id)?
        .shells
        .iter()
        .position(|s| s.id == instance)
}

/// What a strip of `tabs` members measures, derived from the tab's own width and the strip's own
/// spacing (feature 027 FR-001).
///
/// # Why this is derived rather than measured
///
/// `State::tab_strip_content_width` is the same number, reported by the scrollable from the content
/// it actually laid out — and it cannot be used here, because [`leading_slack`]'s result becomes
/// part of that content. Feeding the measurement back in would make the space a function of itself:
/// a strip with room to spare would compute a slack, grow its content by exactly that slack,
/// measure no slack on the next frame, collapse to the left, and flip back — oscillating between
/// flush-right and flush-left for as long as it was on screen. The tab count is an input the strip
/// does not influence, so deriving from it terminates.
pub fn natural_strip_width(tabs: usize) -> f32 {
    match tabs {
        0 => 0.0,
        n => n as f32 * TAB_WIDTH + (n - 1) as f32 * spacing::SM,
    }
}

/// The empty width the strip leaves at its **leading** edge so its tabs finish against the "+" and
/// the AI tab at the bar's trailing end (feature 027 FR-001).
///
/// Zero once the tabs no longer fit: there is no slack to distribute then, and the strip scrolls
/// instead (feature 026 FR-002a). That is also what keeps the edge fades honest — the fade asks
/// whether content exceeds viewport, and a slack that padded an already-full strip would report an
/// overflow that is not there.
pub fn leading_slack(viewport: f32, tabs: usize) -> f32 {
    (viewport - natural_strip_width(tabs)).max(0.0)
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

    /// FR-002e + feature 027 FR-003: the fade asks the tab count, not the last measurement.
    ///
    /// The case this exists for is the one the visual pass found: a strip that fits draws no fade,
    /// on the very first frame, before anything has scrolled. `overflowing` above is still right
    /// about the arithmetic; what was wrong was the numbers it was handed.
    #[test]
    fn a_strip_that_fits_fades_neither_edge_before_anything_has_scrolled() {
        let beyond = |leading, trailing| Beyond { leading, trailing };
        let pitch = TAB_WIDTH + spacing::SM;

        // Never laid out: nothing is known, so nothing is claimed.
        assert_eq!(strip_overflow(0.0, 0.0, 1), beyond(false, false));
        // One tab in a bar with room for five — the state that drew a fade over its own tab.
        assert_eq!(strip_overflow(0.0, 5.0 * pitch, 1), beyond(false, false));
        // Room for exactly as many as there are.
        assert_eq!(
            strip_overflow(0.0, 3.0 * TAB_WIDTH + 2.0 * spacing::SM, 3),
            beyond(false, false)
        );
        // More tabs than room, unscrolled: there is something that way, and only that way.
        assert_eq!(strip_overflow(0.0, 3.0 * pitch, 6), beyond(false, true));
        // Scrolled off the leading end with more still to come.
        assert_eq!(strip_overflow(pitch, 3.0 * pitch, 6), beyond(true, true));
    }

    /// FR-002d: changing the marked tab yields a scroll request for **that** tab, and none at all
    /// when it is already fully visible.
    ///
    /// The second half is the one worth a test. A reveal that did not need to move the strip must
    /// not move it — a user who has scrolled by hand (which FR-002d explicitly allows) would
    /// otherwise be yanked back on every selection, including selections they made from the
    /// keyboard rather than with the strip.
    #[test]
    fn the_marked_tab_is_scrolled_into_view_only_when_it_is_not() {
        let pitch = TAB_WIDTH + spacing::SM;

        // Already fully visible at the leading end: nothing to do.
        assert_eq!(scroll_into_view(0, 0.0, 3.0 * pitch), None);
        assert_eq!(scroll_into_view(1, 0.0, 3.0 * pitch), None);

        // Beyond the trailing edge: scroll just far enough that its **trailing edge** lands on the
        // viewport's, not far enough to centre it — the neighbouring tabs are context, and a strip
        // that recentres on every press moves more than the selection did.
        //
        // The figure is `5·pitch + TAB_WIDTH - viewport`, not `6·pitch - viewport`: a pitch is a
        // tab plus the gap that follows it, and there is no gap after the last tab shown. The 8dp
        // between them is a gap this would have opened at the viewport's trailing edge.
        assert_eq!(
            scroll_into_view(5, 0.0, 3.0 * pitch),
            Some(5.0 * pitch + TAB_WIDTH - 3.0 * pitch),
        );

        // Beyond the leading edge: scroll to put its leading edge on the viewport's.
        assert_eq!(scroll_into_view(0, 4.0 * pitch, 3.0 * pitch), Some(0.0));
        assert_eq!(
            scroll_into_view(2, 4.0 * pitch, 3.0 * pitch),
            Some(2.0 * pitch)
        );
    }

    /// Feature 027 FR-003: the strip sits against the **trailing** edge of its viewport, and the
    /// slack that puts it there is a pure function of the tab count.
    ///
    /// Derived from the count, never from the measured content width. The slack is laid out
    /// *inside* the scrolling content, so it is part of what `State::tab_strip_content_width`
    /// reports back — computing it from that measurement would feed its own output back in and the
    /// strip would swing flush-right, flush-left, flush-right on successive frames. From the count
    /// it terminates by construction, which is why this is arithmetic a test can read rather than a
    /// `Length::Fill`: `Fill` inside a horizontal `Scrollable` resolves against an unbounded width
    /// limit — infinity, not "whatever is left" — so it pushes the strip out of the viewport
    /// entirely.
    #[test]
    fn the_strip_hugs_the_trailing_edge_of_its_viewport() {
        let pitch = TAB_WIDTH + spacing::SM;

        // No tabs, no strip: nothing to push, and a viewport's worth of slack would be a scrollable
        // region made entirely of emptiness.
        assert_eq!(natural_strip_width(0), 0.0);
        assert_eq!(leading_slack(600.0, 0), 600.0);

        // One tab is a tab, not a tab plus a trailing gap — n tabs carry n-1 gaps between them.
        assert_eq!(natural_strip_width(1), TAB_WIDTH);
        assert_eq!(natural_strip_width(3), 3.0 * TAB_WIDTH + 2.0 * spacing::SM);

        // Room to spare: the slack is exactly what is left, so the strip's trailing edge lands on
        // the viewport's however few tabs there are.
        assert_eq!(leading_slack(3.0 * pitch, 1), 3.0 * pitch - TAB_WIDTH);
        assert_eq!(
            leading_slack(3.0 * pitch, 2),
            3.0 * pitch - (2.0 * TAB_WIDTH + spacing::SM)
        );

        // Overflowing: there is no slack to give, and the answer is zero rather than negative. A
        // negative width would be a panic in iced; worse, a slack that shrank as the strip grew
        // would let the strip keep scrolling past its own leading tab.
        assert_eq!(leading_slack(3.0 * pitch, 6), 0.0);
        assert_eq!(leading_slack(0.0, 3), 0.0);
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

    /// R2's other half: the bar's own restart control and the strip must not be able to disagree.
    ///
    /// `attached_process_restartable` is now a call into the predicate above rather than a second
    /// match statement, so this asserts the two give the same answer for the process the bar is
    /// describing. This file has paid twice for the alternative — `empty_terminal_message` and the
    /// bar disagreed "for exactly as long as they were two readings of one fact", and BUG-004 was
    /// `restart_message` re-deriving something the predicate beside it already had.
    #[test]
    fn the_bar_and_the_strip_read_one_predicate() {
        for (mode, shells, active) in [
            (TerminalMode::AiCli, &[][..], None),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Exited][..],
                Some(0),
            ),
            (
                TerminalMode::Regular,
                &[ShellLifecycle::Running][..],
                Some(0),
            ),
            (TerminalMode::Regular, &[][..], None),
        ] {
            for lifecycle in [SessionLifecycle::Idle, SessionLifecycle::Running, failed()] {
                let (state, id) = state_with_instances(mode, lifecycle.clone(), shells, active);
                let attached = match mode {
                    TerminalMode::AiCli => StripTab::Ai,
                    TerminalMode::Regular => state.active_sessions()[0]
                        .active_shell
                        .map(StripTab::Instance)
                        .unwrap_or(StripTab::Ai),
                };
                assert_eq!(
                    attached_process_restartable(&state, id),
                    process_stopped(&state, id, attached),
                    "{mode:?}/{lifecycle:?}: the bar's restart control and the strip's mark are \
                     two readings of one fact and must not be able to differ"
                );
            }
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
}
