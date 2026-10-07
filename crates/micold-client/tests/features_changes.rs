//! The Changes view's list half, in isolation (feature 482, T014, contracts/changes-view.md V1–V3,
//! L1, L3; data-model § Client).
//!
//! Builds only `features::changes::State` and drives `update` with the messages the root and the
//! shell send.

use std::collections::BTreeSet;

use micold_client::features::changes::{
    self, base_line, can_pick, committed_available, diff_body, list_body, DiffBody, Effect,
    ListBody, Load, Msg, State, BOTH_HIDDEN, DEFAULT_ENTRY_NOTE,
};
use micold_core::review::base::{Base, BaseUnavailable, ReviewScope, Toggles};
use micold_core::review::changes::{ChangeKind, ChangeList, ChangedFile, Content, Origin};
use micold_core::review::diff::{parse_unified, FileDiff, LoadedDiff, SideLines};
use micold_core::review::RelPath;
use micold_core::session::SessionLocation;

fn path(p: &str) -> RelPath {
    RelPath::from_native(p).expect("relative path")
}

fn file(p: &str) -> ChangedFile {
    ChangedFile {
        path: path(p),
        kind: ChangeKind::Modified,
        content: Content::Text,
        added: 1,
        removed: 0,
        large: false,
        origin: Origin::Uncommitted,
    }
}

fn main_base() -> ReviewScope {
    ReviewScope::Worktree {
        base: Base::MergeBase {
            branch: "main".into(),
            commit: "0123456789abcdef0123456789abcdef01234567".into(),
        },
    }
}

fn list(paths: &[&str]) -> ChangeList {
    ChangeList {
        files: paths.iter().map(|p| file(p)).collect(),
        scope: main_base(),
    }
}

fn project() -> std::path::PathBuf {
    std::path::PathBuf::from("/projects/p")
}

fn worktree() -> SessionLocation {
    SessionLocation::Worktree("feat-a".into())
}

/// Open the view of `entry` and return the read it asked for.
fn open(state: &mut State, entry: SessionLocation) -> u64 {
    match changes::update(
        state,
        Msg::Opened {
            project: project(),
            entry,
        },
    ) {
        Effect::ReadList { seq, .. } => seq,
        other => panic!("opening reads the list, got {other:?}"),
    }
}

/// Open a worktree's view and answer its first read with `paths`.
fn ready(paths: &[&str]) -> State {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(paths)),
        },
    );
    state
}

/// V1, L1. Opening an entry reads its list with both toggles on.
#[test]
fn opening_an_entry_reads_its_list_with_both_toggles_on() {
    let mut state = State::default();
    let effect = changes::update(
        &mut state,
        Msg::Opened {
            project: project(),
            entry: worktree(),
        },
    );
    let Effect::ReadList {
        seq,
        entry,
        toggles,
    } = effect
    else {
        panic!("opening reads the list, got {effect:?}");
    };
    assert_eq!(entry, worktree());
    assert_eq!(toggles, Toggles::default());
    assert!(toggles.committed && toggles.uncommitted, "both on at open");
    let view = state.open.as_ref().expect("the view is open");
    assert_eq!(view.entry, worktree());
    assert!(matches!(view.list, Load::Loading { seq: s, again: false, .. } if s == seq));
}

/// FR-004: the toggles reset each time the view opens.
#[test]
fn reopening_resets_the_toggles() {
    let mut state = ready(&["a.rs"]);
    changes::update(&mut state, Msg::CommittedToggled);
    changes::update(&mut state, Msg::Closed);
    assert!(state.open.is_none(), "the close action closes the view");
    open(&mut state, worktree());
    assert_eq!(state.open.unwrap().toggles, Toggles::default());
}

