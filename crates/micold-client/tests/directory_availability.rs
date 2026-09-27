//! Which AI CLIs each directory has, held per directory (feature 033, data-model.md).
//!
//! `AvailabilityAnswers` replaced one window-wide answer that Settings and every reconnect
//! overwrote with the home directory's, so a CLI that only one project's environment-include script
//! provides stayed unreachable in that very project. These pin the store as a pure type — filing,
//! staleness, fallback, pruning — and the rule that turns a sidebar row into the directory it is
//! answered for.

use micold_client::app::State;
use micold_client::features::session::{
    wanted_availability_dirs, AvailabilityAnswers, AvailabilityKey, AvailabilitySource,
    CliAvailability, EnvIncludeSettings,
};
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, SessionLocation};
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const P: &str = "/repo";
const Q: &str = "/repo/.claude/worktrees/feat-q";

fn answer(available: &[AiCli]) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::ThisComputer,
    }
}

fn dir(path: &str) -> AvailabilityKey {
    AvailabilityKey::Dir(PathBuf::from(path))
}

fn held(answers: &AvailabilityAnswers, path: &str) -> Option<Vec<AiCli>> {
    answers
        .for_dir(Path::new(path))
        .map(|a| a.available.clone())
}

fn wanted(paths: &[&str]) -> BTreeSet<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

// ---------------------------------------------------------------------------------------------
// AvailabilityAnswers (U1–U10)
// ---------------------------------------------------------------------------------------------

/// U1 (FR-003, R1).
#[test]
fn an_answer_is_filed_under_the_directory_its_request_named() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(7, dir(P));

    assert!(
        answers.answered(7, answer(&[AiCli::ClaudeCode, AiCli::Pi])),
        "an answer to the newest request for its directory is filed"
    );
    assert_eq!(
        held(&answers, P),
        Some(vec![AiCli::ClaudeCode, AiCli::Pi]),
        "and it is read back for the directory the request named"
    );
}

/// U2 (FR-009): the boundary on both sides — the older reply is dropped, the newer one kept.
#[test]
fn a_late_answer_to_an_older_request_for_the_same_directory_is_dropped() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, dir(P));
    answers.asked(2, dir(P));

    assert!(
        answers.answered(2, answer(&[AiCli::Pi])),
        "the newest request's answer is filed"
    );
    assert!(
        !answers.answered(1, answer(&[AiCli::ClaudeCode])),
        "a slower, older answer for the same directory describes a moment already superseded"
    );
    assert_eq!(held(&answers, P), Some(vec![AiCli::Pi]));
}

/// U3 (FR-009, FR-002): two directories never compete, in either arrival order.
#[test]
fn answers_for_different_directories_are_all_kept_in_any_order() {
    for reverse in [false, true] {
        let mut answers = AvailabilityAnswers::default();
        answers.asked(1, dir(P));
        answers.asked(2, dir(Q));
        let mut replies = vec![
            (1, answer(&[AiCli::ClaudeCode, AiCli::Pi])),
            (2, answer(&[AiCli::ClaudeCode])),
        ];
        if reverse {
            replies.reverse();
        }
        for (req, reply) in replies {
            assert!(answers.answered(req, reply), "reverse = {reverse}");
        }

        assert_eq!(
            held(&answers, P),
            Some(vec![AiCli::ClaudeCode, AiCli::Pi]),
            "P keeps its own answer whatever arrived after it (reverse = {reverse})"
        );
        assert_eq!(
            held(&answers, Q),
            Some(vec![AiCli::ClaudeCode]),
            "and so does Q (reverse = {reverse})"
        );
    }
}

/// U4 (FR-011, R4): a reply from before a reconnect names a request this store never recorded.
#[test]
fn an_answer_to_an_unknown_request_is_dropped() {
    let mut answers = AvailabilityAnswers::default();

    assert!(!answers.answered(3, answer(&[AiCli::Pi])));
    assert_eq!(held(&answers, P), None);
    assert_eq!(answers.home(), None);
}

