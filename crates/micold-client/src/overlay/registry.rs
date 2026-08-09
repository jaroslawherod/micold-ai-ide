//! The single place a floating surface is registered, and the generic dispatch that reads it
//! (feature 021, Tier 2, T029 — FR-008 – FR-010, contract R1–R3).
//!
//! # What a registration is
//!
//! A [`FloatingSurface`] describes a surface. It does not say where to *find* one: a surface is
//! open or not according to the application state, and only the feature owning it knows which
//! field carries that. [`Registered`] adds exactly that one fact, and a registration is the pair —
//! a probe that looks at [`State`] and hands back an [`Open`] surface, or nothing.
//!
//! Erasing to `Open` at the boundary is what makes dispatch generic. Everything downstream —
//! Escape, scroll, stacking — asks the same three questions of every surface (which one, which
//! band, what closes it) and never learns which surface it is holding. That is the property
//! SC-001 measures: the six central match statements exist only because dispatch currently has to
//! know.
//!
//! # Two dispatch shapes, deliberately
//!
//! [`escape`] goes to the **topmost** surface and no other; [`scroll_beneath`] goes to **every**
//! surface it reaches. That asymmetry is not an oversight, it is the behaviour being preserved:
//! Escape is a single decision aimed at whatever holds the user's attention (contract D1), while
//! scrolling the content behind the window invalidates every anchored menu at once, which is what
//! `State::dismiss_on_scroll_beneath` does today. A single "dismiss" entry point would have had to
//! pick one and silently change the other.
//!
//! # What is registered
//!
//! All sixteen surfaces the application has: nine dialogs, three panel popovers and four context
//! menus, each described in the feature module that owns it and named here once. The transitional
//! `ModalSurface` that stood in for the whole `Overlay` enum at T029 is gone as of T032.
//!
//! The enum itself is not, yet. It survives as the *storage* the dialog surfaces read — their
//! `open_in` is `state.overlay == Overlay::X` — and as the thing `app::on_escape` still matches
//! on. Those are two independent statements of the same nine facts, which is exactly the drift
//! this feature exists to end, and `tests/overlay_registry.rs` holds them equal over every state
//! either can express until T034 deletes the second one.

use crate::app::{Message, State};
use crate::overlay::{DismissalRules, FloatingSurface, SurfaceId};
use micold_core::overlay::{Layer, Trigger};

/// Where generic dispatch finds a surface in the application state.
///
/// The one thing [`FloatingSurface`] cannot say about itself. Implemented beside the surface, so
/// the field a feature opens its surface with stays the feature's own business.
pub trait Registered: FloatingSurface + Sized {
    /// This surface, if the state says it is open.
    fn open_in(state: &State) -> Option<Self>;
}

/// An open surface, as dispatch sees it: identity, band, and what closes it.
///
/// The erased form of a [`Registered`] surface. Owned rather than borrowed because a surface may
/// be assembled from state on the fly, and dispatch has no place to keep it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Open {
    id: SurfaceId,
    layer: Layer,
    dismissal: DismissalRules,
}

/// Erasing a live surface to what dispatch needs — the conversion the chain terminates in.
///
/// A `From` impl rather than an `Open::of` constructor, so this layer ends the way every other
/// builder in the codebase does: `(&surface).into()`, exactly as a component ends in an
/// `iced::Element` (Principle VIII, FR-030). There is no second way to build an `Open`; a surface
/// describes itself through [`FloatingSurface`] and this is the only door out.
impl<S: FloatingSurface> From<&S> for Open {
    fn from(surface: &S) -> Self {
        Self {
            id: surface.id(),
            layer: surface.layer(),
            dismissal: surface.dismissal(),
        }
    }
}

impl Open {
    /// Which surface this is.
    pub fn id(&self) -> SurfaceId {
        self.id
    }

    /// Which band of the z-order it sits in.
    pub fn layer(&self) -> Layer {
        self.layer
    }

    /// The message to send when `trigger` happens, or `None` when it does not close this surface.
    pub fn on(&self, trigger: Trigger) -> Option<&Message> {
        self.dismissal.on(trigger)
    }

    /// The message that closes this surface, whatever prompted the close.
    pub fn cancel(&self) -> Option<&Message> {
        self.dismissal.cancel()
    }
}

/// A registration: "look at the state and tell me whether this surface is open".
pub type Probe = fn(&State) -> Option<Open>;

/// Declare the registered surfaces. **One line per surface, and this is the only such list**
/// (FR-009).
///
/// A macro rather than a plain array so the line is a type name and nothing else: no closure to
/// get subtly wrong, no place for a per-surface special case to be tucked in. Adding a surface
/// that is never named here compiles, which is why `tests/overlay_registry.rs` checks the list
/// against reality — contract R2 is a guard test precisely because the compiler cannot hold it
/// once the enum is gone.
macro_rules! register {
    ($($surface:ty),+ $(,)?) => {
        static REGISTERED: &[Probe] = &[
            $(|state| <$surface as Registered>::open_in(state).map(|s| Open::from(&s))),+
        ];
    };
}

