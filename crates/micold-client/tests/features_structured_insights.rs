//! T034 (feature 944, data-model.md *View state*, contracts/insights-model.md *Client seam*): each
//! window keeps, per structured session, the session's insights, the subagent threads it opened
//! (as nested transcripts) and whether the background-task list is open, applied after #939's
//! transcript of the same message.

use micold_client::app::{Message, State as App};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::structured_insights::{InsightsMsg, State};
use micold_core::session::SessionId;
use micold_core::structured::insights::{SessionInsights, SubagentState, TaskState};
use micold_core::structured::transcript::{Entry, EntryKey, EntryKind, Transcript};
use micold_core::structured::{SequencedEvent, StructuredEvent, MAX_EVENTS};
use serde_json::json;
use std::collections::HashMap;

#[path = "../../micold-core/tests/support/stream_json.rs"]
mod stream_json;
use stream_json::{sequenced, tool_use, user_text, with_parent, RECORDED};

/// One window's two features for the same session, fed as the app feeds them: the transcript
/// first, then the insights with that transcript.
#[derive(Default)]
struct Window {
    main: HashMap<SessionId, Transcript>,
    insights: State,
}

impl Window {
    fn apply(&mut self, s: SessionId, omitted: u64, events: &[SequencedEvent], live: bool) {
        let main = self.main.entry(s).or_default();
        main.apply(omitted, events, live);
        self.insights.apply(s, omitted, events, live, main);
    }

    fn main(&self, s: SessionId) -> &Transcript {
        &self.main[&s]
    }

    fn toggle_thread(&mut self, s: SessionId, parent: &str) {
        let main = self.main.entry(s).or_default();
        self.insights.toggle_thread(s, parent, main);
    }
}

/// The core's recording `name`, parsed and numbered from `0` as a history holds it.
fn fixture(name: &str) -> Vec<SequencedEvent> {
    let path = format!(
        "{}/../micold-core/tests/fixtures/stream-json/{name}.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} reads: {e}"));
    let mut splitter = micold_core::structured::LineSplitter::default();
    let mut lines = splitter.push(&bytes);
    lines.extend(splitter.finish());
    sequenced(
        lines
            .iter()
            .map(|line| micold_core::structured::parse_line(line)),
    )
}

/// `events` numbered from `first`.
fn seq(first: u64, events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    (first..)
        .zip(events)
        .map(|(seq, event)| SequencedEvent { seq, event })
        .collect()
}

fn sub_text(text: &str, parent: &str) -> StructuredEvent {
    with_parent(user_text(text), parent)
}

/// The nested transcript of `parent` built from the main transcript's held events (research R7).
fn nested_of(main: &Transcript, parent: &str) -> Transcript {
    let events: Vec<SequencedEvent> = main.subagent_events(parent).cloned().collect();
    let mut nested = Transcript::nested(parent);
    nested.apply(0, &events, false);
    nested
}

fn entries(t: &Transcript) -> Vec<Entry> {
    t.entries().cloned().collect()
}

/// The thread `parent` of `s` holds what a fresh nested transcript of its events holds.
fn assert_thread_is_nested(w: &Window, s: SessionId, parent: &str) {
    let thread = w.insights.thread(s, parent).expect("the thread is open");
    assert_eq!(
        entries(thread),
        entries(&nested_of(w.main(s), parent)),
        "thread {parent}"
    );
}

const SUBAGENT_PARENT: &str = "toolu_016Evhgage4WoGiCC3qAXbiy";

/// The tool call `p` and one message of its subagent, then one of a second subagent `r`.
fn two_threads_events() -> Vec<SequencedEvent> {
    seq(
        0,
        [
            tool_use("p", "Agent", "{}"),
            sub_text("p says one", "p"),
            tool_use("r", "Agent", "{}"),
            sub_text("r says one", "r"),
        ],
    )
}

fn two_threads() -> (Window, SessionId) {
    let (mut w, s) = (Window::default(), SessionId::new());
    w.apply(s, 0, &two_threads_events(), false);
    (w, s)
}