/// U5 (FR-002, FR-008): home and a directory are different questions.
#[test]
fn home_and_directory_answers_never_replace_each_other() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, dir(P));
    answers.answered(1, answer(&[AiCli::ClaudeCode, AiCli::Pi]));
    answers.asked(2, AvailabilityKey::Home);
    answers.answered(2, answer(&[AiCli::ClaudeCode]));

    assert_eq!(
        held(&answers, P),
        Some(vec![AiCli::ClaudeCode, AiCli::Pi]),
        "a home answer arriving later (Settings, a reconnect) must not replace P's"
    );

    answers.asked(3, dir(P));
    answers.answered(3, answer(&[AiCli::Pi]));
    assert_eq!(
        answers.home().map(|a| a.available.clone()),
        Some(vec![AiCli::ClaudeCode]),
        "and a directory's answer never becomes the home answer Settings reads"
    );
}

/// U6 (FR-005).
#[test]
fn a_directory_reads_its_own_answer_and_falls_back_to_home() {
    let mut answers = AvailabilityAnswers::default();
    assert_eq!(
        held(&answers, P),
        None,
        "nothing held anywhere reads as nothing"
    );

    answers.asked(1, AvailabilityKey::Home);
    answers.answered(1, answer(&[AiCli::ClaudeCode]));
    assert_eq!(
        held(&answers, P),
        Some(vec![AiCli::ClaudeCode]),
        "a row whose own answer is pending reads the home answer"
    );

    answers.asked(2, dir(P));
    answers.answered(2, answer(&[AiCli::ClaudeCode, AiCli::Pi]));
    assert_eq!(
        held(&answers, P),
        Some(vec![AiCli::ClaudeCode, AiCli::Pi]),
        "and its own answer once it has one"
    );
}

/// U7 (FR-003, FR-012).
#[test]
fn retain_drops_answers_and_requests_for_directories_no_row_has() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, AvailabilityKey::Home);
    answers.answered(1, answer(&[AiCli::ClaudeCode]));
    answers.asked(2, dir(P));
    answers.answered(2, answer(&[AiCli::ClaudeCode, AiCli::Pi]));
    answers.asked(3, dir(Q));

    answers.retain(&wanted(&[]));

    assert_eq!(
        held(&answers, P),
        Some(vec![AiCli::ClaudeCode]),
        "P's answer is gone, so P reads home — which is kept"
    );
    assert!(
        !answers.answered(3, answer(&[AiCli::Pi])),
        "a request to a directory pruned since is dropped when it is answered"
    );
    assert_eq!(held(&answers, Q), Some(vec![AiCli::ClaudeCode]));
}

/// U8 (FR-006): once per distinct directory.
#[test]
fn unasked_lists_only_directories_neither_held_nor_asked() {
    let mut answers = AvailabilityAnswers::default();
    let r = "/repo/.claude/worktrees/feat-r";
    answers.asked(1, dir(P));
    answers.answered(1, answer(&[AiCli::ClaudeCode]));
    answers.asked(2, dir(Q));

    assert_eq!(
        answers.unasked(&wanted(&[P, Q, r])),
        vec![PathBuf::from(r)],
        "P is held and Q is in flight, so only R needs asking"
    );
}

/// U9 (FR-011).
#[test]
fn clear_forgets_every_answer_and_request() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, AvailabilityKey::Home);
    answers.answered(1, answer(&[AiCli::ClaudeCode]));
    answers.asked(2, dir(P));
    answers.answered(2, answer(&[AiCli::Pi]));
    answers.asked(3, dir(Q));

    answers.clear();

    assert_eq!(answers.home(), None);
    assert_eq!(held(&answers, P), None);
    assert!(
        !answers.answered(3, answer(&[AiCli::Pi])),
        "a request from the previous connection is no longer recognised"
    );
    assert_eq!(answers.unasked(&wanted(&[Q])), vec![PathBuf::from(Q)]);
}

/// U10 (FR-004, R6).
#[test]
fn env_include_changed_reports_only_a_real_change() {
    let on = EnvIncludeSettings {
        enabled: true,
        script_path: "/home/u/.env.sh".into(),
        timeout_secs: 5,
    };
    let off = EnvIncludeSettings {
        enabled: false,
        ..on.clone()
    };
    let mut answers = AvailabilityAnswers::default();

    assert!(answers.env_include_changed(&on), "unset → set is a change");
    assert!(
        !answers.env_include_changed(&on),
        "the same settings are not"
    );
    assert!(answers.env_include_changed(&off), "switching it off is");
    assert!(
        !answers.env_include_changed(&off),
        "and the new settings were recorded"
    );
}

// ---------------------------------------------------------------------------------------------
// Which directory a row is answered for, and which rows exist (U16–U21)
// ---------------------------------------------------------------------------------------------

