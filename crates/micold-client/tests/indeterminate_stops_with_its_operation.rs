//! The indeterminate indicator animates only while its operation runs (feature 018, T079 —
//! FR-039d, FR-039a, SC-017).
//!
//! FR-039d is the one animation in this feature whose settle condition is **external**. Every other
//! transition arrives at a target and stops asking; this one has no target — the bar travels across
//! the track for as long as there is something to report — so it stops only when the thing it
//! reports on ends and the indicator is unmounted.
//!
//! That makes it the one animation that can hold the render loop awake for ever. `Bar` asks for a
//! frame on every frame it exists, which is correct while a worktree is being created and is a
//! permanent 60fps wakeup if it is ever mounted with nothing in flight. FR-039d says so in as many
//! words: "an indeterminate indicator visible with no operation in flight is a defect."
//!
//! # Why this is asserted here rather than left to the walkthrough
//!
//! The guard is one `if` in `ui/worktree_form.rs`, and it is correct today. Nothing failed the build
//! if it stopped being. `tests/idle_requests_no_frames.rs` covers the *primitive* — that `Progress`
//! only asks while it is moving — and every word of that stays true while this defect is present,
//! because a mounted `Bar` genuinely is still moving. quickstart §B9 asks an operator to watch for
//! it, which finds it only if someone happens to run the walkthrough.
//!
//! So this drives the real view and asks the question the requirement asks: with nothing running,
//! does anything on screen want another frame?

use std::time::Instant;

use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, Layout, Shell};
use iced::{window, Event, Size};
use micold_client::app::{Message, State};
use micold_client::features::connection::ConnectionStatus;
use micold_client::features::worktree_form::WorktreeFormStatus;
use micold_client::ui;

/// The window the view is laid out in. Large enough that nothing is clipped away unrendered.
const VIEWPORT: Size = Size {
    width: 1280.0,
    height: 800.0,
};

/// Drive the real view frame by frame and report how many frames it took to stop asking for more.
///
/// `None` means it never stopped within `budget`.
///
/// Frames rather than a single tick, because "at rest" is a destination and not a starting state:
/// a dialog that has just opened is *legitimately* animating — it is fading in — and asking the
/// question one frame after opening it measures the entrance, not the rest. What FR-039a requires
/// is that the application **returns** to rest and stays there, which is only observable by letting
/// the transitions run out.
///
/// The widget tree is kept across frames and the element rebuilt against it, exactly as the runtime
/// does. A fresh tree each frame would discard every transition's progress and report quiescence
/// that had never happened.
fn frames_until_rest(state: &State, budget: usize) -> Option<usize> {
    let connection = ConnectionStatus::Connected;
    let outcome = micold_core::env_include::EnvIncludeOutcome::Disabled;
    let renderer = test_renderer();
    let limits = layout::Limits::new(Size::ZERO, VIEWPORT);
    let start = Instant::now();

    let mut tree = Tree::new(ui::view(state, None, None, 0, None, &outcome, &connection));

    for frame in 0..budget {
        let mut element = ui::view(state, None, None, 0, None, &outcome, &connection);
        tree.diff(&element);
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);

        let mut messages: Vec<Message> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        let mut clipboard = iced::advanced::clipboard::Null;
        // A distinct instant per frame: the motion primitive advances once per frame and tells
        // frames apart by the timestamp the redraw carries.
        let now = start + std::time::Duration::from_millis(16 * frame as u64 + 16);
        element.as_widget_mut().update(
            &mut tree,
            &Event::Window(window::Event::RedrawRequested(now)),
            Layout::new(&node),
            // No pointer over the window: FR-039a's "at rest" excludes hover, and a cursor resting
            // on a row would legitimately animate its hover fade.
            mouse::Cursor::Unavailable,
            &renderer,
            &mut clipboard,
            &mut shell,
            &iced::Rectangle::new(iced::Point::ORIGIN, VIEWPORT),
        );

        if !matches!(shell.redraw_request(), window::RedrawRequest::NextFrame) {
            return Some(frame + 1);
        }
    }
    None
}

/// The CPU rasteriser, so the view can be laid out without a GPU.
fn test_renderer() -> iced::Renderer {
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

/// A state with the add-worktree form open and nothing in flight.
fn form_open() -> State {
    let mut state = State::default();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::Opened,
    ));
    state
}

/// With the form open and no create running, nothing on screen wants a frame.
///
/// The form is the screen that *can* show the indicator, which is what makes it the right place to
/// ask. A form sitting idle asking for 60 frames a second is the defect FR-039d names.
#[test]
fn an_idle_form_asks_for_no_frames() {
    let state = form_open();
    assert_eq!(
        state.worktree_form.as_ref().map(|f| f.status),
        Some(WorktreeFormStatus::Editing),
        "the fixture is meant to be an open form with nothing in flight"
    );

    // Generous: the dialog's entrance is a few hundred milliseconds of 16ms frames, and the point
    // is that the count is *bounded*, not what it is.
    let settled = frames_until_rest(&state, 600);
    assert!(
        settled.is_some(),
        "the add-worktree form never stopped asking for frames, over 600 of them, with nothing \
         running. The indeterminate indicator has no target to arrive at, so whatever is asking \
         will keep asking — the application never returns to rest and SC-017 stops holding \
         (FR-039d)."
    );
}

/// While a create *is* running, the indicator animates.
///
/// The other half, and the one that keeps the test above from passing for the wrong reason. An
/// assertion that nothing ever asks for a frame is satisfied just as well by an indicator that was
/// deleted, by a view that failed to compose, or by a fixture that never opened the form at all.
/// This says the difference is observable: the same screen, one field changed, and the answer flips.
#[test]
fn a_running_create_animates_its_indicator() {
    let mut state = form_open();
    state.update(Message::WorktreeForm(
        micold_client::features::worktree_form::Msg::CreateStarted(Default::default()),
    ));
    assert_eq!(
        state.worktree_form.as_ref().map(|f| f.status),
        Some(WorktreeFormStatus::Creating),
        "the create did not start, so this test would prove nothing"
    );

    // Same budget as the idle case, so the two are the same question with one field changed.
    assert_eq!(
        frames_until_rest(&state, 600),
        None,
        "a create is in flight and the screen came to rest — the indeterminate indicator stopped \
         animating while the operation it reports on is still running, which also means the test \
         above passes for the wrong reason (FR-031f, FR-039d)"
    );
}
