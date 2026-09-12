//! The Help menu: what it offers, and the surface it opens as (feature 021, T031).
//!
//! A tenth feature module, and the smallest — but FR-001 asks where a feature lives, not how big
//! it is, and the answer for the toolbar's overflow menu was previously "two constants at the top
//! of `app.rs` and a `bool` two hundred lines below them". The actions and the surface that shows
//! them are one feature; they belong together.
//!
//! # The vocabulary this feature declares
//!
//! Three transitions in [`Msg`] — `MenuToggled`, `AboutOpened`, `AboutClosed` — routed by [`update`],
//! which is pure: `&mut State` in, `Vec<Outcome>` out (data-model.md §1.1 shape A). The root's
//! `Message::Help` arm hands over the whole vocabulary and the binary matches none of it a second
//! time, because opening a menu and opening an About dialog reach nothing outside the process.
//!
//! # The state this feature remembers (feature 028, contract S1)
//!
//! Two `bool`s in [`State`], reached as `state.help`: `help_menu_open` — whether the toolbar's
//! overflow menu is showing — and `about_open`, whether the About dialog is. Both keep the names
//! they had flat on the root: `help_menu_open` names the *menu*, a surface inside this feature
//! rather than the feature itself, and `about_open` names the dialog, so neither stutters against
//! the `help` qualifier (T029).
//!
//! That is the whole of it. Every action the menu offers is a message that leaves through
//! [`Outcome`](crate::features::Outcome); nothing about what the menu *contains* is stored.

use crate::app::Message;
use crate::overlay::registry::Registered;
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::Layer;

/// What this feature remembers (feature 028, contract S1).
///
/// The fields keep the names they had as flat members of `app::State`, and the reducers below
/// spell the root's type `crate::app::State` now that `State` here means this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// Whether the About dialog is open.
    ///
    /// The last remnant of the `Overlay` enum (feature 021, T037). That enum was a single slot
    /// naming which of nine dialogs was showing, and eight of the nine already had a field of
    /// their own saying the same thing — `project.selector`, `project.rename_draft`,
    /// `worktree_form.form`, `settings.settings_draft`, and the four confirm-dialog targets. The slot was a second copy of a fact
    /// the state already held, kept in step by hand at twenty-five reducer sites. Each dialog now
    /// reads its own state, and About, which had none, gets this.
    ///
    /// **What the enum bought and this does not**: two dialogs open at once was unrepresentable
    /// (FR-015, Principle V). That invariant now belongs to [`crate::app::State::clear_for_dialog`], which
    /// closes whatever dialog is open before the next one is set up, and to
    /// `one_dialog_at_a_time` in `tests/overlay_registry.rs`. A mechanism where there was a type;
    /// recorded rather than glossed.
    pub about_open: bool,
    /// Whether the Help menu is currently expanded (transient UI affordance).
    pub help_menu_open: bool,
}

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

impl HelpMenu {
    /// This surface's identity, nameable by the surfaces that displace it or that it
    /// displaces (T067a-2). The declaration has to point at something, and pointing at the
    /// literal string in two places is how the two would come to disagree.
    pub const ID: SurfaceId = SurfaceId::new("help_menu");
}

impl FloatingSurface for HelpMenu {
    fn id(&self) -> SurfaceId {
        Self::ID
    }

    fn layer(&self) -> Layer {
        Layer::Popover
    }

    fn dismissal(&self) -> DismissalRules {
        DismissalRules::for_layer(Layer::Popover).cancelled_by(Message::Help(Msg::MenuToggled))
    }
}

impl Registered for HelpMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.help.help_menu_open.then_some(HelpMenu)
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
        DismissalRules::for_layer(Layer::Dialog).cancelled_by(Message::Help(Msg::AboutClosed))
    }
}

impl Registered for AboutDialog {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.help.about_open.then_some(AboutDialog)
    }
}

/// What can happen to the Help menu and the dialog it opens (feature 028, FR-001).
///
/// # The variants kept their meaning and lost their prefix
///
/// `Message::Help(HelpMsg::MenuToggled)` is `Msg::MenuToggled` here — the type says which menu, so the variant
/// does not have to (contract M1). `AboutOpened` and `AboutClosed` keep their names: "About" is the
/// single action this menu offers, not a restatement of the feature's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The "Help" toolbar entry was selected (reveals/collapses its "About" action).
    MenuToggled,
    /// The "About" action was activated.
    AboutOpened,
    /// The About dialog was dismissed (Close button or Esc).
    AboutClosed,
}

/// This feature's whole reducer surface: one entry point, shape A (contract M2).
///
/// Pure — nothing here reaches the filesystem or the daemon — so the root's arm is a `drain` over
/// what comes back and nothing else.
pub fn update(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    match msg {
        Msg::MenuToggled => return menu_toggled(state),
        Msg::AboutOpened => about_opened(state),
        Msg::AboutClosed => about_closed(state),
    }
    Vec::new()
}

/// The overflow menu was toggled (feature 021, T062 — FR-004a).
///
/// It writes its own field and reports that the menu opened. What that closes — the other two
/// panel popovers and the project row menu (features 009 and 015) — is declared on this surface's
/// registration line and applied by `overlay::registry::displace`, because a rule about which
/// surfaces exclude each other is owned by neither of the two it relates (T067a-2).
#[must_use = "what an opening popover displaces is the registry's business, not the caller's"]
pub fn menu_toggled(state: &mut crate::app::State) -> Vec<crate::features::Outcome> {
    state.help.help_menu_open = !state.help.help_menu_open;
    crate::features::surface_opened(state.help.help_menu_open, HelpMenu::ID)
}

/// The About dialog was opened (feature 001, FR-011).
///
/// Idempotent: opening while already open keeps a single instance (FR-015).
pub fn about_opened(state: &mut crate::app::State) {
    state.clear_for_dialog();
    state.help.about_open = true;
}

/// The About dialog was dismissed (feature 001, FR-012).
///
/// A no-op when nothing is open (edge case); otherwise the main window is unchanged.
pub fn about_closed(state: &mut crate::app::State) {
    state.help.about_open = false;
}
