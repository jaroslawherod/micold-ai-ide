//! A row's tooltip never covers the row it describes (029 BUG-001, FR-013, SC-006).
//!
//! The tooltip opens below its row. For the last row of a list that reaches the bottom of the
//! window there is no "below", and the rendering stack's own tooltip answered that by sliding the
//! panel back inside the window — up, over the row. The row's name, chips and actions went under
//! the panel that was meant to describe them.
//!
//! Every other gate passes that state. `containment` asks whether the panel stays inside the
//! window, and it did: staying inside the window is exactly what moved it. `panel_placement`
//! reads panels against the app bar. `context_menu_anchor` reads a *menu* against the press that
//! opened it. None of them reads a floating panel against **the element it belongs to**, which is
//! the question here.
//!
//! # It drives the hover, not the state
//!
//! A tooltip's openness is widget state, not application state, so there is no `State` to build
//! with it open. The gate dispatches a real `CursorMoved` over the real row into a retained tree
//! and reads whatever overlay that tree then reports. The tooltip's delay is zero, so it opens on
//! that one event.
//!
//! # Compiled into the `layout_snapshot` binary
//!
//! Beside `context_menu_anchor`, whose fixture it reuses — the same long worktree list, the same
//! records, the same row paths — rather than restating it.

use crate::context_menu_anchor::{sidebar_row, with_worktrees};
use crate::support::layout::{self as lay, LayoutRecord};
use iced::Size;
use micold_client::app::{State, MIN_WINDOW_SIZE};
use micold_client::features::connection::ConnectionStatus;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The first worktree row — past the "Default" project-root row, which is row 0.
const FIRST_WORKTREE_ROW: usize = 1;

/// Far more worktrees than any window here can show unscrolled. The search below stops well before
/// it; this only bounds the search.
const MOST_WORKTREES: usize = 60;

/// A list longer than any window here, as in the report (25 worktrees), so it scrolls.
const OVERFLOWING_LIST: usize = 25;

/// A list short enough that its first row has the rest of the window below it.
const SHORT_LIST: usize = 3;

/// Snaps (or, with `snap` off, only reads) every scrollable whose content holds `target`, and
/// records the translation the last of them reports.
struct SnapToEnd {
    target: iced::Rectangle,
    snap: bool,
    translation: Option<iced::Vector>,
}

impl SnapToEnd {
    fn new(target: iced::Rectangle, snap: bool) -> Self {
        Self {
            target,
            snap,
            translation: None,
        }
    }
}

impl iced::advanced::widget::Operation for SnapToEnd {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&iced::advanced::widget::Id>,
        _bounds: iced::Rectangle,
        content_bounds: iced::Rectangle,
        translation: iced::Vector,
        state: &mut dyn iced::advanced::widget::operation::Scrollable,
    ) {
        if content_bounds.contains(self.target.center()) {
            if self.snap {
                state.snap_to(iced::widget::scrollable::RelativeOffset {
                    x: None,
                    y: Some(1.0),
                });
            }
            self.translation = Some(translation);
        }
    }
}

/// Whether the list is scrolled before the hover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Scroll {
    /// Left where it starts, at the top.
    None,
    /// Scrolled to its end, as the report's steps do.
    ToEnd,
}

/// What one hover produced: the row that was hovered, and the tooltip panel it opened.
struct Hovered {
    row: LayoutRecord,
    panel: Option<LayoutRecord>,
}

