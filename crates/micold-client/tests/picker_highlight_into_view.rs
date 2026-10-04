//! Whether the issue list follows its highlight (feature 038, FR-007, SC-007;
//! contracts/picker-row.md §3).
//!
//! An issue's row is as tall as its two wrapping lines come to, so a list of them no longer moves
//! by a fixed step: Down can land on a row that lies below the list's eight-row viewport, or on one
//! taller than the row before it. The reducer only moves an index. What brings the row into view is
//! an operation the shell chains after it, and that operation can fail silently in the ways
//! `focus_scroll.rs` names: it runs, finds nothing or scrolls the wrong panel, and the list sits
//! where it was with the highlight somewhere nobody can see.
//!
//! So this drives the real add-worktree form: it presses Down, Up and Enter on the open list,
//! applies what the list publishes to the real reducer, runs the operation to the end of its chain
//! over the base tree and the floated list as the runtime does, and reads back where the
//! highlighted row and the list's viewport are.

mod support;

use iced::advanced::widget::operation::scrollable::Scrollable;
use iced::advanced::widget::operation::Outcome;
use iced::advanced::widget::{Id, Operation, Tree};
use iced::advanced::{layout, mouse, Layout, Shell};
use iced::{keyboard, Event, Rectangle, Size, Vector};

use micold_client::app::{Message, State};
use micold_client::features::worktree_form::{
    BranchSource, GithubAvailability, IssueList, Msg as FormMsg, SearchState, WorktreeForm,
};
use micold_core::github::{GithubRepo, Issue, IssueListing};
use micold_core::typeahead::{rank, Query};

use support::layout::{renderer, view_of, StateUnderTest, WINDOW};

/// As many issues as SC-007's walk names: more than the list shows at once.
const ISSUES: usize = 12;

/// The first issue's number; the rest count up from it, so a number names its row.
const FIRST_NUMBER: u64 = 101;

/// A title that fits on one line of the list.
const SHORT_TITLE: &str = "Sidebar flickers";

/// One issue, reported by `octocat`.
fn issue(number: u64, title: String) -> Issue {
    Issue::new(
        number,
        title,
        vec!["bug".to_string()],
        "2026-09-29T00:00:00Z".to_string(),
    )
    .reported_by("octocat")
}

/// A title long enough to wrap onto about `lines` lines of the list.
fn title_of(lines: usize) -> String {
    // The list is the form's width; forty-odd characters of this phrase fill one line of it. The
    // tests assert the heights they rely on rather than trusting this estimate.
    const ONE_LINE: &str = "The sidebar flickers when the window is resized ";
    if lines <= 1 {
        SHORT_TITLE.to_string()
    } else {
        ONE_LINE.repeat(lines - 1).trim_end().to_string()
    }
}

/// Twelve issues whose rows are one to five title lines tall, mixed rather than sorted by height.
fn mixed_issues() -> Vec<Issue> {
    const TITLE_LINES: [usize; ISSUES] = [1, 3, 5, 2, 4, 1, 5, 2, 1, 4, 3, 5];
    TITLE_LINES
        .iter()
        .enumerate()
        .map(|(row, lines)| issue(FIRST_NUMBER + row as u64, title_of(*lines)))
        .collect()
}

/// The add-worktree form on the issue source, `issues` loaded and the list open, nothing
/// highlighted: the state the user is in after choosing **GitHub issue** and focusing the field.
fn form_listing(issues: Vec<Issue>) -> StateUnderTest {
    let mut workspace = support::workspace_with(vec![("/fixture/project", vec![])]);
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
            descriptions: Default::default(),
        },
        issue_matches,
        issue_list_open: true,
        type_: None,
        ..WorktreeForm::default()
    });
    StateUnderTest::new(state)
}

