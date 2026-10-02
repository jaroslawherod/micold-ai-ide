//! Which AI CLIs each directory has, held per directory (feature 033, data-model.md).
//!
//! `AvailabilityAnswers` replaced one window-wide answer that Settings and every reconnect
//! overwrote with the home directory's, so a CLI that only one project's environment-include script
//! provides stayed unreachable in that very project. These pin the store as a pure type — filing,
//! staleness, fallback, pruning — and the rule that turns a sidebar row into the directory it is
//! answered for.

use micold_client::app::{drain, interpret, State};
use micold_client::features::session::{
    start_menu_toggled, wanted_availability_dirs, AvailabilityAnswers, AvailabilityKey,
    AvailabilitySource, CliAvailability, EnvIncludeSettings,
};
use micold_core::cli_reason::{start_refusal, AttemptDir, Place, SpawnEnv};
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, SessionLocation};
use micold_core::terminal::LaunchMode;
use micold_core::worktree::{Worktree, WorktreeStatus};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

const P: &str = "/repo";
const Q: &str = "/repo/.claude/worktrees/feat-q";

fn answer(available: &[AiCli]) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::ThisComputer,
        env: None,
        asked_for: AvailabilityKey::Home,
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

// --- Feature 037, contract A4: an answer carries its environment state and the key it is for ---

/// An answer in `env`, as the shell builds it: it does not know which request it answers, so
/// `asked_for` holds a key no request here names.
fn answer_in(available: &[AiCli], env: Option<SpawnEnv>) -> CliAvailability {
    CliAvailability {
        available: available.to_vec(),
        source: AvailabilitySource::ThisComputer,
        env,
        asked_for: dir("/not/the/key/of/any/request"),
    }
}

/// 037 U40 (C2, FR-004a).
#[test]
fn an_answer_to_a_request_without_a_directory_is_stamped_as_the_home_directorys() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, AvailabilityKey::Home);
    answers.answered(
        1,
        answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::ScriptTimedOut)),
    );

    assert_eq!(
        answers.home().map(|a| (a.asked_for.clone(), a.env)),
        Some((AvailabilityKey::Home, Some(SpawnEnv::ScriptTimedOut))),
        "the request named no directory, so the attempt the answer reports was for the home \
         directory, and a reason must say so (FR-004a)"
    );
}

/// 037 U41 (C2, FR-004a).
#[test]
fn an_answer_to_a_request_for_a_directory_is_stamped_with_that_directory() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(2, dir(P));
    answers.answered(
        2,
        answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::ScriptFailed)),
    );

    assert_eq!(
        answers.for_dir(Path::new(P)).map(|a| a.asked_for.clone()),
        Some(dir(P)),
        "the attempt the answer reports was made in the directory the request named"
    );
}

/// 037 U42 (C3, FR-004a, FR-012): a row drawn from the home answer gives the home answer's reason
/// for the home directory, and its own reason for its own directory once it has one.
#[test]
fn a_row_reads_the_home_answers_state_until_its_own_arrives() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, AvailabilityKey::Home);
    answers.answered(
        1,
        answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::IncludeOff)),
    );

    assert_eq!(
        answers
            .for_dir(Path::new(P))
            .map(|a| (a.asked_for.clone(), a.env)),
        Some((AvailabilityKey::Home, Some(SpawnEnv::IncludeOff))),
        "while the row has no answer of its own, what it offers and the reason both come from \
         the home answer, which is about the home directory"
    );

    answers.asked(2, dir(P));
    answers.answered(
        2,
        answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::ScriptTimedOut)),
    );

    assert_eq!(
        answers
            .for_dir(Path::new(P))
            .map(|a| (a.asked_for.clone(), a.env)),
        Some((dir(P), Some(SpawnEnv::ScriptTimedOut))),
        "the row's own answer brings its own state and its own directory together"
    );
}

/// 037 U43 (C2, FR-012, FR-013): the offer and the state are replaced as one value.
#[test]
fn a_newer_answer_replaces_the_state_together_with_the_set() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, AvailabilityKey::Home);
    answers.answered(
        1,
        answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::IncludeOff)),
    );
    answers.asked(2, AvailabilityKey::Home);
    answers.answered(
        2,
        answer_in(&[AiCli::ClaudeCode, AiCli::Pi], Some(SpawnEnv::Applied)),
    );

    assert_eq!(
        answers.home().map(|a| (a.available.clone(), a.env)),
        Some((vec![AiCli::ClaudeCode, AiCli::Pi], Some(SpawnEnv::Applied))),
        "after environment-include is turned on and saved, the next answer changes what is \
         offered and the state in one step"
    );
}

