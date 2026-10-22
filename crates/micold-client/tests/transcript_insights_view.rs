//! T038–T041 (feature 944, contracts/transcript-extensions.md X1–X5): what the session bar, the
//! turn footer, the subagent threads and the background tasks' states say, read off the pure view
//! model, and that the transcript view draws them where the contract puts them.

mod support;

use micold_client::features::structured_insights::{InsightsMsg, State as Insights};
use micold_client::features::structured_transcript::{State, TranscriptMsg};
use micold_client::icons::Icon;
use micold_client::ui::transcript::{
    footer_view, footer_view_with, meter_view, placeholder_title, running_text, session_bar_view,
    task_status, thread_header, view_with_insights, BlockCache, BlockRenderers, InsightsCtx, Tone,
    NEAR_COMPACTION,
};
use micold_core::session::SessionId;
use micold_core::structured::insights::figures::ESTIMATE_NOTE;
use micold_core::structured::insights::{
    ContextState, SessionInsights, SubagentState, Task, TaskKind, TaskState, ThreadSummary,
    ThreadView,
};
use micold_core::structured::transcript::{EntryKind, Outcome, TurnFooter};
use micold_core::structured::{SequencedEvent, StructuredEvent};
use micold_core::task_progress::Ratio;
use serde_json::json;
use support::layout::{painted_text_settled_at, renderer};
use support::transcript::{ctx, fixture_lines, sequenced};

#[path = "../../micold-core/tests/support/stream_json.rs"]
mod stream_json;
use stream_json::{tool_use, user_text, with_parent};

/// One window's transcript and insights of one session, fed as the app feeds them.
struct Window {
    session: SessionId,
    transcript: State,
    insights: Insights,
}

impl Window {
    fn of(events: &[SequencedEvent]) -> Self {
        let session = SessionId::new();
        let mut w = Window {
            session,
            transcript: State::default(),
            insights: Insights::default(),
        };
        w.transcript.apply(session, 0, events, false);
        let main = w.transcript.transcript(session).expect("a transcript");
        w.insights.apply(session, 0, events, false, main);
        w
    }

    fn recorded(name: &str) -> Self {
        Self::of(&sequenced(&fixture_lines(name), 0))
    }

    fn core(&self) -> &SessionInsights {
        self.insights.insights(self.session).expect("insights")
    }

    fn toggle_thread(&mut self, parent: &str) {
        let main = self.transcript.transcript(self.session).unwrap();
        self.insights.toggle_thread(self.session, parent, main);
    }

    /// The text the view paints, the session's CLI `cli_running` or not.
    fn painted(&self, cli_running: bool) -> Vec<String> {
        let cache = BlockCache::default();
        let element = view_with_insights(
            &self.transcript,
            InsightsCtx {
                state: &self.insights,
                cli_running,
            },
            self.session,
            &BlockRenderers::PLAIN,
            ctx(&cache),
        );
        let (painted, _) =
            painted_text_settled_at(element, &mut renderer(), iced::Size::new(1200.0, 8_000.0));
        painted.into_iter().map(|p| p.content).collect()
    }
}

fn has(painted: &[String], text: &str) -> bool {
    painted.iter().any(|p| p.contains(text))
}

fn seq(events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    (0..)
        .zip(events)
        .map(|(seq, event)| SequencedEvent { seq, event })
        .collect()
}

// ---- X1: the meter ----

#[test]
fn before_any_response_the_meter_says_no_data_and_draws_no_bar() {
    let meter = meter_view(&ContextState::NoData);
    assert_eq!(meter.label, "No data yet");
    assert_eq!(meter.bar, None);
    assert!(!meter.warning);
}

#[test]
fn without_a_window_the_meter_shows_tokens_only() {
    let meter = meter_view(&ContextState::TokensOnly { used: 37_141 });
    assert_eq!(meter.label, "37,141 tokens");
    assert_eq!(meter.bar, None, "no share without a window (FR-004)");
}

