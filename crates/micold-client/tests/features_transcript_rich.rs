//! T017–T019 (feature 940, data-model.md *Render cache*, research R5, R7, R8, R11;
//! contracts/rich-renderers.md RR-4–RR-6): each window keeps a render-free cache per structured
//! session that derives documents and diffs from the transcript, prunes with it, says which parts
//! to highlight, copies code with a read-back check, and opens web and mail links only.

use std::collections::{HashMap, HashSet};

use micold_client::features::structured_transcript::{State, TranscriptMsg};
use micold_client::features::transcript_rich::{
    Checked, CopyState, CopyTarget, DerivedBody, Grammar, HighlightJob, Part, RichCache,
    COPY_CONFIRM,
};
use micold_client::features::{OpenRequest, Outcome};
use micold_core::markdown::{CodeId, Document};
use micold_core::review::diff::{SchemeSpans, SideSpans, Spans};
use micold_core::review::limits::{MAX_CHANGED_LINES, MAX_VERSION_BYTES};
use micold_core::session::SessionId;
use micold_core::structured::edit_diff::edit_diff;
use micold_core::structured::transcript::{Entry, EntryKey, EntryKind, Transcript};
use micold_core::structured::{ContentBlock, SequencedEvent, StructuredEvent};
use micold_core::tokens::Rgb;
use serde_json::json;

mod support;

// ---- fixtures ----

/// A recorded stream-json fixture of the core's, numbered from `0` as a history holds it.
fn recorded(name: &str) -> Vec<SequencedEvent> {
    support::transcript::sequenced(&support::transcript::fixture_lines(name), 0)
}

fn sequenced(events: impl IntoIterator<Item = StructuredEvent>) -> Vec<SequencedEvent> {
    events
        .into_iter()
        .zip(0u64..)
        .map(|(event, seq)| SequencedEvent { seq, event })
        .collect()
}

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

fn tool_use(id: &str, name: &str, input: &str) -> StructuredEvent {
    assistant(ContentBlock::ToolUse {
        id: id.into(),
        name: name.into(),
        input: input.into(),
    })
}

fn edit(path: &str, old: &str, new: &str) -> String {
    json!({"file_path": path, "old_string": old, "new_string": new}).to_string()
}

fn transcript_of(events: impl IntoIterator<Item = StructuredEvent>) -> Transcript {
    let mut t = Transcript::new();
    t.apply(0, &sequenced(events), false);
    t
}

fn whole(name: &str) -> Transcript {
    let mut t = Transcript::new();
    t.apply(0, &recorded(name), false);
    t
}

/// The fake grammar the cache is handed: it knows Rust code blocks and `.txt` files only, so the
/// tests need no grammar set (T017).
fn knows(token: &str) -> bool {
    matches!(token, "rust" | "txt")
}

fn token(path: &str) -> String {
    path.rsplit_once('.')
        .map_or(path, |(_, extension)| extension)
        .to_owned()
}

const GRAMMAR: Grammar = Grammar { knows, token };

/// Spans for `job` in both schemes, one line of colour per line.
fn spans_for(job: &HighlightJob) -> Spans {
    let colour = Rgb { r: 1, g: 2, b: 3 };
    let side = SideSpans(job.lines.iter().map(|_| vec![(0..1, colour)]).collect());
    let scheme = SchemeSpans {
        old: SideSpans::default(),
        new: side,
    };
    Spans {
        light: scheme.clone(),
        dark: scheme,
    }
}

fn derived_kinds(t: &Transcript) -> impl Iterator<Item = &Entry> {
    t.entries().filter(|e| match e.kind() {
        EntryKind::AssistantText { .. } | EntryKind::Thinking { .. } => true,
        EntryKind::ToolCall(card) => matches!(card.name.as_str(), "Edit" | "MultiEdit" | "Write"),
        _ => false,
    })
}

fn text_doc<'a>(cache: &'a RichCache, s: SessionId, key: &EntryKey) -> &'a Document {
    match &cache.get(s, key).expect("derived").body {
        DerivedBody::Text(doc) => doc,
        other => panic!("not text: {other:?}"),
    }
}

fn job_parts(jobs: &[HighlightJob]) -> Vec<(EntryKey, Part)> {
    jobs.iter().map(|j| (j.key.clone(), j.part)).collect()
}

// ---- T017: derive, prune, isolate, highlight jobs ----

