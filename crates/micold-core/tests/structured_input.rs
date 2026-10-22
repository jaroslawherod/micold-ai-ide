//! Feature 941 — a structured session's input as a scenario table (SC-004, data-model
//! *SessionInput*): where a sent message goes, when the queue's head goes out, and the hold. After
//! every step of every row the invariant holds: a held queue is never empty.

use micold_core::structured::attachment::{admit, ImageAttachment};
use micold_core::structured::{
    Message, QueueEntryId, QueueRefusal, Release, SessionInput, Submit, TurnId, TurnState,
    UserMessageRefusal,
};

const CLIENT: u64 = 7;

fn text(text: &str) -> Message {
    Message {
        text: text.into(),
        images: Vec::new(),
    }
}

fn picture(name: &str) -> ImageAttachment {
    admit(&[], name, b"\x89PNG\r\n\x1a\n-".to_vec()).expect("a picture")
}

/// The input and the process's turn, checked after every step.
struct Session {
    input: SessionInput,
    turn: TurnState,
}

impl Session {
    fn idle() -> Self {
        Self {
            input: SessionInput::new(),
            turn: TurnState::default(),
        }
    }

    fn check(&self) {
        assert!(
            !self.input.is_held() || !self.input.is_empty(),
            "a held queue is never empty: {:?}",
            self.input
        );
    }

    /// Send `message` as the daemon does: submit, open the turn when delivered, then pump.
    fn send(&mut self, message: Message) -> Result<Submit, UserMessageRefusal> {
        let busy = !self.turn.is_idle();
        let outcome = self.input.submit(message, CLIENT, busy, true);
        if let Ok(Submit::Deliver(turn, _)) = &outcome {
            self.turn.accept(*turn, true).expect("idle accepts");
        }
        self.check();
        outcome
    }

    /// The pump that ends every lock section: the delivered text, if any.
    fn pump(&mut self) -> Option<String> {
        let (turn, head) = self.input.pump(&self.turn)?;
        self.turn
            .accept(turn, true)
            .expect("pump delivers only when idle");
        self.check();
        Some(head.message.text)
    }

    fn end_turn(&mut self) {
        self.turn.on_event(&result());
        self.check();
    }

    fn queued(&self) -> Vec<String> {
        self.input
            .queued()
            .map(|q| q.message.text.clone())
            .collect()
    }

    fn ids(&self) -> Vec<QueueEntryId> {
        self.input.queued().map(|q| q.id).collect()
    }
}

