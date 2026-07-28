//! The showcase's render-free reducer (feature 020, T008).
//!
//! Everything the showcase can be asked to do is here, and none of it renders. That split is what
//! lets [`super::gallery`] be the thin glue Principle I's exception covers: a state transition that
//! lived in the view would be untested production code, so none does.
//!
//! There is no lifecycle to model. Every message is a toggle or a counter bump, nothing is
//! asynchronous, nothing can fail, and no field depends on another. That is a property rather than a
//! gap — it is what makes "the same content in the same order on every launch" (FR-022, SC-010) true
//! by construction.
//!
//! **No clock.** A replay is a changed identity, not a timeline: bumping a counter hands the
//! component a different `restart_on(key)`, and the component plays its own transition and asks for
//! its own frames (feature 017's `Progress`). So the showcase holds no timer, no subscription that
//! ticks, and no animation clock — which is what keeps FR-023's "no frames at rest" literally true
//! rather than approximately true.

use micold_core::theme::ColorScheme;

use crate::grid::GridCache;

/// Which floating surface is open.
///
/// One variant per floating *surface* in the library — the triggers that open them are ordinary
/// controls and are not listed. Deliberately `Copy` and deliberately singular: see [`Showcase::open`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Floating {
    /// A modal dialog (`material::Modal`).
    Modal,
    /// The overflow menu panel (`material::MenuOverlay`).
    Menu,
    /// A cursor-anchored context menu (`material::ContextMenu`).
    ContextMenu,
    /// The project switcher panel (`material::ProjectSwitcherOverlay`).
    ProjectSwitcher,
}

/// Everything a developer can ask the showcase to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Message {
    /// Switch between the light and the dark scheme, applying to every component on the page
    /// (FR-008).
    SchemeToggled,
    /// Play entry `i`'s transition again, from the start (FR-007b).
    Replayed(usize),
    /// Send entry `i` the other way, so its exit can be watched too.
    Reversed(usize),
    /// Start or stop entry `i`, for a component whose appearance runs continuously (FR-023a).
    RunToggled(usize),
    /// Open a floating surface from its own section (FR-007).
    Opened(Floating),
    /// Close whatever floating surface is open.
    Dismissed,
    /// A component in the gallery emitted a message. Components here are real and interactive
    /// (FR-002), so their messages have to go somewhere; a catalogue has no behaviour to invoke, so
    /// they go nowhere. Swallowing them is what keeps the instance genuinely live without the gallery
    /// inventing behaviour the application owns.
    NoOp,
}

/// The showcase's whole state.
///
/// `Clone` and `Debug`, but not `PartialEq`: it carries the sample [`GridCache`], which is a render
/// cache rather than a value. Tests compare the fields they care about.
#[derive(Debug, Clone)]
pub struct Showcase {
    /// The scheme every component on the page resolves against (FR-008). Starts light, and is never
    /// read from the host's theme preference or from the settings file (FR-009, FR-020).
    pub scheme: ColorScheme,
    /// Which floating surface is open, if any (FR-007).
    ///
    /// An `Option`, not a set. The spec's Edge Case — "two floating components could be opened at
    /// once … must not deadlock itself into a state where a surface cannot be dismissed and the page
    /// cannot be scrolled" — is answered by making that state unrepresentable rather than by
    /// defending against it (Principle V). Opening a second surface replaces the first.
    pub open: Option<Floating>,
    /// Per-entry generation counter. A changed value replays that entry's transition.
    replays: Vec<u64>,
    /// Per-entry run state (FR-023a). All `false` at rest; no entry uses it at delivery, because
    /// nothing in the library runs continuously yet.
    running: Vec<bool>,
    /// Per-entry destination, so an exit is watchable as well as an entrance. Starts `true` — an
    /// element built already-open must not animate into existence.
    shown: Vec<bool>,
    /// The fabricated terminal grid `TerminalPane` renders from, so a component that would otherwise
    /// need a live session is present rather than omitted (FR-006).
    grid: GridCache,
}

impl Showcase {
    /// A showcase for a gallery of `entries` entries, at rest.
    ///
    /// The per-entry vectors are sized here — the call site passes `catalogue::COMPONENTS.len()` —
    /// rather than being fixed-length arrays whose length is const-evaluated from the catalogue,
    /// which would ask the compiler to resolve the catalogue and this type through each other
    /// (`Entry::render` names `&Showcase`). Taking the count as an argument also keeps the reducer
    /// independent of the catalogue, so a test can drive four entries without declaring four.
    pub fn new(entries: usize) -> Self {
        Self {
            scheme: ColorScheme::Light,
            open: None,
            replays: vec![0; entries],
            running: vec![false; entries],
            shown: vec![true; entries],
            grid: super::samples::grid(),
        }
    }

    /// How many entries this state was built for.
    pub fn len(&self) -> usize {
        self.replays.len()
    }

    /// Whether the gallery is empty — which the completeness check makes impossible, but which a
    /// caller reading `!is_empty()` states more clearly than `len() > 0`.
    pub fn is_empty(&self) -> bool {
        self.replays.is_empty()
    }

    /// Entry `i`'s replay generation. A component hands this to `.restart_on(...)`, so a change
    /// replays its transition from the start.
    pub fn replays(&self, i: usize) -> u64 {
        self.replays.get(i).copied().unwrap_or(0)
    }

    /// Whether entry `i` has been started (FR-023a).
    pub fn running(&self, i: usize) -> bool {
        self.running.get(i).copied().unwrap_or(false)
    }

    /// Where entry `i` is heading — shown, or on its way out.
    pub fn shown(&self, i: usize) -> bool {
        self.shown.get(i).copied().unwrap_or(true)
    }

    /// The fabricated terminal grid (FR-006).
    pub fn grid(&self) -> &GridCache {
        &self.grid
    }

    /// What the scheme control says it will do, which is the opposite of the scheme in force.
    ///
    /// Here rather than in the view because it is a decision about state, and Principle I's
    /// GUI-wiring exception covers glue that makes none. `tests/showcase_glue.rs` caught the first
    /// draft of the gallery deciding it inline — which is what that gate is for.
    pub fn scheme_control_label(&self) -> &'static str {
        match self.scheme {
            ColorScheme::Light => "Switch to dark",
            ColorScheme::Dark => "Switch to light",
        }
    }

    /// Apply a message.
    ///
    /// An index that is out of range is ignored rather than fatal: the gallery indexes by catalogue
    /// position, and a stale index is the shape a reordered catalogue would take. A showcase that
    /// panicked on one would be less useful than the navigation it replaced.
    pub fn update(&mut self, message: Message) {
        match message {
            Message::SchemeToggled => {
                self.scheme = match self.scheme {
                    ColorScheme::Light => ColorScheme::Dark,
                    ColorScheme::Dark => ColorScheme::Light,
                }
            }
            Message::Replayed(i) => {
                if let Some(count) = self.replays.get_mut(i) {
                    *count = count.wrapping_add(1);
                }
                // Replay plays the *entrance*, so an entry someone had reversed is pointed forward
                // again first. Otherwise the same control would do different things depending on
                // history the developer cannot see.
                if let Some(shown) = self.shown.get_mut(i) {
                    *shown = true;
                }
            }
            Message::Reversed(i) => {
                if let Some(shown) = self.shown.get_mut(i) {
                    *shown = !*shown;
                }
            }
            Message::RunToggled(i) => {
                if let Some(running) = self.running.get_mut(i) {
                    *running = !*running;
                }
            }
            Message::Opened(surface) => self.open = Some(surface),
            Message::Dismissed => self.open = None,
            Message::NoOp => {}
        }
    }
}