/// US1 s2. A toggle change re-reads the list under the new toggles.
#[test]
fn a_toggle_change_rereads_the_list() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let effect = changes::update(&mut state, Msg::UncommittedToggled);
    let Effect::ReadList { toggles, .. } = effect else {
        panic!("a toggle change re-reads, got {effect:?}");
    };
    assert_eq!(
        toggles,
        Toggles {
            committed: true,
            uncommitted: false
        }
    );
    let effect = changes::update(&mut state, Msg::CommittedToggled);
    assert!(matches!(effect, Effect::None | Effect::ReadList { .. }));
}

/// An answer to a read that is no longer the current one is dropped.
#[test]
fn an_answer_with_a_stale_seq_is_dropped() {
    let mut state = State::default();
    let first = open(&mut state, worktree());
    changes::update(&mut state, Msg::Closed);
    let second = open(&mut state, worktree());
    assert_ne!(first, second);
    changes::update(
        &mut state,
        Msg::ListRead {
            seq: first,
            result: Ok(list(&["stale.rs"])),
        },
    );
    assert!(
        matches!(state.open.as_ref().unwrap().list, Load::Loading { seq, .. } if seq == second),
        "the stale answer changed nothing"
    );
}

/// A read asked while one is in flight runs once more after it, not alongside it.
#[test]
fn a_read_asked_while_one_runs_runs_once_after_it() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    assert_eq!(
        changes::update(&mut state, Msg::UncommittedToggled),
        Effect::None,
        "no second read while one runs"
    );
    assert!(matches!(
        state.open.as_ref().unwrap().list,
        Load::Loading { again: true, .. }
    ));
    let effect = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs"])),
        },
    );
    let Effect::ReadList { toggles, .. } = effect else {
        panic!("the queued read runs, got {effect:?}");
    };
    assert!(!toggles.uncommitted, "with the toggles as they are now");
}

/// The selected path survives a re-read while it is still listed, and is dropped when it is not.
#[test]
fn the_selection_is_kept_across_a_reread_while_still_listed() {
    let mut state = ready(&["a.rs", "b.rs"]);
    changes::update(&mut state, Msg::FileSelected(path("b.rs")));
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::CommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["b.rs", "c.rs"])),
        },
    );
    assert_eq!(state.open.as_ref().unwrap().selected, Some(path("b.rs")));

    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::CommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["c.rs"])),
        },
    );
    assert_eq!(state.open.as_ref().unwrap().selected, None);
}

/// V2. Selecting a session closes the view.
#[test]
fn selecting_a_session_closes_the_view() {
    let mut state = ready(&["a.rs"]);
    changes::update(&mut state, Msg::SessionSelected);
    assert!(state.open.is_none());
}

/// V3. The view of a worktree that leaves the list closes; others stay open.
#[test]
fn the_entry_leaving_the_catalog_closes_the_view() {
    let mut state = ready(&["a.rs"]);
    changes::update(
        &mut state,
        Msg::WorktreesListed(BTreeSet::from(["feat-a".to_string(), "other".to_string()])),
    );
    assert!(state.open.is_some(), "still listed: stays open");
    changes::update(
        &mut state,
        Msg::WorktreesListed(BTreeSet::from(["other".to_string()])),
    );
    assert!(state.open.is_none(), "removed: closes");

    let mut state = State::default();
    open(&mut state, SessionLocation::Default);
    changes::update(&mut state, Msg::WorktreesListed(BTreeSet::new()));
    assert!(
        state.open.is_some(),
        "the Default entry never leaves the catalog"
    );
}

/// L1, US1 s9. The Default entry's Committed toggle is unavailable, with the note saying why.
#[test]
fn the_default_entry_has_no_committed_toggle() {
    let mut state = State::default();
    open(&mut state, SessionLocation::Default);
    let view = state.open.as_ref().unwrap();
    assert!(!committed_available(view));
    assert!(DEFAULT_ENTRY_NOTE.contains("only uncommitted changes are listed"));
    assert_eq!(
        changes::update(&mut state, Msg::CommittedToggled),
        Effect::None,
        "pressing the unavailable toggle does nothing"
    );

    let state = ready(&["a.rs"]);
    assert!(committed_available(state.open.as_ref().unwrap()));
}