#[test]
fn text_and_thinking_become_documents_and_edit_cards_diffs() {
    let s = SessionId::new();
    for name in ["markdown-turn", "edit-turn"] {
        let t = whole(name);
        let mut cache = RichCache::default();
        cache.refresh(s, &t, &GRAMMAR);
        let (mut texts, mut diffs) = (0, 0);
        for entry in t.entries() {
            let derived = cache.get(s, entry.key());
            match entry.kind() {
                EntryKind::AssistantText {
                    text, streaming, ..
                }
                | EntryKind::Thinking { text, streaming } => {
                    let d = derived.expect("text is derived");
                    assert_eq!(d.revision, entry.revision());
                    assert_eq!(
                        d.body,
                        DerivedBody::Text(Document::parse(text, *streaming)),
                        "{name}"
                    );
                    texts += 1;
                }
                EntryKind::ToolCall(card) if derived_kinds(&t).any(|e| e.key() == entry.key()) => {
                    let d = derived.expect("an edit card is derived");
                    let diff = edit_diff(&card.name, &card.input);
                    assert!(
                        diff.is_some(),
                        "{name}: the recorded {} is complete",
                        card.name
                    );
                    assert_eq!(d.body, DerivedBody::Diff(diff));
                    diffs += 1;
                }
                // Feature 1550: any other tool card is held for its copy controls only.
                EntryKind::ToolCall(_) => {
                    assert_eq!(
                        derived.expect("a tool card is held").body,
                        DerivedBody::Tool
                    );
                }
                _ => assert!(derived.is_none(), "{name}: nothing else is derived"),
            }
        }
        assert!(texts > 0, "{name}: derived {texts} texts");
        if name == "edit-turn" {
            assert!(diffs >= 3, "the recorded Write and Edits: {diffs}");
        }
    }
}

/// #943: a command's output and a compaction's summary are markdown, as an answer is.
#[test]
fn command_output_and_a_compaction_summary_become_documents() {
    let s = SessionId::new();
    let synthetic = StructuredEvent::Assistant {
        error: None,
        model: Some("<synthetic>".into()),
        message_id: None,
        raw: json!({"type": "assistant", "message": {"content": []}}).to_string(),
        blocks: vec![ContentBlock::Text {
            text: "## Context".into(),
        }],
        parent_tool_use_id: None,
    };
    let boundary = |trigger: &str| StructuredEvent::CompactBoundary {
        trigger: Some(trigger.into()),
        pre_tokens: None,
        post_tokens: None,
        raw: "{}".into(),
    };
    let t = transcript_of([
        StructuredEvent::UserSent {
            text: "/context".into(),
            attachments: vec![],
        },
        synthetic,
        boundary("manual"),
        StructuredEvent::User {
            blocks: vec![ContentBlock::Text {
                text: "**summary**".into(),
            }],
            parent_tool_use_id: None,
            is_synthetic: true,
            raw: "{}".into(),
        },
        boundary("auto"),
    ]);
    let mut cache = RichCache::default();
    cache.refresh(s, &t, &GRAMMAR);
    let mut documents = 0;
    for entry in t.entries() {
        let derived = cache.get(s, entry.key());
        match entry.kind() {
            EntryKind::CommandOutput { text }
            | EntryKind::Compacted {
                summary: Some(text),
                ..
            } => {
                assert_eq!(
                    derived.expect("derived").body,
                    DerivedBody::Text(Document::parse(text, false))
                );
                documents += 1;
            }
            EntryKind::ToolCall(_) => {
                assert_eq!(derived.expect("held").body, DerivedBody::Tool);
            }
            _ => assert!(derived.is_none(), "{:?}", entry.kind()),
        }
    }
    assert_eq!(
        documents, 2,
        "the output and the first summary; no summary, no document"
    );
}

