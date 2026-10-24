//! The sidebar's state groups (feature 1125, FR-020, research R1): which group a location's card
//! state belongs to, the group order, the stable partition that keeps the sidebar's own order inside
//! a group, the header text, a run group's state and whether a group is open. The client feeds these
//! rules the window's state and turns their answer into rows; every rule with a branch lives here so
//! it is tested without the GUI (contract `sidebar-groups.md` G1–G6).
//!
//! Pure: no I/O, no clock and no logging (FR-023).

use crate::overview::CardState;

/// One of the sidebar's four groups, in display order: the derived `Ord` is the order the groups
/// are listed in (FR-001, G2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StateGroup {
    /// The user has to act, or something wants a look (Needs action and Needs attention).
    NeedsAction,
    /// A session is working.
    Working,
    /// Nothing is happening.
    Idle,
    /// The work is merged.
    Done,
}

impl StateGroup {
    /// Every group, in display order (G2).
    pub const ALL: [StateGroup; 4] = [Self::NeedsAction, Self::Working, Self::Idle, Self::Done];

    /// The group a card state is listed in (G1). Needs attention has no group of its own.
    pub fn of(state: CardState) -> Self {
        match state {
            CardState::NeedsAction | CardState::NeedsAttention => Self::NeedsAction,
            CardState::Working => Self::Working,
            CardState::Idle => Self::Idle,
            CardState::Done => Self::Done,
        }
    }

    /// The group's name on its header.
    pub fn label(self) -> &'static str {
        match self {
            Self::NeedsAction => "Needs action",
            Self::Working => "Working",
            Self::Idle => "Idle",
            Self::Done => "Done",
        }
    }

    /// Whether the group is open before the user chooses: every group but Done (FR-005).
    pub fn starts_open(self) -> bool {
        self != Self::Done
    }
}

/// The items of one non-empty group, in input order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group<T> {
    /// Which group.
    pub group: StateGroup,
    /// The group's items, in the order they came in (FR-003).
    pub items: Vec<T>,
}

/// Partition `items` by the group of their card state (G3): the groups that hold an item, in
/// display order, each with its items in input order.
pub fn group_by_state<T>(items: impl IntoIterator<Item = (CardState, T)>) -> Vec<Group<T>> {
    let mut groups: Vec<Group<T>> = StateGroup::ALL
        .iter()
        .map(|&group| Group {
            group,
            items: Vec::new(),
        })
        .collect();
    for (state, item) in items {
        let group = StateGroup::of(state);
        if let Some(slot) = groups.iter_mut().find(|g| g.group == group) {
            slot.items.push(item);
        }
    }
    groups.retain(|g| !g.items.is_empty());
    groups
}

/// A group header's text: its name, " · " and its row count (G4, FR-002).
pub fn header_text(group: StateGroup, count: usize) -> String {
    format!("{} \u{b7} {count}", group.label())
}

/// Whether a group is open (G5, research R4): the user's choice when there is one, else open when
/// the group starts open or holds the current session's row.
pub fn group_open(choice: Option<bool>, group: StateGroup, holds_current: bool) -> bool {
    choice.unwrap_or(group.starts_open() || holds_current)
}

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
    fn g1_every_card_state_has_one_group() {
        let expected = [
            (CardState::NeedsAction, StateGroup::NeedsAction),
            (CardState::NeedsAttention, StateGroup::NeedsAction),
            (CardState::Working, StateGroup::Working),
            (CardState::Done, StateGroup::Done),
            (CardState::Idle, StateGroup::Idle),
        ];
        assert_eq!(expected.len(), CardState::ALL.len());
        for state in CardState::ALL {
            let (_, group) = expected.iter().find(|(s, _)| *s == state).unwrap();
            assert_eq!(StateGroup::of(state), *group, "{state:?}");
        }
    }

    #[test]
    fn g2_order_is_needs_action_working_idle_done() {
        assert_eq!(
            StateGroup::ALL,
            [
                StateGroup::NeedsAction,
                StateGroup::Working,
                StateGroup::Idle,
                StateGroup::Done
            ]
        );
        let mut sorted = StateGroup::ALL;
        sorted.sort();
        assert_eq!(sorted, StateGroup::ALL);
        assert!(StateGroup::ALL.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn g3_partition_is_stable_and_drops_empty_groups() {
        let items = vec![
            (CardState::Idle, "a"),
            (CardState::NeedsAttention, "b"),
            (CardState::Done, "c"),
            (CardState::NeedsAction, "d"),
            (CardState::Idle, "e"),
        ];
        let groups = group_by_state(items.clone());
        assert_eq!(
            groups,
            vec![
                Group {
                    group: StateGroup::NeedsAction,
                    items: vec!["b", "d"]
                },
                Group {
                    group: StateGroup::Idle,
                    items: vec!["a", "e"]
                },
                Group {
                    group: StateGroup::Done,
                    items: vec!["c"]
                },
            ]
        );
        let total: usize = groups.iter().map(|g| g.items.len()).sum();
        assert_eq!(total, items.len());
        for (_, item) in &items {
            assert_eq!(
                groups.iter().filter(|g| g.items.contains(item)).count(),
                1,
                "{item}"
            );
        }
    }

    #[test]
    fn g3_attention_before_action_keeps_input_order() {
        let groups = group_by_state([(CardState::NeedsAction, 1), (CardState::NeedsAttention, 2)]);
        assert_eq!(groups[0].items, vec![1, 2]);
        let groups = group_by_state([(CardState::NeedsAttention, 1), (CardState::NeedsAction, 2)]);
        assert_eq!(groups[0].items, vec![1, 2]);
    }

    #[test]
    fn g3_empty_input_is_empty_output() {
        assert!(group_by_state(Vec::<(CardState, u8)>::new()).is_empty());
    }

    #[test]
    fn g4_header_text() {
        assert_eq!(header_text(StateGroup::NeedsAction, 2), "Needs action · 2");
        assert_eq!(header_text(StateGroup::Working, 1), "Working · 1");
        assert_eq!(header_text(StateGroup::Idle, 3), "Idle · 3");
        assert_eq!(header_text(StateGroup::Done, 4), "Done · 4");
    }

    #[test]
    fn g5_open_rule() {
        for group in StateGroup::ALL {
            for holds in [false, true] {
                assert!(group_open(Some(true), group, holds));
                assert!(!group_open(Some(false), group, holds));
            }
        }
        assert!(!group_open(None, StateGroup::Done, false));
        assert!(group_open(None, StateGroup::Done, true));
        for group in [
            StateGroup::NeedsAction,
            StateGroup::Working,
            StateGroup::Idle,
        ] {
            assert!(group_open(None, group, false));
            assert!(group_open(None, group, true));
        }
        for group in StateGroup::ALL {
            assert_eq!(group.starts_open(), group != StateGroup::Done);
        }
    }

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
