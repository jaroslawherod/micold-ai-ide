//! An issue row shows all of its text (feature 038, SC-001, FR-004).
//!
//! A row of the issue picker is two lines, the title and then the reporter with the labels, and
//! neither is truncated: each wraps onto as many lines as the row's width needs and the row grows
//! to hold them. This gate asks the renderer whether that is what was drawn.
//!
//! Every other gate passes a row that fails here. The geometry fixture records the row's nodes and
//! would record a row one line tall as readily as one five lines tall. `containment` reads nodes
//! against their parents, and a paragraph that runs out of its node is not a node.
//! `layout_text_overflow` reads paragraphs, but as the tree mounts, and a floated list is drawn by
//! the overlay pass that run never makes. So the one surface this feature changes is the one none
//! of them looks at.
//!
//! # What it reads
//!
//! `support::layout::painted_text_settled_at`: the pass that settles and draws the overlay, with
//! the layout boxes of the same pass beside the text. The expected text is not written here. It
//! comes from the issues the state itself holds, through `ui::issue_rows`, which is what the form
//! hands the picker, so a state that gains an issue is held to it without this file changing.
//!
//! # Two widths
//!
//! How many lines a title takes is decided by the width the row is given, so one width proves one
//! arrangement. Each state is read at the canonical window and again at [`NARROW`], where the same
//! rows are narrower and taller.
//!
//! # Advanced mode (feature 1186)
//!
//! The results of a JQL query are drawn by the same row builder in the same list (FR-006), so the
//! advanced state is read by the same findings. Its expected rows are the results, in Jira's order.
//! The Basic/Advanced switch adds a line to the dialog; [`the_query_mode_switch_keeps_the_panel_placement`]
//! holds the dialogs that show it to the panel-placement gate's two rules (SC-005).
//!
//! # Compiled into the `layout_snapshot` binary
//!
//! It reads covered states by name, and that binary is where they are in scope.

use crate::support::covered_states::covered_states;
use crate::support::layout::{self as lay, Layer, LayoutRecord, Overflow, StateUnderTest};
use iced::Size;
use micold_core::github::Issue;
use micold_core::theme::ColorScheme;

/// Half a pixel, matching every geometry gate beside this one.
const TOLERANCE: f32 = 0.5;

/// The covered states whose rows this gate reads.
///
/// Named rather than discovered. "Every state with an open issue list" would be satisfied by none,
/// and a state renamed or deleted would take its rows out of the gate without a failure anywhere.
const STATES: &[&str] = &[
    "add-worktree-dialog-issue-list-longest-titles",
    "add-worktree-dialog-issue-list-labels",
    "add-worktree-dialog-issue-list-mixed-heights",
    ADVANCED_RESULTS,
];

/// The one of [`STATES`] in advanced mode: a query's results, open (feature 1186, FR-006).
const ADVANCED_RESULTS: &str = "add-worktree-dialog-jira-advanced-results";

/// Every dialog that shows the Basic/Advanced switch: basic mode on the Jira source, and each
/// advanced state (feature 1186, FR-001).
const SWITCH_STATES: &[&str] = &[
    "add-worktree-dialog-jira-loaded",
    "add-worktree-dialog-jira-advanced-prompt",
    ADVANCED_RESULTS,
    "add-worktree-dialog-jira-advanced-capped",
    "add-worktree-dialog-jira-advanced-running",
    "add-worktree-dialog-jira-advanced-refused",
    "add-worktree-dialog-jira-advanced-no-match",
    "add-worktree-dialog-jira-advanced-rejected",
];

/// The one of [`STATES`] whose rows are meant to differ in height.
const MIXED_HEIGHTS: &str = "add-worktree-dialog-issue-list-mixed-heights";

