//! #1550 M4a (T033; research R2, contracts/selectable-text.md *T-6*, FR-002, SC-001): what the
//! transcript view draws selectable is what a copy copies. For every recorded fixture, folded and
//! unfolded, through `BlockRenderers::RICH` and `PLAIN`, with the omitted notice and request and
//! resolution rows, the `(TextPath, text)` list the selectable leaves of each row report to a
//! widget operation equals `units(row, view)`.

use crate::support;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::operation::{self, Outcome};
use iced::advanced::widget::{Operation, Tree};
use iced::{Element, Size};
use micold_client::features::permission_prompts::State as Prompts;
use micold_client::features::structured_transcript::{State, TranscriptMsg};
use micold_client::features::transcript_rich::RichCache;
use micold_client::features::transcript_selection::{rows, units, TranscriptView};
use micold_client::ui::syntax::GRAMMAR;
use micold_client::ui::transcript::{view_with_prompts, BlockRenderers, PromptsView, RenderCtx};
use micold_core::protocol::messages::{PermissionRequest, PromptId, Resolution};
use micold_core::session::SessionId;
use micold_core::structured::permission::request::{Question, QuestionOption, RequestKind};
use micold_core::structured::selection::{
    Granularity, RowPos, Selection, TextPath, TextPos, Under,
};
use micold_core::structured::transcript::{EntryKey, EntryKind};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent};
use micold_material::test_support::ShownTexts;
use support::layout::renderer;
use support::transcript::{ctx, fixture_lines, sequenced};

type Slot = (RowPos, TextPath);
type Listed = Vec<(Vec<u16>, String)>;

/// Every recorded stream-json fixture's name.
fn fixtures() -> Vec<String> {
    let dir = format!(
        "{}/../micold-core/tests/fixtures/stream-json",
        env!("CARGO_MANIFEST_DIR")
    );
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir} lists: {e}"))
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            Some(name.strip_suffix(".jsonl")?.to_owned())
        })
        .collect();
    names.sort();
    names
}

fn request(
    session: SessionId,
    id: u64,
    tool_use_id: &str,
    tool: &str,
    input: &str,
) -> PermissionRequest {
    PermissionRequest {
        id: PromptId(id),
        session,
        tool_use_id: Some(tool_use_id.into()),
        tool_name: tool.into(),
        input_preview: input.into(),
        input_bytes: input.len() as u64,
        kind: RequestKind::Tool,
        always_allow: Some(format!("{tool}: *")),
    }
}

