//! `MarkdownView` — a markdown [`Document`] drawn block by block (feature 940, research R9, R12;
//! FR-001–FR-004, FR-009–FR-012, FR-019).
//!
//! Headings take the type scale (a level-1 heading the headline role, level 2 the title role, the
//! rest the section role); paragraphs, list items, quotes and table cells are [`LinkText`], so bold,
//! italic, strikethrough, inline code and links read the same everywhere. Lists have bullet or
//! number gutters, and a task item a read-only checkbox glyph. A quote has the `outline_variant` rule
//! on its left, a rule is the shared [`Divider`], a table is iced's `table` (its header row bold, each
//! column aligned as the separator row says and as wide as its share, so a long cell wraps:
//! FR-003). A code block is a [`CodeBlock`], handed what the caller holds for its `CodeId`: its
//! spans, its copy state, and whether it is shown in full.
//!
//! **While it streams** (research R12) each settled block sits in its own iced `lazy`, keyed by its
//! index, the caller's stamp (bumped when spans, a copy state or a show-in-full choice change), the
//! scheme, the muted flag and the block's own hash; only the open block, the last, is built afresh
//! on each delta. The block's hash keeps a document parsed afresh from showing an old block at the
//! same index.
//!
//! An empty document draws nothing. Everything shown is owned, so the element is `'static`.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;

use iced::alignment::Horizontal;
use iced::widget::{column, container, lazy, row, table, text::Wrapping, Space};
use iced::{Element, Length};
use micold_core::markdown::{Align, Block, CodeBlock as Code, CodeId, Document, Inlines, ListItem};
use micold_core::review::diff::{SchemeSpans, Spans};
use micold_core::review::limits;
use micold_core::theme::ColorScheme;
use micold_core::tokens::{spacing, Roles};

use super::glyph::icon;
use super::link_text::ink;
use super::{CodeBlock, Divider, LinkText, Text, TypeRole};
use crate::features::transcript_rich::CopyState;
use crate::icons::Icon;

/// The width of a list item's gutter: its bullet, number or checkbox.
const GUTTER: f32 = 28.0;

/// A numbered list's gutter: [`GUTTER`], or wide enough for its longest number and its full stop
/// (a digit is at most 0.6 em, the stop 0.3 em) inside the gutter's padding.
fn number_gutter(last: u64) -> f32 {
    let digits = last.to_string().len() as f32;
    GUTTER.max((digits * 0.6 + 0.3) * TypeRole::Body.size() + 2.0 * spacing::XS)
}

/// The width of a quote's rule.
const QUOTE_RULE: f32 = 3.0;

/// A message from a code block's id.
type ById<M> = Rc<dyn Fn(CodeId) -> M>;

/// A markdown document. Builder form (Principle VIII):
/// `MarkdownView::new(&doc, roles).muted(false).stamp(stamp).on_link(f).on_copy(f).code_spans(f)
/// .copy_state(f).full(f).on_show(f).into()`.
pub struct MarkdownView<'a, M> {
    doc: &'a Document,
    roles: Roles,
    muted: bool,
    stamp: u64,
    on_link: Option<Rc<dyn Fn(String) -> M>>,
    on_copy: Option<ById<M>>,
    code_spans: Option<Box<dyn Fn(CodeId) -> Option<&'a Spans> + 'a>>,
    copy_state: Option<Box<dyn Fn(CodeId) -> Option<CopyState> + 'a>>,
    full: Option<Box<dyn Fn(CodeId) -> bool + 'a>>,
    on_show: Option<ById<M>>,
}

