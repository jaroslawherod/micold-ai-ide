//! The worktree Overview (feature 813): a main-pane grid with one card per sidebar location,
//! each showing its type, branch, session counts and card state (research R1, R4).
//!
//! Only the open flag lives here. The cards compose the sidebar, sessions, agent requests and
//! pull request readings, so they are built in [`crate::ui::overview::cards`], not by this module
//! (a feature module names no other feature).

use micold_core::task_progress::FileProgress;
use std::collections::BTreeMap;

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
    /// A worktree's files were read (research R9).
    ProgressRead {
        /// The worktree's directory name.
        dir: String,
        /// What its files say.
        progress: FileProgress,
    },
    /// These tickets' titles are being fetched: each without an entry is pending (R8).
    TitlesRequested(Vec<u64>),
    /// A ticket's title was read: [`TitleLookup::Known`] or [`TitleLookup::Unavailable`].
    TitleRead {
        /// The issue number.
        number: u64,
        /// The answer.
        result: TitleLookup,
    },
    /// The window moved to another project: what was read describes the old one.
    ProjectSwitched,
}

/// Apply a message to the Overview's state.
/// An activation that sends the user to a row asks the root to reveal it; nothing else asks.
pub fn update(state: &mut State, msg: Msg) -> Vec<crate::features::Outcome> {
    use micold_core::overview::Activation;
    if let Msg::CardActivated(Activation::OfferStart(at) | Activation::RevealOnly(at)) = &msg {
        state.open = false;
        return vec![crate::features::Outcome::LocationRevealed(at.clone())];
    }
    match msg {
        Msg::ProgressRead { dir, progress } => {
            state.progress.insert(dir, progress);
        }
        Msg::TitlesRequested(numbers) => {
            for number in numbers {
                state.titles.entry(number).or_default();
            }
        }
        Msg::TitleRead { number, result } => {
            state.titles.insert(number, result);
        }
        Msg::ProjectSwitched => {
            state.progress.clear();
            state.titles.clear();
        }
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