register! {
    crate::features::help::AboutDialog,
    crate::features::help::HelpMenu,
    crate::features::project::ConfirmForgetProjectDialog,
    crate::features::project::ProjectContextMenu,
    crate::features::project::ProjectSelectorDialog,
    crate::features::project::ProjectSwitcher,
    crate::features::project::RenameProjectDialog,
    crate::features::session::ConfirmSessionRemoveDialog,
    crate::features::session::SessionContextMenu,
    crate::features::session::TerminalContextMenu,
    crate::features::settings::SettingsDialog,
    crate::features::sidebar::SidebarFilterPanel,
    crate::features::worktree::ConfirmWorktreeDeleteDialog,
    crate::features::worktree::WorktreeContextMenu,
    crate::features::worktree::RenameWorktreeDialog,
    crate::features::worktree_form::AddWorktreeDialog,
}

/// Every registered surface, in registration order.
///
/// Public so the guard test can reorder it — reordering is the only way to test contract R3, and a
/// property nothing can exercise is a property nobody is holding.
pub fn probes() -> &'static [Probe] {
    REGISTERED
}

/// Every surface `probes` reports open, in registration order.
pub fn open_among(probes: &[Probe], state: &State) -> Vec<Open> {
    probes.iter().filter_map(|probe| probe(state)).collect()
}

/// The surface at the top of the stack, or `None` when nothing is open.
///
/// Selected by band, so the answer does not depend on registration order across bands (R3). Ties
/// within a band resolve to the last registered, matching `micold_core::overlay::stack_order`,
/// which sorts stably and puts the last of a band on top.
pub fn topmost_among(probes: &[Probe], state: &State) -> Option<Open> {
    open_among(probes, state)
        .into_iter()
        .max_by_key(|open| open.layer())
}

/// The surface at the top of the stack, per the registry.
pub fn topmost(state: &State) -> Option<Open> {
    topmost_among(REGISTERED, state)
}

/// What Escape closes: the topmost surface's cancellation, or nothing.
///
/// Topmost and no other — a modal keeps Escape whatever floats above it (contract D1). This is
/// `app::on_escape` expressed generically; T034 replaces that function with this one.
pub fn escape(state: &State) -> Option<Message> {
    topmost(state)?.on(Trigger::Escape).cloned()
}

/// What scrolling the content beneath closes: every open surface the trigger reaches.
///
/// All of them, not just the topmost — a scroll moves the ground under every anchored menu at
/// once, and a dialog above them is unaffected because the core rule says a `Dialog` does not
/// dismiss on this trigger.
pub fn scroll_beneath(state: &State) -> Vec<Message> {
    open_among(REGISTERED, state)
        .iter()
        .filter_map(|open| open.on(Trigger::ScrollBeneath).cloned())
        .collect()
}

/// Every open surface below the dialog band: the lightweight popovers and context menus.
pub fn open_popovers(state: &State) -> Vec<Open> {
    open_among(REGISTERED, state)
        .into_iter()
        .filter(|open| open.layer() < Layer::Dialog)
        .collect()
}

/// Close every open popover, by sending each the cancellation it declared.
///
/// FR-012's second half — opening a modal closes the lightweight popovers — as a rule over the
/// registry rather than a list of field assignments. A popover registered later is closed by it
/// without anyone having remembered to add a line, which is the whole of R2's argument.
pub fn close_popovers(state: &mut State) {
    close_each(state, open_popovers);
}

/// Close every surface a scroll beneath the content reaches.
pub fn close_on_scroll_beneath(state: &mut State) {
    close_each(state, |state| {
        open_among(REGISTERED, state)
            .into_iter()
            .filter(|open| open.on(Trigger::ScrollBeneath).is_some())
            .collect()
    });
}

/// Close surfaces one at a time, re-asking which are open after each.
///
/// Re-asking is not caution, it is required. Several of these cancellations are *toggles*, and the
/// reducer arms behind them close their neighbours too (the three panel popovers are mutually
/// exclusive). Sending a batch collected up front would hand a toggle to a surface that an earlier
/// message had already closed — and reopen it.
///
/// Bounded by the number of registrations, so a surface whose cancellation does not actually close
/// it costs one wasted pass rather than a hang. `every_registered_popover_can_be_closed` in
/// `tests/overlay_registration.rs` is what stops that being reachable.
fn close_each(state: &mut State, open: fn(&State) -> Vec<Open>) {
    for _ in 0..REGISTERED.len() {
        let Some(surface) = open(state).into_iter().next() else {
            return;
        };
        let Some(cancel) = surface.cancel().cloned() else {
            return;
        };
        state.update(cancel);
    }
}
