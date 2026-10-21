//! Assistant and thinking text as a markdown model (feature 940, research R1–R4;
//! contracts/markdown-model.md). Render-free and free of I/O: the client draws the model and
//! never parses markdown.
//!
//! The text is cut into top-level block slices (`split`), each parsed on its own, so a settled
//! block is final: nothing later in the text can reach back into it (M-2). While the text streams,
//! the slices from `open_from` on are open and are the only ones re-parsed on the next `update`.

mod block;
mod fence;
mod inline;
mod split;

use std::hash::{DefaultHasher, Hasher};
use std::iter::Peekable;
use std::ops::Range;
use std::vec::IntoIter;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag};

pub use block::{Align, Block, CodeBlock, CodeId, Inlines, Link, ListItem, Span, Style};
pub use fence::fence_token;

use inline::{is_inline, is_inline_end, lf, Inline};

/// The parser's options: CommonMark with GFM tables, strikethrough and task lists only (R1).
fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

/// A text's top-level blocks; `blocks[..settled]` are final (data-model.md *Document*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    blocks: Vec<Block>,
    settled: usize,
    open_from: usize,
    source_len: usize,
    /// A hash of the text the document was built from, so `update` knows it is being extended.
    source_hash: u64,
    streaming: bool,
}

fn hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    hasher.write(text.as_bytes());
    hasher.finish()
}

impl Document {
    /// The document of `text`; with `streaming`, its last slices are open (research R2).
    pub fn parse(text: &str, streaming: bool) -> Document {
        let mut document = Document {
            blocks: Vec::new(),
            settled: 0,
            open_from: 0,
            source_len: 0,
            source_hash: 0,
            streaming,
        };
        document.build_from(text, 0, streaming);
        document
    }

    /// The document of `text`, re-parsing only the open slices when `text` extends the text this
    /// document was built from while it streamed; otherwise parsed afresh (research R2).
    pub fn update(&mut self, text: &str, streaming: bool) {
        let extends = self.streaming
            && text.len() >= self.source_len
            && text.is_char_boundary(self.source_len)
            && hash(&text[..self.source_len]) == self.source_hash;
        if extends {
            let from = self.open_from;
            self.build_from(text, from, streaming);
        } else {
            *self = Document::parse(text, streaming);
        }
    }

    pub fn blocks(&self) -> &[Block] {
        &self.blocks
    }

    /// How many leading blocks are settled: final whatever text follows.
    pub fn settled(&self) -> usize {
        self.settled
    }

    /// The code block `id` names.
    pub fn code(&self, id: CodeId) -> Option<&CodeBlock> {
        let mut found = None;
        each_code(&self.blocks, &mut |code| {
            if code.id == id {
                found = Some(code);
            }
        });
        found
    }

    /// Every code block in document order, and whether it is in a settled block.
    pub fn code_blocks(&self) -> impl Iterator<Item = (CodeId, &CodeBlock, bool)> {
        let mut out = Vec::new();
        for (n, block) in self.blocks.iter().enumerate() {
            each_code(std::slice::from_ref(block), &mut |code| {
                out.push((code.id, code, n < self.settled));
            });
        }
        out.into_iter()
    }

    /// Keeps `blocks[..settled]`, which `text[..from]` built, and builds the rest from
    /// `text[from..]`.
    fn build_from(&mut self, text: &str, from: usize, streaming: bool) {
        self.blocks.truncate(self.settled);
        let mut next_code = 0;
        each_code(&self.blocks, &mut |_| next_code += 1);
        let tail = &text[from..];
        let cuts = split::cuts(tail);
        let hold = split::last_complete_line(tail);
        let settled_slices = if streaming {
            (1..cuts.len()).take_while(|&i| cuts[i] < hold).count()
        } else {
            cuts.len()
        };
        for (i, &start) in cuts.iter().enumerate() {
            let end = cuts.get(i + 1).copied().unwrap_or(tail.len());
            let blocks = slice_blocks(&tail[start..end], &mut next_code);
            if i < settled_slices {
                self.settled += blocks.len();
            }
            self.blocks.extend(blocks);
        }
        self.open_from = match cuts.get(settled_slices) {
            Some(&cut) => from + cut,
            None if streaming => from,
            None => text.len(),
        };
        self.source_len = text.len();
        self.source_hash = hash(text);
        self.streaming = streaming;
    }
}

