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
}

impl Views {
    /// Store `client`'s report in place of its last one.
    ///
    /// A window without keyboard focus has no session in view (FR-016), so a report that says
    /// `focused: false` is stored with `in_view: None` whatever it carried: the rule does not rest
    /// on every client keeping W1.1.
    pub fn set_view(&mut self, client: ClientId, view: WindowView) {
        let in_view = if view.focused { view.in_view } else { None };
        self.views.insert(
            client,
            WindowView {
                focused: view.focused,
                in_view,
            },
        );
    }

    /// Forget `client`'s report: its connection ended.
    pub fn remove(&mut self, client: ClientId) {
        self.views.remove(&client);
    }

    /// Whether any connection reports `session` in view.
    pub fn is_in_view(&self, session: SessionId) -> bool {
        self.views
            .values()
            .any(|view| view.in_view == Some(session))
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
}
