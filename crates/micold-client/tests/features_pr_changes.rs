//! The Changes view's pull request list, in isolation (feature 775, M1: contracts/changes-view.md,
//! data-model §2). Builds only `features::changes::State` and drives `update`.

use std::path::PathBuf;

use micold_client::features::changes::{
    self, base_line, check_mark, diff_body, list_body, pr_mark, DiffBody, Effect, ListBody, Msg,
    PrDiffLoad, PrList, Source, State,
};
use micold_client::ui::PrMark;
use micold_core::pull_request::{
    BranchPrState, BranchPullRequest, DiffReadFailure, ReadingFailure,
};
use micold_core::review::diff::FileDiff;
use micold_core::review::pr_diff::split_pr_diff;
use micold_core::review::RelPath;
use micold_core::session::SessionLocation;

fn project() -> PathBuf {
    PathBuf::from("/projects/p")
}

fn worktree() -> SessionLocation {
    SessionLocation::Worktree("feat-a".into())
}

fn pr(number: u64, state: BranchPrState) -> BranchPullRequest {
    BranchPullRequest {
        number,
        title: format!("title {number}"),
        url: format!("https://github.com/acme/widgets/pull/{number}"),
        state,
        created_at: "2026-10-10T09:00:00Z".into(),
        base: "main".into(),
        head: "abc".into(),
        head_branch: "feat/a".into(),
        checks: micold_core::pull_request::CheckStatus::None,
    }
}

/// A state with the setting on and the view of `worktree()` open; the list read it asked for is
/// answered nowhere, as the pull request list does not depend on it.
fn opened(enabled: bool) -> State {
    let mut state = State::default();
    changes::update(&mut state, Msg::PullRequestSettingChanged(enabled));
    let effect = changes::update(
        &mut state,
        Msg::Opened {
            project: project(),
            entry: worktree(),
        },
    );
    assert!(
        matches!(effect, Effect::ReadList { .. }),
        "FR-009: the list read comes first"
    );
    state
}

fn wanted(state: &mut State, branch: Option<&str>) -> Effect {
    changes::update(
        state,
        Msg::PullRequestsWanted {
            branch: branch.map(str::to_string),
        },
    )
}

fn view_list(state: &State) -> &PrList {
    &state.open.as_ref().expect("open").pull_requests
}

fn seq_of(effect: Effect) -> u64 {
    match effect {
        Effect::ReadPullRequests { seq, .. } => seq,
        other => panic!("expected ReadPullRequests, got {other:?}"),
    }
}

#[test]
fn opening_reads_the_list_first_and_the_pull_requests_apart_from_it() {
    let mut state = opened(true);
    let effect = wanted(&mut state, Some("feat/a"));
    let Effect::ReadPullRequests {
        project: p,
        entry,
        current,
        ..
    } = effect
    else {
        panic!("expected ReadPullRequests, got {effect:?}");
    };
    assert_eq!(
        (p, entry, current),
        (project(), worktree(), Some("feat/a".to_string())),
        "the shell reads the entry's branch set, starting from its current branch (feature 1003)"
    );
    assert!(matches!(view_list(&state), PrList::Loading { .. }));
}

#[test]
fn the_setting_off_gives_no_section_and_no_read() {
    let mut state = opened(false);
    assert_eq!(wanted(&mut state, Some("feat/a")), Effect::None);
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn a_worktree_on_a_detached_head_still_reads_its_created_branches() {
    let mut state = opened(true);
    let effect = wanted(&mut state, None);
    let Effect::ReadPullRequests { entry, current, .. } = effect else {
        panic!("expected ReadPullRequests, got {effect:?}");
    };
    assert_eq!(
        (entry, current),
        (worktree(), None),
        "no current branch: the set is the branches created there (feature 1003)"
    );
    assert!(matches!(view_list(&state), PrList::Loading { .. }));
}

#[test]
fn the_project_root_gives_no_section_and_no_read() {
    let mut state = opened(true);
    reopen(&mut state, SessionLocation::Default);
    assert_eq!(wanted(&mut state, Some("main")), Effect::None, "FR-012");
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn the_base_line_names_the_shown_pull_requests_own_head_branch() {
    let mut state = opened(true);
    let mut old = pr(5, BranchPrState::Merged);
    old.head_branch = "feat/old".into();
    listed(&mut state, vec![old]);
    chosen(&mut state, 5);
    let view = state.open.as_ref().expect("open");
    assert_eq!(
        base_line(view).as_deref(),
        Some("Pull request #5: main ... feat/old"),
        "its own branch, not the entry's current feat/a (research R6)"
    );
}

#[test]
fn the_answer_fills_the_list_and_a_stale_answer_is_dropped() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let stale = Msg::PullRequestsRead {
        seq: seq + 99,
        read_at: 5,
        result: Ok(vec![pr(1, BranchPrState::Open)]),
    };
    changes::update(&mut state, stale);
    assert!(matches!(view_list(&state), PrList::Loading { .. }));

    let list = vec![pr(2, BranchPrState::Merged), pr(1, BranchPrState::Open)];
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 7,
            result: Ok(list.clone()),
        },
    );
    assert_eq!(
        view_list(&state),
        &PrList::Ready {
            prs: list,
            read_at: 7
        }
    );
}

