//! Which sessions the connected windows have in view (feature 039, wire W1.1, W1.2), and which
//! window a click on a notification is sent to (W3.1).
//!
//! Pure, no I/O, and lost when the service stops: a window reports again after every `Welcome`.
//! The service counts an attention event only for a session that [`Views::is_in_view`] denies.

use std::collections::HashMap;

use micold_core::attention::NotificationKind;
use micold_core::protocol::messages::WindowView;
use micold_core::session::SessionId;

use crate::state::ClientId;

/// The last view report of each connection.
#[derive(Debug, Default)]
pub struct Views {
    views: HashMap<ClientId, WindowView>,
    /// The highest sequence granted for each session (W1.4). Kept in memory only.
    granted: HashMap<SessionId, u64>,
    /// The kind of each event noted while it notified and not yet granted, per session (feature
    /// 613, C10–C12). Kept in memory only.
    pending: HashMap<SessionId, Vec<(u64, NotificationKind)>>,
    /// The connections that reported `focused: true`, in the order they last did, most recent
    /// last (FR-012). A connection is listed once, and leaves when it ends.
    focus_order: Vec<ClientId>,
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
        if view.focused {
            self.focus_order.retain(|listed| *listed != client);
            self.focus_order.push(client);
        }
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
        self.focus_order.retain(|listed| *listed != client);
    }

    /// The connections that reported keyboard focus, in the order they last did, most recent last.
    pub fn focus_order(&self) -> &[ClientId] {
        &self.focus_order
    }

    /// The connection a reveal is forwarded to (W3.1, FR-012): `holder`, the one attached to the
    /// session's project, when there is one; else the one that last reported keyboard focus; else
    /// `sender`, the window that was clicked.
    pub fn reveal_target(&self, holder: Option<ClientId>, sender: ClientId) -> ClientId {
        holder
            .or_else(|| self.focus_order.last().copied())
            .unwrap_or(sender)
    }

    /// Forget what was granted and what is pending for `session`: the session was removed, and
    /// its id is not used again (C12).
    pub fn forget_session(&mut self, session: SessionId) {
        self.granted.remove(&session);
    }

    /// Whether any connection reports `session` in view.
    pub fn is_in_view(&self, session: SessionId) -> bool {
        self.views
            .values()
            .any(|view| view.in_view == Some(session))
    }

    /// The kind granted to the claim of `seq` for `session` (W1.4, FR-006a, C11).
    pub fn grant(
        &mut self,
        session: SessionId,
        seq: u64,
        current_seq: u64,
        notify: impl Fn(NotificationKind) -> bool,
    ) -> Option<NotificationKind> {
        let _ = (session, seq, current_seq, &notify);
        todo!()
    }

    /// Note attention event `seq` of `session`, of `kind` (C10).
    pub fn note_event(&mut self, session: SessionId, seq: u64, kind: NotificationKind, notify: bool) {
        let _ = (session, seq, kind, notify);
        todo!()
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

    const KIND: NotificationKind = NotificationKind::LongTaskFinished;

    fn on(_: NotificationKind) -> bool {
        true
    }

    fn off(_: NotificationKind) -> bool {
        false
    }

    /// Note event `seq` of `session` with `KIND` while it notifies: its kind is kept pending.
    fn noted(views: &mut Views, session: SessionId, seq: u64) {
        views.note_event(session, seq, KIND, true);
    }

    /// U54 (FR-006a): the first claim of a sequence is granted, with the kind noted for it.
    #[test]
    fn the_first_claim_of_a_sequence_is_granted() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        assert_eq!(views.grant(session(1), 1, 1, on), Some(KIND));
    }

    /// U55 (FR-006a): the same sequence is not granted twice.
    #[test]
    fn the_same_sequence_is_not_granted_again() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        assert_eq!(views.grant(session(1), 1, 1, on), Some(KIND));
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            None,
            "a second claim of 1 loses"
        );
    }

    /// U56 (FR-001): a sequence above the session's current one is not granted, and does not
    /// use up the grant of the real one.
    #[test]
    fn a_sequence_above_the_current_one_is_not_granted() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        noted(&mut views, session(1), 2);
        assert_eq!(views.grant(session(1), 2, 1, on), None);
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            Some(KIND),
            "the refused claim left the session's sequence 1 ungranted"
        );
    }

    /// U57 (US1-6, FR-003): a later sequence of the same session is granted.
    #[test]
    fn a_later_sequence_of_the_same_session_is_granted() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        noted(&mut views, session(1), 2);
        assert_eq!(views.grant(session(1), 1, 2, on), Some(KIND));
        assert_eq!(views.grant(session(1), 2, 2, on), Some(KIND));
        assert_eq!(
            views.grant(session(1), 1, 2, on),
            None,
            "an earlier sequence stays used"
        );
    }

    /// U58 (FR-009): one session's grant does not use up another's.
    #[test]
    fn a_grant_for_one_session_does_not_use_up_another_s() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        noted(&mut views, session(2), 1);
        assert_eq!(views.grant(session(1), 1, 1, on), Some(KIND));
        assert_eq!(views.grant(session(2), 1, 1, on), Some(KIND));
    }

    /// Review A F4 (039), C12 (613): a removed session's grants and pending kinds are forgotten.
    #[test]
    fn forgetting_a_session_drops_its_grants_and_pending_kinds() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        noted(&mut views, session(1), 2);
        noted(&mut views, session(2), 1);
        assert_eq!(views.grant(session(1), 1, 2, on), Some(KIND));

        views.forget_session(session(1));

        assert_eq!(
            views.grant(session(1), 2, 2, on),
            None,
            "the forgotten session's pending kind is gone"
        );
        noted(&mut views, session(1), 1);
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            Some(KIND),
            "nothing is remembered of what was granted to the forgotten session"
        );
        assert_eq!(
            views.grant(session(2), 1, 1, on),
            Some(KIND),
            "another session's pending kind stands"
        );
    }

    /// U66 (FR-027), C11: with the kind off now, a claim is refused, and the refusal records
    /// nothing.
    #[test]
    fn with_the_kind_off_now_a_claim_is_refused_and_records_nothing() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        assert_eq!(
            views.grant(session(1), 1, 1, off),
            None,
            "nothing is granted while the kind does not notify"
        );
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            Some(KIND),
            "the refused claim did not use the sequence up"
        );
    }

    /// C11: the predicate is asked about the kind noted for the claimed sequence.
    #[test]
    fn the_claim_asks_about_the_kind_noted_for_its_sequence() {
        let mut views = Views::default();
        views.note_event(session(1), 1, NotificationKind::NeedsPermission, true);
        views.note_event(session(1), 2, NotificationKind::TurnFinished, true);
        let only_permission = |k: NotificationKind| k == NotificationKind::NeedsPermission;
        assert_eq!(
            views.grant(session(1), 1, 2, only_permission),
            Some(NotificationKind::NeedsPermission)
        );
        assert_eq!(
            views.grant(session(1), 2, 2, only_permission),
            None,
            "Turn finished is off now"
        );
    }

    /// C10, C11: a claim of a sequence that has no pending kind is refused.
    #[test]
    fn a_claim_of_a_sequence_with_no_pending_kind_is_refused() {
        let mut views = Views::default();
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            None,
            "nothing was noted (a service restart, or noted while off)"
        );
    }

    /// U67 (FR-027, US4-5), C10: an event noted while it does not notify is never granted
    /// afterwards.
    #[test]
    fn an_event_noted_while_it_does_not_notify_is_not_granted_later() {
        let mut views = Views::default();
        views.note_event(session(1), 1, KIND, false);
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            None,
            "the event happened while its kind did not notify"
        );
        noted(&mut views, session(1), 2);
        noted(&mut views, session(2), 1);
        assert_eq!(
            views.grant(session(1), 2, 2, on),
            Some(KIND),
            "the next event of the same session is granted"
        );
        assert_eq!(
            views.grant(session(2), 1, 1, on),
            Some(KIND),
            "another session's event is not used up"
        );
    }

    /// C12: an event recorded as granted drops the pending kinds at or below it.
    #[test]
    fn recording_an_event_as_granted_drops_the_pending_kinds_below_it() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        views.note_event(session(1), 2, KIND, false);
        assert_eq!(
            views.grant(session(1), 1, 2, on),
            None,
            "sequence 1 is at or below one recorded as granted"
        );
        assert!(
            views.pending.get(&session(1)).is_none_or(|p| p.is_empty()),
            "nothing is kept for it"
        );
    }

    /// C12: a grant drops the pending kinds at or below it.
    #[test]
    fn a_grant_drops_the_pending_kinds_at_or_below_it() {
        let mut views = Views::default();
        noted(&mut views, session(1), 1);
        noted(&mut views, session(1), 2);
        noted(&mut views, session(1), 3);
        assert_eq!(views.grant(session(1), 2, 3, on), Some(KIND));
        assert_eq!(
            views.pending.get(&session(1)).map(|p| p.as_slice()),
            Some([(3, KIND)].as_slice()),
            "only the event above the grant waits"
        );
    }

    /// U67: noting an earlier sequence does not bring a later one, already granted, back.
    #[test]
    fn noting_an_earlier_event_does_not_lower_what_was_granted() {
        let mut views = Views::default();
        noted(&mut views, session(1), 2);
        assert_eq!(views.grant(session(1), 2, 2, on), Some(KIND));
        views.note_event(session(1), 1, KIND, false);
        noted(&mut views, session(1), 2);
        assert_eq!(
            views.grant(session(1), 2, 2, on),
            None,
            "sequence 2 stays granted"
        );
    }

    /// Principle II: two sessions' pending kinds never mix.
    #[test]
    fn two_sessions_pending_kinds_do_not_mix() {
        let mut views = Views::default();
        views.note_event(session(1), 1, NotificationKind::NeedsPermission, true);
        views.note_event(session(2), 1, NotificationKind::LongTaskFinished, true);
        assert_eq!(
            views.grant(session(2), 1, 1, on),
            Some(NotificationKind::LongTaskFinished)
        );
        assert_eq!(
            views.grant(session(1), 1, 1, on),
            Some(NotificationKind::NeedsPermission)
        );
    }

    fn focused() -> WindowView {
        WindowView {
            focused: true,
            in_view: None,
        }
    }

    /// U61 (FR-012): the connection that last reported keyboard focus is last.
    #[test]
    fn focus_order_puts_the_connection_that_last_reported_focus_last() {
        let mut views = Views::default();

        views.set_view(FIRST_WINDOW, focused());
        views.set_view(SECOND_WINDOW, focused());
        assert_eq!(views.focus_order(), [FIRST_WINDOW, SECOND_WINDOW]);

        views.set_view(FIRST_WINDOW, viewing(session(1)));
        assert_eq!(
            views.focus_order(),
            [SECOND_WINDOW, FIRST_WINDOW],
            "the first window reported focus again, so it is the most recent, and listed once"
        );
    }

    /// U61: a report without focus does not make a connection the most recent one.
    #[test]
    fn a_report_without_focus_does_not_move_a_connection_in_focus_order() {
        const NEVER_FOCUSED: ClientId = 3;
        let unfocused = WindowView {
            focused: false,
            in_view: None,
        };
        let mut views = Views::default();
        views.set_view(FIRST_WINDOW, focused());
        views.set_view(SECOND_WINDOW, focused());

        views.set_view(FIRST_WINDOW, unfocused);
        views.set_view(NEVER_FOCUSED, unfocused);

        assert_eq!(
            views.focus_order(),
            [FIRST_WINDOW, SECOND_WINDOW],
            "only a report of `focused: true` counts, and a window that never had focus is absent"
        );
    }

    /// U62 (FR-012): a connection that ended is no reveal target.
    #[test]
    fn remove_takes_a_connection_out_of_focus_order() {
        let mut views = Views::default();
        views.set_view(FIRST_WINDOW, focused());
        views.set_view(SECOND_WINDOW, focused());

        views.remove(SECOND_WINDOW);

        assert_eq!(views.focus_order(), [FIRST_WINDOW]);
    }

    /// U63 (FR-012, US3-6): the window that holds the project is the target, whoever has focus.
    #[test]
    fn reveal_target_is_the_connection_that_holds_the_project() {
        const HOLDER: ClientId = 7;
        const SENDER: ClientId = 9;
        let mut views = Views::default();
        views.set_view(FIRST_WINDOW, focused());

        assert_eq!(views.reveal_target(Some(HOLDER), SENDER), HOLDER);
    }

    /// U64 (FR-012): with no holder, the window that last reported focus.
    #[test]
    fn with_no_holder_reveal_target_is_the_last_of_focus_order() {
        const SENDER: ClientId = 9;
        let mut views = Views::default();
        views.set_view(FIRST_WINDOW, focused());
        views.set_view(SECOND_WINDOW, focused());

        assert_eq!(views.reveal_target(None, SENDER), SECOND_WINDOW);
    }

    /// U65 (FR-012): with no holder and no window that reported focus, the sender.
    #[test]
    fn with_no_holder_and_an_empty_focus_order_reveal_target_is_the_sender() {
        const SENDER: ClientId = 9;
        let views = Views::default();

        assert_eq!(views.reveal_target(None, SENDER), SENDER);
    }
}
