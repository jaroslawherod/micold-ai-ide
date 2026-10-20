//! The Changes view's pull request list, in isolation (feature 775, M1: contracts/changes-view.md,
//! data-model §2). Builds only `features::changes::State` and drives `update`.

use std::path::PathBuf;

use micold_client::features::changes::{
    self, adjacent, base_line, check_mark, diff_body, list_body, pr_header, pr_mark, section,
    DiffBody, Effect, ListBody, Msg, PrDiffLoad, PrList, PrSection, Source, State, Step,
};
use micold_client::ui::PrMark;
use micold_core::pull_request::{
    BranchPrState, BranchPullRequest, CheckStatus, DiffReadFailure, ReadingFailure,
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

/// Answers the pending read of `opened(true)`'s view with `result`.
fn answered(
    state: &mut State,
    read_at: u64,
    result: Result<Vec<BranchPullRequest>, ReadingFailure>,
) {
    let seq = seq_of(wanted(state, Some("feat/a")));
    let effect = changes::update(
        state,
        Msg::PullRequestsRead {
            seq,
            read_at,
            result,
        },
    );
    assert_eq!(effect, Effect::None, "SC-006: no notice, nothing follows");
}

fn section_of(state: &State, enabled: bool, now: u64) -> PrSection<'_> {
    section(state.open.as_ref().expect("open"), enabled, now)
}

fn failures() -> [ReadingFailure; 3] {
    [
        ReadingFailure::Unavailable,
        ReadingFailure::Passing,
        ReadingFailure::RateLimited { until: 99 },
    ]
}

#[test]
fn no_github_access_keeps_unavailable_without_a_list_and_raises_nothing() {
    for failure in failures() {
        let mut state = opened(true);
        answered(&mut state, 3, Err(failure));
        assert_eq!(view_list(&state), &PrList::Unavailable { last: None });
        assert_eq!(section_of(&state, true, 3), PrSection::Unavailable);
    }
}

#[test]
fn every_failure_kind_keeps_the_last_list_and_the_selection() {
    for failure in failures() {
        let mut state = opened(true);
        listed(
            &mut state,
            vec![pr(1, BranchPrState::Open), pr(2, BranchPrState::Open)],
        );
        chosen(&mut state, 2);
        answered(&mut state, 9, Err(failure));
        assert!(matches!(
            view_list(&state),
            PrList::Unavailable { last: Some(_) }
        ));
        assert!(matches!(source(&state), Source::PullRequest(p) if p.number == 2));
        let PrSection::List { prs, stale } = section_of(&state, true, 9) else {
            panic!("FR-015: the last list stays");
        };
        assert_eq!((prs.len(), stale), (2, false));
    }
}

#[test]
fn a_failure_without_a_list_resets_the_source_to_the_working_tree() {
    for failure in failures() {
        let mut state = opened(true);
        listed(&mut state, vec![pr(1, BranchPrState::Open)]);
        chosen(&mut state, 1);
        assert!(matches!(source(&state), Source::PullRequest(_)));
        // A read with no list behind it (as after the setting was toggled) fails.
        state.open.as_mut().expect("open").pull_requests = PrList::Loading {
            seq: 77,
            again: false,
            last: None,
        };
        changes::update(
            &mut state,
            Msg::PullRequestsRead {
                seq: 77,
                read_at: 3,
                result: Err(failure),
            },
        );
        assert_eq!(source(&state), &Source::Changes);
        assert_eq!(view_list(&state), &PrList::Unavailable { last: None });
    }
}

#[test]
fn the_section_follows_the_states_table() {
    use micold_core::pull_request::STALE_AFTER_SECS;
    let mut state = opened(true);
    assert_eq!(
        section_of(&state, true, 0),
        PrSection::Absent,
        "before any ask"
    );
    assert_eq!(
        section_of(&state, false, 0),
        PrSection::Absent,
        "setting off"
    );
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    assert_eq!(
        section_of(&state, true, 0),
        PrSection::Loading,
        "first read pending"
    );
    assert_eq!(section_of(&state, false, 0), PrSection::Absent);
    changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 1_000,
            result: Ok(vec![]),
        },
    );
    assert_eq!(section_of(&state, true, 1_000), PrSection::Empty);
    answered(&mut state, 1_100, Ok(vec![pr(1, BranchPrState::Open)]));
    assert!(matches!(
        section_of(&state, true, 1_100),
        PrSection::List { stale: false, .. }
    ));
    answered(&mut state, 1_200, Err(ReadingFailure::Passing));
    let late = 1_100 + STALE_AFTER_SECS + 1;
    assert!(matches!(
        section_of(&state, true, late),
        PrSection::List { stale: true, .. }
    ));
}

