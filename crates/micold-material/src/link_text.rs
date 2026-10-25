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
//! The focus indicator outlines the focused link only: its rectangles are measured from a
//! paragraph laid out like the text, so a link that wraps is outlined per line fragment.
//!
//! Everything shown is owned, so the element is `'static` and can sit inside iced `lazy`.

use std::borrow::Cow;
use std::ops::Range;
use std::rc::Rc;

use iced::advanced::text::{Paragraph as _, Shaping};
use iced::font::{Style as Slant, Weight};
use iced::keyboard::key::Named;
use iced::widget::text::{Alignment, Span as TextSpan, Wrapping};
use iced::widget::{rich_text, span};
use iced::{alignment, Element, Font, Length, Pixels, Rectangle, Size};
use micold_core::markdown::Span;
use micold_core::tokens::{state, Rgb, Roles};
use unicode_segmentation::UnicodeSegmentation;

use super::keyboard_focus::{Indicator, Regions, TakesTheKeyboard};
use super::roboto::italic_has_glyph;
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
pub fn ink(r: Roles, muted: bool) -> Rgb {
    if muted {
        r.on_surface_variant
    } else {
        r.on_surface
    }
}

/// The colour of an activatable link.
pub fn link_ink(r: Roles) -> Rgb {
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
    // Emphasis draws in Roboto's own italic at the span's weight (feature 1259). Code inside
    // emphasis stays upright: the bundled Fira Mono has no italic, and a synthesised slant would
    // misalign the column a code span is read in.
    if s.style.italic && !s.style.code {
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

/// `spans` with each emphasis run split where Roboto's italic has no glyph, those pieces upright.
///
/// cosmic-text falls back only to faces of the requested style, and hosts ship CJK (and most
/// other scripts) upright only, so an italic run of 漢字 found no face and drew boxes (feature 1259
/// visual pass B6). Upright, those characters fall back to a host face as upright text does (spec
/// Edge: coverage). Decided per grapheme, by its first character, so an emoji sequence or a
/// combining mark is never split. Borrowed when nothing needs splitting.
fn upright_where_italic_lacks(spans: &[Span]) -> Cow<'_, [Span]> {
    let slanted = |s: &Span| s.style.italic && !s.style.code;
    let lacking = |s: &Span| slanted(s) && s.text.chars().any(|c| !italic_has_glyph(c));
    if !spans.iter().any(lacking) {
        return Cow::Borrowed(spans);
    }
    let mut out = Vec::with_capacity(spans.len() + 2);
    for s in spans {
        if !lacking(s) {
            out.push(s.clone());
            continue;
        }
        let piece = |text: &str, upright: bool| Span {
            text: text.to_owned(),
            style: micold_core::markdown::Style {
                italic: !upright,
                ..s.style
            },
            link: s.link.clone(),
        };
        let mut start = 0;
        let mut upright = None;
        for (at, g) in s.text.grapheme_indices(true) {
            let lacks = g.chars().next().is_some_and(|c| !italic_has_glyph(c));
            if let Some(was) = upright.filter(|&was| was != lacks) {
                out.push(piece(&s.text[start..at], was));
                start = at;
            }
            upright = Some(lacks);
        }
        out.push(piece(&s.text[start..], upright.unwrap_or(false)));
    }
    Cow::Owned(out)
}

/// Where spans `run` of `spans` fall when the text is laid out in `room`, relative to the text's
/// top-left: one rectangle per line fragment. The text widget keeps its own paragraph out of reach,
/// so this lays out an identical one (same spans, size, line height, font, wrapping and width).
fn link_regions(
    spans: &[TextSpan<'static, String, Font>],
    run: Range<usize>,
    room: Size,
    size: f32,
    line_height: iced::widget::text::LineHeight,
    font: Font,
) -> Vec<Rectangle> {
    let paragraph = <iced::Renderer as iced::advanced::text::Renderer>::Paragraph::with_spans(
        iced::advanced::text::Text {
            content: spans,
            bounds: room,
            size: Pixels(size),
            line_height,
            font,
            align_x: Alignment::Default,
            align_y: alignment::Vertical::Top,
            shaping: Shaping::Advanced,
            wrapping: Wrapping::WordOrGlyph,
        },
    );
    let mut out: Vec<Rectangle> = Vec::new();
    for r in run.flat_map(|i| paragraph.span_bounds(i)) {
        // A link of several styled spans is one outline: join fragments on the same line.
        match out.last_mut() {
            Some(last)
                if (last.y - r.y).abs() < 0.5
                    && (last.height - r.height).abs() < 0.5
                    && r.x <= last.x + last.width + 0.5 =>
            {
                let right = (last.x + last.width).max(r.x + r.width);
                last.width = right - last.x;
            }
            _ => out.push(r),
        }
    }
    out
}

impl<'a, 'b, M: Clone + 'static> From<LinkText<'a, M>> for Element<'b, M> {
    fn from(t: LinkText<'a, M>) -> Self {
        let r = t.roles;
        let opens = t.on_link.is_some();
        let spans = upright_where_italic_lacks(t.spans);
        let shown: Vec<TextSpan<'static, String, Font>> = spans
            .iter()
            .map(|s| painted(s, t.role.font(), r, t.muted, t.strong, opens))
            .collect();
        let mut text = rich_text(shown.clone())
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
        let mut targets: Vec<(&str, Range<usize>)> = Vec::new();
        let mut previous = None;
        for (i, s) in spans.iter().enumerate() {
            let link = s.link.as_ref().filter(|l| l.activatable);
            if let Some(l) = link.filter(|_| previous != Some(link)) {
                targets.push((l.target.as_str(), i..i + 1));
            } else if link.is_some() {
                if let Some((_, run)) = targets.last_mut() {
                    run.end = i + 1;
                }
            }
            previous = Some(link);
        }
        let shown: Rc<Vec<TextSpan<'static, String, Font>>> = Rc::new(shown);
        let (size, line_height, font) = (t.role.size(), t.role.line_height(), t.role.font());
        for (target, run) in targets.into_iter().rev() {
            let shown = Rc::clone(&shown);
            let regions: Regions = Rc::new(move |room| {
                link_regions(&shown, run.clone(), room, size, line_height, font)
            });
            let message = on_link(target.to_owned());
            element = TakesTheKeyboard::new(element, true)
                .key(Named::Enter, message.clone())
                .key(Named::Space, message)
                .press_takes_focus(false)
                .bare_child()
                .regions(regions)
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

    fn styled(bold: bool, italic: bool, code: bool) -> Span {
        Span {
            style: Style {
                bold,
                italic,
                code,
                ..Style::default()
            },
            ..plain("word")
        }
    }

    /// The font `painted` gives `s` on a regular-weight base, neither muted, strong nor opening.
    fn font_of(s: &Span) -> Font {
        painted(s, super::super::ROBOTO, roles(), false, false, false)
            .font
            .expect("painted always sets a font")
    }

    /// FR-010: an italic span keeps the italic slant, at the base weight.
    #[test]
    fn an_italic_span_is_slanted_at_the_base_weight() {
        let font = font_of(&styled(false, true, false));
        assert_eq!(font.style, Slant::Italic, "an italic span is not slanted");
        assert_eq!(font.weight, Weight::Normal, "an italic span changed weight");
        assert_eq!(font.family, super::super::ROBOTO.family);
    }

    /// US2, FR-003: bold italic is bold and slanted, so it resolves to the 600 italic.
    #[test]
    fn a_bold_italic_span_is_bold_and_slanted() {
        let font = font_of(&styled(true, true, false));
        assert_eq!(
            font.weight,
            Weight::Bold,
            "a bold-italic span lost its weight"
        );
        assert_eq!(
            font.style,
            Slant::Italic,
            "a bold-italic span is not slanted"
        );
    }

    /// Edge (inline code), contract C-4: code inside emphasis stays monospace and upright, and is
    /// bold when also bold.
    #[test]
    fn code_inside_emphasis_stays_upright() {
        let font = font_of(&styled(false, true, true));
        assert_eq!(font.family, Font::MONOSPACE.family, "code is not monospace");
        assert_eq!(font.style, Slant::Normal, "code inside emphasis is slanted");
        let bold = font_of(&styled(true, true, true));
        assert_eq!(
            bold.weight,
            Weight::Bold,
            "bold code inside emphasis lost its weight"
        );
        assert_eq!(
            bold.style,
            Slant::Normal,
            "bold code inside emphasis is slanted"
        );
    }

    /// Edge (coverage), FR-013 of feature 018: characters Roboto's italic lacks (CJK, emoji) leave
    /// the emphasis run upright, so they fall back to a host face as upright text does; cosmic-text
    /// only falls back to faces of the requested style, and hosts ship no italic CJK. The Latin word
    /// stays italic, and a split link stays one link.
    #[test]
    fn emphasis_the_italic_lacks_draws_upright() {
        let target = "https://example.com/";
        let mut emphasis = styled(false, true, false);
        // An emoji, built at run time: `tests/ui_glyph_literals.rs` rejects an emoji literal.
        let emoji = char::from_u32(0x1F642)
            .expect("U+1F642 is a scalar value")
            .to_string();
        emphasis.text = format!("漢字 and {emoji}");
        emphasis.link = link("x", target, true).link;
        let split = upright_where_italic_lacks(std::slice::from_ref(&emphasis));
        let runs: Vec<(&str, bool)> = split
            .iter()
            .map(|s| (s.text.as_str(), s.style.italic))
            .collect();
        assert_eq!(
            runs,
            [("漢字", false), (" and ", true), (emoji.as_str(), false)]
        );
        assert!(
            split.iter().all(|s| s.link == emphasis.link),
            "a split link lost its target"
        );
        let mut plain_italic = styled(false, true, false);
        plain_italic.text = "only Latin".to_owned();
        assert!(
            matches!(
                upright_where_italic_lacks(std::slice::from_ref(&plain_italic)),
                Cow::Borrowed(_)
            ),
            "emphasis the italic covers was copied"
        );
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

    /// The focus indicator outlines the focused link, not the paragraph around it.
    #[test]
    fn the_focus_ring_surrounds_the_focused_link_only() {
        let spans = vec![
            link("docs", "https://example.com", true),
            plain(" and then a good deal of ordinary text that is not a link at all"),
        ];
        let backdrop = style::color(roles().surface);
        let mut m = mounted(&spans);
        let (rest, width) = m.drawn(backdrop);
        m.focus_next();
        let (focused, _) = m.drawn(backdrop);
        let changed: Vec<usize> = (0..rest.len() / 4)
            .filter(|p| rest[p * 4..p * 4 + 4] != focused[p * 4..p * 4 + 4])
            .map(|p| p % width as usize)
            .collect();
        assert!(!changed.is_empty(), "no ring drawn for the focused link");
        let right = changed.into_iter().max().unwrap_or(0);
        assert!(
            (right as f32) < m.node.bounds().width / 4.0,
            "the ring reaches x={right}: it wraps more than the link"
        );
    }

    /// A link of several styled spans on one line is one region, not one per span.
    #[test]
    fn a_link_of_several_spans_is_one_region_per_line() {
        let r = roles();
        let mut bold = link("bold", "https://x.test", true);
        bold.style.bold = true;
        let spans = [
            link("foo ", "https://x.test", true),
            bold,
            link(" baz", "https://x.test", true),
        ];
        let shown: Vec<TextSpan<'static, String, Font>> = spans
            .iter()
            .map(|s| painted(s, TypeRole::Body.font(), r, false, false, true))
            .collect();
        let regions = link_regions(
            &shown,
            0..3,
            Size::new(2000.0, 1000.0),
            TypeRole::Body.size(),
            TypeRole::Body.line_height(),
            TypeRole::Body.font(),
        );
        assert_eq!(regions.len(), 1);
    }

    /// A link that wraps is outlined per line fragment.
    #[test]
    fn a_wrapped_link_has_a_region_per_line() {
        let spans = [link("one two three four five six", "https://x.test", true)];
        let r = roles();
        let shown: Vec<TextSpan<'static, String, Font>> = spans
            .iter()
            .map(|s| painted(s, TypeRole::Body.font(), r, false, false, true))
            .collect();
        let regions = |w: f32| {
            link_regions(
                &shown,
                0..1,
                Size::new(w, 1000.0),
                TypeRole::Body.size(),
                TypeRole::Body.line_height(),
                TypeRole::Body.font(),
            )
        };
        assert_eq!(regions(2000.0).len(), 1);
        assert!(regions(60.0).len() > 1);
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
