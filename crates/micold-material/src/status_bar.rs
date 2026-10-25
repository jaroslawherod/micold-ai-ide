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

use iced::advanced::renderer;
use iced::widget::{container, row, Space};
use iced::{Alignment, Background, Element, Rectangle};
use micold_core::tokens::anatomy::status_bar::HEIGHT;
use micold_core::tokens::{spacing, Rgb, Roles};

use super::give_way::{Floor, GiveWay, Lane, Slot};
use super::{busy_indicator, unread_mark};
use super::{style, Ellipsized, Tooltip, TooltipPosition, TypeRole};

/// The coloured dot's diameter: the unread mark's, so the two marks read alike.
const DOT: f32 = unread_mark::DIAMETER;

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

/// A filled circle in `colour`, as the unread mark draws one.
fn dot<'a, M: 'a>(colour: Rgb) -> Element<'a, M> {
    container(Space::new().width(DOT).height(DOT))
        .style(move |_| unread_mark::filled_circle(colour))
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

impl<'a, M: 'a> From<StatusBar<'a, M>> for Element<'a, M> {
    fn from(b: StatusBar<'a, M>) -> Self {
        let r = b.roles;
        let mut lane = Lane::new(HEIGHT, spacing::MD, spacing::MD).backdrop(Box::new(
            move |renderer, bounds| {
                use iced::advanced::Renderer as _;
                renderer.fill_quad(
                    renderer::Quad {
                        bounds,
                        ..Default::default()
                    },
                    Background::Color(style::color(r.surface_container)),
                );
                renderer.fill_quad(
                    renderer::Quad {
                        bounds: Rectangle {
                            height: 1.0,
                            ..bounds
                        },
                        ..Default::default()
                    },
                    Background::Color(style::separator(r)),
                );
            },
        ));
        let groups = b
            .leading
            .into_iter()
            .map(|e| (e, false))
            .chain(b.trailing.into_iter().map(|e| (e, true)));
        for (entry, trailing) in groups {
            let (element, give_way, floor) = match entry {
                Entry::Item(item) => {
                    // What shortening to the floor text saves, its dot, one character and "…":
                    // measured only for an item that can shorten.
                    let floor = match item.give_way {
                        GiveWay::Shorten(_) => {
                            Floor::Saving(measure(&item.text) - measure(&item.floor_text()))
                        }
                        _ => Floor::Natural,
                    };
                    let give_way = item.give_way;
                    (item.element(r), give_way, floor)
                }
                // No text of its own to shorten, so a `Shorten` element keeps its width.
                Entry::Element(element, give_way) => (element, give_way, Floor::Natural),
            };
            lane = lane.push(
                element,
                Slot {
                    give_way,
                    trailing,
                    floor,
                },
            );
        }
        lane.into()
    }
}

/// How wide `content` shapes at the bar's text role.
fn measure(content: &str) -> f32 {
    super::shaped_text::width_of(content, ROLE)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{bounds_at, dark_roles, laid_out, roles, Mounted};
    use super::*;
    use iced::advanced::layout::{self, Layout};
    use iced::advanced::{mouse, Shell};
    use iced::window::RedrawRequest;
    use iced::{Event, Size};

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