impl<'a, M: Clone + 'static> MarkdownView<'a, M> {
    pub fn new(doc: &'a Document, roles: Roles) -> Self {
        Self {
            doc,
            roles,
            muted: false,
            stamp: 0,
            on_link: None,
            on_copy: None,
            code_spans: None,
            copy_state: None,
            full: None,
            on_show: None,
        }
    }

    /// Draw the text in the muted style of a thinking card (FR-002).
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// The caller's stamp for what is drawn beside the document — spans, copy states, show-in-full
    /// choices — so a settled block is rebuilt when one of them changes (the entry's
    /// `Derived::stamp`).
    pub fn stamp(mut self, stamp: u64) -> Self {
        self.stamp = stamp;
        self
    }

    /// What pressing, or Enter on, an activatable link sends, from its address (FR-004).
    pub fn on_link(mut self, f: impl Fn(String) -> M + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }

    /// What a code block's copy control sends (FR-011). Without it the blocks have none.
    pub fn on_copy(mut self, f: impl Fn(CodeId) -> M + 'static) -> Self {
        self.on_copy = Some(Rc::new(f));
        self
    }

    /// A code block's syntax colours, both schemes, when the caller has them (research R4).
    pub fn code_spans(mut self, f: impl Fn(CodeId) -> Option<&'a Spans> + 'a) -> Self {
        self.code_spans = Some(Box::new(f));
        self
    }

    /// A code block's copy state; `None` is idle (FR-012).
    pub fn copy_state(mut self, f: impl Fn(CodeId) -> Option<CopyState> + 'a) -> Self {
        self.copy_state = Some(Box::new(f));
        self
    }

    /// Whether the user chose to show a code block over the size limits in full (FR-019).
    pub fn full(mut self, f: impl Fn(CodeId) -> bool + 'a) -> Self {
        self.full = Some(Box::new(f));
        self
    }

    /// What a held-back code block's "Show in full" sends (FR-019).
    pub fn on_show(mut self, f: impl Fn(CodeId) -> M + 'static) -> Self {
        self.on_show = Some(Rc::new(f));
        self
    }
}

/// What a code block is drawn with beside its code.
#[derive(Clone)]
struct CodeInputs {
    spans: Option<SchemeSpans>,
    state: Option<CopyState>,
    /// Over the size limits and not chosen to be shown in full.
    large: bool,
}

/// Everything a block is drawn with: owned, so a settled block's `lazy` can keep it.
struct Draw<M> {
    roles: Roles,
    muted: bool,
    on_link: Option<Rc<dyn Fn(String) -> M>>,
    on_copy: Option<ById<M>>,
    on_show: Option<ById<M>>,
    codes: HashMap<CodeId, CodeInputs>,
}

impl<M: Clone + 'static> Draw<M> {
    fn text(&self, inlines: &Inlines, role: TypeRole, strong: bool) -> Element<'static, M> {
        let mut text = LinkText::new(inlines, self.roles)
            .role(role)
            .muted(self.muted)
            .strong(strong);
        if let Some(on_link) = &self.on_link {
            let on_link = Rc::clone(on_link);
            text = text.on_link(move |url| on_link(url));
        }
        text.into()
    }

    fn blocks(&self, blocks: &[Block]) -> Element<'static, M> {
        column(blocks.iter().map(|b| self.block(b)))
            .spacing(spacing::SM)
            .width(Length::Fill)
            .into()
    }

    fn block(&self, block: &Block) -> Element<'static, M> {
        let r = self.roles;
        match block {
            Block::Heading { level, text } => {
                let role = match level {
                    1 => TypeRole::Headline,
                    2 => TypeRole::Title,
                    _ => TypeRole::Section,
                };
                self.text(text, role, false)
            }
            Block::Paragraph { text } => self.text(text, TypeRole::Body, false),
            Block::List { start, items, .. } => self.list(*start, items),
            Block::Quote { blocks } => row![
                container(Space::new())
                    .width(Length::Fixed(QUOTE_RULE))
                    .height(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(iced::Background::Color(super::style::color(
                            r.outline_variant,
                        ))),
                        ..container::Style::default()
                    }),
                self.blocks(blocks),
            ]
            .spacing(spacing::SM)
            .into(),
            Block::Rule => Divider::horizontal(r).into(),
            Block::Code(code) => self.code(code),
            Block::Table {
                align,
                header,
                rows,
            } => self.table(align, header, rows),
        }
    }

    fn list(&self, start: Option<u64>, items: &[ListItem]) -> Element<'static, M> {
        let r = self.roles;
        let tint = ink(r, self.muted);
        let width = match start {
            Some(n) => number_gutter(n + items.len().saturating_sub(1) as u64),
            None => GUTTER,
        };
        column(items.iter().zip(0u64..).map(|(item, k)| {
            let gutter: Element<'static, M> = match (item.task, start) {
                (Some(done), _) => {
                    let glyph = if done { Icon::TaskDone } else { Icon::TaskOpen };
                    icon(glyph, TypeRole::Body.size(), tint)
                }
                (None, Some(n)) => Text::new(format!("{}.", n + k), TypeRole::Body, r)
                    .tint(tint)
                    .wrapping(Wrapping::None)
                    .into(),
                (None, None) => Text::new("\u{2022}", TypeRole::Body, r).tint(tint).into(),
            };
            row![
                container(gutter)
                    .width(Length::Fixed(width))
                    .align_x(Horizontal::Right)
                    .padding([0.0, spacing::XS]),
                self.blocks(&item.blocks),
            ]
            .into()
        }))
        .spacing(spacing::XS)
        .width(Length::Fill)
        .into()
    }

    fn code(&self, code: &Code) -> Element<'static, M> {
        let mut block = CodeBlock::new(code.code.clone(), self.roles);
        if let Some(tag) = &code.tag {
            block = block.language(tag.clone());
        }
        let inputs = self.codes.get(&code.id);
        if let Some(spans) = inputs.and_then(|i| i.spans.as_ref()) {
            block = block.spans(spans);
        }
        if let Some(on_copy) = &self.on_copy {
            block = block.copy(inputs.and_then(|i| i.state), on_copy(code.id));
        }
        if let (Some(on_show), true) = (&self.on_show, inputs.is_some_and(|i| i.large)) {
            block = block.large(code.lines, on_show(code.id));
        }
        block.into()
    }

    fn table(
        &self,
        align: &[Align],
        header: &[Inlines],
        rows: &[Vec<Inlines>],
    ) -> Element<'static, M> {
        let empty = Inlines::new();
        let columns = header.iter().enumerate().map(|(j, head)| {
            let align_x = match align.get(j) {
                Some(Align::Center) => Horizontal::Center,
                Some(Align::Right) => Horizontal::Right,
                Some(Align::Left | Align::None) | None => Horizontal::Left,
            };
            let empty = &empty;
            table::column(self.text(head, TypeRole::Body, true), move |i: usize| {
                let cell = rows[i].get(j).unwrap_or(empty);
                self.text(cell, TypeRole::Body, false)
            })
            .align_x(align_x)
            .width(Length::Fill)
        });
        let grid: Element<'static, M> = table(columns, 0..rows.len())
            .width(Length::Fill)
            .padding_x(spacing::SM)
            .padding_y(spacing::XS)
            .into();
        grid
    }
}

