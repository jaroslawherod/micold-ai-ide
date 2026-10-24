//! `ReviewBar` — the Review tab's bottom bar: the counts, a send error, and the fixed send action
//! (feature 1270, contract review-bar-ui.md B1–B4, B8, B10).
//!
//! A `review_bar::HEIGHT` row with an outline divider on top, on the surface container. The leading
//! part holds the shown diff's counts, then the send error with a × that dismisses it. The trailing
//! group, right to left from the bar's edge, is **Send review (n)** at the fixed
//! `review_bar::SEND_WIDTH`, the overflow trigger (⋮) and the **Comments** toggle; none of them is
//! sized by its label, the counts or the error, so the send button never moves (B3, SC-001).
//!
//! The counts shorten when the bar is narrow (research R3): the leading part picks, when it is laid
//! out, the longest `CountsForm` whose text fits the room it has — Full, then DraftsOnly, then
//! nothing. The error is kept whole while there is room and is ellipsized only once the counts are
//! gone. The forms and the wording are `micold_core::review::bar`'s; only the measuring is here.
//!
//! Builder form: `ReviewBar::new(counts, send_button(&counts, sending, sendable), roles)
//! .drawer_open(open).error(message).on_send(Msg::Send).on_overflow(Msg::Menu)
//! .on_toggle_drawer(Msg::Drawer).on_dismiss_error(Msg::Dismiss).into()`.

use iced::widget::{column, container, responsive, row};
use iced::{Alignment, Background, Element, Length};
use micold_core::review::bar::{counts_text, CountsForm, ScopeCounts, SendButton};
use micold_core::tokens::{anatomy, spacing, Roles};

use super::icons::Icon;
use super::shaped_text::width_of;
use super::{style, Button, Divider, Ellipsized, IconButton, Text, TypeRole};

/// The leading part's text role.
const ROLE: TypeRole = TypeRole::Body;

/// The **Comments** toggle's label.
pub const COMMENTS: &str = "Comments";

/// The room the error's × takes: a compact icon button, its glyph at the body role and its inset
/// on both sides.
fn dismiss_width() -> f32 {
    TypeRole::Body.size() + 2.0 * spacing::XS
}

/// The bar at the bottom of an entry's Review tab.
pub struct ReviewBar<'a, M> {
    counts: ScopeCounts,
    send: SendButton,
    roles: Roles,
    drawer_open: bool,
    error: Option<String>,
    on_send: Option<M>,
    on_overflow: Option<M>,
    on_toggle_drawer: Option<M>,
    on_dismiss_error: Option<M>,
    marker: std::marker::PhantomData<&'a ()>,
}

impl<'a, M: Clone + 'a> ReviewBar<'a, M> {
    /// The bar of a diff with `counts`, its send button `send` (from core's `send_button`), themed
    /// by `roles`.
    pub fn new(counts: ScopeCounts, send: SendButton, roles: Roles) -> Self {
        Self {
            counts,
            send,
            roles,
            drawer_open: false,
            error: None,
            on_send: None,
            on_overflow: None,
            on_toggle_drawer: None,
            on_dismiss_error: None,
            marker: std::marker::PhantomData,
        }
    }

    /// Draw the **Comments** toggle as on: the drawer is open (B8).
    pub fn drawer_open(mut self, open: bool) -> Self {
        self.drawer_open = open;
        self
    }

    /// Show `message` (core's `error_text`) after the counts, with a × (FR-006).
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.error = Some(message.into());
        self
    }

    /// The message **Send review (n)** sends while the button is enabled.
    pub fn on_send(mut self, message: M) -> Self {
        self.on_send = Some(message);
        self
    }

    /// The message the overflow trigger (⋮) sends; without it the trigger is disabled.
    pub fn on_overflow(mut self, message: M) -> Self {
        self.on_overflow = Some(message);
        self
    }

    /// The message the **Comments** toggle sends.
    pub fn on_toggle_drawer(mut self, message: M) -> Self {
        self.on_toggle_drawer = Some(message);
        self
    }

    /// The message the error's × sends.
    pub fn on_dismiss_error(mut self, message: M) -> Self {
        self.on_dismiss_error = Some(message);
        self
    }
}