/// Lay `state` out in a window of `size`, move the pointer onto sidebar row `index`, and read the
/// row and the tooltip panel that opened over it.
///
/// Its own harness rather than `support::layout::resolve`, because that one lays out against the
/// fixed `WINDOW` and this gate's worst case is the smallest window there is.
fn hover_row(state: &State, index: usize, size: Size, scroll: Scroll) -> Hovered {
    use iced::advanced::widget::Tree;
    use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
    use iced::Rectangle;

    let renderer = lay::renderer();
    let mut element = view(state);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, size);
    let mut node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(size);

    // Hand over the frames an entrance needs before it takes input, at this window's size.
    lay::settle(
        &mut element,
        &mut tree,
        &node,
        &renderer,
        std::time::Instant::now(),
        0..lay::SETTLE_FRAMES,
        size,
    );
    let mut ignored = Vec::new();

    let path = sidebar_row(index);
    let records = lay::walk(Layout::new(&node), lay::Layer::Base);
    let mut row = records
        .iter()
        .find(|r| r.path == path)
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "no sidebar row at {} — the sidebar changed shape, so re-point `sidebar_row` \
                 against layout_snapshot.txt",
                lay::path_token(&path)
            )
        });
    assert!(
        row.width > 0.0 && row.height > 0.0,
        "the row at {} has no area, so a hover lands on nothing",
        lay::path_token(&path)
    );

    if scroll == Scroll::ToEnd {
        // Snap the scrollable holding the row to its end — the app's own `snap_to`, not a wheel
        // event whose travel would be this file's guess — then read back how far it moved. Layout
        // records do not move with the scroll (a scrollable offsets what it draws, not where its
        // content is laid out), so the row is placed where it now appears on screen.
        let target = iced::Rectangle::new(
            iced::Point::new(row.x, row.y),
            Size::new(row.width, row.height),
        );
        let mut snap = SnapToEnd::new(target, true);
        element
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut snap);
        let mut read = SnapToEnd::new(target, false);
        element
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut read);
        let moved = read.translation.unwrap_or_else(|| {
            panic!(
                "no scrollable holds the row at {}, so the list cannot be scrolled",
                lay::path_token(&path)
            )
        });
        assert!(
            moved.y > row.height,
            "snapping the list to its end moved it {:.0}px, so it does not overflow the window \
             and this case is the unscrolled one again",
            moved.y
        );
        row.y -= moved.y;
    }

    let before = overlay_records(&mut element, &mut tree, &node, &renderer, size);

    let point = iced::Point::new(row.x + row.width / 2.0, row.y + row.height / 2.0);
    let mut shell = Shell::new(&mut ignored);
    element.as_widget_mut().update(
        &mut tree,
        &iced::Event::Mouse(mouse::Event::CursorMoved { position: point }),
        Layout::new(&node),
        mouse::Cursor::Available(point),
        &renderer,
        &mut clipboard::Null,
        &mut shell,
        &viewport,
    );
    drop(shell);
    node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    let after = overlay_records(&mut element, &mut tree, &node, &renderer, size);

    Hovered {
        row,
        panel: panel_that_opened(&before, &after, size),
    }
}

/// Every record of `element`'s overlay layer, laid out in a window of `size`.
fn overlay_records<'a, M: 'a>(
    element: &mut iced::Element<'a, M>,
    tree: &mut iced::advanced::widget::Tree,
    node: &iced::advanced::layout::Node,
    renderer: &iced::Renderer,
    size: Size,
) -> Vec<LayoutRecord> {
    use iced::advanced::Layout;
    use iced::{Rectangle, Vector};

    element
        .as_widget_mut()
        .overlay(
            tree,
            Layout::new(node),
            renderer,
            &Rectangle::with_size(size),
            Vector::ZERO,
        )
        .map(|mut overlay| {
            let overlay_node = overlay.as_overlay_mut().layout(renderer, size);
            lay::walk(Layout::new(&overlay_node), lay::Layer::Overlay)
        })
        .unwrap_or_default()
}

/// The tooltip's visible surface: the one panel the hover added, less its transparent margin.
///
/// Overlays nest — every container that has more than one child with an overlay groups them, and a
/// group is laid out at the size of the whole window — so a panel is a new overlay node that is
/// **not** window-sized, and the panel the hover opened is the shallowest of them. Its first child
/// is the styled surface a person sees; the panel around it is margin drawn as nothing.
///
/// A hover that added nothing returns `None`, which the tests below report as its own failure: a
/// row without a tooltip is a different defect from one covered by it. A hover that opened two
/// panels — the pointer on a chip or an action with a tooltip of its own — fails here, because
/// which of them to measure would be a guess.
fn panel_that_opened(
    before: &[LayoutRecord],
    after: &[LayoutRecord],
    size: Size,
) -> Option<LayoutRecord> {
    let added: Vec<&LayoutRecord> = after
        .iter()
        .filter(|r| {
            !((r.width - size.width).abs() < TOLERANCE
                && (r.height - size.height).abs() < TOLERANCE)
        })
        .filter(|r| !before.contains(r))
        .collect();
    let depth = added.iter().map(|r| r.path.len()).min()?;
    let panels: Vec<&&LayoutRecord> = added.iter().filter(|r| r.path.len() == depth).collect();
    assert_eq!(
        panels.len(),
        1,
        "one hover opened {} tooltip panels, so which of them is the row's is a guess — the \
         pointer is on something inside the row with a tooltip of its own",
        panels.len()
    );
    let panel = panels[0];
    let mut surface = panel.path.clone();
    surface.push(0);
    let visible = after.iter().find(|r| r.path == surface).unwrap_or(panel);
    Some(visible.clone())
}