/// A settled block's lazy key: what it is drawn from, and what it is drawn with.
fn key(index: usize, stamp: u64, scheme: ColorScheme, muted: bool, block: &Block) -> u64 {
    let mut hasher = DefaultHasher::new();
    (index, stamp, scheme == ColorScheme::Dark, muted, block).hash(&mut hasher);
    hasher.finish()
}

impl<'a, 'b, M: Clone + 'static> From<MarkdownView<'a, M>> for Element<'b, M> {
    fn from(view: MarkdownView<'a, M>) -> Self {
        let r = view.roles;
        let scheme = r.scheme();
        let codes = view
            .doc
            .code_blocks()
            .map(|(id, code, _)| {
                let spans = view
                    .code_spans
                    .as_ref()
                    .and_then(|f| f(id))
                    .map(|s| match scheme {
                        ColorScheme::Dark => s.dark.clone(),
                        ColorScheme::Light => s.light.clone(),
                    });
                let state = view.copy_state.as_ref().and_then(|f| f(id));
                let full = view.full.as_ref().is_some_and(|f| f(id));
                let large = !full && limits::exceeds(u64::from(code.lines), code.bytes);
                (
                    id,
                    CodeInputs {
                        spans,
                        state,
                        large,
                    },
                )
            })
            .collect();
        let draw = Rc::new(Draw {
            roles: r,
            muted: view.muted,
            on_link: view.on_link,
            on_copy: view.on_copy,
            on_show: view.on_show,
            codes,
        });
        let settled = view.doc.settled();
        let blocks = view.doc.blocks().iter().enumerate().map(|(i, block)| {
            if i >= settled {
                return draw.block(block);
            }
            let dependency = key(i, view.stamp, scheme, view.muted, block);
            let (draw, block) = (Rc::clone(&draw), block.clone());
            lazy(dependency, move |_| draw.block(&block)).into()
        });
        let element: Element<'static, M> = column(blocks)
            .spacing(spacing::SM)
            .width(Length::Fill)
            .into();
        element
    }
}

#[cfg(test)]
mod tests {
    use super::super::code_block::{COPIED, SHOW_IN_FULL};
    use super::super::test_support::{roles, Mounted};
    use super::*;
    use crate::features::transcript_rich::Ticket;
    use iced::advanced::mouse;
    use iced::{Event, Point};

