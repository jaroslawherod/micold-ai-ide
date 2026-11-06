//! A run group's state in the sidebar (feature 483, parallel-surfaces G6): the worst card state over
//! its runs. The sidebar lists its rows in one stable order (feature 1718), so this is the only rule
//! left of the state groups feature 1125 added. The rule has a branch, so it lives here and is tested
//! without the GUI.
//!
//! Pure: no I/O, no clock and no logging (FR-023).

use crate::overview::CardState;

/// What one run of a run group adds to the group's state (data-model *RunCard*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunCard {
    /// The run has a worktree, whose card state this is.
    Worktree(CardState),
    /// The run has no worktree (yet, or any more).
    NoWorktree {
        /// The run failed.
        failed: bool,
    },
}

impl RunCard {
    fn state(self) -> CardState {
        match self {
            Self::Worktree(state) => state,
            Self::NoWorktree { failed: true } => CardState::NeedsAttention,
            Self::NoWorktree { failed: false } => CardState::Idle,
        }
    }
}

/// A run group's state: the worst over its runs, `Idle` for none (G6).
pub fn run_group_state(runs: impl IntoIterator<Item = RunCard>) -> CardState {
    runs.into_iter()
        .map(RunCard::state)
        .min()
        .unwrap_or(CardState::Idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn g6_run_group_state() {
        assert_eq!(run_group_state([]), CardState::Idle);
        assert_eq!(
            run_group_state([RunCard::NoWorktree { failed: false }]),
            CardState::Idle
        );
        assert_eq!(
            run_group_state([RunCard::NoWorktree { failed: true }]),
            CardState::NeedsAttention
        );
        assert_eq!(
            run_group_state([RunCard::Worktree(CardState::Working)]),
            CardState::Working
        );
        assert_eq!(
            run_group_state([
                RunCard::Worktree(CardState::Done),
                RunCard::Worktree(CardState::Working),
                RunCard::NoWorktree { failed: false },
            ]),
            CardState::Working
        );
        assert_eq!(
            run_group_state([
                RunCard::Worktree(CardState::Working),
                RunCard::NoWorktree { failed: true },
            ]),
            CardState::NeedsAttention
        );
        assert_eq!(
            run_group_state([
                RunCard::NoWorktree { failed: true },
                RunCard::Worktree(CardState::NeedsAction),
            ]),
            CardState::NeedsAction
        );
    }
}