#[test]
fn a_reread_with_a_list_in_hand_never_shows_the_loading_line() {
    let mut state = opened(true);
    listed(&mut state, vec![]);
    wanted(&mut state, Some("feat/a"));
    assert_eq!(section_of(&state, true, 1), PrSection::Empty);
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
    assert_eq!(
        pr_header(state.open.as_ref().expect("open")),
        None,
        "the header goes with the pull request"
    );
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

fn diff_of(file: &str) -> String {
    format!(
        "diff --git a/{file} b/{file}\nindex 1..2 100644\n--- a/{file}\n+++ b/{file}\n@@ -1 +1 @@\n-old\n+new\n"
    )
}

/// The paths of the files listed for the shown pull request.
fn shown_paths(state: &State) -> Vec<String> {
    let view = state.open.as_ref().expect("open");
    let ListBody::Files(files) = list_body(view) else {
        panic!("files of the pull request");
    };
    files.iter().map(|f| f.path.as_str().to_string()).collect()
}

fn answer(state: &mut State, seq: u64, number: u64, file: &str) {
    changes::update(
        state,
        Msg::PullRequestDiffRead {
            seq,
            number,
            result: Ok(split_pr_diff(&diff_of(file))),
        },
    );
}

fn three(state: &mut State) {
    listed(
        state,
        vec![
            pr(3, BranchPrState::Open),
            pr(2, BranchPrState::Merged),
            pr(1, BranchPrState::Closed),
        ],
    );
}

// SC-002, US2 scenarios 1, 2: each choice shows that pull request's own files.
#[test]
fn choosing_the_second_then_the_third_shows_each_ones_own_files() {
    let mut state = opened(true);
    three(&mut state);
    let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(2))), 2);
    answer(&mut state, seq, 2, "two.rs");
    assert_eq!(shown_paths(&state), ["two.rs"]);
    let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(3))), 3);
    answer(&mut state, seq, 3, "three.rs");
    assert_eq!(shown_paths(&state), ["three.rs"]);
}

// FR-006: list order, none at an end or for a number that is not listed.
#[test]
fn adjacent_follows_list_order_and_stops_at_the_ends() {
    let prs = [
        pr(3, BranchPrState::Open),
        pr(2, BranchPrState::Open),
        pr(1, BranchPrState::Open),
    ];
    assert_eq!(adjacent(&prs, 2, Step::Next), Some(1));
    assert_eq!(adjacent(&prs, 2, Step::Previous), Some(3));
    assert_eq!(adjacent(&prs, 3, Step::Previous), None);
    assert_eq!(adjacent(&prs, 1, Step::Next), None);
    assert_eq!(adjacent(&prs, 9, Step::Next), None);
}

// SC-003, US2 scenario 3.
#[test]
fn stepping_has_nothing_to_step_on_the_working_tree() {
    let mut state = opened(true);
    three(&mut state);
    let effect = changes::update(&mut state, Msg::PullRequestStepped(Step::Next));
    assert_eq!(effect, Effect::None, "the working tree has nothing to step");
}

#[test]
fn stepping_selects_the_adjacent_pull_request_and_stops_at_the_ends() {
    let mut state = opened(true);
    three(&mut state);
    chosen(&mut state, 2);
    let seq = diff_seq(
        changes::update(&mut state, Msg::PullRequestStepped(Step::Next)),
        1,
    );
    answer(&mut state, seq, 1, "one.rs");
    assert_eq!(shown_paths(&state), ["one.rs"]);
    let effect = changes::update(&mut state, Msg::PullRequestStepped(Step::Next));
    assert_eq!(effect, Effect::None, "next on the last entry does nothing");
    let seq = diff_seq(
        changes::update(&mut state, Msg::PullRequestStepped(Step::Previous)),
        2,
    );
    answer(&mut state, seq, 2, "two.rs");
    chosen(&mut state, 3);
    let effect = changes::update(&mut state, Msg::PullRequestStepped(Step::Previous));
    assert_eq!(effect, Effect::None, "previous on the first does nothing");
}

