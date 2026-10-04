//! An issue row's tooltip opens beside its row, inside the floating list, and gets in nobody's way
//! (feature 038; FR-016, FR-021, FR-023; contracts/picker-row.md §4–5).
//!
//! An issue with a description shows it in a tooltip once the cursor has rested on its row for
//! three seconds. The row is inside the picker's floating list, which is itself an overlay, so the
//! tooltip's panel is an overlay floated by an overlay. That is the part no other gate reaches:
//!
//! - `tooltip_clears_its_row` reads a tooltip against its row, but a sidebar row in the base tree,
//!   opened by a hover with no delay.
//! - `tooltip_rest_glue.rs` holds the rest rule, but on a tooltip over a probe, with no list, no
//!   scroll and no window around it.
//! - `issue_rows_show_all_text` and the geometry fixture read the list at rest, when no tooltip is
//!   open.
//!
//! So a list that never passes its rows' panels on, a panel placed from where the row is laid out
//! rather than where the scrolled list draws it, a panel that takes the click meant for its row,
//! and a panel as tall as its whole description all pass everything else.
//!
//! # It drives the form the way the runtime does
//!
//! Openness is widget state, so there is no `State` to build with a panel open. The gate keeps one
//! widget tree across views, as the runtime does, and hands it events in the runtime's order: the
//! floated layers first, the topmost of them first (`iced`'s own `overlay::Nested`, which is what
//! `UserInterface` uses), then the window under them with the cursor withheld where a floated layer
//! has it, and not at all once a floated layer has captured the event.
//!
//! # The clock
//!
//! A redraw carries its instant and a mouse event carries none, so the tooltip reads the wall clock
//! for a cursor move. The gate moves the cursor with a real `CursorMoved`, notes the wall clock on
//! both sides of it, and then hands over redraws at instants it chooses: one at the *later* reading
//! plus the delay is certainly a full delay at rest, and one a millisecond short of the *earlier*
//! reading plus the delay is certainly not.
//!
//! # Finding the panel
//!
//! `Nested` lays the floated layers out as one tree: child 0 of the root is the list, child 1 is
//! whatever the list floats. Everything under child 1 that is not the size of the window is a
//! panel or a part of one (overlays group, and a group is the size of the window), and the
//! shallowest of those are the panels themselves. A panel's first child is the surface a person
//! sees; the panel around it is margin drawn as nothing.
//!
//! # Compiled into the `layout_snapshot` binary
//!
//! Beside `tooltip_clears_its_row`, whose question it asks of another surface. It builds its own
//! form rather than adding a covered state, so the fixture does not change.

use std::time::{Duration, Instant};

use iced::advanced::overlay::Nested;
use iced::advanced::widget::operation::{Focusable, Scrollable};
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{event, keyboard, window, Event, Point, Rectangle, Size, Vector};

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::{
    BranchSource, GithubAvailability, IssueList, Msg as FormMsg, SearchState, WorktreeForm,
};
use micold_core::github::{GithubRepo, Issue, IssueListing};
use micold_core::tokens::{spacing, typography};
use micold_core::tooltip::REST_TOLERANCE;
use micold_core::typeahead::{rank, Query};

use crate::support::layout::{self as lay, LayoutRecord, StateUnderTest};

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The window the form is laid out in: the canonical one, where the list has the room to open
/// below its field at its full eight rows.
const WINDOW: Size = lay::WINDOW;

/// How long the cursor rests on a row before its tooltip opens.
///
/// A copy of `material::picker::ROW_TOOLTIP_REST`, which an integration test cannot name
/// (`ui::material` is `pub(crate)`). FR-015 states the three seconds, so a changed constant has to
/// change this too.
const ROW_TOOLTIP_REST: Duration = Duration::from_secs(3);

/// The most lines a row's tooltip shows: a copy of `material::picker::ROW_TOOLTIP_LINES`, for the
/// same reason (FR-021).
const ROW_TOOLTIP_LINES: f32 = 3.0;

/// One millisecond: how far short of the delay "not yet" is measured.
const MS: Duration = Duration::from_millis(1);

