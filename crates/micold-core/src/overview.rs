//! The worktree Overview's card rules (feature 813, research R4, R7): what type a location is, the
//! one card state its sessions, agent requests and pull request reading put it in, its session
//! counts, and the order and attention summary of a set of cards. Pure: the client builds the
//! signals from what the window holds and renders the result (data-model *CardSignals*, *CardState*,
//! *CardModel*).

use crate::location_state::{session_state, BlockedReason, LocationState};
use crate::naming::{parse_tags, ConventionalType, Tag};
use crate::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
use crate::session::{Session, SessionId, SessionLocation};
use crate::worktree::WorktreeStatus;

/// What kind of location a card stands for, shown as an icon and a word (FR-002, research R7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LocationType {
    /// A `feat` branch.
    Feature,
    /// A `fix` branch.
    Bugfix,
    /// Any other conventional type (`chore`, `docs`, `refactor`, …).
    Chore,
    /// No conventional type in the name.
    Other,
    /// A worktree the user included from outside the app's worktree folder.
    Included,
    /// A worktree whose folder is missing, or a folder git does not know.
    OrphanInvalid,
    /// The project root (the sidebar's Default row).
    ProjectRoot,
}

impl LocationType {
    /// The type's word on the card.
    pub fn label(self) -> &'static str {
        match self {
            Self::Feature => "Feature",
            Self::Bugfix => "Bugfix",
            Self::Chore => "Chore",
            Self::Other => "Other",
            Self::Included => "Included",
            Self::OrphanInvalid => "Orphan",
            Self::ProjectRoot => "Project root",
        }
    }
}

/// A location's type, by the precedence ProjectRoot → Included → OrphanInvalid → branch type
/// (research R7). The branch type is the sidebar's own `Tag::Type`, so card and row agree.
pub fn location_type(
    location: &SessionLocation,
    included: bool,
    status: WorktreeStatus,
) -> LocationType {
    let SessionLocation::Worktree(dir_name) = location else {
        return LocationType::ProjectRoot;
    };
    if included {
        return LocationType::Included;
    }
    if matches!(status, WorktreeStatus::Missing | WorktreeStatus::Invalid) {
        return LocationType::OrphanInvalid;
    }
    let kind = parse_tags(dir_name).into_iter().find_map(|tag| match tag {
        Tag::Type(kind) => Some(kind),
        _ => None,
    });
    match kind {
        Some(ConventionalType::Feat) => LocationType::Feature,
        Some(ConventionalType::Fix) => LocationType::Bugfix,
        Some(_) => LocationType::Chore,
        None => LocationType::Other,
    }
}

/// How many pull requests a branch has, as the status reading counts them (data-model *PrCounts*):
/// those of this repository, not forks, among the newest ten of a group. A full page reads as *10+*
/// (feature 1123, FR-006); a full page with a fork's pull request reads its own count with *+*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PrCounts {
    /// Open, drafts included.
    pub open: u8,
    /// Merged.
    pub merged: u8,
    /// There may be more open ones than `open`.
    pub open_capped: bool,
    /// There may be more merged ones than `merged`.
    pub merged_capped: bool,
}

/// Why a card cannot show its pull request counts (FR-004, research R6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrUnknown {
    /// The pull request status setting is off.
    Off,
    /// The last reading could not reach GitHub.
    Unreachable,
    /// GitHub asked the app to wait until this Unix second.
    RateLimited {
        /// When readings resume, in Unix seconds.
        until: u64,
    },
    /// No reading has covered this branch yet.
    NotReadYet,
}

impl PrUnknown {
    /// The reason in words, the tooltip of the unknown mark. A time reads in the host's zone.
    pub fn label(self) -> String {
        match self {
            Self::Off => "pull request status is off".to_string(),
            Self::Unreachable => "GitHub is not reachable".to_string(),
            Self::RateLimited { until } => format!(
                "rate-limited until {}",
                crate::local_time::LocalTime::local(i64::try_from(until).unwrap_or(i64::MAX))
                    .hh_mm()
            ),
            Self::NotReadYet => "not read yet".to_string(),
        }
    }
}

/// What a card shows for its branch's pull requests (data-model *CardModel*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrDisplay {
    /// The location has no branch (Default, detached HEAD, orphan): nothing is shown.
    Hidden,
    /// The counts cannot be read: *—*, with the reason as tooltip.
    Unknown(PrUnknown),
    /// The branch's open and merged counts.
    Counts(PrCounts),
}

