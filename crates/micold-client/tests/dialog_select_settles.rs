//! The select's list must stop animating once it has arrived (feature 022, BUG-001).
//!
//! The blink is a *reset*, and the thing that resets it is the one thing a widget test forgets to
//! do: **re-run `view()` every frame**. A real iced application rebuilds its whole element tree on
//! each redraw and diffs it against the persistent [`Tree`]; a test that builds one element and
//! pumps events at it never exercises `Widget::diff` at all. `Select` assembles its only child in
//! `layout`, and iced 0.14's *default* `diff` is `tree.children.clear()` — so every rebuild threw
//! that child's state away and the list's entrance started over from zero.
//!
//! That is why the earlier version of this file, which reused a single element, reported a serene
//! settle while the real client blinked for six seconds straight. It is also why the loop is
//! self-sustaining: the reset makes the select ask for another frame, the frame re-runs `view()`,
//! and the rebuild resets it again.
//!
//! The two frame budgets below are deliberately far larger than the ~150 ms the transitions need.
//! An earlier gate for this bug asserted at 30 frames and failed for want of frames rather than for
//! want of a fix, which is indistinguishable from the real thing until you check.
#[path = "support/mod.rs"]
mod support;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{Event, Rectangle, Size, Vector};
use support::covered_states::covered_states;
use support::layout::{walk, Layer, FRAME, WINDOW};

#[test]
fn the_dialogs_select_settles_across_view_rebuilds() {
    let cs = covered_states()
        .iter()
        .find(|c| c.name == "add-worktree-dialog-type-menu-open")
        .expect("covered state exists");
    let under = (cs.build)();
    let path = under
        .press_at
        .expect("this covered state presses something");

    let renderer = support::layout::renderer();
    let build = || {
        micold_client::ui::view(
            &under.state,
            None,
            None,
            0,
            None,
            &micold_core::env_include::EnvIncludeOutcome::Disabled,
            &under.connection,
        )
    };
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let mut element = build();
    let mut tree = Tree::new(element.as_widget());
    let mut node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let origin = std::time::Instant::now();
    let mut clock = 0u32;

    /// One application frame: deliver the event, then rebuild the view and diff it in, exactly as
    /// the runtime does between redraws.
    macro_rules! pump {
        ($ev:expr, $cur:expr) => {{
            let mut msgs = Vec::new();
            let mut shell = Shell::new(&mut msgs);
            element.as_widget_mut().update(
                &mut tree,
                &$ev,
                Layout::new(&node),
                $cur,
                &renderer,
                &mut clipboard::Null,
                &mut shell,
                &Rectangle::with_size(WINDOW),
            );
            if let Some(mut ov) = element.as_widget_mut().overlay(
                &mut tree,
                Layout::new(&node),
                &renderer,
                &Rectangle::with_size(WINDOW),
                Vector::ZERO,
            ) {
                let n = ov.as_overlay_mut().layout(&renderer, WINDOW);
                ov.as_overlay_mut().update(
                    &$ev,
                    Layout::new(&n),
                    mouse::Cursor::Unavailable,
                    &renderer,
                    &mut clipboard::Null,
                    &mut shell,
                );
            }
            element = build();
            tree.diff(element.as_widget());
            node = element
                .as_widget_mut()
                .layout(&mut tree, &renderer, &limits);
            shell.redraw_request()
        }};
    }

    macro_rules! settle {
        ($frames:expr) => {
            for _ in 0..$frames {
                clock += 1;
                let _ = pump!(
                    Event::Window(iced::window::Event::RedrawRequested(origin + FRAME * clock)),
                    mouse::Cursor::Unavailable
                );
            }
        };
    }

    // 1. let the dialog's own entrance finish — a modal swallows every event that is not a redraw
    //    until it has appeared, so a press into a freshly built tree would reach nothing.
    settle!(40);

    // 2. press the select, at the node path the covered state names
    let target = walk(Layout::new(&node), Layer::Base)
        .into_iter()
        .find(|r| r.path == path)
        .expect("the select's node is where the covered state says");
    let at = iced::Point::new(
        target.x + target.width / 2.0,
        target.y + target.height / 2.0,
    );
    let _ = pump!(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(at)
    );

    // 3. let the list arrive
    settle!(40);

    // Without this the whole measurement is about nothing: an earlier bisect reported a confident
    // zero from two arrangements where the press had simply missed.
    let opened = element
        .as_widget_mut()
        .overlay(
            &mut tree,
            Layout::new(&node),
            &renderer,
            &Rectangle::with_size(WINDOW),
            Vector::ZERO,
        )
        .is_some();
    assert!(
        opened,
        "the list never opened — this measurement would be about nothing"
    );

    // 4. idle, and count
    let mut asks = 0;
    for _ in 0..60 {
        clock += 1;
        if pump!(
            Event::Window(iced::window::Event::RedrawRequested(origin + FRAME * clock)),
            mouse::Cursor::Unavailable
        ) != iced::window::RedrawRequest::Wait
        {
            asks += 1;
        }
    }
    assert_eq!(
        asks, 0,
        "an open select that has arrived asked for {asks} more frames over 60 idle ones — \
         its transition is being restarted by the view rebuild (BUG-001)"
    );
}
