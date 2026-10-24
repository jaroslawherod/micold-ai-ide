//! `StatusBar` — the window's bottom bar of short, informational items (feature 1124, contract
//! `status-bar-ui.md` B1–B6).
//!
//! A fixed 28 dp row along the window's bottom edge: a leading group of items (a coloured dot or a
//! busy ring, then a short text) and a trailing group of key hints. No item is a button (FR-014):
//! the bar publishes nothing, takes no focus, and answers a hover with at most a tooltip.
//!
//! The bar lays itself out (research R4). Text width is only known once the renderer shapes it, so
//! the order in which items give way when the window is too narrow (FR-012) is decided here and not
//! in a view: every [`GiveWay::Drop`] item vanishes, lowest rank first, before any
//! [`GiveWay::Shorten`] item is ellipsized, lowest rank first, down to a floor of its dot, one
//! character and "…". A dropped item gets a zero-area node, so it neither draws nor has a hit area
//! (B5). What is drawn is also clipped to the bar's bounds, as a second line of defence.
//!
//! Builder form (Principle VIII):
//! `StatusBar::new(roles).item(StatusItem::new("Sandbox ready")).hint(..).into()`.

use iced::advanced::layout::{self, Layout};
use iced::advanced::text::{self, Paragraph as _, Text as CoreText};
use iced::advanced::widget::{Operation, Tree};
use iced::advanced::{mouse, overlay, renderer, Clipboard, Shell, Widget};
use iced::widget::{container, row, Space};
use iced::{alignment, Alignment, Background, Element, Event, Length, Pixels, Point};
use iced::{Rectangle, Size, Vector};
use micold_core::tokens::anatomy::status_bar::HEIGHT;
use micold_core::tokens::{shape, spacing, Rgb, Roles};

use super::busy_indicator;
use super::{style, Ellipsized, Tooltip, TooltipPosition, TypeRole};

/// The coloured dot's diameter: the unread mark's 8 dp, so the two marks read alike.
const DOT: f32 = 8.0;

/// What an item's text is drawn in: the bar is a quiet strip of `Label` text.
const ROLE: TypeRole = TypeRole::Label;

/// The tone of an item's dot or text. Maps to an existing colour role; the bar adds none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTone {
    /// Ordinary text: `on_surface_variant`.
    Neutral,
    /// All is well: `success`.
    Positive,
    /// Needs attention: `warning`.
    Warning,
    /// Broken: `error`.
    Error,
}

impl ItemTone {
    /// The role this tone is drawn in. One function, so the contrast gate
    /// (`composition_contrast.rs`) measures the colour that is drawn.
    pub fn colour(self, r: Roles) -> Rgb {
        match self {
            ItemTone::Neutral => r.on_surface_variant,
            ItemTone::Positive => r.success,
            ItemTone::Warning => r.warning,
            ItemTone::Error => r.error,
        }
    }
}

/// How an item yields when the bar is too narrow for every item at its natural width (R4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GiveWay {
    /// Never yields.
    #[default]
    Keep,
    /// Vanishes; lower ranks first, and every `Drop` before any `Shorten`.
    Drop(u8),
    /// Ellipsized down to its dot, one character and "…"; lower ranks first.
    Shorten(u8),
}

/// One item of the bar: an optional dot or busy ring, then a one-line text.
#[derive(Debug, Clone)]
pub struct StatusItem {
    text: String,
    dot: Option<ItemTone>,
    tone: ItemTone,
    busy: bool,
    tooltip: Option<String>,
    give_way: GiveWay,
}