    /// A small block, then one over the line limit.
    fn doc() -> Document {
        let big = "x\n".repeat(limits::MAX_CHANGED_LINES as usize + 1);
        Document::parse(
            &format!("Intro\n\n```rust\nfn a() {{}}\n```\n\n```\n{big}```\n"),
            false,
        )
    }

    fn ids(doc: &Document) -> (CodeId, CodeId) {
        let ids: Vec<CodeId> = doc.code_blocks().map(|(id, _, _)| id).collect();
        (ids[0], ids[1])
    }

    fn view(doc: &Document, full: bool) -> MarkdownView<'_, String> {
        let (small, _) = ids(doc);
        MarkdownView::new(doc, roles())
            .on_copy(|id| format!("copy {}", id.ordinal()))
            .copy_state(move |id| (id == small).then_some(CopyState::Copied(Ticket::posed(1))))
            .full(move |_| full)
            .on_show(|id| format!("show {}", id.ordinal()))
    }

    /// T027: the copy state of a `CodeId` reaches that block's control, and only that block's.
    #[test]
    fn a_copy_state_reaches_the_block_of_its_id() {
        let doc = doc();
        let mut m = Mounted::new(view(&doc, false));
        let copied = m
            .painted_text()
            .into_iter()
            .filter(|p| p.content == COPIED)
            .count();
        assert_eq!(copied, 1, "one block says Copied");
    }

    /// T027 (FR-019): a block over the limits and not in `full` shows "Show in full", whose press
    /// sends `on_show` with its id; once in `full` it shows its lines.
    #[test]
    fn a_large_block_not_in_full_shows_show_in_full_and_sends_its_id() {
        let doc = doc();
        let (_, large) = ids(&doc);
        let mut m = Mounted::new(view(&doc, false));
        let painted = m.painted_text();
        let button = painted
            .iter()
            .find(|p| p.content == SHOW_IN_FULL)
            .unwrap_or_else(|| panic!("the large block offers Show in full: {painted:?}"))
            .bounds;
        let at = Point::new(button.x + 1.0, button.y + button.height / 2.0);
        let mut out = m.press(at);
        out.extend(m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        ));
        assert_eq!(out, vec![format!("show {}", large.ordinal())]);
        let mut shown = Mounted::new(view(&doc, true));
        assert!(
            shown
                .painted_text()
                .iter()
                .all(|p| p.content != SHOW_IN_FULL),
            "in full, the block shows its lines"
        );
    }

    /// FR-003: a numbered list's gutter widens to its longest number, so "1000." does not run
    /// into its item's text.
    #[test]
    fn a_long_number_fits_its_gutter() {
        let doc = Document::parse("998. a\n999. b\n1000. c\n", false);
        let mut m = Mounted::new(MarkdownView::<String>::new(&doc, roles()));
        let painted = m.painted_text();
        let at = |content: &str| {
            painted
                .iter()
                .find(|p| p.content == content)
                .unwrap_or_else(|| panic!("`{content}` is painted: {painted:?}"))
                .bounds
        };
        let (number, item) = (at("1000."), at("c"));
        assert!(
            number.x + number.width <= item.x,
            "the number {number:?} ends before its item {item:?}"
        );
    }

    /// An empty document draws nothing.
    #[test]
    fn an_empty_document_draws_nothing() {
        let doc = Document::parse("", false);
        let m = Mounted::new(MarkdownView::<String>::new(&doc, roles()));
        assert_eq!(m.node.size().height, 0.0);
    }

    /// FR-001, FR-003: headings, list gutters, task glyphs, quotes and tables draw their text with
    /// no markdown punctuation.
    #[test]
    fn the_subset_draws_its_text_without_its_punctuation() {
        let doc = Document::parse(
            "# Title\n\n- one\n- [x] done\n\n1. first\n\n> quoted\n\n| a | b |\n|:-|-:|\n| c | d |\n",
            false,
        );
        let mut m = Mounted::new(MarkdownView::<String>::new(&doc, roles()));
        let painted: Vec<String> = m.painted_text().into_iter().map(|p| p.content).collect();
        for content in [
            "Title",
            "\u{2022}",
            "one",
            &Icon::TaskDone.glyph().to_string(),
            "done",
            "1.",
            "first",
            "quoted",
            "a",
            "b",
            "c",
            "d",
        ] {
            assert!(
                painted.iter().any(|p| p == content),
                "`{content}` is painted: {painted:?}"
            );
        }
        assert!(
            painted.iter().all(|p| !p.contains('#') && !p.contains('|')),
            "{painted:?}"
        );
    }
}
