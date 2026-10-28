//! #1550 M4a (T033; research R2, contracts/selectable-text.md *T-6*, FR-002, SC-001): what the
//! transcript view draws selectable is what a copy copies. For every recorded fixture, folded and
//! unfolded, through `BlockRenderers::RICH` and `PLAIN`, with the omitted notice and request and
//! resolution rows, the `(TextPath, text)` list the selectable leaves of each row report to a
//! widget operation equals `units(row, view)`.

mod support;

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
