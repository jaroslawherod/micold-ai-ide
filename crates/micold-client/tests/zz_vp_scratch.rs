//! Scratch: visual pass renders for feature 1272 M4. Not committed.
mod support;
use iced::advanced::renderer::{Headless as _, Style};
use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, Layout, Renderer as _};
use iced::{Rectangle, Size, Vector};
use micold_core::theme::ThemePreference;
use support::covered_states::{review_source, SourcePose};
use support::layout as lay;

fn shot(name: &str, under: lay::StateUnderTest, window: Size) {
    let mut renderer = lay::renderer();
    let mut element = lay::view_of(&under);
    let mut tree = Tree::new(element.as_widget());
    let limits = layout::Limits::new(Size::ZERO, window);
    let mut node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    lay::settle(&mut element, &mut tree, &node, &renderer, std::time::Instant::now(), 0..lay::SETTLE_FRAMES, window);
    node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    renderer.reset(Rectangle::with_size(window));
    let theme = iced::Theme::Light;
    element.as_widget().draw(&tree, &mut renderer, &theme, &Style::default(), Layout::new(&node), mouse::Cursor::Unavailable, &Rectangle::with_size(window));
    if let Some(mut overlay) = element.as_widget_mut().overlay(&mut tree, Layout::new(&node), &renderer, &Rectangle::with_size(window), Vector::ZERO) {
        let onode = overlay.as_overlay_mut().layout(&renderer, window);
        renderer.with_layer(Rectangle::with_size(window), |renderer| {
            overlay.as_overlay().draw(renderer, &theme, &Style::default(), Layout::new(&onode), mouse::Cursor::Unavailable);
        });
    }
    let (w, h) = (window.width as u32, window.height as u32);
    let px = renderer.screenshot(Size::new(w, h), 1.0, iced::Color::WHITE);
    let img = image::RgbaImage::from_raw(w, h, px).unwrap();
    img.save(format!("{}/{name}.png", std::env::var("VP_OUT").unwrap())).unwrap();
}

#[test]
fn render() {
    let full = Size::new(1280.0, 800.0);
    for (theme, t) in [(ThemePreference::Light, "light"), (ThemePreference::Dark, "dark")] {
        for (pose, open, n, window) in [
            (SourcePose::Listed, false, "1-closed", full),
            (SourcePose::Listed, true, "2-open", full),
            (SourcePose::PullRequest, false, "4-pr-shown", full),
            (SourcePose::PullRequest, true, "4-pr-open", full),
            (SourcePose::FilesOn, false, "6-files", full),
            (SourcePose::DefaultEntry, true, "8-default-open", full),
            (SourcePose::Unavailable, true, "9-unavailable-open", full),
            (SourcePose::PullRequest, false, "10-1600", Size::new(1600.0, 800.0)),
            (SourcePose::PullRequest, false, "10-900", Size::new(900.0, 800.0)),
        ] {
            let mut under = review_source(pose, open);
            under.state.settings.theme_pref = theme;
            under.state.sidebar.width = 300;
            shot(&format!("{n}-{t}"), under, window);
        }
        // Committed off, list open.
        let mut under = review_source(SourcePose::Listed, true);
        let _ = micold_client::features::changes::update(&mut under.state.changes, micold_client::features::changes::Msg::CommittedToggled);
        under.state.settings.theme_pref = theme;
        under.state.sidebar.width = 300;
        shot(&format!("3-committed-off-{t}"), under, full);
    }
}
