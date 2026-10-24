//! `AppMark` — the application's icon at the title bar's leading end (feature 1127, FR-003,
//! contract C1).
//!
//! A picture with a name, not a control: it has no press and takes no keyboard focus, so a press on
//! it publishes nothing and Tab passes it by. Hovering it shows the name in a tooltip.
//!
//! The library holds no application asset (#1262's `no_domain_types` rule): the caller makes the
//! `image::Handle` **once** and passes it every frame, since a handle made per frame is a new image
//! to upload each time (the idle budget, `tests/idle_requests_no_frames.rs`).
//!
//! Builder form (Principle VIII): `AppMark::new(handle, "Micold", roles).into()`.

use iced::widget::image::{self, Handle};
use iced::Element;
use micold_core::tokens::{anatomy, Roles};

/// The application's mark: an [`anatomy::app_bar::MARK`] square naming the application on hover.
pub struct AppMark {
    handle: Handle,
    name: String,
    roles: Roles,
}

impl AppMark {
    /// The mark drawn from `handle`, named `name` in its tooltip, themed by `roles`.
    pub fn new(handle: Handle, name: impl Into<String>, roles: Roles) -> Self {
        Self {
            handle,
            name: name.into(),
            roles,
        }
    }
}

impl<'a, M: 'a> From<AppMark> for Element<'a, M> {
    fn from(m: AppMark) -> Self {
        let picture = image::Image::new(m.handle)
            .width(anatomy::app_bar::MARK)
            .height(anatomy::app_bar::MARK);
        super::Tooltip::new(picture, m.name, m.roles).into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{roles, size_of, Focusables, Mounted};
    use super::*;
    use iced::Size;

    const ROOM: Size = Size::new(200.0, 100.0);

    fn mark() -> AppMark {
        AppMark::new(
            Handle::from_rgba(1, 1, vec![255, 0, 0, 255]),
            "Micold",
            roles(),
        )
    }

    /// T006 (contract C1): an `app_bar::MARK` square.
    #[test]
    fn the_mark_is_a_mark_square() {
        assert_eq!(
            size_of::<String>(mark().into(), ROOM),
            Size::new(anatomy::app_bar::MARK, anatomy::app_bar::MARK)
        );
    }

    /// T006 (US1 s8): hovering it names the application.
    #[test]
    fn hovering_the_mark_names_the_application() {
        let mut m = Mounted::new(Element::<String>::from(mark()));
        let at = m.node.bounds().center();
        let tip = m.floated_text(at);
        assert!(tip.iter().any(|t| t == "Micold"), "{tip:?}");
    }

    /// T006 (FR-003): a press publishes nothing, and it is no Tab stop.
    #[test]
    fn the_mark_is_not_a_control() {
        let mut m = Mounted::new(Element::<String>::from(mark()));
        let at = m.node.bounds().center();
        assert!(m.press(at).is_empty(), "a press on the mark published");
        assert_eq!(m.drive(Focusables(0)), Some(0), "the mark is a Tab stop");
    }
}