// ---- the insights ----

#[test]
fn the_insights_are_the_cores_for_every_recording() {
    for name in RECORDED {
        let events = fixture(name);
        let (mut w, s) = (Window::default(), SessionId::new());
        w.apply(s, 0, &events, false);
        let mut core = SessionInsights::new();
        core.apply(0, &events, false);
        assert_eq!(w.insights.insights(s), Some(&core), "{name}");
    }
}

#[test]
fn live_messages_build_the_same_insights_as_one_history() {
    let events = fixture("background-subagent");
    let (mut w, s) = (Window::default(), SessionId::new());
    w.apply(s, 0, &events[..1], false);
    for event in &events[1..] {
        w.apply(s, 0, std::slice::from_ref(event), true);
    }
    let mut core = SessionInsights::new();
    core.apply(0, &events, false);
    assert_eq!(w.insights.insights(s), Some(&core));
}

#[test]
fn a_turns_figures_are_found_by_its_footers_key() {
    let (mut w, s) = (Window::default(), SessionId::new());
    w.apply(s, 0, &fixture("text-turn"), false);
    let footer = w
        .main(s)
        .entries()
        .find(|e| matches!(e.kind(), EntryKind::Footer(_)))
        .expect("text-turn ends with a footer")
        .key()
        .clone();
    let EntryKey::At { seq, .. } = footer else {
        panic!("a footer is keyed by its event");
    };
    let turn = w.insights.turn(s, &footer).expect("the turn's figures");
    assert_eq!(Some(turn), w.insights.insights(s).and_then(|i| i.turn(seq)));
    assert!(w.insights.turn(s, &EntryKey::Tool("x".into())).is_none());
    assert!(w.insights.turn(SessionId::new(), &footer).is_none());
}

// ---- threads ----

#[test]
fn a_thread_starts_folded_and_toggle_opens_it_as_its_nested_transcript() {
    let (mut w, s) = (Window::default(), SessionId::new());
    w.apply(s, 0, &fixture("subagent-turn"), false);
    assert!(w.insights.thread(s, SUBAGENT_PARENT).is_none(), "folded");
    w.toggle_thread(s, SUBAGENT_PARENT);
    assert_thread_is_nested(&w, s, SUBAGENT_PARENT);
    assert!(
        !entries(w.insights.thread(s, SUBAGENT_PARENT).unwrap()).is_empty(),
        "the subagent's messages"
    );
    w.toggle_thread(s, SUBAGENT_PARENT);
    assert!(
        w.insights.thread(s, SUBAGENT_PARENT).is_none(),
        "folded again"
    );
}

#[test]
fn every_thread_of_a_nested_recording_opens_at_any_depth() {
    let (mut w, s) = (Window::default(), SessionId::new());
    w.apply(s, 0, &fixture("nested-subagent"), false);
    let parents: Vec<String> = {
        let insights = w.insights.insights(s).unwrap();
        let mut all: Vec<String> = insights
            .threads_owned_by(&micold_core::structured::insights::Owner::Main, w.main(s))
            .into_iter()
            .map(|t| t.parent.clone())
            .collect();
        for parent in all.clone() {
            all.extend(
                insights
                    .threads_owned_by(
                        &micold_core::structured::insights::Owner::Thread(parent),
                        w.main(s),
                    )
                    .into_iter()
                    .map(|t| t.parent.clone()),
            );
        }
        all
    };
    assert_eq!(parents.len(), 2, "the relay's thread and the inner one");
    for parent in &parents {
        w.toggle_thread(s, parent);
        assert_thread_is_nested(&w, s, parent);
    }
}

#[test]
fn a_thread_that_is_not_held_does_not_open() {
    let (mut w, s) = two_threads();
    w.toggle_thread(s, "nobody");
    assert!(w.insights.thread(s, "nobody").is_none());
    w.toggle_thread(SessionId::new(), "p");
    assert!(w.insights.thread(s, "p").is_none());
}