#[test]
fn turning_the_setting_on_asks_for_the_list_and_off_clears_it() {
    let mut state = opened(false);
    wanted(&mut state, Some("feat/a"));
    let effect = changes::update(&mut state, Msg::PullRequestSettingChanged(true));
    let seq = seq_of(effect);
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1,
            result: Ok(vec![pr(1, BranchPrState::Open)]),
        },
    );
    assert!(matches!(view_list(&state), PrList::Ready { .. }));

    let effect = changes::update(&mut state, Msg::PullRequestSettingChanged(false));
    assert_eq!(effect, Effect::None);
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn a_failed_read_keeps_the_last_list_and_the_view_has_no_notice_channel() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let list = vec![pr(1, BranchPrState::Draft)];
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 3,
            result: Ok(list.clone()),
        },
    );
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 9,
            result: Err(ReadingFailure::Passing),
        },
    );
    assert_eq!(effect, Effect::None, "FR-008: nothing follows a failure");
    assert_eq!(
        view_list(&state)
            .shown()
            .map(|(prs, at)| (prs.to_vec(), at)),
        Some((list, 3))
    );
}

#[test]
fn no_github_access_removes_the_section() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 3,
            result: Err(ReadingFailure::Unavailable),
        },
    );
    assert_eq!(view_list(&state), &PrList::Absent);
}

#[test]
fn a_worktree_with_no_session_lists_the_same() {
    // The view is keyed by the entry, never by a session (FR-002): the same messages give the same
    // list whichever session, if any, is in the worktree.
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1,
            result: Ok(vec![pr(1, BranchPrState::Closed)]),
        },
    );
    assert_eq!(view_list(&state).shown().map(|(p, _)| p.len()), Some(1));
}

#[test]
fn a_request_made_while_one_runs_runs_once_more_after_it() {
    let mut state = opened(true);
    let first = seq_of(wanted(&mut state, Some("feat/a")));
    assert_eq!(wanted(&mut state, Some("feat/a")), Effect::None);
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq: first,
            read_at: 1,
            result: Ok(vec![]),
        },
    );
    assert!(seq_of(effect) > first);
}

#[test]
fn the_four_states_have_four_distinct_marks() {
    let marks = [
        pr_mark(BranchPrState::Open),
        pr_mark(BranchPrState::Draft),
        pr_mark(BranchPrState::Merged),
        pr_mark(BranchPrState::Closed),
    ];
    assert_eq!(
        marks,
        [PrMark::Open, PrMark::Draft, PrMark::Merged, PrMark::Closed]
    );
}

#[test]
fn a_check_status_maps_to_a_mark_and_none_has_no_mark() {
    use micold_client::ui::CheckMark;
    use micold_core::pull_request::CheckStatus;
    assert_eq!(check_mark(CheckStatus::None), None);
    assert_eq!(
        [
            check_mark(CheckStatus::Pending),
            check_mark(CheckStatus::Passing),
            check_mark(CheckStatus::Failing)
        ],
        [
            Some(CheckMark::Pending),
            Some(CheckMark::Passing),
            Some(CheckMark::Failing)
        ]
    );
}

// ---- M2: select a pull request and read its diff ----

const DIFF: &str = "diff --git a/a.rs b/a.rs\nindex 1..2 100644\n--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,2 @@\n keep\n-old\n+new\n\
diff --git a/b.rs b/b.rs\nnew file mode 100644\nindex 0..3\n--- /dev/null\n+++ b/b.rs\n@@ -0,0 +1 @@\n+fresh\n";

fn listed(state: &mut State, prs: Vec<BranchPullRequest>) {
    let seq = seq_of(wanted(state, Some("feat/a")));
    changes::update(
        state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1,
            result: Ok(prs),
        },
    );
}

fn diff_seq(effect: Effect, expected_number: u64) -> u64 {
    match effect {
        Effect::ReadPullRequestDiff {
            seq,
            project: p,
            number,
        } => {
            assert_eq!((p, number), (project(), expected_number));
            seq
        }
        other => panic!("expected ReadPullRequestDiff, got {other:?}"),
    }
}

