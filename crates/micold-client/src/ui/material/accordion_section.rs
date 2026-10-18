//! `AccordionSection` — a titled section that opens and closes (feature 804 M4, FR-013).
//!
//! A header the user presses, and the [`Accordion`](super::Accordion) panel it reveals. The header is
//! a full-width text [`Button`](super::Button), so the keyboard (Space and Enter with focus), the
//! focus ring, the ripple and both themes are the button's own rather than a second copy here. The
//! chevron says which way it goes next: down when closed, up when open.
//!
//! Closed, the header can also say what the section holds — a one-line [`summary`] — and that a
//! required field inside is still empty — the [`required`] marker (feature 890, FR-011, FR-012).
//! Neither changes how the header is pressed or focused, and a section without them is drawn as
//! before (FR-013).
//!
//! [`summary`]: AccordionSection::summary
//! [`required`]: AccordionSection::required
//!
//! Whether it is open is the caller's: this keeps no state, so a form can hold it as view state and
//! decide, for example, to show a section open while an error stands in it.

use super::{Accordion, Button, ButtonVariant, Ellipsized, Text, TypeRole};
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
    summary: Option<String>,
    required: bool,
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
            summary: None,
            required: false,
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

    /// A one-line summary of what the section holds, shown under the title while closed only, in
    /// the caption role and the muted tint, and cut with an ellipsis when the header is too narrow
    /// for it (feature 890, FR-011). Inside the header, so it adds no press target or focus stop
    /// (FR-013).
    pub fn summary(self, summary: impl Into<String>) -> Self {
        self.summary_maybe(Some(summary.into()))
    }

    /// The summary from an `Option` — none when `None`. Mirrors [`Button::on_press_maybe`], so a
    /// caller with an optional summary says so once.
    pub fn summary_maybe(mut self, summary: Option<String>) -> Self {
        self.summary = summary;
        self
    }

    /// Mark the section as holding a required field that is still empty: while closed, the title
    /// ends in the form fields' required marker, in the title's own role and tint (feature 890,
    /// FR-012). Open, the field shows its own marker, so the title does not.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }
}

impl<'a, M: Clone + 'a> From<AccordionSection<'a, M>> for Element<'a, M> {
    fn from(s: AccordionSection<'a, M>) -> Self {
        let chevron = if s.open { Icon::MoveUp } else { Icon::MoveDown };
        let chevron = super::glyph::icon(chevron, CHEVRON, s.roles.on_surface_variant);
        // The marker is a character of the title, as `form_field.rs` marks a field label (FR-012).
        let title = if s.required && !s.open {
            format!("{}{}", s.title, micold_core::required::MARKER)
        } else {
            s.title
        };
        let title = Text::new(title, TypeRole::Action, s.roles);
        let header: Element<'a, M> = match s.summary.filter(|text| !s.open && !text.is_empty()) {
            // The summary is a second line of the same button, so the header stays one press
            // target and one focus stop (FR-011, FR-013). The column fills the row, so the
            // summary is cut against everything the chevron leaves it.
            Some(summary) => row![
                column![
                    title,
                    Ellipsized::at_role(summary, TypeRole::Caption, s.roles.on_surface_variant),
                ]
                .width(Length::Fill),
                chevron,
            ]
            .align_y(alignment::Vertical::Center)
            .into(),
            None => row![title, Space::new().width(Length::Fill), chevron]
                .align_y(alignment::Vertical::Center)
                .into(),
        };
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
    use super::super::style;
    use super::super::test_support::{
        bounds_at, dark_roles, named_key, roles, size_of, Mounted, Painted, WINDOW,
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
            let at = m.origin
                + std::time::Duration::from_millis(micold_core::tokens::motion::duration::SHORT_1)
                    * m.frame;
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
        stops(AccordionSection::new("Network", body, r).open(open))
    }

