//! The worktree Overview (feature 813): a main-pane grid with one card per sidebar location,
//! each showing its type, branch, session counts and card state (research R1, R4).
//!
//! Only the open flag, the card menu (feature 972) and what was read live here. The cards compose the sidebar, sessions, agent requests and
//! pull request readings, so they are built in [`crate::ui::overview::cards`], not by this module
//! (a feature module names no other feature).

use crate::app::Message;
use crate::floating_surface;
use crate::overlay::registry::Registered;
use crate::overlay::SurfaceId;
use micold_core::overview::Activation;
use micold_core::session::SessionLocation;
use micold_core::task_progress::FileProgress;
use micold_core::tokens::motion::duration;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How long the Overview stays up after a card is activated: the ripple's expand time, so the
/// ripple on the activated card is seen before the view closes (feature 971, FR-007).
pub const OVERVIEW_ACTIVATION_DELAY: Duration = Duration::from_millis(duration::MEDIUM_2);

/// What the Overview knows of a ticket's issue title (research R8).
#[derive(Clone, Default, PartialEq, Eq)]
pub enum TitleLookup {
    /// Asked for, not answered yet. The card shows its branch meanwhile.
    #[default]
    Pending,
    /// The title.
    Known(String),
    /// Could not be read (offline, closed, no tooling): the card shows its branch.
    Unavailable,
}

/// A title is the user's text and is never logged (FR-020), so a debug dump leaves it out.
impl std::fmt::Debug for TitleLookup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => f.write_str("Pending"),
            Self::Known(_) => f.write_str("Known(..)"),
            Self::Unavailable => f.write_str("Unavailable"),
        }
    }
}

/// What the Overview remembers: whether it is on screen, the last file reading of each worktree and
/// the issue titles read. The cards are derived per render.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The Overview replaces the main area.
    pub open: bool,
    /// The last file reading per worktree directory name (R9). Cleared on a project switch.
    pub progress: BTreeMap<String, FileProgress>,
    /// The issue title per numeric ticket (R8). Cleared on a project switch.
    pub titles: BTreeMap<u64, TitleLookup>,
    /// The activation accepted and waiting out [`OVERVIEW_ACTIVATION_DELAY`] (feature 971, D3).
    /// While set, further activations, closing and toggling are ignored: the first one stands.
    pub activating: Option<Activation>,
    /// The card menu, when one is open: at most one (feature 972, FR-014).
    pub card_menu: Option<CardMenu>,
    /// The card the keyboard goes back to once a keyboard-opened menu closed or ran an entry
    /// (feature 972, FR-011); drained by the root.
    pub focus_return: Option<SessionLocation>,
}

impl State {
    /// A card was activated: accept it when the Overview is open and no activation is pending.
    /// Returns whether it was accepted, i.e. whether the caller must start the delay timer.
    pub fn request(&mut self, activation: Activation) -> bool {
        if !self.open || self.activating.is_some() {
            return false;
        }
        self.activating = Some(activation);
        true
    }

    /// The delay is over: take the pending activation, once. None when the Overview was left some
    /// other way meanwhile (the activation then has nothing to follow).
    pub fn due(&mut self) -> Option<Activation> {
        let pending = self.activating.take();
        pending.filter(|_| self.open)
    }
}

/// Which gesture opened a card menu (feature 972).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuOrigin {
    /// A secondary click on the card.
    Pointer,
    /// Menu or Shift+F10 on the focused card: the first entry starts highlighted and focus
    /// returns to the card when the menu closes (FR-009, FR-011).
    Keyboard,
}

/// An open card menu (feature 972, data-model).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardMenu {
    /// The target: identity, not a grid index, so the menu follows its card through a reorder.
    pub location: SessionLocation,
    /// The panel's top-left in window pixels, before the render-time clamp.
    pub anchor: (u16, u16),
    /// The gesture that opened it.
    pub origin: MenuOrigin,
    /// The keyboard-highlighted entry; `None` until Up/Down for a pointer-opened menu.
    pub highlighted: Option<usize>,
}

/// The card menu, as a floating surface (feature 972, R2): registering it is how Escape, a scroll
/// beneath it and the row menus' displacement reach it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverviewCardMenu;

