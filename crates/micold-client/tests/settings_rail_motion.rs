//! The settings rail slides between its two widths rather than jumping (030, FR-001–FR-015).
//!
//! # What is being protected
//!
//! Collapsing the rail gives its width to the section beside it, and the section is what the user
//! is reading. A rail that snaps from labels to icons moves every line of that section sideways in
//! one frame, which is the jump the worktree sidebar's slide exists to avoid — and the settings view
//! claimed to make that slide for over a release while the drawer it used was pinned open and did
//! nothing.
//!
//! So these build the real settings surface, flip the rail, and pump frames the way iced_winit's
//! loop does (research R9), reading where the section starts after each. They also hold the
//! keyboard and the pointer to the rail's rest behaviour while it moves: the control that was
//! pressed keeps focus, nothing hidden becomes reachable, and a press lands where it is drawn.

mod support;

use std::time::Instant;

use iced::advanced::widget::operation::{Focusable, Outcome};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::window::RedrawRequest;
use iced::{Element, Point, Rectangle, Size};
use micold_client::app::Message;
use micold_client::features::settings::{Msg as SettingsMsg, SettingsSection};
use micold_core::tokens::motion::duration;
use support::covered_states::covered_states;
use support::layout::{self as lay, StateUnderTest, FRAME, WINDOW};

/// The rail's two widths, as the fixture records them.
const EXPANDED: f32 = 288.0;
const COLLAPSED: f32 = 80.0;

/// The rail, the first child of the row it shares with the section.
const RAIL: &str = "0/0/0/1/0/0";

/// Where the section beside the rail starts: its x is exactly the rail's width.
const PAGE: &str = "0/0/0/1/0/1";

/// The section's scrolled content, padded inside [`PAGE`].
const PAGE_CONTENT: &str = "0/0/0/1/0/1/0";

/// The section's two exits, in the actions row under it.
const CANCEL: &str = "0/0/0/1/0/1/1/1";
const SAVE: &str = "0/0/0/1/0/1/1/2";

/// Far longer than any transition the motion tokens allow, so "settled" means settled.
const SETTLE: usize = 120;

/// How many times a frame repeats update-then-layout while the layout keeps being invalidated,
/// as iced_winit bounds it.
const RELAYOUTS: usize = 3;

/// How far two positions may differ and still be the same: what `f32` loses subtracting two
/// coordinates once the rail's edge is fractional. Far below a pixel, so any movement shows.
const ROUNDING: f32 = 0.001;

/// Every focusable control, and which one holds the keyboard.
#[derive(Default)]
struct Focusables {
    all: Vec<Rectangle>,
    focused: Option<Rectangle>,
}

impl<T> Operation<T> for Focusables {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if state.is_focused() {
            self.focused = Some(bounds);
        }
        self.all.push(bounds);
    }
}

/// Focus exactly the control at `bounds`, the way the keyboard would reach it.
struct FocusAt(Rectangle);

impl<T> Operation<T> for FocusAt {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        if bounds == self.0 {
            state.focus();
        } else {
            state.unfocus();
        }
    }
}

/// Lay `under`'s view out into `tree`, at the window's size.
fn layout_of(under: &StateUnderTest, tree: &mut Tree, renderer: &iced::Renderer) -> layout::Node {
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    lay::view_of(under)
        .as_widget_mut()
        .layout(tree, renderer, &limits)
}

/// Deliver one event to `under`'s view laid out as `node`, returning whether the layout was
/// invalidated and what the shell asked of the next frame.
fn deliver(
    under: &StateUnderTest,
    tree: &mut Tree,
    renderer: &iced::Renderer,
    node: &layout::Node,
    event: &iced::Event,
    cursor: Option<Point>,
    messages: &mut Vec<Message>,
) -> (bool, RedrawRequest) {
    let cursor = cursor.map_or(mouse::Cursor::Unavailable, mouse::Cursor::Available);
    let mut shell = Shell::new(messages);
    lay::view_of(under).as_widget_mut().update(
        tree,
        event,
        Layout::new(node),
        cursor,
        renderer,
        &mut clipboard::Null,
        &mut shell,
        &Rectangle::with_size(WINDOW),
    );
    (shell.is_layout_invalid(), shell.redraw_request())
}

/// The settings surface with the rail in `collapsed`, mounted and settled.
struct Surface {
    under: StateUnderTest,
    tree: Tree,
    renderer: iced::Renderer,
    origin: Instant,
    frame: u32,
    redraw: RedrawRequest,
}

impl Surface {
    fn new(collapsed: bool) -> Self {
        let covered = covered_states()
            .iter()
            .find(|c| c.name == "settings-view-rail-collapsed")
            .expect("the collapsed-rail covered state is gone");
        let mut under = (covered.build)();
        under.state.settings.settings_rail_collapsed = collapsed;
        let tree = Tree::new(lay::view_of(&under).as_widget());
        Self {
            under,
            tree,
            renderer: lay::renderer(),
            origin: Instant::now(),
            frame: 0,
            redraw: RedrawRequest::Wait,
        }
    }

    fn node(&mut self) -> layout::Node {
        layout_of(&self.under, &mut self.tree, &self.renderer)
    }

    /// Where the section starts, which is how wide the rail is right now.
    fn page_x(&mut self) -> f32 {
        self.bounds(PAGE).x
    }

    /// The bounds of the node at `path` in the surface as it is laid out now.
    fn bounds(&mut self, path: &str) -> Rectangle {
        let node = self.node();
        let record = lay::walk(Layout::new(&node), lay::Layer::Base)
            .into_iter()
            .find(|r| lay::path_token(&r.path) == path)
            .unwrap_or_else(|| panic!("nothing is laid out at {path}"));
        Rectangle::new(
            Point::new(record.x, record.y),
            Size::new(record.width, record.height),
        )
    }