fn worktree(name: &str, status: WorktreeStatus) -> Worktree {
    Worktree {
        dir_name: name.to_string(),
        path: PathBuf::from(format!("{P}/.claude/worktrees/{name}")),
        branch: Some(format!("feat/{name}")),
        status,
        included: false,
    }
}

/// A project at `/repo` whose `recorded` worktrees the app created and whose `agent` ones it did
/// not (so they are hidden until revealed, 029 FR-004).
fn state_with(recorded: Vec<Worktree>, agent: Vec<Worktree>) -> State {
    let mut state = State::default();
    let root = PathBuf::from(P);
    state.workspace.projects.push(Project {
        path: root.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    state.workspace.active = Some(root.clone());
    for w in &recorded {
        state.workspace.record_user_created(&root, &w.dir_name);
    }
    state.worktree.worktrees = recorded.into_iter().chain(agent).collect();
    state
}

fn worktree_dir(name: &str) -> PathBuf {
    PathBuf::from(format!("{P}/.claude/worktrees/{name}"))
}

/// U16 (FR-007): the same rule the spawn uses, `SessionLocation::cwd`.
#[test]
fn a_row_is_answered_for_the_directory_its_session_would_run_in() {
    let state = state_with(Vec::new(), Vec::new());
    assert_eq!(
        state.location_dir(&SessionLocation::Default),
        Some(PathBuf::from(P))
    );
    assert_eq!(
        state.location_dir(&SessionLocation::Worktree("feat-a".into())),
        Some(worktree_dir("feat-a"))
    );
    assert_eq!(
        State::default().location_dir(&SessionLocation::Default),
        None,
        "no active project, no row, no directory"
    );
}

/// U17 (FR-006, FR-007).
#[test]
fn the_wanted_set_is_the_rows_on_screen() {
    let state = state_with(
        vec![
            worktree("feat-a", WorktreeStatus::Valid),
            worktree("feat-b", WorktreeStatus::Valid),
        ],
        Vec::new(),
    );
    assert_eq!(
        wanted_availability_dirs(&state),
        [
            PathBuf::from(P),
            worktree_dir("feat-a"),
            worktree_dir("feat-b")
        ]
        .into_iter()
        .collect::<BTreeSet<_>>()
    );
}

/// U18 (FR-003, SC-004): a hidden row is not asked about.
#[test]
fn hidden_agent_worktrees_are_not_asked_about_until_revealed() {
    let mut state = state_with(
        vec![worktree("feat-a", WorktreeStatus::Valid)],
        vec![worktree("agent-1f", WorktreeStatus::Valid)],
    );
    assert!(
        !wanted_availability_dirs(&state).contains(&worktree_dir("agent-1f")),
        "a hidden agent worktree has no row to answer for"
    );

    state.sidebar.show_agent_worktrees = true;
    assert!(
        wanted_availability_dirs(&state).contains(&worktree_dir("agent-1f")),
        "once revealed it has one"
    );
}

/// U19 (FR-004, deleted): a `Missing` worktree cannot start a session, so it offers nothing.
#[test]
fn a_worktree_whose_directory_is_gone_is_not_asked_about() {
    let state = state_with(
        vec![
            worktree("feat-a", WorktreeStatus::Valid),
            worktree("feat-gone", WorktreeStatus::Missing),
        ],
        Vec::new(),
    );
    assert!(!wanted_availability_dirs(&state).contains(&worktree_dir("feat-gone")));
}

/// U20 (FR-007, R3): keyed the way the row's reader keys it, or its answer would never be read.
#[test]
fn an_included_worktree_is_keyed_like_its_reader() {
    let mut included = worktree("elsewhere", WorktreeStatus::Valid);
    included.path = PathBuf::from("/somewhere/else/elsewhere");
    included.included = true;
    let state = state_with(vec![included], Vec::new());

    let wanted = wanted_availability_dirs(&state);
    let key = state
        .location_dir(&SessionLocation::Worktree("elsewhere".into()))
        .expect("an active project");
    assert!(
        wanted.contains(&key),
        "the wanted set must hold the key the row reads: {wanted:?}"
    );
    assert!(!wanted.contains(Path::new("/somewhere/else/elsewhere")));
}

/// U21 (FR-012).
#[test]
fn no_project_no_wanted_directories() {
    assert!(wanted_availability_dirs(&State::default()).is_empty());
}