impl OverviewCardMenu {
    /// This surface's identity, named by the surfaces it displaces and that displace it.
    pub const ID: SurfaceId = SurfaceId::new("overview_card_menu");
}

floating_surface!(OverviewCardMenu, id: Self::ID, layer: ContextMenu, cancelled_by: Message::Overview(Msg::CardMenuDismissed));

impl Registered for OverviewCardMenu {
    fn open_in(state: &crate::app::State) -> Option<Self> {
        state.overview.card_menu.as_ref().map(|_| OverviewCardMenu)
    }
}

/// What happens to the Overview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// The shortcut or the top-bar button. Does nothing while Settings is open, which cannot yield
    /// to another view without losing or saving its draft (research R2).
    Toggled {
        /// Whether Settings is open.
        settings_open: bool,
    },
    /// Leave the Overview.
    Closed,
    /// A card was activated by pointer, Enter or Space: leave the Overview. Following the
    /// activation (select, start menu, reveal) is the root's business (research R12).
    CardActivated(micold_core::overview::Activation),
    /// A card was pressed, by pointer, Enter or Space: the root accepts it with
    /// [`State::request`] and starts the delay; the Overview stays up meanwhile (feature 971).
    ActivationRequested(micold_core::overview::Activation),
    /// [`OVERVIEW_ACTIVATION_DELAY`] has passed since an accepted activation.
    ActivationDue,
    /// A worktree's files were read (research R9).
    ProgressRead {
        /// The project the read was started in; dropped when it is no longer the active one.
        project: PathBuf,
        /// The worktree's directory name.
        dir: String,
        /// What its files say.
        progress: FileProgress,
    },
    /// These tickets' titles are being fetched: each without an entry is pending (R8).
    TitlesRequested(Vec<u64>),
    /// A ticket's title was read: [`TitleLookup::Known`] or [`TitleLookup::Unavailable`].
    TitleRead {
        /// The project the read was started in; dropped when it is no longer the active one.
        project: PathBuf,
        /// The issue number.
        number: u64,
        /// The answer.
        result: TitleLookup,
    },
    /// The window moved to another project: what was read describes the old one.
    ProjectSwitched,
    /// A card's menu opens at `anchor`, replacing any open one (feature 972, US2.4).
    CardMenuOpened {
        /// The card's location.
        location: SessionLocation,
        /// Where the panel hangs, in window pixels.
        anchor: (u16, u16),
        /// The gesture that opened it.
        origin: MenuOrigin,
    },
    /// The card menu closes without running anything: Escape, an outside click, displacement or
    /// a scroll beneath it.
    CardMenuDismissed,
    /// An entry was chosen: the menu closes, then the root dispatches the inner message, the row
    /// menu's own (research R4).
    CardMenuEntryChosen(Box<Message>),
    /// The card grid scrolled: the root closes what floats over it (FR-012).
    GridScrolled,
}

impl Msg {
    /// The project a read was started in, for the messages that carry one. Issue numbers and
    /// directory names collide across repositories, so the root drops a read whose project is no
    /// longer the active one.
    pub fn read_project(&self) -> Option<&Path> {
        match self {
            Self::ProgressRead { project, .. } | Self::TitleRead { project, .. } => Some(project),
            _ => None,
        }
    }
}

/// The root's entry: the Overview's own [`update`], and for [`Msg::GridScrolled`] the shared
/// scroll-beneath rule, which closes a card menu the grid scrolled beneath (feature 972, FR-012).
pub fn routed(state: &mut crate::app::State, msg: Msg) -> Vec<crate::features::Outcome> {
    let scrolled = matches!(msg, Msg::GridScrolled);
    let outcomes = update(&mut state.overview, msg);
    if scrolled {
        state.dismiss_on_scroll_beneath();
    }
    outcomes
}