impl PrDisplay {
    /// The pull request row's text and, for an unknown mark, its tooltip (C9, FR-010); `None` when
    /// hidden or both counts are 0, and a count of 0 beside a non-zero one is left out. A capped
    /// count reads *10+*.
    pub fn footer(self) -> Option<(String, Option<String>)> {
        let count = |n: u8, capped: bool| {
            if capped {
                format!("{}+", u32::from(n))
            } else {
                n.to_string()
            }
        };
        match self {
            Self::Hidden => None,
            Self::Unknown(reason) => Some(("\u{2014}".to_string(), Some(reason.label()))),
            Self::Counts(c) => {
                let open = (c.open > 0 || c.open_capped)
                    .then(|| format!("{} open", count(c.open, c.open_capped)));
                let merged = (c.merged > 0 || c.merged_capped)
                    .then(|| format!("{} merged", count(c.merged, c.merged_capped)));
                let text = match (open, merged) {
                    (None, None) => return None,
                    (Some(one), None) | (None, Some(one)) => one,
                    (Some(open), Some(merged)) => format!("{open} \u{b7} {merged}"),
                };
                Some((text, None))
            }
        }
    }
}

/// What the card knows of the branch's current pull request (data-model *PrSignal*).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CurrentPr {
    /// Its checks fail (only an open or draft one has checks).
    pub checks_failing: bool,
    /// A reviewer asked for changes.
    pub changes_requested: bool,
    /// It cannot merge because of a conflict.
    pub conflict: bool,
    /// It was merged.
    pub merged: bool,
    /// It is open or a draft.
    pub open: bool,
}

impl CurrentPr {
    /// The signals one reading gives for a branch's current pull request.
    pub fn from_status(status: &PullRequestStatus) -> Self {
        let (open, checks_failing) = match status.state {
            PrState::Open { checks } | PrState::Draft { checks } => {
                (true, checks == CheckStatus::Failing)
            }
            PrState::Merged | PrState::Closed => (false, false),
        };
        Self {
            checks_failing,
            changes_requested: status.review == ReviewState::ChangesRequested,
            conflict: status.conflict,
            merged: status.state == PrState::Merged,
            open,
        }
    }
}

/// The pull request reading for a location's branch (data-model *PrSignal*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrSignal {
    /// The reading is off, unavailable, released or not read yet, or the location has no branch.
    Unknown,
    /// The reading covers the branch.
    Known {
        /// Its current pull request, if it has one.
        current: Option<CurrentPr>,
        /// Whether the worktree's work is in `main` and it can be removed.
        removable: bool,
    },
}

/// Everything a card's state is derived from (data-model *CardSignals*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardSignals {
    /// Feature 905's state over the location's un-archived sessions.
    pub location: LocationState,
    /// An agent request (a queued confirmation prompt) waits for one of its sessions.
    pub agent_request: bool,
    /// The pull request reading for its branch.
    pub pr: PrSignal,
}

/// A card's state. The derived `Ord` is worst first, so the minimum of several is the one to show
/// and an ascending sort puts the cards that need the user first (FR-005, FR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CardState {
    /// The user has to act before work can continue.
    NeedsAction,
    /// Something went wrong or wants a look.
    NeedsAttention,
    /// A session is working.
    Working,
    /// The work is merged and the worktree can go.
    Done,
    /// Nothing is happening.
    Idle,
}

impl CardState {
    /// Every state, worst first.
    pub const ALL: [CardState; 5] = [
        Self::NeedsAction,
        Self::NeedsAttention,
        Self::Working,
        Self::Done,
        Self::Idle,
    ];

    /// The state's word on the card.
    pub fn label(self) -> &'static str {
        match self {
            Self::NeedsAction => "Needs action",
            Self::NeedsAttention => "Needs attention",
            Self::Working => "Working",
            Self::Done => "Done",
            Self::Idle => "Idle",
        }
    }
}

/// Why a card is in its state: the table row that matched, shown as the state word's tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CardReason {
    /// Row 1.
    NeedsPermission,
    /// Row 2.
    AgentRequest,
    /// Row 3.
    SessionFailed,
    /// Row 4.
    Resumable,
    /// Row 5.
    ChecksFailing,
    /// Row 6.
    ChangesRequested,
    /// Row 7.
    MergeConflict,
    /// Row 8.
    Working,
    /// Row 9.
    Merged,
    /// Row 10, sessions idle or none.
    Idle,
    /// Row 10, a session's activity cannot be read.
    ActivityUnknown,
}

