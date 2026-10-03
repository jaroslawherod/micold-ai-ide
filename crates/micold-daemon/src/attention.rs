//! Which sessions the connected windows have in view (feature 039, wire W1.1, W1.2).
//!
//! Pure, no I/O, and lost when the service stops: a window reports again after every `Welcome`.
//! The service counts an attention event only for a session that [`Views::is_in_view`] denies.

use std::collections::HashMap;

use micold_core::protocol::messages::WindowView;
use micold_core::session::SessionId;

use crate::state::ClientId;

/// The last view report of each connection.
#[derive(Debug, Default)]
pub struct Views {
    views: HashMap<ClientId, WindowView>,
    /// The highest sequence granted for each session (W1.4). Kept in memory only.
    granted: HashMap<SessionId, u64>,
}

impl Views {
    /// Store `client`'s report in place of its last one.
    ///
    /// A window without keyboard focus has no session in view (FR-016), so a report that says
    /// `focused: false` is stored with `in_view: None` whatever it carried: the rule does not rest
    /// on every client keeping W1.1.
    ///
    /// Returns the session the report brought into view (W2.2, FR-019): the one it names, unless
    /// this connection's last report named it already.
    pub fn set_view(&mut self, client: ClientId, view: WindowView) -> Option<SessionId> {
        let in_view = if view.focused { view.in_view } else { None };
        let before = self
            .views
            .insert(
                client,
                WindowView {
                    focused: view.focused,
                    in_view,
                },
            )
            .and_then(|last| last.in_view);
        in_view.filter(|session| before != Some(*session))
    }

    /// Forget `client`'s report: its connection ended.
    pub fn remove(&mut self, client: ClientId) {
        self.views.remove(&client);
    }

    /// Forget what was granted for `session`: the session was removed, and its id is not used
    /// again.
    pub fn forget_session(&mut self, session: SessionId) {
        self.granted.remove(&session);
    }

    /// Whether any connection reports `session` in view.
    pub fn is_in_view(&self, session: SessionId) -> bool {
        self.views
            .values()
            .any(|view| view.in_view == Some(session))
    }

