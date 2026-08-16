//! An unavailable row consumes its press (016 BUG-002, T073 — FR-035, 021 FR-012a).
//!
//! 021 FR-012a says that attempting to pick an unavailable branch "MUST do nothing — in particular
//! it MUST NOT close the result list". The reducer honours that to the letter, and
//! `tests/branch_search_state.rs` proves it: the message never arrives, so no state changes.
//!
//! What no state-level test can see is that the press *keeps travelling*. "Not pickable" is
//! expressed by withholding the row's press message, and a `button` with no `on_press` does not
//! capture the event — so it passes through the row, through the floating list (which claims only
//! presses **outside** itself), and lands on whatever is behind. Behind a list that floats past a
//! dialog's edges is that dialog's own scrim, whose press message is its cancellation. Reaching for
//! an in-use branch closed the whole add-worktree form.
//!
//! So this asks the component the question the requirement is actually making: **does the press
//! stop here?** Unpressable and transparent-to-presses are different properties, and only the first
//! one was ever wanted (research R14).
//!
//! In-crate because `material` is `pub(crate)` and the type-ahead cannot be constructed from
//! `tests/` — the same reason `picker_parity` gives. `tests/add_worktree_form_survives_a_refusal.rs`
//! is the other half: this one pins the component's rule, that one pins the consequence in the real
//! dialog.

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{Element, Event, Point, Rectangle, Size, Vector};
use micold_core::tokens::{self, density, Roles};

use super::picker::Row;
use super::Typeahead;

const WINDOW: Size = Size::new(400.0, 800.0);

/// Frames to let the list's entrance finish before pressing anything. Generous — the entrance is a
/// few hundred milliseconds and the point is only that it is over.
const OPEN_FRAMES: u32 = 60;

fn roles() -> Roles {
    tokens::roles(micold_core::theme::ColorScheme::Light)
}

/// What one press on one row produced.
struct Outcome {
    /// Whether anything claimed the event. `false` means it is still travelling.
    captured: bool,
    /// What the press published, if anything.
    messages: Vec<String>,
}

/// Press the row at `index` in an open type-ahead's floating list.
///
/// The press is dispatched to the **overlay**, which is where the runtime sends it first and where
/// the list actually lives — the rows are not in the base layout at all.
fn press_row(rows: Vec<Row>, index: usize) -> Outcome {
    let r = roles();
    let mut element: Element<'_, String> = Typeahead::new("", rows, |q: String| q, r)
        .label("Branch")
        .open(true)
        .on_pick(|i: usize| format!("picked {i}"))
        .into();

    let renderer = super::test_support::renderer();
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);

    // The list *arrives* (feature 022, FR-018): built open, it is still invisible on its first
    // frame, and a wrapper below the visibility threshold passes no input to what it is hiding. So
    // let the entrance run out before pressing anything — the same wait `picker_parity` makes.
    //
    // The frames go to the **overlay**, not to the widget beneath it. That is where the list lives
    // and where the runtime sends events first; a redraw delivered to the base tree never reaches
    // it, and the entrance would never start.
    let origin = std::time::Instant::now();
    for frame in 1..=OPEN_FRAMES {
        let mut settling: Vec<String> = Vec::new();
        let mut shell = Shell::new(&mut settling);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Window(iced::window::Event::RedrawRequested(
                origin + crate::ui::cdk::motion::FRAME * frame,
            )),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
        let mut opening = element
            .as_widget_mut()
            .overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &Rectangle::with_size(WINDOW),
                Vector::ZERO,
            )
            .expect("the type-ahead was built open and floated no list");
        let list = opening.as_overlay_mut().layout(&renderer, WINDOW);
        let mut settling: Vec<String> = Vec::new();
        let mut shell = Shell::new(&mut settling);
        opening.as_overlay_mut().update(
            &Event::Window(iced::window::Event::RedrawRequested(
                origin + crate::ui::cdk::motion::FRAME * frame,
            )),
            Layout::new(&list),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard::Null,
            &mut shell,
        );
    }

    let mut overlay = element
        .as_widget_mut()
        .overlay(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::with_size(WINDOW),
            Vector::ZERO,
        )
        .expect("the type-ahead was built open and floated no list, so there is no row to press");
    let list = overlay.as_overlay_mut().layout(&renderer, WINDOW);

    let centres = row_centres(&list);
    let at = *centres
        .get(index)
        .unwrap_or_else(|| panic!("the list laid out {} rows, not {index}", centres.len()));

    // Press **and** release, because the two answer different halves of the question. The library's
    // button claims the press (which is what stops it travelling) and publishes on the release
    // (which is what picking a row means). A row that does neither is the defect.
    let mut messages: Vec<String> = Vec::new();
    let mut captured = false;
    for (event, is_press) in [
        (
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            true,
        ),
        (
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            false,
        ),
    ] {
        let mut shell = Shell::new(&mut messages);
        overlay.as_overlay_mut().update(
            &event,
            Layout::new(&list),
            mouse::Cursor::Available(at),
            &renderer,
            &mut clipboard::Null,
            &mut shell,
        );
        // Only the press matters for travel: the scrim behind the list publishes its cancellation
        // on a press, so a press that is not claimed here is one that reaches it.
        if is_press {
            captured = shell.is_event_captured();
        }
    }

    Outcome { captured, messages }
}

