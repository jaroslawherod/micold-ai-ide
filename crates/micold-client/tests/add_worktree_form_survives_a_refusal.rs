//! Reaching for a branch the app refuses costs the user nothing (016 BUG-002, T074 — FR-034,
//! FR-035, SC-009).
//!
//! The branch list floats over the form and past its edges (feature 021, FR-011/AS8), so some of
//! its rows are drawn where the dialog's own card is not. A row that declines its press lets the
//! press through to what is behind — and what is behind the card is the dialog's scrim, whose press
//! message is `AddWorktreeCancelled`. Pressing an in-use branch closed the form and discarded every
//! input in it.
//!
//! `tests/branch_search_state.rs` cannot see this and never could: the reducer is correct, the
//! refusal is silent, and the message that actually arrives is published by the widget tree, from a
//! surface no state-level test builds. So this drives the real view, at real coordinates.
//!
//! # How the press is dispatched
//!
//! Exactly as the runtime does it (`iced_runtime::user_interface::UserInterface::update`): the
//! overlay is offered the event first, and the widget tree beneath sees it only if the overlay did
//! not capture it. Written out here rather than pulled in, because that ordering *is* the thing
//! under test — a test that dispatched to the base tree directly would report the defect whether or
//! not it existed, and one that dispatched only to the overlay would report it fixed either way.
//!
//! `material::picker_press` is the other half: this one pins the consequence in the real dialog,
//! that one pins the rule in the component every future picker will inherit.

use std::path::PathBuf;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{window, Event, Point, Rectangle, Size, Vector};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::worktree_form::BranchSource;
use micold_client::ui;
use micold_core::env_include::EnvIncludeOutcome;
use micold_core::sandbox::lifecycle::SandboxState;
use micold_core::tokens::density;
use micold_core::worktree::{BlockReason, BranchCandidate, BranchOrigin};

/// Big enough for the dialog to sit in the middle with room below it — which is where the rows that
/// matter are drawn.
const VIEWPORT: Size = Size {
    width: 1280.0,
    height: 800.0,
};

/// Frames to let the dialog and its list finish arriving. Both animate; pressing mid-entrance would
/// land on a row that is not yet where it will be.
const SETTLE_FRAMES: u32 = 60;

/// Enough branches that the list outgrows the card it hangs from.
const BRANCHES: usize = 12;

/// The two values `ui::view` borrows for as long as the element lives. `static` so the element can
/// outlive the call that built it — neither is what this test is about, and both are inert.
static OUTCOME: EnvIncludeOutcome = EnvIncludeOutcome::Disabled;
static CONNECTION: ConnectionStatus = ConnectionStatus::Connected;

fn renderer() -> iced::Renderer {
    use iced::advanced::renderer::Headless;
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll, Waker};

    fn block_on<F: Future>(f: F) -> F::Output {
        let mut f = Box::pin(f);
        let mut cx = Context::from_waker(Waker::noop());
        loop {
            if let Poll::Ready(v) = Pin::as_mut(&mut f).poll(&mut cx) {
                return v;
            }
            std::hint::spin_loop();
        }
    }

    block_on(<iced::Renderer as Headless>::new(
        ui::ROBOTO,
        iced::Pixels(14.0),
        Some("tiny-skia"),
    ))
    .expect("the tiny-skia headless renderer must construct without a GPU")
}

/// A branch that some worktree outside the app is holding — the case the user actually hit.
fn blocked(name: &str) -> BranchCandidate {
    BranchCandidate {
        name: name.to_string(),
        origin: BranchOrigin::Local,
        blocked_by: Some(BlockReason::CheckedOutOutsideApp {
            path: PathBuf::from("/elsewhere/worktrees").join(name.replace('/', "-")),
        }),
    }
}

/// The same branch, free.
fn available(name: &str) -> BranchCandidate {
    BranchCandidate {
        name: name.to_string(),
        origin: BranchOrigin::Local,
        blocked_by: None,
    }
}

/// The add-worktree form, on its existing-branch half, with its list open over `candidates`.
fn form_with(candidates: Vec<BranchCandidate>) -> State {
    let mut state = State::default();
    state.update(Message::AddWorktreeOpened);
    state.update(Message::AddWorktreeSourceChanged(BranchSource::Existing));
    state.update(Message::AddWorktreeBranchesListed(candidates));
    state.update(Message::AddWorktreeBranchFocused);
    state
}