fn pressed(named: keyboard::key::Named) -> Event {
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

/// The form with its widget tree, kept across views the way the runtime keeps it: the list's
/// scroll offset lives in the tree, and a fresh tree per view would put it back at the top.
struct Form {
    under: StateUnderTest,
    tree: Tree,
    renderer: iced::Renderer,
}

impl Form {
    fn new(issues: Vec<Issue>) -> Self {
        let under = form_listing(issues);
        let tree = Tree::new(view_of(&under).as_widget());
        Self {
            under,
            tree,
            renderer: renderer(),
        }
    }

    fn form(&self) -> &WorktreeForm {
        self.under
            .state
            .worktree_form
            .form
            .as_ref()
            .expect("the form is open")
    }

    fn highlight(&self) -> Option<usize> {
        self.form().issue_highlight
    }

    /// Hand `event` to the view the way the runtime does, the floated list first and then the
    /// window under it with the same shell, and return what was published.
    fn publish(&mut self, event: &Event) -> Vec<Message> {
        let mut messages = Vec::new();
        let mut element = view_of(&self.under);
        self.tree.diff(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        let viewport = Rectangle::with_size(WINDOW);
        let node = element
            .as_widget_mut()
            .layout(&mut self.tree, &self.renderer, &limits);
        let mut shell = Shell::new(&mut messages);
        let mut clipboard = iced::advanced::clipboard::Null;

        if let Some(mut overlay) = element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&node),
            &self.renderer,
            &viewport,
            Vector::ZERO,
        ) {
            let overlay_node = overlay.as_overlay_mut().layout(&self.renderer, WINDOW);
            overlay.as_overlay_mut().update(
                event,
                Layout::new(&overlay_node),
                mouse::Cursor::Unavailable,
                &self.renderer,
                &mut clipboard,
                &mut shell,
            );
        }
        element.as_widget_mut().update(
            &mut self.tree,
            event,
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &self.renderer,
            &mut clipboard,
            &mut shell,
            &viewport,
        );
        messages
    }

    /// Press a key and apply what it published to the reducer. Returns the messages, for the
    /// caller that asserts on them.
    fn press(&mut self, named: keyboard::key::Named) -> Vec<Message> {
        let messages = self.publish(&pressed(named));
        for message in &messages {
            self.under.state.update(message.clone());
        }
        messages
    }

    /// One pass of `operation` over the window and then the floated list, as
    /// `UserInterface::operate` runs one.
    fn pass(&mut self, operation: &mut dyn Operation<()>) {
        let mut element = view_of(&self.under);
        self.tree.diff(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        let viewport = Rectangle::with_size(WINDOW);
        let node = element
            .as_widget_mut()
            .layout(&mut self.tree, &self.renderer, &limits);
        element.as_widget_mut().operate(
            &mut self.tree,
            Layout::new(&node),
            &self.renderer,
            operation,
        );
        if let Some(mut overlay) = element.as_widget_mut().overlay(
            &mut self.tree,
            Layout::new(&node),
            &self.renderer,
            &viewport,
            Vector::ZERO,
        ) {
            let overlay_node = overlay.as_overlay_mut().layout(&self.renderer, WINDOW);
            overlay
                .as_overlay_mut()
                .operate(Layout::new(&overlay_node), &self.renderer, operation);
        };
    }
}

/// Where the highlighted row and the list around it are, read back out of the tree.
#[derive(Default)]
struct Probe {
    /// The scrollable most recently entered: its viewport, where its content starts, and how far
    /// it is scrolled.
    last_list: Option<Showing>,
    found: Option<(Rectangle, Showing)>,
    /// How many widgets carry the highlight's `Id`: more than one and "the highlighted row" is
    /// not one row.
    highlighted_rows: usize,
}

/// What a list is showing.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Showing {
    viewport_height: f32,
    content_top: f32,
    scrolled: f32,
}

impl Showing {
    /// The band of content on screen, in the content's own coordinates.
    fn visible(&self) -> (f32, f32) {
        let top = self.content_top + self.scrolled;
        (top, top + self.viewport_height)
    }
}

