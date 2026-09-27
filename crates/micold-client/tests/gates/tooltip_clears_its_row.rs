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

use crate::context_menu_anchor::{record_every_worktree, sidebar_row, worktree, PROJECT};
use crate::support::layout::{self as lay, LayoutRecord};
use iced::Size;
use micold_client::app::State;
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::sidebar;
use micold_client::features::window;
use micold_core::theme::ThemePreference;

/// The smallest window the application allows (`shell/startup.rs`'s `MIN_WINDOW_SIZE`, private to
/// that module). The worst case for this bug: the least room on every side of every row.
const SMALLEST_WINDOW: Size = Size::new(640.0, 480.0);

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The first worktree row — past the "Default" project-root row, which is row 0.
const FIRST_WORKTREE_ROW: usize = 1;

/// Far more worktrees than any window here can show unscrolled. The search below stops well before
/// it; this only bounds the search.
const MOST_WORKTREES: usize = 60;

/// A project open with `count` worktrees, laid out for a window of `size`.
fn with_worktrees(count: usize, size: Size) -> State {
    let mut workspace = crate::support::workspace_with(vec![(PROJECT, Vec::new())]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());

    let mut state = State {
        sidebar: sidebar::State {
            width: 260,
            ..Default::default()
        },
        window: window::State {
            window_size: (size.width as u16, size.height as u16),
            ..Default::default()
        },
        workspace,
        worktree: micold_client::features::worktree::State {
            worktrees: (0..count)
                .map(|i| worktree(&format!("feat-{i:02}"), &format!("feat/{i:02}")))
                .collect(),
            ..Default::default()
        },
        ..State::default()
    };
    state.settings.theme_pref = ThemePreference::Light;
    record_every_worktree(state)
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
fn hover_row(state: &State, index: usize, size: Size) -> Hovered {
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

    // Hand over the frames an entrance needs before it takes input — the same settle
    // `context_menu_anchor` gives a press, for the same reason.
    const SETTLE_FRAMES: u32 = 8;
    let origin = std::time::Instant::now();
    let mut ignored = Vec::new();
    for frame in 0..SETTLE_FRAMES {
        let mut shell = Shell::new(&mut ignored);
        element.as_widget_mut().update(
            &mut tree,
            &iced::Event::Window(iced::window::Event::RedrawRequested(
                origin + lay::FRAME * frame,
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &viewport,
        );
    }

    let path = sidebar_row(index);
    let row = lay::walk(Layout::new(&node), lay::Layer::Base)
        .into_iter()
        .find(|r| r.path == path)
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

/// The tooltip panel: the outermost overlay node the hover added.
///
/// Overlays nest — every container that has more than one child with an overlay groups them, and a
/// group is laid out at the size of the whole window — so "outermost" means the shallowest new node
/// that is **not** window-sized. A hover that added nothing returns `None`, which the tests below
/// report as its own failure: a row without a tooltip is a different defect from one covered by it.
fn panel_that_opened(
    before: &[LayoutRecord],
    after: &[LayoutRecord],
    size: Size,
) -> Option<LayoutRecord> {
    after
        .iter()
        .filter(|r| {
            !((r.width - size.width).abs() < TOLERANCE
                && (r.height - size.height).abs() < TOLERANCE)
        })
        .filter(|r| !before.contains(r))
        .min_by_key(|r| r.path.len())
        .cloned()
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

/// The base record of sidebar row `index`, without hovering anything.
fn row_record(state: &State, index: usize, size: Size) -> Option<LayoutRecord> {
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
    let path = sidebar_row(index);
    lay::walk(Layout::new(&node), lay::Layer::Base)
        .into_iter()
        .find(|r| r.path == path)
}

/// The longest worktree list whose last row is still fully inside a window of `size`, unscrolled,
/// and that list's state.
///
/// Searched rather than stated: the row pitch is the sidebar's business, and a count that is right
/// today puts the last row below the fold — where it cannot be hovered at all — the day a row grows
/// a pixel. The search asks the layout instead.
fn fullest_unscrolled_list(size: Size) -> (State, usize) {
    let mut fullest = None;
    for count in 1..=MOST_WORKTREES {
        let state = with_worktrees(count, size);
        // Row 0 is the Default row, so the last worktree row is row `count`.
        let Some(last) = row_record(&state, count, size) else {
            break;
        };
        if last.y + last.height > size.height + TOLERANCE {
            break;
        }
        fullest = Some((state, count));
    }
    let (state, count) = fullest.unwrap_or_else(|| {
        panic!(
            "not even one worktree row fits a {}×{} window",
            size.width, size.height
        )
    });
    assert!(
        count < MOST_WORKTREES,
        "{MOST_WORKTREES} worktree rows all fit a {}×{} window, so the search never reached the \
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
    let hovered = hover_row(&state, last, size);
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
    assert_last_row_keeps_its_tooltip_off_itself(SMALLEST_WINDOW);
}

/// Unchanged behaviour: a row with room below it gets its tooltip below it, clear of it (T032).
#[test]
fn a_row_with_room_below_gets_its_tooltip_below_it() {
    let (state, _) = fullest_unscrolled_list(lay::WINDOW);
    let hovered = hover_row(&state, FIRST_WORKTREE_ROW, lay::WINDOW);
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