    /// Diff the tree onto the view of the state as it is now.
    fn rebuild(&mut self) {
        let element = lay::view_of(&self.under);
        self.tree.diff(element.as_widget());
    }

    /// Flip the rail the way the toggle's message does, and diff the tree onto the new view.
    fn toggle(&mut self) {
        let collapsed = &mut self.under.state.settings.settings_rail_collapsed;
        *collapsed = !*collapsed;
        self.rebuild();
    }

    /// Apply `message` through the application's reducer, as the runtime would, and rebuild.
    fn send(&mut self, message: Message) {
        self.under.state.update(message);
        self.rebuild();
    }

    /// The instant of the next frame in the surface's own clock, advancing it by [`FRAME`].
    fn next_instant(&mut self) -> Instant {
        self.frame += 1;
        self.origin + FRAME * self.frame
    }

    /// One frame of iced_winit's loop at `at`: `RedrawRequested` with `cursor`, then a layout, and
    /// again with the same instant while the shell reports the layout invalidated, at most
    /// [`RELAYOUTS`] times. Returns the messages published.
    fn frame(&mut self, at: Instant, cursor: Option<Point>) -> Vec<Message> {
        let event = iced::Event::Window(iced::window::Event::RedrawRequested(at));
        let mut messages = Vec::new();
        let mut node = self.node();
        self.redraw = RedrawRequest::Wait;
        for _ in 0..RELAYOUTS {
            let (invalid, redraw) = deliver(
                &self.under,
                &mut self.tree,
                &self.renderer,
                &node,
                &event,
                cursor,
                &mut messages,
            );
            self.redraw = self.redraw.min(redraw);
            node = self.node();
            if !invalid {
                break;
            }
        }
        messages
    }

    /// Whether the last frame asked for another.
    fn wants_a_frame(&self) -> bool {
        self.redraw != RedrawRequest::Wait
    }

    /// Press and release the left button at `at` over one layout, and feed every message published
    /// through the reducer before the next view is built. Returns the messages published.
    fn press(&mut self, at: Point) -> Vec<Message> {
        let node = self.node();
        let mut messages = Vec::new();
        for event in [
            mouse::Event::ButtonPressed(mouse::Button::Left),
            mouse::Event::ButtonReleased(mouse::Button::Left),
        ] {
            let _ = deliver(
                &self.under,
                &mut self.tree,
                &self.renderer,
                &node,
                &iced::Event::Mouse(event),
                Some(at),
                &mut messages,
            );
        }
        for message in messages.clone() {
            self.under.state.update(message);
        }
        self.rebuild();
        messages
    }

    /// Run `operation`, and each operation it chains to, over the surface.
    fn run(&mut self, operation: impl Operation<()> + 'static) {
        let node = self.node();
        let mut element: Element<'_, ()> = lay::view_of(&self.under).map(|_| ());
        let mut current: Box<dyn Operation<()>> = Box::new(operation);
        loop {
            element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&node),
                &self.renderer,
                current.as_mut(),
            );
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::None | Outcome::Some(_) => break,
            }
        }
    }

    fn focusables(&mut self) -> Focusables {
        let node = self.node();
        let mut element: Element<'_, ()> = lay::view_of(&self.under).map(|_| ());
        let mut probe = Focusables::default();
        element.as_widget_mut().operate(
            &mut self.tree,
            Layout::new(&node),
            &self.renderer,
            &mut probe,
        );
        probe
    }

    /// The section's x on every frame from the toggle until the rail has settled.
    fn toggle_and_trace(&mut self) -> Vec<f32> {
        self.toggle();
        (0..SETTLE)
            .map(|_| {
                let at = self.next_instant();
                self.frame(at, None);
                self.page_x()
            })
            .collect()
    }
}

/// Whether `width` lies strictly between the rail's two rest widths.
fn between(width: f32) -> bool {
    width > COLLAPSED && width < EXPANDED
}

/// The trace passed through a width strictly between the two, and settled at `to`.
fn assert_slides(trace: &[f32], to: f32) {
    assert!(
        trace.iter().copied().any(between),
        "the rail reached {to} without passing through a width between {COLLAPSED} and \
         {EXPANDED}; the section's x frame by frame: {trace:?}"
    );
    assert_eq!(
        *trace.last().unwrap(),
        to,
        "the rail did not settle at {to}: {trace:?}"
    );
}

/// The defect: collapsing snapped the section 208px left in a single frame (FR-001, SC-001).
#[test]
fn collapsing_the_rail_slides_it_to_its_icons() {
    let mut surface = Surface::new(false);
    assert_eq!(surface.page_x(), EXPANDED, "precondition: mounted expanded");
    let trace = surface.toggle_and_trace();
    assert_slides(&trace, COLLAPSED);
}

/// And back, which is the half a one-way fix would miss (FR-001, SC-001).
#[test]
fn expanding_the_rail_slides_it_back_to_its_labels() {
    let mut surface = Surface::new(true);
    assert_eq!(
        surface.page_x(),
        COLLAPSED,
        "precondition: mounted collapsed"
    );
    let trace = surface.toggle_and_trace();
    assert_slides(&trace, EXPANDED);
}

/// A rail mounted in either state is already there: opening Settings is not a transition (FR-005,
/// SC-006).
#[test]
fn the_rail_is_mounted_at_its_width_rather_than_animating_to_it() {
    for (collapsed, width) in [(false, EXPANDED), (true, COLLAPSED)] {
        let mut surface = Surface::new(collapsed);
        let first = surface.page_x();
        let at = surface.next_instant();
        surface.frame(at, None);
        assert_eq!(
            (first, surface.page_x()),
            (width, width),
            "collapsed: {collapsed}"
        );
    }
}

/// The centre of `bounds`, where a press on it lands.
fn centre(bounds: Rectangle) -> Point {
    bounds.center()
}