/// The window's prompts for `state`'s transcript: a tool request under its first tool card, how
/// the request of its second card ended, and a plan approval and a question after the last entry.
fn prompts_for(state: &State, session: SessionId) -> Prompts {
    let mut prompts = Prompts::default();
    let cards: Vec<(String, String, String)> = state
        .transcript(session)
        .map(|t| {
            t.entries()
                .filter_map(|e| match e.kind() {
                    EntryKind::ToolCall(card) => {
                        Some((card.id.clone(), card.name.clone(), card.input.clone()))
                    }
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    if let Some((id, name, input)) = cards.first() {
        prompts.requested(request(session, 1, id, name, input));
    }
    if let Some((id, name, input)) = cards.get(1) {
        prompts.requested(request(session, 2, id, name, input));
        prompts.resolved(session, PromptId(2), Some(id.clone()), Resolution::Allowed);
    }
    prompts.requested(PermissionRequest {
        kind: RequestKind::PlanApproval {
            plan: "1. Read\n2. Write".into(),
        },
        always_allow: None,
        ..request(session, 3, "toolu_gone_plan", "ExitPlanMode", "{}")
    });
    let option = |label: &str| QuestionOption {
        label: label.into(),
        description: String::new(),
    };
    prompts.requested(PermissionRequest {
        kind: RequestKind::Question {
            questions: vec![Question {
                header: "Colour".into(),
                text: "Which colour?".into(),
                options: vec![option("Red"), option("Blue")],
                multi_select: false,
            }],
        },
        always_allow: None,
        ..request(session, 4, "toolu_gone_question", "AskUserQuestion", "{}")
    });
    prompts
}

/// What the selectable leaves of `element` report, by slot: each slot's lines joined in order.
fn shown<M: 'static>(mut element: Element<'_, M>) -> BTreeMap<Slot, String> {
    let renderer = renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, Size::new(900.0, 1.0e6));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let found = Arc::new(Mutex::new(None));
    let sink = Arc::clone(&found);
    let mut current: Box<dyn Operation> = Box::new(operation::map(
        ShownTexts::<Slot>(Vec::new()),
        move |value| *sink.lock().expect("not poisoned") = Some(value),
    ));
    loop {
        element
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, current.as_mut());
        match current.finish() {
            Outcome::Chain(next) => current = next,
            Outcome::Some(()) | Outcome::None => break,
        }
    }
    let reported = found
        .lock()
        .expect("not poisoned")
        .take()
        .unwrap_or_default();
    let mut lines: BTreeMap<Slot, Vec<(usize, String)>> = BTreeMap::new();
    for (slot, base, text) in reported {
        lines.entry(slot).or_default().push((base, text));
    }
    lines
        .into_iter()
        .map(|(slot, mut lines)| {
            lines.sort_by_key(|(base, _)| *base);
            let mut joined = String::new();
            for (i, (base, text)) in lines.iter().enumerate() {
                if i > 0 {
                    joined.push('\n');
                }
                assert_eq!(
                    *base,
                    joined.len(),
                    "{slot:?}: a line starts where the lines before it end"
                );
                joined.push_str(text);
            }
            (slot, joined)
        })
        .collect()
}

/// One case: a fixture's transcript as the window holds it.
struct Case {
    session: SessionId,
    state: State,
    rich: RichCache,
    prompts: Prompts,
}

impl Case {
    fn new(name: &str, omitted: u64, open: bool, rich: bool) -> Self {
        let session = SessionId::new();
        let mut state = State::default();
        state.apply(session, omitted, &sequenced(&fixture_lines(name), 0), false);
        Self::finish(session, state, open, rich)
    }

    /// A case over `events` instead of a fixture.
    fn of_events(events: &[SequencedEvent], open: bool) -> Self {
        let session = SessionId::new();
        let mut state = State::default();
        state.apply(session, 0, events, false);
        Self::finish(session, state, open, false)
    }

    fn finish(session: SessionId, mut state: State, open: bool, rich: bool) -> Self {
        if open {
            let keys: Vec<EntryKey> = state
                .transcript(session)
                .map(|t| t.entries().map(|e| e.key().clone()).collect())
                .unwrap_or_default();
            for key in keys {
                state.toggle(session, key);
            }
        }
        let mut cache = RichCache::default();
        if rich {
            if let Some(t) = state.transcript(session) {
                cache.refresh(session, t, &GRAMMAR);
            }
        }
        let prompts = prompts_for(&state, session);
        Self {
            session,
            state,
            rich: cache,
            prompts,
        }
    }

    fn view(&self) -> Option<TranscriptView<'_>> {
        Some(TranscriptView {
            session: self.session,
            transcript: self.state.transcript(self.session)?,
            open: self.state.open_cards(self.session)?,
            rich: &self.rich,
            prompts: Some(&self.prompts),
            insights: None,
            show_raw: true,
        })
    }

    /// Each row's expected and drawn `(path, text)` lists, where they differ.
    fn mismatches(&self, renderers: &BlockRenderers, selected: Option<&Selection>) -> Vec<String> {
        let Some(view) = self.view() else {
            return Vec::new();
        };
        let ctx = RenderCtx {
            selected,
            ..ctx(&self.rich)
        };
        let element = view_with_prompts(
            &self.state,
            self.session,
            renderers,
            ctx,
            |_: TranscriptMsg| (),
            PromptsView {
                state: &self.prompts,
                on_prompt: |_| (),
                focused: None,
                on_focus: |_, _| (),
            },
        );
        let mut drawn: BTreeMap<RowPos, Listed> = BTreeMap::new();
        for ((row, path), text) in shown(element) {
            drawn
                .entry(row)
                .or_default()
                .push((path.segments().to_vec(), text));
        }
        let mut out = Vec::new();
        for row in rows(&view) {
            let mut expected: Listed = units(&row, &view)
                .iter()
                .map(|u| (u.path.segments().to_vec(), u.text.to_string()))
                .collect();
            expected.sort();
            let got = drawn.remove(&row).unwrap_or_default();
            if got != expected {
                out.push(format!("{row:?}:\n  units {expected:?}\n  drawn {got:?}"));
            }
        }
        for (row, got) in drawn {
            out.push(format!("{row:?} is no row of the view, yet draws {got:?}"));
        }
        out
    }
}