// Edge case: more pull requests than fit in the list.
#[test]
fn next_from_the_first_of_thirty_reaches_the_last() {
    let mut state = opened(true);
    listed(
        &mut state,
        (1..=30).rev().map(|n| pr(n, BranchPrState::Open)).collect(),
    );
    chosen(&mut state, 30);
    for expected in (1..30).rev() {
        let seq = diff_seq(
            changes::update(&mut state, Msg::PullRequestStepped(Step::Next)),
            expected,
        );
        answer(&mut state, seq, expected, "f.rs");
    }
    let effect = changes::update(&mut state, Msg::PullRequestStepped(Step::Next));
    assert_eq!(effect, Effect::None);
    let header = pr_header(state.open.as_ref().expect("open")).expect("header");
    assert_eq!(header.number, 1);
}

// FR-007, US2 scenarios 4, 5.
#[test]
fn the_header_names_the_shown_pull_request_with_state_and_checks() {
    let mut state = opened(true);
    let mut second = pr(2, BranchPrState::Merged);
    second.checks = CheckStatus::Failing;
    listed(&mut state, vec![pr(3, BranchPrState::Open), second]);
    assert_eq!(pr_header(state.open.as_ref().expect("open")), None);
    chosen(&mut state, 2);
    let header = pr_header(state.open.as_ref().expect("open")).expect("header");
    assert_eq!((header.number, header.title.as_str()), (2, "title 2"));
    assert_eq!(header.mark, PrMark::Merged);
    assert_eq!(header.check, Some(micold_client::ui::CheckMark::Failing));
    assert_eq!(header.base, "Pull request #2: main ... feat/a");

    changes::update(&mut state, Msg::SourceChosen(None));
    assert_eq!(pr_header(state.open.as_ref().expect("open")), None);
    let effect = changes::update(&mut state, Msg::PullRequestStepped(Step::Previous));
    assert_eq!(
        effect,
        Effect::None,
        "stepping is a no-op on the working tree"
    );
}

// FR-016: the last choice wins; a late answer to an earlier choice is dropped.
#[test]
fn three_fast_choices_show_only_the_last_ones_diff_under_its_header() {
    let mut state = opened(true);
    three(&mut state);
    let s3 = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(3))), 3);
    let s2 = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(2))), 2);
    let s1 = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(1))), 1);
    answer(&mut state, s2, 2, "two.rs");
    answer(&mut state, s1, 1, "one.rs");
    answer(&mut state, s3, 3, "three.rs");
    assert_eq!(shown_paths(&state), ["one.rs"]);
    let header = pr_header(state.open.as_ref().expect("open")).expect("header");
    assert_eq!(header.number, 1);
}

#[test]
fn a_failed_or_too_large_diff_keeps_the_header_and_stepping() {
    let mut state = opened(true);
    three(&mut state);
    let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(2))), 2);
    changes::update(
        &mut state,
        Msg::PullRequestDiffRead {
            seq,
            number: 2,
            result: Err(DiffReadFailure::TooLarge),
        },
    );
    assert_eq!(
        pr_header(state.open.as_ref().expect("open")).map(|h| h.number),
        Some(2)
    );
    let seq = diff_seq(
        changes::update(&mut state, Msg::PullRequestStepped(Step::Next)),
        1,
    );
    changes::update(
        &mut state,
        Msg::PullRequestDiffRead {
            seq,
            number: 1,
            result: Err(DiffReadFailure::Failed(ReadingFailure::Passing)),
        },
    );
    assert_eq!(
        pr_header(state.open.as_ref().expect("open")).map(|h| h.number),
        Some(1)
    );
    let effect = changes::update(&mut state, Msg::PullRequestStepped(Step::Previous));
    diff_seq(effect, 2);
}