/// The section region starts at the rail's right edge on every frame of both slides, and the
/// section's content keeps its offset from that edge, so nothing in the section moves except with
/// the edge (FR-002).
#[test]
fn the_section_follows_the_rail_edge() {
    for start_collapsed in [false, true] {
        let mut surface = Surface::new(start_collapsed);
        let rest = surface.bounds(PAGE_CONTENT).x - surface.bounds(PAGE).x;
        surface.toggle();
        let mut widths = Vec::new();
        for frame in 0..SETTLE {
            let at = surface.next_instant();
            surface.frame(at, None);
            let rail = surface.bounds(RAIL);
            let page = surface.bounds(PAGE);
            let content = surface.bounds(PAGE_CONTENT);
            widths.push(rail.width);
            assert_eq!(
                page.x,
                rail.x + rail.width,
                "frame {frame}: the section is not at the rail's edge"
            );
            let offset = content.x - page.x;
            assert!(
                (offset - rest).abs() <= ROUNDING,
                "frame {frame}: the section's content moved against the edge ({offset}, at rest {rest})"
            );
        }
        assert!(
            widths.iter().copied().any(between),
            "no frame had the rail between its widths, so nothing was followed: {widths:?}"
        );
    }
}

/// A second press mid-slide turns the rail back from the width it has reached, never jumping to a
/// rest width first, and never faster than an uninterrupted slide moves (FR-004, SC-004).
#[test]
fn a_second_press_reverses_from_where_it_is() {
    let mut reference = Surface::new(false);
    let full = reference.toggle_and_trace();
    let largest = std::iter::once(EXPANDED)
        .chain(full.iter().copied())
        .collect::<Vec<_>>()
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0_f32, f32::max);

    let mut surface = Surface::new(false);
    surface.toggle();
    let reached = (0..SETTLE)
        .find_map(|_| {
            let at = surface.next_instant();
            surface.frame(at, None);
            let x = surface.page_x();
            between(x).then_some(x)
        })
        .expect("precondition: the rail never reached a width between its two rest widths");

    surface.toggle();
    let mut previous = reached;
    for frame in 0..SETTLE {
        let at = surface.next_instant();
        surface.frame(at, None);
        let x = surface.page_x();
        assert!(
            (x - previous).abs() <= largest + 0.01,
            "frame {frame} of the reversal stepped {} (from {previous} to {x}); an uninterrupted \
             slide never steps more than {largest}",
            (x - previous).abs()
        );
        previous = x;
    }
    assert_eq!(previous, EXPANDED, "the reversal did not settle expanded");
}

/// Pump frames until the rail is between its rest widths, or fail the precondition.
fn pump_to_mid_slide(surface: &mut Surface) {
    for _ in 0..SETTLE {
        let at = surface.next_instant();
        surface.frame(at, None);
        if between(surface.bounds(RAIL).width) {
            return;
        }
    }
    panic!("precondition: the rail never reached a width between its two rest widths");
}

/// The rail's rows, top to bottom, and the collapse control under them, each cut to the part inside
/// the rail. Mid-slide a row draws a form wider than itself, so its uncut centre can lie past the
/// rail's edge, where nothing is pressed.
fn rail_controls(surface: &mut Surface) -> (Vec<Rectangle>, Rectangle) {
    let rail = surface.bounds(RAIL);
    let mut inside: Vec<Rectangle> = surface
        .focusables()
        .all
        .into_iter()
        .filter_map(|b| b.intersection(&rail))
        .collect();
    inside.sort_by(|a, b| a.y.total_cmp(&b.y));
    let control = inside.pop().expect("the rail has no controls");
    (inside, control)
}

/// What the user asked for takes effect on the press, whatever the rail is doing: the collapsed
/// flag flips before any frame (FR-010).
#[test]
fn the_state_flips_on_the_press() {
    let mut surface = Surface::new(false);
    let (_, control) = rail_controls(&mut surface);
    let messages = surface.press(centre(control));
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Settings(SettingsMsg::RailToggled))),
        "precondition: the press did not reach the collapse control: {messages:?}"
    );
    assert!(
        surface.under.state.settings.settings_rail_collapsed,
        "the flag waited for the slide instead of flipping on the press"
    );
}

/// A section chosen mid-slide is shown at once (FR-010).
#[test]
fn a_section_chosen_mid_slide_is_shown_at_once() {
    let mut surface = Surface::new(false);
    surface.toggle();
    pump_to_mid_slide(&mut surface);
    let shown = |s: &Surface| {
        s.under
            .state
            .settings
            .settings_draft
            .as_ref()
            .expect("precondition: Settings is open")
            .section
    };
    let before = shown(&surface);
    let (rows, _) = rail_controls(&mut surface);
    let (index, target) = SettingsSection::ALL
        .iter()
        .enumerate()
        .find(|(_, section)| **section != before)
        .expect("there is only one section");
    let messages = surface.press(centre(rows[index]));
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Settings(SettingsMsg::SectionShown(s)) if s == target)),
        "a row pressed mid-slide published nothing for {target:?}: {messages:?}"
    );
    assert_eq!(shown(&surface), *target, "the section waited for the slide");
}

/// Each of Settings' two exits closes the draft mid-slide (FR-010).
#[test]
fn an_exit_mid_slide_closes_settings() {
    let mut surface = Surface::new(false);
    for (exit, published) in [(SAVE, "Saved"), (CANCEL, "Cancelled")] {
        surface.send(Message::Settings(SettingsMsg::Opened));
        surface.toggle();
        pump_to_mid_slide(&mut surface);
        let target = centre(surface.bounds(exit));
        let messages = surface.press(target);
        assert!(
            messages.iter().any(|m| match m {
                Message::Settings(SettingsMsg::Saved) => published == "Saved",
                Message::Settings(SettingsMsg::Cancelled) => published == "Cancelled",
                _ => false,
            }),
            "{published} pressed mid-slide published nothing: {messages:?}"
        );
        assert!(
            surface.under.state.settings.settings_draft.is_none(),
            "{published} mid-slide left Settings open"
        );
    }
}