/// L3, US1 s6. No rows under the toggles: "No changes against <base>".
#[test]
fn an_empty_list_says_there_are_no_changes_against_the_base() {
    let state = ready(&[]);
    let view = state.open.as_ref().unwrap();
    assert_eq!(
        list_body(view),
        ListBody::Empty("No changes against main".into())
    );
    assert_eq!(
        base_line(view).as_deref(),
        Some("Compared with main at 0123456")
    );
}

/// L3, Edge "Both toggles off".
#[test]
fn both_toggles_off_says_both_are_hidden() {
    let mut state = ready(&["a.rs"]);
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::CommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs"])),
        },
    );
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::UncommittedToggled) else {
        panic!("re-read");
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&[])),
        },
    );
    assert_eq!(
        list_body(state.open.as_ref().unwrap()),
        ListBody::Empty(BOTH_HIDDEN.into())
    );
}

/// L3, Edge "No base found": the header says why committed changes are not listed.
#[test]
fn a_worktree_without_a_base_says_why() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(ChangeList {
                files: vec![file("a.rs")],
                scope: ReviewScope::Worktree {
                    base: Base::Unavailable(BaseUnavailable::NoCommonHistory),
                },
            }),
        },
    );
    let view = state.open.as_ref().unwrap();
    let line = base_line(view).expect("a base line");
    assert!(line.contains("no history in common"), "{line}");
    assert!(line.contains("only uncommitted changes"), "{line}");
    assert!(matches!(list_body(view), ListBody::Files(files) if files.len() == 1));
}

/// A failed read shows git's message; a read under way with nothing yet shows loading.
#[test]
fn loading_and_failure_have_their_own_bodies() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    assert_eq!(list_body(state.open.as_ref().unwrap()), ListBody::Loading);
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Err("fatal: not a git repository".into()),
        },
    );
    assert_eq!(
        list_body(state.open.as_ref().unwrap()),
        ListBody::Failed("fatal: not a git repository")
    );
}

// ---- The diff half (M2, T029, contracts/changes-view.md D3, D4) ----

fn loaded(diff: FileDiff) -> LoadedDiff {
    LoadedDiff {
        diff,
        old: SideLines::from_bytes(b"a\n"),
        new: SideLines::from_bytes(b"b\n"),
        spans: Default::default(),
    }
}

fn text_diff() -> LoadedDiff {
    loaded(parse_unified(b"@@ -1 +1 @@\n-a\n+b\n"))
}

/// Select `p` in a ready view and return the diff read it asked for.
fn select(state: &mut State, p: &str) -> (u64, bool) {
    match changes::update(state, Msg::FileSelected(path(p))) {
        Effect::ReadDiff {
            seq,
            path: asked,
            force_large,
            ..
        } => {
            assert_eq!(asked, path(p));
            (seq, force_large)
        }
        other => panic!("selecting a file reads its diff, got {other:?}"),
    }
}

/// US1 s3, D1. Selecting a file reads its diff under the list's scope and toggles, not forced.
#[test]
fn selecting_a_file_reads_its_diff() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let effect = changes::update(&mut state, Msg::FileSelected(path("b.rs")));
    let Effect::ReadDiff {
        seq,
        entry,
        scope,
        toggles,
        path: asked,
        from,
        force_large,
    } = effect
    else {
        panic!("selecting reads the diff, got {effect:?}");
    };
    assert_eq!(entry, worktree());
    assert_eq!(scope, main_base(), "the scope the list was read under");
    assert_eq!(toggles, Toggles::default());
    assert_eq!(asked, path("b.rs"));
    assert_eq!(from, None);
    assert!(!force_large);
    let view = state.open.as_ref().unwrap();
    assert!(matches!(view.diff, Load::Loading { seq: s, .. } if s == seq));
    assert_eq!(diff_body(view), DiffBody::Loading);

    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(text_diff()),
        },
    );
    let view = state.open.as_ref().unwrap();
    assert_eq!(diff_body(view), DiffBody::Diff(&text_diff()));
    assert!(can_pick(view), "a text diff takes comments");
}