impl StatusItem {
    /// An item saying `text` in the neutral tone, with no dot, not busy, that never yields.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            dot: None,
            tone: ItemTone::Neutral,
            busy: false,
            tooltip: None,
            give_way: GiveWay::Keep,
        }
    }

    /// A dot in `tone` before the text.
    pub fn dot(mut self, tone: ItemTone) -> Self {
        self.dot = Some(tone);
        self
    }

    /// The text's colour (default [`ItemTone::Neutral`]).
    pub fn tone(mut self, tone: ItemTone) -> Self {
        self.tone = tone;
        self
    }

    /// A turning ring before the text while `busy`; it asks for frames only then.
    pub fn busy(mut self, busy: bool) -> Self {
        self.busy = busy;
        self
    }

    /// The full detail, shown on hover. Without one, the hover shows the item's full text, which
    /// is what a shortened item needs.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// How the item yields at narrow widths (default [`GiveWay::Keep`]).
    pub fn give_way(mut self, give_way: GiveWay) -> Self {
        self.give_way = give_way;
        self
    }

    /// The item as an element: the ring or dot, the text, and the tooltip around them.
    fn element<'a, M: 'a>(self, r: Roles) -> Element<'a, M> {
        let mut parts: Vec<Element<'a, M>> = Vec::new();
        if self.busy {
            parts.push(busy_indicator::ring(r));
        }
        if let Some(tone) = self.dot {
            parts.push(dot(tone.colour(r)));
        }
        parts.push(
            Ellipsized::at_role(self.text.clone(), ROLE, self.tone.colour(r))
                .shrink()
                .into(),
        );
        let content = row(parts).spacing(spacing::XS).align_y(Alignment::Center);
        let label = self.tooltip.unwrap_or(self.text);
        Tooltip::new(content, label, r)
            .position(TooltipPosition::Top)
            .into()
    }

    /// The text kept when the item is shortened as far as it goes: one character and "…".
    fn floor_text(&self) -> String {
        let mut out: String = self.text.chars().take(1).collect();
        out.push('…');
        out
    }
}

/// A filled circle in `colour`.
fn dot<'a, M: 'a>(colour: Rgb) -> Element<'a, M> {
    container(Space::new().width(DOT).height(DOT))
        .style(move |_| container::Style {
            background: Some(Background::Color(style::color(colour))),
            border: iced::Border {
                radius: shape::FULL.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

/// The bar, built item by item, then `.into()` an element. See the module documentation.
pub struct StatusBar<'a, M> {
    roles: Roles,
    leading: Vec<Entry<'a, M>>,
    trailing: Vec<Entry<'a, M>>,
}

/// An item before it is turned into an element.
enum Entry<'a, M> {
    Item(StatusItem),
    Element(Element<'a, M>, GiveWay),
}

impl<'a, M: 'a> StatusBar<'a, M> {
    /// An empty bar themed by `roles`.
    pub fn new(roles: Roles) -> Self {
        Self {
            roles,
            leading: Vec::new(),
            trailing: Vec::new(),
        }
    }

    /// An item in the leading group, after those already added.
    pub fn item(mut self, item: StatusItem) -> Self {
        self.leading.push(Entry::Item(item));
        self
    }

    /// An existing component as a leading item (the plan-usage indicator). It can drop, but it has
    /// no text of its own to shorten, so a `Shorten` element keeps its width.
    pub fn item_element(mut self, element: impl Into<Element<'a, M>>, give_way: GiveWay) -> Self {
        self.leading.push(Entry::Element(element.into(), give_way));
        self
    }

    /// An item in the trailing group, right-aligned, after those already added.
    pub fn hint(mut self, item: StatusItem) -> Self {
        self.trailing.push(Entry::Item(item));
        self
    }
}

/// What layout needs to know about one child beside its element.
struct Meta {
    give_way: GiveWay,
    trailing: bool,
    /// The full and the floor text, for an item that can be shortened.
    texts: Option<(String, String)>,
}

/// The laid-out bar.
struct Bar<'a, M> {
    roles: Roles,
    children: Vec<Element<'a, M>>,
    metas: Vec<Meta>,
}