/// More issues than the list shows at once (eight base rows), so the list scrolls and its last row
/// is not the row at its lower edge.
const ISSUES: usize = 12;

/// The first issue's number; the rest count up from it, so a number names its row.
const FIRST_NUMBER: u64 = 101;

/// A row away from both ends of the list and wholly in view: the row a test hovers when which row
/// it is does not matter.
const MIDDLE_ROW: usize = 3;

/// How far inside a row's edge the cursor is put when the test needs it near that edge. Twice this
/// is the step from one row onto the next, and has to stay under `REST_TOLERANCE`.
const INSIDE_EDGE: f32 = 1.0;

/// The list's scrollable among the floated layers: what is visible of the rows.
///
/// `Nested`'s root, the list (its child 0), then the path `issue_rows_show_all_text` names: the
/// floating layer's positioning wrappers, the menu panel, the container that caps the list at
/// eight rows, and the scrollable. Its one child is the column of rows, and that column's children
/// are the rows. `Form::list` checks the count against [`ISSUES`], so a tree that renumbers fails
/// there rather than reading some other node as a row.
const LIST_VIEWPORT: &[usize] = &[0, 0, 0, 0, 0, 0, 0];

/// Where `Nested` puts what the list itself floats: child 1 of its root.
const FLOATED_BY_THE_LIST: usize = 1;

/// A description far longer than three lines of a tooltip, so the line limit is what bounds the
/// panel's height. `the_panel` checks that it is.
fn long_description(number: u64) -> String {
    format!(
        "Issue {number}. {}",
        "Resizing the window makes the sidebar repaint every row, and the rows flicker. ".repeat(6)
    )
}

/// Twelve issues with short titles, each described at length.
fn described_issues() -> Vec<Issue> {
    (0..ISSUES as u64)
        .map(|row| {
            let number = FIRST_NUMBER + row;
            Issue::new(
                number,
                format!("Sidebar flickers ({number})"),
                vec!["bug".to_string()],
                "2026-09-29T00:00:00Z".to_string(),
            )
            .reported_by("octocat")
            .described(&long_description(number))
        })
        .collect()
}

/// The add-worktree form on the issue source, `issues` loaded and the list open, nothing
/// highlighted: the state the user is in after choosing **GitHub issue** and focusing the field.
fn form_listing(issues: Vec<Issue>) -> StateUnderTest {
    let mut workspace = crate::support::workspace_with(vec![("/fixture/project", vec![])]);
    workspace.active = workspace.projects.first().map(|p| p.path.clone());
    let mut state = State {
        workspace,
        ..State::default()
    };

    let issue_matches = rank(&issues, |i| i.row_text(), &Query::new(""));
    let total_open = issues.len() as u64;
    state.worktree_form.form = Some(WorktreeForm {
        source: BranchSource::Issue,
        github: GithubAvailability::Available(
            GithubRepo::from_remote_url("git@github.com:octo/widgets.git").expect("a GitHub URL"),
        ),
        issues: IssueList::Loaded {
            listing: IssueListing {
                issues,
                total_open,
                complete: true,
            },
            gh: std::path::PathBuf::from("/usr/bin/gh"),
            searched: Vec::new(),
            search: SearchState::Idle,
        },
        issue_matches,
        issue_list_open: true,
        type_: None,
        ..WorktreeForm::default()
    });
    StateUnderTest::new(state)
}

fn key(named: keyboard::key::Named) -> Event {
    Event::Keyboard(keyboard::Event::KeyPressed {
        key: keyboard::Key::Named(named),
        modified_key: keyboard::Key::Named(named),
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    })
}

/// The list as it is on screen.
struct List {
    /// What is visible of the rows.
    viewport: LayoutRecord,
    /// Every row where it is drawn: its laid-out place less how far the list is scrolled. A row
    /// outside the viewport is here too, above or below it.
    rows: Vec<LayoutRecord>,
}

