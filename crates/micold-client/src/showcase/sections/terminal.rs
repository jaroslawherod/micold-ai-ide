//! The terminal pane (feature 020, T022).
//!
//! `TerminalPane` renders a live session's grid, which the showcase has no access to — so it renders
//! the fabricated one from [`samples`] instead (FR-006). No component is omitted on the grounds that
//! it needs data, and this is the component that most needs it: in the application it appears only
//! after opening a project, creating a worktree and starting a session.

use iced::{Element, Length};
use micold_core::tokens::Roles;

use crate::showcase::catalogue::Layout;
use crate::showcase::gallery::{arrange, posed};
use crate::showcase::state::{Message, Showcase};
use crate::ui::material;
use crate::ui::terminal::TermPalette;

/// One pane at a given focus, at a height that shows the whole fabricated screen.
///
/// `TerminalPane` is the one component in the library that is **not generic over its message type**:
/// it emits `app::Message` directly, so it can only be composed by the application. The gallery maps
/// its messages to [`Message::NoOp`] rather than changing the component — a message-type parameter is
/// a change to the library, and FR-019 forbids this feature touching the application's behaviour. The
/// limitation is real and recorded in `docs/development/component-showcase.md`: it is exactly the kind
/// of thing a gallery reveals, and the fix belongs to whichever feature next needs the pane elsewhere.
fn pane<'a>(showcase: &'a Showcase, focused: bool) -> Element<'a, Message> {
    let palette = TermPalette::from_scheme(showcase.scheme);
    let native: Element<'a, crate::app::Message> =
        material::TerminalPane::new(showcase.grid(), palette)
            .focused(focused)
            .into();
    iced::widget::container(native.map(|_| Message::NoOp))
        .height(Length::Fixed(220.0))
        .into()
}

/// `TerminalPane` — focused and unfocused, over the fabricated grid.
///
/// The palette follows the active scheme, so switching the scheme re-renders the terminal's own
/// colours too (FR-008) rather than leaving one component behind in the other scheme.
pub fn terminal_pane<'a>(showcase: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed("unfocused", pane(showcase, false), roles),
            posed("focused", pane(showcase, true), roles),
        ],
        Layout::FullWidth,
    )
}

/// `Tab` — the three states a single tab has, side by side.
///
/// Marked, unmarked, and unmarked-with-something-in-its-leading-slot. The third is not a fourth
/// state of the tab so much as a demonstration that the slot is there: every tab reserves it whether
/// or not a mark goes in it (feature 026 FR-012c), and a slot you can only see when it is full looks
/// like a slot that appears.
///
/// The highlight is deliberately **not** posed, because it cannot be: a state layer is drawn on
/// hover and on press, so it is one of the entry's `live` states and the caption is what says so
/// (FR-004/FR-005). What a reader is meant to do here is rest the pointer on a tab and see a
/// rectangle that fills it, rather than the rounded pill a text button draws (FR-015, SC-010).
pub fn tab<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    arrange(
        vec![
            posed(
                "marked",
                material::Tab::new(
                    material::Text::new("1", material::TypeRole::Label, roles)
                        .tint(material::tab_content_colour(true, roles)),
                    roles,
                )
                .active(true)
                .on_press(Message::NoOp),
                roles,
            ),
            posed(
                "unmarked",
                material::Tab::new(
                    material::Text::new("2", material::TypeRole::Label, roles)
                        .tint(material::tab_content_colour(false, roles)),
                    roles,
                )
                .on_press(Message::NoOp),
                roles,
            ),
            posed(
                "stopped",
                material::Tab::new(
                    material::Text::new("3", material::TypeRole::Label, roles)
                        .tint(material::tab_content_colour(false, roles)),
                    roles,
                )
                .leading(material::ActivityBadge::for_emphasis(
                    Some(material::BadgeEmphasis::Stopped),
                    roles,
                ))
                .on_press(Message::NoOp),
                roles,
            ),
        ],
        Layout::Inline,
    )
}

/// `TabStrip` — the same strip twice, with its indicator on opposite edges (FR-014, SC-011).
///
/// The pairing is the entry. This application puts the indicator on a tab's **top** edge because its
/// strip is anchored to the window's bottom and the pane a tab selects is above it, which is the
/// opposite of Material's default placement (feature 012 FR-004b). A deliberate inversion that is
/// never shown next to the thing it inverts reads as a mistake to the next person who meets it —
/// and the two are exactly the kind of difference a gallery exists to settle by comparison rather
/// than by memory.
///
/// Both strips carry the same three tabs, so the only difference on the page is the one being
/// demonstrated.
pub fn tab_strip<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let strip = |edge: material::IndicatorEdge| {
        let tabs = (1..=3)
            .map(|n| {
                let active = n == 1;
                material::Tab::new(
                    material::Text::new(n.to_string(), material::TypeRole::Label, roles)
                        .tint(material::tab_content_colour(active, roles)),
                    roles,
                )
                .active(active)
                .on_press(Message::NoOp)
            })
            .collect();
        material::TabStrip::new(tabs, roles).edge(edge)
    };
    arrange(
        vec![
            posed(
                "Top — this application's own",
                strip(material::IndicatorEdge::Top),
                roles,
            ),
            posed(
                "Bottom — Material's default",
                strip(material::IndicatorEdge::Bottom),
                roles,
            ),
        ],
        Layout::FullWidth,
    )
}

/// `EdgeFade` — the four states of a scrolling region's edges, posed side by side.
///
/// Posed rather than left live, and that is the entry's whole reason to exist. The fade's two
/// states differ **only in role** — the surface's own tint for "there is more that way", the
/// indicator's accent for "and the marked member is what is out there" (FR-002e) — and a magnitude
/// or a hue difference is unreadable without the other state beside it to compare against. A live
/// instance would show one of them at a time, which is exactly how this cue fails.
///
/// The content behind it is a strip of tabs, because that is what it fades in the application and
/// a gradient over a flat ground says nothing about whether it competes with what it covers.
pub fn edge_fade<'a>(_s: &'a Showcase, roles: Roles, _i: usize) -> Element<'a, Message> {
    let strip = || {
        let tabs = (1..=4)
            .map(|n| {
                material::Tab::new(
                    material::Text::new(n.to_string(), material::TypeRole::Label, roles)
                        .tint(material::tab_content_colour(n == 1, roles)),
                    roles,
                )
                .active(n == 1)
                .on_press(Message::NoOp)
            })
            .collect();
        material::TabStrip::new(tabs, roles)
    };
    let posed_fade = |label: &'static str, leading: bool, trailing: bool, accent: Option<bool>| {
        posed(
            label,
            iced::widget::container(
                material::EdgeFade::new(strip(), roles)
                    .leading(leading)
                    .trailing(trailing)
                    .accent_on(accent)
                    .width(Length::Fixed(300.0)),
            )
            .width(Length::Fixed(300.0)),
            roles,
        )
    };
    arrange(
        vec![
            posed_fade("nothing beyond either edge", false, false, None),
            posed_fade("more that way", false, true, None),
            posed_fade("both edges", true, true, None),
            posed_fade("the marked tab is out there", false, true, Some(false)),
        ],
        Layout::FullWidth,
    )
}