/// A renamed file's diff is read with its old path.
#[test]
fn a_renamed_file_is_read_with_its_old_path() {
    let mut state = State::default();
    let seq = open(&mut state, worktree());
    let mut renamed = list(&["new.rs"]);
    renamed.files[0].kind = ChangeKind::Renamed {
        from: path("old.rs"),
    };
    changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(renamed),
        },
    );
    let Effect::ReadDiff { from, .. } =
        changes::update(&mut state, Msg::FileSelected(path("new.rs")))
    else {
        panic!("read");
    };
    assert_eq!(from, Some(path("old.rs")));
}

/// An answer for an earlier selection is dropped.
#[test]
fn a_stale_diff_answer_is_dropped() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let (first, _) = select(&mut state, "a.rs");
    let (second, _) = select(&mut state, "b.rs");
    assert_ne!(first, second);
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq: first,
            result: Ok(text_diff()),
        },
    );
    assert_eq!(diff_body(state.open.as_ref().unwrap()), DiffBody::Loading);
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq: second,
            result: Err("fatal: bad revision".into()),
        },
    );
    assert_eq!(
        diff_body(state.open.as_ref().unwrap()),
        DiffBody::Failed("fatal: bad revision")
    );
}

/// US1 s7, D4. A large answer shows its counts; Show diff reads it forced and remembers the path.
#[test]
fn show_diff_reads_a_large_file_forced_and_remembers_it() {
    let mut state = ready(&["a.rs", "big.txt"]);
    let (seq, _) = select(&mut state, "big.txt");
    let large = loaded(FileDiff::TooLarge {
        added: 6_000,
        removed: 0,
    });
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(large.clone()),
        },
    );
    let view = state.open.as_ref().unwrap();
    assert_eq!(
        diff_body(view),
        DiffBody::Diff(&large),
        "counts and Show diff, no lines"
    );
    assert!(!can_pick(view));

    let effect = changes::update(&mut state, Msg::ShowLarge);
    let Effect::ReadDiff {
        path: asked,
        force_large,
        ..
    } = effect
    else {
        panic!("Show diff reads the diff, got {effect:?}");
    };
    assert_eq!(asked, path("big.txt"));
    assert!(force_large);
    assert!(state
        .open
        .as_ref()
        .unwrap()
        .shown_large
        .contains(&path("big.txt")));

    // Coming back to it later reads it forced again.
    select(&mut state, "a.rs");
    let (_, forced) = select(&mut state, "big.txt");
    assert!(forced, "a file the user asked to see stays shown");
}

/// D3. Binary, not-UTF-8 and mode-only answers take no comments.
#[test]
fn binary_not_utf8_and_mode_only_allow_no_pick() {
    for diff in [FileDiff::Binary, FileDiff::NotUtf8, FileDiff::ModeOnly] {
        let mut state = ready(&["x"]);
        let (seq, _) = select(&mut state, "x");
        changes::update(
            &mut state,
            Msg::DiffRead {
                seq,
                result: Ok(loaded(diff.clone())),
            },
        );
        assert!(!can_pick(state.open.as_ref().unwrap()), "{diff:?}");
    }
}

/// A list re-read keeps the selection and re-reads its diff, keeping the old one on screen.
#[test]
fn a_list_reread_keeps_the_selection_and_rereads_its_diff() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let (seq, _) = select(&mut state, "b.rs");
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(text_diff()),
        },
    );
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::UncommittedToggled) else {
        panic!("re-read");
    };
    let effect = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["b.rs"])),
        },
    );
    let Effect::ReadDiff {
        path: asked,
        toggles,
        ..
    } = effect
    else {
        panic!("the kept selection's diff is re-read, got {effect:?}");
    };
    assert_eq!(asked, path("b.rs"));
    assert!(!toggles.uncommitted, "under the new toggles");
    assert_eq!(
        diff_body(state.open.as_ref().unwrap()),
        DiffBody::Diff(&text_diff()),
        "the old diff stays on screen while it is re-read"
    );
}

