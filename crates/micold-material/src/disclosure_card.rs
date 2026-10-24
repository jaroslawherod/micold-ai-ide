//! `DisclosureCard` — a card whose header folds its body away (feature 939, research R11).
//!
//! The transcript's thinking and tool-call cards. The header is
//! [`AccordionSection`](super::AccordionSection)'s — a full-width text button, so the pointer, Space
//! and Enter with focus, the focus ring and the ripple are that one implementation's (FR-010) — with
//! a leading icon saying what the card is, a one-line [`Ellipsized`](super::Ellipsized) summary of
//! what it holds while folded, and a trailing status. The body is revealed by the
//! [`Accordion`](super::Accordion) panel under it.
//!
//! **Failed** is drawn three ways, so it never rests on colour alone (FR-012, SC-005): the caller's
//! trailing status carries the words and the icon (`Icon::ToolFailed` and "Failed" in the error
//! role, an [`IconLabel`](super::IconLabel)), and the card swaps its shadow for an `error` outline.
//!
//! Whether it is open is the caller's, as for `AccordionSection`: the card keeps no state.
//!
//! A short [`meta`](DisclosureCard::meta) — an Edit card's `+12 −3` (feature 940, FR-017) — sits
//! between the summary and the trailing status, folded or open. It is never cut: the summary,
//! which fills what is left, gives way instead.

use super::{style, AccordionSection, Text, TypeRole};
use crate::icons::Icon;
use iced::widget::text::Wrapping;
use iced::widget::{container, Row, Space};
use iced::{Alignment, Element, Length};
use micold_core::tokens::{spacing, Roles};

/// A folding card. Builder form (Principle VIII):
/// `DisclosureCard::new("Bash", msg, roles).leading(Icon::ToolCall).summary("cargo test")
/// .meta("+12 −3").trailing(status).open(open).body(body).failed(failed).into()`.
pub struct DisclosureCard<'a, M> {
    title: String,
    on_toggle: M,
    roles: Roles,
    leading: Option<Icon>,
    summary: Option<String>,
    meta: Option<String>,
    trailing: Option<Element<'a, M>>,
    open: bool,
    body: Option<Element<'a, M>>,
    failed: bool,
}

impl<'a, M: Clone + 'a> DisclosureCard<'a, M> {
    /// A folded card called `title` whose header emits `on_toggle`.
    pub fn new(title: impl Into<String>, on_toggle: M, roles: Roles) -> Self {
        Self {
            title: title.into(),
            on_toggle,
            roles,
            leading: None,
            summary: None,
            meta: None,
            trailing: None,
            open: false,
            body: None,
            failed: false,
        }
    }

    /// The icon before the title: what kind of card this is.
    pub fn leading(mut self, icon: Icon) -> Self {
        self.leading = Some(icon);
        self
    }

    /// One line saying what the card holds, shown while folded and cut with an ellipsis when the
    /// header is too narrow for it. An empty summary is none.
    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// A short text between the summary and the trailing status, folded and open, never cut with
    /// an ellipsis — an Edit card's added and removed line counts (feature 940, FR-017). An empty
    /// meta is none.
    pub fn meta(mut self, meta: impl Into<String>) -> Self {
        self.meta = Some(meta.into()).filter(|meta| !meta.is_empty());
        self
    }

    /// The status after the title, shown folded and open — an `IconLabel` such as "Running".
    pub fn trailing(mut self, status: impl Into<Element<'a, M>>) -> Self {
        self.trailing = Some(status.into());
        self
    }

    /// Show the card open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// What unfolding the card shows. Without it the card unfolds to nothing.
    pub fn body(mut self, body: impl Into<Element<'a, M>>) -> Self {
        self.body = Some(body.into());
        self
    }

    /// Draw the card as failed: its shadow becomes an `error` outline. The status the caller puts in
    /// [`trailing`](Self::trailing) says so in words and an icon (FR-012).
    pub fn failed(mut self, failed: bool) -> Self {
        self.failed = failed;
        self
    }
}