#[test]
fn live_events_reach_an_open_thread_and_not_a_folded_one() {
    let (mut w, s) = two_threads();
    w.toggle_thread(s, "p");
    let before = entries(w.insights.thread(s, "p").unwrap()).len();
    w.apply(
        s,
        0,
        &seq(
            4,
            [sub_text("p says two", "p"), sub_text("r says two", "r")],
        ),
        true,
    );
    assert_eq!(
        entries(w.insights.thread(s, "p").unwrap()).len(),
        before + 1
    );
    assert_thread_is_nested(&w, s, "p");
    assert!(w.insights.thread(s, "r").is_none(), "r stays folded");
    w.toggle_thread(s, "r");
    assert_eq!(
        entries(w.insights.thread(s, "r").unwrap()).len(),
        2,
        "opened later, it holds every held event of r"
    );
}

#[test]
fn an_open_thread_is_rebuilt_after_the_main_transcript_rebuilds() {
    let (mut w, s) = two_threads();
    w.toggle_thread(s, "p");
    let rebuilds = w.main(s).rebuilds();
    let history = seq(
        0,
        [
            tool_use("p", "Agent", "{}"),
            sub_text("p again", "p"),
            sub_text("and again", "p"),
        ],
    );
    w.apply(s, 0, &history, false);
    assert_eq!(w.main(s).rebuilds(), rebuilds + 1);
    assert_thread_is_nested(&w, s, "p");
    assert_eq!(entries(w.insights.thread(s, "p").unwrap()).len(), 2);
}

#[test]
fn an_open_thread_is_rebuilt_after_the_bound_drops_its_events() {
    let (mut w, s) = two_threads();
    w.toggle_thread(s, "p");
    let fillers = seq(4, (0..MAX_EVENTS).map(|_| user_text("filler")));
    w.apply(s, 0, &fillers, true);
    assert!(w.main(s).omitted_notice().is_some());
    assert_thread_is_nested(&w, s, "p");
    assert!(
        entries(w.insights.thread(s, "p").unwrap()).is_empty(),
        "its events left"
    );
}

#[test]
fn a_history_split_over_messages_keeps_an_open_thread_whole() {
    let (mut w, s) = two_threads();
    w.toggle_thread(s, "p");
    // A history message continuing the one before (#938 wire.md): no rebuild.
    w.apply(s, 0, &seq(4, [sub_text("p says two", "p")]), false);
    assert_eq!(entries(w.insights.thread(s, "p").unwrap()).len(), 2);
    assert_thread_is_nested(&w, s, "p");
}

#[test]
fn a_thread_without_a_running_cli_shows_as_ended() {
    let (w, s) = two_threads();
    let view = |running| w.insights.thread_view(s, "p", None, running).unwrap().state;
    assert_eq!(view(true), SubagentState::Running);
    assert_eq!(view(false), SubagentState::EndedWithoutResult);
    assert!(w.insights.thread_view(s, "nobody", None, true).is_none());
}

// ---- background tasks ----

fn task_started(task_id: &str) -> StructuredEvent {
    micold_core::structured::parse_line(
        &json!({
            "type": "system", "subtype": "task_started", "session_id": "<SESSION_ID>",
            "task_id": task_id, "tool_use_id": "b", "description": "sleep",
            "task_type": "local_bash", "is_backgrounded": true,
        })
        .to_string(),
    )
}

#[test]
fn a_running_background_task_without_a_running_cli_shows_as_ended() {
    let (mut w, s) = (Window::default(), SessionId::new());
    w.apply(s, 0, &seq(0, [task_started("t1")]), false);
    assert_eq!(w.insights.running_background(s, true), 1);
    assert_eq!(w.insights.running_background(s, false), 0);
    let states: Vec<TaskState> = w
        .insights
        .insights(s)
        .unwrap()
        .background_tasks()
        .map(|t| t.display_state(false))
        .collect();
    assert_eq!(states, [TaskState::EndedWithoutReport]);
    assert_eq!(w.insights.running_background(SessionId::new(), true), 0);
}