fn result() -> micold_core::structured::StructuredEvent {
    micold_core::structured::parse_line(r#"{"type":"result","subtype":"success","is_error":false}"#)
}

/// A session with one turn open and `queued` waiting behind it.
fn busy_with(queued: &[&str]) -> Session {
    let mut session = Session::idle();
    assert!(matches!(
        session.send(text("first")),
        Ok(Submit::Deliver(..))
    ));
    for message in queued {
        assert!(matches!(session.send(text(message)), Ok(Submit::Queued(_))));
    }
    session
}

#[test]
fn a_blank_message_is_refused_and_the_turn_stays_idle() {
    // #938's turn test, moved here with the blank check (plan *Superseded assertions*).
    for blank in ["", "  \n\t"] {
        let mut session = Session::idle();
        assert_eq!(session.send(text(blank)), Err(UserMessageRefusal::Blank));
        assert!(session.turn.is_idle());
        assert!(session.input.is_empty());
    }
}

#[test]
fn a_message_of_pictures_only_is_delivered() {
    let mut session = Session::idle();
    let message = Message {
        text: " ".into(),
        images: vec![picture("a.png")],
    };
    assert!(matches!(session.send(message), Ok(Submit::Deliver(_, m)) if m.images.len() == 1));
}

#[test]
fn a_session_that_is_not_running_refuses_after_the_blank_check() {
    let mut input = SessionInput::new();
    assert_eq!(
        input.submit(text(""), CLIENT, false, false),
        Err(UserMessageRefusal::Blank)
    );
    assert_eq!(
        input.submit(text("hi"), CLIENT, false, false),
        Err(UserMessageRefusal::NotRunning)
    );
    assert!(input.is_empty());
}

#[test]
fn a_message_sent_while_busy_is_queued_and_nothing_is_delivered() {
    let mut session = busy_with(&["second", "third"]);
    assert_eq!(session.queued(), ["second", "third"]);
    assert_eq!(session.pump(), None, "never with a turn open (FR-019)");
}

#[test]
fn an_idle_session_with_an_empty_queue_delivers_at_once() {
    let mut session = Session::idle();
    let outcome = session.send(text("hi"));
    assert_eq!(outcome, Ok(Submit::Deliver(TurnId(1), text("hi"))));
    assert!(!session.turn.is_idle());
}

#[test]
fn idle_behind_a_queue_not_on_hold_queues_and_the_pump_delivers_the_head() {
    let mut session = busy_with(&["second"]);
    session.end_turn();
    // Before the driver's pump, a new message arrives (FR-032 boundary).
    assert!(matches!(session.send(text("third")), Ok(Submit::Queued(_))));
    assert_eq!(session.pump().as_deref(), Some("second"));
    assert_eq!(session.queued(), ["third"]);
}

#[test]
fn one_turn_takes_one_queued_message_in_order() {
    let mut session = busy_with(&["a", "b", "c"]);
    let mut delivered = Vec::new();
    for _ in 0..3 {
        assert_eq!(session.pump(), None);
        session.end_turn();
        delivered.push(session.pump().expect("the head"));
        assert_eq!(session.pump(), None, "the turn it opened holds the next");
    }
    assert_eq!(delivered, ["a", "b", "c"]);
    assert!(session.input.is_empty());
}

#[test]
fn a_result_never_holds_the_queue() {
    let mut session = busy_with(&["a"]);
    session.end_turn();
    assert!(!session.input.is_held());
    assert_eq!(session.pump().as_deref(), Some("a"));
}

#[test]
fn an_interrupt_holds_a_queue_and_nothing_goes_out_until_released() {
    let mut session = busy_with(&["a", "b"]);
    session.input.on_interrupt();
    assert!(session.input.is_held());
    session.end_turn();
    assert_eq!(session.pump(), None, "held (Q2)");
    assert_eq!(session.input.release(), Release::Released);
    assert_eq!(session.pump().as_deref(), Some("a"));
}

#[test]
fn hold_is_set_only_on_a_non_empty_queue() {
    let mut input = SessionInput::new();
    input.on_interrupt();
    assert!(!input.is_held());
    input.on_process_end();
    assert!(!input.is_held());
    input.on_process_start();
    assert!(!input.is_held());
    assert_eq!(input.release(), Release::NothingHeld);
}

#[test]
fn a_process_end_and_a_restart_hold_the_queue() {
    for step in [SessionInput::on_process_end, SessionInput::on_process_start] {
        let mut session = busy_with(&["a"]);
        step(&mut session.input);
        session.turn = TurnState::default();
        session.check();
        assert_eq!(session.pump(), None, "held (FR-022, Q1)");
        assert_eq!(session.input.release(), Release::Released);
        assert_eq!(session.pump().as_deref(), Some("a"));
    }
}

#[test]
fn a_message_sent_while_idle_with_a_held_queue_goes_out_at_once_and_the_queue_stays() {
    let mut session = busy_with(&["a", "b"]);
    session.input.on_interrupt();
    session.end_turn();
    let before = session.input.snapshot();
    assert!(matches!(session.send(text("now")), Ok(Submit::Deliver(..))));
    assert_eq!(session.input.snapshot(), before, "held and unchanged (Q5)");
    assert_eq!(session.pump(), None);
}

#[test]
fn two_releases_with_their_pumps_deliver_once() {
    let mut session = busy_with(&["a", "b"]);
    session.input.on_interrupt();
    session.end_turn();
    assert_eq!(session.input.release(), Release::Released);
    let first = session.pump();
    assert_eq!(session.input.release(), Release::NothingHeld);
    let second = session.pump();
    assert_eq!(first.as_deref(), Some("a"));
    assert_eq!(second, None);
    assert_eq!(session.queued(), ["b"]);
}

#[test]
fn a_release_while_busy_waits_for_the_result() {
    let mut session = busy_with(&["a"]);
    session.input.on_interrupt();
    assert_eq!(session.input.release(), Release::Released);
    assert_eq!(session.pump(), None);
    session.end_turn();
    assert_eq!(session.pump().as_deref(), Some("a"));
}

#[test]
fn an_edit_replaces_the_text_keeps_chosen_pictures_and_the_position() {
    let mut session = Session::idle();
    session.send(text("first")).expect("delivered");
    session.send(text("a")).expect("queued");
    let with_pictures = Message {
        text: "b".into(),
        images: vec![picture("0.png"), picture("1.png"), picture("2.png")],
    };
    session.send(with_pictures).expect("queued");
    session.send(text("c")).expect("queued");
    let id = session.ids()[1];
    session
        .input
        .edit(id, "b, edited".into(), &[2, 0])
        .expect("edited");
    assert_eq!(session.queued(), ["a", "b, edited", "c"]);
    let names: Vec<_> = session
        .input
        .queued()
        .nth(1)
        .expect("b")
        .message
        .images
        .iter()
        .map(|image| image.name().to_owned())
        .collect();
    assert_eq!(names, ["0.png", "2.png"]);
    session.check();
}

#[test]
fn an_edit_leaving_nothing_to_send_is_refused() {
    let mut session = busy_with(&["a"]);
    let id = session.ids()[0];
    assert_eq!(
        session.input.edit(id, "  ".into(), &[]),
        Err(QueueRefusal::Blank)
    );
    assert_eq!(session.queued(), ["a"]);
}

#[test]
fn edits_of_an_entry_no_longer_queued_are_gone() {
    let mut session = busy_with(&["a"]);
    let id = session.ids()[0];
    session.end_turn();
    session.pump().expect("delivered");
    assert_eq!(
        session.input.edit(id, "x".into(), &[]),
        Err(QueueRefusal::Gone)
    );
    assert_eq!(session.input.remove(id), Err(QueueRefusal::Gone));
    assert_eq!(session.input.take(id), Err(QueueRefusal::Gone));
    assert_eq!(
        QueueRefusal::Gone.to_string(),
        "That message was already sent or removed"
    );
}

#[test]
fn a_removed_entry_is_never_delivered() {
    let mut session = busy_with(&["a", "b"]);
    let id = session.ids()[0];
    session.input.remove(id).expect("removed");
    session.end_turn();
    assert_eq!(session.pump().as_deref(), Some("b"));
}

#[test]
fn take_returns_the_text_and_pictures() {
    let mut session = busy_with(&[]);
    let message = Message {
        text: "with a picture".into(),
        images: vec![picture("a.png")],
    };
    session.send(message.clone()).expect("queued");
    let id = session.ids()[0];
    assert_eq!(session.input.take(id), Ok(message));
    assert!(session.input.is_empty());
}

#[test]
fn removing_or_taking_the_last_entry_clears_the_hold() {
    let mut session = busy_with(&["a"]);
    session.input.on_interrupt();
    session.input.remove(session.ids()[0]).expect("removed");
    assert!(!session.input.is_held());
    session.check();

    let mut session = busy_with(&["a", "b"]);
    session.input.on_interrupt();
    session.input.take(session.ids()[0]).expect("taken");
    assert!(session.input.is_held(), "one is left");
    session.input.take(session.ids()[0]).expect("taken");
    assert!(!session.input.is_held());
    session.check();
}

#[test]
fn a_failed_delivery_goes_back_to_the_head_on_hold() {
    let mut session = busy_with(&["a", "b"]);
    session.end_turn();
    let (_, head) = session.input.pump(&session.turn).expect("a");
    session.input.put_back(head);
    session.check();
    assert!(session.input.is_held());
    assert_eq!(session.queued(), ["a", "b"]);
}

#[test]
fn the_snapshot_carries_names_and_sizes_but_no_bytes() {
    let mut session = busy_with(&[]);
    session
        .send(Message {
            text: "look".into(),
            images: vec![picture("a.png")],
        })
        .expect("queued");
    let (queue, hold) = session.input.snapshot();
    assert!(!hold);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].text, "look");
    assert_eq!(queue[0].attachments[0].name, "a.png");
    assert_eq!(queue[0].attachments[0].size, 9);
}

