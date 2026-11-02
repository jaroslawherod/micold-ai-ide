//! `drop_cue` — the cue that a region will take the files being dragged over the window
//! (#1464, FR-005).
//!
//! The content is laid over by an outline and a faint tint of the primary accent, so the region
//! reads as "drop here" without hiding what is in it. The overlay is drawn, not laid out: it takes
//! the content's own box and no pointer event, so the content below keeps working and a gate that
//! reads the layout sees the same rectangle with or without the cue. Where the cue shows is the
//! caller's decision; this only draws it.
//!
//! Form: `drop_cue(content, roles, active)`.

use iced::widget::{container, stack, Space};
use iced::{Background, Border, Element, Length};
use micold_core::tokens::{shape, Roles};

/// How thick the outline is, in dp.
const OUTLINE: f32 = 2.0;

/// How strongly the accent tints the content, 0 to 1: enough to see on a dark and a light
/// surface, little enough to read the transcript through it.
const TINT: f32 = 0.12;

/// The overlay's look: the accent outline and tint while `active`, nothing while not.
fn overlay_style(roles: Roles, active: bool) -> container::Style {
    if !active {
        return container::Style::default();
    }
    let accent = crate::style::color(roles.primary);
    container::Style {
        background: Some(Background::Color(iced::Color { a: TINT, ..accent })),
        border: Border {
            color: accent,
            width: OUTLINE,
            radius: shape::SMALL.into(),
        },
        ..container::Style::default()
    }
}

/// `content` with the drop-target cue over it while `active`, the size `content` has without it.
///
/// The overlay is in the tree whether or not the cue shows, so the content's widget state (a
/// scroll offset, a focused editor) survives the cue appearing and going.
pub fn drop_cue<'a, M: 'a>(
    content: impl Into<Element<'a, M>>,
    roles: Roles,
    active: bool,
) -> Element<'a, M> {
    let overlay = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_: &iced::Theme| overlay_style(roles, active));
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
        for active in [false, true] {
            assert_eq!(
                bare,
                size_of::<()>(drop_cue(child(), roles(), active), room)
            );
        }
        let fill = || {
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
        };
        assert_eq!(size_of::<()>(drop_cue(fill(), roles(), true), room), room);
    }

    /// The cue is the accent as outline and as a faint tint; off, it draws nothing.
    #[test]
    fn active_it_outlines_and_tints_in_the_accent_and_idle_it_draws_nothing() {
        let r = roles();
        let accent = crate::style::color(r.primary);
        let on = overlay_style(r, true);
        assert_eq!(on.border.color, accent);
        assert_eq!(on.border.width, OUTLINE);
        assert_eq!(
            on.background,
            Some(Background::Color(iced::Color { a: TINT, ..accent }))
        );
        let off = overlay_style(r, false);
        assert_eq!(off.background, None);
        assert_eq!(off.border.width, 0.0);
    }

    /// Same widget tree on and off, so the content below keeps its state when the cue comes.
    #[test]
    fn the_widget_tree_has_the_same_shape_with_the_cue_on_and_off() {
        use iced::advanced::widget::Tree;
        let shape = |active| {
            let element: Element<'_, ()> = drop_cue(Space::new(), roles(), active);
            let tree = Tree::new(element.as_widget());
            (tree.tag, tree.children.len())
        };
        assert_eq!(shape(false), shape(true));
    }
}