    fn stops(section: AccordionSection<'_, String>) -> usize {
        let mut m = Mounted::new(section);
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

    /// The summary the M1 tests pose (FR-011).
    const SUMMARY: &str = "Docker · Basic image";

    /// The header's height before feature 890, measured on `origin/main` (FR-013: a section with
    /// neither a summary nor a marker looks as it does today).
    const TODAYS_HEADER_HEIGHT: f32 = 40.0;

    /// A header narrower than [`SUMMARY`] at the caption role.
    const NARROW: f32 = 120.0;

    fn painted_with(content: &str, painted: &[Painted]) -> Option<Painted> {
        painted.iter().find(|p| p.content == content).cloned()
    }

    /// Closed, the summary is painted under the title, one line, in the caption role and the muted
    /// tint — in both themes (FR-011, US3-1, T001 a, g).
    #[test]
    fn a_closed_section_paints_its_summary_under_the_title() {
        for r in [roles(), dark_roles()] {
            let mut m = Mounted::new(section(false, r).summary(SUMMARY));
            let header = m.node.children()[0].bounds();
            let painted = m.painted_text();
            let title = painted_with("Network", &painted).expect("the title is painted");
            let summary = painted_with(SUMMARY, &painted)
                .unwrap_or_else(|| panic!("a closed section paints its summary: {painted:?}"));
            assert_eq!(
                summary.size,
                TypeRole::Caption.size(),
                "the summary is a caption"
            );
            assert_eq!(
                summary.font,
                TypeRole::Caption.font(),
                "the summary is a caption"
            );
            assert_eq!(
                summary.color,
                style::color(r.on_surface_variant),
                "the summary is muted"
            );
            assert!(
                summary.bounds.y >= title.bounds.y + title.bounds.height - 0.5,
                "the summary sits under the title: {summary:?} vs {title:?}"
            );
            assert!(
                title.bounds.y >= header.y - 0.5
                    && summary.bounds.y + summary.bounds.height <= header.y + header.height + 0.5,
                "both lines fit inside the fixed-height header {header:?}: {title:?}, {summary:?}"
            );
        }
    }

    /// Open, the summary is not painted (FR-011, US3-1, T001 b).
    #[test]
    fn an_open_section_paints_no_summary() {
        let mut m = Mounted::new(section(true, roles()).summary(SUMMARY));
        let painted = m.painted_text();
        assert!(
            painted_with(SUMMARY, &painted).is_none(),
            "an open section hides its summary: {painted:?}"
        );
    }

    /// Too long for the header, the summary ends in `…` and the chevron stays inside the header
    /// (FR-011, T001 c).
    #[test]
    fn a_narrow_header_ellipsizes_the_summary_and_keeps_the_chevron_inside() {
        let r = roles();
        let mut m = Mounted::new(
            iced::widget::container(section(false, r).summary(SUMMARY))
                .width(Length::Fixed(NARROW)),
        );
        let header = m.node.children()[0].children()[0].bounds();
        let painted = m.painted_text();
        let summary = painted
            .iter()
            .find(|p| p.content.starts_with("Docker"))
            .unwrap_or_else(|| panic!("the summary is painted: {painted:?}"));
        assert!(
            summary.content.ends_with('…'),
            "a summary wider than its header ends in an ellipsis: {:?}",
            summary.content
        );
        for p in &painted {
            assert!(
                p.bounds.x + p.bounds.width <= header.x + header.width + 0.5,
                "{:?} runs past the header {header:?}",
                p
            );
        }
    }

    /// Closed and required, the title carries the form fields' marker in its own role and tint;
    /// open, it does not — in both themes (FR-012, US3-3, T001 d, g).
    #[test]
    fn a_closed_required_section_marks_its_title() {
        let marked = format!("Network{}", micold_core::required::MARKER);
        for r in [roles(), dark_roles()] {
            let plain = painted_with("Network", &Mounted::new(section(false, r)).painted_text())
                .expect("the plain title is painted");
            let closed = Mounted::new(section(false, r).required(true)).painted_text();
            let title = painted_with(&marked, &closed)
                .unwrap_or_else(|| panic!("the closed title carries the marker: {closed:?}"));
            assert_eq!(
                (title.size, title.font, title.color),
                (plain.size, plain.font, plain.color),
                "the marker is drawn in the title's role and tint"
            );
            let open = Mounted::new(section(true, r).required(true)).painted_text();
            assert!(
                painted_with("Network", &open).is_some() && painted_with(&marked, &open).is_none(),
                "an open section's title has no marker: {open:?}"
            );
        }
    }

    /// An empty summary is no summary: the header is drawn as without one (review A, M1).
    #[test]
    fn an_empty_summary_is_drawn_as_none() {
        let contents = |s: AccordionSection<'static, String>| -> Vec<String> {
            Mounted::new(s)
                .painted_text()
                .into_iter()
                .map(|p| p.content)
                .collect()
        };
        assert_eq!(
            contents(section(false, roles()).summary("")),
            contents(section(false, roles())),
        );
    }

    /// Without a summary or a marker the header is as tall as today; the marker alone does not
    /// change it either (FR-013, US3-2, T001 e).
    #[test]
    fn a_plain_header_is_as_tall_as_today() {
        for s in [
            section(false, roles()),
            section(false, roles()).required(true),
        ] {
            let m = Mounted::new(s);
            assert_eq!(m.node.children()[0].bounds().height, TODAYS_HEADER_HEIGHT);
        }
    }

    /// With both a summary and the marker, the header is still one press target and one focus
    /// stop: a press, Space and Enter each toggle once, and a closed body offers no stop (FR-013,
    /// SC-005, T001 f).
    #[test]
    fn a_summary_and_marker_leave_the_headers_behaviour_alone() {
        let r = roles();
        let mut m = Mounted::new(section(false, r).summary(SUMMARY).required(true));
        let at = header_centre(&m);
        assert_eq!(click(&mut m, at), vec!["toggled".to_string()]);
        for key in [Named::Space, Named::Enter] {
            let out = m.send(named_key(key), mouse::Cursor::Unavailable);
            assert_eq!(out, vec!["toggled".to_string()], "{key:?} must toggle once");
        }
        let body = crate::ui::material::TextField::<String>::new("", "v", r).label("Label");
        let decorated = AccordionSection::new("Network", body, r)
            .summary(SUMMARY)
            .required(true);
        assert_eq!(
            stops(decorated),
            focus_stops(false),
            "a closed section's body offers no focus stop, summary or not"
        );
    }
}