impl List {
    /// The middle of row `index`.
    fn centre_of(&self, index: usize) -> Point {
        let row = &self.rows[index];
        Point::new(row.x + row.width / 2.0, row.y + row.height / 2.0)
    }

    /// Whether a cursor at `at` is on the visible part of the list.
    fn shows(&self, at: Point) -> bool {
        at.x > self.viewport.x
            && at.x < self.viewport.x + self.viewport.width
            && at.y > self.viewport.y
            && at.y < self.viewport.y + self.viewport.height
    }

    /// The lowest row whose middle is on screen: the row at the list's lower edge.
    ///
    /// The middle rather than the whole row, because the middle is where the cursor goes, and a
    /// row whose middle is cut off takes no hover at all.
    fn lowest_in_view(&self) -> usize {
        (0..self.rows.len())
            .rev()
            .find(|index| self.shows(self.centre_of(*index)))
            .expect("the list shows at least one row")
    }
}

/// A tooltip panel: the surface a person sees.
type Panel = LayoutRecord;

/// The scrollable of the list: how far it is scrolled, and optionally scrolled to its end first.
#[derive(Default)]
struct Scroll {
    to_end: bool,
    seen: usize,
    scrolled: f32,
}

impl Operation for Scroll {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn scrollable(
        &mut self,
        _id: Option<&Id>,
        _bounds: Rectangle,
        _content_bounds: Rectangle,
        translation: Vector,
        state: &mut dyn Scrollable,
    ) {
        if self.to_end {
            // The app's own `snap_to`, not a wheel event whose travel would be this file's guess.
            state.snap_to(iced::widget::scrollable::RelativeOffset {
                x: None,
                y: Some(1.0),
            });
        }
        self.seen += 1;
        self.scrolled = translation.y;
    }
}

/// Every widget that can hold keyboard focus, and whether it does.
#[derive(Default)]
struct Focus {
    all: Vec<Rectangle>,
    focused: Vec<Rectangle>,
}

impl Operation for Focus {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn focusable(&mut self, _id: Option<&Id>, bounds: Rectangle, state: &mut dyn Focusable) {
        self.all.push(bounds);
        if state.is_focused() {
            self.focused.push(bounds);
        }
    }
}

/// The form with its widget tree, kept across views the way the runtime keeps it: a tooltip's wait
/// and the list's scroll offset live in the tree.
struct Form {
    under: StateUnderTest,
    tree: Tree,
    renderer: iced::Renderer,
}

impl Form {
    /// The form with its list open and its entrance over, the cursor nowhere.
    fn new() -> Self {
        let under = form_listing(described_issues());
        let tree = Tree::new(lay::view_of(&under).as_widget());
        let mut form = Self {
            under,
            tree,
            renderer: lay::renderer(),
        };
        // The dialog and the list both mount at the start of an entrance and take no input until
        // it has run. The first frames reach the base tree only, because the list is not floated
        // until its visibility has moved; the rest reach the list as well.
        let origin = Instant::now();
        for frame in 0..lay::SETTLE_FRAMES * 2 {
            form.frame(origin + lay::FRAME * frame, mouse::Cursor::Unavailable);
        }
        form
    }

    fn highlight(&self) -> Option<usize> {
        self.under
            .state
            .worktree_form
            .form
            .as_ref()
            .expect("the form is open")
            .issue_highlight
    }