impl<'a, M: 'a> From<StatusBar<'a, M>> for Element<'a, M> {
    fn from(b: StatusBar<'a, M>) -> Self {
        let r = b.roles;
        let mut children = Vec::new();
        let mut metas = Vec::new();
        let groups = b
            .leading
            .into_iter()
            .map(|e| (e, false))
            .chain(b.trailing.into_iter().map(|e| (e, true)));
        for (entry, trailing) in groups {
            let (element, give_way, texts) = match entry {
                Entry::Item(item) => {
                    let texts = Some((item.text.clone(), item.floor_text()));
                    let give_way = item.give_way;
                    (item.element(r), give_way, texts)
                }
                Entry::Element(element, give_way) => (element, give_way, None),
            };
            children.push(element);
            metas.push(Meta {
                give_way,
                trailing,
                texts,
            });
        }
        Element::new(Bar {
            roles: r,
            children,
            metas,
        })
    }
}

/// One child's widths for the give-way pass.
#[derive(Debug, Clone, Copy)]
struct Widths {
    give_way: GiveWay,
    natural: f32,
    floor: f32,
}

/// The width each child gets in `available`, separated by `gap`; `None` is a dropped child (R4).
///
/// While the total is over: drop `Drop` children by rank (within a rank, the last first, so of two
/// key hints the rightmost goes first); then shorten `Shorten` children by rank, each by as much of
/// the excess as its floor allows.
fn plan(items: &[Widths], gap: f32, available: f32) -> Vec<Option<f32>> {
    let mut widths: Vec<Option<f32>> = items.iter().map(|w| Some(w.natural)).collect();
    let total = |widths: &[Option<f32>]| {
        let shown: Vec<f32> = widths.iter().flatten().copied().collect();
        shown.iter().sum::<f32>() + gap * shown.len().saturating_sub(1) as f32
    };

    let mut drops: Vec<(u8, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, w)| match w.give_way {
            GiveWay::Drop(rank) => Some((rank, i)),
            _ => None,
        })
        .collect();
    drops.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));
    for (_, i) in drops {
        if total(&widths) <= available {
            break;
        }
        widths[i] = None;
    }

    let mut shortens: Vec<(u8, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, w)| match w.give_way {
            GiveWay::Shorten(rank) => Some((rank, i)),
            _ => None,
        })
        .collect();
    shortens.sort();
    for (_, i) in shortens {
        let excess = total(&widths) - available;
        if excess <= 0.0 {
            break;
        }
        if let Some(w) = widths[i] {
            widths[i] = Some((w - excess).max(items[i].floor.min(w)));
        }
    }
    widths
}

/// How wide `content` shapes at the bar's text role.
fn measure(content: &str) -> f32 {
    <iced::Renderer as text::Renderer>::Paragraph::with_text(CoreText {
        content,
        bounds: Size::INFINITE,
        size: Pixels(ROLE.size()),
        line_height: text::LineHeight::default(),
        font: ROLE.font(),
        align_x: text::Alignment::Left,
        align_y: alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    })
    .min_bounds()
    .width
}

