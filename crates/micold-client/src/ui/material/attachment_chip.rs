//! `AttachmentChip` — a picture attached to a draft: its preview, its file name and a remove
//! control (feature 941, T051, FR-027).
//!
//! A screenshot dropped on a structured session's pane waits in the composer as one of these until
//! the message is sent. The preview is decoded by the renderer from the bytes the window already
//! holds — the caller makes an `image::Handle` from them once, when the picture is attached, and
//! passes it every frame (a handle made per frame would be a new image to decode each time). The
//! name is cut with an ellipsis past [`NAME_MAX_WIDTH`], and the remove control says which file it
//! removes, on hover and as a keyboard stop of its own.
//!
//! Builder form: `AttachmentChip::new(name, handle, roles).on_remove(Msg::Remove(i)).into()`.

use iced::keyboard::key::Named;
use iced::widget::image::{self, Handle};
use iced::widget::{container, row};
use iced::{Alignment, ContentFit, Element, Length, Padding};
use micold_core::tokens::{shape, spacing, Roles};

use super::keyboard_focus::{Indicator, TakesTheKeyboard};
use super::{style, Ellipsized, IconButton, Tooltip, TypeRole};
use crate::icons::Icon;

/// The preview's square side.
pub const THUMBNAIL: f32 = 32.0;

/// The widest the file name is drawn before it is cut with an ellipsis.
pub const NAME_MAX_WIDTH: f32 = 180.0;

/// The words the remove control is labelled with, for `name`.
pub fn remove_label(name: &str) -> String {
    format!("Remove {name}")
}

/// One attached picture.
pub struct AttachmentChip<'a, M> {
    name: String,
    preview: Handle,
    roles: Roles,
    on_remove: Option<M>,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> AttachmentChip<'a, M> {
    /// A chip for the picture `name`, previewed from `preview`, themed by `roles`; without
    /// [`Self::on_remove`] it offers no remove control.
    pub fn new(name: impl Into<String>, preview: Handle, roles: Roles) -> Self {
        Self {
            name: name.into(),
            preview,
            roles,
            on_remove: None,
            _marker: std::marker::PhantomData,
        }
    }

    /// The message the remove control sends.
    pub fn on_remove(mut self, message: M) -> Self {
        self.on_remove = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<AttachmentChip<'a, M>> for Element<'a, M> {
    fn from(c: AttachmentChip<'a, M>) -> Self {
        let r = c.roles;
        let thumbnail = image::Image::new(c.preview)
            .width(THUMBNAIL)
            .height(THUMBNAIL)
            .content_fit(ContentFit::Cover)
            .border_radius(shape::EXTRA_SMALL);
        let name = container(
            Ellipsized::<M>::at_role(c.name.clone(), TypeRole::Label, r.on_surface).shrink(),
        )
        .max_width(NAME_MAX_WIDTH);
        let mut parts = row![thumbnail, name]
            .spacing(spacing::SM)
            .align_y(Alignment::Center);
        if let Some(message) = c.on_remove {
            let button = IconButton::new(Icon::Close, r)
                .compact()
                .size(TypeRole::Body)
                .tint(r.on_surface_variant)
                .on_press(message.clone());
            let focus = TakesTheKeyboard::new(button, true)
                .key(Named::Enter, message.clone())
                .key(Named::Space, message)
                .indicator(Indicator {
                    outline: r.secondary,
                    layer: r.on_surface_variant,
                    radius: shape::FULL,
                });
            parts = parts.push(Tooltip::new(focus, remove_label(&c.name), r));
        }
        container(parts)
            .padding(Padding {
                top: spacing::XS,
                bottom: spacing::XS,
                left: spacing::XS,
                right: spacing::SM,
            })
            .width(Length::Shrink)
            .style(style::attachment_chip(r))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, named_key, roles, Focusables, Mounted};
    use super::*;
    use iced::advanced::mouse;
    use iced::Point;

    /// A one-pixel PNG.
    fn handle() -> Handle {
        Handle::from_rgba(1, 1, vec![255, 0, 0, 255])
    }

    fn chip(name: &str, r: Roles) -> AttachmentChip<'static, String> {
        AttachmentChip::new(name, handle(), r).on_remove("remove".to_string())
    }

    /// FR-027: the chip draws its file name, in both schemes.
    #[test]
    fn the_name_is_drawn() {
        for r in [roles(), dark_roles()] {
            let painted = Mounted::new(chip("screenshot.png", r)).painted_text();
            assert!(
                painted.iter().any(|p| p.content == "screenshot.png"),
                "{painted:?}"
            );
        }
    }

    /// A long name is cut with an ellipsis rather than widening the chip.
    #[test]
    fn a_long_name_is_cut() {
        let long = "a-very-long-screenshot-name-from-the-capture-tool-2026-10-22.png";
        let painted = Mounted::new(chip(long, roles())).painted_text();
        let name = painted
            .iter()
            .find(|p| p.content.starts_with("a-very"))
            .unwrap_or_else(|| panic!("{painted:?}"));
        assert!(name.content.ends_with('…'), "{:?}", name.content);
    }

    /// The remove control is one keyboard stop, answers Enter and the pointer, and is labelled
    /// with the file it removes.
    #[test]
    fn the_remove_control_is_a_labelled_keyboard_stop() {
        let mut m = Mounted::new(chip("shot.png", roles()));
        assert_eq!(m.drive(Focusables(0)), Some(1));
        m.focus_next();
        let out = m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
        assert_eq!(out, vec!["remove".to_string()]);

        let mut m = Mounted::new(chip("shot.png", roles()));
        let glyph = m
            .painted_text()
            .into_iter()
            .find(|p| p.content == Icon::Close.glyph().to_string())
            .expect("the remove glyph is drawn")
            .bounds;
        let at = Point::new(glyph.x + glyph.width / 2.0, glyph.y + glyph.height / 2.0);
        let tip = m.floated_text(at);
        assert!(tip.contains(&remove_label("shot.png")), "{tip:?}");
    }

    /// Without `on_remove` there is no control, and no keyboard stop.
    #[test]
    fn without_on_remove_there_is_no_control() {
        let mut m = Mounted::new(AttachmentChip::<String>::new("shot.png", handle(), roles()));
        assert_eq!(m.drive(Focusables(0)), Some(0));
    }
}