#[test]
fn an_edit_card_has_no_diff_while_its_input_streams_or_when_it_is_malformed() {
    let s = SessionId::new();
    let mut cache = RichCache::default();
    let mut t = Transcript::new();
    let mut streamed = 0;
    for event in recorded("edit-turn") {
        t.apply(0, std::slice::from_ref(&event), true);
        cache.refresh(s, &t, &GRAMMAR);
        for entry in t.entries() {
            if let EntryKind::ToolCall(card) = entry.kind() {
                if card.streaming && card.name != "Bash" {
                    assert_eq!(
                        cache.get(s, entry.key()).map(|d| &d.body),
                        Some(&DerivedBody::Diff(None)),
                        "seq {}",
                        event.seq
                    );
                    streamed += 1;
                }
            }
        }
    }
    assert!(streamed > 0, "the recording streams an edit's input");

    let t = transcript_of([tool_use("m", "Edit", r#"{"file_path": 1}"#)]);
    cache.refresh(s, &t, &GRAMMAR);
    assert_eq!(
        cache.get(s, &EntryKey::Tool("m".into())).map(|d| &d.body),
        Some(&DerivedBody::Diff(None)),
        "malformed input falls back"
    );
}

#[test]
fn a_refresh_after_a_delta_re_derives_only_the_entry_it_changed() {
    let (mut s, mut cache, mut t) = (SessionId::new(), RichCache::default(), Transcript::new());
    let mut changed_while_others_held = 0;
    for event in ["markdown-turn", "edit-turn"]
        .into_iter()
        .flat_map(recorded)
    {
        if event.seq == 0 {
            (s, cache, t) = (SessionId::new(), RichCache::default(), Transcript::new());
        }
        let before: HashMap<EntryKey, (u64, _)> = t
            .entries()
            .filter_map(|e| {
                Some((
                    e.key().clone(),
                    (e.revision(), cache.get(s, e.key())?.clone()),
                ))
            })
            .collect();
        t.apply(0, std::slice::from_ref(&event), true);
        cache.refresh(s, &t, &GRAMMAR);
        for entry in derived_kinds(&t) {
            let now = cache.get(s, entry.key()).expect("derived");
            assert_eq!(now.revision, entry.revision(), "seq {}", event.seq);
            if let Some((revision, was)) = before.get(entry.key()) {
                if *revision == entry.revision() {
                    assert_eq!(
                        now, was,
                        "seq {}: an unchanged entry is untouched",
                        event.seq
                    );
                } else if before.len() > 1 {
                    changed_while_others_held += 1;
                }
            }
        }
    }
    assert!(
        changed_while_others_held > 0,
        "a delta grew one entry of several"
    );
}

#[test]
fn keys_the_transcript_no_longer_holds_are_pruned() {
    let s = SessionId::new();
    let mut cache = RichCache::default();
    let both = transcript_of([text("one"), text("two")]);
    cache.refresh(s, &both, &GRAMMAR);
    let keys: Vec<EntryKey> = both.entries().map(|e| e.key().clone()).collect();
    assert!(keys.iter().all(|k| cache.get(s, k).is_some()));
    let other = transcript_of([tool_use(
        "x",
        "Write",
        r#"{"file_path":"a.txt","content":"a\n"}"#,
    )]);
    cache.refresh(s, &other, &GRAMMAR);
    assert!(keys.iter().all(|k| cache.get(s, k).is_none()), "pruned");
    assert!(cache.get(s, &EntryKey::Tool("x".into())).is_some());
}

#[test]
fn sessions_and_windows_share_nothing() {
    let (a, b) = (SessionId::new(), SessionId::new());
    let t = transcript_of([text("```rust\nfn a() {}\n```\n")]);
    let key = t.entries().next().unwrap().key().clone();
    let mut cache = RichCache::default();
    let mut other_window = RichCache::default();
    assert_eq!(cache.refresh(a, &t, &GRAMMAR).len(), 1);
    assert_eq!(
        cache.refresh(b, &t, &GRAMMAR).len(),
        1,
        "b asks for its own"
    );
    assert_eq!(
        other_window.refresh(a, &t, &GRAMMAR).len(),
        1,
        "so does another window"
    );
    assert_eq!(
        text_doc(&cache, a, &key),
        text_doc(&other_window, a, &key),
        "two windows on one session draw the same formatting"
    );
    let code = text_doc(&cache, a, &key).code_blocks().next().unwrap().0;
    let entry = t.entries().next().unwrap();
    cache
        .copy(a, entry, CopyTarget::Code(code))
        .expect("copies");
    assert!(cache.get(b, &key).unwrap().copy.is_empty());
    assert!(other_window.get(a, &key).unwrap().copy.is_empty());
    cache.forget(a);
    assert!(cache.get(a, &key).is_none(), "forgotten");
    assert!(cache.get(b, &key).is_some(), "the other session stays");
    assert!(
        other_window.get(a, &key).is_some(),
        "the other window stays"
    );
}

#[test]
fn edit_counts_are_the_diffs_added_and_removed_lines() {
    let s = SessionId::new();
    let t = transcript_of([
        tool_use("e", "Edit", &edit("a.txt", "one\ntwo\n", "one\n2\n3\n")),
        text("hi"),
    ]);
    let mut cache = RichCache::default();
    cache.refresh(s, &t, &GRAMMAR);
    assert_eq!(
        cache.edit_counts(s, &EntryKey::Tool("e".into())),
        Some((2, 1))
    );
    let text_key = t.entries().nth(1).unwrap().key().clone();
    assert_eq!(cache.edit_counts(s, &text_key), None);
}

#[test]
fn jobs_are_issued_once_for_settled_known_code_and_complete_known_diffs() {
    let s = SessionId::new();
    let mut cache = RichCache::default();
    let mut t = Transcript::new();
    let mut asked: HashSet<(EntryKey, Part)> = HashSet::new();
    for event in recorded("markdown-turn") {
        t.apply(0, std::slice::from_ref(&event), true);
        for job in cache.refresh(s, &t, &GRAMMAR) {
            let Part::Code(id) = job.part else {
                panic!("no diffs in a text turn")
            };
            let doc = text_doc(&cache, s, &job.key);
            let (_, code, settled) = doc.code_blocks().find(|(c, ..)| *c == id).unwrap();
            assert!(
                settled,
                "seq {}: only settled code is highlighted",
                event.seq
            );
            assert_eq!(code.token.as_deref(), Some("rust"), "only a known language");
            assert_eq!(job.token, "rust");
            assert_eq!(
                job.lines,
                code.code.lines().map(str::to_owned).collect::<Vec<_>>()
            );
            assert!(asked.insert((job.key.clone(), job.part)), "once each");
        }
    }
    let (mut rust, mut other) = (0, 0);
    for entry in derived_kinds(&t) {
        for (_, code, settled) in text_doc(&cache, s, entry.key()).code_blocks() {
            assert!(settled, "the finished turn is settled");
            match code.token.as_deref() {
                Some("rust") => rust += 1,
                _ => other += 1,
            }
        }
    }
    assert!(rust > 0 && other > 0, "the turn has Rust and teapot blocks");
    assert_eq!(asked.len(), rust, "every Rust block, none of the others");
    assert!(cache.refresh(s, &t, &GRAMMAR).is_empty(), "nothing new");

    let diffs = transcript_of([
        tool_use("k", "Edit", &edit("a.txt", "a\n", "b\n")),
        tool_use("u", "Edit", &edit("a.md", "a\n", "b\n")),
    ]);
    let jobs = cache.refresh(s, &diffs, &GRAMMAR);
    assert_eq!(
        job_parts(&jobs),
        vec![(EntryKey::Tool("k".into()), Part::Diff)]
    );
    assert_eq!(jobs[0].token, "txt");
    assert_eq!(jobs[0].lines, vec!["a".to_owned(), "b".to_owned()]);
}

#[test]
fn spans_hold_both_schemes_so_a_theme_switch_asks_for_nothing() {
    let s = SessionId::new();
    let t = transcript_of([text("```rust\nfn a() {}\nfn b() {}\n```\n")]);
    let key = t.entries().next().unwrap().key().clone();
    let mut cache = RichCache::default();
    let jobs = cache.refresh(s, &t, &GRAMMAR);
    let job = &jobs[0];
    let stamp = cache.get(s, &key).unwrap().stamp;
    assert!(cache.highlighted(job.session, &job.key, job.part, spans_for(job)));
    let d = cache.get(s, &key).unwrap();
    assert_eq!(d.stamp, stamp + 1, "the arrival bumps the stamp");
    let spans = &d.spans[&job.part];
    assert_eq!(spans.light.new.0.len(), 2);
    assert_eq!(spans.dark.new.0.len(), 2);
    assert!(
        cache.refresh(s, &t, &GRAMMAR).is_empty(),
        "no new job for either scheme"
    );
}

#[test]
fn spans_for_a_part_that_is_gone_are_not_stored() {
    let s = SessionId::new();
    let mut t = transcript_of([text("```rust\nfn a() {}\n```\n")]);
    let key = t.entries().next().unwrap().key().clone();
    let mut cache = RichCache::default();
    let job = cache.refresh(s, &t, &GRAMMAR).remove(0);
    assert!(
        !cache.highlighted(s, &key, Part::Diff, spans_for(&job)),
        "no such part"
    );
    // A fresh history (a rebuild) holds other text under the same key: the code block is gone.
    t.apply(0, &sequenced([text("other")]), false);
    cache.refresh(s, &t, &GRAMMAR);
    assert!(cache.get(s, &key).is_some(), "the entry is still there");
    assert!(
        !cache.highlighted(s, &key, job.part, spans_for(&job)),
        "its code block is not"
    );
    assert!(cache.get(s, &key).unwrap().spans.is_empty());
    t.apply(0, &sequenced([tool_use("t", "Bash", "{}")]), false);
    cache.refresh(s, &t, &GRAMMAR);
    assert!(
        !cache.highlighted(s, &key, job.part, spans_for(&job)),
        "the entry left"
    );
    assert!(cache.get(s, &key).is_none());
}

#[test]
fn a_part_over_the_limits_waits_for_show_in_full_in_its_session_only() {
    let (s, other) = (SessionId::new(), SessionId::new());
    let long: String = (0..=MAX_CHANGED_LINES)
        .map(|n| format!("let v{n} = {n};\n"))
        .collect();
    let wide = "x".repeat(MAX_VERSION_BYTES as usize + 1);
    let t = transcript_of([
        text(&format!("```rust\n{long}```\n")),
        tool_use(
            "w",
            "Write",
            &json!({"file_path": "a.txt", "content": wide}).to_string(),
        ),
    ]);
    let code_key = t.entries().next().unwrap().key().clone();
    let mut cache = RichCache::default();
    assert!(
        cache.refresh(s, &t, &GRAMMAR).is_empty(),
        "over a limit: no job"
    );
    assert!(cache.refresh(other, &t, &GRAMMAR).is_empty());
    let code = Part::Code(
        text_doc(&cache, s, &code_key)
            .code_blocks()
            .next()
            .unwrap()
            .0,
    );
    assert!(cache.get(s, &code_key).unwrap().is_large(code));

    let stamp = cache.get(s, &code_key).unwrap().stamp;
    let job = cache.show_in_full(s, &code_key, code).expect("its job");
    assert_eq!((job.session, &job.key, job.part), (s, &code_key, code));
    assert_eq!(job.lines.len(), MAX_CHANGED_LINES as usize + 1);
    let d = cache.get(s, &code_key).unwrap();
    assert!(d.full.contains(&code) && !d.is_large(code));
    assert_eq!(d.stamp, stamp + 1);
    assert!(
        cache.show_in_full(s, &code_key, code).is_none(),
        "asked once"
    );
    assert!(
        cache.get(other, &code_key).unwrap().full.is_empty(),
        "this session only"
    );

    let write = EntryKey::Tool("w".into());
    let job = cache
        .show_in_full(s, &write, Part::Diff)
        .expect("the Write's job");
    assert_eq!(job.token, "txt");
    assert!(cache.refresh(s, &t, &GRAMMAR).is_empty());
}

// ---- T018: copy ----

fn copy_setup(md: &str) -> (State, RichCache, SessionId, EntryKey) {
    let s = SessionId::new();
    let mut state = State::default();
    let mut cache = RichCache::default();
    let events = sequenced([text(md)]);
    state.apply(s, 0, &events, false);
    cache.refresh(s, state.transcript(s).unwrap(), &GRAMMAR);
    let key = state
        .transcript(s)
        .unwrap()
        .entries()
        .next()
        .unwrap()
        .key()
        .clone();
    (state, cache, s, key)
}

fn copy(
    state: &mut State,
    cache: &mut RichCache,
    s: SessionId,
    key: &EntryKey,
    code: CodeId,
) -> (String, micold_client::features::transcript_rich::Ticket) {
    let effect = state.update(
        cache,
        TranscriptMsg::Copy {
            session: s,
            key: key.clone(),
            target: CopyTarget::Code(code),
        },
    );
    match effect.outcomes.as_slice() {
        [Outcome::ClipboardWriteChecked { text, ticket }] => (text.clone(), *ticket),
        other => panic!("one checked write, got {other:?}"),
    }
}

#[test]
fn a_copy_writes_the_code_byte_for_byte() {
    let md = "```rust\nfn a() {\n\tx\n}\n```\n\n    indented\n      more\n\n```\n```\n";
    let (mut state, mut cache, s, key) = copy_setup(md);
    let codes: Vec<(CodeId, String)> = text_doc(&cache, s, &key)
        .code_blocks()
        .map(|(id, c, _)| (id, c.code.clone()))
        .collect();
    assert_eq!(
        codes.iter().map(|(_, c)| c.as_str()).collect::<Vec<_>>(),
        ["fn a() {\n\tx\n}", "indented\n  more", ""],
        "the code without fence, tag or indent, its last line break the fence's"
    );
    for (id, code) in codes {
        let (text, ticket) = copy(&mut state, &mut cache, s, &key, id);
        assert_eq!(text, code, "SC-005: byte for byte");
        assert_eq!(
            cache.get(s, &key).unwrap().copy[&CopyTarget::Code(id)],
            CopyState::Pending(ticket)
        );
    }
}

#[test]
fn a_copy_of_a_streaming_block_writes_the_code_so_far() {
    let s = SessionId::new();
    let mut state = State::default();
    let mut cache = RichCache::default();
    for event in recorded("markdown-turn") {
        state.apply(s, 0, std::slice::from_ref(&event), true);
        cache.refresh(s, state.transcript(s).unwrap(), &GRAMMAR);
        let open = state.transcript(s).unwrap().entries().find_map(|e| {
            let EntryKind::AssistantText {
                streaming: true, ..
            } = e.kind()
            else {
                return None;
            };
            let doc = text_doc(&cache, s, e.key());
            let (id, code, _) = doc
                .code_blocks()
                .find(|(_, c, _)| c.open && !c.code.is_empty())?;
            Some((e.key().clone(), id, code.code.clone()))
        });
        if let Some((key, id, so_far)) = open {
            let (text, _) = copy(&mut state, &mut cache, s, &key, id);
            assert_eq!(text, so_far, "US3 s4: the code received so far");
            return;
        }
    }
    panic!("the recording streams a code block");
}

#[test]
fn the_read_back_decides_copied_or_failed_and_asks_for_the_timer() {
    let (mut state, mut cache, s, key) = copy_setup("```\none\n```\n\n```\ntwo\n```\n");
    let ids: Vec<CodeId> = text_doc(&cache, s, &key)
        .code_blocks()
        .map(|c| c.0)
        .collect();
    for (read, id, expected) in [
        (Some("one".to_owned()), ids[0], "copied"),
        (Some("something else".to_owned()), ids[1], "failed"),
        (None, ids[1], "failed"),
    ] {
        let (_, ticket) = copy(&mut state, &mut cache, s, &key, id);
        let stamp = cache.get(s, &key).unwrap().stamp;
        let effect = state.update(&mut cache, TranscriptMsg::CopyChecked { ticket, read });
        assert_eq!(
            effect.outcomes,
            vec![Outcome::CopyConfirmTimer {
                ticket,
                after: COPY_CONFIRM
            }]
        );
        let d = cache.get(s, &key).unwrap();
        assert_eq!(d.stamp, stamp + 1);
        let want = match expected {
            "copied" => CopyState::Copied(ticket),
            _ => CopyState::Failed(ticket),
        };
        assert_eq!(d.copy[&CopyTarget::Code(id)], want);
    }
    assert_eq!(COPY_CONFIRM, std::time::Duration::from_secs(2));
}

#[test]
fn only_the_current_tickets_expiry_returns_the_control_to_idle() {
    let (mut state, mut cache, s, key) = copy_setup("```\none\n```\n");
    let id = text_doc(&cache, s, &key).code_blocks().next().unwrap().0;
    let (_, first) = copy(&mut state, &mut cache, s, &key, id);
    state.update(
        &mut cache,
        TranscriptMsg::CopyChecked {
            ticket: first,
            read: Some("one".into()),
        },
    );
    let (_, second) = copy(&mut state, &mut cache, s, &key, id);
    assert_ne!(first, second, "tickets rise");
    let checked = state.update(
        &mut cache,
        TranscriptMsg::CopyChecked {
            ticket: second,
            read: Some("one".into()),
        },
    );
    assert_eq!(checked.outcomes.len(), 1);
    let before = cache.get(s, &key).unwrap().clone();
    let stale = state.update(&mut cache, TranscriptMsg::CopyExpired { ticket: first });
    assert!(stale.outcomes.is_empty());
    assert_eq!(
        cache.get(s, &key).unwrap(),
        &before,
        "a stale timer changes nothing"
    );
    state.update(&mut cache, TranscriptMsg::CopyExpired { ticket: second });
    let d = cache.get(s, &key).unwrap();
    assert!(d.copy.is_empty(), "idle again");
    assert_eq!(d.stamp, before.stamp + 1);
}

#[test]
fn copy_state_is_per_session_entry_and_block() {
    let md = "```\none\n```\n\n```\ntwo\n```\n";
    let (mut state, mut cache, s, key) = copy_setup(md);
    let other = SessionId::new();
    state.apply(other, 0, &sequenced([text(md), text(md)]), false);
    cache.refresh(other, state.transcript(other).unwrap(), &GRAMMAR);
    let ids: Vec<CodeId> = text_doc(&cache, s, &key)
        .code_blocks()
        .map(|c| c.0)
        .collect();
    copy(&mut state, &mut cache, s, &key, ids[0]);
    let d = cache.get(s, &key).unwrap();
    assert_eq!(d.copy.len(), 1, "one block");
    assert!(!d.copy.contains_key(&CopyTarget::Code(ids[1])));
    for e in state.transcript(other).unwrap().entries() {
        assert!(cache.get(other, e.key()).unwrap().copy.is_empty());
    }
}

// ---- T019: links ----

#[test]
fn only_web_and_mail_links_open() {
    let mut state = State::default();
    let mut cache = RichCache::default();
    for url in [
        "http://example.com",
        "https://example.com/a?b#c",
        "mailto:someone@example.com",
    ] {
        let effect = state.update(&mut cache, TranscriptMsg::OpenLink { url: url.into() });
        assert_eq!(
            effect.outcomes,
            vec![Outcome::OpenLink(OpenRequest::Url(url.into()))],
            "{url}"
        );
    }
    for url in [
        "javascript:alert(1)",
        "file:///etc/passwd",
        "docs/readme.md",
        "#anchor",
        "",
    ] {
        let effect = state.update(&mut cache, TranscriptMsg::OpenLink { url: url.into() });
        assert!(effect.outcomes.is_empty(), "{url} opens nothing");
        assert_eq!(effect.scroll, None);
    }
}

// ---- #1550 T022: a selection's copy beside the code blocks' (research R9, S-12, C-6) ----

/// A selection's copy takes a ticket of its own and answers its read-back once, as held or
/// failed; it touches no code block's control.
#[test]
fn a_selection_copy_is_checked_once_and_touches_no_control() {
    let s = SessionId::new();
    let t = transcript_of([text("```rust\nfn a() {}\n```\n")]);
    let mut cache = RichCache::default();
    cache.refresh(s, &t, &GRAMMAR);
    let key = t.entries().next().unwrap().key().clone();
    let ticket = cache.copy_selection(s, "fn".into());
    assert_eq!(
        cache.copy_checked(ticket, Some("fn".into())),
        Checked::Selection {
            session: s,
            held: true
        }
    );
    assert_eq!(
        cache.copy_checked(ticket, Some("fn".into())),
        Checked::Stale
    );
    for read in [Some("other".to_owned()), None] {
        let ticket = cache.copy_selection(s, "fn".into());
        assert_eq!(
            cache.copy_checked(ticket, read),
            Checked::Selection {
                session: s,
                held: false
            }
        );
    }
    assert!(
        cache.get(s, &key).unwrap().copy.is_empty(),
        "no control changed"
    );
    // A code block's copy is keyed by its target and still wants its confirmation timer.
    let code = text_doc(&cache, s, &key).code_blocks().next().unwrap().0;
    let entry = t.entries().next().unwrap();
    let (_, ticket) = cache
        .copy(s, entry, CopyTarget::Code(code))
        .expect("copies");
    assert_eq!(
        cache
            .get(s, &key)
            .unwrap()
            .copy
            .get(&CopyTarget::Code(code)),
        Some(&CopyState::Pending(ticket))
    );
    assert_eq!(
        cache.copy_checked(ticket, Some("fn a() {}".into())),
        Checked::Control
    );
    // A selection copy outlives a refresh of its session's entries.
    let kept = cache.copy_selection(s, "x".into());
    cache.refresh(s, &t, &GRAMMAR);
    assert_eq!(
        cache.copy_checked(kept, Some("x".into())),
        Checked::Selection {
            session: s,
            held: true
        }
    );
}

// ---- T044 (feature 1550): the tool cards' and diffs' copy controls ----

/// A result of tool use `id` whose content is the block list `parts`.
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

/// `events` applied to one session, refreshed into a fresh cache.
fn session_of(events: Vec<StructuredEvent>) -> (State, RichCache, SessionId) {
    let s = SessionId::new();
    let mut state = State::default();
    let mut cache = RichCache::default();
    state.apply(s, 0, &sequenced(events), false);
    cache.refresh(s, state.transcript(s).unwrap(), &GRAMMAR);
    (state, cache, s)
}

/// The key of `s`'s tool card `id`.
fn card_key(state: &State, s: SessionId, id: &str) -> EntryKey {
    state
        .transcript(s)
        .unwrap()
        .entries()
        .find(|e| matches!(e.kind(), EntryKind::ToolCall(card) if card.id == id))
        .expect("the card")
        .key()
        .clone()
}

/// What a `TranscriptMsg::Copy` of `target` on `key` writes, with its ticket; `None` when it
/// writes nothing.
fn copy_of(
    state: &mut State,
    cache: &mut RichCache,
    s: SessionId,
    key: &EntryKey,
    target: CopyTarget,
) -> Option<(String, micold_client::features::transcript_rich::Ticket)> {
    let effect = state.update(
        cache,
        TranscriptMsg::Copy {
            session: s,
            key: key.clone(),
            target,
        },
    );
    match effect.outcomes.as_slice() {
        [] => None,
        [Outcome::ClipboardWriteChecked { text, ticket }] => Some((text.clone(), *ticket)),
        other => panic!("at most one checked write, got {other:?}"),
    }
}

/// C-1, FR-015, SC-004: the header copies `work_value` uncut, on all its lines, and a tool
/// without one has nothing to copy.
#[test]
fn the_header_copies_the_uncut_work_value_and_only_when_there_is_one() {
    let command = format!(
        "cargo test \\\n  --workspace {}\n\techo done",
        "x".repeat(300)
    );
    let bash = json!({"command": command, "description": "run"}).to_string();
    let (mut state, mut cache, s) = session_of(vec![
        tool_use("b", "Bash", &bash),
        tool_use("t", "TodoWrite", r#"{"todos":[]}"#),
    ]);
    let key = card_key(&state, s, "b");
    let (text, ticket) = copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolValue)
        .expect("a Bash card's command copies");
    assert_eq!(text, command, "byte for byte, not the one-line summary");
    assert_eq!(
        cache.get(s, &key).unwrap().copy[&CopyTarget::ToolValue],
        CopyState::Pending(ticket)
    );
    let todo = card_key(&state, s, "t");
    assert_eq!(
        micold_core::structured::transcript::work_value("TodoWrite", r#"{"todos":[]}"#),
        None
    );
    assert!(
        copy_of(&mut state, &mut cache, s, &todo, CopyTarget::ToolValue).is_none(),
        "no work value: nothing to copy"
    );
}

/// C-2, FR-016: the input copies `ToolCard::input` as held; the result copies its parts' text
/// joined with `\n`, an unshown part as its caption, and exists only once a result exists.
#[test]
fn the_input_and_result_copy_what_the_card_holds() {
    let input = r#"{"file_path": "/src/a.rs",   "limit": 5}"#;
    let (mut state, mut cache, s) = session_of(vec![tool_use("r", "Read", input)]);
    let key = card_key(&state, s, "r");
    let (text, _) =
        copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolInput).expect("the input");
    assert_eq!(text, input, "as held, spacing and all");
    assert!(
        copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolResult).is_none(),
        "no result yet"
    );
    let parts = json!([
        {"type": "text", "text": "one\n\ttwo"},
        {"type": "image", "source": {"type": "base64", "data": ""}},
        {"type": "text", "text": "three"}
    ]);
    let (mut state, mut cache, s) =
        session_of(vec![tool_use("r", "Read", input), tool_result("r", parts)]);
    let key = card_key(&state, s, "r");
    let (text, _) =
        copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolResult).expect("the result");
    assert_eq!(text, "one\n\ttwo\n[image not shown]\nthree");
}

/// SC-004: the input copies byte for byte, never trimmed or reflowed.
#[test]
fn an_input_with_edge_whitespace_copies_untrimmed() {
    let (mut state, mut cache, s) =
        session_of(vec![tool_use("r", "Bash", "  {\"command\":\n \"ls\"}\t\n")]);
    let key = card_key(&state, s, "r");
    let (text, _) =
        copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolInput).expect("the input");
    assert_eq!(text, "  {\"command\":\n \"ls\"}\t\n");
}