// US3 scenario 1, FR-008: a new entry at the top does not move the selection.
#[test]
fn a_refresh_adding_a_pull_request_keeps_the_selected_one_shown() {
    let mut state = opened(true);
    listed(
        &mut state,
        vec![
            pr(3, BranchPrState::Open),
            pr(2, BranchPrState::Open),
            pr(1, BranchPrState::Open),
        ],
    );
    chosen(&mut state, 2);
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 2,
            result: Ok(vec![
                pr(4, BranchPrState::Open),
                pr(3, BranchPrState::Open),
                pr(2, BranchPrState::Open),
                pr(1, BranchPrState::Open),
            ]),
        },
    );
    assert_eq!(effect, Effect::None, "no new diff read");
    assert_eq!(
        pr_header(state.open.as_ref().expect("open")).map(|h| h.number),
        Some(2)
    );
    assert_eq!(shown_paths(&state), ["a.rs", "b.rs"]);
}

// US3 scenario 3: a state and check change updates the row and header, and reads nothing.
#[test]
fn a_refresh_changing_the_selected_ones_state_updates_the_header_without_a_read() {
    let mut state = opened(true);
    listed(&mut state, vec![pr(2, BranchPrState::Open)]);
    chosen(&mut state, 2);
    let mut merged = pr(2, BranchPrState::Merged);
    merged.checks = CheckStatus::Passing;
    let seq = seq_of(wanted(&mut state, Some("feat/a")));
    let effect = changes::update(
        &mut state,
        Msg::PullRequestsRead {
            seq,
            read_at: 2,
            result: Ok(vec![merged]),
        },
    );
    assert_eq!(effect, Effect::None);
    let header = pr_header(state.open.as_ref().expect("open")).expect("header");
    assert_eq!(header.mark, PrMark::Merged);
    assert_eq!(header.check, Some(micold_client::ui::CheckMark::Passing));
    assert_eq!(shown_paths(&state), ["a.rs", "b.rs"]);
}

// ---- 1149 M1: review comments on a pull request's diff ----

mod review_comments {
    use super::*;
    use micold_client::features::changes::{
        can_pick, file_comments, jump_target, panel_groups, panel_rows, pending_counts,
        review_toolbar_label, send_action, status_counts, tidy_actions, Jump, PanelGroup,
    };
    use micold_core::protocol::messages::ReviewEditOp;
    use micold_core::review::comment::{
        CommentId, CommentState, PrAnchor, ReviewComment, Scope, Status,
    };
    use micold_core::review::{LineRange, Side};

    const HEAD: &str = "0123456789abcdef0123456789abcdef01234567";

    fn listed_pr(number: u64) -> BranchPullRequest {
        BranchPullRequest {
            head: HEAD.into(),
            ..pr(number, BranchPrState::Open)
        }
    }

    fn comment_on(
        number: Option<u64>,
        file: &str,
        side: Side,
        line: u32,
        state: CommentState,
    ) -> ReviewComment {
        ReviewComment {
            id: CommentId::new(),
            path: RelPath::from_native(file).unwrap(),
            side,
            range: LineRange::new(line, line).unwrap(),
            quote: vec!["q".into()],
            text: format!("about {file}:{line}"),
            state,
            created: 1,
            pr: number.map(|number| PrAnchor {
                number,
                head_branch: "feat/a".into(),
                head_commit: HEAD.into(),
            }),
        }
    }

    fn push(state: &mut State, comments: Vec<ReviewComment>) {
        changes::update(
            state,
            Msg::ReviewChanged {
                project: project(),
                worktree_dir: "feat-a".into(),
                comments,
                sending: false,
            },
        );
    }

    /// Pull request 7 shown with `a.rs` selected, 8 also listed.
    fn showing_seven() -> State {
        let mut state = opened(true);
        listed(&mut state, vec![listed_pr(7), listed_pr(8)]);
        chosen(&mut state, 7);
        changes::update(
            &mut state,
            Msg::FileSelected(RelPath::from_native("a.rs").unwrap()),
        );
        state
    }

