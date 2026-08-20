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
//! `ModalSurface` that stood in for the whole `Overlay` enum at T029 is gone as of T032, and the
//! enum itself as of T037.
//!
//! It went in three steps, because it was doing three things. T033 deleted its *description* of
//! the dialogs — `Overlay::as_surface`, the nine-arm bridge T029 built so the two representations
//! could not answer differently while they coexisted — leaving `app::on_escape` as one call into
//! [`escape`] below. T034–T036 collapsed the sites that matched on the remaining slot. T037
//! removed the slot: eight of the nine dialogs already had a field of their own saying they were
//! open, so their `open_in` reads that, and About, which had none, gained `State::about_open`.
//!
//! What holds the nine facts now is `tests/overlay_registry.rs`, which states each dialog's
//! identity and cancellation in an exhaustive match of its own and checks dispatch against it.
//! That is a stronger check than the equality it replaces: the expectation is written
//! independently of the code under test rather than read out of it.

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
#[derive(Debug, Clone)]
pub struct Open {
    id: SurfaceId,
    layer: Layer,
    dismissal: DismissalRules,
    displaces: &'static [SurfaceId],
    view: Option<crate::ui::DialogView>,
}

/// Two open surfaces are the same one when they are the same surface — identity, band and
/// dismissal. The view is deliberately not part of it.
///
/// Hand-written rather than derived because `view` is a function pointer, and comparing those
/// compares addresses the compiler does not promise are unique: the same function can have two,
/// and two functions can share one. So a derived `PartialEq` would be answering a question nobody
/// asked with a result nobody can rely on. Which view a surface is drawn by is not part of what
/// makes it that surface anyway — `drawn_by` attaches it at registration.
impl PartialEq for Open {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.layer == other.layer
            && self.dismissal == other.dismissal
            && self.displaces == other.displaces
    }
}

impl Eq for Open {}

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
            displaces: &[],
            view: None,
        }
    }
}

impl Open {
    /// Pair this surface with the view that draws it — the optional step in the chain (T035).
    ///
    /// Set from the registration line rather than from the surface, because a surface is a
    /// feature-module type and FR-006 forbids one naming a rendering framework. The two halves
    /// are still named in one place, which is what FR-009 is about; they simply cannot be named
    /// in the same *module*.
    ///
    /// Only the dialog band has one. A popover is drawn by the panel it hangs off, which already
    /// outlives the flag that opened it so that it has something to fade out.
    pub fn drawn_by(mut self, view: crate::ui::DialogView) -> Self {
        self.view = Some(view);
        self
    }

    /// Declare what this surface closes by opening — the other optional step in the chain
    /// (T067a-2).
    ///
    /// Set from the registration line for the same reason [`Self::drawn_by`] is, and a stronger
    /// one. Displacement is a fact about the *relation between* surfaces, so it belongs to neither
    /// of the two surfaces it relates; and `tests/surface_registration_cost.rs` holds that a
    /// surface may be named only in its own module and here, which a feature module declaring what
    /// it closes would break five times over. This file is already the one place every surface is
    /// named, which is exactly what a relation over them needs.
    pub fn displacing(mut self, displaces: &'static [SurfaceId]) -> Self {
        self.displaces = displaces;
        self
    }

    /// The view registered for this surface, if any.
    pub fn view(&self) -> Option<crate::ui::DialogView> {
        self.view
    }

    /// Which surface this is.
    pub fn id(&self) -> SurfaceId {
        self.id
    }

    /// Which band of the z-order it sits in.
    pub fn layer(&self) -> Layer {
        self.layer
    }

    /// The surfaces this one closes by opening.
    pub fn displaces(&self) -> &'static [SurfaceId] {
        self.displaces
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

/// A dialog on its way out: which surface it was, and the state it was drawn from.
///
/// # Why a whole `State`, and why there is nothing per-surface here
///
/// Closing a dialog clears its draft synchronously in the reducer, so the exit animation would
/// have nothing left to draw. Something has to outlive the close (FR-011), and until T036 that was
/// `ClosingOverlay` — a nine-variant enum, one per dialog, each carrying a hand-picked clone of
/// exactly the fields its renderer needed, plus a match mapping each back to the dialog it was a
/// snapshot of. Three per-surface lists for one idea.
///
/// The idea is just "the state as it was". Keeping that literally makes all three lists vanish:
/// the snapshot draws through [`Open::view`], the same function the live path calls, so the exit
/// really is the enter in reverse rather than a second renderer that resembles it. This is why
/// nothing was added to [`FloatingSurface`] despite T036's brief — the snapshot turned out not to
/// be per-surface behaviour at all, so a `snapshot` method would have been a hook with one uniform
/// implementation.
///
/// The cost is a `State` clone where there used to be a draft clone, taken before every message
/// **while a dialog is open** — the same gate the old capture had, so nothing is cloned in the
/// common case of no dialog on screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Closing {
    id: SurfaceId,
    state: Box<State>,
}