/// Once the slide has settled, the rail asks for no further frame (FR-011).
#[test]
fn a_settled_rail_asks_for_nothing() {
    let mut surface = Surface::new(false);
    let _ = surface.toggle_and_trace();
    let at = surface.next_instant();
    surface.frame(at, None);
    assert!(
        !surface.wants_a_frame(),
        "a frame after the slide settled asked for another"
    );
}

/// How far an icon may stray from the bound SC-007 puts on its step, in dp.
const HALF_PIXEL: f32 = 0.5;

/// One row of the rail as laid out now: its height, and its icon's x when it has an icon.
struct RowLayout {
    height: f32,
    icon_x: Option<f32>,
}

/// The rail's rows, top to bottom, the collapse control last: the nodes three levels under the rail
/// (its padded container, a column, a row). Structural, as `gates/rail_icons_align.rs` reads them,
/// so a destination added later is covered. The spacer between the destinations and the control has
/// no descendants and is left out. Rows are told apart by where they start rather than by index, so
/// a rail that keeps each form as its own column reads as one row per line: at each y the row whose
/// icon is drawn stands for the line.
///
/// A row's icon is its first drawn leaf in tree order: the glyph, which leads every row. Drawn means
/// inside the rail: a parked form keeps its size but is moved about −8.5e37 away
/// (`navigation_drawer::parked`), so it is never taken for it.
fn rail_rows(surface: &mut Surface) -> Vec<RowLayout> {
    let node = surface.node();
    let records = lay::walk(Layout::new(&node), lay::Layer::Base);
    let rail = records
        .iter()
        .find(|r| lay::path_token(&r.path) == RAIL)
        .expect("the rail is not laid out");
    let (left, right) = (rail.x, rail.x + rail.width);
    let rail = rail.path.clone();
    let depth = rail.len() + 3;
    let under =
        |r: &lay::LayoutRecord, p: &[usize]| r.path.len() > p.len() && r.path.starts_with(p);
    records
        .iter()
        .filter(|r| r.path.len() == depth && r.path.starts_with(&rail))
        .filter(|row| records.iter().any(|r| under(r, &row.path)))
        .map(|row| {
            let descendants: Vec<_> = records.iter().filter(|r| under(r, &row.path)).collect();
            let icon_x = descendants
                .iter()
                .find(|leaf| {
                    leaf.width > 0.0
                        && leaf.height > 0.0
                        && leaf.x >= left
                        && leaf.x + leaf.width <= right
                        && !descendants.iter().any(|r| under(r, &leaf.path))
                })
                .map(|leaf| leaf.x);
            (
                row.y,
                RowLayout {
                    height: row.height,
                    icon_x,
                },
            )
        })
        .fold(Vec::<(f32, RowLayout)>::new(), |mut lines, (y, row)| {
            match lines.iter_mut().find(|(at, _)| (at - y).abs() <= ROUNDING) {
                Some((_, line)) if line.icon_x.is_none() => *line = row,
                Some(_) => {}
                None => lines.push((y, row)),
            }
            lines
        })
        .into_iter()
        .map(|(_, row)| row)
        .collect()
}

/// How far through its width change the rail is: 0 collapsed, 1 expanded.
fn rail_fraction(width: f32) -> f32 {
    (width - COLLAPSED) / (EXPANDED - COLLAPSED)
}

/// No row reflows while the rail moves and no icon jumps: every row with an icon, and the collapse
/// control, keeps its rest height on every frame of both slides, and between frames each icon moves
/// no further than its two rest positions apart times the fraction of the width change made, plus
/// half a pixel, is within half a pixel of the line between its rest positions at that fraction,
/// and never moves away from where it is heading (FR-013, FR-014, SC-007).
#[test]
fn rows_keep_their_height_and_icons_their_line() {
    let expanded = rail_rows(&mut Surface::new(false));
    let collapsed = rail_rows(&mut Surface::new(true));
    assert_eq!(
        expanded.len(),
        collapsed.len(),
        "precondition: the rail has the same rows in both states"
    );
    let icon_rows: Vec<usize> = (0..expanded.len())
        .filter(|&i| expanded[i].icon_x.is_some())
        .collect();
    assert!(
        !icon_rows.is_empty()
            && (0..collapsed.len())
                .all(|i| collapsed[i].icon_x.is_some() == icon_rows.contains(&i)),
        "precondition: the rows with an icon are the same rows in both states"
    );

    for start_collapsed in [false, true] {
        let (from, to) = if start_collapsed {
            (&collapsed, &expanded)
        } else {
            (&expanded, &collapsed)
        };
        let mut surface = Surface::new(start_collapsed);
        let mut previous_x: Vec<Option<f32>> = from.iter().map(|r| r.icon_x).collect();
        let mut previous_f = rail_fraction(surface.bounds(RAIL).width);
        surface.toggle();
        let mut widths = Vec::new();
        for frame in 0..SETTLE {
            let at = surface.next_instant();
            surface.frame(at, None);
            let width = surface.bounds(RAIL).width;
            widths.push(width);
            let f = rail_fraction(width);
            let rows = rail_rows(&mut surface);
            assert_eq!(
                rows.len(),
                from.len(),
                "frame {frame}: the rail's rows changed"
            );
            for (index, row) in rows.iter().enumerate() {
                let (Some(x_labelled), Some(x_icons)) =
                    (expanded[index].icon_x, collapsed[index].icon_x)
                else {
                    continue;
                };
                assert_eq!(
                    row.height, from[index].height,
                    "frame {frame} (from collapsed: {start_collapsed}): row {index} reflowed at \
                     width {width}"
                );
                let x = row
                    .icon_x
                    .unwrap_or_else(|| panic!("frame {frame}: row {index} drew no icon"));
                let before = previous_x[index].expect("an icon row has an icon at rest");
                let bound = (x_labelled - x_icons).abs() * (f - previous_f).abs() + HALF_PIXEL;
                assert!(
                    (x - before).abs() <= bound,
                    "frame {frame} (from collapsed: {start_collapsed}): row {index}'s icon stepped \
                     {} (from {before} to {x}) at width {width}; the bound is {bound}",
                    (x - before).abs()
                );
                // The step bound alone lets an icon lag; it is on the line itself, every frame.
                let on_line = x_icons + (x_labelled - x_icons) * f;
                assert!(
                    (x - on_line).abs() <= HALF_PIXEL,
                    "frame {frame} (from collapsed: {start_collapsed}): row {index}'s icon is at {x}, \
                     off its line at {on_line} (fraction {f})"
                );
                let target = to[index].icon_x.expect("an icon row has an icon at rest");
                assert!(
                    (x - target).abs() <= (before - target).abs() + ROUNDING,
                    "frame {frame} (from collapsed: {start_collapsed}): row {index}'s icon moved \
                     away from {target} (from {before} to {x})"
                );
                previous_x[index] = Some(x);
            }
            previous_f = f;
        }
        assert!(
            widths.iter().copied().any(between),
            "precondition: no frame had the rail between its widths: {widths:?}"
        );
    }
}