/// Apply a message to the Overview's state.
/// An activation that sends the user to a row asks the root to reveal it; nothing else asks.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
    // While an activation waits out its delay the first one stands (feature 971, FR-007): no
    // card menu opens over a card that is about to take the window away (feature 972).
    if state.activating.is_some() {
        match msg {
            Msg::Closed
            | Msg::Toggled { .. }
            | Msg::ActivationRequested(_)
            | Msg::CardMenuOpened { .. } => return Vec::new(),
            _ => {}
        }
    }
    if let Msg::CardActivated(Activation::OfferStart(at) | Activation::RevealOnly(at)) = &msg {
        state.open = false;
        return vec![crate::features::Outcome::LocationRevealed(at.clone())];
    }
    match msg {
        Msg::ProgressRead { dir, progress, .. } => {
            state.progress.insert(dir, progress);
        }
        Msg::TitlesRequested(numbers) => {
            for number in numbers {
                state.titles.entry(number).or_default();
            }
        }
        Msg::TitleRead { number, result, .. } => {
            state.titles.insert(number, result);
        }
        Msg::ActivationRequested(_) | Msg::ActivationDue => {}
        Msg::ProjectSwitched => {
            state.activating = None;
            state.progress.clear();
            state.titles.clear();
            state.card_menu = None;
            state.focus_return = None;
        }
        Msg::CardMenuOpened {
            location,
            anchor,
            origin,
        } => {
            state.card_menu = Some(CardMenu {
                location,
                anchor,
                origin,
                highlighted: (origin == MenuOrigin::Keyboard).then_some(0),
            });
            return vec![crate::features::Outcome::SurfaceOpened(
                OverviewCardMenu::ID,
            )];
        }
        Msg::CardMenuDismissed | Msg::CardMenuEntryChosen(_) => {
            if let Some(menu) = state.card_menu.take() {
                if menu.origin == MenuOrigin::Keyboard {
                    state.focus_return = Some(menu.location);
                }
            }
        }
        // The root answers it with the shared scroll-beneath rule; nothing of the Overview's own.
        Msg::GridScrolled => {}
        Msg::Toggled {
            settings_open: true,
        } => {}
        Msg::Toggled {
            settings_open: false,
        } => state.open = !state.open,
        Msg::Closed | Msg::CardActivated(_) => state.open = false,
    }
    Vec::new()
}

/// The numeric ticket of a worktree directory name; a Jira key or no ticket is `None`.
pub fn numeric_ticket(dir_name: &str) -> Option<u64> {
    micold_core::naming::ticket(dir_name)?.parse().ok()
}

/// The titles to fetch on opening: each numeric ticket of `dirs` with no entry yet, once, in
/// order. None unless the project's issue tooling is GitHub and available (R8, FR-019).
pub fn titles_to_fetch(state: &State, dirs: &[String], github_available: bool) -> Vec<u64> {
    let mut wanted = Vec::new();
    if !github_available {
        return wanted;
    }
    for number in dirs.iter().filter_map(|dir| numeric_ticket(dir)) {
        if !state.titles.contains_key(&number) && !wanted.contains(&number) {
            wanted.push(number);
        }
    }
    wanted
}

/// The card's one line saying what the location is for: the issue title when known, else the
/// branch, else the display name; the project root says so (FR-015).
pub fn summary(
    location: &micold_core::session::SessionLocation,
    display_name: &str,
    branch: Option<&str>,
    title: Option<&TitleLookup>,
) -> String {
    use micold_core::session::SessionLocation;
    match (location, title) {
        (SessionLocation::Default, _) => "Project root".to_string(),
        (_, Some(TitleLookup::Known(title))) => title.clone(),
        _ => branch.unwrap_or(display_name).to_string(),
    }
}

/// Which colour family the badge is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Some card needs action: the `error` family.
    Error,
    /// Some card needs attention: the `warning` family.
    Warning,
}

/// What the Overview button shows beside its label, whether or not the Overview is open
/// (FR-012).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Badge {
    /// The colour family.
    pub tone: Tone,
    /// How many cards are in the worst state.
    pub count: usize,
    /// The words for the tooltip: *N need action* or *N need attention*.
    pub label: String,
}