/// Calls `f` with every code block of `blocks`, nested ones included, in document order.
fn each_code<'a>(blocks: &'a [Block], f: &mut impl FnMut(&'a CodeBlock)) {
    for block in blocks {
        match block {
            Block::Code(code) => f(code),
            Block::List { items, .. } => {
                for item in items {
                    each_code(&item.blocks, f);
                }
            }
            Block::Quote { blocks } => each_code(blocks, f),
            Block::Heading { .. } | Block::Paragraph { .. } | Block::Rule | Block::Table { .. } => {
            }
        }
    }
}

/// A paragraph of `text` as written: a raw HTML block or a link reference definition (FR-001).
fn literal(text: &str) -> Block {
    Block::Paragraph {
        text: vec![Span {
            text: lf(text.trim_end_matches(['\n', '\r'])),
            style: Style::default(),
            link: None,
        }],
    }
}

/// The blocks of one slice, parsed on its own. Text no block covers (a link reference
/// definition) is kept as a literal paragraph in its place, so nothing received is hidden.
fn slice_blocks(src: &str, next_code: &mut u32) -> Vec<Block> {
    let events: Vec<_> = Parser::new_ext(src, options()).into_offset_iter().collect();
    let mut builder = Builder {
        src,
        events: events.into_iter().peekable(),
        next_code: *next_code,
        task: None,
        nested: 0,
    };
    let mut out = Vec::new();
    let mut covered = 0;
    while let Some((event, range)) = builder.events.next() {
        let start = split::line_start(src, range.start).max(covered);
        if !src[covered..start].trim().is_empty() {
            out.push(literal(src[covered..start].trim()));
        }
        covered = covered.max(range.end);
        out.extend(builder.block(event, range));
    }
    if !src[covered..].trim().is_empty() {
        out.push(literal(src[covered..].trim()));
    }
    *next_code = builder.next_code;
    out
}

/// Builds blocks from one slice's events.
struct Builder<'a> {
    src: &'a str,
    events: Peekable<IntoIter<(Event<'a>, Range<usize>)>>,
    next_code: u32,
    /// The task-list mark of the item being built.
    task: Option<bool>,
    /// How many containers (quotes, list items) the builder is in.
    nested: u32,
}

