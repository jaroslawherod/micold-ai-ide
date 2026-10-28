//! #1550 M2 (T013, T015, T018, T022; contracts/selection-model.md S-2, S-3, S-5–S-12,
//! keys-menu-copy.md M-2, M-3; data-model.md *Unit*, *TranscriptView*, *State transitions*;
//! research R2, R3, R7, R9): what each row of a structured session's transcript contributes to a
//! copied selection, and the window's selection state per session — the gesture, granularity,
//! select-all, the context menu, the rules that clear it and the checked copy. Render-free.

use std::collections::HashSet;

use micold_client::app::State as App;
use micold_client::features::permission_prompts::State as Prompts;
use micold_client::features::structured_transcript::TranscriptMsg;
use micold_client::features::transcript_rich::{DerivedBody, RichCache};
use micold_client::features::transcript_selection::shown::{
    footer_view, hunk_title, omitted_text, resolution_label, structured_input, unshown_caption,
    INPUT_CUT, PLAN_TITLE,
};
use micold_client::features::transcript_selection::{
    copy_text, rows, units, Click, Joint, SelectMsg, TranscriptView, Unit, COPY_FAILED,
};
use micold_client::features::window::FieldId;
use micold_client::features::Outcome;
use micold_client::ui::syntax::GRAMMAR;
use micold_core::protocol::messages::{
    PermissionRequest, PromptId, Resolution, SessionProcess, TerminalRef,
};
use micold_core::session::SessionId;
use micold_core::structured::permission::request::RequestKind;
use micold_core::structured::selection::{
    Granularity, RowPos, Selection, TextPath, TextPos, Under,
};
use micold_core::structured::transcript::{EntryId, EntryKey, EntryKind, ResultPart, Transcript};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent, MAX_EVENTS};
use serde_json::json;

// ---- events ----

fn assistant(block: ContentBlock) -> StructuredEvent {
    StructuredEvent::Assistant {
        error: None,
        model: None,
        message_id: None,
        raw: json!({"type": "assistant", "message": {"content": []}}).to_string(),
        blocks: vec![block],
        parent_tool_use_id: None,
    }
}

fn text(t: &str) -> StructuredEvent {
    assistant(ContentBlock::Text { text: t.into() })
}

fn user(t: &str) -> StructuredEvent {
    StructuredEvent::UserSent {
        text: t.into(),
        attachments: vec![],
    }
}

fn tool_use(id: &str, name: &str, input: &str) -> StructuredEvent {
    assistant(ContentBlock::ToolUse {
        id: id.into(),
        name: name.into(),
        input: input.into(),
    })
}

/// A result of tool use `id` whose content is `parts` (`{"type": "text", "text": …}` and others).
fn tool_result(id: &str, parts: serde_json::Value) -> StructuredEvent {
    let first = parts[0]["text"].as_str().unwrap_or_default().to_owned();
    StructuredEvent::User {
        is_synthetic: false,
        raw: json!({"type": "user", "message": {"content": [
            {"type": "tool_result", "tool_use_id": id, "content": parts}
        ]}})
        .to_string(),
        blocks: vec![ContentBlock::ToolResult {
            tool_use_id: id.into(),
            content: first,
            is_error: false,
        }],
        parent_tool_use_id: None,
    }
}

fn result_event() -> StructuredEvent {
    StructuredEvent::Result {
        subtype: "success".into(),
        is_error: false,
        duration_ms: Some(1_500),
        num_turns: Some(2),
        total_cost_usd: None,
        usage: None,
        result: None,
        session_id: None,
        raw: "{}".into(),
    }
}

fn seq(first: u64, events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    events
        .into_iter()
        .zip(first..)
        .map(|(event, seq)| SequencedEvent { seq, event })
        .collect()
}

fn request(session: SessionId, id: u64, tool_use_id: &str, kind: RequestKind) -> PermissionRequest {
    PermissionRequest {
        id: PromptId(id),
        session,
        tool_use_id: Some(tool_use_id.into()),
        tool_name: "Bash".into(),
        input_preview: r#"{"command":"npm test"}"#.into(),
        input_bytes: 22,
        kind,
        always_allow: None,
    }
}

// ---- a held transcript and its view, without a window ----

/// One session's transcript as a window holds it: its entries, open cards, render cache and
/// permission prompts.
struct Held {
    session: SessionId,
    transcript: Transcript,
    open: HashSet<EntryKey>,
    rich: RichCache,
    prompts: Prompts,
}

impl Held {
    fn new(omitted: u64, events: impl IntoIterator<Item = StructuredEvent>) -> Self {
        let mut transcript = Transcript::new();
        transcript.apply(omitted, &seq(0, events), false);
        let session = SessionId::new();
        let mut rich = RichCache::default();
        rich.refresh(session, &transcript, &GRAMMAR);
        Self {
            session,
            transcript,
            open: HashSet::new(),
            rich,
            prompts: Prompts::default(),
        }
    }