fn view(state: &State) -> iced::Element<'_, micold_client::app::Message> {
    micold_client::ui::view(
        state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &ConnectionStatus::Connected,
        &micold_client::features::sandbox::Sandbox::default(),
    )
}

/// Every base record of `state` laid out in a window of `size`, without hovering anything.
fn base_records(state: &State, size: Size) -> Vec<LayoutRecord> {
    use iced::advanced::widget::Tree;
    use iced::advanced::{layout, Layout};

    let renderer = lay::renderer();
    let mut element = view(state);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, size),
    );
    lay::walk(Layout::new(&node), lay::Layer::Base)
}

/// Where `path`'s content stops being visible: the nearest bottom edge among it and its ancestors.
///
/// A scrollable's own record is its viewport while its content column runs on below it, so this is
/// the scrollable's bottom for a row inside one — or the bottom of whatever clips it, should the
/// sidebar grow a footer under the list.
fn visible_bottom(records: &[LayoutRecord], path: &[usize]) -> f32 {
    (0..path.len())
        .filter_map(|depth| records.iter().find(|r| r.path == path[..depth]))
        .map(|r| r.y + r.height)
        .fold(f32::INFINITY, f32::min)
}

/// The longest worktree list whose last row still sits inside a window of `size`, unscrolled, with
/// its centre — where the hover lands — on screen, and that list's state.
///
/// The centre rather than the whole row: once a list overflows, its scrollable stops a few pixels
/// short of the window's bottom edge, and the row that reaches the bottom edge is the one BUG-001
/// is about. A row whose centre is clipped would take no hover at all, and the gate would report a
/// missing tooltip rather than a misplaced one.
///
/// Measured rather than stated: the row pitch is the sidebar's business, and a count that is right
/// today puts the last row below the fold — where it cannot be hovered at all — the day a row grows
/// a pixel. Each count is laid out on its own, because a list that overflows is laid out
/// differently from one that fits.
fn fullest_unscrolled_list(size: Size) -> (State, usize) {
    let mut fullest = None;
    // Row 0 is the Default row, so the last worktree row of a list of `count` is row `count`.
    for count in 1..=MOST_WORKTREES {
        let state = with_worktrees(Vec::new(), count, size);
        let records = base_records(&state, size);
        let path = sidebar_row(count);
        let visible = records.iter().find(|r| r.path == path).is_some_and(|r| {
            r.y + r.height <= size.height + TOLERANCE
                && r.y + r.height / 2.0 < visible_bottom(&records, &path)
        });
        if !visible {
            break;
        }
        fullest = Some((state, count));
    }
    let (state, count) = fullest.unwrap_or_else(|| {
        panic!(
            "not even one worktree row can be hovered in a {}×{} window",
            size.width, size.height
        )
    });
    assert!(
        count < MOST_WORKTREES,
        "{MOST_WORKTREES} worktree rows all fit a {}×{} window, so the list never reaches the \
         bottom of it and the last row is not where this gate needs it",
        size.width,
        size.height
    );
    (state, count)
}

fn intersects(a: &LayoutRecord, b: &LayoutRecord) -> bool {
    let overlap_x = (a.x + a.width).min(b.x + b.width) - a.x.max(b.x);
    let overlap_y = (a.y + a.height).min(b.y + b.height) - a.y.max(b.y);
    overlap_x > TOLERANCE && overlap_y > TOLERANCE
}

fn describe(r: &LayoutRecord) -> String {
    format!(
        "{:.0},{:.0} {:.0}×{:.0} (y {:.0}–{:.0})",
        r.x,
        r.y,
        r.width,
        r.height,
        r.y,
        r.y + r.height
    )
}