impl Closing {
    /// Snapshot the open dialog, or `None` when no dialog is open.
    pub fn of(state: &State) -> Option<Self> {
        Some(Self {
            id: open_dialog(state)?.id(),
            state: Box::new(state.clone()),
        })
    }

    /// Which dialog this is a snapshot of.
    ///
    /// The identity `material::Modal` keys its transition on. It has to survive the close: the
    /// live state says nothing is open the instant the dialog is dismissed, and a renderer reading
    /// *that* would see the identity change, restart the transition, and make the dialog vanish
    /// instead of animating out.
    pub fn id(&self) -> SurfaceId {
        self.id
    }

    /// The dialog as the registry sees it in the snapshot — including the view that draws it.
    pub fn surface(&self) -> Option<Open> {
        open_dialog(&self.state)
    }

    /// The state as it was when the dialog was still open.
    pub fn state(&self) -> &State {
        &self.state
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
///
/// # The two optional clauses, and why displacement is one of them
///
/// `=> view` pairs a dialog with the function that draws it (T035); `{ displaces: … }` names what
/// a surface closes by opening (T067a-2). Both are facts a surface cannot state about itself —
/// the first because FR-006 forbids a feature module naming a rendering framework, the second
/// because displacement is a fact about the *relation between* two surfaces and belongs to
/// neither. `tests/surface_registration_cost.rs` makes that concrete: a surface may be named only
/// in its own module and here, so a feature module declaring what it displaces would break the
/// cost guarantee five times over.
///
/// Until T067a-2 the relation was not written down anywhere. Five toggle reducers each assigned
/// their neighbours' fields — twelve cross-feature writes — and reading the rule meant opening
/// five files and comparing them. It is not a uniform rule and never was, which is why it is
/// declared per surface rather than derived from the band:
/// `tests/popover_displacement.rs` states all forty-two ordered pairs independently.
macro_rules! register {
    ($($surface:ty $(=> $view:path)? $({ displaces: $($displaced:ty),+ $(,)? })?),+ $(,)?) => {
        static REGISTERED: &[Probe] = &[
            $(|state| <$surface as Registered>::open_in(state)
                .map(|s| Open::from(&s)
                    $(.drawn_by($view))?
                    $(.displacing(&[$(<$displaced>::ID),+]))?)),+
        ];
    };
}

register! {
    crate::features::help::AboutDialog => crate::ui::about::dialog,
    crate::features::help::HelpMenu {
        displaces:
            crate::features::project::ProjectSwitcher,
            crate::features::sidebar::SidebarFilterPanel,
            crate::features::project::ProjectContextMenu,
    },
    crate::features::project::ConfirmForgetProjectDialog => crate::ui::confirm_forget::dialog,
    // The switcher is deliberately absent: this menu is opened by right-clicking a row *inside*
    // the open switcher, and the row list has to stay visible behind it. The same fact
    // `ProjectContextMenu::layer` states about the z-order, stated here about displacement, and
    // pinned by `tests/switcher_forget_menu.rs`.
    crate::features::project::ProjectContextMenu {
        displaces:
            crate::features::help::HelpMenu,
            crate::features::sidebar::SidebarFilterPanel,
            crate::features::worktree::WorktreeContextMenu,
    },
    crate::features::project::ProjectSelectorDialog => crate::ui::project_selector::dialog,
    crate::features::project::ProjectSwitcher {
        displaces:
            crate::features::help::HelpMenu,
            crate::features::sidebar::SidebarFilterPanel,
            crate::features::project::ProjectContextMenu,
    },
    crate::features::project::RenameProjectDialog => crate::ui::rename::dialog,
    crate::features::session::ConfirmSessionRemoveDialog => crate::ui::confirm_session_remove::dialog,
    crate::features::session::SessionContextMenu,
    crate::features::session::TerminalContextMenu,
    crate::features::session::ShellInstanceMenu,
    crate::features::settings::SettingsDialog => crate::ui::settings_form::dialog,
    crate::features::sidebar::SidebarFilterPanel {
        displaces:
            crate::features::help::HelpMenu,
            crate::features::project::ProjectSwitcher,
            crate::features::project::ProjectContextMenu,
    },
    crate::features::worktree::ConfirmWorktreeDeleteDialog => crate::ui::confirm_delete::dialog,
    // The project row menu and nothing else: the two context menus replace each other, and a panel
    // popover open elsewhere in the window is unaffected by a right-click in the sidebar.
    crate::features::worktree::WorktreeContextMenu {
        displaces: crate::features::project::ProjectContextMenu,
    },
    crate::features::worktree::RenameWorktreeDialog => crate::ui::worktree_rename::dialog,
    crate::features::worktree_form::AddWorktreeDialog => crate::ui::worktree_form::dialog,
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
/// Topmost and no other — a modal keeps Escape whatever floats above it (contract D1). As of T033
/// this *is* `app::on_escape`, which delegates here; T034 brings the keyboard subscription's
/// hand-written mirror of it onto the same call and retires the wrapper.
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

/// The open dialog, if there is one: the surface in the modal band.
///
/// The subject of the view's old ten-arm match (T035). It answers *which* dialog rather than
/// drawing it, because the caller needs both halves — the body to draw, and the identity to key
/// the transition on, so switching straight from one dialog to another replays the entrance
/// instead of inheriting a transition the previous one had already finished.
///
/// At most one is open, and since T037 that is a rule rather than a type: the `Overlay` enum was a
/// single slot, so a second dialog could not be represented; each dialog now says it is open by
/// holding the state it draws from, and two of those could in principle be set at once.
/// [`crate::app::State::clear_for_dialog`] is what keeps it from happening, and
/// `one_dialog_at_a_time` in `tests/overlay_registry.rs` is what would notice if it did.
/// `max_by_key` rather than `find` because it never depended on the slot in the first place —
/// registration order decides nothing (contract R3).
pub fn open_dialog(state: &State) -> Option<Open> {
    open_among(REGISTERED, state)
        .into_iter()
        .filter(|open| open.layer() == Layer::Dialog)
        .max_by_key(|open| open.layer())
}

/// Every open dialog: the surfaces in the modal band. Normally none or one — see [`open_dialog`].
pub fn open_dialogs(state: &State) -> Vec<Open> {
    open_among(REGISTERED, state)
        .into_iter()
        .filter(|open| open.layer() == Layer::Dialog)
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

/// Close any open dialog, by sending it the cancellation it declared.
///
/// The invariant the `Overlay` enum used to hold in the type system (T037): opening a dialog
/// closes whichever was open. Expressed the same way [`close_popovers`] is — as a rule over the
/// registry, so a dialog added later is closed by it without a line being added anywhere.
pub fn close_dialogs(state: &mut State) {
    close_each(state, open_dialogs);
}

/// Close whatever Escape reaches: the topmost open surface, and nothing else.
///
/// [`escape`] answers the question; this one acts on the answer, for the caller that has a
/// `&mut State` rather than a message to return. `Message::EscapePressed` is that caller.
pub fn close_topmost(state: &mut State) {
    if let Some(cancel) = escape(state) {
        state.update(cancel);
    }
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
/// Re-asking is not caution, it is required. Several of these cancellations are *toggles*, and
/// sending one to a surface that an earlier message already closed would **open** it. That was
/// reachable before T067a-2 because the toggle reducers assigned their neighbours' fields, and it
/// still is: a toggle now reports `Outcome::SurfaceOpened` and [`displace`] closes what that
/// surface displaces, so the batch a caller collected up front can still be stale by the time it
/// is sent. The mechanism moved; the hazard did not.
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

/// Close whatever the surface `id` displaces, now that it has opened (T067a-2).
///
/// The other half of `Outcome::SurfaceOpened`. A feature reports that its surface opened; which
/// *other* surfaces that closes is the registry's business, because it is a fact about the
/// relation between surfaces and no single feature owns a relation. Until T067a-2 the five toggle
/// reducers each assigned their neighbours' fields directly — twelve cross-feature writes, and a
/// rule that could only be read by opening five files and comparing them.
///
/// Each displaced surface is closed through the cancellation it declared, exactly as [`dismiss`]
/// does, so a surface still closes only one way. Nothing cascades: a cancellation that reopens
/// nothing has nothing to displace, and the surfaces here are closing rather than opening.
///
/// A surface that is not open is silently skipped, and an unopened `id` displaces nothing —
/// the same tolerance [`dismiss`] has, and for the same reason.
pub fn displace(state: &mut State, id: SurfaceId) {
    let Some(displaced) = open_among(probes(), state)
        .into_iter()
        .find(|open| open.id() == id)
        .map(|open| open.displaces())
    else {
        return;
    };
    for target in displaced {
        dismiss(state, *target);
    }
}

/// Close one surface by id, if it is open (feature 021, T065 — `Outcome::OverlayDismissed`).
///
/// The registry is what knows *how* a surface closes — its cancellation message — so a feature
/// that has decided a surface should go names it and stops there (contract O2). Unknown or
/// already-closed ids are silently fine: an outcome describes a consequence, and a consequence
/// that has already happened is not an error.
pub fn dismiss(state: &mut State, id: SurfaceId) {
    let Some(cancel) = open_among(probes(), state)
        .into_iter()
        .find(|open| open.id() == id)
        .and_then(|open| open.cancel().cloned())
    else {
        return;
    };
    state.update(cancel);
}