    /// Hand `event` to the view as `UserInterface::update` does, apply what it published to the
    /// reducer, and return it.
    fn send(&mut self, event: &Event, cursor: mouse::Cursor) -> Vec<Message> {
        let mut messages = Vec::new();
        {
            let mut element = lay::view_of(&self.under);
            self.tree.diff(element.as_widget());
            let limits = layout::Limits::new(Size::ZERO, WINDOW);
            let viewport = Rectangle::with_size(WINDOW);
            let node = element
                .as_widget_mut()
                .layout(&mut self.tree, &self.renderer, &limits);
            let mut clipboard = clipboard::Null;

            // The floated layers first. They decide two things for the window under them: whether
            // it sees the event at all, and whether it sees the cursor.
            let mut base_cursor = cursor;
            let mut captured = false;
            if let Some(overlay) = element.as_widget_mut().overlay(
                &mut self.tree,
                Layout::new(&node),
                &self.renderer,
                &viewport,
                Vector::ZERO,
            ) {
                let mut floated = Nested::new(overlay);
                let floated_node = floated.layout(&self.renderer, WINDOW);
                let mut shell = Shell::new(&mut messages);
                floated.update(
                    event,
                    Layout::new(&floated_node),
                    cursor,
                    &self.renderer,
                    &mut clipboard,
                    &mut shell,
                );
                captured = shell.event_status() == event::Status::Captured;
                let taken = cursor.position().is_some_and(|at| {
                    floated.mouse_interaction(
                        Layout::new(&floated_node),
                        mouse::Cursor::Available(at),
                        &self.renderer,
                    ) != mouse::Interaction::None
                });
                if taken {
                    base_cursor = mouse::Cursor::Unavailable;
                }
            }

            if !captured {
                let mut shell = Shell::new(&mut messages);
                element.as_widget_mut().update(
                    &mut self.tree,
                    event,
                    Layout::new(&node),
                    base_cursor,
                    &self.renderer,
                    &mut clipboard,
                    &mut shell,
                    &viewport,
                );
            }
        }
        for message in &messages {
            self.under.state.update(message.clone());
        }
        messages
    }

    /// A redraw at `now` with the cursor at `cursor`.
    fn frame(&mut self, now: Instant, cursor: mouse::Cursor) {
        self.send(&Event::Window(window::Event::RedrawRequested(now)), cursor);
    }

    /// Move the cursor to `at`, and return the wall clock just before and just after: the tooltip
    /// under it started its wait somewhere between the two.
    fn move_to(&mut self, at: Point) -> (Instant, Instant) {
        let before = Instant::now();
        self.send(
            &Event::Mouse(mouse::Event::CursorMoved { position: at }),
            mouse::Cursor::Available(at),
        );
        (before, Instant::now())
    }

    /// Move the cursor to `at` and keep it there for the whole delay.
    fn rest_at(&mut self, at: Point) {
        let (_, moved) = self.move_to(at);
        self.frame(moved + ROW_TOOLTIP_REST, mouse::Cursor::Available(at));
    }

    /// A left click with the cursor at `at`: the press and the release, and what both published.
    fn click(&mut self, at: Point) -> Vec<Message> {
        let cursor = mouse::Cursor::Available(at);
        let mut published = self.send(
            &Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        );
        published.extend(self.send(
            &Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor,
        ));
        published
    }

