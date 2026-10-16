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
            Accordion::new(s.body, s.roles)
                .width(Length::Fill)
                .open(s.open),
        ]
        .spacing(spacing::XS)
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        bounds_at, dark_roles, named_key, roles, size_of, Mounted, WINDOW,
    };
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

    /// An open section is still open after the frames a screen would run: it starts settled.
    #[test]
    fn an_open_section_stays_open_through_the_first_frames() {
        let r = roles();
        let mut m = Mounted::new(section(true, r));
        let before = m.node.size().height;
        for _ in 0..40 {
            m.frame += 1;
            let at = m.origin + std::time::Duration::from_millis(16) * m.frame;
            m.send(
                Event::Window(iced::window::Event::RedrawRequested(at)),
                mouse::Cursor::Unavailable,
            );
            m.relayout();
        }
        assert_eq!(
            m.node.size().height,
            before,
            "an open section must not collapse"
        );
    }

    /// A body that fills its width gets the section's width, not the padding alone.
    #[test]
    fn a_body_that_fills_gets_the_sections_width() {
        let r = roles();
        let body = crate::ui::material::TextField::<String>::new("", "value", r).label("Label");
        let size = size_of(
            AccordionSection::new("Network", body, r).open(true).into(),
            WINDOW,
        );
        let panel = bounds_at(
            AccordionSection::new(
                "Network",
                crate::ui::material::TextField::<String>::new("", "value", r).label("Label"),
                r,
            )
            .open(true)
            .into(),
            WINDOW,
            &[1],
        );
        assert_eq!(size.width, WINDOW.width);
        assert_eq!(panel.width, WINDOW.width, "the panel must span the section");
    }

    /// Counts what a focus traversal would be offered.
    struct Count(usize);
    impl iced::advanced::widget::Operation for Count {
        fn traverse(
            &mut self,
            operate: &mut dyn FnMut(&mut dyn iced::advanced::widget::Operation),
        ) {
            operate(self);
        }
        fn focusable(
            &mut self,
            _id: Option<&iced::advanced::widget::Id>,
            _bounds: iced::Rectangle,
            _state: &mut dyn iced::advanced::widget::operation::Focusable,
        ) {
            self.0 += 1;
        }
    }

    fn focus_stops(open: bool) -> usize {
        let r = roles();
        let body = crate::ui::material::TextField::<String>::new("", "v", r).label("Label");
        let mut m = Mounted::new(AccordionSection::new("Network", body, r).open(open));
        let mut count = Count(0);
        m.element.as_widget_mut().operate(
            &mut m.tree,
            iced::advanced::Layout::new(&m.node),
            &m.renderer,
            &mut count,
        );
        count.0
    }

    /// Tab must not walk into the fields of a closed section.
    #[test]
    fn a_closed_section_offers_no_focus_stops_in_its_body() {
        assert!(
            focus_stops(true) > focus_stops(false),
            "open {} vs closed {}",
            focus_stops(true),
            focus_stops(false)
        );
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
