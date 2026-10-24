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
use micold_core::overview_expansion::{Event, Expansion};
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

/// What the Overview knows of the active project's GitHub repository (feature 1122, research R2):
/// the issue links need it at render, so it is read once per project and kept here.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum RepoLookup {
    /// Not asked for yet in this project.
    #[default]
    Unknown,
    /// Asked for; the daemon has not answered.
    Asked,
    /// The repository the daemon's remotes name, or `None` when none is on GitHub.
    Known(Option<micold_core::github::GithubRepo>),
}

/// What the Overview remembers: whether it is on screen, the last file reading of each worktree and
/// the issue titles read. The cards are derived per render.
/// Where the keyboard goes back to after a keyboard-opened card menu (feature 972, FR-011): one
/// value, so the card and the "to its link" flag are armed and cleared together (review A F4, M4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusReturn {
    /// The card.
    pub location: SessionLocation,
    /// To this link of the card, not the card: a menu opened from that link closed without a
    /// choice (feature 1122, FR-009; feature 1123, any of the card's links).
    pub to_link: Option<micold_core::overview_expansion::CardLink>,
}

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
    /// Where the keyboard goes back to once a keyboard-opened menu closed or ran an entry
    /// (feature 972, FR-011); drained by the root.
    pub focus_return: Option<FocusReturn>,
    /// The card the keyboard was just handed back to. While it is still listed nothing happens;
    /// if it goes (a delete that finishes after its dialog closed) the keyboard moves to the
    /// first card instead of being lost. Cleared by any other focus move (feature 972, FR-011).
    pub focus_landed: Option<SessionLocation>,
    /// Full-screen mode (feature 970, plan D1): the sidebar retracts and the Overview takes the
    /// window's width. Read only through [`State::full_screen`], which gates it on `open`, so every
    /// way out of the Overview leaves the mode with no code of its own. Not persisted (FR-001).
    pub full_screen: bool,
    /// Which card is expanded, and the pointer, focus and menu facts that decide the next one
    /// (feature 1121, data-model *Expansion*). Reset whenever the Overview closes
    /// ([`crate::ui::overview::settle_card_menu`]) and on a project switch.
    pub expansion: Expansion,
    /// The project's GitHub repository, for the cards' issue links (feature 1122, R2). Forgotten on
    /// a project switch.
    pub repo: RepoLookup,
}

impl State {
    /// Whether full-screen mode is on: the Overview is open and the mode was entered since it
    /// opened. The only reader of the field (data-model).
    pub fn full_screen(&self) -> bool {
        self.open && self.full_screen
    }

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
    /// Menu or Shift+F10 on one of a card's focused links (feature 1122, FR-009; feature 1123):
    /// as [`Self::Keyboard`], except that closing the menu without a choice returns the keyboard
    /// to that link.
    KeyboardFromLink(micold_core::overview_expansion::CardLink),
}