fn source(state: &State) -> &Source {
    &state.open.as_ref().expect("open").source
}

fn chosen(state: &mut State, number: u64) -> u64 {
    let effect = changes::update(state, Msg::SourceChosen(Some(number)));
    let seq = diff_seq(effect, number);
    changes::update(
        state,
        Msg::PullRequestDiffRead {
            seq,
            number,
            result: Ok(split_pr_diff(DIFF)),
        },
    );
    seq
}

#[test]
fn choosing_a_pull_request_reads_its_diff_and_lists_its_files() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Merged)]);
    let effect = changes::update(&mut state, Msg::SourceChosen(Some(7)));
    let seq = diff_seq(effect, 7);
    let view = state.open.as_ref().expect("open");
    assert!(matches!(list_body(view), ListBody::Loading));

    changes::update(
        &mut state,
        Msg::PullRequestDiffRead {
            seq,
            number: 7,
            result: Ok(split_pr_diff(DIFF)),
        },
    );
    let view = state.open.as_ref().expect("open");
    let ListBody::Files(files) = list_body(view) else {
        panic!("files of the pull request");
    };
    let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, ["a.rs", "b.rs"]);
    assert_eq!(
        base_line(view).as_deref(),
        Some("Pull request #7: main ... feat/a")
    );
}

#[test]
fn selecting_a_file_of_a_pull_request_reads_nothing() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    let effect = changes::update(
        &mut state,
        Msg::FileSelected(RelPath::from_native("a.rs").unwrap()),
    );
    assert_eq!(effect, Effect::None, "the patch is parsed, not read");
    let view = state.open.as_ref().expect("open");
    let DiffBody::Diff(loaded) = diff_body(view) else {
        panic!("the file's diff");
    };
    assert!(matches!(&loaded.diff, FileDiff::Text(hunks) if hunks.len() == 1));
}

#[test]
fn choosing_the_working_tree_returns_to_it() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    let effect = changes::update(&mut state, Msg::SourceChosen(None));
    assert_eq!(effect, Effect::None);
    assert_eq!(source(&state), &Source::Changes);
}

#[test]
fn each_entry_remembers_its_own_choice() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    // Switch to another entry, then back: the first entry shows its pull request again once the
    // list that holds it is read, and the other entry never showed it.
    changes::update(
        &mut state,
        Msg::Opened {
            project: project(),
            entry: SessionLocation::Worktree("feat-b".into()),
        },
    );
    assert_eq!(source(&state), &Source::Changes);
    changes::update(
        &mut state,
        Msg::Opened {
            project: project(),
            entry: worktree(),
        },
    );
    assert_eq!(source(&state), &Source::Changes);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 2,
            result: Ok(vec![pr(7, BranchPrState::Open)]),
        },
    );
    diff_seq(effect, 7);
    assert!(matches!(source(&state), Source::PullRequest(p) if p.number == 7));
}

#[test]
fn a_pull_request_that_leaves_the_list_returns_the_view_to_the_working_tree() {
    let mut state = opened(true);
    listed(
        &mut state,
        vec![pr(7, BranchPrState::Open), pr(8, BranchPrState::Closed)],
    );
    chosen(&mut state, 7);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 5,
            result: Ok(vec![pr(8, BranchPrState::Closed)]),
        },
    );
    assert_eq!(source(&state), &Source::Changes);
    assert!(state.remembered.is_empty(), "forgotten, not restored later");
}

#[test]
fn a_too_large_pull_request_and_a_failed_read_show_a_line_and_raise_nothing() {
    for (failure, too_large) in [
        (DiffReadFailure::TooLarge, true),
        (DiffReadFailure::Failed(ReadingFailure::Passing), false),
    ] {
        let mut state = opened(true);
        listed(&mut state, vec![pr(7, BranchPrState::Open)]);
        let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(7))), 7);
        let effect = changes::update(
            &mut state,
            Msg::PullRequestDiffRead {
                seq,
                number: 7,
                result: Err(failure),
            },
        );
        assert_eq!(effect, Effect::None);
        let Source::PullRequest(p) = source(&state) else {
            panic!("still on the pull request");
        };
        assert_eq!(matches!(p.diff, PrDiffLoad::TooLarge), too_large);
        assert!(matches!(
            list_body(state.open.as_ref().expect("open")),
            ListBody::PullRequestUnread { too_large: t } if t == too_large
        ));
    }
}

#[test]
fn an_answer_for_another_read_is_dropped() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(7))), 7);
    changes::update(
        &mut state,
        Msg::PullRequestDiffRead {
            seq: seq + 1,
            number: 7,
            result: Ok(split_pr_diff(DIFF)),
        },
    );
    let Source::PullRequest(p) = source(&state) else {
        panic!("pull request");
    };
    assert!(matches!(p.diff, PrDiffLoad::Loading { .. }));
}

