//! Feature 939 M5: the recorded stream-json fixtures as a window's transcript state, and the
//! transcript view mounted headless, for `transcript_*.rs`.

use iced::advanced::layout;
use iced::advanced::widget::operation::scrollable::Scrollable;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::{Element, Rectangle, Size, Vector};
use micold_client::features::structured_transcript::{State, TranscriptMsg};
use micold_client::ui::transcript::{view, BlockCache, BlockRenderers, RenderCtx};
use micold_core::session::SessionId;
use micold_core::structured::transcript::{EntryKey, EntryKind};
use micold_core::structured::{parse_line, LineSplitter, SequencedEvent};
use micold_core::theme::ColorScheme;
use micold_core::tokens;

/// A recorded fixture's lines (`micold-core/tests/fixtures/stream-json/<name>.jsonl`).
pub fn fixture_lines(name: &str) -> Vec<String> {
    let path = format!(
        "{}/../micold-core/tests/fixtures/stream-json/{name}.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("fixture {path} reads: {e}"));
    let mut splitter = LineSplitter::default();
    let mut lines = splitter.push(&bytes);
    lines.extend(splitter.finish());
    lines
}

/// `lines` parsed and numbered from `first`.
pub fn sequenced(lines: &[String], first: u64) -> Vec<SequencedEvent> {
    lines
        .iter()
        .zip(first..)
        .map(|(line, seq)| SequencedEvent {
            seq,
            event: parse_line(line),
        })
        .collect()
}

/// A window holding `session`'s transcript of the fixture `name`.
pub fn state_of(session: SessionId, name: &str) -> State {
    let mut state = State::default();
    state.apply(session, 0, &sequenced(&fixture_lines(name), 0), false);
    state
}

/// Unfold every card of `session`'s transcript (thinking and tool calls).
pub fn open_every_card(state: &mut State, session: SessionId) {
    let cards: Vec<EntryKey> = state
        .transcript(session)
        .expect("the session has a transcript")
        .entries()
        .filter(|e| {
            matches!(
                e.kind(),
                EntryKind::ToolCall(_) | EntryKind::Thinking { .. }
            )
        })
        .map(|e| e.key().clone())
        .collect();
    for key in cards {
        state.toggle(session, key);
    }
}

/// The light theme's render context over an empty cache.
pub fn ctx(cache: &BlockCache) -> RenderCtx<'_> {
    RenderCtx {
        roles: tokens::roles(ColorScheme::Light),
        scheme: ColorScheme::Light,
        cache,
    }
}

/// The view of `session` with `renderers`, in the light theme.
pub fn view_of<'a>(
    state: &'a State,
    session: SessionId,
    renderers: &'a BlockRenderers,
    cache: &'a BlockCache,
) -> Element<'a, TranscriptMsg> {
    view(state, session, renderers, ctx(cache))
}

/// What a scrollable shows, read back out of the tree.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Showing {
    /// How far the content is scrolled from its top.
    pub offset: f32,
    /// The viewport's height.
    pub viewport: f32,
    /// The content's height.
    pub content: f32,
}

impl Showing {
    /// Whether the content's end is in view.
    pub fn at_end(&self) -> bool {
        self.offset + self.viewport + 0.5 >= self.content
    }
}

struct Read(Option<Showing>);

impl<T> Operation<T> for Read {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        bounds: Rectangle,
        content_bounds: Rectangle,
        translation: Vector,
        _state: &mut dyn Scrollable,
    ) {
        self.0.get_or_insert(Showing {
            offset: translation.y,
            viewport: bounds.height,
            content: content_bounds.height,
        });
    }
}

/// A view kept mounted across frames: one widget tree, laid out again after each change, as the
/// runtime keeps it.
pub struct Mounted {
    pub tree: Option<Tree>,
    pub room: Size,
    pub renderer: iced::Renderer,
}

impl Mounted {
    pub fn new(room: Size) -> Self {
        Self {
            tree: None,
            room,
            renderer: super::layout::renderer(),
        }
    }

    /// Diff `element` into the tree, lay it out, and run each of `operations` over it in turn.
    pub fn frame<M>(
        &mut self,
        mut element: Element<'_, M>,
        operations: Vec<Box<dyn Operation>>,
    ) -> (layout::Node, Showing) {
        let tree = match &mut self.tree {
            Some(tree) => {
                tree.diff(element.as_widget());
                tree
            }
            None => self.tree.insert(Tree::new(element.as_widget())),
        };
        let limits = layout::Limits::new(Size::ZERO, self.room);
        let mut node = element
            .as_widget_mut()
            .layout(tree, &self.renderer, &limits);
        for mut op in operations {
            element.as_widget_mut().operate(
                tree,
                layout::Layout::new(&node),
                &self.renderer,
                op.as_mut(),
            );
            node = element
                .as_widget_mut()
                .layout(tree, &self.renderer, &limits);
        }
        let mut read = Read(None);
        element.as_widget_mut().operate(
            tree,
            layout::Layout::new(&node),
            &self.renderer,
            &mut read,
        );
        (node, read.0.expect("the view holds a scrollable"))
    }
}