#[test]
fn entry_and_turn_ids_increase() {
    let mut session = busy_with(&["a", "b"]);
    let ids = session.ids();
    assert!(ids[0] < ids[1]);
    session.end_turn();
    let (turn_a, _) = session.input.pump(&session.turn).expect("a");
    assert!(turn_a > TurnId(1), "after the first turn's id");
    assert!(session.input.next_turn() > turn_a);
}

#[test]
fn two_sessions_never_share_a_queue() {
    let mut one = busy_with(&["a"]);
    let other = Session::idle();
    one.input.on_interrupt();
    assert!(other.input.is_empty() && !other.input.is_held());
    assert_eq!(other.input.snapshot(), (vec![], false));
}

/// R9: a saved queue is restored in order with fresh ids and on hold; an empty one is not held.
#[test]
fn a_restored_queue_is_held_with_fresh_ids() {
    let restored = SessionInput::restored(vec![text("a"), text("b")]);
    assert!(restored.is_held(), "restored on hold (Q1)");
    let queued: Vec<_> = restored.queued().collect();
    assert_eq!(queued[0].message, text("a"));
    assert_eq!(queued[1].message, text("b"));
    assert!(queued[0].id < queued[1].id, "fresh ids, in order");
    let empty = SessionInput::restored(Vec::new());
    assert!(!empty.is_held() && empty.is_empty());
}

/// R9: the revision rises with every change of the entries and never with the hold alone.
#[test]
fn the_revision_follows_the_entries_not_the_hold() {
    let mut session = busy_with(&["a", "b"]);
    let ids = session.ids();
    let mut last = session.input.revision();
    fn changed(input: &SessionInput, last: &mut u64, what: &str) {
        assert!(input.revision() > *last, "{what} changes the revision");
        *last = input.revision();
    }
    session.input.edit(ids[0], "a2".into(), &[]).expect("edit");
    changed(&session.input, &mut last, "an edit");
    session.input.on_interrupt();
    session.input.release();
    assert_eq!(
        session.input.revision(),
        last,
        "the hold alone changes nothing"
    );
    session.end_turn();
    let (_, head) = session.input.pump(&session.turn).expect("the head");
    changed(&session.input, &mut last, "a delivery");
    session.input.put_back(head);
    changed(&session.input, &mut last, "a put back");
    session.input.remove(ids[1]).expect("remove");
    changed(&session.input, &mut last, "a removal");
    session.input.take(ids[0]).expect("take");
    changed(&session.input, &mut last, "a take");
}
