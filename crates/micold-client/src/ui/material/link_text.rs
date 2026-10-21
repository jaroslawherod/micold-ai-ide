//! `LinkText` — a run of formatted text whose links open (feature 940, research R9, R11; FR-004).
//!
//! The text is one iced `rich_text`, so it wraps as a paragraph does; each markdown span keeps its
//! weight, slant, strike and inline code (monospace on the code fill). A link the model marks
//! activatable — a web or mail address — is drawn in `primary`, underlined, and opens on a press
//! (`on_link_click`). It is also one keyboard focus stop of its own: a [`TakesTheKeyboard`] per link
//! around the paragraph, the first link outermost so Tab visits them in reading order, each
//! answering Enter and Space with that link's message. A press on the text never takes the keyboard
//! (`press_takes_focus(false)`): it is the span that was pressed that opens, not a focused one.
//!
//! A link that is not activatable (a relative path, a file address) is drawn underlined in the text
//! colour, with no link: it offers no stop and sends nothing (FR-004).
//!
//! Everything shown is owned, so the element is `'static` and can sit inside iced `lazy`.

use std::rc::Rc;

use iced::font::{Style as Slant, Weight};
use iced::keyboard::key::Named;
use iced::widget::text::{Span as TextSpan, Wrapping};
use iced::widget::{rich_text, span};
use iced::{Element, Font, Length};
use micold_core::markdown::Span;
use micold_core::tokens::{state, Rgb, Roles};

use super::keyboard_focus::{Indicator, TakesTheKeyboard};
use super::{style, TypeRole};

/// What a link press sends, from the link's address.
type OnLink<M> = Rc<dyn Fn(String) -> M>;

/// A run of formatted text. Builder form (Principle VIII):
/// `LinkText::new(&spans, roles).role(TypeRole::Body).muted(false).strong(false).on_link(f).into()`.
pub struct LinkText<'a, M> {
    spans: &'a [Span],
    roles: Roles,
    role: TypeRole,
    muted: bool,
    strong: bool,
    on_link: Option<OnLink<M>>,
}

impl<'a, M: Clone + 'static> LinkText<'a, M> {
    /// `spans` as body text in `on_surface`, links drawn but inert until [`Self::on_link`].
    pub fn new(spans: &'a [Span], roles: Roles) -> Self {
        Self {
            spans,
            roles,
            role: TypeRole::Body,
            muted: false,
            strong: false,
            on_link: None,
        }
    }

    /// The type role: a heading's, or body text by default.
    pub fn role(mut self, role: TypeRole) -> Self {
        self.role = role;
        self
    }

    /// Draw the text in `on_surface_variant`: a thinking card's muted style (FR-002).
    pub fn muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// Draw every span at the bold weight: a table's header row.
    pub fn strong(mut self, strong: bool) -> Self {
        self.strong = strong;
        self
    }

    /// What pressing, or Enter on, an activatable link sends, from its address.
    pub fn on_link(mut self, f: impl Fn(String) -> M + 'static) -> Self {
        self.on_link = Some(Rc::new(f));
        self
    }
}

/// The colour of plain text: `on_surface`, or `on_surface_variant` when muted.
pub(super) fn ink(r: Roles, muted: bool) -> Rgb {
    if muted {
        r.on_surface_variant
    } else {
        r.on_surface
    }
}

/// The colour of an activatable link.
pub(super) fn link_ink(r: Roles) -> Rgb {
    r.primary
}

/// One markdown span as an iced span: its font, colour, underline, strike and code fill, and its
/// address when it opens.
fn painted(
    s: &Span,
    base: Font,
    r: Roles,
    muted: bool,
    strong: bool,
    opens: bool,
) -> TextSpan<'static, String, Font> {
    let mut font = if s.style.code { Font::MONOSPACE } else { base };
    if s.style.bold || strong {
        font.weight = Weight::Bold;
    }
    if s.style.italic {
        font.style = Slant::Italic;
    }
    let link = s.link.as_ref();
    let colour = match link {
        Some(link) if link.activatable => link_ink(r),
        _ => ink(r, muted),
    };
    let mut out = span(s.text.clone())
        .font(font)
        .color(style::color(colour))
        .underline(link.is_some())
        .strikethrough(s.style.strike);
    if s.style.code {
        out = out
            .background(style::color(style::code_fill(r)))
            .padding([0.0, 2.0]);
    }
    match link {
        Some(link) if link.activatable && opens => out.link(link.target.clone()),
        _ => out,
    }
}