/// A list re-read that drops the selection drops its diff.
#[test]
fn a_list_reread_that_drops_the_selection_drops_its_diff() {
    let mut state = ready(&["a.rs", "b.rs"]);
    select(&mut state, "b.rs");
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::UncommittedToggled) else {
        panic!("re-read");
    };
    let effect = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs"])),
        },
    );
    assert_eq!(effect, Effect::None);
    assert_eq!(
        diff_body(state.open.as_ref().unwrap()),
        DiffBody::NoSelection
    );
}

/// A list re-read that fails drops the selection and its diff: nothing listed backs them.
#[test]
fn a_failed_list_reread_drops_the_selection_and_its_diff() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let (seq, _) = select(&mut state, "b.rs");
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(text_diff()),
        },
    );
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::UncommittedToggled) else {
        panic!("re-read");
    };
    let effect = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Err("fatal: not a git repository".into()),
        },
    );
    assert_eq!(effect, Effect::None);
    let view = state.open.as_ref().unwrap();
    assert_eq!(view.selected, None);
    assert_eq!(diff_body(view), DiffBody::NoSelection);
}

/// Selecting the file already shown keeps its diff and its scroll; a failed read is retried.
#[test]
fn selecting_the_shown_file_again_keeps_its_diff_unless_it_failed() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let (seq, _) = select(&mut state, "b.rs");
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(text_diff()),
        },
    );
    changes::update(
        &mut state,
        Msg::DiffScrolled {
            offset: 120,
            viewport: 400,
        },
    );
    let effect = changes::update(&mut state, Msg::FileSelected(path("b.rs")));
    assert_eq!(effect, Effect::None);
    let view = state.open.as_ref().unwrap();
    assert_eq!(diff_body(view), DiffBody::Diff(&text_diff()));
    assert_eq!(view.diff_offset, 120);

    let (seq, _) = select(&mut state, "a.rs");
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Err("fatal: bad object".into()),
        },
    );
    select(&mut state, "a.rs");
}

// ---- Syntax spans (M3, T042, research R10) ----

use micold_client::features::changes::{cap_spans, SPAN_CAP};
use micold_core::tokens::Rgb;

const KEYWORD: Rgb = Rgb {
    r: 0xA7,
    g: 0x1D,
    b: 0x5D,
};

#[test]
fn spans_past_two_thousand_bytes_are_cut_at_the_cap() {
    let capped = cap_spans([
        (0..4, Some(KEYWORD)),
        (1_990..2_500, Some(KEYWORD)),
        (2_500..3_000, Some(KEYWORD)),
    ]);
    assert_eq!(SPAN_CAP, 2_000);
    assert_eq!(
        capped,
        vec![(0..4, KEYWORD), (1_990..SPAN_CAP, KEYWORD)],
        "the span across the cap ends at it; the one past it is gone"
    );
}

#[test]
fn plain_text_as_an_unknown_extension_highlights_keeps_no_spans() {
    assert_eq!(cap_spans([(0..10, None), (10..20, None)]), vec![]);
    assert_eq!(
        cap_spans([(0..0, Some(KEYWORD)), (0..3, None), (3..5, Some(KEYWORD))]),
        vec![(3..5, KEYWORD)],
        "empty and uncoloured spans are dropped"
    );
}

// ---- Comments (M4, T053, contracts/changes-view.md C1–C4, L2) ----

