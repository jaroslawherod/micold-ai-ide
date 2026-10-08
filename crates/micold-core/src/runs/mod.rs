//! Parallel agent runs (feature 483): one prompt run across several agents, each in its own
//! worktree, kept together as a [`RunGroup`] (data-model.md § Core).
//!
//! Render-free and serialisable. Statuses are closed enums so an impossible run state is
//! unrepresentable (Principle V); [`RunStatus::can_become`] is the only transition table.

pub mod integrate;
pub mod naming;
pub mod store;
pub mod summary;

use std::fmt;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::naming::{DerivedNames, WorktreeNaming};
use crate::review::RelPath;
use crate::session::{AiCli, SessionId};

/// The fewest runs a group starts with (research R6).
pub const MIN_RUNS: usize = 2;
/// How many runs the Run in parallel dialog opens with (research R6).
pub const DEFAULT_RUNS: usize = 2;
/// The most runs a group can hold (research R6).
pub const MAX_RUNS: usize = 8;

/// A group's identity on the wire and in the runs file, as [`SessionId`] is a session's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct GroupId(pub Uuid);

impl GroupId {
    /// A fresh random id for a new group.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for GroupId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for GroupId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// One prompt run across several agents (data-model.md § `RunGroup`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunGroup {
    /// Identity.
    pub id: GroupId,
    /// The user's free-text name, shown on the group row (FR-007).
    pub name: String,
    /// What the runs' names were derived from (R5).
    pub naming: WorktreeNaming,
    /// The one prompt sent to every run (FR-004). Never logged.
    pub prompt: String,
    /// The branch every run started from and a pick integrates into.
    pub base_branch: String,
    /// `base_branch`'s tip when the group started.
    pub base_commit: String,
    /// Creation time, for ordering groups.
    pub created: SystemTime,
    /// The runs, ordered by `number`.
    pub runs: Vec<Run>,
    /// The picked run's number; `None` until a pick succeeds (FR-018).
    pub winner: Option<u8>,
}

/// What a new group is made of, before [`RunGroup::new`] checks it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewGroup {
    /// Identity.
    pub id: GroupId,
    /// The user's free-text name.
    pub name: String,
    /// What the runs' names were derived from.
    pub naming: WorktreeNaming,
    /// The prompt.
    pub prompt: String,
    /// The base branch.
    pub base_branch: String,
    /// The base branch's tip now.
    pub base_commit: String,
    /// Now.
    pub created: SystemTime,
    /// The runs, numbered `1..=runs.len()`.
    pub runs: Vec<Run>,
}

/// One run of a group: one worktree, one session of one provider (data-model.md § `Run`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Run {
    /// 1-based position within the group (FR-003, FR-007). Never reused.
    pub number: u8,
    /// The provider of its session.
    pub provider: AiCli,
    /// Its worktree folder and branch (R5).
    pub names: DerivedNames,
    /// Its session, once the session record exists.
    pub session: Option<SessionId>,
    /// Where it got to.
    pub status: RunStatus,
}

/// Which step of a run failed (FR-005), so it is reportable without parsing the reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RunStep {
    /// Creating the run's worktree.
    Worktree,
    /// Starting the run's session.
    Session,
}

/// Where a run got to (data-model.md § `RunStatus`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    /// Its worktree is being created.
    Creating,
    /// Its session is starting.
    Starting,
    /// The prompt was typed; live status comes from the session.
    Prompted,
    /// The session started but the prompt was not delivered; worktree and session are kept.
    PromptNotDelivered {
        /// Why, as `FirstPromptUndelivered::message` words it.
        reason: String,
    },
    /// A step failed (FR-005, FR-019).
    Failed {
        /// The step that failed.
        step: RunStep,
        /// Why.
        reason: String,
    },
    /// This run won the group. Terminal.
    Picked,
}

impl RunStatus {
    /// Whether a run in this status may move to `next` — the only transitions there are.
    pub fn can_become(&self, next: &RunStatus) -> bool {
        use RunStatus::*;
        matches!(
            (self, next),
            (Creating, Starting)
                | (
                    Creating,
                    Failed {
                        step: RunStep::Worktree,
                        ..
                    }
                )
                | (Starting, Prompted)
                | (Starting, PromptNotDelivered { .. })
                | (
                    Starting,
                    Failed {
                        step: RunStep::Session,
                        ..
                    }
                )
                | (Prompted, Picked)
                | (PromptNotDelivered { .. }, Picked)
        )
    }

    /// Whether the run failed.
    pub fn is_failed(&self) -> bool {
        matches!(self, RunStatus::Failed { .. })
    }
}

/// A run's changes against its base (derived for Compare, never persisted; R7, R8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSummary {
    /// Files changed.
    pub files: u32,
    /// Lines added.
    pub added: u32,
    /// Lines removed.
    pub removed: u32,
    /// Whether the run's worktree holds uncommitted changes.
    pub uncommitted: bool,
}