impl<'a, M: Clone + 'a> From<DisclosureCard<'a, M>> for Element<'a, M> {
    fn from(c: DisclosureCard<'a, M>) -> Self {
        let body = c
            .body
            .unwrap_or_else(|| Space::new().width(Length::Fill).into());
        let mut section = AccordionSection::new(c.title, body, c.roles)
            .open(c.open)
            .on_toggle(c.on_toggle)
            .summary_maybe(c.summary);
        if let Some(icon) = c.leading {
            section = section.leading(icon);
        }
        // Without a meta the trailing status goes in as it is, so a card without one lays out as
        // it always has.
        let trailing = match (c.meta, c.trailing) {
            (None, trailing) => trailing,
            (Some(meta), status) => {
                let meta: Element<'a, M> = Text::new(meta, TypeRole::Label, c.roles)
                    .muted()
                    .wrapping(Wrapping::None)
                    .into();
                Some(
                    Row::with_children(std::iter::once(meta).chain(status))
                        .spacing(spacing::SM)
                        .align_y(Alignment::Center)
                        .into(),
                )
            }
        };
        if let Some(trailing) = trailing {
            section = section.trailing(trailing);
        }
        container(section)
            .padding(spacing::XS)
            .width(Length::Fill)
            .style(style::disclosure_card(c.roles, c.failed))
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{dark_roles, named_key, roles, Mounted, WINDOW};
    use super::super::{Text, TypeRole};
    use super::*;
    use iced::advanced::mouse;
    use iced::keyboard::key::Named;
    use iced::{Event, Point};

    const BODY: &str = "full tool input";
    const SUMMARY: &str = "cargo test --workspace --all-targets -- --nocapture";

    fn card<'a>(open: bool, r: Roles) -> DisclosureCard<'a, String> {
        DisclosureCard::new("Bash", "toggled".to_string(), r)
            .leading(Icon::ToolCall)
            .summary(SUMMARY)
            .trailing(Text::new("Running", TypeRole::Label, r))
            .open(open)
            .body(Text::new(BODY, TypeRole::Body, r))
    }

    /// The header button: the card's padding, then the section's first child.
    fn header(m: &Mounted<'_>) -> iced::Rectangle {
        m.node.children()[0].children()[0].bounds()
    }

    fn header_centre(m: &Mounted<'_>) -> Point {
        let h = header(m);
        Point::new(h.x + h.width / 2.0, h.y + h.height / 2.0)
    }

    /// FR-010: a press on the header, and Space or Enter once it holds the keyboard, each emit the
    /// toggle — the `AccordionSection` header's own behaviour.
    #[test]
    fn the_header_toggles_by_pointer_and_by_keyboard() {
        let mut m = Mounted::new(card(false, roles()));
        let at = header_centre(&m);
        let mut out = m.press(at);
        out.extend(m.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            mouse::Cursor::Available(at),
        ));
        assert_eq!(out, vec!["toggled".to_string()]);
        for key in [Named::Space, Named::Enter] {
            let out = m.send(named_key(key), mouse::Cursor::Unavailable);
            assert_eq!(out, vec!["toggled".to_string()], "{key:?} must toggle");
        }
    }

    /// The header is one Tab stop, like `AccordionSection`'s; a folded body offers none.
    #[test]
    fn the_header_is_one_focus_stop_and_a_folded_body_offers_none() {
        let stops = |open: bool| {
            let r = roles();
            let body = crate::TextField::<String>::new("", "v", r).label("Label");
            Mounted::new(card(open, r).body(body))
                .drive(super::super::test_support::Focusables(0))
                .expect("the count finishes")
        };
        assert_eq!(stops(false), 1, "folded: the header alone");
        assert!(stops(true) > 1, "open: the body's field too");
    }

    /// Folded, the body is not shown and takes no room; open, it is painted — both themes.
    #[test]
    fn folded_hides_the_body_and_open_shows_it() {
        for r in [roles(), dark_roles()] {
            let mut folded = Mounted::new(card(false, r));
            let mut open = Mounted::new(card(true, r));
            assert!(
                open.node.size().height > folded.node.size().height,
                "open is taller than folded"
            );
            let shown = |m: &mut Mounted<'_>| m.painted_text().iter().any(|p| p.content == BODY);
            assert!(!shown(&mut folded), "a folded card paints no body");
            assert!(shown(&mut open), "an open card paints its body");
        }
    }

    /// Folded, the summary is one caption line inside the header, cut with an ellipsis when the
    /// card is narrow; the title, icon and status are painted too.
    #[test]
    fn the_summary_is_one_line() {
        let r = roles();
        let mut m = Mounted::new(
            iced::widget::container(card(false, r)).width(Length::Fixed(WINDOW.width / 2.0)),
        );
        let painted = m.painted_text();
        let summary = painted
            .iter()
            .find(|p| p.content.starts_with("cargo"))
            .unwrap_or_else(|| panic!("the summary is painted: {painted:?}"));
        assert!(summary.content.ends_with('…'), "{:?}", summary.content);
        assert!(
            summary.bounds.height <= TypeRole::Caption.line_height_dp() + 0.5,
            "one line: {summary:?}"
        );
        for content in ["Bash", "Running", &Icon::ToolCall.glyph().to_string()] {
            assert!(
                painted.iter().any(|p| p.content == content),
                "`{content}` is painted: {painted:?}"
            );
        }
    }

    /// FR-017: a meta sits in the header beside the status, whole, while the summary gives way;
    /// folded and open alike.
    #[test]
    fn a_meta_is_never_cut_and_the_summary_gives_way() {
        let r = roles();
        for open in [false, true] {
            let mut m = Mounted::new(
                iced::widget::container(card(open, r).meta("+12 \u{2212}3"))
                    .width(Length::Fixed(WINDOW.width * 0.75)),
            );
            let painted = m.painted_text();
            assert!(
                painted.iter().any(|p| p.content == "+12 \u{2212}3"),
                "open {open}: the meta is painted whole: {painted:?}"
            );
            if !open {
                let summary = painted
                    .iter()
                    .find(|p| p.content.starts_with("cargo"))
                    .expect("the summary is painted");
                assert!(summary.content.ends_with('…'), "{:?}", summary.content);
            }
        }
    }

    /// An open card shows no summary: the body holds the whole of it.
    #[test]
    fn an_open_card_paints_no_summary() {
        let mut m = Mounted::new(card(true, roles()));
        assert!(m
            .painted_text()
            .iter()
            .all(|p| !p.content.starts_with("cargo")));
    }
}