use micold_client::features::changes::{
    file_comments, pending_counts, ComposerTarget, Pick, ReviewView,
};
use micold_core::protocol::messages::ReviewEditOp;
use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
use micold_core::review::{LineRange, Side};

/// Old `a b c x`, new `a B c d`: context 1 and 3, `b`→`B` at 2, `x`→`d` at 4.
fn commentable() -> LoadedDiff {
    LoadedDiff {
        diff: parse_unified(b"@@ -1,4 +1,4 @@\n a\n-b\n+B\n c\n-x\n+d\n"),
        old: SideLines::from_bytes(b"a\nb\nc\nx\n"),
        new: SideLines::from_bytes(b"a\nB\nc\nd\n"),
        spans: Default::default(),
    }
}

/// A view of `feat-a` listing `a.rs` and `b.rs`, with `b.rs` selected and its diff `diff` shown.
fn showing(diff: LoadedDiff) -> State {
    let mut state = ready(&["a.rs", "b.rs"]);
    let (seq, _) = select(&mut state, "b.rs");
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(diff),
        },
    );
    state
}

fn gutter(state: &mut State, old: Option<u32>, new: Option<u32>, extend: bool) -> Effect {
    changes::update(state, Msg::GutterPressed { old, new, extend })
}

fn pick(state: &State) -> Option<Pick> {
    state.open.as_ref().unwrap().pick
}

fn comment(p: &str, side: Side, start: u32, end: u32, state: CommentState) -> ReviewComment {
    ReviewComment {
        id: CommentId::new(),
        path: path(p),
        side,
        range: LineRange::new(start, end).unwrap(),
        quote: (start..=end).map(|n| format!("line {n}")).collect(),
        text: format!("about {p}:{start}"),
        state,
        created: 1,
    }
}

fn push(state: &mut State, dir: &str, comments: Vec<ReviewComment>) {
    changes::update(
        state,
        Msg::ReviewChanged {
            project: project(),
            worktree_dir: dir.into(),
            comments,
            sending: false,
        },
    );
}

/// C1. A gutter click picks one line; shift-click on the same side extends to a range.
#[test]
fn a_gutter_click_picks_a_line_and_shift_click_extends_it() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(2), false);
    assert_eq!(
        pick(&state),
        Some(Pick {
            side: Side::New,
            anchor: 2,
            head: 2
        })
    );
    gutter(&mut state, None, Some(4), true);
    let picked = pick(&state).unwrap();
    assert_eq!((picked.side, picked.anchor, picked.head), (Side::New, 2, 4));
    assert_eq!(picked.range(), LineRange::new(2, 4).unwrap());
    gutter(&mut state, None, Some(1), true);
    assert_eq!(
        pick(&state).unwrap().range(),
        LineRange::new(1, 2).unwrap(),
        "extending above the anchor keeps the range in order"
    );
}

/// C1. A click on the other side starts a new pick there, even with shift: a pick never spans
/// sides.
#[test]
fn a_click_on_the_other_side_starts_a_new_pick() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(2), false);
    gutter(&mut state, Some(4), None, true);
    assert_eq!(
        pick(&state),
        Some(Pick {
            side: Side::Old,
            anchor: 4,
            head: 4
        })
    );
}

/// C1. A context line counts as the new side, numbered in the new version.
#[test]
fn a_context_line_counts_as_the_new_side() {
    let mut state = showing(commentable());
    gutter(&mut state, Some(2), None, false);
    gutter(&mut state, Some(3), Some(3), true);
    assert_eq!(
        pick(&state),
        Some(Pick {
            side: Side::New,
            anchor: 3,
            head: 3
        }),
        "the context line is on the new side, so it starts a new pick"
    );
}

/// D3, C1. A binary file allows no pick.
#[test]
fn a_binary_file_allows_no_pick() {
    let mut state = showing(loaded(FileDiff::Binary));
    gutter(&mut state, None, Some(1), false);
    assert_eq!(pick(&state), None);
}

