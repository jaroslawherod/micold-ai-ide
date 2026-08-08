//! The select's list never stops animating inside the dialog (feature 022, BUG-001).
//!
//! **Ignored because it fails.** Un-ignoring it is how the fix proves itself.
//!
//! This is the reproduction the bug report asked for, and it is fast: 0.07s, headless, no GUI. It
//! took four wrong turns to get here and the shape of it is the lesson.
//!
//! - The same select **bare** settles. Inside a plain `stack`, it settles. Only in the real dialog
//!   does it keep going — 21 of 60 idle frames ask for another one.
//! - Synthetic nesting did not reproduce it. A hand-built `Surface` and `Modal` both reported a
//!   serene zero, and both were **lying**: the press never landed, so nothing was measured. That is
//!   why the `opened` assertion below is not decoration — without it this file would have reported
//!   the opposite conclusion.
//! - What works is the covered state the layout fixture already has. It builds the real
//!   application view, presses the select at a node path that is checked against the fixture, and
//!   settles the dialog's entrance first — a modal swallows every event that is not a redraw until
//!   it has appeared, which is why a press into a freshly built tree reaches nothing.
//!
//! Instrumentation showed *what* is wrong: the transition's `Progress` is **recreated** rather than
//! restarted — `from` is always 0.0 and the target never flips, so nothing is retargeting it. Its
//! widget-tree state is not surviving the frame here, and does survive one level shallower.
#[path = "support/mod.rs"]
mod support;

use iced::advanced::widget::Tree;
use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
use iced::{Event, Rectangle, Size, Vector};
use support::covered_states::covered_states;
use support::layout::{walk, Layer, FRAME, WINDOW};

#[test]
#[ignore = "BUG-001: reproduces the blink; un-ignore it with the fix"]
fn does_the_dialog_select_settle() {
    let cs = covered_states()
        .iter()
        .find(|c| c.name == "add-worktree-dialog-type-menu-open")
        .expect("covered state exists");
    let under = (cs.build)();
    let path = under
        .press_at
        .expect("this covered state presses something");

    let renderer = support::layout::renderer();
    let mut element = micold_client::ui::view(
        &under.state,
        None,
        None,
        0,
        None,
        &micold_core::env_include::EnvIncludeOutcome::Disabled,
        &under.connection,
    );
    let limits = layout::Limits::new(Size::ZERO, WINDOW);
    let mut tree = Tree::new(element.as_widget());
    let mut node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let origin = std::time::Instant::now();
    let mut clock = 0u32;

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
            node = element
                .as_widget_mut()
                .layout(&mut tree, &renderer, &limits);
            shell.redraw_request()
        }};
    }

    // 1. settle the dialog's entrance
    for _ in 0..10 {
        clock += 1;
        pump!(
            Event::Window(iced::window::Event::RedrawRequested(origin + FRAME * clock)),
            mouse::Cursor::Unavailable
        );
    }

    // 2. press the select, at the node path the covered state names
    let target = walk(Layout::new(&node), Layer::Base)
        .into_iter()
        .find(|r| r.path == path)
        .expect("the select's node is where the covered state says");
    let at = iced::Point::new(
        target.x + target.width / 2.0,
        target.y + target.height / 2.0,
    );
    pump!(
        Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
        mouse::Cursor::Available(at)
    );

    // 3. settle the list's arrival
    for _ in 0..10 {
        clock += 1;
        pump!(
            Event::Window(iced::window::Event::RedrawRequested(origin + FRAME * clock)),
            mouse::Cursor::Unavailable
        );
    }

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
    println!("DIALOG SELECT: asked on {asks} of 60 idle frames (opened={opened})");
    assert_eq!(
        asks, 0,
        "the dialog's select asked on {asks} of 60 idle frames"
    );
}
