//! `AccordionSection` — a titled section that opens and closes (feature 804 M4, FR-013).
//!
//! A header the user presses, and the [`Accordion`](super::Accordion) panel it reveals. The header is
//! a full-width text [`Button`](super::Button), so the keyboard (Space and Enter with focus), the
//! focus ring, the ripple and both themes are the button's own rather than a second copy here. The
//! chevron says which way it goes next: down when closed, up when open.
//!
//! Whether it is open is the caller's: this keeps no state, so a form can hold it as view state and
//! decide, for example, to show a section open while an error stands in it.

use super::{Accordion, Button, ButtonVariant, Text, TypeRole};
use crate::icons::Icon;
use iced::widget::{column, row, Space};
use iced::{alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};

/// The chevron's size: the same as a select's trailing glyph.
const CHEVRON: f32 = 24.0;

/// A titled, collapsible section. Builder form (Principle VIII):
/// `AccordionSection::new("Network", body, roles).open(flag).on_toggle(msg).into()`.
pub struct AccordionSection<'a, M> {
    title: String,
    body: Element<'a, M>,
    roles: Roles,
    open: bool,
    on_toggle: Option<M>,
}

impl<'a, M: Clone + 'a> AccordionSection<'a, M> {
    /// A closed section called `title` holding `body`.
    pub fn new(title: impl Into<String>, body: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            roles,
            open: false,
            on_toggle: None,
        }
    }

    /// Show the section open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// What pressing the header emits. Without it the header is inert.
    pub fn on_toggle(mut self, message: M) -> Self {
        self.on_toggle = Some(message);
        self
    }
}

impl<'a, M: Clone + 'a> From<AccordionSection<'a, M>> for Element<'a, M> {
    fn from(s: AccordionSection<'a, M>) -> Self {
        let chevron = if s.open { Icon::MoveUp } else { Icon::MoveDown };
        let header = row![
            Text::new(s.title, TypeRole::Action, s.roles),
            Space::new().width(Length::Fill),
            super::glyph::icon(chevron, CHEVRON, s.roles.on_surface_variant),
        ]
        .align_y(alignment::Vertical::Center);
        column![
            Button::with_content(header, ButtonVariant::Text, s.roles)
                .width(Length::Fill)
                .on_press_maybe(s.on_toggle),
            Accordion::new(s.body, s.roles).open(s.open),
        ]
        .spacing(spacing::XS)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, named_key, roles, size_of, Mounted, WINDOW};
    use super::*;
    use iced::advanced::mouse;
    use iced::keyboard::key::Named;
    use iced::{Event, Point};

    fn section<'a>(open: bool, r: Roles) -> AccordionSection<'a, String> {
        AccordionSection::new("Network", Text::new("body text", TypeRole::Body, r), r)
            .open(open)
            .on_toggle("toggled".to_string())
    }

    fn header_centre(m: &Mounted<'_>) -> Point {
        let header = m.node.children()[0].bounds();
        Point::new(
            header.x + header.width / 2.0,
            header.y + header.height / 2.0,
        )
    }

    fn click(m: &mut Mounted<'_>, at: Point) -> Vec<String> {
        let mut out = m.press(at);
        out.extend(m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        ));
        out
    }

    /// The body takes room only while open, in both themes; the header is there either way.
    #[test]
    fn the_body_is_revealed_only_when_open() {
        for r in [roles(), dark_roles()] {
            let closed = size_of(section(false, r).into(), WINDOW);
            let open = size_of(section(true, r).into(), WINDOW);
            assert!(closed.height > 0.0, "the header must show while closed");
            assert!(
                open.height > closed.height,
                "open {open:?} must be taller than closed {closed:?}"
            );
        }
    }

    /// A press on the header, and Space or Enter once it holds the keyboard, each emit the toggle.
    #[test]
    fn the_header_toggles_by_pointer_and_by_keyboard() {
        let mut m = Mounted::new(section(false, roles()));
        let at = header_centre(&m);
        assert_eq!(click(&mut m, at), vec!["toggled".to_string()]);
        for key in [Named::Space, Named::Enter] {
            let out = m.send(named_key(key), mouse::Cursor::Unavailable);
            assert_eq!(out, vec!["toggled".to_string()], "{key:?} must toggle");
        }
    }

    /// Without a message the header is inert, like every other control here.
    #[test]
    fn a_section_without_a_toggle_does_nothing() {
        let r = roles();
        let mut m = Mounted::new(AccordionSection::new(
            "Network",
            Text::new("body", TypeRole::Body, r),
            r,
        ));
        let at = header_centre(&m);
        assert!(click(&mut m, at).is_empty());
    }
}