/// The destination the covered state badges: Session service, sharing a credential.
const BADGED: SettingsSection = SettingsSection::Daemon;

/// A slot of a sliding row: its labelled form, the same form with its glyph tinted, and its
/// icons-only form, in that order (contract §3).
const LABELLED: usize = 0;
const MARKED: usize = 1;
const ICONS_ONLY: usize = 2;

/// A badged row never loses its mark to the slide: on every frame of both slides it draws a
/// tinted glyph, or a form whose badge chip is at least partly inside the rail (FR-015).
///
/// Read from layout. The tinted glyph is the marked slot, or the icons-only slot, which is tinted
/// whenever the row is badged (U4); either is drawn when it is not parked (moved about −8.5e37
/// away). Otherwise the labelled slot is drawn, and its chip is its last drawn leaf.
#[test]
fn a_badged_row_keeps_its_mark() {
    let index = SettingsSection::ALL
        .iter()
        .position(|s| *s == BADGED)
        .expect("the badged section is not in the rail");
    for start_collapsed in [false, true] {
        let mut surface = Surface::new(start_collapsed);
        surface.toggle();
        let mut widths = Vec::new();
        for frame in 0..SETTLE {
            let at = surface.next_instant();
            surface.frame(at, None);
            let node = surface.node();
            let records = lay::walk(Layout::new(&node), lay::Layer::Base);
            let rail = records
                .iter()
                .find(|r| lay::path_token(&r.path) == RAIL)
                .expect("the rail is not laid out");
            let bounds = Rectangle::new(
                Point::new(rail.x, rail.y),
                Size::new(rail.width, rail.height),
            );
            widths.push(rail.width);
            let mut row = rail.path.clone();
            row.extend([0, 0, index]);
            let slot = |k: usize| {
                let mut path = row.clone();
                path.push(k);
                records
                    .iter()
                    .find(|r| r.path == path)
                    .unwrap_or_else(|| panic!("frame {frame}: the badged row has no slot {k}"))
            };
            let drawn = |r: &lay::LayoutRecord| r.x > -1.0e6;
            if drawn(slot(MARKED)) || drawn(slot(ICONS_ONLY)) {
                continue;
            }
            let labelled = slot(LABELLED);
            assert!(
                drawn(labelled),
                "frame {frame}: the badged row drew no form"
            );
            let under = |r: &&lay::LayoutRecord, p: &[usize]| {
                r.path.len() > p.len() && r.path.starts_with(p)
            };
            let chip = records
                .iter()
                .filter(|r| under(r, &labelled.path) && r.width > 0.0 && r.height > 0.0)
                .rfind(|leaf| !records.iter().any(|r| under(&r, &leaf.path)))
                .unwrap_or_else(|| panic!("frame {frame}: the labelled form has no leaves"));
            let chip = Rectangle::new(
                Point::new(chip.x, chip.y),
                Size::new(chip.width, chip.height),
            );
            assert!(
                bounds.intersects(&chip),
                "frame {frame} (from collapsed: {start_collapsed}): the badged row drew its \
                 untinted form with its chip {chip:?} outside the rail {bounds:?}"
            );
        }
        assert!(
            widths.iter().copied().any(between),
            "precondition: no frame had the rail between its widths: {widths:?}"
        );
    }
}

impl Surface {
    /// Deliver `event` alone with the pointer at `cursor`, over one layout, without feeding what it
    /// publishes through the reducer. Returns the messages published.
    fn event(&mut self, event: iced::Event, cursor: Option<Point>) -> Vec<Message> {
        let node = self.node();
        let mut messages = Vec::new();
        let _ = deliver(
            &self.under,
            &mut self.tree,
            &self.renderer,
            &node,
            &event,
            cursor,
            &mut messages,
        );
        messages
    }

    /// What the pointer would look like at `at` over the surface as it is laid out now.
    fn interaction(&mut self, at: Point) -> mouse::Interaction {
        let node = self.node();
        lay::view_of(&self.under).as_widget().mouse_interaction(
            &self.tree,
            Layout::new(&node),
            mouse::Cursor::Available(at),
            &Rectangle::with_size(WINDOW),
            &self.renderer,
        )
    }