impl MenuOrigin {
    /// Opened from the keyboard, on the card or on its link: the first entry starts highlighted
    /// and the keyboard is handed back on close (feature 972, FR-009, FR-011).
    pub fn is_keyboard(self) -> bool {
        matches!(self, Self::Keyboard | Self::KeyboardFromLink(_))
    }
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
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    /// The shortcut or the top-bar button. Does nothing while Settings is open, which cannot yield
    /// to another view without losing or saving its draft (research R2).
    Toggled {
        /// Whether Settings is open.
        settings_open: bool,
    },
    /// The Overview header's *Full screen* button or its chord: enter or leave full-screen mode
    /// (feature 970, FR-001). Ignored while the Overview is closed, while Settings is open and while
    /// an activation is pending.
    FullScreenToggled {
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
    /// Menu or Shift+F10 on a focused card: the root measures the card, then opens its menu from
    /// the keyboard (feature 972, FR-009). Changes no state of its own.
    CardMenuRequested(SessionLocation),
    /// Menu or Shift+F10 on one of a card's focused links, its issue link or a row's number
    /// (feature 1122, FR-009, R9; feature 1123): the root measures that link and opens the card's
    /// menu with [`MenuOrigin::KeyboardFromLink`]. Changes no state of its own.
    LinkMenuRequested(SessionLocation, micold_core::overview_expansion::CardLink),
    /// Up or Down in the open card menu: the highlight steps by one and wraps (FR-010). `items`
    /// is the menu's entry count, known to the root and not to the key listener, which sends 0 and
    /// has the root fill it in; with 0 the highlight does not move.
    CardMenuMoved {
        /// Down was pressed (else Up).
        forward: bool,
        /// How many entries the menu holds.
        items: usize,
    },
    /// Enter or Space in the open card menu: the root runs the highlighted entry (FR-010). Changes
    /// no state of its own: the entry's own message closes the menu.
    CardMenuActivated,
    /// The card menu closes without running anything: Escape, an outside click, displacement or
    /// a scroll beneath it.
    CardMenuDismissed,
    /// An entry was chosen: the menu closes, then the root dispatches the inner message, the row
    /// menu's own (research R4).
    CardMenuEntryChosen(Box<Message>),
    /// The card grid scrolled: the root closes what floats over it (FR-012).
    GridScrolled,
    /// A zoom moved the card at the top of the view, and the grid asks for this vertical scroll
    /// offset, in pixels, to keep it there (feature 970, FR-013). The root scrolls the grid;
    /// nothing of the Overview's own changes. A bare `f32` because a feature module names no
    /// rendering type.
    Reflowed(f32),
    /// Something that decides which card is expanded happened: a pointer rest or leave, a focus
    /// move, Escape, the inner scroll of the expanded card (feature 1121, contract U4).
    Expansion(Event),
    /// The active project's remotes were read: `repo` is the GitHub repository they name, if any
    /// (feature 1122, R2).
    RepoRead {
        /// The project the read was started in; dropped when it is no longer the active one.
        project: PathBuf,
        /// The repository, or `None` when no remote is on GitHub.
        repo: Option<micold_core::github::GithubRepo>,
    },
    /// A card's issue link was clicked (feature 1122, FR-004): the root opens the address it
    /// recomputes from state. Changes no state of its own.
    IssueLinkActivated(SessionLocation),
    /// A pull request row's number was clicked on the card at the location (feature 1123, R8):
    /// the root opens the address it looks up for that number in what the window holds now. It
    /// carries no address, so its `Debug` prints none (FR-014). Changes no state of its own.
    PullRequestLinkActivated(SessionLocation, u64),
    /// An arrow key on the Overview (feature 1122, R9): the root moves the keyboard to the next
    /// (`forward`) or previous card, never to a link.
    CardStepped {
        /// Right or Down; `false` for Left or Up.
        forward: bool,
    },
    /// Page Down (`down`) or Page Up on the focused, expanded card whose body can move that way
    /// (feature 1121, FR-008, contract K4): the root scrolls the body by one visible height.
    CardPaged {
        /// Page Down; `false` for Page Up.
        down: bool,
    },
}

// `Reflowed` holds a float, so `Eq` cannot be derived; the offset is a finite scroll position
// (never NaN), so equality is reflexive, which is all `Eq` promises. `Message` and the overlay
// registry need it.
impl Eq for Msg {}

impl Msg {
    /// The project a read was started in, for the messages that carry one. Issue numbers and
    /// directory names collide across repositories, so the root drops a read whose project is no
    /// longer the active one.
    pub fn read_project(&self) -> Option<&Path> {
        match self {
            Self::ProgressRead { project, .. }
            | Self::TitleRead { project, .. }
            | Self::RepoRead { project, .. } => Some(project),
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
            | Msg::FullScreenToggled { .. }
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
            state.focus_landed = None;
            state.expansion = Expansion::default();
            state.repo = RepoLookup::Unknown;
        }
        Msg::CardMenuOpened {
            location,
            anchor,
            origin,
        } => {
            state.focus_landed = None;
            // The target keeps its form and no other card expands while the menu is open (FR-012).
            state.expansion.apply(Event::MenuOpened);
            state.card_menu = Some(CardMenu {
                location,
                anchor,
                origin,
                highlighted: origin.is_keyboard().then_some(0),
            });
            return vec![crate::features::Outcome::SurfaceOpened(
                OverviewCardMenu::ID,
            )];
        }
        Msg::CardMenuDismissed | Msg::CardMenuEntryChosen(_) => {
            let chosen = matches!(msg, Msg::CardMenuEntryChosen(_));
            state.expansion.apply(Event::MenuClosed);
            if let Some(menu) = state.card_menu.take() {
                if menu.origin.is_keyboard() {
                    // Without a choice, a menu opened from a link hands the keyboard back to
                    // that link (feature 1122, FR-009); an entry's hand-back is the card's (972).
                    let to_link = match menu.origin {
                        MenuOrigin::KeyboardFromLink(link) if !chosen => Some(link),
                        _ => None,
                    };
                    state.focus_return = Some(FocusReturn {
                        location: menu.location,
                        to_link,
                    });
                }
            }
        }
        // The root answers it with the shared scroll-beneath rule; nothing of the Overview's own.
        Msg::GridScrolled => {}
        Msg::Expansion(event) => state.expansion.apply(event),
        // The root pages the body (a task, not state).
        Msg::CardPaged { .. } => {}
        Msg::RepoRead { repo, .. } => state.repo = RepoLookup::Known(repo),
        // The root opens the address (a task, not state).
        Msg::IssueLinkActivated(_) | Msg::PullRequestLinkActivated(..) => {}
        // The root moves the keyboard; like any other focus move it ends the hand-back watch.
        Msg::CardStepped { .. } => state.focus_landed = None,
        // The root scrolls the grid to the offset (a task, not state).
        Msg::Reflowed(_) => {}
        // The root answers these two with the measured card and the highlighted entry.
        Msg::CardMenuRequested(_) | Msg::LinkMenuRequested(..) | Msg::CardMenuActivated => {}
        Msg::CardMenuMoved { forward, items } => {
            if let Some(menu) = state.card_menu.as_mut().filter(|_| items > 0) {
                menu.highlighted = Some(match (menu.highlighted, forward) {
                    (None, true) => 0,
                    (None, false) => items - 1,
                    (Some(at), true) => (at + 1) % items,
                    (Some(at), false) => (at + items - 1) % items,
                });
            }
        }
        Msg::FullScreenToggled { settings_open } => {
            // Not under an open card menu: the grid would move beneath it.
            if state.open && !settings_open && state.card_menu.is_none() {
                state.full_screen = !state.full_screen;
            }
        }
        Msg::Toggled {
            settings_open: true,
        } => {}
        Msg::Toggled {
            settings_open: false,
        } => {
            state.open = !state.open;
            // The Overview always opens in the normal layout (FR-001).
            if state.open {
                state.full_screen = false;
            }
        }
        Msg::Closed | Msg::CardActivated(_) => state.open = false,
    }
    Vec::new()
}

/// The numeric ticket of a worktree directory name; a Jira key or no ticket is `None`.
pub fn numeric_ticket(dir_name: &str) -> Option<u64> {
    micold_core::naming::ticket(dir_name)?.parse().ok()
}

/// A location's card state and why (feature 1125, research R2): from its un-archived sessions, the
/// agent requests waiting on them and its branch's pull request reading. The Overview's card and the
/// sidebar's state group both ask this, so the two cannot disagree about a location (US1.7).
pub fn location_card_state(
    app: &crate::app::State,
    location: &SessionLocation,
    branch: Option<&str>,
) -> (
    micold_core::overview::CardState,
    micold_core::overview::CardReason,
) {
    let sessions: Vec<&micold_core::session::Session> = app
        .active_sessions()
        .iter()
        .filter(|s| s.location == *location && !s.archived)
        .collect();
    let agent_request = app
        .agent_confirm
        .pending
        .iter()
        .any(|prompt| sessions.iter().any(|s| s.id == prompt.session));
    micold_core::overview::card_state(&micold_core::overview::CardSignals {
        location: micold_core::location_state::location_state(sessions.iter().copied()),
        agent_request,
        pr: pr_signal(&app.pr_status, branch),
    })
}

/// The pull request reading for a branch: unknown while the reading is off or holds nothing (never
/// read, or released), and for a location with no branch, such as Default (research R6, R14).
pub fn pr_signal(
    pr: &crate::features::pr_status::State,
    branch: Option<&str>,
) -> micold_core::overview::PrSignal {
    use micold_core::overview::{CurrentPr, PrSignal};
    match branch {
        Some(branch) if pr.enabled && pr.read_at.is_some() => PrSignal::Known {
            current: pr.statuses.get(branch).map(CurrentPr::from_status),
            removable: pr.removable.contains(branch),
        },
        _ => PrSignal::Unknown,
    }
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
