//! `Accordion` — a panel that opens downward from its trigger (Constitution Principle VIII).
//!
//! The sidebar's tag-filter panel (feature 009): collapsed to nothing by default, pushing the list
//! below it down as it opens rather than floating over it. That last part is what makes it an
//! accordion rather than a popover, and why it uses [`expand`](super::expand) — a top-anchored
//! reveal — instead of the fade every floating surface gets.
//!
//! It owns its own reveal (feature 017, FR-011): a caller says whether it is open, never how far
//! open it currently is.

use std::time::Duration;

use iced::{Element, Length};
use micold_core::tokens::Roles;

/// How long the panel takes to open or close. Matches the menu fade — both are small panels
/// answering a press on a toolbar control, and they would read as inconsistent if they differed.
const REVEAL: Duration = Duration::from_millis(90);

/// A panel that reveals downward. Builder form (Principle VIII):
/// `Accordion::new(body, roles).open(flag).into()`.
pub struct Accordion<'a, M> {
    content: Element<'a, M>,
    roles: Roles,
    open: bool,
}

impl<'a, M: Clone + 'a> Accordion<'a, M> {
    /// A closed accordion holding `content`.
    pub fn new(content: impl Into<Element<'a, M>>, roles: Roles) -> Self {
        Self {
            content: content.into(),
            roles,
            open: false,
        }
    }

    /// Whether the panel is open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }
}

impl<'a, M: Clone + 'a> From<Accordion<'a, M>> for Element<'a, M> {
    fn from(a: Accordion<'a, M>) -> Self {
        super::expand(
            super::menu_panel(a.content, Length::Shrink, a.roles, false),
            a.open,
            REVEAL,
        )
        .into()
    }
}