    fn add_of(effect: Effect) -> ReviewEditOp {
        let Effect::ReviewEdit { edit, .. } = effect else {
            panic!("an edit, got {effect:?}");
        };
        edit
    }

    // US1 s1, FR-001, FR-002: a pick on a pull request's text file adds a comment carrying the
    // pull request's anchor, quoting the hunk's lines.
    #[test]
    fn a_pick_on_a_pull_request_file_builds_an_add_with_its_anchor() {
        let mut state = showing_seven();
        assert!(can_pick(state.open.as_ref().unwrap()));
        changes::update(
            &mut state,
            Msg::GutterPressed {
                old: Some(2),
                new: None,
                extend: false,
            },
        );
        changes::update(
            &mut state,
            Msg::GutterPressed {
                old: None,
                new: Some(2),
                extend: false,
            },
        );
        changes::update(&mut state, Msg::AddComment);
        changes::update(&mut state, Msg::ComposerEdited("rename".into()));
        let edit = add_of(changes::update(&mut state, Msg::ComposerSaved));
        assert_eq!(
            edit,
            ReviewEditOp::Add {
                path: "a.rs".into(),
                side: Side::New,
                start: 2,
                end: 2,
                quote: vec!["new".into()],
                text: "rename".into(),
                pr: Some(PrAnchor {
                    number: 7,
                    head_branch: "feat/a".into(),
                    head_commit: HEAD.into(),
                }),
            }
        );
    }

    #[test]
    fn a_range_on_the_old_side_quotes_the_removed_and_context_lines() {
        let mut state = showing_seven();
        for (line, extend) in [(1, false), (2, true)] {
            changes::update(
                &mut state,
                Msg::GutterPressed {
                    old: Some(line),
                    new: None,
                    extend,
                },
            );
        }
        changes::update(&mut state, Msg::AddComment);
        changes::update(&mut state, Msg::ComposerEdited("why".into()));
        let ReviewEditOp::Add { quote, side, .. } =
            add_of(changes::update(&mut state, Msg::ComposerSaved))
        else {
            panic!("an add");
        };
        assert_eq!(
            (side, quote),
            (Side::Old, vec!["keep".into(), "old".into()])
        );
    }

    // FR-014: nothing can be picked while the diff or the file is not a ready text diff.
    #[test]
    fn nothing_is_picked_on_a_pull_request_diff_that_is_not_ready_text() {
        let mut state = opened(true);
        listed(&mut state, vec![listed_pr(7)]);
        let seq = diff_seq(changes::update(&mut state, Msg::SourceChosen(Some(7))), 7);
        assert!(!can_pick(state.open.as_ref().unwrap()), "still loading");
        let binary = "diff --git a/p.png b/p.png\nnew file mode 100644\nindex 0..3\nBinary files /dev/null and b/p.png differ\n";
        changes::update(
            &mut state,
            Msg::PullRequestDiffRead {
                seq,
                number: 7,
                result: Ok(split_pr_diff(binary)),
            },
        );
        changes::update(
            &mut state,
            Msg::FileSelected(RelPath::from_native("p.png").unwrap()),
        );
        assert!(!can_pick(state.open.as_ref().unwrap()), "binary");
        let effect = changes::update(
            &mut state,
            Msg::GutterPressed {
                old: None,
                new: Some(1),
                extend: false,
            },
        );
        assert_eq!(effect, Effect::None);
        assert_eq!(state.open.as_ref().unwrap().pick, None);
    }

