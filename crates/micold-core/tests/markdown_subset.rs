//! Feature 940 — the markdown model over the synthetic subset fixture and the recorded
//! `markdown-turn` reply (contracts/markdown-model.md *Rules*, M-1, M-5, M-6; FR-001, FR-003,
//! FR-004, FR-007, FR-011 at the model; SC-001).

use micold_core::markdown::{Align, Block, CodeBlock, Document, Link, ListItem, Span, Style};

#[path = "support/markdown.rs"]
mod markdown;
use markdown::{assert_nothing_hidden, plain, prefixes, spans, subset, turn_text, Rng};

fn finished(text: &str) -> Document {
    Document::parse(text, false)
}

fn headings(blocks: &[Block]) -> Vec<(u8, String)> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Heading { level, text } => Some((*level, plain(text))),
            _ => None,
        })
        .collect()
}

fn paragraphs(blocks: &[Block]) -> Vec<String> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::Paragraph { text } => Some(plain(text)),
            _ => None,
        })
        .collect()
}

fn paragraph_with<'a>(blocks: &'a [Block], needle: &str) -> &'a Vec<Span> {
    blocks
        .iter()
        .find_map(|b| match b {
            Block::Paragraph { text } if plain(text).contains(needle) => Some(text),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a paragraph holds {needle:?}: {:#?}", paragraphs(blocks)))
}

fn lists(blocks: &[Block]) -> Vec<(Option<u64>, bool, &Vec<ListItem>)> {
    blocks
        .iter()
        .filter_map(|b| match b {
            Block::List {
                start,
                tight,
                items,
            } => Some((*start, *tight, items)),
            _ => None,
        })
        .collect()
}

fn item_text(item: &ListItem) -> String {
    match item.blocks.first() {
        Some(Block::Paragraph { text }) => plain(text),
        other => panic!("an item starts with its text, got {other:?}"),
    }
}

fn codes(document: &Document) -> Vec<CodeBlock> {
    document.code_blocks().map(|(_, c, _)| c.clone()).collect()
}

fn span_with<'a>(spans: &'a [&'a Span], text: &str) -> &'a Span {
    spans
        .iter()
        .copied()
        .find(|s| s.text == text)
        .unwrap_or_else(|| panic!("a span reads {text:?}"))
}

fn link(target: &str, activatable: bool) -> Option<Link> {
    Some(Link {
        target: target.to_string(),
        activatable,
    })
}

// ---------------------------------------------------------------------------------------
// The subset fixture: each row of the contract's *Rules* maps to its model.

#[test]
fn atx_and_setext_headings_keep_their_level() {
    let doc = finished(&subset());
    assert_eq!(
        headings(doc.blocks()),
        [
            (1, "Heading one"),
            (2, "Setext heading two"),
            (3, "Heading three"),
            (4, "Heading four"),
            (5, "Heading five"),
            (6, "Heading six"),
        ]
        .map(|(l, t)| (l, t.to_string()))
    );
}

#[test]
fn emphasis_strikethrough_and_inline_code_are_styled_spans() {
    let doc = finished(&subset());
    let all = spans(doc.blocks());
    let style = |text| span_with(&all, text).style;
    assert_eq!(
        style("bold"),
        Style {
            bold: true,
            ..Style::default()
        }
    );
    assert_eq!(
        style("italic"),
        Style {
            italic: true,
            ..Style::default()
        }
    );
    assert_eq!(
        style("struck"),
        Style {
            strike: true,
            ..Style::default()
        }
    );
    assert_eq!(
        style("inline code"),
        Style {
            code: true,
            ..Style::default()
        }
    );
}

#[test]
fn a_soft_break_is_a_space_and_a_hard_break_a_line_break() {
    let doc = finished(&subset());
    let text = plain(paragraph_with(doc.blocks(), "a soft"));
    assert!(text.contains("a soft break,"), "{text:?}");
    assert!(
        text.contains("a hard break here\nthen the next line"),
        "{text:?}"
    );
}

#[test]
fn web_and_mail_links_are_activatable_and_others_are_not() {
    let doc = finished(&subset());
    let all = spans(doc.blocks());
    assert_eq!(
        span_with(&all, "the web").link,
        link("https://example.com/web", true)
    );
    assert_eq!(
        span_with(&all, "mail").link,
        link("mailto:team@example.com", true)
    );
    assert_eq!(
        span_with(&all, "https://example.com/auto").link,
        link("https://example.com/auto", true)
    );
    assert_eq!(
        span_with(&all, "https://example.com/bare").link,
        link("https://example.com/bare", true),
        "a bare address in text is a link (R3)"
    );
    assert_eq!(
        span_with(&all, "script").link,
        link("javascript:alert", false)
    );
    assert_eq!(span_with(&all, "relative").link, link("docs/readme", false));
    assert_eq!(
        span_with(&all, "a file").link,
        link("file:///etc/hosts", false)
    );
}

#[test]
fn an_image_is_its_description_and_address_and_no_link() {
    let doc = finished(&subset());
    let all = spans(doc.blocks());
    let image = span_with(&all, "a teapot (https://example.com/teapot.png)");
    assert_eq!(image.link, None, "nothing to fetch or open (FR-004)");
}

#[test]
fn raw_html_stays_literal_text() {
    let doc = finished(&subset());
    let block = plain(paragraph_with(doc.blocks(), "<div"));
    assert_eq!(block, "<div class=\"note\">\nraw HTML block\n</div>");
    let inline = plain(paragraph_with(doc.blocks(), "Inline"));
    assert_eq!(inline, "Inline <span>raw html</span> stays literal.");
}

#[test]
fn a_reference_defined_in_a_later_block_stays_literal() {
    let doc = finished(&subset());
    let text = paragraph_with(doc.blocks(), "later reference");
    assert!(plain(text).contains("[later reference][later]"), "{text:?}");
    assert!(text.iter().all(|s| s.link.is_none()), "{text:?}");
    let definition = plain(paragraph_with(doc.blocks(), "[later]:"));
    assert_eq!(
        definition, "[later]: https://example.com/later",
        "nothing hidden (FR-001)"
    );
}

#[test]
fn lists_keep_their_start_tightness_nesting_and_task_marks() {
    let doc = finished(&subset());
    let lists = lists(doc.blocks());
    let (start, tight, items) = lists[0];
    assert_eq!((start, tight, items.len()), (None, true, 3));
    assert_eq!(item_text(&items[1]), "bullet two");
    let Some(Block::List { items: nested, .. }) = items[1].blocks.get(1) else {
        panic!("bullet two holds a nested list: {:?}", items[1]);
    };
    assert_eq!(item_text(&nested[0]), "nested bullet");
    assert!(
        matches!(nested[0].blocks.get(1), Some(Block::List { .. })),
        "two levels deep"
    );

    let (start, tight, items) = lists[1];
    assert_eq!((start, tight), (Some(3), true));
    assert_eq!(
        items.iter().map(item_text).collect::<Vec<_>>(),
        ["third", "fourth"]
    );

    let (start, tight, items) = lists[2];
    assert_eq!(
        (start, tight, items.len()),
        (Some(1), false, 2),
        "a loose list"
    );

    let (_, _, items) = lists[3];
    assert_eq!(
        items
            .iter()
            .map(|i| (i.task, item_text(i)))
            .collect::<Vec<_>>(),
        [
            (Some(true), "done task".to_string()),
            (Some(false), "open task".to_string())
        ]
    );
}

#[test]
fn a_quote_holds_its_blocks_and_rules_are_rules() {
    let doc = finished(&subset());
    let quote = doc
        .blocks()
        .iter()
        .find_map(|b| match b {
            Block::Quote { blocks } => Some(blocks),
            _ => None,
        })
        .expect("a quote");
    assert_eq!(paragraphs(quote), ["A quote with bold"]);
    assert!(
        matches!(quote.last(), Some(Block::Quote { .. })),
        "a nested quote: {quote:?}"
    );
    let rules = doc.blocks().iter().filter(|b| **b == Block::Rule).count();
    assert_eq!(rules, 2);
}

#[test]
fn a_table_has_its_alignment_header_and_rows_and_without_separator_is_a_paragraph() {
    let doc = finished(&subset());
    let (align, header, rows) = doc
        .blocks()
        .iter()
        .find_map(|b| match b {
            Block::Table {
                align,
                header,
                rows,
            } => Some((align, header, rows)),
            _ => None,
        })
        .expect("a table");
    assert_eq!(
        align,
        &[Align::Left, Align::Center, Align::Right, Align::None]
    );
    assert_eq!(
        header.iter().map(plain).collect::<Vec<_>>(),
        ["Left", "Centre", "Right", "Plain"]
    );
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[1].iter().map(plain).collect::<Vec<_>>(),
        ["long cell text", "code", "l", ""]
    );
    assert_eq!(rows[1][2][0].link, link("https://example.com/cell", true));
    let tables = doc
        .blocks()
        .iter()
        .filter(|b| matches!(b, Block::Table { .. }))
        .count();
    assert_eq!(tables, 1, "no separator row, no table");
    assert_eq!(
        plain(paragraph_with(doc.blocks(), "separator")),
        "| no | separator | | just | pipes |"
    );
}

#[test]
fn unmatched_emphasis_stays_literal() {
    let doc = finished(&subset());
    let text = paragraph_with(doc.blocks(), "unmatched");
    assert_eq!(plain(text), "An unmatched **bold opener stays literal.");
    assert!(text.iter().all(|s| s.style == Style::default()));
}

#[test]
fn code_blocks_hold_exactly_their_code_tag_and_token() {
    let doc = finished(&subset());
    let codes = codes(&doc);
    let view: Vec<(Option<&str>, Option<&str>, &str, bool)> = codes
        .iter()
        .map(|c| {
            (
                c.tag.as_deref(),
                c.token.as_deref(),
                c.code.as_str(),
                c.open,
            )
        })
        .collect();
    assert_eq!(
        view,
        [
            (None, None, "indented code\n  keeps its indent", false),
            (
                Some("rust"),
                Some("rust"),
                "fn main() {\n    println!(\"hi\");\n}",
                false
            ),
            (
                Some("teapot-lang"),
                Some("teapot-lang"),
                "brew black",
                false
            ),
            (None, None, "untagged", false),
            (Some("python"), Some("python"), "print(\"tilde\")", false),
            (
                Some("sh"),
                Some("sh"),
                "echo \"this fence is never closed\"\nstill code",
                true
            ),
        ]
    );
    for code in &codes {
        assert_eq!(code.bytes, code.code.len() as u64);
        assert_eq!(code.lines, code.code.lines().count() as u32);
    }
}

#[test]
fn code_ids_are_document_order_ordinals_and_found_by_id() {
    let doc = finished(&subset());
    for (n, (id, code, settled)) in doc.code_blocks().enumerate() {
        assert_eq!(id.ordinal(), n as u32);
        assert_eq!(code.id, id);
        assert!(settled, "a finished document is settled");
        assert_eq!(doc.code(id), Some(code));
    }
    assert_eq!(doc.code_blocks().count(), 6);
}

#[test]
fn crlf_text_gives_the_blocks_lf_text_gives() {
    let text = subset();
    assert!(text.contains("\r\n"), "the fixture holds CRLF text");
    let lf = text.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    let doc = finished(&text);
    assert_eq!(doc.blocks(), finished(&lf).blocks());
    assert_eq!(doc.blocks(), finished(&crlf).blocks());
    assert!(paragraphs(doc.blocks())
        .contains(&"A paragraph written with CRLF line endings".to_string()));
}

#[test]
fn nothing_received_is_hidden() {
    for (name, text) in [("subset", subset()), ("markdown-turn", turn_text())] {
        assert_nothing_hidden(&text, &finished(&text), name);
        assert_nothing_hidden(&text, &Document::parse(&text, true), name);
    }
}

// ---------------------------------------------------------------------------------------
// The recorded reply.

#[test]
fn the_recorded_reply_maps_every_element() {
    let doc = finished(&turn_text());
    let blocks = doc.blocks();
    assert_eq!(
        headings(blocks).iter().map(|(l, _)| *l).collect::<Vec<_>>(),
        [1, 2, 3]
    );
    let all = spans(blocks);
    assert!(all.iter().any(|s| s.style.bold));
    assert!(all.iter().any(|s| s.style.italic));
    assert!(all.iter().any(|s| s.style.strike));
    assert!(all.iter().any(|s| s.style.code));
    assert!(all
        .iter()
        .any(|s| s.link == link("https://example.com", true)));
    let lists = lists(blocks);
    assert!(
        lists
            .iter()
            .any(|(_, _, items)| items.iter().any(|i| i.blocks.len() > 1)),
        "a nested list"
    );
    assert!(
        lists.iter().any(|(start, _, _)| *start == Some(1)),
        "a numbered list"
    );
    assert!(lists
        .iter()
        .any(|(_, _, items)| items.iter().any(|i| i.task == Some(true))));
    assert!(lists
        .iter()
        .any(|(_, _, items)| items.iter().any(|i| i.task == Some(false))));
    assert!(blocks.iter().any(|b| matches!(b, Block::Quote { .. })));
    assert!(blocks.contains(&Block::Rule));
    assert!(blocks.iter().any(|b| matches!(b, Block::Table { align, .. }
        if align == &[Align::Left, Align::Center, Align::Right])));
    let tags: Vec<Option<String>> = codes(&doc).into_iter().map(|c| c.tag).collect();
    assert_eq!(
        tags,
        [Some("rust".into()), Some("teapot-lang".into()), None]
    );
    assert!(codes(&doc)
        .iter()
        .all(|c| !c.open && !c.code.contains("```")));
}

// ---------------------------------------------------------------------------------------
// M-6: code ids are stable as the text grows.

#[test]
fn a_code_id_names_the_same_block_in_every_longer_prefix() {
    for text in [subset(), turn_text()] {
        let full = finished(&text);
        let full_codes = codes(&full);
        assert!(full_codes.len() >= 3, "the fixture has code blocks");
        for prefix in prefixes(&text) {
            let doc = Document::parse(prefix, true);
            for (n, (id, code, settled)) in doc.code_blocks().enumerate() {
                assert_eq!(id.ordinal(), n as u32, "ordinals in document order");
                let later = &full_codes[n];
                if settled {
                    assert_eq!(code, later, "a settled code block is final");
                } else {
                    let tag = |c: &CodeBlock| c.tag.clone().unwrap_or_default();
                    assert!(tag(later).starts_with(&tag(code)), "id {n}: its tag grows");
                    // A closing fence still arriving (`` `` ``) reads as code until it is one.
                    let so_far = code.code.trim_end_matches(['`', '~']).trim_end();
                    assert!(later.code.starts_with(so_far), "id {n}: open code grows");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// M-5: no input panics or drops text.

const ALPHABET: &[&str] = &[
    "a",
    "b",
    "z",
    "Q",
    "7",
    " ",
    " ",
    "\n",
    "\n",
    "\r\n",
    "\t",
    "*",
    "_",
    "~",
    "`",
    "#",
    ">",
    "-",
    "+",
    "|",
    "[",
    "]",
    "(",
    ")",
    "!",
    "<",
    "/",
    ":",
    ".",
    "=",
    "é",
    "漢",
    "🍵",
    "https://x.io",
];

#[test]
fn random_cuts_and_random_text_never_fail_or_hide_text() {
    let texts = [subset(), turn_text()];
    let mut rng = Rng::new(0x940_5eed);
    for round in 0..10_000 {
        let text = if round % 3 == 2 {
            (0..rng.below(120))
                .map(|_| ALPHABET[rng.below(ALPHABET.len())])
                .collect::<String>()
        } else {
            let source = &texts[round % 3];
            let mut a = rng.below(source.len() + 1);
            let mut b = rng.below(source.len() + 1);
            if a > b {
                std::mem::swap(&mut a, &mut b);
            }
            while !source.is_char_boundary(a) {
                a -= 1;
            }
            while !source.is_char_boundary(b) {
                b -= 1;
            }
            source[a..b].to_string()
        };
        let streaming = rng.below(2) == 0;
        let doc = Document::parse(&text, streaming);
        assert_nothing_hidden(&text, &doc, &format!("round {round}"));
    }
}