fn reopen(state: &mut State, entry: SessionLocation) {
    changes::update(
        state,
        Msg::Opened {
            project: project(),
            entry,
        },
    );
}

#[test]
fn choosing_the_working_tree_forgets_the_remembered_pull_request() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    changes::update(&mut state, Msg::SourceChosen(None));
    assert!(
        state.remembered.is_empty(),
        "the user left the pull request"
    );
    reopen(&mut state, SessionLocation::Worktree("feat-b".into()));
    reopen(&mut state, worktree());
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 2,
            result: Ok(vec![pr(7, BranchPrState::Open)]),
        },
    );
    assert_eq!(effect, Effect::None, "nothing reopens");
    assert_eq!(source(&state), &Source::Changes);
}

#[test]
fn choosing_an_unlisted_number_does_nothing() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    let effect = changes::update(&mut state, Msg::SourceChosen(Some(99)));
    assert_eq!(effect, Effect::None);
    assert_eq!(source(&state), &Source::Changes);
    assert!(state.remembered.is_empty());
}

#[test]
fn choosing_the_shown_pull_request_again_reads_nothing() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    let effect = changes::update(&mut state, Msg::SourceChosen(Some(7)));
    assert_eq!(effect, Effect::None);
    assert!(matches!(source(&state), Source::PullRequest(p) if p.number == 7));
}

#[test]
fn choosing_a_pull_request_whose_diff_failed_reads_it_again() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(7))), 7);
    changes::update(
        &mut state,
        Msg::PullRequestDiffRead {
            seq,
            number: 7,
            result: Err(DiffReadFailure::Failed(ReadingFailure::Passing)),
        },
    );
    let Source::PullRequest(p) = source(&state) else {
        panic!("pull request");
    };
    assert!(matches!(p.diff, PrDiffLoad::Failed));
    let effect = changes::update(&mut state, Msg::SourceChosen(Some(7)));
    diff_seq(effect, 7);
}

#[test]
fn choosing_the_working_tree_while_on_it_keeps_a_remembered_pull_request() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    reopen(&mut state, SessionLocation::Worktree("feat-b".into()));
    reopen(&mut state, worktree());
    assert_eq!(source(&state), &Source::Changes);
    let effect = changes::update(&mut state, Msg::SourceChosen(None));
    assert_eq!(effect, Effect::None);
    assert_eq!(state.remembered.len(), 1, "still awaiting its list");
}

#[test]
fn the_setting_turned_off_returns_to_the_working_tree() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(7, BranchPrState::Open)]);
    chosen(&mut state, 7);
    changes::update(&mut state, Msg::PullRequestSettingChanged(false));
    assert_eq!(source(&state), &Source::Changes);
}

// FR-007: a status trigger's refresh is the same ask; the list stays while it runs and the answer
// replaces it.
#[test]
fn pr_list_refresh_replaces_the_list_with_the_new_state() {
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 100,
            result: Ok(vec![pr(1, BranchPrState::Open)]),
        },
    );
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    assert_eq!(
        view_list(&state).shown().map(|(prs, _)| prs.len()),
        Some(1),
        "the last list stays while the refresh runs"
    );
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 200,
            result: Ok(vec![
                pr(1, BranchPrState::Merged),
                pr(2, BranchPrState::Open),
            ]),
        },
    );
    let (prs, at) = view_list(&state).shown().expect("listed");
    assert_eq!((prs.len(), at), (2, 200));
    assert_eq!(prs[0].state, BranchPrState::Merged);
}

// FR-012: a failed refresh keeps the list; it is marked once older than the stale threshold, and
// a refreshed list replaces the mark.
#[test]
fn pr_list_refresh_failure_keeps_the_list_and_marks_it_stale_by_age() {
    use micold_core::pull_request::STALE_AFTER_SECS;
    let mut state = opened(true);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1_000,
            result: Ok(vec![pr(1, BranchPrState::Open)]),
        },
    );
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1_000 + STALE_AFTER_SECS,
            result: Err(ReadingFailure::Passing),
        },
    );
    let list = view_list(&state);
    assert_eq!(
        list.shown().map(|(prs, at)| (prs.len(), at)),
        Some((1, 1_000))
    );
    assert!(!list.possibly_stale(1_000 + STALE_AFTER_SECS));
    assert!(list.possibly_stale(1_000 + STALE_AFTER_SECS + 1));

    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 5_000,
            result: Ok(vec![pr(1, BranchPrState::Open)]),
        },
    );
    assert!(!view_list(&state).possibly_stale(5_000));
}