    /// The pixels of `region` as the surface draws now, with no pointer over it.
    fn pixels(&mut self, region: Rectangle) -> Vec<u8> {
        use iced::advanced::renderer::{Headless as _, Style};
        use iced::advanced::Renderer as _;
        let node = self.node();
        // A fresh frame: what an earlier draw queued would otherwise be drawn again under this one.
        self.renderer.reset(Rectangle::with_size(WINDOW));
        lay::view_of(&self.under).as_widget().draw(
            &self.tree,
            &mut self.renderer,
            &iced::Theme::Dark,
            &Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(WINDOW),
        );
        let (width, height) = (WINDOW.width as u32, WINDOW.height as u32);
        let shot = self
            .renderer
            .screenshot(Size::new(width, height), 1.0, iced::Color::BLACK);
        let (left, top) = (region.x.floor() as u32, region.y.floor() as u32);
        let (right, bottom) = (
            (region.x + region.width).ceil() as u32,
            (region.y + region.height).ceil() as u32,
        );
        (top..bottom.min(height))
            .flat_map(|y| (left..right.min(width)).map(move |x| (y, x)))
            .flat_map(|(y, x)| {
                let i = ((y * width + x) * 4) as usize;
                shot[i..i + 4].to_vec()
            })
            .collect()
    }

    /// Pump frames until the rail settles, as [`SETTLE`] bounds it.
    fn settle(&mut self) {
        for _ in 0..SETTLE {
            let at = self.next_instant();
            self.frame(at, None);
        }
    }
}

/// The node of a rail row's slot `slot` (0 labelled, 1 marked, 2 icons-only), for row `index`.
fn slot_bounds(surface: &mut Surface, index: usize, slot: usize) -> Rectangle {
    let rail = RAIL
        .split('/')
        .map(|p| p.parse::<usize>().unwrap())
        .chain([0, 0, index, slot])
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("/");
    surface.bounds(&rail)
}

/// The bounds of rail row `index` itself.
fn row_bounds(surface: &mut Surface, index: usize) -> Rectangle {
    let row = RAIL
        .split('/')
        .map(|p| p.parse::<usize>().unwrap())
        .chain([0, 0, index])
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join("/");
    surface.bounds(&row)
}

/// The keyboard is held to the rail's rest behaviour while it moves: the collapse control focused
/// by keyboard keeps focus on every frame of both slides, including each row's change of form; the
/// number of controls the keyboard reaches never changes, so no parked copy of a row becomes
/// reachable; and the section's controls are reached in the same order as at rest (FR-007, FR-008,
/// SC-003).
#[test]
fn the_keyboard_stays_on_the_collapse_control_and_reaches_nothing_hidden() {
    for start_collapsed in [false, true] {
        let mut surface = Surface::new(start_collapsed);
        // The section's controls in traversal order, each by its line, its height and its place
        // on that line, including controls scrolled out of view. A control that fills the section,
        // or is right-aligned in it, widens or moves with the width the rail gives up, which is the
        // point of collapsing, so its x and width are not part of what it is; its order among the
        // controls on its line is.
        let section = |surface: &mut Surface| {
            let page = surface.bounds(PAGE);
            // By left edge: a rail row's form wider than the row reaches into the section.
            let all: Vec<Rectangle> = surface
                .focusables()
                .all
                .into_iter()
                .filter(|b| b.x >= page.x - ROUNDING)
                .collect();
            all.iter()
                .map(|b| {
                    let place = all.iter().filter(|o| o.y == b.y && o.x < b.x).count();
                    (b.y, b.height, place)
                })
                .collect::<Vec<_>>()
        };
        // Nothing the keyboard reaches is off screen: a parked form sits about −8.5e37 away, so a
        // hidden copy of a row that became reachable would show here, at rest as much as moving.
        let hidden = |all: &[Rectangle]| {
            all.iter()
                .filter(|b| b.intersection(&Rectangle::with_size(WINDOW)).is_none())
                .count()
        };
        let rest_count = surface.focusables().all.len();
        assert_eq!(
            hidden(&surface.focusables().all),
            0,
            "(from collapsed: {start_collapsed}) the keyboard reaches a control off screen at rest"
        );
        let rest_section = section(&mut surface);
        let (_, control) = rail_controls(&mut surface);
        surface.run(FocusAt(control));
        assert_eq!(surface.focusables().focused, Some(control), "precondition");

        surface.toggle();
        let mut widths = Vec::new();
        for frame in 0..SETTLE {
            let at = surface.next_instant();
            surface.frame(at, None);
            widths.push(surface.bounds(RAIL).width);
            let now = surface.focusables();
            assert_eq!(
                hidden(&now.all),
                0,
                "frame {frame} (from collapsed: {start_collapsed}): the keyboard reaches a control \
                 off screen"
            );
            assert_eq!(
                now.all.len(),
                rest_count,
                "frame {frame} (from collapsed: {start_collapsed}): the keyboard reaches a \
                 different number of controls while the rail moves"
            );
            let (_, control) = rail_controls(&mut surface);
            let rail = surface.bounds(RAIL);
            assert_eq!(
                now.focused.and_then(|f| f.intersection(&rail)),
                Some(control),
                "frame {frame} (from collapsed: {start_collapsed}): focus left the collapse control"
            );
            let now_section = section(&mut surface);
            assert!(
                now_section == rest_section,
                "frame {frame} (from collapsed: {start_collapsed}): the section's controls are not \
                 reached as at rest: {now_section:?} against {rest_section:?}"
            );
        }
        assert!(
            widths.iter().copied().any(between),
            "precondition: no frame had the rail between its widths: {widths:?}"
        );
    }
}

