//! `drop_cue` — the cue that a region will take the files being dragged over the window
//! (#1464, FR-005).
//!
//! The content is laid over by an outline and a faint tint of the primary accent, so the region
//! reads as "drop here" without hiding what is in it. The overlay is drawn, not laid out: it takes
//! the content's own box and no pointer event, so the content below keeps working and a gate that
//! reads the layout sees the same rectangle with or without the cue. Where the cue shows is the
//! caller's decision; this only draws it.
//!
//! Form: `drop_cue(content, roles)`.

use iced::widget::{container, stack, Space};
use iced::{Background, Border, Element, Length};
use micold_core::tokens::{shape, Roles};

/// How thick the outline is, in dp.
pub const OUTLINE: f32 = 2.0;

/// How strongly the accent tints the content, 0 to 1: enough to see on a dark and a light
/// surface, little enough to read the transcript through it.
pub const TINT: f32 = 0.12;

/// `content` with the drop-target cue over it, the size `content` has without it.
pub fn drop_cue<'a, M: 'a>(content: impl Into<Element<'a, M>>, roles: Roles) -> Element<'a, M> {
    let accent = crate::style::color(roles.primary);
    let overlay = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &iced::Theme| container::Style {
            background: Some(Background::Color(iced::Color { a: TINT, ..accent })),
            border: Border {
                color: accent,
                width: OUTLINE,
                radius: shape::SMALL.into(),
            },
            ..container::Style::default()
        });
    stack![content.into(), overlay].into()
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles as roles, size_of};
    use super::*;
    use iced::Size;

    /// The cue is drawn over the content, not around it: wrapping a child leaves its size alone.
    #[test]
    fn wrapping_a_child_leaves_its_size_unchanged() {
        let room = Size::new(400.0, 300.0);
        let child = || {
            container(Space::new())
                .width(Length::Fixed(120.0))
                .height(Length::Fixed(80.0))
        };
        let bare = size_of::<()>(child().into(), room);
        let cued = size_of::<()>(drop_cue(child(), roles()), room);
        assert_eq!(bare, cued);
        let fill = size_of::<()>(
            drop_cue(
                container(Space::new())
                    .width(Length::Fill)
                    .height(Length::Fill),
                roles(),
            ),
            room,
        );
        assert_eq!(fill, room);
    }

    /// The outline and the tint are the accent, so the cue is seen against the surface.
    #[test]
    fn the_accent_differs_from_the_surface_it_tints() {
        let r = roles();
        assert_ne!(r.primary, r.surface);
        assert!(TINT > 0.0 && TINT < 1.0);
    }
}