/// C2, US2 s2. Add comment opens the composer under the pick; Save sends `Add` with the quote
/// taken from the loaded lines, then closes the composer and drops the pick.
#[test]
fn save_sends_the_comment_with_the_quote_from_the_loaded_lines() {
    let mut state = showing(commentable());
    gutter(&mut state, Some(4), None, false);
    gutter(&mut state, Some(2), None, true);
    assert_eq!(changes::update(&mut state, Msg::AddComment), Effect::None);
    let composer = state.open.as_ref().unwrap().composer.clone().unwrap();
    assert!(
        matches!(composer.target, ComposerTarget::New(p) if p.range() == LineRange::new(2, 4).unwrap())
    );
    assert_eq!(composer.text, "");
    changes::update(
        &mut state,
        Msg::ComposerEdited("  removed too much ".into()),
    );
    let effect = changes::update(&mut state, Msg::ComposerSaved);
    assert_eq!(
        effect,
        Effect::ReviewEdit {
            project: project(),
            worktree_dir: "feat-a".into(),
            edit: ReviewEditOp::Add {
                path: "b.rs".into(),
                side: Side::Old,
                start: 2,
                end: 4,
                quote: vec!["b".into(), "c".into(), "x".into()],
                text: "  removed too much ".into(),
            },
        }
    );
    let view = state.open.as_ref().unwrap();
    assert_eq!(view.composer, None);
    assert_eq!(view.pick, None);
}

/// C2. Save with nothing written sends nothing and keeps the composer open.
#[test]
fn save_with_blank_text_sends_nothing() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(2), false);
    changes::update(&mut state, Msg::AddComment);
    changes::update(&mut state, Msg::ComposerEdited(" \n ".into()));
    assert_eq!(
        changes::update(&mut state, Msg::ComposerSaved),
        Effect::None
    );
    assert!(state.open.as_ref().unwrap().composer.is_some());
    changes::update(&mut state, Msg::ComposerCancelled);
    assert_eq!(state.open.as_ref().unwrap().composer, None);
}

/// US2 s7. Edit opens the composer with the comment's text and Save sends `SetText`; Delete sends
/// `Delete`.
#[test]
fn edit_and_delete_send_set_text_and_delete() {
    let mut state = showing(commentable());
    let c = comment("b.rs", Side::New, 2, 2, CommentState::Pending);
    let id = c.id;
    push(&mut state, "feat-a", vec![c.clone()]);
    changes::update(&mut state, Msg::EditComment(id));
    let composer = state.open.as_ref().unwrap().composer.clone().unwrap();
    assert_eq!(composer.target, ComposerTarget::Edit(id));
    assert_eq!(composer.text, c.text);
    changes::update(&mut state, Msg::ComposerEdited("better".into()));
    assert_eq!(
        changes::update(&mut state, Msg::ComposerSaved),
        Effect::ReviewEdit {
            project: project(),
            worktree_dir: "feat-a".into(),
            edit: ReviewEditOp::SetText {
                id,
                text: "better".into()
            },
        }
    );
    assert_eq!(
        changes::update(&mut state, Msg::DeleteComment(id)),
        Effect::ReviewEdit {
            project: project(),
            worktree_dir: "feat-a".into(),
            edit: ReviewEditOp::Delete { id },
        }
    );
}

/// C4. The composer and its text survive a re-read of the list and of the diff.
#[test]
fn the_composer_survives_a_list_and_diff_reread() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(2), false);
    changes::update(&mut state, Msg::AddComment);
    changes::update(&mut state, Msg::ComposerEdited("half a thought".into()));
    let before = state.open.as_ref().unwrap().composer.clone();
    let Effect::ReadList { seq, .. } = changes::update(&mut state, Msg::UncommittedToggled) else {
        panic!("re-read");
    };
    let Effect::ReadDiff { seq, .. } = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs", "b.rs"])),
        },
    ) else {
        panic!("diff re-read");
    };
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(commentable()),
        },
    );
    assert!(before.is_some());
    assert_eq!(state.open.as_ref().unwrap().composer, before);
}