impl<T> Operation<T> for Probe {
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
        self.last_list = Some(Showing {
            viewport_height: bounds.height,
            content_top: content_bounds.y,
            scrolled: translation.y,
        });
    }

    fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
        if id == Some(&*micold_client::ui::PICKER_HIGHLIGHT) {
            self.highlighted_rows += 1;
            if let Some(list) = self.last_list {
                self.found = Some((bounds, list));
            }
        }
    }
}

/// A shaped line's height is not a round number, so a row's edge can sit a fraction of a pixel
/// past the viewport's without any of it being cut. A row that is not in view misses by tens.
const SLACK: f32 = 0.5;

impl Form {
    /// The highlighted row's bounds and the list it is in.
    fn highlighted_row(&mut self) -> (Rectangle, Showing) {
        let mut probe = Probe::default();
        self.pass(&mut probe);
        assert_eq!(
            probe.highlighted_rows, 1,
            "exactly one row carries the highlight's Id"
        );
        probe.found.expect("the highlighted row is inside a list")
    }

    /// Run the shell's operation the way the runtime does: to the end of its chain.
    fn follow_highlight(&mut self) {
        let mut current: Box<dyn Operation<()>> = Box::new(micold_client::ui::picker_into_view());
        loop {
            self.pass(current.as_mut());
            match current.finish() {
                Outcome::Chain(next) => current = next,
                Outcome::None | Outcome::Some(_) => break,
            }
        }
    }

    /// Press `named`, then do what the shell chains after the move.
    fn step(&mut self, named: keyboard::key::Named) {
        self.press(named);
        self.follow_highlight();
    }

    fn assert_highlight_in_view(&mut self, after: &str) {
        let (row, list) = self.highlighted_row();
        let (top, bottom) = list.visible();
        assert!(
            row.y >= top - SLACK && row.y + row.height <= bottom + SLACK,
            "{after}: the highlighted row spans {}..{} and the list shows {top}..{bottom}",
            row.y,
            row.y + row.height,
        );
    }
}

/// What a Down or an Up published, as the direction it asked the highlight to move in.
fn moves(messages: &[Message]) -> usize {
    messages
        .iter()
        .filter(|m| matches!(m, Message::WorktreeForm(FormMsg::IssueHighlightMoved(_))))
        .count()
}

/// U39 (FR-007): with rows of differing height, each Down and each Up moves the highlight by
/// exactly one issue, and Enter picks the issue the highlight is on.
#[test]
fn each_press_moves_the_highlight_by_one_issue_and_enter_picks_it() {
    use keyboard::key::Named;

    let mut form = Form::new(mixed_issues());
    assert_eq!(form.highlight(), None, "nothing is highlighted on opening");

    // Down from the field lands on the first issue; every later Down is one issue further.
    for row in 0..ISSUES {
        let published = form.press(Named::ArrowDown);
        assert_eq!(moves(&published), 1, "Down {row} published {published:?}");
        assert_eq!(form.highlight(), Some(row), "after Down {row}");
    }
    for row in (0..ISSUES - 1).rev() {
        let published = form.press(Named::ArrowUp);
        assert_eq!(moves(&published), 1, "Up to {row} published {published:?}");
        assert_eq!(form.highlight(), Some(row), "after Up to {row}");
    }

    // Enter, part-way down: the list reports the highlighted row, which is that row's issue.
    const PICKED_ROW: usize = 4;
    for _ in 0..PICKED_ROW {
        form.press(Named::ArrowDown);
    }
    let published = form.publish(&pressed(Named::Enter));
    let picked: Vec<usize> = published
        .iter()
        .filter_map(|m| match m {
            Message::WorktreeForm(FormMsg::IssueRowPicked(row)) => Some(*row),
            _ => None,
        })
        .collect();
    assert_eq!(picked, vec![PICKED_ROW], "Enter published {published:?}");
    assert_eq!(
        form.under.state.worktree_form.issue_number_at(PICKED_ROW),
        Some(FIRST_NUMBER + PICKED_ROW as u64),
        "the picked row is the highlighted issue"
    );
}