#[test]
fn a_measured_meter_fills_its_share_and_marks_the_threshold() {
    let meter = meter_view(&ContextState::Measured {
        used: 250_000,
        window: 1_000_000,
        percent: 25,
        threshold: Some(800_000),
        warning: false,
    });
    assert_eq!(
        meter.bar,
        Some((
            Ratio {
                done: 250,
                total: 1_000
            },
            Some(Ratio {
                done: 800,
                total: 1_000
            })
        ))
    );
    assert!(meter.label.ends_with("25%"));
    assert!(meter
        .details
        .iter()
        .any(|l| l.starts_with("Compaction threshold")));
}

#[test]
fn the_warning_says_near_compaction_as_well_as_its_colour() {
    let meter = meter_view(&ContextState::Measured {
        used: 900_000,
        window: 1_000_000,
        percent: 90,
        threshold: None,
        warning: true,
    });
    assert!(meter.warning);
    assert!(meter.label.contains(NEAR_COMPACTION), "{}", meter.label);
}

#[test]
fn a_count_beyond_the_window_fills_the_bar_and_no_more() {
    let meter = meter_view(&ContextState::Measured {
        used: 5_000_000_000,
        window: 1_000_000,
        percent: 500_000,
        threshold: None,
        warning: true,
    });
    assert_eq!(
        meter.bar.map(|(r, _)| r),
        Some(Ratio {
            done: 1_000,
            total: 1_000
        })
    );
}

// ---- X1: totals and background tasks ----

#[test]
fn the_running_count_is_absent_at_zero_and_counts_otherwise() {
    assert_eq!(running_text(0), None);
    assert_eq!(
        running_text(1).as_deref(),
        Some("1 background task running")
    );
    assert_eq!(
        running_text(3).as_deref(),
        Some("3 background tasks running")
    );
}

#[test]
fn a_session_without_background_tasks_shows_no_count_and_no_list() {
    let w = Window::recorded("text-turn");
    let bar = session_bar_view(w.core(), true);
    assert_eq!(bar.running, None);
    assert!(bar.tasks.is_empty());
    assert!(bar.totals.as_deref().is_some_and(|t| t.contains("est.")));
    assert!(bar.totals_details.iter().any(|l| l == ESTIMATE_NOTE));
}

fn task_started(task_id: &str, tool: &str, kind: &str) -> StructuredEvent {
    micold_core::structured::parse_line(
        &json!({
            "type": "system", "subtype": "task_started", "session_id": "<SESSION_ID>",
            "task_id": task_id, "tool_use_id": tool, "description": "sleep 20",
            "task_type": kind, "is_backgrounded": true,
        })
        .to_string(),
    )
}

