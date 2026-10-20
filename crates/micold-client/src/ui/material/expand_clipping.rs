//! An `expand` section stays inside the scrollable that holds it, checked in pixels (feature 1092).
//!
//! A pushed clip replaces the enclosing one, so a section that clipped to its own bounds painted its
//! whole body over whatever lay beyond the edge of the list it was scrolled in. In the daemon form
//! dialog that was the Save and Cancel row. Geometry cannot see it: every node is where it should
//! be, and only the pixels show the overdraw.

use iced::advanced::renderer::Headless as _;
use iced::advanced::widget::Tree;
use iced::advanced::{layout, mouse, renderer::Style, Layout, Renderer as _};
use iced::widget::{column, Space};
use iced::{Color, Element, Length, Size};

use super::test_support::{renderer, roles};
use super::{expand, Scrollable, Surface, SurfaceKind};
use crate::showcase::state::Message;

const WIDTH: u32 = 200;
const HEIGHT: u32 = 300;
/// The scrollable's height; everything below it must stay the backdrop.
const VIEW: f32 = 100.0;
/// The space above the section, so it starts inside the view and runs well past its bottom edge.
const ABOVE: f32 = 80.0;

fn pixel(pixels: &[u8], y: u32) -> [u8; 3] {
    let i = ((y * WIDTH + WIDTH / 2) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2]]
}

/// An opaque section 200dp tall in a 100dp scrollable, at its top: drawn into `WIDTH` x `HEIGHT`.
fn scrolled_section() -> Vec<u8> {
    let r = roles();
    let body: Element<'_, Message> = Surface::new(
        Space::new().height(Length::Fixed(200.0)),
        SurfaceKind::Chip(r.primary),
        r,
    )
    .width(Length::Fill)
    .into();
    let list = column![
        Space::new().height(Length::Fixed(ABOVE)),
        expand(body, true, std::time::Duration::ZERO)
    ];
    let mut element: Element<'_, Message> =
        Scrollable::new(list, r).height(Length::Fixed(VIEW)).into();
    let mut renderer = renderer();
    let mut tree = Tree::new(&element);
    let size = Size::new(WIDTH as f32, HEIGHT as f32);
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, size),
    );
    let viewport = iced::Rectangle::with_size(size);
    renderer.reset(viewport);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        &iced::Theme::Dark,
        &Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    renderer.screenshot(Size::new(WIDTH, HEIGHT), 1.0, Color::BLACK)
}

/// The section paints inside the view, and nothing of it paints below the view's edge.
#[test]
fn an_expanded_section_does_not_paint_past_the_scrollable_edge() {
    let pixels = scrolled_section();
    assert_ne!(
        pixel(&pixels, 90),
        [0, 0, 0],
        "the section did not paint inside the view, so the check below proves nothing"
    );
    for y in (VIEW as u32 + 2)..HEIGHT {
        assert_eq!(
            pixel(&pixels, y),
            [0, 0, 0],
            "the section painted at y={y}, below the {VIEW}dp scrollable"
        );
    }
}