impl Builder<'_> {
    /// The block `event` opens, its events consumed.
    fn block(&mut self, event: Event<'_>, range: Range<usize>) -> Option<Block> {
        let tag = match event {
            Event::Start(tag) => tag,
            Event::Rule => return Some(Block::Rule),
            Event::End(_) => return None,
            other => {
                let mut inline = Inline::default();
                inline.feed(other);
                let mut text = inline.finish();
                text.extend(self.inline_run());
                return Some(Block::Paragraph { text });
            }
        };
        Some(match tag {
            Tag::Paragraph => Block::Paragraph {
                text: self.inlines_to_end(),
            },
            Tag::Heading { level, .. } => Block::Heading {
                level: level as u8,
                text: self.inlines_to_end(),
            },
            Tag::BlockQuote(_) => {
                self.nested += 1;
                let (blocks, _) = self.blocks_to_end();
                self.nested -= 1;
                Block::Quote { blocks }
            }
            Tag::CodeBlock(kind) => self.code(kind, range),
            Tag::List(start) => self.list(start),
            Tag::Table(align) => self.table(&align),
            Tag::HtmlBlock => {
                let mut html = String::new();
                self.text_to_end(&mut html);
                literal(&html)
            }
            tag if is_inline(&tag) => {
                let mut inline = Inline::default();
                inline.feed(Event::Start(tag));
                let mut text = inline.finish();
                text.extend(self.inline_run());
                Block::Paragraph { text }
            }
            _ => {
                let mut text = String::new();
                self.text_to_end(&mut text);
                literal(&text)
            }
        })
    }

    /// Every text of the events up to the end of the open block, appended to `out`.
    fn text_to_end(&mut self, out: &mut String) {
        let mut depth = 0u32;
        for (event, _) in self.events.by_ref() {
            match event {
                Event::Start(_) => depth += 1,
                Event::End(_) if depth == 0 => break,
                Event::End(_) => depth -= 1,
                Event::Text(t) | Event::Html(t) | Event::InlineHtml(t) | Event::Code(t) => {
                    out.push_str(&t);
                }
                Event::SoftBreak | Event::HardBreak => out.push('\n'),
                _ => {}
            }
        }
    }

    /// The spans up to the end of the open block (a paragraph, heading or table cell).
    fn inlines_to_end(&mut self) -> Inlines {
        let mut inline = Inline::default();
        let mut depth = 0u32;
        for (event, _) in self.events.by_ref() {
            match &event {
                Event::End(end) if depth == 0 || !is_inline_end(end) => break,
                Event::End(_) => depth -= 1,
                Event::Start(_) => depth += 1,
                Event::TaskListMarker(done) => {
                    self.task = Some(*done);
                    continue;
                }
                _ => {}
            }
            inline.feed(event);
        }
        inline.finish()
    }

    /// The spans of a run of inline events directly in a container (a tight list item's text).
    fn inline_run(&mut self) -> Inlines {
        let mut inline = Inline::default();
        let mut depth = 0u32;
        while let Some((event, _)) = self.events.peek() {
            match event {
                Event::Start(tag) if is_inline(tag) => depth += 1,
                Event::Start(_) | Event::Rule => break,
                Event::End(end) if depth > 0 && is_inline_end(end) => depth -= 1,
                Event::End(_) => break,
                _ => {}
            }
            let (event, _) = self.events.next().expect("peeked");
            if let Event::TaskListMarker(done) = event {
                self.task = Some(done);
                continue;
            }
            inline.feed(event);
        }
        inline.finish()
    }

    /// The blocks up to the end of the open container, and whether a paragraph was among them
    /// (a loose list item).
    fn blocks_to_end(&mut self) -> (Vec<Block>, bool) {
        let mut blocks = Vec::new();
        let mut paragraph = false;
        while let Some((event, _)) = self.events.peek() {
            match event {
                Event::End(_) => {
                    self.events.next();
                    break;
                }
                Event::Start(tag) if !is_inline(tag) => {}
                Event::Rule => {}
                _ => {
                    blocks.push(Block::Paragraph {
                        text: self.inline_run(),
                    });
                    continue;
                }
            }
            let (event, range) = self.events.next().expect("peeked");
            paragraph |= matches!(event, Event::Start(Tag::Paragraph));
            blocks.extend(self.block(event, range));
        }
        (blocks, paragraph)
    }

    fn code(&mut self, kind: CodeBlockKind<'_>, range: Range<usize>) -> Block {
        let mut code = String::new();
        self.text_to_end(&mut code);
        let mut code = lf(&code);
        if code.ends_with('\n') {
            code.pop();
        }
        let (tag, open) = match kind {
            CodeBlockKind::Fenced(info) => {
                let word = fence::first_word(&info);
                let tag = (!word.is_empty()).then(|| word.to_string());
                (tag, !fence_closed(&self.src[range], self.nested > 0))
            }
            CodeBlockKind::Indented => (None, false),
        };
        let id = CodeId::new(self.next_code);
        self.next_code += 1;
        Block::Code(CodeBlock {
            id,
            token: tag.as_deref().and_then(fence_token),
            tag,
            lines: code.lines().count() as u32,
            bytes: code.len() as u64,
            code,
            open,
        })
    }

    fn list(&mut self, start: Option<u64>) -> Block {
        let mut items = Vec::new();
        let mut tight = true;
        while let Some((event, _)) = self.events.next() {
            match event {
                Event::Start(Tag::Item) => {
                    let outer = self.task.take();
                    self.nested += 1;
                    let (blocks, paragraph) = self.blocks_to_end();
                    self.nested -= 1;
                    let task = std::mem::replace(&mut self.task, outer);
                    tight &= !paragraph;
                    items.push(ListItem { task, blocks });
                }
                Event::End(_) => break,
                _ => {}
            }
        }
        Block::List {
            start,
            tight,
            items,
        }
    }

    fn table(&mut self, align: &[Alignment]) -> Block {
        let align = align
            .iter()
            .map(|a| match a {
                Alignment::None => Align::None,
                Alignment::Left => Align::Left,
                Alignment::Center => Align::Center,
                Alignment::Right => Align::Right,
            })
            .collect();
        let mut header = Vec::new();
        let mut rows = Vec::new();
        while let Some((event, _)) = self.events.next() {
            match event {
                Event::Start(Tag::TableHead) => header = self.cells_to_end(),
                Event::Start(Tag::TableRow) => rows.push(self.cells_to_end()),
                Event::End(_) => break,
                _ => {}
            }
        }
        Block::Table {
            align,
            header,
            rows,
        }
    }

    /// The cells up to the end of the open row.
    fn cells_to_end(&mut self) -> Vec<Inlines> {
        let mut cells = Vec::new();
        while let Some((event, _)) = self.events.next() {
            match event {
                Event::Start(Tag::TableCell) => cells.push(self.inlines_to_end()),
                Event::End(_) => break,
                _ => {}
            }
        }
        cells
    }
}