/// The longest counts text that fits `room`, if any (research R3).
fn fitting_counts(counts: &ScopeCounts, room: f32) -> Option<String> {
    [CountsForm::Full, CountsForm::DraftsOnly]
        .into_iter()
        .filter_map(|form| counts_text(counts, form))
        .find(|text| width_of(text, ROLE) <= room)
}

impl<'a, M: Clone + 'a> From<ReviewBar<'a, M>> for Element<'a, M> {
    fn from(bar: ReviewBar<'a, M>) -> Self {
        let r = bar.roles;
        let (counts, error, dismiss) = (bar.counts, bar.error, bar.on_dismiss_error);
        let leading = responsive(move |size| {
            let reserve = error.as_deref().map_or(0.0, |message| {
                width_of(message, ROLE) + spacing::SM + dismiss_width() + spacing::SM
            });
            let mut part = row![].spacing(spacing::SM).align_y(Alignment::Center);
            if let Some(text) = fitting_counts(&counts, size.width - reserve) {
                part = part.push(Text::new(text, ROLE, r).muted());
            }
            if let Some(message) = error.clone() {
                part = part.push(Ellipsized::<M>::at_role(message, ROLE, r.error));
                part = part.push(
                    IconButton::new(Icon::Close, r)
                        .compact()
                        .on_press_maybe(dismiss.clone()),
                );
            }
            container(part)
                .height(Length::Fill)
                .align_y(Alignment::Center)
                .clip(true)
                .into()
        });

        let mut toggle = Button::text(COMMENTS, r).selected(bar.drawer_open);
        if let Some(message) = bar.on_toggle_drawer {
            toggle = toggle.on_press(message);
        }
        let mut send = if bar.send.primary {
            Button::filled(bar.send.label, r)
        } else {
            Button::outlined(bar.send.label, r)
        };
        if bar.send.enabled {
            send = send.on_press_maybe(bar.on_send);
        }
        let trailing = row![
            toggle,
            IconButton::new(Icon::Menu, r)
                .compact()
                .on_press_maybe(bar.on_overflow),
            send.width(Length::Fixed(anatomy::review_bar::SEND_WIDTH)),
        ]
        .spacing(spacing::SM)
        .align_y(Alignment::Center);

        let strip = row![leading, trailing]
            .spacing(spacing::SM)
            .padding([0.0, spacing::MD])
            .height(Length::Fill)
            .align_y(Alignment::Center);
        container(column![Divider::horizontal(r), strip])
            .width(Length::Fill)
            .height(Length::Fixed(anatomy::review_bar::HEIGHT))
            .style(move |_| container::Style {
                background: Some(Background::Color(style::color(r.surface_container))),
                ..container::Style::default()
            })
            .into()
    }
}

#[cfg(test)]
mod tests {
    //! Contract B2, B3 (feature 1270 T008): the counts take the longest form that fits, and the
    //! trailing group never moves.

    use super::*;
    use iced::advanced::layout::Layout;
    use iced::advanced::mouse;
    use iced::{Event, Rectangle, Size};
    use micold_core::review::bar::{counts_text, send_button, CountsForm, ScopeCounts};
    use micold_core::tokens::LIGHT;

    use crate::ui::material::test_support::{self, Mounted};

    /// Where the trailing group sits in the bar's tree: container → column → [divider, row] →
    /// [leading, trailing].
    const TRAILING: [usize; 3] = [0, 1, 1];

    const WIDE: f32 = 1_280.0;

    fn counts(draft: usize, sent: usize, applied: usize, answered: usize) -> ScopeCounts {
        ScopeCounts {
            draft,
            sent,
            applied,
            answered,
        }
    }

    fn mixed() -> ScopeCounts {
        counts(3, 2, 1, 1)
    }

