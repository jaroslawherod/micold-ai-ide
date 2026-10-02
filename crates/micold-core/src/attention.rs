//! Which session a window has in view (feature 039, FR-002, FR-016, spec Terms).
//!
//! Render-free. The client fills [`ViewFacts`] from its own state and reports [`in_view`]'s answer
//! to the session service, which counts an attention event only for a session that no window has
//! in view.

use crate::session::SessionId;

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
}