    fn view(&self) -> TranscriptView<'_> {
        TranscriptView {
            session: self.session,
            transcript: &self.transcript,
            open: &self.open,
            rich: &self.rich,
            prompts: Some(&self.prompts),
            insights: None,
        }
    }

    fn id(&self, n: usize) -> EntryId {
        self.transcript
            .entries()
            .nth(n)
            .expect("the entry is held")
            .id()
    }

    fn open_all(&mut self) {
        self.open = self.transcript.entries().map(|e| e.key().clone()).collect();
    }

    fn body(&self, n: usize) -> RowPos {
        RowPos::Entry(self.id(n), Under::Body)
    }

    fn units_of(&self, row: RowPos) -> Vec<(Vec<u16>, String)> {
        listed(&units(&row, &self.view()))
    }

    /// The selection from the start of `from`'s first unit to the end of `to`'s last unit.
    fn rows_selection(&self, from: RowPos, to: RowPos) -> Selection {
        let view = self.view();
        let first = units(&from, &view).into_iter().next().expect("a unit");
        let last = units(&to, &view).into_iter().last().expect("a unit");
        let len = last.text.len();
        selection(
            TextPos {
                row: from,
                path: first.path,
                offset: 0,
            },
            TextPos {
                row: to,
                path: last.path,
                offset: len,
            },
        )
    }
}

