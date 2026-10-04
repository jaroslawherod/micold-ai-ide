//! When this window reads the pull request status of its active project, and what it holds from
//! the last reading (feature 040, contracts/reading-and-wire.md §2, data-model §3).
//!
//! Render-free and clock-free: every message that needs the time carries `now` (Unix seconds).
//! The shell half (`shell/pr_status.rs`) turns [`Effect::Read`] into the reading itself.

use std::collections::{BTreeMap, BTreeSet};

use micold_core::pull_request::{PullRequestStatus, ReadingFailure};

/// Whether a reading is under way.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Phase {
    /// No reading is under way.
    #[default]
    Idle,
    /// Reading `seq`, started at `started` (Unix seconds). `again`: one further reading follows.
    Reading {
        /// The reading's sequence number; an answer with another is dropped.
        seq: u64,
        /// One further reading starts when this one ends.
        again: bool,
        /// Unix seconds at which the reading started.
        started: u64,
    },
}

/// What this window holds about its active project's pull requests (data-model §3).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct State {
    /// The live value of the setting, from `Welcome` and `SettingsChanged`.
    pub enabled: bool,
    /// The window holds its active project (`Attached`, and no release since).
    pub held: bool,
    /// Set by `Attached`; cleared by the next listing, which starts the first reading.
    pub awaiting_listing: bool,
    /// Whether a reading is under way.
    pub phase: Phase,
    /// The sequence number the next reading gets.
    pub next_seq: u64,
    /// Branch → status of the last successful reading.
    pub statuses: BTreeMap<String, PullRequestStatus>,
    /// Branches whose merged pull request holds all of the branch's work.
    pub removable: BTreeSet<String>,
    /// Unix seconds at which the last successful reading started.
    pub read_at: Option<u64>,
    /// No reading starts while `now < pause_until` (FR-024). Outlives a project switch.
    pub pause_until: Option<u64>,
}

/// How a reading ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The statuses of the branches that have a pull request.
    Ok {
        /// Branch → status.
        statuses: BTreeMap<String, PullRequestStatus>,
        /// Branches whose merged pull request holds all of their work.
        removable: BTreeSet<String>,
        /// Unix seconds at which the reading started.
        started_at: u64,
    },
    /// Why there are no statuses.
    Err(ReadingFailure),
}

/// What this feature is told.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Msg {
    /// `Attached` for the active project: the window holds it.
    Held,
    /// A listing arrived (`CatalogChanged`); the first one after `Held` starts a reading (S1).
    ListingArrived {
        /// Unix seconds.
        now: u64,
    },
    /// The setting's live value, from `Welcome` or `SettingsChanged` (S2 when it turns on).
    EnabledChanged {
        /// The value now in force.
        enabled: bool,
        /// Unix seconds.
        now: u64,
    },
    /// Reading `seq` ended.
    Finished {
        /// The reading it answers.
        seq: u64,
        /// How it ended.
        outcome: Outcome,
        /// Unix seconds.
        now: u64,
    },
    /// The hold ended: project switched or forgotten, displaced, refused, disconnected.
    Released,
    /// The shell's 10-second bound on reading `seq`'s `RemoteList` (request `req`) ran out. The
    /// shell answers it; to this reducer it changes nothing.
    RemotesTimedOut {
        /// The reading that asked.
        seq: u64,
        /// The request that was asked.
        req: u64,
    },
}

/// What the shell must do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Nothing.
    None,
    /// Start reading `seq`.
    Read {
        /// The new reading's sequence number.
        seq: u64,
    },
}

/// Apply `msg` (contracts/reading-and-wire.md §2).
pub fn update(state: &mut State, msg: Msg) -> Effect {
    match msg {
        Msg::Held => {
            state.held = true;
            state.awaiting_listing = true;
            Effect::None
        }
        Msg::ListingArrived { now } => {
            if !state.awaiting_listing {
                return Effect::None;
            }
            state.awaiting_listing = false;
            let wanted = state.enabled && !paused(state, now);
            match &mut state.phase {
                Phase::Reading { again, .. } => {
                    *again |= wanted;
                    Effect::None
                }
                Phase::Idle => start(state, now),
            }
        }
        Msg::EnabledChanged { enabled, now } => {
            if enabled == state.enabled {
                return Effect::None;
            }
            state.enabled = enabled;
            if enabled {
                start(state, now)
            } else {
                state.statuses.clear();
                state.removable.clear();
                state.read_at = None;
                state.pause_until = None;
                state.phase = Phase::Idle;
                Effect::None
            }
        }
        Msg::Finished { seq, outcome, now } => {
            let Phase::Reading {
                seq: current,
                again,
                ..
            } = state.phase
            else {
                return Effect::None;
            };
            if seq != current {
                return Effect::None;
            }
            state.phase = Phase::Idle;
            let mut again = again;
            match outcome {
                Outcome::Ok {
                    statuses,
                    removable,
                    started_at,
                } => {
                    state.statuses = statuses;
                    state.removable = removable;
                    state.read_at = Some(started_at);
                    state.pause_until = None;
                }
                Outcome::Err(ReadingFailure::Unavailable) => {
                    state.statuses.clear();
                    state.removable.clear();
                    state.read_at = None;
                }
                Outcome::Err(ReadingFailure::Passing) => {}
                Outcome::Err(ReadingFailure::RateLimited { until }) => {
                    state.pause_until = Some(until);
                    again = false;
                }
            }
            if again {
                start(state, now)
            } else {
                Effect::None
            }
        }
        Msg::Released => {
            state.held = false;
            state.awaiting_listing = false;
            state.phase = Phase::Idle;
            state.statuses.clear();
            state.removable.clear();
            state.read_at = None;
            Effect::None
        }
        Msg::RemotesTimedOut { .. } => Effect::None,
    }
}

/// Whether readings are held back by GitHub's request limit at `now` (FR-024).
fn paused(state: &State, now: u64) -> bool {
    state.pause_until.is_some_and(|until| now < until)
}

/// Start a reading if every condition of contracts/reading-and-wire.md §1 holds.
fn start(state: &mut State, now: u64) -> Effect {
    if !state.enabled
        || !state.held
        || state.awaiting_listing
        || paused(state, now)
        || state.phase != Phase::Idle
    {
        return Effect::None;
    }
    let seq = state.next_seq;
    state.next_seq += 1;
    state.phase = Phase::Reading {
        seq,
        again: false,
        started: now,
    };
    Effect::Read { seq }
}
