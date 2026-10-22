//! Feature 940 — the markdown model while text streams (contracts/markdown-model.md M-2–M-4;
//! FR-005–FR-008, SC-002): settled blocks are final, the incremental document equals a fresh
//! parse, and the finished text gives the streamed blocks.

use std::collections::HashMap;

use micold_core::markdown::Document;
use micold_core::structured::transcript::{EntryKey, EntryKind, Transcript};

#[path = "support/markdown.rs"]
mod markdown;
use markdown::{events_of, lines_of, prefixes, subset, turn_text};

fn texts() -> [(&'static str, String); 2] {
    [("subset", subset()), ("markdown-turn", turn_text())]
}

/// M-2. Checked between each prefix and the next one, with `settled` never falling: a settled
/// block equal in the next document stays equal in every longer one, by induction.
#[test]
fn a_settled_block_is_final_in_every_longer_prefix() {
    for (name, text) in texts() {
        let mut before: Option<Document> = None;
        for prefix in prefixes(&text) {
            let doc = Document::parse(prefix, true);
            if let Some(before) = &before {
                let settled = before.settled();
                assert!(
                    doc.settled() >= settled,
                    "{name} at {}: settled blocks never become open again",
                    prefix.len()
                );
                assert_eq!(
                    &doc.blocks()[..settled],
                    &before.blocks()[..settled],
                    "{name} at {}: a settled block is final (FR-006)",
                    prefix.len()
                );
            }
            assert!(doc.settled() <= doc.blocks().len());
            before = Some(doc);
        }
        let last = before.expect("a prefix");
        assert!(
            last.settled() + 2 >= last.blocks().len() && last.blocks().len() >= 10,
            "{name}: blocks settle as the text grows, {} of {}",
            last.settled(),
            last.blocks().len()
        );
    }
}

/// M-3 and M-4.
#[test]
fn the_incremental_document_equals_a_fresh_parse_and_the_finished_one() {
    for (name, text) in texts() {
        let mut doc = Document::parse("", true);
        for prefix in prefixes(&text) {
            doc.update(prefix, true);
            assert_eq!(
                doc,
                Document::parse(prefix, true),
                "{name} at {}: incremental = fresh (FR-005)",
                prefix.len()
            );
        }
        let streamed = doc.blocks().to_vec();
        assert!(streamed.len() >= 10, "{name}: the text has its blocks");
        doc.update(&text, false);
        assert_eq!(
            doc,
            Document::parse(&text, false),
            "{name}: finished = fresh"
        );
        assert_eq!(
            doc.settled(),
            doc.blocks().len(),
            "{name}: all settled when finished"
        );
        assert_eq!(
            doc.blocks(),
            streamed,
            "{name}: finished = streamed (FR-008)"
        );
    }
}

#[test]
fn a_text_that_does_not_extend_the_old_one_is_parsed_afresh() {
    let text = subset();
    let mut doc = Document::parse(&text, true);
    doc.update("# Another reply\n\nwith other text", true);
    assert_eq!(
        doc,
        Document::parse("# Another reply\n\nwith other text", true)
    );
    doc.update("", false);
    assert_eq!(doc, Document::parse("", false));
    assert!(doc.blocks().is_empty());
}

/// SC-002's replay: `markdown-turn.jsonl` one event at a time through #939's `Transcript`, the
/// assistant text after each applied event through one document per entry.
#[test]
fn replaying_the_recorded_turn_delta_by_delta_gives_the_same_documents() {
    let events = events_of(&lines_of("markdown-turn"));
    let mut transcript = Transcript::new();
    let mut documents: HashMap<EntryKey, Document> = HashMap::new();
    let mut finished = 0;
    let mut deltas = 0;
    for event in &events {
        transcript.apply(0, std::slice::from_ref(event), true);
        for entry in transcript.entries() {
            let EntryKind::AssistantText {
                text, streaming, ..
            } = entry.kind()
            else {
                continue;
            };
            let doc = documents
                .entry(entry.key().clone())
                .or_insert_with(|| Document::parse("", true));
            let before = doc.clone();
            doc.update(text, *streaming);
            assert_eq!(*doc, Document::parse(text, *streaming), "seq {}", event.seq);
            if before != *doc {
                deltas += 1;
            }
            if !streaming && before.settled() < before.blocks().len() {
                assert_eq!(
                    doc.blocks(),
                    before.blocks(),
                    "the finished message leaves the blocks unchanged (FR-008)"
                );
                finished += 1;
            }
        }
    }
    assert!(
        deltas > 10,
        "the reply streamed in many deltas, got {deltas}"
    );
    assert_eq!(
        finished, 1,
        "the finished message replaced the streamed text once"
    );
}