#[test]
fn a_running_background_task_is_listed_and_counted_and_ends_when_the_cli_does() {
    let w = Window::of(&seq([
        tool_use("b", "Bash", r#"{"command":"sleep 20"}"#),
        task_started("t1", "b", "local_bash"),
    ]));
    let running = session_bar_view(w.core(), true);
    assert_eq!(
        running.running.as_deref(),
        Some("1 background task running")
    );
    assert_eq!(running.tasks.len(), 1);
    assert_eq!(running.tasks[0].kind, Icon::BackgroundTask);
    assert_eq!(running.tasks[0].description, "sleep 20");
    assert_eq!(running.tasks[0].status.label, "Running");

    let ended = session_bar_view(w.core(), false);
    assert_eq!(ended.running, None, "FR-022");
    assert_eq!(ended.tasks[0].status.label, "Ended without a report");
    assert_eq!(ended.tasks[0].status.icon, Icon::ToolNoResult);
}

#[test]
fn the_recorded_background_command_finishes_with_its_outcome() {
    let w = Window::recorded("background-command");
    let bar = session_bar_view(w.core(), true);
    assert_eq!(bar.running, None);
    let task = &bar.tasks[0];
    assert!(task.status.label.starts_with("Finished · "), "{task:?}");
    assert_eq!(task.status.icon, Icon::ToolDone);
}

fn task(state: TaskState, kind: Option<TaskKind>) -> Task {
    Task {
        task_id: "t".into(),
        tool_use_id: Some("b".into()),
        description: "d".into(),
        kind,
        background: true,
        state,
        outcome: None,
        activity: None,
    }
}

#[test]
fn a_failed_task_says_failed_with_the_failed_icon_in_the_error_tone() {
    let status = task_status(&task(TaskState::Failed, None), true);
    assert_eq!(status.label, "Failed");
    assert_eq!(status.icon, Icon::ToolFailed);
    assert_eq!(status.tone, Tone::Error);
}

#[test]
fn a_subagent_task_takes_the_subagent_icon() {
    use micold_client::ui::transcript::kind_icon;
    assert_eq!(kind_icon(Some(TaskKind::Subagent)), Icon::Subagent);
    assert_eq!(kind_icon(Some(TaskKind::Command)), Icon::BackgroundTask);
    assert_eq!(kind_icon(None), Icon::BackgroundTask);
}

// ---- X2: the footer ----

fn footer() -> TurnFooter {
    TurnFooter {
        outcome: Outcome::Success,
        duration_ms: Some(6_674),
        cost_usd: Some(0.0371549),
        num_turns: Some(1),
        error_text: None,
    }
}

#[test]
fn a_footer_adds_the_turns_cost_and_tokens_after_939s_figures() {
    let w = Window::recorded("text-turn");
    let main = w.transcript.transcript(w.session).unwrap();
    let entry = main
        .entries()
        .find(|e| matches!(e.kind(), EntryKind::Footer(_)))
        .expect("a footer");
    let EntryKind::Footer(f) = entry.kind() else {
        unreachable!()
    };
    let turn = w.insights.turn(w.session, entry.key()).expect("figures");
    let plain = footer_view(f);
    let view = footer_view_with(f, Some(turn));
    assert_eq!(view.figures[..plain.figures.len()], plain.figures[..]);
    let added = &view.figures[plain.figures.len()..];
    assert!(added.iter().any(|f| f.ends_with(" est.")), "{added:?}");
    assert!(added.iter().any(|f| f.starts_with("in ")), "{added:?}");
    assert!(view.details.iter().any(|l| l == ESTIMATE_NOTE));
    assert!(view.details.iter().any(|l| l.starts_with("Input cached")));
}

#[test]
fn a_footer_without_figures_adds_only_the_estimate_note() {
    let view = footer_view_with(&footer(), None);
    assert_eq!(view.figures, footer_view(&footer()).figures);
    assert_eq!(view.details, vec![ESTIMATE_NOTE.to_owned()]);
}

// ---- X3, X5: threads ----

fn summary(parent: &str) -> ThreadSummary {
    ThreadSummary {
        parent: parent.into(),
        first_seq: 0,
        messages: 2,
        tool_calls: 1,
        latest_tool: Some("Read".into()),
    }
}

#[test]
fn a_failed_thread_says_failed_with_the_failed_icon() {
    let s = summary("p");
    let failed = thread_header(&ThreadView {
        summary: &s,
        state: SubagentState::Failed,
        activity: None,
    });
    let status = failed.failed.expect("drawn failed");
    assert_eq!((status.icon, status.label), (Icon::ToolFailed, "Failed"));
    let running = thread_header(&ThreadView {
        summary: &s,
        state: SubagentState::Running,
        activity: Some("Reading notes.txt".into()),
    });
    assert_eq!(running.failed, None);
    assert_eq!(
        running.summary,
        "Subagent · running · 1 tool call · Reading notes.txt"
    );
}

#[test]
fn a_placeholder_names_the_parent() {
    assert_eq!(
        placeholder_title("toolu_9"),
        "Subagent of an earlier step (toolu_9)"
    );
}

const SUBAGENT_PARENT: &str = "toolu_016Evhgage4WoGiCC3qAXbiy";

#[test]
fn a_thread_is_folded_under_its_card_and_shows_its_entries_once_opened() {
    let mut w = Window::recorded("subagent-turn");
    let folded = w.painted(true);
    assert!(
        has(&folded, "Subagent · "),
        "the thread's header: {folded:?}"
    );
    assert!(
        !has(&folded, "Finished · "),
        "a foreground subagent's task state is its thread's, not its card's (X4 is background)"
    );
    let thread_text: Vec<String> = {
        let main = w.transcript.transcript(w.session).unwrap();
        let mut t = micold_core::structured::transcript::Transcript::nested(SUBAGENT_PARENT);
        let events: Vec<_> = main.subagent_events(SUBAGENT_PARENT).cloned().collect();
        t.apply(0, &events, false);
        t.entries()
            .filter_map(|e| match e.kind() {
                EntryKind::AssistantText { text, .. } => Some(text.clone()),
                _ => None,
            })
            .filter(|t| !t.trim().is_empty())
            .collect()
    };
    let first = thread_text
        .first()
        .expect("the subagent wrote text")
        .lines()
        .next()
        .unwrap()
        .to_owned();
    assert!(
        !has(&folded, &first),
        "folded: none of the thread is drawn: {first:?} in {folded:?}"
    );

    w.toggle_thread(SUBAGENT_PARENT);
    assert!(
        has(&w.painted(true), &first),
        "open: the thread's text is drawn"
    );
}

#[test]
fn an_inner_thread_folds_under_the_inner_card_inside_the_outer_thread() {
    let mut w = Window::recorded("nested-subagent");
    let threads: Vec<String> = w
        .core()
        .threads_owned_by(
            &micold_core::structured::insights::Owner::Main,
            w.transcript.transcript(w.session).unwrap(),
        )
        .iter()
        .map(|t| t.parent.clone())
        .collect();
    let outer = threads.first().expect("an outer thread").clone();
    let headers = |p: &[String]| p.iter().filter(|t| t.starts_with("Subagent · ")).count();
    let before = headers(&w.painted(true));
    w.toggle_thread(&outer);
    let after = headers(&w.painted(true));
    assert!(
        after > before,
        "the inner thread's header is drawn inside the outer one ({before} → {after})"
    );
}

#[test]
fn events_of_an_unknown_parent_are_under_a_placeholder_that_opens() {
    let mut w = Window::of(&seq([
        user_text("hello"),
        with_parent(user_text("from the gone step"), "toolu_gone"),
    ]));
    let painted = w.painted(true);
    assert!(
        has(&painted, &placeholder_title("toolu_gone")),
        "{painted:?}"
    );
    assert!(!has(&painted, "not shown"), "the notice is replaced");
    assert!(!has(&painted, "from the gone step"));
    w.toggle_thread("toolu_gone");
    assert!(has(&w.painted(true), "from the gone step"));
}

#[test]
fn a_card_inside_a_thread_unfolds_through_the_insights() {
    let mut w = Window::of(&seq([
        tool_use("p", "Agent", "{}"),
        with_parent(
            tool_use("inner", "Read", r#"{"file_path":"notes.txt"}"#),
            "p",
        ),
    ]));
    w.toggle_thread("p");
    let key = w
        .insights
        .thread(w.session, "p")
        .unwrap()
        .entries()
        .find(|e| matches!(e.kind(), EntryKind::ToolCall(_)))
        .expect("the thread holds the inner card")
        .key()
        .clone();
    assert!(!w.insights.entry_open(w.session, &key));
    let main = w.transcript.transcript(w.session).unwrap().clone();
    w.insights.update(
        InsightsMsg::ToggleEntry {
            session: w.session,
            key: key.clone(),
        },
        &main,
    );
    assert!(w.insights.entry_open(w.session, &key));
    // Folding the thread forgets the card's fold with it.
    w.toggle_thread("p");
    assert!(!w.insights.entry_open(w.session, &key));
}

// ---- X4: a card shows its task's state ----

#[test]
fn a_card_that_started_a_background_task_shows_its_state() {
    let w = Window::of(&seq([
        tool_use("b", "Bash", r#"{"command":"sleep 20"}"#),
        task_started("t1", "b", "local_bash"),
    ]));
    assert!(has(&w.painted(true), "Running"));
    assert!(has(&w.painted(false), "Ended without a report"));
}

// ---- X1 drawn ----

#[test]
fn the_view_draws_the_session_bar_above_the_entries() {
    let w = Window::recorded("text-turn");
    let painted = w.painted(true);
    let label =
        micold_core::structured::insights::figures::context_label(&w.core().context().state());
    assert!(has(&painted, &label), "{label:?} in {painted:?}");
    let empty = Window::of(&[]);
    assert!(has(&empty.painted(true), "No data yet"));
}

#[test]
fn the_views_insight_messages_are_the_insights() {
    let w = Window::of(&[]);
    let msg = TranscriptMsg::Insights(InsightsMsg::ToggleBackgroundTasks { session: w.session });
    assert_eq!(msg.session(), Some(w.session));
}