fn check(rich: bool, renderers: &BlockRenderers) {
    let mut failures = Vec::new();
    for name in fixtures() {
        for open in [false, true] {
            let case = Case::new(&name, 3, open, rich);
            for m in case.mismatches(renderers, None) {
                failures.push(format!("{name} (open {open}): {m}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// SC-001: through the rich renderers, every row's leaves are its units.
#[test]
fn every_row_draws_its_units_through_the_rich_renderers() {
    check(true, &BlockRenderers::RICH);
}

/// SC-001: through the plain renderers (no derived document), every row's leaves are its units.
#[test]
fn every_row_draws_its_units_through_the_plain_renderers() {
    check(false, &BlockRenderers::PLAIN);
}

/// A folded card's row holds its header's units only.
#[test]
fn a_folded_card_draws_its_header_units_only() {
    let case = Case::new("tool-turn", 0, false, true);
    let view = case.view().expect("held");
    let card = view
        .transcript
        .entries()
        .find(|e| matches!(e.kind(), EntryKind::ToolCall(_)))
        .expect("a tool card");
    let listed: Vec<Vec<u16>> = units(&RowPos::Entry(card.id(), Under::Body), &view)
        .iter()
        .map(|u| u.path.segments().to_vec())
        .collect();
    assert!(listed.iter().all(|p| p == &[0] || p == &[1]), "{listed:?}");
    assert!(case.mismatches(&BlockRenderers::RICH, None).is_empty());
}

/// A selection drawn as highlight changes no leaf's text (and draws without a fault).
#[test]
fn a_selected_view_reports_the_same_leaves() {
    for name in ["markdown-turn", "edit-turn", "tool-turn"] {
        let case = Case::new(name, 3, true, true);
        let view = case.view().expect("held");
        let all = rows(&view);
        let first = all
            .iter()
            .find_map(|r| units(r, &view).into_iter().next().map(|u| (*r, u)))
            .expect("a unit");
        let last = all
            .iter()
            .rev()
            .find_map(|r| units(r, &view).into_iter().last().map(|u| (*r, u)))
            .expect("a unit");
        let mut sel = Selection::start(
            TextPos {
                row: first.0,
                path: first.1.path,
                offset: 1,
            },
            Granularity::Char,
            0..0,
        );
        sel.extend(
            TextPos {
                row: last.0,
                offset: last.1.text.len().saturating_sub(1),
                path: last.1.path,
            },
            0..0,
        );
        sel.release();
        let m = case.mismatches(&BlockRenderers::RICH, Some(&sel));
        assert!(m.is_empty(), "{name}: {}", m.join("\n"));
    }
}

/// One of each low-signal kind: a session start, a raw line, an orphan result and a footer.
fn low_signal_events() -> Vec<SequencedEvent> {
    let at = |seq, event| SequencedEvent { seq, event };
    vec![
        at(
            0,
            StructuredEvent::Init {
                session_id: "s".into(),
                model: Some("opus".into()),
                cwd: Some("/work/tree".into()),
                tools: vec![],
                permission_mode: None,
                slash_commands: vec![],
                skills: vec![],
                terminal_slash_commands: vec![],
                raw: "{}".into(),
            },
        ),
        at(
            1,
            StructuredEvent::Unreadable {
                text: "{not\tjson".into(),
            },
        ),
        at(
            2,
            StructuredEvent::User {
                blocks: vec![ContentBlock::ToolResult {
                    tool_use_id: "toolu_gone".into(),
                    content: "orphan output".into(),
                    is_error: true,
                }],
                parent_tool_use_id: None,
                is_synthetic: false,
                raw: "{}".into(),
            },
        ),
        at(
            3,
            StructuredEvent::Result {
                subtype: "error_max_turns".into(),
                is_error: true,
                duration_ms: Some(900),
                num_turns: Some(3),
                total_cost_usd: Some(0.5),
                usage: None,
                result: Some("ran out".into()),
                session_id: None,
                raw: "{}".into(),
            },
        ),
    ]
}

/// #1664 FR-001, FR-003: the four low-signal kinds' units are the summary line, plus their detail
/// only while open; what is drawn is what a copy copies (`mismatches`).
#[test]
fn a_low_signal_entry_has_its_summary_as_unit_zero_and_detail_only_while_open() {
    for open in [false, true] {
        let case = Case::of_events(&low_signal_events(), open);
        let view = case.view().expect("held");
        let mut seen = 0;
        for entry in view.transcript.entries() {
            if !matches!(
                entry.kind(),
                EntryKind::Raw { .. }
                    | EntryKind::SessionStart { .. }
                    | EntryKind::Footer(_)
                    | EntryKind::OrphanResult { .. }
            ) {
                continue;
            }
            seen += 1;
            let listed: Vec<Vec<u16>> = units(&RowPos::Entry(entry.id(), Under::Body), &view)
                .iter()
                .map(|u| u.path.segments().to_vec())
                .collect();
            assert_eq!(
                listed.first(),
                Some(&vec![0]),
                "{:?}: {listed:?}",
                entry.kind()
            );
            let has_detail = listed.iter().any(|p| p[0] >= 2);
            assert_eq!(
                has_detail,
                open,
                "{:?} (open {open}): {listed:?}",
                entry.kind()
            );
        }
        assert!(seen >= 4, "{seen} low-signal entries in the fixture");
        assert!(case.mismatches(&BlockRenderers::PLAIN, None).is_empty());
    }
}

/// #1609: a window with a tool card `p`, its open subagent thread (one message) and a message
/// after it.
struct Threaded {
    session: SessionId,
    state: State,
    insights: micold_client::features::structured_insights::State,
    rich: RichCache,
}

impl Threaded {
    fn new() -> Self {
        use crate::stream_json::{tool_use, user_text, with_parent};
        let session = SessionId::new();
        let events: Vec<SequencedEvent> = (0..)
            .zip([
                user_text("before"),
                tool_use("p", "Agent", "{}"),
                with_parent(user_text("thread says one"), "p"),
                user_text("after"),
            ])
            .map(|(seq, event)| SequencedEvent { seq, event })
            .collect();
        let mut state = State::default();
        state.apply(session, 0, &events, false);
        let mut insights = micold_client::features::structured_insights::State::default();
        let main = state.transcript(session).expect("held");
        insights.apply(session, 0, &events, false, main);
        insights.toggle_thread(session, "p", main);
        Self {
            session,
            state,
            insights,
            rich: RichCache::default(),
        }
    }

    fn view(&self) -> TranscriptView<'_> {
        TranscriptView {
            session: self.session,
            transcript: self.state.transcript(self.session).expect("held"),
            open: self.state.open_cards(self.session).expect("held"),
            rich: &self.rich,
            prompts: None,
            insights: Some(&self.insights),
            show_raw: true,
        }
    }

    fn drawn(&self, selected: Option<&Selection>) -> BTreeMap<Slot, String> {
        let ctx = RenderCtx {
            selected,
            ..ctx(&self.rich)
        };
        shown(micold_client::ui::transcript::view_full(
            &self.state,
            micold_client::ui::transcript::InsightsCtx {
                state: &self.insights,
                cli_running: true,
                mode: micold_core::session::PermissionMode::Default,
            },
            self.session,
            &BlockRenderers::PLAIN,
            ctx,
            |_: TranscriptMsg| (),
            None,
        ))
    }
}

/// #1609: an open thread's entries are rows of the view, and what they draw is their units.
#[test]
fn an_open_threads_entries_are_rows_drawn_as_their_units() {
    let case = Threaded::new();
    let view = case.view();
    let thread_rows: Vec<RowPos> = rows(&view)
        .into_iter()
        .filter(|r| matches!(r, RowPos::Entry(_, Under::Thread(..))))
        .collect();
    assert_eq!(thread_rows.len(), 1, "one thread row");
    let drawn = case.drawn(None);
    let texts: Vec<&String> = drawn
        .iter()
        .filter(|((row, _), _)| *row == thread_rows[0])
        .map(|(_, text)| text)
        .collect();
    assert_eq!(texts, ["thread says one"]);
}

/// #1609: a copy of a selection across the thread, and Select all, include its text, in order.
#[test]
fn a_copy_across_a_thread_includes_its_text() {
    use micold_client::features::transcript_selection::copy_text;
    let case = Threaded::new();
    let view = case.view();
    let all = rows(&view);
    let (first, last) = (all[0], *all.last().expect("rows"));
    let at = |row, offset| TextPos {
        row,
        path: units(&row, &view)[0].path.clone(),
        offset,
    };
    let mut selection = Selection::start(at(first, 0), Granularity::Char, 0..0);
    selection.extend(at(last, 5), 0..0);
    let text = copy_text(&view, &selection);
    assert!(text.starts_with("before"), "{text:?}");
    assert!(text.contains("thread says one"), "{text:?}");
    assert!(
        text.find("thread says one") < text.find("after"),
        "{text:?}"
    );
}