impl CardReason {
    /// The reason in words, for the state word's tooltip.
    pub fn label(self) -> &'static str {
        match self {
            Self::NeedsPermission => "needs permission",
            Self::AgentRequest => "agent request waiting",
            Self::SessionFailed => "session failed",
            Self::Resumable => "resumable",
            Self::ChecksFailing => "checks failing",
            Self::ChangesRequested => "changes requested",
            Self::MergeConflict => "merge conflict",
            Self::Working => "working",
            Self::Merged => "merged",
            Self::Idle => "idle",
            Self::ActivityUnknown => "activity unknown",
        }
    }
}

/// A card's state and reason, by data-model's ordered table: the first matching row wins, which
/// is always the worst state that applies (scenario 1.5).
pub fn card_state(signals: &CardSignals) -> (CardState, CardReason) {
    use CardReason as R;
    use CardState as S;
    let open = match signals.pr {
        PrSignal::Known {
            current: Some(pr), ..
        } if pr.open => Some(pr),
        _ => None,
    };
    let merged_removable = matches!(
        signals.pr,
        PrSignal::Known { current: Some(pr), removable: true } if pr.merged
    );
    match signals.location {
        LocationState::Blocked(BlockedReason::NeedsPermission) => {
            (S::NeedsAction, R::NeedsPermission)
        }
        _ if signals.agent_request => (S::NeedsAction, R::AgentRequest),
        LocationState::Blocked(BlockedReason::Failed) => (S::NeedsAttention, R::SessionFailed),
        LocationState::Blocked(BlockedReason::Resumable) => (S::NeedsAttention, R::Resumable),
        _ if open.is_some_and(|pr| pr.checks_failing) => (S::NeedsAttention, R::ChecksFailing),
        _ if open.is_some_and(|pr| pr.changes_requested) => {
            (S::NeedsAttention, R::ChangesRequested)
        }
        _ if open.is_some_and(|pr| pr.conflict) => (S::NeedsAttention, R::MergeConflict),
        LocationState::Running => (S::Working, R::Working),
        _ if merged_removable => (S::Done, R::Merged),
        LocationState::Unknown => (S::Idle, R::ActivityUnknown),
        _ => (S::Idle, R::Idle),
    }
}

/// A location's session counts: its un-archived sessions, and how many of them are running by
/// feature 905's per-session state (FR-003).
pub fn session_counts<'a>(sessions: impl IntoIterator<Item = &'a Session>) -> (u32, u32) {
    sessions
        .into_iter()
        .filter(|s| !s.archived)
        .fold((0, 0), |(total, running), s| {
            let is_running = session_state(s) == LocationState::Running;
            (total + 1, running + u32::from(is_running))
        })
}

/// What activating a card does (FR-013, research R12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Activation {
    /// Select and show the location's main session, as the sidebar does.
    OpenSession(SessionId),
    /// Select the location and offer the start menu; nothing starts until the user picks.
    OfferStart(SessionLocation),
    /// Select the location only: a session cannot start there (orphan / invalid).
    RevealOnly(SessionLocation),
}

/// What activating the card of `location` does: open its `main_session` when it has one, else offer
/// to start one, unless `kind` says none can start there (US3.1-3.3). The card's state plays no
/// part, so an Idle card activates as any other (US3.5).
pub fn activation(
    location: &SessionLocation,
    main_session: Option<SessionId>,
    kind: LocationType,
) -> Activation {
    match (main_session, kind) {
        (Some(id), _) => Activation::OpenSession(id),
        (None, LocationType::OrphanInvalid) => Activation::RevealOnly(location.clone()),
        (None, _) => Activation::OfferStart(location.clone()),
    }
}