#[test]
fn the_background_task_list_starts_folded_and_toggles() {
    let (mut w, s) = two_threads();
    assert!(!w.insights.tasks_open(s));
    w.insights.toggle_tasks(s);
    assert!(w.insights.tasks_open(s));
    w.apply(s, 0, &seq(4, [user_text("more")]), true);
    assert!(w.insights.tasks_open(s), "new events keep it open");
    w.insights.toggle_tasks(s);
    assert!(!w.insights.tasks_open(s));
    w.insights.toggle_tasks(SessionId::new());
    assert!(
        !w.insights.tasks_open(SessionId::new()),
        "no session, no fold"
    );
}

// ---- windows, sessions, forget ----

#[test]
fn two_windows_hold_the_same_insights_and_their_own_folds() {
    let (mut a, s) = two_threads();
    let mut b = Window::default();
    b.apply(s, 0, &two_threads_events(), false);
    a.toggle_thread(s, "p");
    a.insights.toggle_tasks(s);
    let more = seq(4, [sub_text("p says two", "p")]);
    a.apply(s, 0, &more, true);
    b.apply(s, 0, &more, true);
    assert_eq!(a.insights.insights(s), b.insights.insights(s));
    assert!(a.insights.thread(s, "p").is_some());
    assert!(
        b.insights.thread(s, "p").is_none(),
        "FR-019: the other window stays folded"
    );
    assert!(!b.insights.tasks_open(s));
}

#[test]
fn two_sessions_share_no_insights_threads_or_fold() {
    let (mut w, s) = two_threads();
    let t = SessionId::new();
    w.apply(t, 0, &fixture("text-turn"), false);
    w.toggle_thread(s, "p");
    w.insights.toggle_tasks(s);
    assert!(w.insights.thread(t, "p").is_none());
    assert!(!w.insights.tasks_open(t));
    assert_ne!(w.insights.insights(s), w.insights.insights(t));
    let mut alone = SessionInsights::new();
    alone.apply(0, &fixture("text-turn"), false);
    assert_eq!(w.insights.insights(t), Some(&alone));
}

#[test]
fn forgetting_a_session_drops_its_insights_threads_and_fold() {
    let (mut w, s) = two_threads();
    w.toggle_thread(s, "p");
    w.insights.toggle_tasks(s);
    w.insights.forget(s);
    assert!(w.insights.insights(s).is_none());
    assert!(w.insights.thread(s, "p").is_none());
    assert!(!w.insights.tasks_open(s));
}

// ---- T036: the app's routing and messages ----

fn app_with_threads() -> (App, SessionId) {
    let (mut app, s) = (App::default(), SessionId::new());
    let events = seq(
        0,
        [tool_use("p", "Agent", "{}"), sub_text("p says one", "p")],
    );
    app.structured_events(s, 0, &events, false);
    (app, s)
}

#[test]
fn structured_events_reach_the_insights_after_the_transcript() {
    let (app, s) = app_with_threads();
    assert!(app.structured_transcript.transcript(s).is_some());
    assert!(app
        .structured_insights
        .insights(s)
        .is_some_and(|i| i.thread("p").is_some()));
}

#[test]
fn the_session_family_carries_the_insights_messages() {
    let (mut app, s) = app_with_threads();
    let send = |app: &mut App, msg| app.update(Message::Session(SessionMsg::Insights(msg)));
    send(
        &mut app,
        InsightsMsg::ToggleThread {
            session: s,
            parent: "p".into(),
        },
    );
    assert!(app.structured_insights.thread(s, "p").is_some());
    send(&mut app, InsightsMsg::ToggleBackgroundTasks { session: s });
    assert!(app.structured_insights.tasks_open(s));
    send(
        &mut app,
        InsightsMsg::ToggleThread {
            session: s,
            parent: "p".into(),
        },
    );
    assert!(app.structured_insights.thread(s, "p").is_none());
}

#[test]
fn forgetting_a_structured_session_drops_it_from_both_features() {
    let (mut app, s) = app_with_threads();
    app.forget_structured(s);
    assert!(app.structured_transcript.transcript(s).is_none());
    assert!(app.structured_insights.insights(s).is_none());
}
