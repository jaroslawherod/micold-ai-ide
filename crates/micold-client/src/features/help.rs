//! The Help menu: what it offers, and the surface it opens as (feature 021, T031).
//!
//! A tenth feature module, and the smallest — but FR-001 asks where a feature lives, not how big
//! it is, and the answer for the toolbar's overflow menu was previously "two constants at the top
//! of `app.rs` and a `bool` two hundred lines below them". The actions and the surface that shows
//! them are one feature; they belong together.

use crate::app::{Message, State};
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;

/// The labels of the actions revealed under the "Help" menu, in display order.
///
/// "Help" exposes exactly one action — "About" (feature 001, FR-003, FR-004).
pub const HELP_ACTIONS: [&str; 1] = ["About"];

/// The actions under the "Help" menu. See [`HELP_ACTIONS`].
pub fn help_actions() -> &'static [&'static str] {
    &HELP_ACTIONS
}

/// The toolbar's overflow menu, as a floating surface.
///
/// A marker: the menu's contents are the constant above and its position is the toolbar's, so
/// "open" is the whole of its state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HelpMenu;

impl FloatingSurface for HelpMenu {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("help_menu")
    }

    fn layer(&self) -> Layer {
        Layer::Popover
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Popover).cancelled_by(Message::HelpMenuToggled)
    }
}

impl Registered for HelpMenu {
    fn open_in(state: &State) -> Option<Self> {
        state.help_menu_open.then_some(HelpMenu)
    }
}

/// The About dialog, as a floating surface (feature 021, T032).
///
/// Here rather than in a module of its own: "About" is the single action the Help menu offers
/// (feature 001, FR-003), so the menu and the dialog it opens are one feature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AboutDialog;

impl FloatingSurface for AboutDialog {
    fn id(&self) -> SurfaceId {
        SurfaceId::new("about")
    }

    fn layer(&self) -> Layer {
        Layer::Dialog
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::AboutClosed)
    }
}

impl Registered for AboutDialog {
    fn open_in(state: &State) -> Option<Self> {
        state.about_open.then_some(AboutDialog)
    }
}