/// The badge over the cards, or `None` when no card needs the user.
pub fn badge(cards: &[micold_core::overview::CardModel]) -> Option<Badge> {
    use micold_core::overview::{attention, CardState};
    let attention = attention(cards)?;
    let (tone, words) = match attention.state {
        CardState::NeedsAction => (Tone::Error, "need action"),
        _ => (Tone::Warning, "need attention"),
    };
    Some(Badge {
        tone,
        count: attention.count,
        label: format!("{} {words}", attention.count),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use micold_core::session::SessionLocation;

    fn activation(name: &str) -> Activation {
        Activation::OfferStart(SessionLocation::Worktree(name.into()))
    }

    fn open() -> State {
        State {
            open: true,
            ..State::default()
        }
    }

    /// T002: the delay is the ripple's expand token, so the two cannot drift (FR-007).
    #[test]
    fn the_delay_is_the_ripples_expand_time() {
        assert_eq!(OVERVIEW_ACTIVATION_DELAY, Duration::from_millis(300));
        assert_eq!(duration::MEDIUM_2, 300);
    }

    /// T007: an open Overview with nothing pending accepts and stores the activation.
    #[test]
    fn request_accepts_when_open_and_idle() {
        let mut state = open();
        assert!(state.request(activation("a")));
        assert_eq!(state.activating, Some(activation("a")));
        assert!(state.open, "the Overview stays up while the ripple plays");
    }

    /// T007: a closed Overview accepts nothing.
    #[test]
    fn request_is_refused_when_closed() {
        let mut state = State::default();
        assert!(!state.request(activation("a")));
        assert_eq!(state.activating, None);
    }

    /// T007: while one is pending, a second request, Closed and Toggled are ignored; the first
    /// activation's target stands (US3-3).
    #[test]
    fn a_pending_activation_ignores_further_input() {
        let mut state = open();
        assert!(state.request(activation("a")));
        assert!(!state.request(activation("b")));
        update(&mut state, Msg::ActivationRequested(activation("b")));
        update(&mut state, Msg::Closed);
        update(
            &mut state,
            Msg::Toggled {
                settings_open: false,
            },
        );
        assert!(
            state.open,
            "neither Escape nor the shortcut closes it early"
        );
        assert_eq!(state.activating, Some(activation("a")));
    }

    /// Feature 972 with 971: no card menu opens while an activation waits out its delay.
    #[test]
    fn a_pending_activation_opens_no_card_menu() {
        let mut state = open();
        assert!(state.request(activation("a")));
        let outcomes = update(
            &mut state,
            Msg::CardMenuOpened {
                location: SessionLocation::Worktree("b".into()),
                anchor: (10, 10),
                origin: MenuOrigin::Pointer,
            },
        );
        assert!(outcomes.is_empty());
        assert_eq!(state.card_menu, None);
    }

    /// T007: a project switch clears the pending activation.
    #[test]
    fn a_project_switch_clears_the_pending_activation() {
        let mut state = open();
        state.request(activation("a"));
        update(&mut state, Msg::ProjectSwitched);
        assert_eq!(state.activating, None);
        assert_eq!(state.due(), None);
    }

    /// T007: `due` takes the activation once, and the follow-up's own `Closed` is not ignored.
    #[test]
    fn due_takes_the_activation_once() {
        let mut state = open();
        state.request(activation("a"));
        assert_eq!(state.due(), Some(activation("a")));
        assert_eq!(state.due(), None);
        update(&mut state, Msg::Closed);
        assert!(!state.open);
    }

    /// A view that took the main area over inside the delay closed the Overview: nothing follows.
    #[test]
    fn due_is_empty_when_the_overview_was_left_meanwhile() {
        let mut state = open();
        state.request(activation("a"));
        state.open = false;
        assert_eq!(state.due(), None);
        assert_eq!(state.activating, None);
    }

    /// T007: `CardActivated` after `due` closes as before (outcome unchanged, FR-008).
    #[test]
    fn the_activation_after_the_delay_closes_as_before() {
        let mut state = open();
        state.request(activation("a"));
        let a = state.due().expect("pending");
        let outcomes = update(&mut state, Msg::CardActivated(a));
        assert!(!state.open);
        assert_eq!(outcomes.len(), 1, "an OfferStart still reveals its row");
    }
}