/// Whether the fenced block `src` (from its opening fence) has its closing fence: a later line of
/// the fence's character, at least as long, and nothing else. In a container the line's own
/// prefix (`>`, the item's indent) comes first.
fn fence_closed(src: &str, nested: bool) -> bool {
    let mut lines = src.split(['\n', '\r']);
    let first = lines.next().unwrap_or("").trim_start();
    let Some(fence) = first.chars().next().filter(|c| matches!(c, '`' | '~')) else {
        return true;
    };
    let length = first.chars().take_while(|&c| c == fence).count();
    lines.any(|line| {
        let body = if nested {
            line.trim_start_matches([' ', '\t', '>'])
        } else {
            let body = line.trim_start_matches(' ');
            if line.len() - body.len() > 3 {
                return false;
            }
            body
        };
        let run = body.chars().take_while(|&c| c == fence).count();
        run >= length && body[run..].trim().is_empty()
    })
}

#[cfg(test)]
mod tests {
    use super::split::{cuts, last_complete_line};
    use super::{Block, Document};

    #[test]
    fn a_text_is_cut_at_each_top_level_block_line() {
        assert_eq!(cuts(""), Vec::<usize>::new());
        assert_eq!(cuts("a\n\nb\n"), [0, 3]);
        assert_eq!(cuts("\n\n# h\n    code\n"), [0, 6]);
        assert_eq!(
            cuts("x\n| a |\n|-|\n"),
            [0, 2],
            "a table interrupts a paragraph"
        );
        assert_eq!(cuts("- a\n- b\n\nc"), [0, 9]);
    }

    #[test]
    fn the_last_complete_line_is_where_the_open_part_may_start() {
        assert_eq!(last_complete_line("abc"), 0);
        assert_eq!(last_complete_line("a\nbc"), 0);
        assert_eq!(last_complete_line("a\nb\n"), 2);
        assert_eq!(last_complete_line("a\r\nb\r\nc"), 3);
        assert_eq!(last_complete_line("a\rb\r"), 2);
    }

    #[test]
    fn a_cut_on_an_unfinished_line_is_not_final() {
        let doc = Document::parse("a\n***", true);
        assert_eq!(doc.settled(), 0, "`***b` would make it one paragraph");
        let doc = Document::parse("x\n| a |\n|-|", true);
        assert_eq!(doc.settled(), 0, "`|-|x` would make it one paragraph");
        let doc = Document::parse("x\n| a |\n|-|\n", true);
        assert_eq!(doc.settled(), 1);
    }

    #[test]
    fn an_open_image_shows_what_has_arrived() {
        let doc = Document::parse("![a tea", true);
        let text: String = doc
            .blocks()
            .iter()
            .flat_map(|b| match b {
                Block::Paragraph { text } => text.iter().map(|s| s.text.as_str()).collect(),
                _ => Vec::new(),
            })
            .collect();
        assert_eq!(text, "![a tea");
    }
}