/// Why a pick changed nothing (FR-012, FR-013, SC-006).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PickRefusal {
    /// The group already has a winner.
    AlreadyPicked,
    /// The run cannot be picked; the reason says why.
    RunUnavailable(String),
    /// The run holds uncommitted changes in these files.
    Uncommitted {
        /// The files.
        files: Vec<RelPath>,
    },
    /// Merging would conflict in these files.
    Conflicts {
        /// The files.
        files: Vec<RelPath>,
    },
    /// The base branch's checkout is busy; the reason says how.
    BaseBusy(String),
    /// The base branch moved while the pick ran.
    BaseMoved,
    /// The installed git cannot merge without a checkout.
    GitTooOld,
    /// git failed; its message.
    Git(String),
}

/// What a successful pick did to the base branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Integration {
    /// The base branch moved forward to the run's tip.
    FastForward {
        /// The base branch's new tip.
        base_tip: String,
    },
    /// A merge commit joined the run's branch into the base branch.
    MergeCommit {
        /// The merge commit.
        commit: String,
    },
}

/// Why a group breaks an invariant of data-model.md § `RunGroup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidGroup(pub String);

impl fmt::Display for InvalidGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for InvalidGroup {}

impl RunGroup {
    /// A new group: `MIN_RUNS..=MAX_RUNS` runs numbered exactly `1..=runs.len()`, no winner.
    pub fn new(new: NewGroup) -> Result<Self, InvalidGroup> {
        let count = new.runs.len();
        if !(MIN_RUNS..=MAX_RUNS).contains(&count) {
            return Err(InvalidGroup(format!(
                "a group takes {MIN_RUNS} to {MAX_RUNS} runs, not {count}"
            )));
        }
        let numbered_from_one = new
            .runs
            .iter()
            .zip(1..)
            .all(|(run, expected)| usize::from(run.number) == expected);
        if !numbered_from_one {
            return Err(InvalidGroup(format!(
                "a new group's runs are numbered 1 to {count} in order"
            )));
        }
        Ok(Self {
            id: new.id,
            name: new.name,
            naming: new.naming,
            prompt: new.prompt,
            base_branch: new.base_branch,
            base_commit: new.base_commit,
            created: new.created,
            runs: new.runs,
            winner: None,
        })
    }

    /// The invariants a group keeps for its whole life, checked on load.
    pub fn validate(&self) -> Result<(), InvalidGroup> {
        if self.runs.is_empty() {
            return Err(InvalidGroup("a group holds at least one run".into()));
        }
        if self.runs.len() > MAX_RUNS {
            return Err(InvalidGroup(format!(
                "a group holds at most {MAX_RUNS} runs"
            )));
        }
        let in_range = |number: u8| (1..=MAX_RUNS).contains(&usize::from(number));
        let mut previous = 0;
        for run in &self.runs {
            if !in_range(run.number) || run.number <= previous {
                return Err(InvalidGroup(format!(
                    "run numbers are distinct, increasing and within 1 to {MAX_RUNS}; #{} is not",
                    run.number
                )));
            }
            previous = run.number;
        }
        if let Some(winner) = self.winner.filter(|&winner| !in_range(winner)) {
            return Err(InvalidGroup(format!(
                "the winner #{winner} is outside 1 to {MAX_RUNS}"
            )));
        }
        Ok(())
    }