/// One card, built per render from the window's state (data-model *CardModel*).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardModel {
    /// The location the card stands for.
    pub location: SessionLocation,
    /// The sidebar's name for it.
    pub display_name: String,
    /// Its branch, when it has one.
    pub branch: Option<String>,
    /// Its type.
    pub kind: LocationType,
    /// Un-archived sessions, and how many of them run.
    pub sessions: (u32, u32),
    /// Its card state.
    pub state: CardState,
    /// Why it is in that state.
    pub reason: CardReason,
    /// The card's title: the issue title, else the branch, else the display name (FR-006,
    /// [`card_title`](crate::overview_text::card_title)).
    pub title: String,
    /// Its pull request counts, or why there are none.
    pub prs: PrDisplay,
    /// The pull requests its rows show: `Some` exactly when `prs` shows counts (feature 1123, R5).
    pub pull_requests: Option<crate::pr_list::BranchPrList>,
    /// The feature's phase and task ratio, or none (data-model *CardModel*, R9).
    pub progress: crate::task_progress::Progress,
    /// What the main session's hooks say it is doing now, when they report (Story 5, R10).
    pub activity: Option<crate::hook_activity::CurrentActivity>,
    /// What activating the card does.
    pub activation: Activation,
    /// Its directory: the worktree's path, or the project root for *Default* (feature 1121 R8).
    pub dir: std::path::PathBuf,
    /// The link to the issue its worktree is named after, when the name carries a ticket and its
    /// source is configured and switched on (feature 1122, FR-001, FR-002).
    pub issue_link: Option<crate::issue_link::IssueLink>,
}

/// Orders cards worst state first, keeping the sidebar's order within a state (FR-007).
pub fn order(cards: &mut [CardModel]) {
    cards.sort_by_key(|card| card.state);
}

/// The worst state among the cards when it needs the user, with how many cards are in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Attention {
    /// NeedsAction or NeedsAttention.
    pub state: CardState,
    /// The number of cards in that state.
    pub count: usize,
}