/// `BRANCHES` names, all of them held elsewhere unless `free` names one.
fn candidates(free: Option<usize>) -> Vec<BranchCandidate> {
    (0..BRANCHES)
        .map(|i| {
            let name = format!("feat/branch-{i:02}");
            if free == Some(i) {
                available(&name)
            } else {
                blocked(&name)
            }
        })
        .collect()
}

/// The view, laid out and settled, ready to be pressed.
struct Screen<'a> {
    element: iced::Element<'a, Message>,
    tree: Tree,
    node: layout::Node,
    renderer: iced::Renderer,
}

impl<'a> Screen<'a> {
    fn of(state: &'a State) -> Self {
        let renderer = renderer();
        let limits = layout::Limits::new(Size::ZERO, VIEWPORT);

        // The view is built once and kept, exactly as the runtime keeps it between frames: a fresh
        // element each frame would restart every entrance and nothing would ever settle.
        let mut element = ui::view(
            state,
            None,
            None,
            0,
            None,
            &OUTCOME,
            &CONNECTION,
            &SandboxState::Disabled,
        );
        let mut tree = Tree::new(&element);
        let mut node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);

        let origin = std::time::Instant::now();
        for frame in 1..=SETTLE_FRAMES {
            let at = origin + std::time::Duration::from_millis(16 * u64::from(frame));
            let tick = Event::Window(window::Event::RedrawRequested(at));

            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            element.as_widget_mut().update(
                &mut tree,
                &tick,
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(VIEWPORT),
            );

            // The floating list keeps its own clock, and it only advances when the frame reaches
            // it — which, in the runtime, is via the overlay and not the tree beneath.
            if let Some(mut floating) = element.as_widget_mut().overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &Rectangle::with_size(VIEWPORT),
                Vector::ZERO,
            ) {
                let list = floating.as_overlay_mut().layout(&renderer, VIEWPORT);
                let mut messages = Vec::new();
                let mut shell = Shell::new(&mut messages);
                floating.as_overlay_mut().update(
                    &tick,
                    Layout::new(&list),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard::Null,
                    &mut shell,
                );
            }

            node = element
                .as_widget_mut()
                .layout(&mut tree, &renderer, &limits);
        }

        Self {
            element,
            tree,
            node,
            renderer,
        }
    }

    /// Where each row of the open list sits, top to bottom.
    fn rows(&mut self) -> Vec<Rectangle> {
        let mut floating = self
            .element
            .as_widget_mut()
            .overlay(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &Rectangle::with_size(VIEWPORT),
                Vector::ZERO,
            )
            .expect("the branch list is open, so the form must be floating one");
        let list = floating.as_overlay_mut().layout(&self.renderer, VIEWPORT);

        fn walk(node: &layout::Node, offset: Vector, out: &mut Vec<Rectangle>) {
            let bounds = node.bounds() + offset;
            out.push(bounds);
            for child in node.children() {
                walk(child, Vector::new(bounds.x, bounds.y), out);
            }
        }
        let mut all = Vec::new();
        walk(&list, Vector::ZERO, &mut all);

        let mut rows: Vec<Rectangle> = all
            .into_iter()
            .filter(|b| (b.height - density::MENU_ITEM_BASE).abs() < 0.5)
            .collect();
        rows.sort_by(|a, b| a.y.total_cmp(&b.y));
        rows.dedup_by(|a, b| (a.y - b.y).abs() < 0.5);
        rows
    }

    /// Press at `at` the way the runtime would: the overlay first, then the tree beneath it — and
    /// the tree beneath **only if** the overlay let the event go.
    fn press(&mut self, at: Point) -> Vec<Message> {
        let mut messages = Vec::new();
        for event in press_and_release() {
            let mut captured = false;
            if let Some(mut floating) = self.element.as_widget_mut().overlay(
                &mut self.tree,
                Layout::new(&self.node),
                &self.renderer,
                &Rectangle::with_size(VIEWPORT),
                Vector::ZERO,
            ) {
                let list = floating.as_overlay_mut().layout(&self.renderer, VIEWPORT);
                let mut shell = Shell::new(&mut messages);
                floating.as_overlay_mut().update(
                    &event,
                    Layout::new(&list),
                    mouse::Cursor::Available(at),
                    &self.renderer,
                    &mut clipboard::Null,
                    &mut shell,
                );
                captured = shell.is_event_captured();
            }
            if !captured {
                let mut shell = Shell::new(&mut messages);
                self.element.as_widget_mut().update(
                    &mut self.tree,
                    &event,
                    Layout::new(&self.node),
                    mouse::Cursor::Available(at),
                    &self.renderer,
                    &mut clipboard::Null,
                    &mut shell,
                    &Rectangle::with_size(VIEWPORT),
                );
            }
        }
        messages
    }

    /// What the widget tree **alone** would do with a press at `at`, ignoring the list floating over
    /// it. Used only to establish where the dialog's card ends: a point that cancels the form from
    /// here is a point over the scrim, which is to say outside the card.
    fn press_beneath_the_list(&mut self, at: Point) -> Vec<Message> {
        let mut messages = Vec::new();
        for event in press_and_release() {
            let mut shell = Shell::new(&mut messages);
            self.element.as_widget_mut().update(
                &mut self.tree,
                &event,
                Layout::new(&self.node),
                mouse::Cursor::Available(at),
                &self.renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(VIEWPORT),
            );
        }
        messages
    }
}