/// U40 (SC-007): over rows of one to five title lines, the highlighted row is wholly inside the
/// list's viewport after every one of 11 Down and 11 Up presses.
#[test]
fn the_highlighted_row_is_in_view_after_every_down_and_up() {
    use keyboard::key::Named;

    let mut form = Form::new(mixed_issues());
    // Down from the field lands on the first issue; the walk is the 11 presses after it.
    form.step(Named::ArrowDown);
    form.assert_highlight_in_view("on the first issue");
    let (_, list) = form.highlighted_row();
    assert_eq!(
        list.viewport_height,
        8.0 * micold_core::tokens::density::MENU_ITEM_BASE,
        "the list is eight base rows high"
    );

    let mut scrolled = false;
    for press in 1..ISSUES {
        form.step(Named::ArrowDown);
        form.assert_highlight_in_view(&format!("Down {press}"));
        scrolled |= form.highlighted_row().1.scrolled > 0.0;
    }
    assert!(
        scrolled,
        "twelve rows fitted in the list, so nothing was tested"
    );
    for press in 1..ISSUES {
        form.step(Named::ArrowUp);
        form.assert_highlight_in_view(&format!("Up {press}"));
    }
    assert_eq!(form.highlight(), Some(0));
}

/// Eight issues with one-line titles: eight base rows, which is exactly the list's height.
fn eight_short_issues() -> Vec<Issue> {
    (0..8)
        .map(|row| issue(FIRST_NUMBER + row, SHORT_TITLE.to_string()))
        .collect()
}

/// U41 (FR-007): a highlighted row that is already wholly visible causes no scroll, flush against
/// the list's edge included. Eight base rows fill the list exactly, so the last one's bottom edge
/// is the viewport's: an operation that wanted room to spare around the row would move the list.
#[test]
fn a_row_already_wholly_visible_causes_no_scroll() {
    use keyboard::key::Named;

    let mut issues = eight_short_issues();
    // A ninth, so the list can scroll at all: a list that cannot move proves nothing by not moving.
    issues.push(issue(FIRST_NUMBER + 8, SHORT_TITLE.to_string()));
    let mut form = Form::new(issues);

    for row in 0..8 {
        form.step(Named::ArrowDown);
        let (bounds, list) = form.highlighted_row();
        assert_eq!(
            list.scrolled,
            0.0,
            "row {row} spans {}..{} of a list showing {:?}, and the list moved",
            bounds.y,
            bounds.y + bounds.height,
            list.visible(),
        );
    }
    let (last, list) = form.highlighted_row();
    assert!(
        (last.y + last.height - list.visible().1).abs() <= SLACK,
        "precondition: the eighth row ends at {} and the viewport at {}",
        last.y + last.height,
        list.visible().1
    );

    // And the ninth, which is below the fold, does move it.
    form.step(Named::ArrowDown);
    assert!(form.highlighted_row().1.scrolled > 0.0);
}

/// U42 (FR-007): a highlighted row taller than the list cannot be shown whole, so its top is
/// aligned to the viewport's top: that is where its number and title start.
#[test]
fn a_row_taller_than_the_list_is_aligned_to_its_top() {
    use keyboard::key::Named;

    // GitHub's longest title is 256 characters, which is not tall enough to outgrow eight rows;
    // the list makes no promise about the length of what it is handed, so this is longer.
    const TALL_ROW: usize = 2;
    let mut issues = eight_short_issues();
    issues[TALL_ROW] = issue(FIRST_NUMBER + TALL_ROW as u64, title_of(40));
    let mut form = Form::new(issues);

    for _ in 0..=TALL_ROW {
        form.step(Named::ArrowDown);
    }
    assert_eq!(form.highlight(), Some(TALL_ROW));
    let (row, list) = form.highlighted_row();
    assert!(
        row.height > list.viewport_height,
        "precondition: the row is {} tall and the list {}",
        row.height,
        list.viewport_height
    );
    let (top, _) = list.visible();
    assert!(
        (row.y - top).abs() <= SLACK,
        "the row starts at {} and the list shows from {top}",
        row.y
    );

    // From below, too: Up onto it from the row after shows its top, not its end.
    form.step(Named::ArrowDown);
    form.assert_highlight_in_view("on the row after the tall one");
    form.step(Named::ArrowUp);
    let (row, list) = form.highlighted_row();
    assert!(
        (row.y - list.visible().0).abs() <= SLACK,
        "back on it, the row starts at {} and the list shows from {}",
        row.y,
        list.visible().0
    );
}

