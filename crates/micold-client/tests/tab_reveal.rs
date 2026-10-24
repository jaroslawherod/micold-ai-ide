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
//! **Scope: the strip's own controls.** Since feature 1126 the strip above the terminal holds every
//! session of the shown location, so pressing another session's tab moves the mark too (contract
//! S5): it sends the sidebar row's `Selected`, which arms the reveal like the other four.
//!
//! The flag rather than a scroll offset, because that is all a reducer can decide: the viewport's
//! width is not known in Tier 1 and `main.rs`'s `tab_reveal_scroll` drains the flag once it is
//! (which is also why it may still resolve to "already visible, move nothing" — FR-002d lets a user
//! scroll away by hand).

use micold_client::app::{Message, State};
use micold_client::features::session::Msg as SessionMsg;
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionId, SessionLocation};

/// Two sessions at a project's root, the second displayed, with a terminal in front of the user.
fn showing_a_terminal() -> (State, SessionId) {
    let mut s = State::default();
    let path = std::path::PathBuf::from("/repo");
    s.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    s.workspace.active = Some(path);
    let first = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
    let other = first.id;
    s.update(Message::Session(SessionMsg::Started(first)));
    s.update(Message::Session(SessionMsg::Started(Session::start_new(
        SessionLocation::Default,
        AiCli::ClaudeCode,
    ))));
    s.session.pending_tab_reveal = false;
    (s, other)
}

#[test]
fn every_strip_control_that_moves_the_mark_arms_the_reveal() {
    let shell = micold_core::session::ShellInstanceId(1);
    /// A press, built from the state it is applied to (some name the displayed session).
    type Press = (&'static str, Box<dyn Fn(&State, SessionId) -> Message>);

    let presses: Vec<Press> = vec![
        (
            "ShellInstanceOpenRequested",
            Box::new(|_: &State, _| Message::Session(SessionMsg::ShellInstanceOpenRequested)),
        ),
        (
            "ShellInstanceSelected",
            Box::new(move |s: &State, _| {
                Message::Session(SessionMsg::ShellInstanceSelected(
                    s.session.active.expect("displayed"),
                    shell,
                ))
            }),
        ),
        (
            "ShellInstanceCloseRequested",
            Box::new(move |s: &State, _| {
                Message::Session(SessionMsg::ShellInstanceCloseRequested(
                    s.session.active.expect("displayed"),
                    shell,
                ))
            }),
        ),
        (
            "TerminalAiCliSelected",
            Box::new(|s: &State, _| {
                Message::Session(SessionMsg::TerminalAiCliSelected(
                    s.session.active.expect("displayed"),
                ))
            }),
        ),
        (
            "Selected (another session's tab)",
            Box::new(|_: &State, other| Message::Session(SessionMsg::Selected(other))),
        ),
    ];

    for (name, build) in presses {
        let (mut s, other) = showing_a_terminal();
        assert!(
            !s.session.pending_tab_reveal,
            "{name}: precondition — nothing armed yet"
        );

        let message = build(&s, other);
        s.update(message);

        assert!(
            s.session.pending_tab_reveal,
            "{name} changes which tab is marked, so it must ask for that tab to be scrolled into \
             view (feature 026 FR-002d).\n\n\
             Unarmed, the mark can land outside the strip's viewport and stay there: the tab is \
             selected, the pane switches, the process attaches — and the one cue that says which \
             tab the user is on is the one thing off screen. Feature 027 made that reachable with \
             a press, because the \"+\" now sits beside the strip and creates the tab it hides."
        );
    }
}