/// Whether a laid-out child was dropped: a zero-area node.
fn dropped(layout: Layout<'_>) -> bool {
    let b = layout.bounds();
    b.width <= 0.0 || b.height <= 0.0
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Bar<'_, M> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(&self.children);
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fixed(HEIGHT))
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        let size = limits.resolve(Length::Fill, Length::Fixed(HEIGHT), Size::ZERO);
        let available = (size.width - 2.0 * spacing::MD).max(0.0);
        let loose = layout::Limits::new(Size::ZERO, Size::new(f32::INFINITY, HEIGHT));

        let widths: Vec<Widths> = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(&self.metas)
            .map(|((child, state), meta)| {
                let natural = child
                    .as_widget_mut()
                    .layout(state, renderer, &loose)
                    .size()
                    .width;
                let floor = match (&meta.texts, meta.give_way) {
                    (Some((full, floor)), GiveWay::Shorten(_)) => {
                        let saved = measure(full) - measure(floor);
                        (natural - saved.max(0.0)).ceil()
                    }
                    _ => natural,
                };
                Widths {
                    give_way: meta.give_way,
                    natural,
                    floor,
                }
            })
            .collect();
        let planned = plan(&widths, spacing::MD, available);

        let mut nodes: Vec<layout::Node> = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(&planned)
            .map(|((child, state), width)| match width {
                Some(w) => {
                    let fixed = layout::Limits::new(Size::ZERO, Size::new(*w, HEIGHT));
                    child.as_widget_mut().layout(state, renderer, &fixed)
                }
                None => layout::Node::new(Size::ZERO),
            })
            .collect();

        let centred = |node: &layout::Node, x: f32| {
            Point::new(x, ((HEIGHT - node.size().height) / 2.0).max(0.0))
        };
        let mut x = spacing::MD;
        for (node, meta) in nodes.iter_mut().zip(&self.metas) {
            if meta.trailing || node.size().width <= 0.0 {
                continue;
            }
            let at = centred(node, x);
            node.move_to_mut(at);
            x += node.size().width + spacing::MD;
        }
        let mut x = size.width - spacing::MD;
        for (node, meta) in nodes.iter_mut().zip(&self.metas).rev() {
            if !meta.trailing || node.size().width <= 0.0 {
                continue;
            }
            x -= node.size().width;
            let at = centred(node, x);
            node.move_to_mut(at);
            x -= spacing::MD;
        }
        layout::Node::with_children(size, nodes)
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        operation: &mut dyn Operation,
    ) {
        operation.traverse(&mut |operation| {
            for ((child, state), l) in self
                .children
                .iter_mut()
                .zip(tree.children.iter_mut())
                .zip(layout.children())
            {
                child.as_widget_mut().operate(state, l, renderer, operation);
            }
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, M>,
        viewport: &Rectangle,
    ) {
        for ((child, state), l) in self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
        {
            if dropped(l) {
                continue;
            }
            child.as_widget_mut().update(
                state, event, l, cursor, renderer, clipboard, shell, viewport,
            );
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        self.children
            .iter()
            .zip(tree.children.iter())
            .zip(layout.children())
            .filter(|(_, l)| !dropped(*l))
            .map(|((c, s), l)| {
                c.as_widget()
                    .mouse_interaction(s, l, cursor, viewport, renderer)
            })
            .max()
            .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style_: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        let bounds = layout.bounds();
        renderer.fill_quad(
            renderer::Quad {
                bounds,
                ..Default::default()
            },
            Background::Color(style::color(self.roles.surface_container)),
        );
        renderer.fill_quad(
            renderer::Quad {
                bounds: Rectangle {
                    height: 1.0,
                    ..bounds
                },
                ..Default::default()
            },
            Background::Color(style::separator(self.roles)),
        );
        let Some(visible) = bounds.intersection(viewport) else {
            return;
        };
        renderer.with_layer(visible, |renderer| {
            for ((child, state), l) in self
                .children
                .iter()
                .zip(tree.children.iter())
                .zip(layout.children())
            {
                if dropped(l) {
                    continue;
                }
                child
                    .as_widget()
                    .draw(state, renderer, theme, style_, l, cursor, viewport);
            }
        });
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, M, iced::Theme, iced::Renderer>> {
        let overlays: Vec<_> = self
            .children
            .iter_mut()
            .zip(tree.children.iter_mut())
            .zip(layout.children())
            .filter(|(_, l)| !dropped(*l))
            .filter_map(|((child, state), l)| {
                child
                    .as_widget_mut()
                    .overlay(state, l, renderer, viewport, translation)
            })
            .collect();
        (!overlays.is_empty()).then(|| overlay::Group::with_children(overlays).overlay())
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{bounds_at, dark_roles, laid_out, roles, Mounted};
    use super::*;
    use iced::window::RedrawRequest;

    const ROOM: Size = Size::new(1200.0, 600.0);

    fn steady(r: Roles) -> StatusBar<'static, String> {
        StatusBar::new(r)
            .item(
                StatusItem::new("Daemon connected · local")
                    .dot(ItemTone::Positive)
                    .give_way(GiveWay::Shorten(2)),
            )
            .item(StatusItem::new("Sandbox ready").give_way(GiveWay::Shorten(1)))
            .hint(StatusItem::new("Ctrl+Shift+O Overview").give_way(GiveWay::Drop(0)))
            .hint(StatusItem::new("Ctrl+Shift+P Jump").give_way(GiveWay::Drop(0)))
    }

    fn children(node: &layout::Node) -> Vec<Rectangle> {
        node.children().iter().map(|c| c.bounds()).collect()
    }

    /// B1: exactly the anatomy's height, and the whole width it is given.
    #[test]
    fn the_bar_is_the_anatomy_height_and_fills_the_width() {
        let b = bounds_at(steady(roles()).into(), ROOM, &[]);
        assert_eq!(b.height, HEIGHT);
        assert_eq!(b.width, ROOM.width);
    }

    /// B2: the leading group starts and the trailing group ends `spacing::MD` from the edges, with
    /// `spacing::MD` between items, all vertically centred.
    #[test]
    fn items_sit_md_from_the_edges_and_md_apart() {
        let node = laid_out(steady(roles()).into(), ROOM);
        let c = children(&node);
        assert_eq!(c[0].x, spacing::MD);
        assert_eq!(c[1].x, c[0].x + c[0].width + spacing::MD);
        assert_eq!(c[3].x + c[3].width, ROOM.width - spacing::MD);
        assert_eq!(c[2].x + c[2].width + spacing::MD, c[3].x);
        for b in &c {
            assert!(
                (b.center_y() - HEIGHT / 2.0).abs() < 0.5,
                "{b:?} is not centred"
            );
        }
    }

    fn w(give_way: GiveWay, natural: f32, floor: f32) -> Widths {
        Widths {
            give_way,
            natural,
            floor,
        }
    }

    /// B4, the order: `Drop(0)` before `Drop(1)`, every `Drop` before any `Shorten`.
    #[test]
    fn drops_go_by_rank_before_any_shortening() {
        let items = [
            w(GiveWay::Shorten(2), 100.0, 20.0),
            w(GiveWay::Shorten(1), 100.0, 20.0),
            w(GiveWay::Drop(1), 50.0, 50.0),
            w(GiveWay::Drop(0), 50.0, 50.0),
        ];
        // All fit: 300 + 3 gaps of 10.
        assert_eq!(
            plan(&items, 10.0, 330.0),
            vec![Some(100.0), Some(100.0), Some(50.0), Some(50.0)]
        );
        // 10 over: the rank-0 drop goes, and nothing else changes.
        assert_eq!(
            plan(&items, 10.0, 320.0),
            vec![Some(100.0), Some(100.0), Some(50.0), None]
        );
        // Still over without the rank-1 drop too: both drops go before any shortening.
        assert_eq!(
            plan(&items, 10.0, 260.0),
            vec![Some(100.0), Some(100.0), None, None]
        );
        // Over with both drops gone: Shorten(1) gives first.
        assert_eq!(
            plan(&items, 10.0, 180.0),
            vec![Some(100.0), Some(70.0), None, None]
        );
    }

    /// B4: `Shorten(0)` before `Shorten(1)` before `Shorten(2)`, each down to its floor.
    #[test]
    fn shortening_goes_by_rank_down_to_the_floor() {
        let items = [
            w(GiveWay::Shorten(2), 100.0, 20.0),
            w(GiveWay::Shorten(1), 100.0, 20.0),
            w(GiveWay::Shorten(0), 100.0, 20.0),
        ];
        assert_eq!(
            plan(&items, 0.0, 250.0),
            vec![Some(100.0), Some(100.0), Some(50.0)]
        );
        assert_eq!(
            plan(&items, 0.0, 180.0),
            vec![Some(100.0), Some(60.0), Some(20.0)]
        );
        assert_eq!(
            plan(&items, 0.0, 50.0),
            vec![Some(20.0), Some(20.0), Some(20.0)],
            "nothing goes below its floor"
        );
    }

    /// Of two key hints at the same rank, the rightmost goes first.
    #[test]
    fn of_two_hints_the_rightmost_drops_first() {
        let items = [
            w(GiveWay::Keep, 100.0, 100.0),
            w(GiveWay::Drop(0), 50.0, 50.0),
            w(GiveWay::Drop(0), 50.0, 50.0),
        ];
        assert_eq!(
            plan(&items, 0.0, 160.0),
            vec![Some(100.0), Some(50.0), None]
        );
    }

    /// B4/B5 mounted: at a too-narrow width the hints have zero-area nodes and the shortened text
    /// keeps its dot, one character and "…"; nothing is painted past the bar.
    #[test]
    fn a_too_narrow_bar_drops_hints_then_shortens_to_the_floor() {
        let narrow = Size::new(120.0, 600.0);
        let mut m = Mounted::new_in(steady(roles()), narrow);
        let c = children(&m.node);
        assert_eq!(c[2].width * c[2].height, 0.0, "the hint is dropped");
        assert_eq!(c[3].width * c[3].height, 0.0, "the hint is dropped");
        let painted = m.painted_text();
        for p in &painted {
            assert!(
                p.bounds.x >= 0.0 && p.bounds.x + p.bounds.width <= narrow.width + 0.5,
                "{p:?} is painted past the bar"
            );
        }
        let connection = painted
            .iter()
            .find(|p| p.content.starts_with('D'))
            .expect("the connection item keeps its floor");
        assert!(connection.content.ends_with('…'), "{connection:?}");
        assert!(c[0].width > DOT, "the dot is kept");
    }

    /// B5: a dropped item draws nothing.
    #[test]
    fn a_dropped_item_draws_nothing() {
        let mut m = Mounted::new_in(steady(roles()), Size::new(400.0, 600.0));
        let drawn: Vec<String> = m.painted_text().into_iter().map(|p| p.content).collect();
        assert!(
            !drawn
                .iter()
                .any(|t| t.contains("Overview") || t.contains("Jump")),
            "{drawn:?}"
        );
    }

    /// B3: no item is a button: a press anywhere on the bar publishes nothing.
    #[test]
    fn a_press_on_any_item_publishes_nothing() {
        let mut m = Mounted::new_in(steady(roles()), ROOM);
        for b in children(&m.node) {
            assert!(m.press(b.center()).is_empty());
            assert!(m.hover(b.center()).is_empty());
        }
    }

    /// The tones are the existing roles, in both schemes.
    #[test]
    fn tones_map_to_their_roles() {
        for r in [roles(), dark_roles()] {
            assert_eq!(ItemTone::Neutral.colour(r), r.on_surface_variant);
            assert_eq!(ItemTone::Positive.colour(r), r.success);
            assert_eq!(ItemTone::Warning.colour(r), r.warning);
            assert_eq!(ItemTone::Error.colour(r), r.error);
        }
    }

    fn asks_for_a_frame(m: &mut Mounted<'_>) -> bool {
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        m.element.as_widget_mut().update(
            &mut m.tree,
            &Event::Window(iced::window::Event::RedrawRequested(
                std::time::Instant::now(),
            )),
            Layout::new(&m.node),
            mouse::Cursor::Unavailable,
            &m.renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &Rectangle::with_size(m.room),
        );
        shell.redraw_request() != RedrawRequest::Wait
    }

    /// A bar at rest asks for no frame; one with a busy item does (the ring turns).
    #[test]
    fn only_a_busy_bar_asks_for_frames() {
        let mut idle = Mounted::new_in(steady(roles()), ROOM);
        assert!(!asks_for_a_frame(&mut idle));
        let busy = StatusBar::new(roles()).item(StatusItem::new("Getting image… 42%").busy(true));
        let mut busy = Mounted::new_in(busy, ROOM);
        assert!(asks_for_a_frame(&mut busy));
    }
}