impl<'a, 'b, M: Clone + 'static> From<LinkText<'a, M>> for Element<'b, M> {
    fn from(t: LinkText<'a, M>) -> Self {
        let r = t.roles;
        let opens = t.on_link.is_some();
        let spans: Vec<TextSpan<'static, String, Font>> = t
            .spans
            .iter()
            .map(|s| painted(s, t.role.font(), r, t.muted, t.strong, opens))
            .collect();
        let mut text = rich_text(spans)
            .size(t.role.size())
            .line_height(t.role.line_height())
            .font(t.role.font())
            .wrapping(Wrapping::WordOrGlyph)
            .width(Length::Fill);
        let Some(on_link) = t.on_link else {
            let text: Element<'static, M> = text.into();
            return text;
        };
        let press = Rc::clone(&on_link);
        text = text.on_link_click(move |url: String| press(url));
        let mut element: Element<'static, M> = text.into();
        // Innermost the last link, so the outermost stop, which traversal visits first, is the
        // first link. A link whose text changes style is several spans in a row with the same
        // link: one stop.
        let mut targets: Vec<&str> = Vec::new();
        let mut previous = None;
        for s in t.spans {
            let link = s.link.as_ref().filter(|l| l.activatable);
            if let Some(l) = link.filter(|_| previous != Some(link)) {
                targets.push(l.target.as_str());
            }
            previous = Some(link);
        }
        for target in targets.into_iter().rev() {
            let message = on_link(target.to_owned());
            element = TakesTheKeyboard::new(element, true)
                .key(Named::Enter, message.clone())
                .key(Named::Space, message)
                .press_takes_focus(false)
                .bare_child()
                .indicator(Indicator {
                    outline: r.secondary,
                    layer: link_ink(r),
                    radius: state::FOCUS_RING_WIDTH,
                })
                .into();
        }
        element
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{named_key, roles, Focusables, Mounted};
    use super::*;
    use iced::advanced::mouse;
    use iced::{Event, Point};
    use micold_core::markdown::{Link, Style};

    fn plain(text: &str) -> Span {
        Span {
            text: text.to_owned(),
            style: Style::default(),
            link: None,
        }
    }

    fn link(text: &str, target: &str, activatable: bool) -> Span {
        Span {
            link: Some(Link {
                target: target.to_owned(),
                activatable,
            }),
            ..plain(text)
        }
    }

    fn sample() -> Vec<Span> {
        vec![
            plain("See "),
            link("the docs", "https://example.com/docs", true),
            plain(", "),
            link("the notes", "notes/readme.md", false),
            plain(" and "),
            link("mail", "mailto:dev@example.com", true),
            plain("."),
        ]
    }

    fn mounted(spans: &[Span]) -> Mounted<'_> {
        Mounted::new(LinkText::new(spans, roles()).on_link(|url| format!("open {url}")))
    }

    /// FR-004: one focus stop per activatable link; the relative link offers none.
    #[test]
    fn each_activatable_link_is_one_focus_stop() {
        let spans = sample();
        assert_eq!(mounted(&spans).drive(Focusables(0)), Some(2));
        let none = vec![plain("no "), link("links", "notes.md", false)];
        assert_eq!(mounted(&none).drive(Focusables(0)), Some(0));
    }

    /// FR-004: a link whose text changes style part-way (`[**bold** rest](url)`) is several spans
    /// but one link, so one stop; the same address again after other text is a stop of its own.
    #[test]
    fn a_link_split_across_styles_is_one_focus_stop() {
        let mut bold = link("bold", "https://example.com", true);
        bold.style.bold = true;
        let split = vec![bold, link(" rest", "https://example.com", true)];
        assert_eq!(mounted(&split).drive(Focusables(0)), Some(1));
        let again = vec![
            link("one", "https://example.com", true),
            plain(" and "),
            link("two", "https://example.com", true),
        ];
        assert_eq!(mounted(&again).drive(Focusables(0)), Some(2));
    }

    /// FR-004: Tab reaches the links in reading order and Enter opens the one focused.
    #[test]
    fn enter_opens_the_focused_link_in_reading_order() {
        let spans = sample();
        let mut m = mounted(&spans);
        m.focus_next();
        let first = m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
        assert_eq!(first, vec!["open https://example.com/docs".to_string()]);
        m.focus_next();
        let second = m.send(named_key(Named::Enter), mouse::Cursor::Unavailable);
        assert_eq!(second, vec!["open mailto:dev@example.com".to_string()]);
    }

    /// FR-004: a press on an activatable link opens it; on a link that is not, nothing is sent.
    #[test]
    fn a_press_opens_an_activatable_link_only() {
        let click = |spans: &[Span]| {
            let mut m = mounted(spans);
            let at = Point::new(4.0, m.node.bounds().height / 2.0);
            let mut out = m.hover(at);
            out.extend(m.press(at));
            out.extend(m.send(
                Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
                mouse::Cursor::Available(at),
            ));
            out
        };
        let opens = vec![link("docs", "https://example.com", true)];
        assert_eq!(click(&opens), vec!["open https://example.com".to_string()]);
        let inert = vec![link("docs", "docs/index.md", false)];
        assert!(click(&inert).is_empty());
    }

    /// An activatable link is drawn in `primary`; the rest of the text in `on_surface`, or
    /// `on_surface_variant` when muted.
    #[test]
    fn links_take_primary_and_muted_text_its_variant() {
        let r = roles();
        assert_eq!(link_ink(r), r.primary);
        assert_eq!(ink(r, false), r.on_surface);
        assert_eq!(ink(r, true), r.on_surface_variant);
    }
}