// ---------------------------------------------------------------------------------------------
// U43 — who chains the operation
// ---------------------------------------------------------------------------------------------

/// Every `.rs` file under `dir`, with its path relative to the crate.
fn sources(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for entry in std::fs::read_dir(dir).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let relative = path
                .strip_prefix(root)
                .expect("under the crate")
                .to_string_lossy()
                .replace('\\', "/");
            out.push((
                relative,
                std::fs::read_to_string(&path).expect("a source file"),
            ));
        }
    }
}

/// The non-comment lines under `src/` that contain `needle`, as `(file, trimmed line)`.
fn lines_naming(needle: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    files.sort();
    let mut found = Vec::new();
    for (file, text) in files {
        for line in text.lines().map(str::trim) {
            if !line.starts_with("//") && line.contains(needle) {
                found.push((file.clone(), line.to_string()));
            }
        }
    }
    found
}

/// U43 (FR-007, FR-029): the shell chains the operation after `FormMsg::IssueHighlightMoved`, and
/// after no other message. The branch picker and `Select` have rows of one height and are not
/// wired, so a second caller would be a change of scope this feature did not make.
///
/// A source check, because the shell's `update` returns an opaque `Task`: nothing can ask one
/// whether it holds this operation.
#[test]
fn only_the_issue_highlight_move_chains_the_operation() {
    // The call, with its parentheses: the definition and the export do not match.
    let calls: Vec<(String, String)> = lines_naming("picker_highlight_into_view()")
        .into_iter()
        .filter(|(_, line)| !line.starts_with("pub fn "))
        .collect();
    assert_eq!(
        calls,
        vec![(
            "src/shell/issues.rs".to_string(),
            "micold_client::ui::picker_highlight_into_view()".to_string()
        )],
        "the operation has exactly one caller, the tail of the issue shell's handler"
    );

    // That call is in the handler of the highlight move, and nowhere else in the file.
    let shell = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/issues.rs"),
    )
    .expect("the issue shell");
    // Without whitespace or trailing commas: rustfmt wraps the handler's `update` call as it sees
    // fit, and a wrapped argument list gains a trailing comma.
    let handler: String = shell
        .split("\npub fn ")
        .find(|item| item.starts_with("on_issue_highlight_moved("))
        .expect("the issue shell handles the highlight move")
        .split_whitespace()
        .collect::<String>()
        .replace(",)", ")");
    assert!(
        handler.contains("FormMsg::IssueHighlightMoved(direction)")
            && handler.contains("picker_highlight_into_view()"),
        "the handler applies the move and then chains the operation:\n{handler}"
    );

    // And `main.rs` routes the message to that handler, which has no other caller.
    let handler_calls = lines_naming("on_issue_highlight_moved(");
    assert_eq!(
        handler_calls
            .iter()
            .map(|(file, _)| file.as_str())
            .collect::<Vec<_>>(),
        vec!["src/main.rs", "src/shell/issues.rs"],
        "one route and one definition: {handler_calls:?}"
    );
    let main: String = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs"),
    )
    .expect("the shell")
    .split_whitespace()
    .collect();
    let arm = "Message::WorktreeForm(FormMsg::IssueHighlightMoved(direction))=>";
    let routed = main
        .split(arm)
        .nth(1)
        .expect("main.rs has an arm for the highlight move");
    assert!(
        routed
            .trim_start_matches('{')
            .starts_with("shell::issues::on_issue_highlight_moved(app,direction)"),
        "the arm routes to the issue shell's handler"
    );
}