/// A full click: the library's button claims the press and publishes on the release, and the
/// scrim's press handler fires on the press. Both halves are sent so neither is assumed.
fn press_and_release() -> [Event; 2] {
    [
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
    ]
}

/// The first row that hangs past the dialog's card, and its index.
///
/// "Past the card" is established by asking the widget tree what it would do with a press there:
/// if it cancels the form, the point is over the scrim and the card does not cover it. That is the
/// only geometry this test needs, and it is measured rather than assumed — a viewport or a dialog
/// that changed shape would make this test say so instead of quietly passing over nothing.
fn first_row_past_the_card(state: &State) -> (usize, Point) {
    let mut screen = Screen::of(state);
    let rows = screen.rows();
    assert!(
        rows.len() >= 2,
        "the list laid out {} rows; the fixture is meant to open a list of {BRANCHES}",
        rows.len(),
    );

    for (index, row) in rows.iter().enumerate() {
        let at = row.center();
        // A fresh screen per probe: the probe itself cancels the form when it lands on the scrim,
        // and a cancelled form floats no list to measure.
        let mut probe = Screen::of(state);
        if probe
            .press_beneath_the_list(at)
            .iter()
            .any(|m| matches!(m, Message::AddWorktreeCancelled))
        {
            return (index, at);
        }
    }

    panic!(
        "no row of the branch list lies outside the dialog's card, so this test would pass over \
         nothing. The defect needs a row drawn where the card is not — widen the list, shrink the \
         viewport, or check whether the list has stopped floating past the form (feature 021, \
         FR-011)."
    );
}

/// The defect: pressing a branch the app refuses must not close the form.
#[test]
fn pressing_an_in_use_branch_leaves_the_form_open() {
    let state = form_with(candidates(None));
    let (index, at) = first_row_past_the_card(&state);

    let mut screen = Screen::of(&state);
    let published = screen.press(at);

    assert!(
        !published
            .iter()
            .any(|m| matches!(m, Message::AddWorktreeCancelled)),
        "pressing row {index} of the branch list — an in-use branch, drawn past the dialog's own \
         edge — cancelled the add-worktree form. The user reached for a branch and lost the form \
         and everything typed into it, with no message and no way to tell a refusal from a \
         cancellation they never made (FR-034, FR-035, SC-009; 021 FR-012a says this press must do \
         nothing at all).\n\nWhat it published: {published:?}",
    );
    assert!(
        published.is_empty(),
        "pressing an in-use branch published {published:?}. It must do nothing whatsoever — not \
         select the branch, not close the list, and not close the form (021 FR-012a).",
    );
}

/// …and the same press at the same place on an *available* branch still picks it.
///
/// Without this the test above passes just as well if the row is not there at all, if the press
/// misses the list entirely, or if the whole list has stopped accepting input.
#[test]
fn the_same_press_on_an_available_branch_still_selects_it() {
    // Which row is past the card is a property of the geometry, not of the branches, so it is
    // measured on the all-blocked fixture and then freed in place.
    let blocked_state = form_with(candidates(None));
    let (index, at) = first_row_past_the_card(&blocked_state);

    let state = form_with(candidates(Some(index)));
    let mut screen = Screen::of(&state);
    let published = screen.press(at);

    assert!(
        published
            .iter()
            .any(|m| matches!(m, Message::AddWorktreeBranchSelected(_))),
        "pressing row {index} of the branch list did not select the available branch under it, so \
         the fixture is not pressing rows at all and the refusal test proves nothing. Published: \
         {published:?}",
    );
    assert!(
        !published
            .iter()
            .any(|m| matches!(m, Message::AddWorktreeCancelled)),
        "selecting an available branch cancelled the form: {published:?}",
    );
}
