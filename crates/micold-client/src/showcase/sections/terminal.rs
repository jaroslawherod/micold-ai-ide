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