/// What the Overview button's badge shows: `None` unless some card needs action or attention
/// (FR-012).
pub fn attention(cards: &[CardModel]) -> Option<Attention> {
    let worst = cards.iter().map(|card| card.state).min()?;
    if worst > CardState::NeedsAttention {
        return None;
    }
    let count = cards.iter().filter(|card| card.state == worst).count();
    Some(Attention {
        state: worst,
        count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::messages::ActivitySignal;
    use crate::session::{AiCli, SessionId, SessionLifecycle};

    fn worktree(dir_name: &str) -> SessionLocation {
        SessionLocation::Worktree(dir_name.into())
    }

    fn kind_of(dir_name: &str) -> LocationType {
        location_type(&worktree(dir_name), false, WorktreeStatus::Valid)
    }

    #[test]
    fn the_project_root_is_its_own_type_whatever_else_holds() {
        for status in [
            WorktreeStatus::Valid,
            WorktreeStatus::Missing,
            WorktreeStatus::Invalid,
        ] {
            for included in [false, true] {
                assert_eq!(
                    location_type(&SessionLocation::Default, included, status),
                    LocationType::ProjectRoot,
                    "Default is the project root before every other rule"
                );
            }
        }
    }

    #[test]
    fn an_included_worktree_is_included_before_its_health_or_branch_type() {
        for status in [
            WorktreeStatus::Valid,
            WorktreeStatus::Missing,
            WorktreeStatus::Invalid,
        ] {
            assert_eq!(
                location_type(&worktree("feat-1_alpha"), true, status),
                LocationType::Included,
                "Included comes before orphan / invalid and the branch type"
            );
        }
    }

    #[test]
    fn a_missing_or_invalid_worktree_is_an_orphan_before_its_branch_type() {
        for status in [WorktreeStatus::Missing, WorktreeStatus::Invalid] {
            assert_eq!(
                location_type(&worktree("feat-1_alpha"), false, status),
                LocationType::OrphanInvalid,
                "an unhealthy worktree is an orphan whatever its name says"
            );
        }
    }

    #[test]
    fn the_branch_type_decides_the_rest() {
        assert_eq!(
            kind_of("feat-42_alpha"),
            LocationType::Feature,
            "feat is a feature"
        );
        assert_eq!(
            kind_of("fix-7_beta"),
            LocationType::Bugfix,
            "fix is a bugfix"
        );
        for token in [
            "chore", "docs", "refactor", "test", "build", "ci", "perf", "style",
        ] {
            assert_eq!(
                kind_of(&format!("{token}-9_gamma")),
                LocationType::Chore,
                "every other conventional type is a chore"
            );
        }
        assert_eq!(kind_of("misc"), LocationType::Other, "no type is Other");
    }

    #[test]
    fn each_location_type_has_its_word() {
        let words: Vec<_> = [
            LocationType::Feature,
            LocationType::Bugfix,
            LocationType::Chore,
            LocationType::Other,
            LocationType::Included,
            LocationType::OrphanInvalid,
            LocationType::ProjectRoot,
        ]
        .into_iter()
        .map(LocationType::label)
        .collect();
        assert_eq!(
            words,
            [
                "Feature",
                "Bugfix",
                "Chore",
                "Other",
                "Included",
                "Orphan",
                "Project root"
            ],
            "the card names each type by the spec's word"
        );
    }

    fn locations() -> Vec<LocationState> {
        vec![
            LocationState::Idle,
            LocationState::Unknown,
            LocationState::Running,
            LocationState::Blocked(BlockedReason::Resumable),
            LocationState::Blocked(BlockedReason::Failed),
            LocationState::Blocked(BlockedReason::NeedsPermission),
        ]
    }

    fn current_prs() -> Vec<CurrentPr> {
        let open = CurrentPr {
            open: true,
            ..CurrentPr::default()
        };
        vec![
            open,
            CurrentPr {
                checks_failing: true,
                ..open
            },
            CurrentPr {
                changes_requested: true,
                ..open
            },
            CurrentPr {
                conflict: true,
                ..open
            },
            CurrentPr {
                checks_failing: true,
                changes_requested: true,
                conflict: true,
                ..open
            },
            CurrentPr {
                merged: true,
                ..CurrentPr::default()
            },
            // A closed pull request: neither open nor merged.
            CurrentPr::default(),
        ]
    }

    fn pr_signals() -> Vec<PrSignal> {
        let mut signals = vec![PrSignal::Unknown];
        for removable in [false, true] {
            signals.push(PrSignal::Known {
                current: None,
                removable,
            });
            for pr in current_prs() {
                signals.push(PrSignal::Known {
                    current: Some(pr),
                    removable,
                });
            }
        }
        signals
    }

    fn every_signal() -> Vec<CardSignals> {
        let mut all = Vec::new();
        for location in locations() {
            for agent_request in [false, true] {
                for pr in pr_signals() {
                    all.push(CardSignals {
                        location,
                        agent_request,
                        pr,
                    });
                }
            }
        }
        all
    }

    /// Data-model's table, written as its rows' conditions, each checked on its own.
    fn table(s: &CardSignals) -> (CardState, CardReason) {
        let current = match s.pr {
            PrSignal::Known { current, .. } => current,
            PrSignal::Unknown => None,
        };
        let open = current.filter(|pr| pr.open);
        let removable = matches!(
            s.pr,
            PrSignal::Known {
                removable: true,
                ..
            }
        );
        let rows: [(bool, CardState, CardReason); 9] = [
            (
                s.location == LocationState::Blocked(BlockedReason::NeedsPermission),
                CardState::NeedsAction,
                CardReason::NeedsPermission,
            ),
            (
                s.agent_request,
                CardState::NeedsAction,
                CardReason::AgentRequest,
            ),
            (
                s.location == LocationState::Blocked(BlockedReason::Failed),
                CardState::NeedsAttention,
                CardReason::SessionFailed,
            ),
            (
                s.location == LocationState::Blocked(BlockedReason::Resumable),
                CardState::NeedsAttention,
                CardReason::Resumable,
            ),
            (
                open.is_some_and(|pr| pr.checks_failing),
                CardState::NeedsAttention,
                CardReason::ChecksFailing,
            ),
            (
                open.is_some_and(|pr| pr.changes_requested),
                CardState::NeedsAttention,
                CardReason::ChangesRequested,
            ),
            (
                open.is_some_and(|pr| pr.conflict),
                CardState::NeedsAttention,
                CardReason::MergeConflict,
            ),
            (
                s.location == LocationState::Running,
                CardState::Working,
                CardReason::Working,
            ),
            (
                current.is_some_and(|pr| pr.merged) && removable,
                CardState::Done,
                CardReason::Merged,
            ),
        ];
        rows.into_iter()
            .find(|(applies, ..)| *applies)
            .map(|(_, state, reason)| (state, reason))
            .unwrap_or(if s.location == LocationState::Unknown {
                (CardState::Idle, CardReason::ActivityUnknown)
            } else {
                (CardState::Idle, CardReason::Idle)
            })
    }

    /// Data-model's table as literal rows (signals, state, reason), independent of `table()`.
    #[test]
    fn card_state_matches_the_literal_rows_of_the_data_model() {
        use BlockedReason::{Failed, NeedsPermission, Resumable};
        let open = CurrentPr {
            open: true,
            ..CurrentPr::default()
        };
        let known = |current: Option<CurrentPr>, removable| PrSignal::Known { current, removable };
        let rows = [
            // 1
            (
                LocationState::Blocked(NeedsPermission),
                false,
                PrSignal::Unknown,
                CardState::NeedsAction,
                CardReason::NeedsPermission,
            ),
            // 2
            (
                LocationState::Idle,
                true,
                PrSignal::Unknown,
                CardState::NeedsAction,
                CardReason::AgentRequest,
            ),
            // 3
            (
                LocationState::Blocked(Failed),
                false,
                PrSignal::Unknown,
                CardState::NeedsAttention,
                CardReason::SessionFailed,
            ),
            // 4
            (
                LocationState::Blocked(Resumable),
                false,
                PrSignal::Unknown,
                CardState::NeedsAttention,
                CardReason::Resumable,
            ),
            // 5
            (
                LocationState::Idle,
                false,
                known(
                    Some(CurrentPr {
                        checks_failing: true,
                        ..open
                    }),
                    false,
                ),
                CardState::NeedsAttention,
                CardReason::ChecksFailing,
            ),
            // 6
            (
                LocationState::Idle,
                false,
                known(
                    Some(CurrentPr {
                        changes_requested: true,
                        ..open
                    }),
                    false,
                ),
                CardState::NeedsAttention,
                CardReason::ChangesRequested,
            ),
            // 7
            (
                LocationState::Idle,
                false,
                known(
                    Some(CurrentPr {
                        conflict: true,
                        ..open
                    }),
                    false,
                ),
                CardState::NeedsAttention,
                CardReason::MergeConflict,
            ),
            // 8
            (
                LocationState::Running,
                false,
                PrSignal::Unknown,
                CardState::Working,
                CardReason::Working,
            ),
            // 9
            (
                LocationState::Idle,
                false,
                known(
                    Some(CurrentPr {
                        merged: true,
                        ..CurrentPr::default()
                    }),
                    true,
                ),
                CardState::Done,
                CardReason::Merged,
            ),
            // 9 needs removable: merged but not removable falls through to 10
            (
                LocationState::Idle,
                false,
                known(
                    Some(CurrentPr {
                        merged: true,
                        ..CurrentPr::default()
                    }),
                    false,
                ),
                CardState::Idle,
                CardReason::Idle,
            ),
            // 10
            (
                LocationState::Idle,
                false,
                PrSignal::Unknown,
                CardState::Idle,
                CardReason::Idle,
            ),
            (
                LocationState::Unknown,
                false,
                PrSignal::Unknown,
                CardState::Idle,
                CardReason::ActivityUnknown,
            ),
            // Several signals: the first row wins (scenario 1.5).
            (
                LocationState::Running,
                true,
                known(
                    Some(CurrentPr {
                        checks_failing: true,
                        ..open
                    }),
                    false,
                ),
                CardState::NeedsAction,
                CardReason::AgentRequest,
            ),
            (
                LocationState::Running,
                false,
                known(
                    Some(CurrentPr {
                        conflict: true,
                        ..open
                    }),
                    false,
                ),
                CardState::NeedsAttention,
                CardReason::MergeConflict,
            ),
        ];
        for (location, agent_request, pr, state, reason) in rows {
            let signals = CardSignals {
                location,
                agent_request,
                pr,
            };
            assert_eq!(card_state(&signals), (state, reason), "{signals:?}");
        }
    }

    #[test]
    fn card_state_follows_the_table_for_every_combination_of_signals() {
        let all = every_signal();
        assert_eq!(
            all.len(),
            6 * 2 * 17,
            "every location × request × PR signal (SC-003)"
        );
        for signals in all {
            assert_eq!(
                card_state(&signals),
                table(&signals),
                "the first matching row of the table decides: {signals:?}"
            );
        }
    }

    #[test]
    fn an_unknown_pull_request_reading_never_decides_the_state() {
        for signals in every_signal()
            .into_iter()
            .filter(|s| s.pr == PrSignal::Unknown)
        {
            let (_, reason) = card_state(&signals);
            assert!(
                !matches!(
                    reason,
                    CardReason::ChecksFailing
                        | CardReason::ChangesRequested
                        | CardReason::MergeConflict
                        | CardReason::Merged
                ),
                "no PR-derived row without a reading: {signals:?}"
            );
        }
    }

    #[test]
    fn an_unknown_activity_alone_is_never_working_or_done() {
        let signals = CardSignals {
            location: LocationState::Unknown,
            agent_request: false,
            pr: PrSignal::Unknown,
        };
        assert_eq!(
            card_state(&signals),
            (CardState::Idle, CardReason::ActivityUnknown),
            "an unreadable activity claims nothing"
        );
    }

    #[test]
    fn pull_request_problems_count_only_on_an_open_pull_request() {
        let closed_with_problems = CurrentPr {
            checks_failing: true,
            changes_requested: true,
            conflict: true,
            ..CurrentPr::default()
        };
        let signals = CardSignals {
            location: LocationState::Idle,
            agent_request: false,
            pr: PrSignal::Known {
                current: Some(closed_with_problems),
                removable: false,
            },
        };
        assert_eq!(
            card_state(&signals).0,
            CardState::Idle,
            "rows 5–7 apply only to an open or draft pull request"
        );
    }

    #[test]
    fn several_signals_show_the_worst_state() {
        let signals = CardSignals {
            location: LocationState::Blocked(BlockedReason::NeedsPermission),
            agent_request: false,
            pr: PrSignal::Known {
                current: Some(CurrentPr {
                    open: true,
                    checks_failing: true,
                    ..CurrentPr::default()
                }),
                removable: false,
            },
        };
        assert_eq!(
            card_state(&signals),
            (CardState::NeedsAction, CardReason::NeedsPermission),
            "a permission request beats failing checks (scenario 1.5)"
        );
    }

    #[test]
    fn each_card_state_has_its_word() {
        let words: Vec<_> = CardState::ALL.into_iter().map(CardState::label).collect();
        assert_eq!(
            words,
            ["Needs action", "Needs attention", "Working", "Done", "Idle"],
            "the card names each state by the spec's word"
        );
    }

    fn session(lifecycle: SessionLifecycle, activity: ActivitySignal, archived: bool) -> Session {
        let mut s = Session::start_new(worktree("wt"), AiCli::ClaudeCode);
        s.lifecycle = lifecycle;
        s.activity = activity;
        s.archived = archived;
        s
    }

    #[test]
    fn session_counts_skip_archived_sessions_and_count_the_running_ones() {
        let sessions = [
            session(SessionLifecycle::Running, ActivitySignal::Working, false),
            session(SessionLifecycle::Starting, ActivitySignal::Unknown, false),
            session(SessionLifecycle::Idle, ActivitySignal::Unknown, false),
            session(SessionLifecycle::Running, ActivitySignal::Working, true),
        ];
        assert_eq!(
            session_counts(&sessions),
            (3, 2),
            "3 sessions, 2 running; the archived one is not counted (scenario 1.2)"
        );
    }

    fn id(n: u128) -> SessionId {
        SessionId(uuid::Uuid::from_u128(n))
    }

    /// US3.1: a location with a main session opens it, whatever its type.
    #[test]
    fn activation_opens_the_main_session_when_there_is_one() {
        for kind in [LocationType::Feature, LocationType::OrphanInvalid] {
            assert_eq!(
                activation(&worktree("a"), Some(id(1)), kind),
                Activation::OpenSession(id(1)),
                "{kind:?}"
            );
        }
        assert_eq!(
            activation(
                &SessionLocation::Default,
                Some(id(2)),
                LocationType::ProjectRoot
            ),
            Activation::OpenSession(id(2))
        );
    }

    /// US3.2: no un-archived session on a location a session can start in offers the start.
    #[test]
    fn activation_offers_to_start_where_there_is_no_session() {
        for kind in [
            LocationType::Feature,
            LocationType::Bugfix,
            LocationType::Chore,
            LocationType::Other,
            LocationType::Included,
        ] {
            assert_eq!(
                activation(&worktree("a"), None, kind),
                Activation::OfferStart(worktree("a")),
                "{kind:?}"
            );
        }
        assert_eq!(
            activation(&SessionLocation::Default, None, LocationType::ProjectRoot),
            Activation::OfferStart(SessionLocation::Default)
        );
    }

    /// US3.3: an orphan / invalid location without a session is only revealed.
    #[test]
    fn activation_only_reveals_an_orphan_or_invalid_location() {
        assert_eq!(
            activation(&worktree("gone"), None, LocationType::OrphanInvalid),
            Activation::RevealOnly(worktree("gone"))
        );
    }

    /// US3.5: the state does not change what activation does; the card carries it.
    #[test]
    fn an_idle_card_activates_as_any_other() {
        let mut idle = card("a", CardState::Idle);
        let mut busy = card("a", CardState::Working);
        idle.activation = activation(&idle.location, None, idle.kind);
        busy.activation = activation(&busy.location, None, busy.kind);
        assert_eq!(idle.activation, busy.activation);
        assert_eq!(idle.activation, Activation::OfferStart(worktree("a")));
    }

    /// C9, FR-010, US2.7: no row at 0 · 0; one count alone when the other is 0; the unknown mark
    /// keeps its reason.
    #[test]
    fn the_pull_request_row_hides_zero_counts() {
        let counts = |open: u8, merged: u8, open_capped: bool| {
            PrDisplay::Counts(PrCounts {
                open,
                merged,
                open_capped,
                merged_capped: false,
            })
            .footer()
        };
        let text = |s: &str| Some((s.to_string(), None));
        assert_eq!(counts(0, 0, false), None, "nothing to say: no row");
        assert_eq!(counts(1, 0, false), text("1 open"));
        assert_eq!(counts(0, 2, false), text("2 merged"));
        assert_eq!(counts(1, 2, false), text("1 open \u{b7} 2 merged"));
        assert_eq!(counts(10, 0, true), text("10+ open"), "a capped count");
        assert_eq!(PrDisplay::Hidden.footer(), None);
        assert_eq!(
            PrDisplay::Unknown(PrUnknown::Unreachable).footer(),
            Some(("\u{2014}".to_string(), Some(PrUnknown::Unreachable.label())))
        );
    }

    fn card(name: &str, state: CardState) -> CardModel {
        CardModel {
            location: worktree(name),
            display_name: name.into(),
            branch: None,
            kind: LocationType::Other,
            sessions: (0, 0),
            state,
            reason: CardReason::Idle,
            title: name.into(),
            prs: PrDisplay::Hidden,
            pull_requests: None,
            progress: Default::default(),
            activity: None,
            activation: Activation::RevealOnly(worktree(name)),
            dir: std::path::PathBuf::from(format!("/repo/{name}")),
            issue_link: None,
        }
    }

    #[test]
    fn order_puts_the_worst_state_first_and_keeps_the_sidebar_order_within_a_state() {
        let mut cards = vec![
            card("a", CardState::Idle),
            card("b", CardState::Working),
            card("c", CardState::NeedsAction),
            card("d", CardState::Idle),
            card("e", CardState::Done),
            card("f", CardState::NeedsAttention),
            card("g", CardState::Working),
        ];
        order(&mut cards);
        let names: Vec<_> = cards.iter().map(|c| c.display_name.as_str()).collect();
        assert_eq!(
            names,
            ["c", "f", "b", "g", "e", "a", "d"],
            "worst first; ties keep the sidebar's order (FR-007)"
        );
    }

    #[test]
    fn attention_names_the_worst_state_that_needs_the_user_and_its_count() {
        let cards = [
            card("a", CardState::NeedsAttention),
            card("b", CardState::NeedsAction),
            card("c", CardState::NeedsAction),
        ];
        assert_eq!(
            attention(&cards),
            Some(Attention {
                state: CardState::NeedsAction,
                count: 2
            }),
            "the badge counts the cards in the worst state (FR-012)"
        );
        let cards = [
            card("a", CardState::NeedsAttention),
            card("b", CardState::Idle),
        ];
        assert_eq!(
            attention(&cards),
            Some(Attention {
                state: CardState::NeedsAttention,
                count: 1
            }),
            "Needs attention alone shows a badge"
        );
    }

    #[test]
    fn attention_is_none_when_nothing_needs_the_user() {
        let cards = [card("a", CardState::Working), card("b", CardState::Done)];
        assert_eq!(
            attention(&cards),
            None,
            "no badge without action or attention"
        );
        assert_eq!(attention(&[]), None, "no badge without cards");
    }
}