/// 037 U44 (C2, FR-012): the other side of U43.
#[test]
fn an_older_answer_changes_neither_the_set_nor_the_state() {
    let mut answers = AvailabilityAnswers::default();
    answers.asked(1, AvailabilityKey::Home);
    answers.asked(2, AvailabilityKey::Home);
    answers.answered(2, answer_in(&[AiCli::Pi], Some(SpawnEnv::Applied)));

    assert!(
        !answers.answered(1, answer_in(&[], Some(SpawnEnv::IncludeOff))),
        "fixture check: the older answer is dropped"
    );
    assert_eq!(
        answers.home().map(|a| (a.available.clone(), a.env)),
        Some((vec![AiCli::Pi], Some(SpawnEnv::Applied))),
        "a dropped answer leaves no part of itself behind: not its set and not its state"
    );
}

/// 037 U75 (FR-012, contract W6): the missing-default message is an event, said once from the
/// answer in use at the press. A newer answer changes what the row offers and what a later press
/// would say. It does not say anything itself, and it does not rewrite what was said.
#[test]
fn a_newer_answer_after_the_missing_default_message_says_nothing_and_leaves_it_as_said() {
    let mut state = state_with(Vec::new(), Vec::new());
    state.session.default_ai_cli = AiCli::Pi;
    state.session.availability.asked(1, dir(P));
    state.session.availability.answered(
        1,
        answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::ScriptTimedOut)),
    );

    let outcomes = start_menu_toggled(&mut state, SessionLocation::Default, Some(AiCli::Pi));
    drain(outcomes, |outcome| interpret(&mut state, outcome));

    let first = start_refusal(
        AiCli::Pi,
        SpawnEnv::ScriptTimedOut,
        Place::ThisComputer,
        AttemptDir::Dir(Path::new(P)),
        LaunchMode::Fresh,
    );
    let said = |state: &State| {
        let queue = &state.notifications.queue;
        (queue.visible().map(|n| n.message.clone()), queue.pending())
    };
    assert_eq!(said(&state), (Some(first.clone()), 0));

    // The press itself asked again (033 contract C1), and the script now runs and still does not
    // provide Pi: another state, and another sentence if anything were to say it.
    state.session.availability.asked(2, dir(P));
    assert!(state
        .session
        .availability
        .answered(2, answer_in(&[AiCli::ClaudeCode], Some(SpawnEnv::Applied)),));
    assert_eq!(
        said(&state),
        (Some(first.clone()), 0),
        "the message on screen is the one said at the press, and nothing waits behind it"
    );

    // Then the CLI is found. The row offers it, and the message is still what was said.
    state.session.availability.asked(3, dir(P));
    assert!(state.session.availability.answered(
        3,
        answer_in(&[AiCli::ClaudeCode, AiCli::Pi], Some(SpawnEnv::Applied)),
    ));
    assert_eq!(said(&state), (Some(first), 0));
    assert_eq!(
        held(&state.session.availability, P),
        Some(vec![AiCli::ClaudeCode, AiCli::Pi])
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

// ---------------------------------------------------------------------------------------------
// 037 surface U6: the note in a row's CLI list (U78–U85)
// ---------------------------------------------------------------------------------------------

const TWO: &[AiCli] = &[AiCli::ClaudeCode, AiCli::Copilot];
const HOME_KEY: u64 = 1;

/// `explain`'s `{reason} {action}` for `missing` on this computer (contract W4, surface U6).
fn explained(missing: &[AiCli], env: SpawnEnv, dir: AttemptDir<'_>) -> String {
    let said = micold_core::cli_reason::explain(missing, env, Place::ThisComputer, dir)
        .expect("something is missing");
    format!("{} {}", said.reason, said.action)
}

/// A state whose answer for `P` is `available` in `env`, and which holds no home answer.
fn row_answered(available: &[AiCli], env: Option<SpawnEnv>) -> State {
    let mut state = State::default();
    state.session.availability.asked(2, dir(P));
    state
        .session
        .availability
        .answered(2, answer_in(available, env));
    state
}

fn note(state: &State, path: &str) -> Option<String> {
    state.session.start_menu_note(Path::new(path))
}

/// 037 U78 (FR-010, US3-AS1, SC-007, R6).
#[test]
fn a_list_of_two_says_which_cli_is_missing_and_why_for_the_directory_asked_about() {
    let state = row_answered(TWO, Some(SpawnEnv::ScriptFailed));

    assert_eq!(
        note(&state, P),
        Some(explained(
            &[AiCli::Pi],
            SpawnEnv::ScriptFailed,
            AttemptDir::Dir(Path::new(P))
        )),
        "the note is `explain`'s reason and action for what the answer lacks, about the \
         directory the answer was asked for"
    );
}

/// 037 U79 (FR-011, US3-AS2, W5).
#[test]
fn a_list_of_every_cli_has_no_note() {
    let state = row_answered(&AiCli::ALL, Some(SpawnEnv::Applied));
    assert_eq!(note(&state, P), None);
}

/// 037 U80 (FR-010, US3-AS1a, US3-AS1b, D5, D6): the other side of U78's "two or more".
#[test]
fn a_list_of_one_has_no_note() {
    let state = row_answered(&[AiCli::ClaudeCode], Some(SpawnEnv::ScriptFailed));
    assert_eq!(
        note(&state, P),
        None,
        "a row with one CLI has no chevron, and the list a missing default opens there is \
         explained by its message (D6)"
    );
}

/// 037 U81 (FR-010, Edge Cases nothing is available).
#[test]
fn a_list_of_none_has_no_note() {
    let state = row_answered(&[], Some(SpawnEnv::ScriptFailed));
    assert_eq!(note(&state, P), None);
}

/// 037 U82 (FR-011, W5): a CLI is missing and the answer does not say why.
#[test]
fn an_answer_without_a_state_has_no_note() {
    let state = row_answered(TWO, None);
    assert_eq!(note(&state, P), None);
}

/// 037 U83 (FR-011, Edge Cases not answered yet).
#[test]
fn no_answer_in_use_has_no_note() {
    assert_eq!(note(&State::default(), P), None);
}

/// 037 U84 (FR-004a, FR-012, D7): the reason and the offer come from one answer.
#[test]
fn a_row_on_the_home_answer_names_the_home_directory_until_its_own_answer_is_filed() {
    let mut state = State::default();
    state
        .session
        .availability
        .asked(HOME_KEY, AvailabilityKey::Home);
    state
        .session
        .availability
        .answered(HOME_KEY, answer_in(TWO, Some(SpawnEnv::ScriptTimedOut)));

    let on_home = note(&state, P).expect("the home answer is in use for the row");
    assert_eq!(
        on_home,
        explained(&[AiCli::Pi], SpawnEnv::ScriptTimedOut, AttemptDir::Home)
    );
    assert!(on_home.contains("your home directory"), "{on_home}");

    state.session.availability.asked(2, dir(P));
    state
        .session
        .availability
        .answered(2, answer_in(TWO, Some(SpawnEnv::ScriptFailed)));

    assert_eq!(
        note(&state, P),
        Some(explained(
            &[AiCli::Pi],
            SpawnEnv::ScriptFailed,
            AttemptDir::Dir(Path::new(P))
        )),
        "the row's own answer brings its own state and its own directory"
    );
}

/// 037 U85 (US3-AS3, FR-012, Edge Cases several rows at once).
#[test]
fn two_rows_each_get_their_own_note_and_filing_one_does_not_change_the_other() {
    let mut state = row_answered(TWO, Some(SpawnEnv::ScriptFailed));
    state.session.availability.asked(3, dir(Q));
    state.session.availability.answered(
        3,
        answer_in(&[AiCli::ClaudeCode, AiCli::Pi], Some(SpawnEnv::IncludeOff)),
    );
    let p_says = explained(
        &[AiCli::Pi],
        SpawnEnv::ScriptFailed,
        AttemptDir::Dir(Path::new(P)),
    );
    assert_eq!(note(&state, P), Some(p_says.clone()));
    assert_eq!(
        note(&state, Q),
        Some(explained(
            &[AiCli::Copilot],
            SpawnEnv::IncludeOff,
            AttemptDir::Dir(Path::new(Q))
        ))
    );

    state.session.availability.asked(4, dir(Q));
    state
        .session
        .availability
        .answered(4, answer_in(&AiCli::ALL, Some(SpawnEnv::Applied)));

    assert_eq!(note(&state, Q), None, "Q's newer answer lacks nothing");
    assert_eq!(note(&state, P), Some(p_says), "and P's note is still P's");
}
