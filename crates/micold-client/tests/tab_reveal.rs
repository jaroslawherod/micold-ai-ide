//! Every strip control that changes which tab is marked asks for it to be scrolled into view
//! (feature 026 FR-002d, feature 027 FR-002/FR-004).
//!
//! # Why a table rather than a case
//!
//! `features::session::arm_tab_reveal` claims the invariant in its own doc comment — "called from
//! every reducer arm that can change which tab is marked" — and nothing held it to that. Two of the
//! four arms called it; `ShellInstanceOpenRequested` and `ShellInstanceCloseRequested` did not, and
//! the second of those is the one a visual pass caught (T024): pressing "+" for the sixth instance
//! created a tab, marked it, and left it beyond the strip's trailing edge, so the user's own new
//! terminal was the one thing the bar would not show them. The reveal machinery was all present and
//! correct; the arm simply never asked it to run.
//!
//! That is a claim about a *set* of arms, so it is asserted about the set. A single case would have
//! passed for the two that already armed and said nothing about the next control added to the strip.
//!
//! **Scope: the strip's own controls.** Switching sessions changes the strip wholesale and is the
//! sidebar reveal's business (feature 024 FR-009); what is pinned here is the four messages that
//! move the mark *within* one session's strip.
//!
//! The flag rather than a scroll offset, because that is all a reducer can decide: the viewport's
//! width is not known in Tier 1 and `main.rs`'s `tab_reveal_scroll` drains the flag once it is
//! (which is also why it may still resolve to "already visible, move nothing" — FR-002d lets a user
//! scroll away by hand).

use micold_client::app::{Message, State};
use micold_core::session::{AiCli, Session, SessionLocation};

/// One session, displayed, with a terminal in front of the user.
fn showing_a_terminal() -> State {
    let mut s = State::default();
    s.update(Message::SessionStarted(Session::start_new(
        SessionLocation::Worktree("feat-x".to_string()),
        AiCli::ClaudeCode,
    )));
    s
}

#[test]
fn every_strip_control_that_moves_the_mark_arms_the_reveal() {
    let shell = micold_core::session::ShellInstanceId(1);
    /// A press, built from the state it is applied to (some name the displayed session).
    type Press = (&'static str, Box<dyn Fn(&State) -> Message>);

    let presses: Vec<Press> = vec![
        (
            "ShellInstanceOpenRequested",
            Box::new(|_: &State| Message::ShellInstanceOpenRequested),
        ),
        (
            "ShellInstanceSelected",
            Box::new(move |s: &State| {
                Message::ShellInstanceSelected(s.active_session.expect("displayed"), shell)
            }),
        ),
        (
            "ShellInstanceCloseRequested",
            Box::new(move |s: &State| {
                Message::ShellInstanceCloseRequested(s.active_session.expect("displayed"), shell)
            }),
        ),
        (
            "TerminalAiCliSelected",
            Box::new(|s: &State| {
                Message::TerminalAiCliSelected(s.active_session.expect("displayed"))
            }),
        ),
    ];

    for (name, build) in presses {
        let mut s = showing_a_terminal();
        assert!(
            !s.pending_tab_reveal,
            "{name}: precondition — nothing armed yet"
        );

        let message = build(&s);
        s.update(message);

        assert!(
            s.pending_tab_reveal,
            "{name} changes which tab is marked, so it must ask for that tab to be scrolled into \
             view (feature 026 FR-002d).\n\n\
             Unarmed, the mark can land outside the strip's viewport and stay there: the tab is \
             selected, the pane switches, the process attaches — and the one cue that says which \
             tab the user is on is the one thing off screen. Feature 027 made that reachable with \
             a press, because the \"+\" now sits beside the strip and creates the tab it hides."
        );
    }
}
