//! Feature 940 — the markdown fixtures and checks shared by `markdown_subset.rs` and
//! `markdown_stream.rs` (each `#[path]`-includes this file).

#![allow(dead_code)]

use micold_core::markdown::{Block, Document, Inlines, Span};
use micold_core::structured::transcript::{EntryKind, Transcript};

#[path = "stream_json.rs"]
mod stream_json;
pub use stream_json::{events_of, lines_of, without_stream_events};

/// The synthetic subset fixture, as written (it holds one CRLF section on purpose).
pub fn subset() -> String {
    let path = format!(
        "{}/tests/fixtures/stream-json/markdown-subset.md",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture {path} reads: {e}"))
}

/// The assistant text of the recorded `markdown-turn.jsonl`, from its finished message.
pub fn turn_text() -> String {
    let events = events_of(&without_stream_events(&lines_of("markdown-turn")));
    let mut transcript = Transcript::new();
    transcript.apply(0, &events, false);
    let texts: Vec<String> = transcript
        .entries()
        .filter_map(|e| match e.kind() {
            EntryKind::AssistantText { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts.len(),
        1,
        "markdown-turn holds one assistant text block"
    );
    texts.into_iter().next().unwrap()
}

/// Every char-boundary prefix of `text`, the empty one and `text` itself included.
pub fn prefixes(text: &str) -> impl Iterator<Item = &str> {
    (0..=text.len())
        .filter(|&i| text.is_char_boundary(i))
        .map(|i| &text[..i])
}

/// The text of a run of spans.
pub fn plain(spans: &Inlines) -> String {
    spans.iter().map(|s| s.text.as_str()).collect()
}

/// Every span of `blocks`, nested ones included, in document order.
pub fn spans(blocks: &[Block]) -> Vec<&Span> {
    let mut out = Vec::new();
    walk_spans(blocks, &mut out);
    out
}

fn walk_spans<'a>(blocks: &'a [Block], out: &mut Vec<&'a Span>) {
    for block in blocks {
        match block {
            Block::Heading { text, .. } | Block::Paragraph { text } => out.extend(text),
            Block::List { items, .. } => {
                for item in items {
                    walk_spans(&item.blocks, out);
                }
            }
            Block::Quote { blocks } => walk_spans(blocks, out),
            Block::Rule | Block::Code(_) => {}
            Block::Table { header, rows, .. } => {
                for cell in header.iter().chain(rows.iter().flatten()) {
                    out.extend(cell);
                }
            }
        }
    }
}

/// What the model shows, in document order: each span's text and link target, each code block's
/// tag and code.
pub fn model_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    walk_text(blocks, &mut out);
    out
}

fn push_spans(spans: &Inlines, out: &mut String) {
    for span in spans {
        out.push_str(&span.text);
        if let Some(link) = &span.link {
            out.push_str(&link.target);
        }
    }
}

fn walk_text(blocks: &[Block], out: &mut String) {
    for block in blocks {
        match block {
            Block::Heading { text, .. } | Block::Paragraph { text } => push_spans(text, out),
            Block::List { items, .. } => {
                for item in items {
                    walk_text(&item.blocks, out);
                }
            }
            Block::Quote { blocks } => walk_text(blocks, out),
            Block::Rule => {}
            Block::Code(code) => {
                out.push_str(code.tag.as_deref().unwrap_or(""));
                out.push_str(&code.code);
            }
            Block::Table { header, rows, .. } => {
                for cell in header.iter().chain(rows.iter().flatten()) {
                    push_spans(cell, out);
                }
            }
        }
    }
}

/// The characters of `source` that are not formatting: no whitespace, no ASCII punctuation, and
/// none of an ordered-list number, a task mark's `x` or an entity's name (formatting syntax, like
/// the punctuation around them).
pub fn received(source: &str) -> Vec<char> {
    let mut out = Vec::new();
    for line in source.split_inclusive(['\n', '\r']) {
        let body = info_word_only(&strip_markers(line));
        let mut chars = body.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '&' {
                let rest: String = chars.clone().take(33).collect();
                if let Some(end) = rest.find(';') {
                    let name = &rest[..end];
                    let entity = name.strip_prefix('#').map_or_else(
                        || !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric()),
                        |n| {
                            let n = n.strip_prefix(['x', 'X']).unwrap_or(n);
                            !n.is_empty() && n.chars().all(|c| c.is_ascii_hexdigit())
                        },
                    );
                    if entity {
                        for _ in 0..=end {
                            chars.next();
                        }
                        continue;
                    }
                }
            }
            out.push(c);
        }
    }
    out.retain(|c| !c.is_whitespace() && !c.is_ascii_punctuation());
    out
}

/// A fence line with its info string cut to the first word: the model shows the tag, and the rest
/// (`{.numbered}`, `,ignore`) is fence syntax, not text (data-model.md *CodeBlock*).
fn info_word_only(line: &str) -> String {
    let body = line.trim_start_matches([' ', '\t', '>']);
    let Some(fence) = ["```", "~~~"].iter().find(|f| body.starts_with(**f)) else {
        return line.to_string();
    };
    let info = body.trim_start_matches(fence.chars().next().unwrap());
    let word = info
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("");
    format!("{}{fence}{word}\n", &line[..line.len() - body.len()])
}

/// `line` without its container and list markers' digits and its task mark's `x`.
fn strip_markers(line: &str) -> String {
    let mut rest = line;
    let mut kept = String::new();
    loop {
        let trimmed = rest.trim_start_matches([' ', '\t', '>']);
        kept.push_str(&rest[..rest.len() - trimmed.len()]);
        rest = trimmed;
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let after = &rest[digits..];
        let ordered = (1..=9).contains(&digits) && after.starts_with(['.', ')']);
        let bullet = digits == 0 && rest.starts_with(['-', '*', '+']);
        let marker = if ordered {
            digits + 1
        } else {
            usize::from(bullet)
        };
        let ends = rest[marker..].trim_start_matches(['\n', '\r']).is_empty();
        if marker == 0 || !(ends || rest[marker..].starts_with([' ', '\t'])) {
            break;
        }
        kept.push_str(&rest[digits..marker]);
        rest = &rest[marker..];
        let spaced = rest.trim_start_matches([' ', '\t']);
        if let Some(task) = ["[x]", "[X]"].iter().find(|t| spaced.starts_with(**t)) {
            kept.push_str(&rest[..rest.len() - spaced.len()]);
            kept.push_str("[]");
            rest = &spaced[task.len()..];
        }
    }
    kept.push_str(rest);
    kept
}

/// The hidden-text check (SC-001, M-1, M-5): every received character that is not formatting
/// appears in the model's text, in order.
pub fn assert_nothing_hidden(source: &str, document: &Document, what: &str) {
    let shown: Vec<char> = model_text(document.blocks())
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
        .collect();
    let received = received(source);
    let mut shown_at = 0;
    for (n, &c) in received.iter().enumerate() {
        match shown[shown_at..].iter().position(|&s| s == c) {
            Some(found) => shown_at += found + 1,
            None => panic!(
                "{what}: received character #{n} {c:?} is not shown (FR-001)\n\
                 received before it: {:?}\nshown after the last match: {:?}\nsource: {source:?}",
                received[n.saturating_sub(30)..=n]
                    .iter()
                    .collect::<String>(),
                shown[shown_at..(shown_at + 40).min(shown.len())]
                    .iter()
                    .collect::<String>(),
            ),
        }
    }
}

/// A fixed-seed xorshift64 (no new dependency).
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}