/// A window narrow enough that the dialog, and so every row in its list, is narrower than at the
/// canonical size — and exactly as tall, because the list takes its height from the room below the
/// field and a shorter window would be measuring that instead.
///
/// Narrower than the application lets its window become (`app::MIN_WINDOW_SIZE` is 640 wide, where
/// the dialog still has its full width). That is the point: a row has to wrap to whatever width it
/// is given, and this is a width no row was tuned at.
const NARROW: Size = Size::new(440.0, lay::WINDOW.height);

/// The list's scrollable in the overlay layer: what is visible of the rows.
///
/// Overlay root, the floating layer's positioning wrappers, the menu panel, the container that
/// caps the list at eight rows, then the scrollable. Its one child is the column of rows, and that
/// column's children are the rows. `rows_of` checks the count against the state's issues, so a
/// tree that renumbers fails here rather than reading some other node as a row.
const LIST_VIEWPORT: &[usize] = &[0, 0, 0, 0, 0, 0];

/// [`LIST_VIEWPORT`] in advanced mode, one level deeper: the query field shares a row with **Run**,
/// and the row groups the overlays of its children (feature 1186).
const ADVANCED_LIST_VIEWPORT: &[usize] = &[0, 0, 0, 0, 0, 0, 0];

/// Where the list's scrollable is in `name`'s overlay.
fn list_viewport(name: &str) -> &'static [usize] {
    if name == ADVANCED_RESULTS {
        ADVANCED_LIST_VIEWPORT
    } else {
        LIST_VIEWPORT
    }
}

/// A covered state by name, built.
fn build(name: &str) -> StateUnderTest {
    let covered = covered_states()
        .iter()
        .find(|c| c.name == name)
        .unwrap_or_else(|| panic!("no covered state is named {name}; see STATES"));
    (covered.build)()
}

/// The issues behind the state's rows, in row order: in advanced mode the results as Jira returned
/// them, else the held issues in match order.
fn issues_of(under: &StateUnderTest) -> Vec<Issue> {
    let form = under
        .state
        .worktree_form
        .form
        .as_ref()
        .expect("an issue-list state opens the add-worktree form");
    if form.shows_advanced() {
        if let Some(results) = form.jira_query.results() {
            return results.to_vec();
        }
    }
    let held = form.issues.held();
    form.issue_matches
        .iter()
        .filter_map(|(index, _)| held.get(*index).map(|issue| (*issue).clone()))
        .collect()
}

/// The title line and the details line of every row, as the form hands them to the picker.
fn lines_of(under: &StateUnderTest) -> Vec<(String, String)> {
    let form = under
        .state
        .worktree_form
        .form
        .as_ref()
        .expect("an issue-list state opens the add-worktree form");
    micold_client::ui::issue_rows(form)
        .0
        .into_iter()
        .map(|row| {
            let details = row.details.map(|(text, _)| text).unwrap_or_default();
            (row.label, details)
        })
        .collect()
}

/// What one pass over a state painted and laid out.
struct Pass {
    text: Vec<Overflow>,
    boxes: Vec<LayoutRecord>,
}

fn pass(under: &StateUnderTest, window: Size) -> Pass {
    let mut renderer = lay::renderer();
    let (text, boxes) = lay::painted_text_settled_at(lay::view_of(under), &mut renderer, window);
    Pass { text, boxes }
}

impl Pass {
    fn overlay(&self, path: &[usize]) -> Option<&LayoutRecord> {
        self.boxes
            .iter()
            .find(|b| b.layer == Layer::Overlay && b.path == path)
    }

    /// The list's viewport at `at` and its rows, top to bottom.
    fn rows(&self, at: &[usize]) -> Option<(&LayoutRecord, Vec<&LayoutRecord>)> {
        let viewport = self.overlay(at)?;
        let mut content = at.to_vec();
        content.push(0);
        let rows = self
            .boxes
            .iter()
            .filter(|b| {
                b.layer == Layer::Overlay
                    && b.path.len() == content.len() + 1
                    && b.path.starts_with(&content)
            })
            .collect();
        Some((viewport, rows))
    }