    /// How many runs failed.
    pub fn failed_count(&self) -> usize {
        self.runs
            .iter()
            .filter(|run| run.status.is_failed())
            .count()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::naming::ConventionalType;

    pub(crate) fn naming() -> WorktreeNaming {
        WorktreeNaming {
            type_: Some(ConventionalType::Feat),
            ticket: None,
            name: "login page".into(),
        }
    }

    pub(crate) fn run(number: u8, status: RunStatus) -> Run {
        Run {
            number,
            provider: AiCli::ClaudeCode,
            names: DerivedNames {
                dir_name: format!("feat-login-page-{number}"),
                branch: format!("feat/login-page-{number}"),
            },
            session: None,
            status,
        }
    }

    fn new_group(numbers: &[u8]) -> NewGroup {
        NewGroup {
            id: GroupId(Uuid::from_u128(483)),
            name: "login page".into(),
            naming: naming(),
            prompt: "Add a login page".into(),
            base_branch: "main".into(),
            base_commit: "0123abcd".into(),
            created: SystemTime::UNIX_EPOCH,
            runs: numbers
                .iter()
                .map(|&n| run(n, RunStatus::Creating))
                .collect(),
        }
    }

    fn loaded(numbers: &[u8], winner: Option<u8>) -> RunGroup {
        let new = new_group(numbers);
        RunGroup {
            id: new.id,
            name: new.name,
            naming: new.naming,
            prompt: new.prompt,
            base_branch: new.base_branch,
            base_commit: new.base_commit,
            created: new.created,
            runs: new.runs,
            winner,
        }
    }

    #[test]
    fn the_run_count_limits_are_those_of_research_r6() {
        assert_eq!((MIN_RUNS, DEFAULT_RUNS, MAX_RUNS), (2, 2, 8));
    }

    #[test]
    fn a_new_group_takes_two_to_eight_runs_numbered_from_one() {
        for count in MIN_RUNS..=MAX_RUNS {
            let numbers: Vec<u8> = (1..=count as u8).collect();
            let group = RunGroup::new(new_group(&numbers))
                .unwrap_or_else(|err| panic!("{count} runs are a group: {err}"));
            assert_eq!(group.runs.len(), count);
            assert_eq!(group.winner, None, "a new group has no winner");
        }
    }

    #[test]
    fn a_new_group_refuses_one_run_nine_runs_and_other_numbering() {
        for numbers in [
            vec![1u8],
            (1..=9).collect::<Vec<u8>>(),
            vec![1, 3],
            vec![0, 1],
            vec![2, 1],
            vec![1, 1],
            vec![2, 3],
        ] {
            assert!(
                RunGroup::new(new_group(&numbers)).is_err(),
                "runs numbered {numbers:?} are not a new group"
            );
        }
    }

    #[test]
    fn a_loaded_group_may_have_gaps_one_run_and_a_removed_winner() {
        for (numbers, winner) in [
            (vec![1u8, 3], None),
            (vec![2u8], None),
            (vec![1u8, 3], Some(2u8)),
            (vec![1u8, 2, 3, 4, 5, 6, 7, 8], Some(8)),
        ] {
            assert_eq!(
                loaded(&numbers, winner).validate(),
                Ok(()),
                "runs {numbers:?} with winner {winner:?} are a valid loaded group"
            );
        }
    }

    #[test]
    fn a_loaded_group_refuses_no_runs_repeats_disorder_and_out_of_range_numbers() {
        for (numbers, winner) in [
            (vec![], None),
            (vec![1u8, 1], None),
            (vec![2u8, 1], None),
            (vec![0u8, 1], None),
            (vec![1u8, 9], None),
            (vec![1u8, 2], Some(0u8)),
            (vec![1u8, 2], Some(9u8)),
        ] {
            assert!(
                loaded(&numbers, winner).validate().is_err(),
                "runs {numbers:?} with winner {winner:?} are refused"
            );
        }
        let mut too_many = loaded(&[1, 2, 3, 4, 5, 6, 7, 8], None);
        too_many.runs.push(run(8, RunStatus::Creating));
        assert!(too_many.validate().is_err(), "nine runs are refused");
    }

    fn every_status() -> Vec<RunStatus> {
        vec![
            RunStatus::Creating,
            RunStatus::Starting,
            RunStatus::Prompted,
            RunStatus::PromptNotDelivered { reason: "r".into() },
            RunStatus::Failed {
                step: RunStep::Worktree,
                reason: "r".into(),
            },
            RunStatus::Failed {
                step: RunStep::Session,
                reason: "r".into(),
            },
            RunStatus::Picked,
        ]
    }

    #[test]
    fn only_the_data_model_transitions_are_allowed() {
        use RunStatus::*;
        let allowed = |from: &RunStatus, to: &RunStatus| match (from, to) {
            (Creating, Starting) => true,
            (
                Creating,
                Failed {
                    step: RunStep::Worktree,
                    ..
                },
            ) => true,
            (Starting, Prompted | PromptNotDelivered { .. }) => true,
            (
                Starting,
                Failed {
                    step: RunStep::Session,
                    ..
                },
            ) => true,
            (Prompted | PromptNotDelivered { .. }, Picked) => true,
            _ => false,
        };
        let statuses = every_status();
        let mut seen_allowed = 0;
        for from in &statuses {
            for to in &statuses {
                let expected = allowed(from, to);
                seen_allowed += usize::from(expected);
                assert_eq!(
                    from.can_become(to),
                    expected,
                    "{from:?} -> {to:?} should be {}",
                    if expected { "allowed" } else { "refused" }
                );
            }
        }
        assert_eq!(seen_allowed, 7, "seven transitions exist");
    }

    #[test]
    fn a_group_id_serialises_as_a_bare_uuid_string() {
        let id = GroupId(Uuid::from_u128(0x483));
        assert_eq!(
            serde_json::to_value(id).expect("serialises"),
            serde_json::json!(Uuid::from_u128(0x483).to_string())
        );
        let session = SessionId::from_uuid(Uuid::from_u128(0x483));
        assert_eq!(
            serde_json::to_value(id).expect("serialises"),
            serde_json::to_value(session).expect("serialises"),
            "a group id is shaped as a session id"
        );
    }
}