/// The shared body: in a window of `size`, the last row of the fullest list keeps its tooltip off
/// itself, and the tooltip stays inside the window.
fn assert_last_row_keeps_its_tooltip_off_itself(size: Size) {
    let (state, last) = fullest_unscrolled_list(size);
    let hovered = hover_row(&state, last, size, Scroll::None);
    let row = &hovered.row;

    assert!(
        size.height - (row.y + row.height) < row.height,
        "the last worktree row ends at y={:.0} in a {:.0}px window — more than a row's height \
         ({:.0}px) above its bottom edge, so this fixture no longer reaches the case BUG-001 is \
         about",
        row.y + row.height,
        size.height,
        row.height
    );

    let panel = hovered.panel.unwrap_or_else(|| {
        panic!(
            "hovering the last worktree row at {} opened no tooltip, so the row has lost its \
             tooltip — a different defect from the one this gate guards",
            describe(row)
        )
    });

    assert!(
        !intersects(&panel, row),
        "in a {:.0}×{:.0} window, the last worktree row's tooltip covers the row it describes: \
         row {}, tooltip {}. The tooltip opens below its row, or on the other side when there is \
         no room below — never over it (FR-013, SC-006, BUG-001)",
        size.width,
        size.height,
        describe(row),
        describe(&panel)
    );

    assert!(
        panel.x >= -TOLERANCE
            && panel.y >= -TOLERANCE
            && panel.x + panel.width <= size.width + TOLERANCE
            && panel.y + panel.height <= size.height + TOLERANCE,
        "the tooltip {} left the {:.0}×{:.0} window (SC-005)",
        describe(&panel),
        size.width,
        size.height
    );
}

/// The reported case: the last row of a full list, at the bottom of the window (T031).
#[test]
fn the_last_row_keeps_its_tooltip_off_itself() {
    assert_last_row_keeps_its_tooltip_off_itself(lay::WINDOW);
}

/// The same, in the smallest window the application allows (T032).
#[test]
fn the_last_row_keeps_its_tooltip_off_itself_in_the_smallest_window() {
    assert_last_row_keeps_its_tooltip_off_itself(MIN_WINDOW_SIZE);
}

/// Unchanged behaviour: a row with room below it gets its tooltip below it, clear of it (T032).
#[test]
fn a_row_with_room_below_gets_its_tooltip_below_it() {
    // Any list with rows below the first will do; a short one keeps its first row far from every
    // edge.
    let state = with_worktrees(Vec::new(), SHORT_LIST, lay::WINDOW);
    let hovered = hover_row(&state, FIRST_WORKTREE_ROW, lay::WINDOW, Scroll::None);
    let row = &hovered.row;
    let panel = hovered.panel.unwrap_or_else(|| {
        panic!(
            "hovering the first worktree row at {} opened no tooltip",
            describe(row)
        )
    });

    assert!(
        panel.y >= row.y + row.height - TOLERANCE,
        "the first worktree row has the whole list below it, so its tooltip opens below it: row \
         {}, tooltip {} (FR-010, FR-013)",
        describe(row),
        describe(&panel)
    );
}

/// The report's own steps: a list longer than the window, scrolled to its end, and its last row
/// hovered (BUG-001 Reproduction, quickstart §B7). The row is then drawn somewhere other than
/// where it is laid out, which the unscrolled cases above never exercise.
#[test]
fn the_last_row_of_a_scrolled_list_keeps_its_tooltip_off_itself() {
    for size in [lay::WINDOW, Size::new(1280.0, 720.0), MIN_WINDOW_SIZE] {
        let state = with_worktrees(Vec::new(), OVERFLOWING_LIST, size);
        let hovered = hover_row(&state, OVERFLOWING_LIST, size, Scroll::ToEnd);
        let row = &hovered.row;
        let panel = hovered.panel.unwrap_or_else(|| {
            panic!(
                "hovering the last row of a list scrolled to its end, at {}, opened no tooltip",
                describe(row)
            )
        });
        assert!(
            !intersects(&panel, row),
            "in a {:.0}×{:.0} window, with the list scrolled to its end, the last worktree row's \
             tooltip covers the row it describes: row {}, tooltip {} (FR-013, SC-006, BUG-001)",
            size.width,
            size.height,
            describe(row),
            describe(&panel)
        );
    }
}