    /// Everything the overlay painted with its origin inside `row`.
    fn text_in(&self, row: &LayoutRecord) -> Vec<&Overflow> {
        self.text
            .iter()
            .filter(|t| t.layer == Layer::Overlay && holds(row, t.origin))
            .collect()
    }
}

fn holds(row: &LayoutRecord, at: iced::Point) -> bool {
    at.x >= row.x - TOLERANCE
        && at.x <= row.x + row.width + TOLERANCE
        && at.y >= row.y - TOLERANCE
        && at.y <= row.y + row.height + TOLERANCE
}

fn describe(r: &LayoutRecord) -> String {
    format!(
        "{:.1}, {:.1} to {:.1}, {:.1}",
        r.x,
        r.y,
        r.x + r.width,
        r.y + r.height,
    )
}

/// Everything wrong with one state's rows at one window size, one line per finding.
fn findings(name: &str, window: Size) -> Vec<String> {
    let under = build(name);
    let issues = issues_of(&under);
    let lines = lines_of(&under);
    let pass = pass(&under, window);
    let at = format!("{name} at {}x{}", window.width, window.height);

    // No rows is a finding, not a pass. A list that did not open, or one whose rows this gate can
    // no longer find, would otherwise have every one of its zero rows show all of its text.
    if issues.is_empty() || issues.len() != lines.len() {
        return vec![format!(
            "{at}: the state holds {} issue(s) and hands the picker {} row(s); it has to hold at \
             least one, and a row for each",
            issues.len(),
            lines.len(),
        )];
    }
    let Some((viewport, rows)) = pass.rows(list_viewport(name)) else {
        return vec![format!(
            "{at}: no list was laid out in the overlay layer at {}; the list is closed, or the \
             overlay's tree has changed shape around LIST_VIEWPORT",
            lay::path_token(list_viewport(name)),
        )];
    };
    if rows.len() != issues.len() {
        return vec![format!(
            "{at}: the list laid out {} row(s) for {} issue(s)",
            rows.len(),
            issues.len(),
        )];
    }

    let mut found = Vec::new();

    for (index, row) in rows.iter().enumerate() {
        let issue = &issues[index];
        let (title_line, details_line) = &lines[index];
        let row_name = format!("{at}: row {index} (#{})", issue.number());

        // Wholly inside the viewport. A row partly below it is a row whose last lines are reached
        // only by scrolling, and this gate would then be reading text nobody can see.
        let outside = row.x < viewport.x - TOLERANCE
            || row.y < viewport.y - TOLERANCE
            || row.x + row.width > viewport.x + viewport.width + TOLERANCE
            || row.y + row.height > viewport.y + viewport.height + TOLERANCE;
        if outside {
            found.push(format!(
                "{row_name} is not wholly inside the list's viewport: the row is {}, the viewport \
                 {}",
                describe(row),
                describe(viewport),
            ));
        }

        let text = pass.text_in(row);
        let joined = text
            .iter()
            .map(|t| t.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        // The whole of both lines, then the parts by name: the lines are what the form asked the
        // picker to draw, the parts are what the requirement is about, and a line that came out of
        // `issue_rows` without its reporter would satisfy the first and not the second.
        let mut wanted: Vec<(&str, &str)> = vec![
            ("title line", title_line.as_str()),
            ("details line", details_line.as_str()),
            ("reporter", issue.reporter()),
        ];
        wanted.extend(issue.labels().iter().map(|l| ("label", l.as_str())));
        for (what, expected) in wanted {
            if !joined.contains(expected) {
                found.push(format!(
                    "{row_name} does not paint its {what} {expected:?}; it painted {joined:?}",
                ));
            }
        }

        // Every paragraph inside the row. Its origin is inside by selection, so what is left is its
        // extent. `content` is the paragraph's source text, which is whole however much of it was
        // painted, so the checks above cannot see a cut: these do. Downwards, a paragraph that
        // wrapped to more lines than the row grew for ends below the row, over the next one, and
        // one taller than its clip loses its last lines.
        for t in &text {
            let bottom = t.origin.y + t.natural_height;
            if bottom > row.y + row.height + TOLERANCE {
                found.push(format!(
                    "{row_name} paints text down to {bottom:.1}, below the row's end at {:.1}: the \
                     row did not grow for every line of {:?}",
                    row.y + row.height,
                    t.content,
                ));
            }
            let clip_bottom = t.clip.y + t.clip.height;
            if t.origin.y < t.clip.y - TOLERANCE || bottom > clip_bottom + TOLERANCE {
                found.push(format!(
                    "{row_name} paints text from {:.1} to {bottom:.1} under a clip from {:.1} to \
                     {clip_bottom:.1}: lines of {:?} are cut",
                    t.origin.y, t.clip.y, t.content,
                ));
            }
        }

        // Sideways: a paragraph that did not wrap wants more than the row has, and is drawn across
        // the list's edge or cut off at it.
        for t in &text {
            if t.natural_width > row.width + TOLERANCE {
                found.push(format!(
                    "{row_name} paints text {:.1}px wide in a row {:.1}px wide, starting at {:.1}, \
                     {:.1}: {:?}",
                    t.natural_width, row.width, t.origin.x, t.origin.y, t.content,
                ));
            }
        }
    }

    // And nothing the list painted is in no row at all: text that started above, below or beside
    // every row was selected by none of them, and would otherwise go unread.
    for t in pass.text.iter().filter(|t| t.layer == Layer::Overlay) {
        if holds(viewport, t.origin) && !rows.iter().any(|row| holds(row, t.origin)) {
            found.push(format!(
                "{at}: the list paints text at {:.1}, {:.1} that is in none of its rows: {:?}",
                t.origin.x, t.origin.y, t.content,
            ));
        }
    }

    found
}

fn assert_every_row_shows_all_of_its_text(window: Size) {
    let found: Vec<String> = STATES
        .iter()
        .flat_map(|name| findings(name, window))
        .collect();
    assert!(
        found.is_empty(),
        "{} finding(s): an issue row has to show its whole title, its reporter and every label, \
         inside the row and inside the list (038 SC-001, FR-004). A row is two wrapping lines; if \
         one of them stopped wrapping it is drawn on a single line past the row's edge, and if the \
         row stopped growing its last lines are under the row below.\n  {}",
        found.len(),
        found.join("\n  "),
    );
}

/// At the canonical window, where the fixture records these rows.
#[test]
fn every_issue_row_shows_all_of_its_text() {
    assert_every_row_shows_all_of_its_text(lay::WINDOW);
}

/// And where the rows are narrower: the same text on more lines, in taller rows.
#[test]
fn every_issue_row_shows_all_of_its_text_in_a_narrow_window() {
    assert_every_row_shows_all_of_its_text(NARROW);
}

/// The narrow window is narrow *for the rows*: a size at which the dialog kept its width would run
/// the same arrangement twice and call it two.
#[test]
fn the_narrow_window_narrows_the_rows() {
    for name in STATES {
        let under = build(name);
        let width = |window: Size| {
            let pass = pass(&under, window);
            let (_, rows) = pass.rows(list_viewport(name)).expect("the list is open");
            rows.first().expect("the list has a row").width
        };
        let (canonical, narrow) = (width(lay::WINDOW), width(NARROW));
        assert!(
            narrow < canonical - TOLERANCE,
            "{name}: a row is {narrow:.1}px wide in the narrow window and {canonical:.1}px in the \
             canonical one; NARROW no longer narrows the dialog",
        );
    }
}

/// The states hold the cases the gate exists for, so that passing it means something: the longest
/// title with no space to break at, twenty labels, no labels, and rows that differ in height.
///
/// A state edited down to short titles would pass every assertion above and prove nothing.
#[test]
fn the_states_hold_the_hard_cases() {
    let issues: Vec<Issue> = STATES
        .iter()
        .flat_map(|name| issues_of(&build(name)))
        .collect();

    assert!(
        issues
            .iter()
            .any(|i| i.title().chars().count() == 256 && !i.title().contains(char::is_whitespace)),
        "no state holds a 256-character title without a space, the title only a glyph break wraps",
    );
    assert!(
        issues
            .iter()
            .any(|i| i.title().chars().count() == 256 && i.title().contains(' ')),
        "no state holds a 256-character title of words",
    );
    assert!(
        issues.iter().any(|i| i.labels().len() == 20),
        "no state holds an issue with 20 labels",
    );
    assert!(
        issues.iter().any(|i| i.labels().is_empty()),
        "no state holds an issue without labels",
    );

    let under = build(MIXED_HEIGHTS);
    let pass = pass(&under, lay::WINDOW);
    let (_, rows) = pass.rows(LIST_VIEWPORT).expect("the list is open");
    let mut heights: Vec<f32> = rows.iter().map(|r| r.height).collect();
    let count = heights.len();
    heights.dedup_by(|a, b| (*a - *b).abs() <= TOLERANCE);
    assert!(
        count >= 4 && heights.len() == count,
        "{MIXED_HEIGHTS} has to hold at least four rows, each a different height from the one \
         above it; its rows are {:?}px tall",
        rows.iter().map(|r| r.height).collect::<Vec<_>>(),
    );
}

/// The advanced state is what its name says: advanced mode with results, so the rows read above are
/// a query's (feature 1186). A state that fell back to basic mode would pass the gate on basic rows.
#[test]
fn the_advanced_state_lists_a_querys_results() {
    let under = build(ADVANCED_RESULTS);
    let form = under
        .state
        .worktree_form
        .form
        .as_ref()
        .expect("the state opens the add-worktree form");
    assert!(
        form.shows_advanced(),
        "{ADVANCED_RESULTS} is not in advanced mode"
    );
    let results = form
        .jira_query
        .results()
        .expect("{ADVANCED_RESULTS} has run a query");
    assert!(!results.is_empty(), "{ADVANCED_RESULTS} lists no result");
    assert!(
        form.issue_matches.is_empty() && form.issues.held().is_empty(),
        "{ADVANCED_RESULTS} holds a basic list too, so its rows could be either"
    );
}

/// The dialogs with the Basic/Advanced switch keep the panel placement the basic dialog has: no
/// panel over the app bar, none past the window (feature 1186, FR-006, SC-005). The switch is one
/// more line in the tallest dialog, so this is where it would push the dialog out.
#[test]
fn the_query_mode_switch_keeps_the_panel_placement() {
    let renderer = lay::renderer();
    let mut found = Vec::new();
    for name in SWITCH_STATES {
        let covered = covered_states()
            .iter()
            .find(|c| c.name == *name)
            .unwrap_or_else(|| panic!("no covered state is named {name}; see SWITCH_STATES"));
        let under = (covered.build)();
        let form = under
            .state
            .worktree_form
            .form
            .as_ref()
            .expect("the state opens the add-worktree form");
        if !form.offers_query_modes() {
            found.push(format!(
                "{name}: the dialog does not show the Basic/Advanced switch"
            ));
        }
        let records = lay::records_for(covered, &renderer, ColorScheme::Light);
        let panels: Vec<_> = crate::panel_placement::anchored_panels(&records).collect();
        if panels.is_empty() {
            found.push(format!("{name}: no dialog panel was laid out"));
        }
        for panel in panels {
            if panel.y < micold_core::tokens::anatomy::app_bar::BOTTOM_EDGE - TOLERANCE {
                found.push(format!(
                    "{name}: the dialog starts at y={:.1}, inside the app bar",
                    panel.y
                ));
            }
            let past = panel.y + panel.height - lay::WINDOW.height;
            if past > TOLERANCE {
                found.push(format!(
                    "{name}: the dialog hangs {past:.1}px past the window"
                ));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n  "));
}