/// FR-021. A `ReviewChanged` push replaces that entry's comments only.
#[test]
fn a_review_push_replaces_only_that_entrys_comments() {
    let mut state = showing(commentable());
    let a = comment("b.rs", Side::New, 2, 2, CommentState::Pending);
    let d = comment("x.rs", Side::New, 1, 1, CommentState::Pending);
    push(&mut state, "feat-a", vec![a.clone()]);
    push(&mut state, "", vec![d.clone()]);
    push(&mut state, "feat-a", vec![]);
    let key = |dir: &str| (project(), dir.to_string());
    assert_eq!(
        state.reviews.get(&key("feat-a")),
        Some(&ReviewView::default())
    );
    assert_eq!(
        state.reviews.get(&key("")).map(|r| r.comments.clone()),
        Some(vec![d])
    );
}

/// L2. Each listed file has its pending-comment count; sent comments and other entries' do not
/// count.
#[test]
fn the_list_counts_each_files_pending_comments() {
    let mut state = showing(commentable());
    push(
        &mut state,
        "feat-a",
        vec![
            comment("b.rs", Side::New, 2, 2, CommentState::Pending),
            comment("b.rs", Side::Old, 4, 4, CommentState::Pending),
            comment("b.rs", Side::New, 1, 1, CommentState::Sent { at: 5 }),
            comment("a.rs", Side::New, 9, 9, CommentState::Pending),
        ],
    );
    push(
        &mut state,
        "",
        vec![comment("b.rs", Side::New, 3, 3, CommentState::Pending)],
    );
    let counts = pending_counts(&state);
    assert_eq!(counts.get(&path("a.rs")), Some(&1));
    assert_eq!(counts.get(&path("b.rs")), Some(&2));
}

/// C3. The selected file's comments are placed under the row of their last line; one whose anchor
/// is not in the current diff goes to the "Not in the current diff" group.
#[test]
fn comments_not_in_the_diff_are_grouped_apart() {
    let mut state = showing(commentable());
    let on_new = comment("b.rs", Side::New, 1, 2, CommentState::Pending);
    let on_old = comment("b.rs", Side::Old, 4, 4, CommentState::Pending);
    let gone = comment("b.rs", Side::New, 40, 40, CommentState::Pending);
    let other_file = comment("a.rs", Side::New, 2, 2, CommentState::Pending);
    push(
        &mut state,
        "feat-a",
        vec![on_new.clone(), on_old.clone(), gone.clone(), other_file],
    );
    let placed = file_comments(&state);
    assert_eq!(placed.under(Side::New, 2), vec![&on_new]);
    assert_eq!(placed.under(Side::Old, 4), vec![&on_old]);
    assert!(
        placed.under(Side::New, 1).is_empty(),
        "under the last line only"
    );
    assert_eq!(placed.not_in_diff, vec![&gone]);
}

#[test]
fn slot_heights_are_kept_for_the_shown_file_and_layout_only() {
    use micold_core::settings::DiffLayout;
    let mut state = showing(commentable());
    changes::update(
        &mut state,
        Msg::SlotMeasured {
            side: Side::New,
            line: 2,
            height: 122,
        },
    );
    let heights = |s: &State| s.open.as_ref().unwrap().slot_heights.clone();
    assert_eq!(heights(&state).get(&(Side::New, 2)), Some(&122));
    changes::update(&mut state, Msg::LayoutChosen(DiffLayout::SideBySide));
    assert!(
        heights(&state).is_empty(),
        "another layout lays the slots out anew"
    );
    changes::update(
        &mut state,
        Msg::SlotMeasured {
            side: Side::Old,
            line: 4,
            height: 80,
        },
    );
    select(&mut state, "a.rs");
    assert!(heights(&state).is_empty(), "another file has other slots");
}