/// A click on **Collapse** takes the keyboard without showing it, and the slide does not change
/// that: once settled the control is focused and draws exactly as it does unfocused (FR-006,
/// FR-007).
#[test]
fn a_click_on_collapse_shows_no_focus_ring() {
    let mut surface = Surface::new(false);
    let (_, control) = rail_controls(&mut surface);
    let messages = surface.press(centre(control));
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Settings(SettingsMsg::RailToggled))),
        "precondition: the press did not reach the collapse control: {messages:?}"
    );
    surface.settle();
    let (_, control) = rail_controls(&mut surface);
    assert_eq!(
        surface.focusables().focused,
        Some(control),
        "the clicked collapse control lost the keyboard to the slide"
    );
    let focused = surface.pixels(control);
    surface.run(FocusAt(Rectangle::default()));
    assert_eq!(
        surface.focusables().focused,
        None,
        "precondition: focus cleared"
    );
    let unfocused = surface.pixels(control);
    assert!(
        focused == unfocused,
        "the clicked collapse control draws a focus indicator once the slide settles"
    );
}

/// The row the credential badge marks.
fn badged_index() -> usize {
    SettingsSection::ALL
        .iter()
        .position(|s| *s == BADGED)
        .expect("the badged section is not in the rail")
}

/// Share or stop sharing the credential that badges the session-service row, and rebuild.
fn share_credential(surface: &mut Surface, share: bool) {
    let credentials = &mut surface
        .under
        .state
        .settings
        .settings_draft
        .as_mut()
        .expect("precondition: Settings is open")
        .daemon
        .profile
        .credentials;
    if share {
        credentials.insert(micold_core::sandbox::CredentialShare::GitConfig);
    } else {
        credentials.clear();
    }
    surface.rebuild();
}

/// A badge that clears and comes back while the rail moves changes which form the row draws, and
/// the keyboard stays on that row's control through both changes (FR-007, FR-015).
#[test]
fn a_badge_appearing_mid_slide_keeps_focus() {
    let index = badged_index();
    let mut surface = Surface::new(false);
    let (rows, _) = rail_controls(&mut surface);
    surface.run(FocusAt(rows[index]));
    assert_eq!(
        surface.focusables().focused,
        Some(rows[index]),
        "precondition"
    );
    assert_eq!(
        rows[index].y,
        row_bounds(&mut surface, index).y,
        "precondition: the badged row"
    );

    let check = |surface: &mut Surface, when: &str| {
        let row = row_bounds(surface, index);
        let focused = surface
            .focusables()
            .focused
            .unwrap_or_else(|| panic!("{when}: the badged row's control lost the keyboard"));
        assert!(
            focused.y == row.y && focused.height == row.height,
            "{when}: focus moved to {focused:?}, off the badged row {row:?}"
        );
    };
    surface.toggle();
    pump_to_mid_slide(&mut surface);
    check(&mut surface, "mid-slide");
    share_credential(&mut surface, false);
    for frame in 0..3 {
        let at = surface.next_instant();
        surface.frame(at, None);
        check(
            &mut surface,
            &format!("frame {frame} with the badge cleared"),
        );
    }
    share_credential(&mut surface, true);
    for frame in 0..SETTLE {
        let at = surface.next_instant();
        surface.frame(at, None);
        check(&mut surface, &format!("frame {frame} with the badge back"));
    }
}

/// The index of a rail row that is neither current nor badged.
fn plain_index(surface: &Surface) -> usize {
    let current = surface
        .under
        .state
        .settings
        .settings_draft
        .as_ref()
        .expect("precondition: Settings is open")
        .section;
    SettingsSection::ALL
        .iter()
        .position(|s| *s != current && *s != BADGED)
        .expect("the rail has no plain row")
}

/// Whether `messages` holds anything the rail publishes.
fn rail_message(messages: &[Message]) -> bool {
    messages.iter().any(|m| {
        matches!(
            m,
            Message::Settings(SettingsMsg::SectionShown(_) | SettingsMsg::RailToggled)
        )
    })
}

/// Mid-collapse a row's labelled form is wider than the row; the part past the row's edge is cut
/// off, and the pointer over it reaches nothing (FR-009).
#[test]
fn a_pointer_past_a_row_reaches_nothing() {
    let mut surface = Surface::new(false);
    let index = plain_index(&surface);
    surface.toggle();
    pump_to_mid_slide(&mut surface);
    let row = row_bounds(&mut surface, index);
    let form = slot_bounds(&mut surface, index, 0);
    let past = Point::new(row.x + row.width + 2.0, row.center_y());
    assert!(
        form.contains(past) && !row.contains(past),
        "precondition: {past:?} is inside the drawn labelled form {form:?} and outside its row \
         {row:?}"
    );
    let at = surface.next_instant();
    surface.frame(at, Some(past));
    assert_eq!(
        surface.interaction(past),
        mouse::Interaction::None,
        "the pointer past the row's edge hovers something"
    );
    let messages = surface.press(past);
    assert!(
        !rail_message(&messages),
        "a press past the row's edge published {messages:?}"
    );
}

/// Pump frames on a collapsing rail until its fraction is below `below`, returning it.
fn pump_until_fraction_below(surface: &mut Surface, below: f32) -> f32 {
    for _ in 0..SETTLE {
        let at = surface.next_instant();
        surface.frame(at, None);
        let f = rail_fraction(surface.bounds(RAIL).width);
        if f < below {
            return f;
        }
    }
    panic!("precondition: the rail never came within {below} of collapsed");
}

/// A press held on a row while its form parks is let go with the release, so the row is not left
/// held down: once the rail is back, a release with no press before it publishes nothing (FR-009).
#[test]
fn a_press_held_as_its_form_parks_is_released() {
    let mut surface = Surface::new(false);
    let index = plain_index(&surface);
    surface.toggle();
    let f = pump_until_fraction_below(&mut surface, 0.02);
    assert!(
        f > 0.001,
        "precondition: the row's labelled form is still drawn at {f}"
    );
    let row = centre(row_bounds(&mut surface, index));
    let _ = surface.event(
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Some(row),
    );
    pump_until_fraction_below(&mut surface, 0.001);
    let row = centre(row_bounds(&mut surface, index));
    let released = surface.event(
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Some(row),
    );
    assert!(
        !rail_message(&released),
        "precondition: the release over the icons-only form published {released:?}"
    );
    surface.settle();
    surface.toggle();
    surface.settle();
    let row = centre(row_bounds(&mut surface, index));
    let messages = surface.event(
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        Some(row),
    );
    assert!(
        !rail_message(&messages),
        "a row left held down by the slide published {messages:?} on a bare release"
    );
}