    fn bar(counts: ScopeCounts) -> ReviewBar<'static, String> {
        ReviewBar::new(counts, send_button(&counts, false, true), LIGHT)
            .on_send("send".to_owned())
            .on_overflow("overflow".to_owned())
            .on_toggle_drawer("toggle".to_owned())
            .on_dismiss_error("dismiss".to_owned())
    }

    fn room(width: f32) -> Size {
        Size::new(width, 200.0)
    }

    /// The bounds of the trailing group's three controls (Comments, ⋮, Send), absolute.
    fn trailing(bar: ReviewBar<'static, String>, width: f32) -> Vec<Rectangle> {
        let node = test_support::laid_out(bar.into(), room(width));
        let mut layout = Layout::new(&node);
        for index in TRAILING {
            layout = layout
                .children()
                .nth(index)
                .expect("the bar's tree changed shape");
        }
        layout.children().map(|child| child.bounds()).collect()
    }

    /// The texts the bar paints laid out `width` wide.
    fn painted(bar: ReviewBar<'static, String>, width: f32) -> Vec<String> {
        Mounted::new_in(bar, room(width))
            .painted_text()
            .into_iter()
            .map(|p| p.content)
            .collect()
    }

    /// Press and release the pointer at the centre of the painted text `content`.
    fn click(bar: ReviewBar<'static, String>, width: f32, content: &str) -> Vec<String> {
        let mut mounted = Mounted::new_in(bar, room(width));
        let at = mounted
            .painted_text()
            .into_iter()
            .find(|p| p.content == content)
            .unwrap_or_else(|| panic!("{content:?} is not painted"))
            .bounds
            .center();
        let cursor = mouse::Cursor::Available(at);
        mounted.send(
            Event::Mouse(mouse::Event::CursorMoved { position: at }),
            cursor,
        );
        mounted.send(
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            cursor,
        );
        mounted.send(
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
            cursor,
        )
    }

    /// The close glyph's character, as `IconButton` paints it.
    fn close_glyph() -> String {
        crate::ui::material::icons::Icon::Close.glyph().to_string()
    }

    /// The room the trailing group and the bar's padding take: what is left is the counts'.
    fn taken(width: f32) -> f32 {
        let group = trailing(bar(mixed()), width);
        width - group[0].x + spacing::MD
    }

    /// B2: at three widths the bar draws Full, then DraftsOnly, then nothing.
    #[test]
    fn the_counts_take_the_longest_form_that_fits() {
        let full = counts_text(&mixed(), CountsForm::Full).expect("full text");
        let drafts = counts_text(&mixed(), CountsForm::DraftsOnly).expect("drafts text");
        assert_eq!(full, "3 drafts · 2 sent · 1 applied · 1 answered");

        let wide = painted(bar(mixed()), WIDE);
        assert!(wide.contains(&full), "{wide:?}");

        let medium = taken(WIDE) + width_of(&drafts, ROLE) + 4.0 * spacing::SM;
        assert!(
            width_of(&full, ROLE) > medium - taken(WIDE),
            "precondition: the full text does not fit"
        );
        let at_medium = painted(bar(mixed()), medium);
        assert!(at_medium.contains(&drafts), "{at_medium:?}");
        assert!(!at_medium.contains(&full), "{at_medium:?}");

        let narrow = taken(WIDE) + spacing::SM;
        let at_narrow = painted(bar(mixed()), narrow);
        assert!(
            !at_narrow.iter().any(|t| t.contains("draft")),
            "{at_narrow:?}"
        );
        assert!(
            at_narrow.iter().any(|t| t == "Send review (3)"),
            "{at_narrow:?}"
        );
    }

    /// B2: no comments reads "No review comments yet"; no drafts and no room shows nothing.
    #[test]
    fn no_comments_reads_so_and_drafts_only_needs_drafts() {
        let none = painted(bar(counts(0, 0, 0, 0)), WIDE);
        assert!(
            none.iter().any(|t| t == "No review comments yet"),
            "{none:?}"
        );
        let sent_only = counts(0, 4, 0, 0);
        assert_eq!(counts_text(&sent_only, CountsForm::DraftsOnly), None);
        let width = taken(WIDE) + width_of("4 sent", ROLE) - 1.0;
        let at = painted(bar(sent_only), width);
        assert!(!at.iter().any(|t| t.contains("sent")), "{at:?}");
    }

    /// B3, SC-001: the trailing group's bounds are the same whatever the label, the counts, the
    /// error and the drawer, and the send button is `SEND_WIDTH` wide at the bar's right padding.
    #[test]
    fn the_trailing_group_never_moves() {
        let width = 1_000.0;
        let sending = SendButton {
            label: "Sending…".to_owned(),
            enabled: false,
            primary: false,
        };
        let many = counts(5_000, 120, 40, 7);
        let variants: Vec<ReviewBar<'static, String>> = vec![
            bar(counts(0, 0, 0, 0)),
            bar(mixed()),
            bar(many),
            bar(mixed()).drawer_open(true),
            bar(mixed()).error("Couldn't send the review: no session could be started"),
            bar(many)
                .error("Couldn't send the review: ".to_owned() + &"why ".repeat(80))
                .drawer_open(true),
            ReviewBar::new(mixed(), sending, LIGHT).on_toggle_drawer("toggle".to_owned()),
        ];
        let expected = trailing(bar(mixed()), width);
        assert_eq!(expected.len(), 3, "Comments, ⋮ and Send");
        for (i, variant) in variants.into_iter().enumerate() {
            assert_eq!(trailing(variant, width), expected, "variant {i}");
        }
        let send = expected[2];
        assert_eq!(send.width, anatomy::review_bar::SEND_WIDTH);
        assert_eq!(send.x + send.width, width - spacing::MD);
        assert!(
            expected[1].x + expected[1].width <= send.x,
            "⋮ left of Send"
        );
        assert!(
            expected[0].x + expected[0].width <= expected[1].x,
            "Comments left of ⋮"
        );
        assert_eq!(
            send_button(&many, false, true).label,
            "Send review (999+)",
            "the widest count label"
        );
    }

    /// R4, T009: the widest label fits `SEND_WIDTH`, which is that width rounded up.
    #[test]
    fn the_widest_label_fits_the_send_width() {
        let natural = |label: &str| {
            test_support::size_of(
                Button::<()>::filled(label, LIGHT).into(),
                Size::new(1_000.0, 100.0),
            )
            .width
        };
        let widest = ["Send review (999+)", "Send review (999)", "Sending…"]
            .into_iter()
            .map(natural)
            .fold(0.0_f32, f32::max);
        assert!(widest <= anatomy::review_bar::SEND_WIDTH, "{widest}");
        assert_eq!(
            anatomy::review_bar::SEND_WIDTH,
            widest.ceil(),
            "SEND_WIDTH is the widest label's width"
        );
    }

    /// B1: the bar is `review_bar::HEIGHT` high whatever it shows.
    #[test]
    fn the_bar_is_its_height() {
        for variant in [
            bar(counts(0, 0, 0, 0)),
            bar(mixed()).drawer_open(true),
            bar(mixed()).error("Couldn't send the review: no session"),
        ] {
            let size = test_support::size_of(variant.into(), room(320.0));
            assert_eq!(size.height, anatomy::review_bar::HEIGHT);
        }
    }

    /// B2, FR-006: the error is drawn after the counts and its × emits the dismiss message.
    #[test]
    fn the_error_is_shown_and_its_close_emits_the_dismiss_message() {
        let message = "Couldn't send the review: no session";
        let shown = painted(bar(mixed()).error(message), WIDE);
        assert!(shown.iter().any(|t| t == message), "{shown:?}");
        assert!(
            !painted(bar(mixed()), WIDE).contains(&close_glyph()),
            "no × without an error"
        );
        assert_eq!(
            click(bar(mixed()).error(message), WIDE, &close_glyph()),
            ["dismiss"]
        );
    }

    /// B4, B8: Send, ⋮ and Comments emit their messages; a disabled Send emits nothing.
    #[test]
    fn the_controls_emit_their_messages() {
        assert_eq!(click(bar(mixed()), WIDE, "Send review (3)"), ["send"]);
        assert_eq!(click(bar(mixed()), WIDE, "Comments"), ["toggle"]);
        let menu = crate::ui::material::icons::Icon::Menu.glyph().to_string();
        assert_eq!(click(bar(mixed()), WIDE, &menu), ["overflow"]);
        assert!(click(bar(counts(0, 2, 0, 0)), WIDE, "Send review (0)").is_empty());
    }
}