    /// One pass of `operation` over every floated layer, after the window's own tree when
    /// `window_too`, as `UserInterface::operate` runs one.
    fn operate(&mut self, operation: &mut dyn Operation, window_too: bool) {
        let mut element = lay::view_of(&self.under);
        self.tree.diff(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        let node = element
            .as_widget_mut()
            .layout(&mut self.tree, &self.renderer, &limits);
        if window_too {
            element.as_widget_mut().operate(
                &mut self.tree,
                Layout::new(&node),
                &self.renderer,
                operation,
            );
        }
        if let Some(overlay) = element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&node),
            &self.renderer,
            &Rectangle::with_size(WINDOW),
            Vector::ZERO,
        ) {
            let mut floated = Nested::new(overlay);
            let floated_node = floated.layout(&self.renderer, WINDOW);
            floated.operate(Layout::new(&floated_node), &self.renderer, operation);
        };
    }

    /// Every node of every floated layer, the list and whatever it floats in turn.
    fn floated(&mut self) -> Vec<LayoutRecord> {
        let mut element = lay::view_of(&self.under);
        self.tree.diff(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        let node = element
            .as_widget_mut()
            .layout(&mut self.tree, &self.renderer, &limits);
        element
            .as_widget_mut()
            .overlay(
                &mut self.tree,
                Layout::new(&node),
                &self.renderer,
                &Rectangle::with_size(WINDOW),
                Vector::ZERO,
            )
            .map(|overlay| {
                let mut floated = Nested::new(overlay);
                let floated_node = floated.layout(&self.renderer, WINDOW);
                lay::walk(Layout::new(&floated_node), lay::Layer::Overlay)
            })
            .unwrap_or_default()
    }

    /// The list as it is on screen now.
    fn list(&mut self) -> List {
        let records = self.floated();
        let viewport = records
            .iter()
            .find(|r| r.path == LIST_VIEWPORT)
            .cloned()
            .unwrap_or_else(|| {
                panic!(
                    "no list is floated at {}: the list is closed, or the floated tree changed \
                     shape around LIST_VIEWPORT",
                    lay::path_token(LIST_VIEWPORT)
                )
            });
        let mut content = LIST_VIEWPORT.to_vec();
        content.push(0);
        let mut rows: Vec<LayoutRecord> = records
            .iter()
            .filter(|r| r.path.len() == content.len() + 1 && r.path.starts_with(&content))
            .cloned()
            .collect();
        assert_eq!(
            rows.len(),
            ISSUES,
            "the list lays out a row for each of its {ISSUES} issues under {}",
            lay::path_token(&content)
        );

        // Layout records do not move with the scroll: a scrollable offsets what it draws, not
        // where its content is laid out. So each row is put where it appears on screen.
        let mut scroll = Scroll::default();
        self.operate(&mut scroll, false);
        assert_eq!(scroll.seen, 1, "the floated list holds one scrollable");
        for row in &mut rows {
            row.y -= scroll.scrolled;
        }
        List { viewport, rows }
    }

    /// Scroll the list to its end, and return how far that moved it.
    fn scroll_to_end(&mut self) -> f32 {
        let mut snap = Scroll {
            to_end: true,
            ..Scroll::default()
        };
        self.operate(&mut snap, false);
        let mut read = Scroll::default();
        self.operate(&mut read, false);
        read.scrolled
    }

    /// The tooltip panels open now, each as the surface a person sees.
    fn panels(&mut self) -> Vec<Panel> {
        let records = self.floated();
        let parts: Vec<&LayoutRecord> = records
            .iter()
            .filter(|r| r.path.first() == Some(&FLOATED_BY_THE_LIST))
            .filter(|r| {
                !((r.width - WINDOW.width).abs() < TOLERANCE
                    && (r.height - WINDOW.height).abs() < TOLERANCE)
            })
            .collect();
        let Some(depth) = parts.iter().map(|r| r.path.len()).min() else {
            return Vec::new();
        };
        parts
            .iter()
            .filter(|r| r.path.len() == depth)
            .map(|panel| {
                let mut surface = panel.path.clone();
                surface.push(0);
                records
                    .iter()
                    .find(|r| r.path == surface)
                    .unwrap_or(panel)
                    .clone()
            })
            .collect()
    }

    /// The one panel open now. `after` says what was done, for the failure.
    fn the_panel(&mut self, after: &str) -> Panel {
        let panels = self.panels();
        assert_eq!(
            panels.len(),
            1,
            "{after}: exactly one tooltip panel has to be open above the list, and {} are. Every \
             issue here has a description, so its row has a tooltip; the list is an overlay, so \
             the panel is an overlay the list itself floats (038 FR-015, FR-023; \
             contracts/picker-row.md §5)",
            panels.len()
        );
        panels.into_iter().next().expect("one panel")
    }

    /// Every widget that holds keyboard focus, in the window and in what floats over it.
    fn focused(&mut self) -> Vec<Rectangle> {
        let mut focus = Focus::default();
        self.operate(&mut focus, true);
        focus.focused
    }

    /// The search field: the focusable directly above the list, in the list's own column.
    fn search_field(&mut self) -> Rectangle {
        let list = self.list();
        let mut focus = Focus::default();
        self.operate(&mut focus, true);
        focus
            .all
            .into_iter()
            .filter(|f| {
                f.x >= list.viewport.x - spacing::LG
                    && f.x + f.width <= list.viewport.x + list.viewport.width + spacing::LG
                    && f.y + f.height <= list.viewport.y
            })
            .max_by(|a, b| a.y.total_cmp(&b.y))
            .expect("a field the list hangs under")
    }
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

/// The shared body of U75: with the cursor rested on row `index` of `form`, one panel is open, it
/// is inside the window, it is clear of the row, and it is no taller than three lines.
fn assert_the_rested_row_shows_its_panel_clear_of_itself(form: &mut Form, index: usize) {
    let list = form.list();
    let row = list.rows[index].clone();
    let at = list.centre_of(index);
    assert!(
        list.shows(at),
        "row {index} at {} has its middle outside the list's viewport {}, so a hover there lands \
         on nothing",
        describe(&row),
        describe(&list.viewport)
    );

    form.rest_at(at);
    let panel = form.the_panel(&format!(
        "with the cursor held still for {ROW_TOOLTIP_REST:?} on row {index} at {}",
        describe(&row)
    ));

    assert!(
        !intersects(&panel, &row),
        "row {index}'s tooltip covers the row it describes: row {}, tooltip {}. It opens below \
         its row, or above when there is no room below, never over it (038 FR-023)",
        describe(&row),
        describe(&panel)
    );

    assert!(
        panel.x >= -TOLERANCE
            && panel.y >= -TOLERANCE
            && panel.x + panel.width <= WINDOW.width + TOLERANCE
            && panel.y + panel.height <= WINDOW.height + TOLERANCE,
        "row {index}'s tooltip {} left the {:.0}×{:.0} window (038 FR-023)",
        describe(&panel),
        WINDOW.width,
        WINDOW.height
    );

    // The bound is only a bound if the text wants more: on one line the description is more than
    // three times as wide as the panel that opened, so it cannot be shown whole in three.
    let caption = typography::BODY_SMALL;
    let number = FIRST_NUMBER + index as u64;
    let unwrapped = lay::measure(
        &long_description(number),
        lay::reference_font(),
        caption.size,
    );
    assert!(
        unwrapped > ROW_TOOLTIP_LINES * panel.width,
        "the description is {unwrapped:.0}px on one line and the panel {:.0}px wide, so it fits \
         in three lines and this fixture no longer exercises the line limit",
        panel.width
    );

    // Three lines of `Caption` (Material's `body_small`) and the panel's padding, `spacing::XS`
    // above and below.
    let tallest = ROW_TOOLTIP_LINES * caption.line_height + 2.0 * spacing::XS;
    assert!(
        panel.height <= tallest + TOLERANCE,
        "row {index}'s tooltip is {:.1}px tall; three `Caption` lines and the panel's padding \
         come to {tallest:.1}px. A description that does not fit is cut and ends in an ellipsis \
         (038 FR-021)",
        panel.height
    );
}

/// U75, the first row: the whole list is below it, and the panel opens over the rows beneath.
#[test]
fn the_first_row_shows_one_panel_clear_of_itself() {
    let mut form = Form::new();
    assert_the_rested_row_shows_its_panel_clear_of_itself(&mut form, 0);
}

/// U75, a row at the list's lower edge: there is no list left below it, so the panel is either
/// outside the list or on the other side of the row.
#[test]
fn a_row_at_the_lists_lower_edge_shows_one_panel_clear_of_itself() {
    let mut form = Form::new();
    let list = form.list();
    let lowest = list.lowest_in_view();
    assert!(
        lowest > 0 && lowest < ISSUES - 1,
        "row {lowest} is the lowest row in view of {ISSUES}; the list has to scroll, so that the \
         row at its lower edge is neither its first nor its last"
    );
    let row = &list.rows[lowest];
    let edge = list.viewport.y + list.viewport.height;
    assert!(
        edge - (row.y + row.height) < row.height,
        "row {lowest} ends at y={:.0} and the list at y={edge:.0}, more than a row's height \
         ({:.0}px) apart, so it is not the row at the lower edge",
        row.y + row.height,
        row.height
    );

    assert_the_rested_row_shows_its_panel_clear_of_itself(&mut form, lowest);
}

/// U75, the last row: reached by scrolling, so it is drawn somewhere other than where it is laid
/// out, and the panel has to be placed from where it is drawn.
#[test]
fn the_last_row_of_the_scrolled_list_shows_one_panel_clear_of_itself() {
    let mut form = Form::new();
    let unscrolled = form.list();
    let moved = form.scroll_to_end();
    assert!(
        moved > unscrolled.rows[0].height,
        "scrolling the list to its end moved it {moved:.0}px, less than a row, so the list does \
         not overflow and this is the unscrolled case again"
    );

    assert_the_rested_row_shows_its_panel_clear_of_itself(&mut form, ISSUES - 1);
}

/// U76 (FR-023, US3 scenario 10): the panel takes no input, so a click on the row whose panel is
/// open is that row's click and picks its issue.
#[test]
fn a_click_on_the_row_under_an_open_panel_picks_its_issue() {
    let mut form = Form::new();
    let at = form.list().centre_of(MIDDLE_ROW);
    form.rest_at(at);
    form.the_panel(&format!("before the click on row {MIDDLE_ROW}"));

    let published = form.click(at);

    let picked: Vec<usize> = published
        .iter()
        .filter_map(|m| match m {
            Message::WorktreeForm(FormMsg::IssueRowPicked(row)) => Some(*row),
            _ => None,
        })
        .collect();
    assert_eq!(
        picked,
        vec![MIDDLE_ROW],
        "a click on row {MIDDLE_ROW} with its tooltip open picks that row once; it published \
         {published:?}"
    );
    assert_eq!(
        form.under.state.worktree_form.issue_number_at(MIDDLE_ROW),
        Some(FIRST_NUMBER + MIDDLE_ROW as u64),
        "the picked row is the issue the tooltip described"
    );
}

/// Two points less than `REST_TOLERANCE` apart, the first on row `index` and the second on the row
/// below it.
fn either_side_of_the_edge_below(list: &List, index: usize) -> (Point, Point) {
    let (upper, lower) = (&list.rows[index], &list.rows[index + 1]);
    let x = upper.x + upper.width / 2.0;
    let on_upper = Point::new(x, upper.y + upper.height - INSIDE_EDGE);
    let on_lower = Point::new(x, lower.y + INSIDE_EDGE);
    let apart = on_lower.y - on_upper.y;
    assert!(
        apart > 0.0 && apart < REST_TOLERANCE,
        "the two points are {apart:.1}px apart; they have to be on different rows and closer than \
         the rest tolerance of {REST_TOLERANCE}px, or this is an ordinary move"
    );
    (on_upper, on_lower)
}

/// U84 (FR-016), first half: another row is another tooltip, however small the move that reached
/// it. A move inside the rest tolerance keeps a panel open within one row, and must not across two.
#[test]
fn moving_onto_the_adjacent_row_by_less_than_the_tolerance_closes_the_panel() {
    let mut form = Form::new();
    let list = form.list();
    let (on_first, on_second) = either_side_of_the_edge_below(&list, 0);
    form.rest_at(on_first);
    form.the_panel("with the cursor held still near the lower edge of row 0");

    form.move_to(on_second);

    let open = form.panels();
    assert!(
        open.is_empty(),
        "the cursor moved from row 0 onto row 1 and {} panel(s) are still open: {}. Movement to \
         another row closes the tooltip, however small (038 FR-016, FR-017)",
        open.len(),
        open.iter().map(describe).collect::<Vec<_>>().join("; ")
    );
}

/// U84 (FR-016), second half: the row the cursor arrived on waits its own full delay. The time the
/// cursor spent on the row above counts for nothing.
#[test]
fn the_adjacent_row_opens_its_panel_only_after_its_own_full_delay() {
    let mut form = Form::new();
    let list = form.list();
    let (on_first, on_second) = either_side_of_the_edge_below(&list, 0);
    form.rest_at(on_first);
    form.the_panel("with the cursor held still near the lower edge of row 0");

    let (before, after) = form.move_to(on_second);
    let cursor = mouse::Cursor::Available(on_second);

    form.frame(before + ROW_TOOLTIP_REST - MS, cursor);
    let early = form.panels();
    assert!(
        early.is_empty(),
        "{} panel(s) are open a millisecond short of {ROW_TOOLTIP_REST:?} after the cursor \
         arrived on row 1: {}. Row 1 waits its own full delay (038 FR-016)",
        early.len(),
        early.iter().map(describe).collect::<Vec<_>>().join("; ")
    );

    form.frame(after + ROW_TOOLTIP_REST, cursor);
    let panel = form.the_panel(&format!(
        "with the cursor held still for {ROW_TOOLTIP_REST:?} on row 1"
    ));
    let second = &list.rows[1];
    assert!(
        !intersects(&panel, second),
        "the panel open after the delay on row 1 lies over row 1, so it is not row 1's own: row \
         {}, tooltip {}",
        describe(second),
        describe(&panel)
    );
}

/// U85 (FR-023), focus: the panel is not a thing that can be focused, and opening it moves focus
/// nowhere. The developer who was typing in the search field still is.
#[test]
fn the_search_field_keeps_keyboard_focus_while_a_panel_is_open() {
    let mut form = Form::new();
    // Focus the field the way a person does, with a click in it.
    let field = form.search_field();
    form.click(field.center());
    assert_eq!(
        form.focused(),
        vec![field],
        "a click in the search field at {field:?} gives it, and nothing else, keyboard focus"
    );

    let at = form.list().centre_of(MIDDLE_ROW);
    form.rest_at(at);
    form.the_panel(&format!(
        "with the search field focused and the cursor held still on row {MIDDLE_ROW}"
    ));

    assert_eq!(
        form.focused(),
        vec![field],
        "with a tooltip open the search field at {field:?} still holds keyboard focus, and \
         nothing else does (038 FR-023)"
    );
}

/// U85 (FR-023), keys: with a panel open, Down, Up and Enter do what they do without one. The
/// same presses go to a second form nobody is pointing at, and the two have to agree press by
/// press.
#[test]
fn up_down_and_enter_act_under_an_open_panel_as_without_one() {
    use keyboard::key::Named;

    let mut pointed_at = Form::new();
    let at = pointed_at.list().centre_of(MIDDLE_ROW);
    pointed_at.rest_at(at);
    pointed_at.the_panel(&format!(
        "with the cursor held still on row {MIDDLE_ROW}, before any key"
    ));
    let mut left_alone = Form::new();

    // Down from the field lands on the first issue, Down again on the second, Up back on the
    // first, and Enter picks it. None of them is the row the cursor rests on.
    let presses = [
        (Named::ArrowDown, Some(0)),
        (Named::ArrowDown, Some(1)),
        (Named::ArrowUp, Some(0)),
        (Named::Enter, Some(0)),
    ];
    let mut last = Vec::new();
    for (step, (named, highlight)) in presses.into_iter().enumerate() {
        let with_panel = pointed_at.send(&key(named), mouse::Cursor::Available(at));
        let without = left_alone.send(&key(named), mouse::Cursor::Unavailable);

        assert_eq!(
            left_alone.highlight(),
            highlight,
            "press {step} ({named:?}) with no tooltip anywhere: the highlight this gate expects"
        );
        assert_eq!(
            (with_panel.as_slice(), pointed_at.highlight()),
            (without.as_slice(), left_alone.highlight()),
            "press {step} ({named:?}) with a tooltip open on row {MIDDLE_ROW} has to publish what \
             it publishes without one, and leave the highlight where that leaves it (038 FR-023)"
        );
        last = with_panel;
    }

    let picked: Vec<usize> = last
        .iter()
        .filter_map(|m| match m {
            Message::WorktreeForm(FormMsg::IssueRowPicked(row)) => Some(*row),
            _ => None,
        })
        .collect();
    assert_eq!(
        picked,
        vec![0],
        "Enter with a tooltip open picks the highlighted row, not the row the cursor rests on; it \
         published {last:?}"
    );
}