fn listed(units: &[Unit<'_>]) -> Vec<(Vec<u16>, String)> {
    units
        .iter()
        .map(|u| (u.path.segments().to_vec(), u.text.to_string()))
        .collect()
}

fn at(row: RowPos, path: &[u16], offset: usize) -> TextPos {
    TextPos {
        row,
        path: TextPath::new(path),
        offset,
    }
}

fn selection(from: TextPos, to: TextPos) -> Selection {
    let mut s = Selection::start(from, Granularity::Char, 0..0);
    s.extend(to, 0..0);
    s
}

fn card(held: &Held, n: usize) -> micold_core::structured::transcript::ToolCard {
    match held.transcript.entries().nth(n).expect("held").kind() {
        EntryKind::ToolCall(card) => card.clone(),
        other => panic!("not a tool card: {other:?}"),
    }
}

// ---- T013: rows, units and copy text ----

/// S-1: the omitted notice, each entry's body then the request or resolution rows under it, then
/// the request blocks at the end, in drawing order.
#[test]
fn rows_list_the_notice_entries_rows_under_cards_and_requests_at_the_end_in_drawing_order() {
    let mut held = Held::new(
        3,
        [
            text("hello"),
            tool_use("toolu_a", "Bash", r#"{"command":"ls"}"#),
            tool_use("toolu_b", "Bash", r#"{"command":"pwd"}"#),
        ],
    );
    let s = held.session;
    held.prompts
        .requested(request(s, 1, "toolu_a", RequestKind::Tool));
    held.prompts
        .requested(request(s, 2, "toolu_gone", RequestKind::Tool));
    held.prompts
        .requested(request(s, 3, "toolu_b", RequestKind::Tool));
    held.prompts
        .resolved(s, PromptId(3), Some("toolu_b".into()), Resolution::Allowed);
    assert_eq!(
        rows(&held.view()),
        vec![
            RowPos::Omitted,
            held.body(0),
            held.body(1),
            RowPos::Entry(held.id(1), Under::Prompt(0)),
            held.body(2),
            RowPos::Entry(held.id(2), Under::Prompt(0)),
            RowPos::End(0),
        ]
    );
    assert_eq!(
        held.units_of(RowPos::Omitted),
        vec![(vec![0], omitted_text(3))]
    );
    // A resolution row says how the request ended, nothing else.
    assert_eq!(
        held.units_of(RowPos::Entry(held.id(2), Under::Prompt(0))),
        vec![(vec![0], resolution_label(Resolution::Allowed).to_owned())]
    );
}

/// S-2: a request block contributes its descriptive text only — the tool, its input, the cut
/// notice — never a button label or an input field.
#[test]
fn a_request_block_copies_its_descriptive_text_without_buttons_or_fields() {
    let mut held = Held::new(0, [text("hello")]);
    let s = held.session;
    held.prompts.requested(PermissionRequest {
        input_bytes: 500,
        always_allow: Some("Bash: npm test".into()),
        ..request(s, 1, "toolu_gone", RequestKind::Tool)
    });
    let units = held.units_of(RowPos::End(0));
    let texts: Vec<&str> = units.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(texts[0], "Bash");
    assert!(texts.contains(&structured_input(r#"{"command":"npm test"}"#).as_str()));
    assert!(texts.contains(&INPUT_CUT));
    for label in [
        "Allow",
        "Deny",
        "Always allow",
        "Show all",
        "Tell Claude why (optional)",
    ] {
        assert!(!texts.contains(&label), "{label} is a control: {texts:?}");
    }
    let mut plan = Held::new(0, [text("hello")]);
    let s = plan.session;
    plan.prompts.requested(request(
        s,
        1,
        "toolu_gone",
        RequestKind::PlanApproval {
            plan: "1. Read".into(),
        },
    ));
    assert_eq!(
        plan.units_of(RowPos::End(0)),
        vec![
            (vec![0], PLAN_TITLE.to_owned()),
            (vec![1], "1. Read".into())
        ]
    );
}

/// A user message as drawn: tabs expanded; its "You" label is not a unit.
#[test]
fn a_user_message_copies_its_text_as_drawn() {
    let held = Held::new(0, [user("hello\tworld\nnext")]);
    assert_eq!(
        held.units_of(held.body(0)),
        vec![(vec![0], "hello   world\nnext".to_owned())]
    );
}

/// S-3, R3: markdown copies its shown text; list markers before their item text on its line,
/// table cells joined by a tab and rows by a line break, inline code without backticks, links as
/// their shown text, a code block's lines in full.
#[test]
fn markdown_copies_its_shown_text_markers_cells_and_code() {
    let md = "# Title\n\nSome **bold** `code` [link](https://example.com)\n\n- one\n- two\n\n\
              3. c\n4. d\n\n| h1 | h2 |\n|----|----|\n| c1 |    |\n\n```rust\nfn a() {}\n\tx\n```\n";
    let held = Held::new(0, [text(md)]);
    let units = held.units_of(held.body(0));
    let paths: Vec<Vec<u16>> = units.iter().map(|(p, _)| p.clone()).collect();
    let mut sorted = paths.clone();
    sorted.sort();
    assert_eq!(paths, sorted, "units come in drawing order");
    let code = match &held
        .rich
        .get(
            held.session,
            held.transcript.entries().next().unwrap().key(),
        )
        .expect("derived")
        .body
    {
        DerivedBody::Text(doc) => doc
            .code_blocks()
            .next()
            .expect("a code block")
            .1
            .code
            .clone(),
        other => panic!("{other:?}"),
    };
    assert!(code.contains("\tx"), "the code keeps its tab: {code:?}");
    let all = held.rows_selection(held.body(0), held.body(0));
    assert_eq!(
        copy_text(&held.view(), &all),
        format!(
            "Title\nSome bold code link\n\u{2022} one\n\u{2022} two\n3. c\n4. d\nh1\th2\nc1\t\n{code}"
        )
    );
}

/// A markdown entry the cache has not derived is drawn, and copied, as plain text.
#[test]
fn text_without_a_derived_document_copies_as_plain_text() {
    let mut held = Held::new(0, [text("**bold**\tx")]);
    held.rich = RichCache::default();
    assert_eq!(
        held.units_of(held.body(0)),
        vec![(vec![0], "**bold**    x".to_owned())]
    );
}

/// FR-002, US1 scenario 3: a folded tool card contributes its header only; unfolded, its input
/// and each result part, an unshown part as its caption.
#[test]
fn a_folded_card_copies_its_header_and_an_unfolded_one_its_input_and_result_parts() {
    let input = r#"{"command":"ls -la"}"#;
    let mut held = Held::new(
        0,
        [
            tool_use("toolu_a", "Bash", input),
            tool_result(
                "toolu_a",
                json!([{"type": "text", "text": "a\tb.txt"}, {"type": "image", "source": {}}]),
            ),
        ],
    );
    let header = card(&held, 0);
    assert!(!header.summary.is_empty());
    assert_eq!(
        held.units_of(held.body(0)),
        vec![
            (vec![0], "Bash".to_owned()),
            (vec![1], header.summary.clone())
        ]
    );
    held.open_all();
    let view = held.view();
    let open = units(&held.body(0), &view);
    assert_eq!(
        listed(&open),
        vec![
            (vec![0], "Bash".to_owned()),
            (vec![1], header.summary.clone()),
            (vec![2], input.to_owned()),
            (vec![3, 0], "a   b.txt".to_owned()),
            (vec![3, 1], unshown_caption("image")),
        ]
    );
    assert!(matches!(
        header.result.as_ref().unwrap().parts[1],
        ResultPart::Unshown { .. }
    ));
    // The name and the summary share the header's line.
    assert_eq!(open[1].joint, Joint::Space);
    assert_eq!(open[2].joint, Joint::Line);
}

/// R2: an inline diff copies its path, its hunks' labels and each line's text, without the
/// gutter's marker or number.
#[test]
fn an_inline_diff_copies_its_path_hunk_labels_and_line_text() {
    let input = json!({"file_path": "src/a.rs", "edits": [
        {"old_string": "one", "new_string": "uno"},
        {"old_string": "two", "new_string": "dos"},
    ]})
    .to_string();
    let mut held = Held::new(
        0,
        [
            tool_use("toolu_e", "MultiEdit", &input),
            tool_result("toolu_e", json!([{"type": "text", "text": "done"}])),
        ],
    );
    held.open_all();
    let units = held.units_of(held.body(0));
    let texts: Vec<&str> = units.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(&texts[..2], ["MultiEdit", card(&held, 0).summary.as_str()]);
    assert_eq!(texts[2], "src/a.rs");
    assert!(texts.contains(&hunk_title(1, 2).as_str()), "{texts:?}");
    assert!(texts.contains(&"one\nuno"), "{texts:?}");
    assert!(texts.contains(&"two\ndos"), "{texts:?}");
    assert!(
        !texts
            .iter()
            .any(|t| t.starts_with('+') || t.starts_with('-')),
        "{texts:?}"
    );
    // A diff's lines are source lines: a triple click selects one of them.
    let view = held.view();
    assert!(units_of_view(&view, held.body(0))
        .iter()
        .any(|u| u.source && u.text == "one\nuno"));
}

fn units_of_view<'a>(view: &TranscriptView<'a>, row: RowPos) -> Vec<Unit<'a>> {
    units(&row, view)
}

/// Footers and notices copy as drawn.
#[test]
fn a_footer_copies_its_status_and_figures() {
    let held = Held::new(0, [user("go"), text("done"), result_event()]);
    let footer = held
        .transcript
        .entries()
        .find_map(|e| match e.kind() {
            EntryKind::Footer(f) => Some(f.clone()),
            _ => None,
        })
        .expect("a footer");
    let view = footer_view(&footer);
    let n = held.transcript.entries().len() - 1;
    assert_eq!(
        held.units_of(held.body(n)),
        vec![
            (vec![0], view.status.label.to_owned()),
            (vec![1], view.figures.join(" \u{b7} ")),
        ]
    );
}

/// S-2: copy text slices the first and last unit at the two offsets, joins units and rows with
/// one line break, and holds no partial grapheme.
#[test]
fn copy_text_slices_the_ends_and_joins_rows_with_a_line_break() {
    let held = Held::new(
        0,
        [user("first line"), user("second"), user("thi\u{301}rd")],
    );
    let view = held.view();
    let sel = selection(at(held.body(0), &[0], 6), at(held.body(2), &[0], 5));
    assert_eq!(copy_text(&view, &sel), "line\nsecond\nthi\u{301}");
    // An offset inside a grapheme (between `i` and its combining accent) never splits it.
    let inside = selection(at(held.body(2), &[0], 0), at(held.body(2), &[0], 3));
    let copied = copy_text(&view, &inside);
    assert!(copied == "thi\u{301}" || copied == "th", "{copied:?}");
    // Backwards is the same range.
    let back = selection(at(held.body(2), &[0], 5), at(held.body(0), &[0], 6));
    assert_eq!(copy_text(&view, &back), "line\nsecond\nthi\u{301}");
}

// ---- T015: the state ----

fn app_with(events: impl IntoIterator<Item = StructuredEvent>) -> (App, SessionId) {
    let mut app = App::default();
    let s = SessionId::new();
    let _ = app.structured_events(s, 0, &seq(0, events), false, &GRAMMAR);
    (app, s)
}

fn send(app: &mut App, msg: SelectMsg) -> Vec<Outcome> {
    app.update_transcript(TranscriptMsg::Select(msg)).outcomes
}

fn id_of(app: &App, s: SessionId, n: usize) -> EntryId {
    app.structured_transcript
        .transcript(s)
        .expect("held")
        .entries()
        .nth(n)
        .expect("the entry")
        .id()
}

fn body(app: &App, s: SessionId, n: usize) -> RowPos {
    RowPos::Entry(id_of(app, s, n), Under::Body)
}

fn hit(session: SessionId, pos: TextPos, click: Click) -> SelectMsg {
    SelectMsg::Hit {
        session,
        pos,
        click,
        line: None,
    }
}

/// A row of a session's transcript, by the place of its entry or as the omitted notice.
#[derive(Debug, Clone)]
enum Row {
    Body(usize),
    Omitted,
}

/// A place in a row: the unit's path and an offset, resolved against the window when used.
#[derive(Debug, Clone)]
struct Spot(Row, Vec<u16>, usize);

/// `offset` into entry `n`'s first unit.
fn e(n: usize, offset: usize) -> Spot {
    Spot(Row::Body(n), vec![0], offset)
}

fn resolve(app: &App, s: SessionId, spot: &Spot) -> TextPos {
    let row = match spot.0 {
        Row::Body(n) => body(app, s, n),
        Row::Omitted => RowPos::Omitted,
    };
    at(row, &spot.1, spot.2)
}

/// Press, a hit at `from`, a hit at `to`, release: a drag.
fn drag(app: &mut App, s: SessionId, from: Spot, to: Spot) -> Vec<Outcome> {
    let (from, to) = (resolve(app, s, &from), resolve(app, s, &to));
    let mut outcomes = send(
        app,
        SelectMsg::Press {
            session: s,
            clicks: Click::Single,
        },
    );
    outcomes.extend(send(app, hit(s, from, Click::Single)));
    outcomes.extend(send(app, hit(s, to, Click::Single)));
    outcomes.extend(send(app, SelectMsg::Release { session: s }));
    outcomes
}

fn copied(app: &App, s: SessionId) -> Option<String> {
    let view = app.transcript_view(s)?;
    let sel = app.transcript_selection.selection(s)?;
    Some(copy_text(&view, sel))
}

fn live(
    app: &mut App,
    s: SessionId,
    first: u64,
    events: impl IntoIterator<Item = StructuredEvent>,
) {
    let _ = app.structured_events(s, 0, &seq(first, events), true, &GRAMMAR);
}

/// S-8, S-9 and the gesture: press, hits and release hold the selection and write nothing; a
/// click without a drag clears it and writes nothing.
#[test]
fn a_drag_holds_the_selection_and_a_click_clears_it_without_touching_the_clipboard() {
    let (mut app, s) = app_with([user("hello world"), user("second")]);
    let outcomes = drag(&mut app, s, e(0, 6), e(1, 3));
    assert!(
        outcomes.is_empty(),
        "release writes no clipboard: {outcomes:?}"
    );
    assert_eq!(copied(&app, s).as_deref(), Some("world\nsec"));
    assert!(app.transcript_selection.selecting(s));
    let outcomes = drag(&mut app, s, e(0, 2), e(0, 2));
    assert!(outcomes.is_empty());
    assert!(!app.transcript_selection.selecting(s));
    assert!(
        app.transcript_selection.selection(s).is_none(),
        "a click clears"
    );
}

/// A hit without a press (a move with no button the area saw go down) starts nothing.
#[test]
fn a_hit_without_a_press_starts_nothing() {
    let (mut app, s) = app_with([user("hello world")]);
    for offset in [2, 8] {
        let pos = resolve(&app, s, &e(0, offset));
        send(&mut app, hit(s, pos, Click::Single));
    }
    assert!(app.transcript_selection.selection(s).is_none());
}

/// SC-005, S-5: a full turn streamed after the selection leaves its copy text unchanged.
#[test]
fn a_streamed_turn_after_the_selection_leaves_it_unchanged() {
    let (mut app, s) = app_with([user("hello world"), text("an answer")]);
    drag(&mut app, s, e(0, 0), Spot(Row::Body(1), vec![0, 0], 2));
    let before = copied(&app, s).expect("selected");
    assert_eq!(before, "hello world\nan");
    live(
        &mut app,
        s,
        2,
        [
            text("more"),
            tool_use("toolu_x", "Bash", r#"{"command":"ls"}"#),
            tool_result("toolu_x", json!([{"type": "text", "text": "ok"}])),
            text("done"),
            result_event(),
        ],
    );
    assert_eq!(copied(&app, s), Some(before));
}

/// Growth of the last selected entry past the focus keeps the selection; text of an entry in
/// range replaced by different text clears it (#939 FR-015, research R7).
#[test]
fn growth_past_the_focus_keeps_it_and_replaced_text_in_range_clears_it() {
    let line = |v: serde_json::Value| micold_core::structured::parse_line(&v.to_string());
    let delta = |t: &str| {
        line(
            json!({"type": "stream_event", "event": {"type": "content_block_delta", "index": 0,
            "delta": {"type": "text_delta", "text": t}}}),
        )
    };
    let (mut app, s) = app_with([
        line(
            json!({"type": "stream_event", "event": {"type": "message_start",
            "message": {"id": "msg_1", "type": "message", "role": "assistant", "content": []}}}),
        ),
        line(
            json!({"type": "stream_event", "event": {"type": "content_block_start", "index": 0,
            "content_block": {"type": "text", "text": ""}}}),
        ),
        delta("Hello wor"),
    ]);
    let row = body(&app, s, 0);
    let path = {
        let view = app.transcript_view(s).expect("view");
        units(&row, &view)[0].path.segments().to_vec()
    };
    let _ = row;
    drag(
        &mut app,
        s,
        Spot(Row::Body(0), path.clone(), 0),
        Spot(Row::Body(0), path, 5),
    );
    assert_eq!(copied(&app, s).as_deref(), Some("Hello"));
    live(&mut app, s, 3, [delta("ld, again")]);
    assert_eq!(
        copied(&app, s).as_deref(),
        Some("Hello"),
        "growth after the focus"
    );
    live(
        &mut app,
        s,
        4,
        [line(
            json!({"type": "assistant", "message": {"id": "msg_1", "role": "assistant",
            "content": [{"type": "text", "text": "Goodbye, all"}]}}),
        )],
    );
    assert!(
        app.transcript_selection.selection(s).is_none(),
        "the selected text was replaced: {:?}",
        copied(&app, s)
    );
}

/// S-6: a rebuild clears the selection.
#[test]
fn a_rebuild_clears_it() {
    let (mut app, s) = app_with([user("hello world")]);
    drag(&mut app, s, e(0, 0), e(0, 5));
    let _ = app.structured_events(s, 0, &seq(0, [user("other history")]), false, &GRAMMAR);
    assert!(app.transcript_selection.selection(s).is_none());
}

/// S-6: the eviction of the anchor's entry clears it; an omitted-count change clears a selection
/// with an end in the notice and keeps one that has none there.
#[test]
fn eviction_of_an_end_or_a_changed_notice_under_an_end_clears_it() {
    let many: Vec<StructuredEvent> = (0..MAX_EVENTS).map(|i| user(&format!("m{i}"))).collect();
    let (mut app, s) = app_with(many.clone());
    drag(&mut app, s, e(0, 0), e(3, 1));
    live(&mut app, s, MAX_EVENTS as u64, [user("one more")]);
    assert!(
        app.transcript_selection.selection(s).is_none(),
        "the anchor left"
    );

    // The notice now reads "1 earlier event not shown".
    drag(
        &mut app,
        s,
        Spot(Row::Omitted, vec![0], 0),
        Spot(Row::Omitted, vec![0], 1),
    );
    assert!(app.transcript_selection.selecting(s));
    live(&mut app, s, MAX_EVENTS as u64 + 1, [user("again")]);
    assert!(
        app.transcript_selection.selection(s).is_none(),
        "the notice changed"
    );

    drag(&mut app, s, e(10, 0), e(12, 1));
    let before = copied(&app, s);
    live(&mut app, s, MAX_EVENTS as u64 + 2, [user("and again")]);
    assert_eq!(
        copied(&app, s),
        before,
        "no end in the notice, none evicted"
    );
}

/// S-6: a toggle of a card in range clears it; one outside keeps it.
#[test]
fn a_toggle_in_range_clears_it_and_one_outside_keeps_it() {
    let (mut app, s) = app_with([
        tool_use("toolu_a", "Bash", r#"{"command":"ls"}"#),
        user("between"),
        tool_use("toolu_b", "Bash", r#"{"command":"pwd"}"#),
    ]);
    drag(&mut app, s, e(0, 0), e(1, 3));
    app.update_transcript(TranscriptMsg::Toggle {
        session: s,
        key: EntryKey::Tool("toolu_b".into()),
    });
    assert!(app.transcript_selection.selecting(s), "outside the range");
    app.update_transcript(TranscriptMsg::Toggle {
        session: s,
        key: EntryKey::Tool("toolu_a".into()),
    });
    assert!(
        app.transcript_selection.selection(s).is_none(),
        "in the range"
    );
}

/// S-6: a request block appearing inside the range clears it; one at the end, outside it, keeps
/// it.
#[test]
fn a_prompt_row_appearing_in_range_clears_it_and_one_outside_keeps_it() {
    let (mut app, s) = app_with([
        tool_use("toolu_a", "Bash", r#"{"command":"ls"}"#),
        user("after"),
    ]);
    let shown = [TerminalRef {
        session: s,
        process: SessionProcess::Primary,
    }];
    drag(&mut app, s, e(0, 0), e(1, 3));
    app.permission_prompts
        .requested(request(s, 1, "toolu_gone", RequestKind::Tool));
    app.transcript_selection_sync(&[s], &shown);
    assert!(
        app.transcript_selection.selecting(s),
        "at the end, after the range"
    );
    app.permission_prompts
        .requested(request(s, 2, "toolu_a", RequestKind::Tool));
    app.transcript_selection_sync(&[s], &shown);
    assert!(
        app.transcript_selection.selection(s).is_none(),
        "under a card in the range"
    );
}

/// S-6: forgetting the session, or no pane showing it, clears it.
#[test]
fn forgetting_the_session_or_no_pane_showing_it_clears_it() {
    let (mut app, s) = app_with([user("hello world")]);
    drag(&mut app, s, e(0, 0), e(0, 5));
    app.forget_structured(s);
    assert!(app.transcript_selection.selection(s).is_none());

    let (mut app, s) = app_with([user("hello world")]);
    drag(&mut app, s, e(0, 0), e(0, 5));
    app.transcript_selection_sync(&[s], &[]);
    assert!(app.transcript_selection.selection(s).is_none());
}

/// S-7: two sessions never touch each other's selection or menu.
#[test]
fn two_sessions_keep_their_own_selection_and_menu() {
    let mut app = App::default();
    let (a, b) = (SessionId::new(), SessionId::new());
    for s in [a, b] {
        let _ = app.structured_events(s, 0, &seq(0, [user("hello world")]), false, &GRAMMAR);
    }
    drag(&mut app, a, e(0, 0), e(0, 5));
    send(
        &mut app,
        SelectMsg::Context {
            session: b,
            origin: (10, 20),
        },
    );
    assert!(app.transcript_selection.selecting(a));
    assert!(!app.transcript_selection.selecting(b));
    assert_eq!(app.transcript_selection.menu(a), None);
    assert_eq!(app.transcript_selection.menu(b), Some((10, 20)));
    send(&mut app, SelectMsg::SelectAll { session: b });
    assert_eq!(copied(&app, a).as_deref(), Some("hello"));
    app.forget_structured(b);
    assert!(app.transcript_selection.selecting(a));
}

/// S-7 across windows: each window holds its own selection state.
#[test]
fn two_windows_keep_their_own_selection() {
    let (mut one, s) = app_with([user("hello world")]);
    let mut two = App::default();
    let _ = two.structured_events(s, 0, &seq(0, [user("hello world")]), false, &GRAMMAR);
    drag(&mut one, s, e(0, 0), e(0, 5));
    assert!(one.transcript_selection.selecting(s));
    assert!(!two.transcript_selection.selecting(s));
}

/// S-10: select-all spans the first unit's start to the last unit's end; on an empty transcript
/// it does nothing.
#[test]
fn select_all_spans_everything_and_does_nothing_on_an_empty_transcript() {
    let (mut app, s) = app_with([user("hello"), user("world")]);
    send(&mut app, SelectMsg::SelectAll { session: s });
    assert_eq!(copied(&app, s).as_deref(), Some("hello\nworld"));
    let mut empty = App::default();
    let e = SessionId::new();
    let _ = empty.structured_events(e, 0, &[], false, &GRAMMAR);
    send(&mut empty, SelectMsg::SelectAll { session: e });
    assert!(empty.transcript_selection.selection(e).is_none());
}

/// S-11: a double click selects the word, a triple click the reported visual line in prose and
/// the source line in code and diffs; a drag after them extends by that unit.
#[test]
fn double_and_triple_clicks_select_a_word_and_a_line() {
    let (mut app, s) = app_with([
        user("alpha beta gamma\nsecond line"),
        text("```\nfn a() {}\nlet b;\n```\n"),
    ]);
    let row = body(&app, s, 0);
    send(
        &mut app,
        SelectMsg::Press {
            session: s,
            clicks: Click::Double,
        },
    );
    send(&mut app, hit(s, at(row, &[0], 7), Click::Double));
    send(&mut app, SelectMsg::Release { session: s });
    assert_eq!(copied(&app, s).as_deref(), Some("beta"));
    assert_eq!(
        app.transcript_selection
            .selection(s)
            .map(Selection::granularity),
        Some(Granularity::Word)
    );
    // A drag after a double click extends by words.
    send(
        &mut app,
        SelectMsg::Press {
            session: s,
            clicks: Click::Double,
        },
    );
    send(&mut app, hit(s, at(row, &[0], 7), Click::Double));
    send(&mut app, hit(s, at(row, &[0], 12), Click::Double));
    send(&mut app, SelectMsg::Release { session: s });
    assert_eq!(copied(&app, s).as_deref(), Some("beta gamma"));

    // Prose: the visual line the view reported.
    send(
        &mut app,
        SelectMsg::Press {
            session: s,
            clicks: Click::Triple,
        },
    );
    send(
        &mut app,
        SelectMsg::Hit {
            session: s,
            pos: at(row, &[0], 7),
            click: Click::Triple,
            line: Some(6..11),
        },
    );
    send(&mut app, SelectMsg::Release { session: s });
    assert_eq!(copied(&app, s).as_deref(), Some("beta "));

    // Code: the source line, whatever line the view reported.
    let code = body(&app, s, 1);
    let path = {
        let view = app.transcript_view(s).expect("view");
        let unit = units(&code, &view)
            .into_iter()
            .find(|u| u.source)
            .expect("a code unit");
        unit.path.segments().to_vec()
    };
    send(
        &mut app,
        SelectMsg::Press {
            session: s,
            clicks: Click::Triple,
        },
    );
    send(
        &mut app,
        SelectMsg::Hit {
            session: s,
            pos: at(code, &path, 2),
            click: Click::Triple,
            line: Some(0..3),
        },
    );
    send(&mut app, SelectMsg::Release { session: s });
    assert_eq!(copied(&app, s).as_deref(), Some("fn a() {}"));
}

/// M-3: the menu opens at its origin and closes, neither changing the selection.
#[test]
fn the_menu_opens_and_closes_without_changing_the_selection() {
    let (mut app, s) = app_with([user("hello world")]);
    drag(&mut app, s, e(0, 0), e(0, 5));
    send(
        &mut app,
        SelectMsg::Context {
            session: s,
            origin: (3, 4),
        },
    );
    assert_eq!(app.transcript_selection.menu(s), Some((3, 4)));
    assert_eq!(copied(&app, s).as_deref(), Some("hello"));
    send(&mut app, SelectMsg::MenuDismissed { session: s });
    assert_eq!(app.transcript_selection.menu(s), None);
    assert_eq!(copied(&app, s).as_deref(), Some("hello"));
}

// ---- T018: panes ----

/// S-6 with two panes: switching one pane to another session clears that session's selection
/// and keeps the other pane's; a swap keeps both; another project's selections are untouched.
#[test]
fn switching_a_pane_clears_its_sessions_selection_only() {
    let mut app = App::default();
    let (a, b, c, other) = (
        SessionId::new(),
        SessionId::new(),
        SessionId::new(),
        SessionId::new(),
    );
    for s in [a, b, c, other] {
        let _ = app.structured_events(s, 0, &seq(0, [user("hello world")]), false, &GRAMMAR);
        drag(&mut app, s, e(0, 0), e(0, 5));
    }
    let primary = |session| TerminalRef {
        session,
        process: SessionProcess::Primary,
    };
    let project = [a, b, c];
    // Two panes show `a` and `b`; `c` is shown by none of them; `other` is another project's.
    app.transcript_selection_sync(&project, &[primary(a), primary(b)]);
    assert!(app.transcript_selection.selecting(a));
    assert!(app.transcript_selection.selecting(b));
    assert!(!app.transcript_selection.selecting(c));
    assert!(app.transcript_selection.selecting(other));
    // A swap: both still shown.
    app.transcript_selection_sync(&project, &[primary(b), primary(a)]);
    assert!(app.transcript_selection.selecting(a));
    assert!(app.transcript_selection.selecting(b));
    // The second pane switches from `b` to `c`; a pane on `b`'s shell does not count.
    let shell = TerminalRef {
        session: b,
        process: SessionProcess::Shell(micold_core::session::ShellInstanceId(1)),
    };
    app.transcript_selection_sync(&project, &[primary(a), primary(c), shell]);
    assert!(app.transcript_selection.selecting(a));
    assert!(!app.transcript_selection.selecting(b));
    assert!(app.transcript_selection.selecting(other));
}

// ---- T022: copy ----

/// S-12: a copy of a non-empty selection writes its copy text with a selection ticket and keeps
/// the selection; an empty one writes nothing.
#[test]
fn copy_writes_the_selection_checked_and_keeps_it() {
    let (mut app, s) = app_with([user("hello world")]);
    assert!(send(&mut app, SelectMsg::CopySelection { session: s }).is_empty());
    drag(&mut app, s, e(0, 0), e(0, 5));
    let outcomes = send(&mut app, SelectMsg::CopySelection { session: s });
    let [Outcome::ClipboardWriteChecked { text, ticket }] = outcomes.as_slice() else {
        panic!("one checked write: {outcomes:?}");
    };
    assert_eq!(text, "hello");
    assert_eq!(copied(&app, s).as_deref(), Some("hello"), "kept");
    // A held read-back changes nothing visible and asks for no timer.
    let effect = app.update_transcript(TranscriptMsg::CopyChecked {
        ticket: *ticket,
        read: Some("hello".into()),
    });
    assert!(effect.outcomes.is_empty());
    assert_eq!(
        app.composers.composer(s).and_then(|c| c.notice.clone()),
        None
    );
}

/// FR-014: a failed or unreadable read-back puts "Copy failed" on that session's composer and
/// keeps the selection.
#[test]
fn a_failed_copy_says_so_in_the_pane_and_keeps_the_selection() {
    for read in [Some("something else".to_owned()), None] {
        let (mut app, s) = app_with([user("hello world")]);
        drag(&mut app, s, e(0, 0), e(0, 5));
        let outcomes = send(&mut app, SelectMsg::CopySelection { session: s });
        let Some(Outcome::ClipboardWriteChecked { ticket, .. }) = outcomes.first() else {
            panic!("a checked write: {outcomes:?}");
        };
        app.update_transcript(TranscriptMsg::CopyChecked {
            ticket: *ticket,
            read,
        });
        assert_eq!(
            app.composers
                .composer(s)
                .and_then(|c| c.notice.clone())
                .as_deref(),
            Some(COPY_FAILED)
        );
        assert_eq!(copied(&app, s).as_deref(), Some("hello"));
    }
}

/// M-2, FR-019a: Copy in the menu gives the transcript the keyboard, closes the menu and copies.
#[test]
fn the_menus_copy_focuses_the_transcript_and_copies() {
    let (mut app, s) = app_with([user("hello world")]);
    drag(&mut app, s, e(0, 6), e(0, 11));
    send(
        &mut app,
        SelectMsg::Context {
            session: s,
            origin: (1, 1),
        },
    );
    let outcomes = send(&mut app, SelectMsg::CopySelection { session: s });
    assert!(matches!(
        outcomes.as_slice(),
        [Outcome::ClipboardWriteChecked { text, .. }] if text == "world"
    ));
    assert_eq!(app.window.focused_field, Some(FieldId::Transcript(s)));
    assert_eq!(app.transcript_selection.menu(s), None);
}

/// Forgetting a session gives up the keyboard its transcript held.
#[test]
fn forgetting_a_session_releases_its_transcripts_focus() {
    let (mut app, s) = app_with([user("hello world")]);
    drag(&mut app, s, e(0, 0), e(0, 5));
    send(&mut app, SelectMsg::CopySelection { session: s });
    assert_eq!(app.window.focused_field, Some(FieldId::Transcript(s)));
    app.forget_structured(s);
    assert_eq!(app.window.focused_field, None);
}