/// A ripple started just before its row's form parks runs out rather than freezing: after the slide
/// has settled the parked ripple still asks for frames, and once it is over nothing does (FR-009,
/// FR-011).
#[test]
fn a_ripple_in_a_parked_form_settles() {
    let mut surface = Surface::new(false);
    let index = plain_index(&surface);
    surface.toggle();
    let f = pump_until_fraction_below(&mut surface, 0.02);
    assert!(
        f > 0.001,
        "precondition: the row's labelled form is still drawn at {f}"
    );
    let row = centre(row_bounds(&mut surface, index));
    let _ = surface.event(
        iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Some(row),
    );
    let _ = surface.event(
        iced::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        None,
    );
    let pressed = surface.frame;
    let ripple = (duration::MEDIUM_2 + duration::SHORT_4) as u32;
    let mut settled = None;
    loop {
        let at = surface.next_instant();
        surface.frame(at, None);
        let elapsed = (surface.frame - pressed) * FRAME.as_millis() as u32;
        let width = surface.bounds(RAIL).width;
        if settled.is_none() && width == COLLAPSED {
            settled = Some(elapsed);
        }
        if let Some(settled) = settled {
            if elapsed > settled && elapsed + 2 * FRAME.as_millis() as u32 <= ripple {
                assert!(
                    surface.wants_a_frame(),
                    "{elapsed} ms after the press, the slide settled at {settled} ms and the \
                     ripple has {} ms left, but nothing asked for a frame",
                    ripple - elapsed
                );
            }
        }
        if elapsed > ripple + 4 * FRAME.as_millis() as u32 && settled.is_some() {
            break;
        }
        assert!(elapsed < 10_000, "the slide never settled");
    }
    assert!(
        settled.unwrap() + 2 * (FRAME.as_millis() as u32) < ripple,
        "precondition: the slide settled at {} ms, too late to watch the ripple outlive it",
        settled.unwrap()
    );
    let at = surface.next_instant();
    surface.frame(at, None);
    assert!(
        !surface.wants_a_frame(),
        "a frame after the ripple and the slide were both over asked for another"
    );
}

/// Choosing another section mid-slide moves the icons of the two rows whose selection changed by
/// the current row's extra inset at once, and leaves every other row where the slide has it; the
/// slide carries on to rest (FR-014's exception, US2 scenario 4).
#[test]
fn selecting_mid_slide_moves_only_the_changed_icons() {
    let mut surface = Surface::new(false);
    let current = surface
        .under
        .state
        .settings
        .settings_draft
        .as_ref()
        .expect("precondition: Settings is open")
        .section;
    let old = SettingsSection::ALL
        .iter()
        .position(|s| *s == current)
        .unwrap();
    let new = plain_index(&surface);
    surface.toggle();
    pump_to_mid_slide(&mut surface);
    let f = rail_fraction(surface.bounds(RAIL).width);
    let before = rail_rows(&mut surface);
    let (rows, _) = rail_controls(&mut surface);
    let messages = surface.press(centre(rows[new]));
    assert!(
        rail_message(&messages),
        "precondition: the press chose nothing"
    );
    let after = rail_rows(&mut surface);
    for (i, (b, a)) in before.iter().zip(&after).enumerate() {
        let (Some(b), Some(a)) = (b.icon_x, a.icon_x) else {
            continue;
        };
        let expected = if i == new {
            12.0 * f
        } else if i == old {
            -12.0 * f
        } else {
            0.0
        };
        assert!(
            ((a - b) - expected).abs() <= HALF_PIXEL,
            "row {i}'s icon moved {} on the selection at fraction {f}; expected {expected}",
            a - b
        );
    }
    surface.settle();
    assert_eq!(
        surface.bounds(RAIL).width,
        COLLAPSED,
        "the slide did not settle after the selection"
    );
}

/// A click on **Collapse** draws nothing past the rail while it collapses: the press's ripple is cut
/// off at the row's edge with the rest of the form. A pushed clip replaces the enclosing one rather
/// than intersecting with it, so a row that clips its form must hand its children a viewport cut
/// to the row, or the ripple paints over the section (FR-009, FR-013; found by the M2 visual pass).
#[test]
fn a_ripple_is_cut_off_at_the_rail() {
    let mut clicked = Surface::new(false);
    let mut toggled = Surface::new(false);
    let (_, control) = rail_controls(&mut clicked);
    let messages = clicked.press(centre(control));
    assert!(
        messages
            .iter()
            .any(|m| matches!(m, Message::Settings(SettingsMsg::RailToggled))),
        "precondition: the press did not reach the collapse control: {messages:?}"
    );
    toggled.toggle();
    let mut compared = 0;
    for frame in 0..SETTLE {
        let at = clicked.next_instant();
        clicked.frame(at, None);
        let at = toggled.next_instant();
        toggled.frame(at, None);
        let rail = clicked.bounds(RAIL);
        assert_eq!(
            rail,
            toggled.bounds(RAIL),
            "precondition: the two slides diverged"
        );
        if !between(rail.width) {
            continue;
        }
        let (_, row) = rail_controls(&mut clicked);
        let past = Rectangle::new(
            Point::new(rail.x + rail.width, row.y),
            Size::new(EXPANDED - rail.width, row.height),
        );
        assert!(
            clicked.pixels(past) == toggled.pixels(past),
            "frame {frame}: the clicked control drew past the rail's edge at {} into {past:?}",
            rail.width
        );
        compared += 1;
    }
    assert!(
        compared > 0,
        "precondition: no frame had the rail between its widths"
    );
}