/// The centre of each row in `list`, top to bottom.
///
/// Rows are found by their height rather than by walking to a known depth: §7.5's item height is
/// the one thing every row has and no wrapper around them does, so this keeps working when the
/// panel gains or loses a container.
fn row_centres(list: &layout::Node) -> Vec<Point> {
    fn walk(node: &layout::Node, offset: Vector, out: &mut Vec<Rectangle>) {
        let bounds = node.bounds() + offset;
        out.push(bounds);
        for child in node.children() {
            walk(child, Vector::new(bounds.x, bounds.y), out);
        }
    }
    let mut all = Vec::new();
    walk(list, Vector::ZERO, &mut all);

    let mut rows: Vec<Rectangle> = all
        .into_iter()
        .filter(|b| (b.height - density::MENU_ITEM_BASE).abs() < 0.5)
        .collect();
    rows.sort_by(|a, b| a.y.total_cmp(&b.y));
    // A row is a button inside a ripple, so the same box appears twice. One point per row.
    rows.dedup_by(|a, b| (a.y - b.y).abs() < 0.5);
    rows.into_iter().map(|b| b.center()).collect()
}

/// Two rows: the first available, the second not.
fn one_of_each() -> Vec<Row> {
    vec![
        Row::new("feat/available", Vec::new()),
        Row::new("feat/in-use", Vec::new()).disabled(),
    ]
}

/// The defect, as a question about the component: a press on an unavailable row must stop there.
///
/// If it does not, the event reaches whatever is behind the list — and what is behind the branch
/// list is the add-worktree dialog's scrim. Nothing about this row can know that, which is exactly
/// why the rule belongs here and not at the call site.
#[test]
fn an_unavailable_row_consumes_its_press() {
    let outcome = press_row(one_of_each(), 1);

    assert!(
        outcome.messages.is_empty(),
        "an unavailable row published {:?}. It must not be pickable at all (021 FR-012a).",
        outcome.messages,
    );
    assert!(
        outcome.captured,
        "a press on an unavailable row was not captured, so it is still travelling — past the row, \
         past the list, to whatever is behind. Behind the branch list is the add-worktree dialog's \
         scrim, whose press message cancels the dialog, so reaching for an in-use branch closes the \
         form and discards every input in it (016 BUG-002, FR-035, SC-009).\n\n\
         \"Unpressable\" is the absence of a press message. It is not, and must not be, \
         transparency to the press itself.",
    );
}

/// …and the check is over a list that does work, so the assertion above cannot pass by the rows
/// having never been laid out or the press having missed them.
#[test]
fn an_available_row_still_picks_and_still_consumes_its_press() {
    let outcome = press_row(one_of_each(), 0);

    assert_eq!(
        outcome.messages,
        vec!["picked 0".to_string()],
        "an available row did not publish its pick, so the fixture is not pressing rows at all and \
         the unavailable-row assertion proves nothing",
    );
    assert!(
        outcome.captured,
        "an available row did not capture its own press either — the two rows must differ in what \
         they publish, never in whether the press stops at them",
    );
}