/// US3 scenario 6: while the input streams, the input copies the text received so far.
#[test]
fn a_streaming_input_copies_the_text_so_far() {
    let s = SessionId::new();
    let mut state = State::default();
    let mut cache = RichCache::default();
    for event in recorded("tool-turn") {
        state.apply(s, 0, std::slice::from_ref(&event), true);
        cache.refresh(s, state.transcript(s).unwrap(), &GRAMMAR);
        let open = state
            .transcript(s)
            .unwrap()
            .entries()
            .find_map(|e| match e.kind() {
                EntryKind::ToolCall(card) if card.streaming && !card.input.is_empty() => {
                    Some(e.key().clone())
                }
                _ => None,
            });
        if let Some(key) = open {
            let (text, _) = copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolInput)
                .expect("a streaming input copies");
            assert_eq!(text, "{\"file_path\": \"<CWD", "the input received so far");
            return;
        }
    }
    panic!("the recording streams a tool input");
}

/// C-3, FR-017: a diff's path copies `EditDiff::path`; a card without a diff has none.
#[test]
fn a_diffs_path_copies_the_edits_file_path() {
    let path = "/home/u/a long/dir/src/lib.rs";
    let (mut state, mut cache, s) = session_of(vec![
        tool_use("e", "Edit", &edit(path, "a", "b")),
        tool_use("b", "Bash", r#"{"command":"ls"}"#),
    ]);
    let key = card_key(&state, s, "e");
    let (text, _) =
        copy_of(&mut state, &mut cache, s, &key, CopyTarget::DiffPath).expect("the path");
    assert_eq!(text, path);
    let bash = card_key(&state, s, "b");
    assert!(copy_of(&mut state, &mut cache, s, &bash, CopyTarget::DiffPath).is_none());
}

/// C-5: an empty value copies the empty string.
#[test]
fn an_empty_value_copies_the_empty_string() {
    let (mut state, mut cache, s) = session_of(vec![
        tool_use("b", "Bash", r#"{"command":""}"#),
        tool_result("b", json!([{"type": "text", "text": ""}])),
    ]);
    let key = card_key(&state, s, "b");
    for target in [CopyTarget::ToolValue, CopyTarget::ToolResult] {
        let (text, _) = copy_of(&mut state, &mut cache, s, &key, target).expect("copies");
        assert_eq!(text, "", "{target:?}");
    }
}

/// C-4, FR-018: a control's state is keyed by session, entry and target in its window only; the
/// read-back decides it and it expires after `COPY_CONFIRM` as a code block's does.
#[test]
fn a_controls_confirmation_is_its_own() {
    let events = || {
        vec![
            tool_use("b", "Bash", r#"{"command":"ls"}"#),
            tool_result("b", json!([{"type": "text", "text": "a.rs"}])),
            tool_use("c", "Bash", r#"{"command":"pwd"}"#),
        ]
    };
    let (mut state, mut cache, s) = session_of(events());
    let other = SessionId::new();
    state.apply(other, 0, &sequenced(events()), false);
    cache.refresh(other, state.transcript(other).unwrap(), &GRAMMAR);
    let mut other_window = cache.clone();
    let key = card_key(&state, s, "b");
    let (_, ticket) =
        copy_of(&mut state, &mut cache, s, &key, CopyTarget::ToolResult).expect("copies");
    let checked = state.update(
        &mut cache,
        TranscriptMsg::CopyChecked {
            ticket,
            read: Some("a.rs".into()),
        },
    );
    assert_eq!(
        checked.outcomes,
        vec![Outcome::CopyConfirmTimer {
            ticket,
            after: COPY_CONFIRM
        }]
    );
    let d = cache.get(s, &key).unwrap();
    assert_eq!(
        d.copy,
        HashMap::from([(CopyTarget::ToolResult, CopyState::Copied(ticket))]),
        "only the activated control confirms"
    );
    let c = card_key(&state, s, "c");
    assert!(
        cache.get(s, &c).unwrap().copy.is_empty(),
        "not another card"
    );
    assert!(
        cache.get(other, &key).unwrap().copy.is_empty(),
        "not another session"
    );
    assert!(
        other_window.get(s, &key).unwrap().copy.is_empty(),
        "not another window"
    );
    let failed = copy_of(
        &mut state,
        &mut other_window,
        s,
        &key,
        CopyTarget::ToolValue,
    )
    .expect("copies")
    .1;
    assert_eq!(
        other_window.copy_checked(failed, Some("elsewhere".into())),
        Checked::Control
    );
    assert_eq!(
        other_window.get(s, &key).unwrap().copy[&CopyTarget::ToolValue],
        CopyState::Failed(failed)
    );
    state.update(&mut cache, TranscriptMsg::CopyExpired { ticket });
    assert!(cache.get(s, &key).unwrap().copy.is_empty(), "idle again");
}