    // US1 s4, FR-003: a comment shows on its own pull request's diff only.
    #[test]
    fn comments_are_filtered_by_the_shown_scope() {
        let mut state = showing_seven();
        let mine = comment_on(Some(7), "a.rs", Side::New, 2, CommentState::Draft);
        let other = comment_on(Some(8), "a.rs", Side::New, 2, CommentState::Draft);
        let tree = comment_on(None, "a.rs", Side::New, 2, CommentState::Draft);
        let ids = (mine.id, other.id, tree.id);
        push(&mut state, vec![mine, other, tree]);

        let placed = file_comments(&state);
        let under: Vec<_> = placed.under(Side::New, 2).iter().map(|c| c.id).collect();
        assert_eq!(under, vec![ids.0]);
        assert_eq!(pending_counts(&state).values().sum::<usize>(), 1);
        assert_eq!(review_toolbar_label(&state), "Review (1)");
        assert_eq!(status_counts(&state)[0], (Status::Draft, 1));
        assert!(jump_target(&state, ids.0).is_some());
        assert_eq!(jump_target(&state, ids.1), None);
        assert_eq!(jump_target(&state, ids.2), None);

        changes::update(&mut state, Msg::SourceChosen(None));
        changes::update(
            &mut state,
            Msg::FileSelected(RelPath::from_native("a.rs").unwrap()),
        );
        assert_eq!(review_toolbar_label(&state), "Review (1)");
        assert_eq!(pending_counts(&state).values().sum::<usize>(), 1);
        assert!(matches!(jump_target(&state, ids.2), Some(Jump::Hidden(_))));
    }

    // FR-004, FR-013: working tree first, then pull requests by number; unlisted flagged.
    #[test]
    fn the_panel_groups_the_working_tree_first_then_pull_requests_by_number() {
        let mut state = showing_seven();
        push(
            &mut state,
            vec![
                comment_on(Some(9), "a.rs", Side::New, 1, CommentState::Draft),
                comment_on(Some(8), "a.rs", Side::New, 1, CommentState::Draft),
                comment_on(None, "z.rs", Side::New, 1, CommentState::Draft),
                comment_on(Some(7), "a.rs", Side::New, 1, CommentState::Draft),
            ],
        );
        let scopes: Vec<_> = panel_rows(&state).iter().map(|r| r.scope).collect();
        assert_eq!(
            scopes,
            [
                Scope::WorkingTree,
                Scope::PullRequest(7),
                Scope::PullRequest(8),
                Scope::PullRequest(9)
            ]
        );
        let group = |scope, title: Option<&str>, listed| PanelGroup {
            scope,
            title: title.map(str::to_string),
            listed,
        };
        assert_eq!(
            panel_groups(&state),
            vec![
                group(Scope::WorkingTree, None, true),
                group(Scope::PullRequest(7), Some("title 7"), true),
                group(Scope::PullRequest(8), Some("title 8"), true),
                group(Scope::PullRequest(9), None, false),
            ]
        );
        assert!(panel_groups(&state)[3]
            .heading()
            .unwrap()
            .contains("not in the list"));
        assert_eq!(panel_rows(&state).len(), 4, "the unlisted comment is kept");
    }

    // FR-015: tidy acts on the shown scope; Send is not offered for a pull request yet.
    #[test]
    fn tidy_emits_the_shown_scope_and_send_is_not_offered_on_a_pull_request() {
        let mut state = showing_seven();
        push(
            &mut state,
            vec![
                comment_on(Some(7), "a.rs", Side::New, 2, CommentState::Draft),
                comment_on(Some(7), "a.rs", Side::New, 1, CommentState::Sent { at: 5 }),
                comment_on(None, "a.rs", Side::New, 1, CommentState::Draft),
            ],
        );
        let tidy = tidy_actions(&state).unwrap();
        assert!(tidy.clear_sent && tidy.discard_pending);
        assert_eq!(send_action(&state), None);
        assert_eq!(changes::update(&mut state, Msg::SendPressed), Effect::None);
        assert_eq!(
            add_of(changes::update(&mut state, Msg::ClearSentPressed)),
            ReviewEditOp::ClearSent {
                scope: Scope::PullRequest(7)
            }
        );
        changes::update(&mut state, Msg::DiscardPendingPressed);
        assert_eq!(
            add_of(changes::update(&mut state, Msg::DiscardConfirmed)),
            ReviewEditOp::DiscardPending {
                scope: Scope::PullRequest(7)
            }
        );
        // Another pull request without comments has nothing to tidy.
        changes::update(&mut state, Msg::SourceChosen(Some(8)));
        let tidy = tidy_actions(&state).unwrap();
        assert!(!tidy.clear_sent && !tidy.discard_pending);
    }
}