    /// Whether the claim of `seq` for `session` is granted (W1.4, FR-006a): only when `seq` is
    /// above every sequence already granted for the session and not above `current_seq`.
    pub fn grant(&mut self, session: SessionId, seq: u64, current_seq: u64) -> bool {
        if seq > current_seq || seq <= self.granted.get(&session).copied().unwrap_or(0) {
            return false;
        }
        self.granted.insert(session, seq);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIRST_WINDOW: ClientId = 1;
    const SECOND_WINDOW: ClientId = 2;

    fn session(n: u128) -> SessionId {
        SessionId::from_uuid(uuid::Uuid::from_u128(n))
    }

    fn viewing(session: SessionId) -> WindowView {
        WindowView {
            focused: true,
            in_view: Some(session),
        }
    }

    #[test]
    fn a_second_report_from_a_connection_replaces_its_first() {
        let (a, b) = (session(1), session(2));
        let mut views = Views::default();

        views.set_view(FIRST_WINDOW, viewing(a));
        views.set_view(FIRST_WINDOW, viewing(b));

        assert!(
            views.is_in_view(b),
            "the window's last report names the session it has in view"
        );
        assert!(
            !views.is_in_view(a),
            "one window has one session in view: its earlier report is replaced, not kept"
        );
    }

    /// U59 (FR-019): the report tells the service which session came into view.
    #[test]
    fn a_report_returns_the_session_that_came_into_view() {
        let (a, b) = (session(1), session(2));
        let mut views = Views::default();

        assert_eq!(
            views.set_view(FIRST_WINDOW, viewing(a)),
            Some(a),
            "the window's first report brings its session into view"
        );
        assert_eq!(
            views.set_view(FIRST_WINDOW, viewing(b)),
            Some(b),
            "a report naming another session brings that one into view"
        );
    }

    /// U60 (FR-019): a report that brings nothing into view returns nothing.
    #[test]
    fn a_report_naming_the_same_session_or_none_returns_nothing() {
        let a = session(1);
        let mut views = Views::default();
        views.set_view(FIRST_WINDOW, viewing(a));

        assert_eq!(
            views.set_view(FIRST_WINDOW, viewing(a)),
            None,
            "the session was in this window's view already"
        );
        assert_eq!(
            views.set_view(
                FIRST_WINDOW,
                WindowView {
                    focused: true,
                    in_view: None,
                },
            ),
            None,
            "a report with no session brings none into view"
        );
        assert_eq!(
            views.set_view(
                FIRST_WINDOW,
                WindowView {
                    focused: false,
                    in_view: Some(a),
                },
            ),
            None,
            "a window without keyboard focus has no session in view"
        );
    }

    #[test]
    fn an_unfocused_report_is_stored_with_nothing_in_view() {
        let a = session(1);
        let mut views = Views::default();

        views.set_view(
            FIRST_WINDOW,
            WindowView {
                focused: false,
                in_view: Some(a),
            },
        );

        assert!(
            !views.is_in_view(a),
            "a window without keyboard focus has no session in view, whatever its report carried"
        );
    }

    #[test]
    fn a_session_is_in_view_while_any_report_names_it() {
        let (a, b, nobody_views) = (session(1), session(2), session(3));
        let mut views = Views::default();

        views.set_view(FIRST_WINDOW, viewing(a));
        views.set_view(SECOND_WINDOW, viewing(b));

        assert!(views.is_in_view(a), "the first window has it in view");
        assert!(views.is_in_view(b), "the second window has it in view");
        assert!(
            !views.is_in_view(nobody_views),
            "no report names this session"
        );

        views.set_view(
            FIRST_WINDOW,
            WindowView {
                focused: true,
                in_view: None,
            },
        );
        assert!(
            !views.is_in_view(a),
            "the only window that had it in view reports nothing in view now"
        );
        assert!(views.is_in_view(b), "the other window's report stands");
    }

    #[test]
    fn a_removed_connection_s_report_is_forgotten() {
        let a = session(1);
        let mut views = Views::default();
        views.set_view(FIRST_WINDOW, viewing(a));
        views.set_view(SECOND_WINDOW, viewing(a));

        views.remove(FIRST_WINDOW);
        assert!(
            views.is_in_view(a),
            "the second window still has the session in view"
        );

        views.remove(SECOND_WINDOW);
        assert!(
            !views.is_in_view(a),
            "no connection is left that reported the session in view"
        );
    }

    /// U54 (FR-006a): the first claim of a sequence is granted.
    #[test]
    fn the_first_claim_of_a_sequence_is_granted() {
        let mut views = Views::default();
        assert!(views.grant(session(1), 1, 1));
    }

    /// U55 (FR-006a): the same sequence is not granted twice.
    #[test]
    fn the_same_sequence_is_not_granted_again() {
        let mut views = Views::default();
        assert!(views.grant(session(1), 1, 1));
        assert!(!views.grant(session(1), 1, 1), "a second claim of 1 loses");
    }

    /// U56 (FR-001): a sequence above the session's current one is not granted, and does not
    /// use up the grant of the real one.
    #[test]
    fn a_sequence_above_the_current_one_is_not_granted() {
        let mut views = Views::default();
        assert!(!views.grant(session(1), 2, 1));
        assert!(
            views.grant(session(1), 1, 1),
            "the refused claim left the session's sequence 1 ungranted"
        );
    }

    /// U57 (US1-6, FR-003): a later sequence of the same session is granted.
    #[test]
    fn a_later_sequence_of_the_same_session_is_granted() {
        let mut views = Views::default();
        assert!(views.grant(session(1), 1, 2));
        assert!(views.grant(session(1), 2, 2));
        assert!(
            !views.grant(session(1), 1, 2),
            "an earlier sequence stays used"
        );
    }

    /// U58 (FR-009): one session's grant does not use up another's.
    #[test]
    fn a_grant_for_one_session_does_not_use_up_another_s() {
        let mut views = Views::default();
        assert!(views.grant(session(1), 1, 1));
        assert!(views.grant(session(2), 1, 1));
    }

    /// Review A F4: a removed session's grant is forgotten, so nothing is kept for a session
    /// that no longer exists.
    #[test]
    fn after_a_session_is_forgotten_the_same_sequence_is_granted_again() {
        let mut views = Views::default();
        assert!(views.grant(session(1), 1, 1));
        assert!(views.grant(session(2), 1, 1));

        views.forget_session(session(1));

        assert!(
            views.grant(session(1), 1, 1),
            "nothing is remembered of the forgotten session"
        );
        assert!(
            !views.grant(session(2), 1, 1),
            "another session's grant stands"
        );
    }
}
