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
    let Effect::ReadList { seq, toggles, .. } = effect else {
        panic!("a toggle change re-reads, got {effect:?}");
    };
    assert_eq!(
        toggles,
        Toggles {
            committed: true,
            uncommitted: false
        }
    );
    // A second change while that read runs queues one more read instead of starting a second.
    let effect = changes::update(&mut state, Msg::CommittedToggled);
    assert_eq!(
        effect,
        Effect::None,
        "a toggle change during a read waits for it"
    );
    let off = Toggles {
        committed: false,
        uncommitted: false,
    };
    let view = state.open.as_ref().expect("the view is open");
    assert_eq!(view.toggles, off, "both toggles are off now");
    assert!(
        matches!(view.list, Load::Loading { seq: s, again: true, .. } if s == seq),
        "the running read is marked to run again: {:?}",
        view.list
    );
    // When the running read answers, the queued one reads under the toggles as they are now.
    let effect = changes::update(
        &mut state,
        Msg::ListRead {
            seq,
            result: Ok(list(&["a.rs"])),
        },
    );
    let Effect::ReadList {
        seq: next, toggles, ..
    } = effect
    else {
        panic!("the queued read runs once the first answers, got {effect:?}");
    };
    assert_ne!(next, seq, "the queued read is a new read");
    assert_eq!(toggles, off, "it reads with both toggles off");
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

    let (failed, _) = select(&mut state, "a.rs");
    changes::update(
        &mut state,
        Msg::DiffRead {
            seq: failed,
            result: Err("fatal: bad object".into()),
        },
    );
    let (retry, _) = select(&mut state, "a.rs");
    assert_ne!(
        retry, failed,
        "a failed diff is read again under a new read"
    );
    let view = state.open.as_ref().unwrap();
    assert!(
        matches!(view.diff, Load::Loading { seq, .. } if seq == retry),
        "the body goes back to loading the new read: {:?}",
        view.diff
    );
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
    let c = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
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

/// An edit composer whose comment is gone (deleted here or from another window) closes, rather
/// than saving into a comment that no longer exists (review A M4 F4).
#[test]
fn an_edit_composer_closes_when_its_comment_is_gone() {
    let mut state = showing(commentable());
    let c = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    let id = c.id;
    push(&mut state, "feat-a", vec![c.clone()]);
    changes::update(&mut state, Msg::EditComment(id));
    assert!(state.open.as_ref().unwrap().composer.is_some());
    push(&mut state, "feat-a", vec![]);
    assert_eq!(state.open.as_ref().unwrap().composer, None);

    push(&mut state, "feat-a", vec![c]);
    changes::update(&mut state, Msg::EditComment(id));
    changes::update(&mut state, Msg::DeleteComment(id));
    assert_eq!(state.open.as_ref().unwrap().composer, None);
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
    let a = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    let d = comment("x.rs", Side::New, 1, 1, CommentState::Draft);
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
            comment("b.rs", Side::New, 2, 2, CommentState::Draft),
            comment("b.rs", Side::Old, 4, 4, CommentState::Draft),
            comment("b.rs", Side::New, 1, 1, CommentState::Sent { at: 5 }),
            comment("a.rs", Side::New, 9, 9, CommentState::Draft),
        ],
    );
    push(
        &mut state,
        "",
        vec![comment("b.rs", Side::New, 3, 3, CommentState::Draft)],
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
    let on_new = comment("b.rs", Side::New, 1, 2, CommentState::Draft);
    let on_old = comment("b.rs", Side::Old, 4, 4, CommentState::Draft);
    let gone = comment("b.rs", Side::New, 40, 40, CommentState::Draft);
    let other_file = comment("a.rs", Side::New, 2, 2, CommentState::Draft);
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

// --- M5: Send review (482 S1, S2; renamed by 946 FR-009) ---

use micold_client::features::changes::{send_action, send_error_text, sent_text, SendAction};

fn sending(state: &mut State, dir: &str, comments: Vec<ReviewComment>, on: bool) {
    changes::update(
        state,
        Msg::ReviewChanged {
            project: project(),
            worktree_dir: dir.into(),
            comments,
            sending: on,
        },
    );
}

/// S1. Enabled iff n ≥ 1 pending and no send is open; counts only the open entry's pending
/// comments; reads "Sending…" while a send is open (US2 s5, FR-018).
#[test]
fn send_to_session_is_enabled_only_with_pending_comments_and_no_send_open() {
    let mut state = showing(commentable());
    assert_eq!(
        send_action(&state),
        Some(SendAction {
            label: "Send review (0)".into(),
            enabled: false,
        }),
        "no comments: unavailable (US2 s5)"
    );
    let sent = comment("a.rs", Side::New, 1, 1, CommentState::Sent { at: 5 });
    let pending_a = comment("a.rs", Side::New, 2, 2, CommentState::Draft);
    let pending_b = comment("b.rs", Side::Old, 2, 2, CommentState::Draft);
    push(
        &mut state,
        "",
        vec![comment("x", Side::New, 1, 1, CommentState::Draft)],
    );
    push(&mut state, "feat-a", vec![sent.clone()]);
    assert_eq!(
        send_action(&state).map(|a| (a.label, a.enabled)),
        Some(("Send review (0)".into(), false)),
        "only sent comments here, and another entry's pending one does not count (FR-021)"
    );
    push(
        &mut state,
        "feat-a",
        vec![sent.clone(), pending_a.clone(), pending_b.clone()],
    );
    assert_eq!(
        send_action(&state),
        Some(SendAction {
            label: "Send review (2)".into(),
            enabled: true,
        })
    );
    sending(&mut state, "feat-a", vec![sent, pending_a, pending_b], true);
    assert_eq!(
        send_action(&state),
        Some(SendAction {
            label: "Sending…".into(),
            enabled: false,
        }),
        "a send open in any window disables it (FR-018)"
    );
    assert_eq!(send_action(&State::default()), None, "no view, no action");
}

/// S1. Pressing it sends `ReviewSend` for the open entry (no comment outdated here: the shown
/// file's comment quotes its line as it is); pressing it while unavailable does nothing.
#[test]
fn pressing_send_to_session_sends_review_send_for_the_open_entry() {
    let mut state = showing(commentable());
    assert_eq!(
        changes::update(&mut state, Msg::SendPressed),
        Effect::None,
        "nothing pending"
    );
    let comments = vec![
        comment("a.rs", Side::New, 2, 2, CommentState::Draft),
        quoting("b.rs", Side::New, 4, &["d"]),
    ];
    push(&mut state, "feat-a", comments.clone());
    assert_eq!(
        changes::update(&mut state, Msg::SendPressed),
        Effect::ReviewSend {
            project: project(),
            worktree_dir: "feat-a".into(),
            outdated: Vec::new(),
            count: 2,
        }
    );
    sending(&mut state, "feat-a", comments, true);
    assert_eq!(
        changes::update(&mut state, Msg::SendPressed),
        Effect::None,
        "never while a send is open"
    );
}

/// S2. The snackbars: success names the count and the session; failure carries the service's
/// message.
#[test]
fn the_send_snackbars_name_the_count_the_session_and_the_reason() {
    assert_eq!(
        sent_text(2, "Fix the parser", false),
        "Sent a review of 2 comments to Fix the parser"
    );
    assert_eq!(
        sent_text(1, "Fix the parser", false),
        "Sent a review of 1 comment to Fix the parser"
    );
    assert_eq!(
        sent_text(3, "ignored", true),
        "Started a session and sent a review of 3 comments"
    );
    assert_eq!(
        send_error_text("no session is running in this entry"),
        "Couldn't send the review: no session is running in this entry"
    );
}

// ---- Live refresh and outdated comments (M7, T083, contracts/changes-view.md R1, R2, C4) ----

/// A comment on `p` at `start..=end` of `side` quoting `quote`.
fn quoting(p: &str, side: Side, start: u32, quote: &[&str]) -> ReviewComment {
    let end = start + u32::try_from(quote.len()).unwrap() - 1;
    ReviewComment {
        quote: quote.iter().map(|line| (*line).to_owned()).collect(),
        ..comment(p, side, start, end, CommentState::Draft)
    }
}

/// Answer the list read `effect` asked for with `paths`, returning what the answer asks next.
fn answer_list(state: &mut State, effect: Effect, paths: &[&str]) -> Effect {
    let Effect::ReadList { seq, .. } = effect else {
        panic!("a list read, got {effect:?}");
    };
    changes::update(
        state,
        Msg::ListRead {
            seq,
            result: Ok(list(paths)),
        },
    )
}

/// Answer the diff read `effect` asked for with `diff`.
fn answer_diff(state: &mut State, effect: Effect, diff: LoadedDiff) {
    let Effect::ReadDiff { seq, .. } = effect else {
        panic!("a diff read, got {effect:?}");
    };
    changes::update(
        state,
        Msg::DiffRead {
            seq,
            result: Ok(diff),
        },
    );
}

/// A refresh of `showing(..)`: the change, the list answer `paths`, the diff answer `diff`.
fn refresh(state: &mut State, paths: &[&str], diff: LoadedDiff) {
    let effect = changes::update(state, Msg::Changed);
    let effect = answer_list(state, effect, paths);
    answer_diff(state, effect, diff);
}

fn outdated(state: &State) -> BTreeSet<CommentId> {
    state.open.as_ref().unwrap().outdated.clone()
}

/// R1, FR-010. A change re-reads the list, then the open diff.
#[test]
fn a_change_rereads_the_list_and_the_open_diff() {
    let mut state = showing(commentable());
    let effect = changes::update(&mut state, Msg::Changed);
    assert!(
        matches!(effect, Effect::ReadList { .. }),
        "a change re-reads the list, got {effect:?}"
    );
    let effect = answer_list(&mut state, effect, &["a.rs", "b.rs"]);
    let Effect::ReadDiff { path: asked, .. } = effect else {
        panic!("then the open diff, got {effect:?}");
    };
    assert_eq!(asked, path("b.rs"));
}

/// R9. Changes arriving during a read run one more read after it, not one each.
#[test]
fn changes_during_a_read_are_coalesced_into_one_more_read() {
    let mut state = ready(&["a.rs"]);
    let first = changes::update(&mut state, Msg::Changed);
    assert!(matches!(first, Effect::ReadList { .. }), "got {first:?}");
    for _ in 0..3 {
        assert_eq!(changes::update(&mut state, Msg::Changed), Effect::None);
    }
    let again = answer_list(&mut state, first, &["a.rs"]);
    assert!(
        matches!(again, Effect::ReadList { .. }),
        "one more read after it, got {again:?}"
    );
    assert_eq!(
        answer_list(&mut state, again, &["a.rs"]),
        Effect::None,
        "and no more"
    );
}

/// R2, C4. A refresh drops a pick whose lines are gone and keeps the composer and its text.
#[test]
fn a_refresh_drops_a_pick_whose_lines_are_gone_and_keeps_the_composer() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(4), false);
    changes::update(&mut state, Msg::AddComment);
    changes::update(&mut state, Msg::ComposerEdited("half written".into()));
    let shorter = LoadedDiff {
        diff: parse_unified(b"@@ -1,4 +1,2 @@\n a\n-b\n-c\n-x\n+B\n"),
        old: SideLines::from_bytes(b"a\nb\nc\nx\n"),
        new: SideLines::from_bytes(b"a\nB\n"),
        spans: Default::default(),
    };
    refresh(&mut state, &["a.rs", "b.rs"], shorter);
    assert_eq!(pick(&state), None, "line 4 of the new side is gone");
    let view = state.open.as_ref().unwrap();
    assert_eq!(
        view.composer.as_ref().map(|c| c.text.as_str()),
        Some("half written"),
        "the composer survives the refresh (C4)"
    );
}

/// R2. A refresh keeps a pick whose lines are still there.
#[test]
fn a_refresh_keeps_a_pick_whose_lines_remain() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(2), false);
    refresh(&mut state, &["a.rs", "b.rs"], commentable());
    assert_eq!(
        pick(&state),
        Some(Pick {
            side: Side::New,
            anchor: 2,
            head: 2
        })
    );
}

/// US4 s2, FR-013, R14. A comment whose lines no longer hold its quote is outdated, and its id
/// goes with the send.
#[test]
fn a_comment_whose_quote_no_longer_matches_is_outdated_and_sent_as_such() {
    let mut state = showing(commentable());
    let current = quoting("b.rs", Side::New, 2, &["B"]);
    let changed = quoting("b.rs", Side::New, 3, &["c", "x"]);
    let removed_side = quoting("b.rs", Side::Old, 4, &["x"]);
    push(
        &mut state,
        "feat-a",
        vec![current.clone(), changed.clone(), removed_side.clone()],
    );
    assert_eq!(
        outdated(&state),
        BTreeSet::from([changed.id]),
        "only the comment whose line 4 is now `d`"
    );
    let effect = changes::update(&mut state, Msg::SendPressed);
    let Effect::ReviewSend { outdated: sent, .. } = effect else {
        panic!("a send, got {effect:?}");
    };
    assert_eq!(sent, vec![changed.id], "the send names it (FR-013)");
}

/// US4 s2. An edit on disk makes a shown comment outdated on the next refresh.
#[test]
fn a_refresh_marks_a_comment_outdated_when_its_lines_change() {
    let mut state = showing(commentable());
    let about_b = quoting("b.rs", Side::New, 2, &["B"]);
    push(&mut state, "feat-a", vec![about_b.clone()]);
    assert!(
        outdated(&state).is_empty(),
        "the lines still hold the quote"
    );
    let edited = LoadedDiff {
        new: SideLines::from_bytes(b"a\nBee\nc\nd\n"),
        ..commentable()
    };
    refresh(&mut state, &["a.rs", "b.rs"], edited);
    assert_eq!(outdated(&state), BTreeSet::from([about_b.id]));
}

/// R14. A file no longer listed makes its comments outdated; another file's stay as they were.
#[test]
fn a_file_gone_from_the_list_makes_its_comments_outdated() {
    let mut state = showing(commentable());
    let on_a = quoting("a.rs", Side::New, 1, &["x"]);
    let on_b = quoting("b.rs", Side::New, 2, &["B"]);
    push(&mut state, "feat-a", vec![on_a.clone(), on_b.clone()]);
    assert!(
        outdated(&state).is_empty(),
        "a.rs is listed, its diff not read: not judged"
    );
    refresh(&mut state, &["b.rs"], commentable());
    assert_eq!(outdated(&state), BTreeSet::from([on_a.id]));
}

/// The `shorter` answer of a refresh: line 4 of the new side is gone.
fn shorter() -> LoadedDiff {
    LoadedDiff {
        diff: parse_unified(b"@@ -1,4 +1,2 @@\n a\n-b\n-c\n-x\n+B\n"),
        old: SideLines::from_bytes(b"a\nb\nc\nx\n"),
        new: SideLines::from_bytes(b"a\nB\n"),
        spans: Default::default(),
    }
}

/// C4, review A M7 F1. A composer whose lines a refresh removed is no longer placed, so it
/// cannot save; a new pick places it again with its text, and Save adds the comment there.
#[test]
fn a_composer_whose_lines_are_gone_waits_for_a_new_pick_and_keeps_its_text() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(4), false);
    changes::update(&mut state, Msg::AddComment);
    changes::update(&mut state, Msg::ComposerEdited("half written".into()));
    refresh(&mut state, &["a.rs", "b.rs"], shorter());
    let view = state.open.as_ref().unwrap();
    assert_eq!(changes::composer_placed(view), None, "line 4 is gone");
    assert_eq!(
        changes::update(&mut state, Msg::ComposerSaved),
        Effect::None,
        "nowhere to save it"
    );
    gutter(&mut state, None, Some(2), false);
    let view = state.open.as_ref().unwrap();
    assert_eq!(
        changes::composer_placed(view),
        Some(Pick {
            side: Side::New,
            anchor: 2,
            head: 2
        }),
        "the new pick places it"
    );
    let effect = changes::update(&mut state, Msg::ComposerSaved);
    let Effect::ReviewEdit {
        edit: ReviewEditOp::Add {
            start, end, text, ..
        },
        ..
    } = effect
    else {
        panic!("an add, got {effect:?}");
    };
    assert_eq!((start, end, text.as_str()), (2, 2, "half written"));
}

/// Review A M7 F1. A refresh that drops the selected file closes a new comment's composer: its
/// file is gone, as when another file is selected.
#[test]
fn a_refresh_that_drops_the_selected_file_closes_a_new_comment() {
    let mut state = showing(commentable());
    gutter(&mut state, None, Some(2), false);
    changes::update(&mut state, Msg::AddComment);
    let effect = changes::update(&mut state, Msg::Changed);
    answer_list(&mut state, effect, &["a.rs"]);
    assert_eq!(state.open.as_ref().unwrap().composer, None);
}

// ---- Clear sent and Discard pending (M8, T092, contracts/changes-view.md S3) ----

use micold_client::features::changes::{discard_prompt, tidy_actions, TidyActions};

/// S3. **Clear sent** is enabled iff the open entry has a sent comment, **Discard drafts…** iff
/// it has a pending one and no send is open; another entry's comments do not count (FR-021).
#[test]
fn clear_sent_and_discard_pending_are_enabled_only_with_comments_to_act_on() {
    let mut state = showing(commentable());
    assert_eq!(
        tidy_actions(&state),
        Some(TidyActions {
            clear_sent: false,
            discard_pending: false,
        }),
        "no comments: neither"
    );
    let sent = comment("a.rs", Side::New, 1, 1, CommentState::Sent { at: 5 });
    let pending = comment("a.rs", Side::New, 2, 2, CommentState::Draft);
    push(&mut state, "", vec![sent.clone(), pending.clone()]);
    assert_eq!(
        tidy_actions(&state),
        Some(TidyActions {
            clear_sent: false,
            discard_pending: false,
        }),
        "another entry's comments do not count"
    );
    push(&mut state, "feat-a", vec![sent.clone()]);
    assert_eq!(
        tidy_actions(&state),
        Some(TidyActions {
            clear_sent: true,
            discard_pending: false,
        })
    );
    push(&mut state, "feat-a", vec![pending.clone()]);
    assert_eq!(
        tidy_actions(&state),
        Some(TidyActions {
            clear_sent: false,
            discard_pending: true,
        })
    );
    sending(&mut state, "feat-a", vec![sent, pending], true);
    assert_eq!(
        tidy_actions(&state),
        Some(TidyActions {
            clear_sent: true,
            discard_pending: false,
        }),
        "no discarding while a send is open"
    );
    assert_eq!(tidy_actions(&State::default()), None, "no view, no actions");
}

/// S3. **Clear sent** sends `ClearSent` for the open entry; with nothing sent it sends nothing.
#[test]
fn clear_sent_sends_clear_sent_for_the_open_entry() {
    let mut state = showing(commentable());
    assert_eq!(
        changes::update(&mut state, Msg::ClearSentPressed),
        Effect::None,
        "nothing sent"
    );
    push(
        &mut state,
        "feat-a",
        vec![comment(
            "a.rs",
            Side::New,
            1,
            1,
            CommentState::Sent { at: 5 },
        )],
    );
    assert_eq!(
        changes::update(&mut state, Msg::ClearSentPressed),
        Effect::ReviewEdit {
            project: project(),
            worktree_dir: "feat-a".into(),
            edit: ReviewEditOp::ClearSent,
        }
    );
}

/// S3. **Discard drafts…** only opens the confirmation; Cancel closes it sending nothing; only
/// its confirm sends `DiscardPending` (US4 s3, FR-019).
#[test]
fn discard_pending_asks_first_and_only_the_confirm_sends_it() {
    let mut state = showing(commentable());
    assert_eq!(discard_prompt(&state), None, "no confirmation yet");
    assert_eq!(
        changes::update(&mut state, Msg::DiscardPendingPressed),
        Effect::None
    );
    assert_eq!(
        discard_prompt(&state),
        None,
        "nothing pending: nothing to confirm"
    );

    push(
        &mut state,
        "feat-a",
        vec![
            comment("a.rs", Side::New, 1, 1, CommentState::Draft),
            comment("a.rs", Side::New, 2, 2, CommentState::Draft),
            comment("a.rs", Side::New, 3, 3, CommentState::Sent { at: 5 }),
        ],
    );
    assert_eq!(
        changes::update(&mut state, Msg::DiscardPendingPressed),
        Effect::None,
        "pressing it sends nothing"
    );
    assert_eq!(
        discard_prompt(&state).as_deref(),
        Some("Discard 2 draft comments? This cannot be undone.")
    );
    assert_eq!(
        changes::update(&mut state, Msg::DiscardCancelled),
        Effect::None
    );
    assert_eq!(discard_prompt(&state), None, "Cancel closes it");

    changes::update(&mut state, Msg::DiscardPendingPressed);
    assert_eq!(
        changes::update(&mut state, Msg::DiscardConfirmed),
        Effect::ReviewEdit {
            project: project(),
            worktree_dir: "feat-a".into(),
            edit: ReviewEditOp::DiscardPending,
        }
    );
    assert_eq!(discard_prompt(&state), None, "the confirm closes it");
    assert_eq!(
        changes::update(&mut state, Msg::DiscardConfirmed),
        Effect::None,
        "a confirm with no confirmation open sends nothing"
    );
}

/// S3, review A M8 F4: a push that leaves nothing pending while the question is open makes its
/// confirm send nothing.
#[test]
fn a_confirm_after_the_pending_comments_went_elsewhere_sends_nothing() {
    let mut state = showing(commentable());
    push(
        &mut state,
        "feat-a",
        vec![comment("a.rs", Side::New, 1, 1, CommentState::Draft)],
    );
    changes::update(&mut state, Msg::DiscardPendingPressed);
    push(
        &mut state,
        "feat-a",
        vec![comment(
            "a.rs",
            Side::New,
            1,
            1,
            CommentState::Sent { at: 5 },
        )],
    );
    assert_eq!(
        changes::update(&mut state, Msg::DiscardConfirmed),
        Effect::None
    );
    assert_eq!(discard_prompt(&state), None, "the confirm still closes it");
}

/// S3. One pending comment reads in the singular; closing the view drops the confirmation.
#[test]
fn the_discard_confirmation_counts_in_the_singular_and_closes_with_the_view() {
    let mut state = showing(commentable());
    push(
        &mut state,
        "feat-a",
        vec![comment("a.rs", Side::New, 1, 1, CommentState::Draft)],
    );
    changes::update(&mut state, Msg::DiscardPendingPressed);
    assert_eq!(
        discard_prompt(&state).as_deref(),
        Some("Discard 1 draft comment? This cannot be undone.")
    );
    changes::update(&mut state, Msg::Closed);
    assert_eq!(discard_prompt(&state), None);
}

/// #771: a file the shown list does not have starts no read, so its pane must not say "loading"
/// for good.
#[test]
fn a_selection_the_list_does_not_have_is_not_left_loading() {
    let mut state = ready(&["a.rs"]);
    let effect = changes::update(&mut state, Msg::FileSelected(path("ghost.rs")));
    assert!(matches!(effect, Effect::None), "{effect:?}");
    assert!(
        matches!(diff_body(state.open.as_ref().unwrap()), DiffBody::Failed(_)),
        "{:?}",
        diff_body(state.open.as_ref().unwrap())
    );
}

// ---- Agent reports on the cards (946 M2, T028, FR-002, FR-007) ----

use micold_client::features::changes::{card_model, CardModel};
use micold_core::review::comment::{EntryReview, Outcome, Reporter, Status};
use micold_core::review::reply::CARD_CHARS;

/// `comment` sent at 5, then reported `outcome` with `reply` by a session at 9.
fn reported(comment: ReviewComment, outcome: Outcome, reply: &str) -> ReviewComment {
    let reference = comment.reference();
    let mut entry = EntryReview::from_comments(vec![comment]);
    entry
        .report(
            reference,
            outcome,
            reply,
            Reporter::Agent {
                session: uuid::Uuid::nil(),
            },
            9,
        )
        .unwrap();
    entry.comments()[0].clone()
}

fn card_of(state: &State) -> CardModel {
    let view = state.open.as_ref().unwrap();
    let review = changes::review_of(state, view).unwrap();
    card_model(&review.comments[0], review.sending)
}

/// US1 s1–s2. A pushed report shows its status and its reply made displayable; a reported card
/// has no Edit or Delete. Both windows given the same push show the same card.
#[test]
fn a_pushed_report_shows_its_status_and_reply_in_every_window() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    for (outcome, status) in [
        (Outcome::Applied, Status::Applied),
        (Outcome::Answered, Status::Answered),
    ] {
        let pushed = reported(sent.clone(), outcome, "Renamed\tas \u{1b}asked\nok");
        let mut first = showing(commentable());
        let mut second = showing(commentable());
        push(&mut first, "feat-a", vec![pushed.clone()]);
        push(&mut second, "feat-a", vec![pushed]);
        let card = card_of(&first);
        assert_eq!(
            card,
            CardModel {
                status,
                in_send: false,
                reply: Some(("Renamed␉as ␛asked\nok".to_owned(), false)),
                editable: false,
                markable: false,
            }
        );
        assert_eq!(card_of(&second), card);
    }
}

/// Edge Cases "very long reply": over `CARD_CHARS` the card shows the start and is marked shortened.
#[test]
fn a_reply_over_the_card_limit_is_shortened() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    let long = "x".repeat(CARD_CHARS + 1);
    let model = card_model(&reported(sent.clone(), Outcome::Answered, &long), false);
    assert_eq!(model.reply, Some(("x".repeat(CARD_CHARS), true)));
    let exact = "x".repeat(CARD_CHARS);
    let model = card_model(&reported(sent, Outcome::Answered, &exact), false);
    assert_eq!(model.reply, Some((exact, false)));
}

/// The 482 states keep their cards: an applied report with no reply draws none; a draft is
/// editable only outside a send.
#[test]
fn drafts_and_sent_comments_keep_their_cards() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    let applied = card_model(&reported(sent.clone(), Outcome::Applied, ""), false);
    assert_eq!((applied.status, applied.reply), (Status::Applied, None));
    let model = card_model(&sent, true);
    assert_eq!(
        (model.status, model.editable, model.in_send),
        (Status::Sent, false, false)
    );
    let draft = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    let model = card_model(&draft, false);
    assert_eq!(
        (model.status, model.editable, model.in_send),
        (Status::Draft, true, false)
    );
    let model = card_model(&draft, true);
    assert_eq!((model.editable, model.in_send), (false, true));
}

// ---- Mark by hand and read a full reply (946 M3, T037, FR-006, W14) ----

use micold_client::features::changes::reply_dialog;
use micold_core::review::reply;

/// The `ReviewEdit` an effect carries, if it is one.
fn edit_of(effect: Effect) -> Option<ReviewEditOp> {
    match effect {
        Effect::ReviewEdit { edit, .. } => Some(edit),
        _ => None,
    }
}

/// A view with `comment` pushed for `feat-a`.
fn showing_with(comment: &ReviewComment) -> State {
    let mut state = showing(commentable());
    push(&mut state, "feat-a", vec![comment.clone()]);
    state
}

/// US1 s6. Only a Sent card offers the two hand marks; Draft and Reported cards offer neither.
#[test]
fn only_a_sent_card_offers_mark_applied_and_answer() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    assert!(card_model(&sent, false).markable);
    assert!(
        card_model(&sent, true).markable,
        "another send does not hold it"
    );
    let draft = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    assert!(!card_model(&draft, false).markable);
    for outcome in [Outcome::Applied, Outcome::Answered] {
        let done = reported(sent.clone(), outcome, "ok");
        assert!(!card_model(&done, false).markable, "{outcome:?}");
    }
}

/// W14. **Mark applied** sends `Mark { Applied, "" }` for a Sent comment and nothing otherwise.
#[test]
fn mark_applied_sends_a_mark_for_a_sent_comment_only() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    let mut state = showing_with(&sent);
    assert_eq!(
        edit_of(changes::update(&mut state, Msg::MarkApplied(sent.id))),
        Some(ReviewEditOp::Mark {
            id: sent.id,
            outcome: Outcome::Applied,
            note: String::new(),
        })
    );
    let draft = comment("b.rs", Side::New, 3, 3, CommentState::Draft);
    let done = reported(
        comment("b.rs", Side::New, 4, 4, CommentState::Sent { at: 5 }),
        Outcome::Applied,
        "",
    );
    push(&mut state, "feat-a", vec![draft.clone(), done.clone()]);
    for id in [draft.id, done.id, CommentId::new()] {
        assert_eq!(
            changes::update(&mut state, Msg::MarkApplied(id)),
            Effect::None
        );
    }
}

/// W14. **Answer…** opens the composer; Save is a no-op while the note is blank, then sends
/// `Mark { Answered, note }` and closes it.
#[test]
fn answer_opens_the_composer_and_saves_an_answered_mark() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    let mut state = showing_with(&sent);
    assert_eq!(
        changes::update(&mut state, Msg::AnswerComment(sent.id)),
        Effect::None
    );
    let composer = state.open.as_ref().unwrap().composer.clone().unwrap();
    assert_eq!(composer.target, ComposerTarget::Answer(sent.id));
    assert_eq!(composer.text, "");
    changes::update(&mut state, Msg::ComposerEdited("  \n ".into()));
    assert_eq!(
        changes::update(&mut state, Msg::ComposerSaved),
        Effect::None
    );
    assert!(
        state.open.as_ref().unwrap().composer.is_some(),
        "still open"
    );
    changes::update(&mut state, Msg::ComposerEdited("Kept on purpose".into()));
    assert_eq!(
        edit_of(changes::update(&mut state, Msg::ComposerSaved)),
        Some(ReviewEditOp::Mark {
            id: sent.id,
            outcome: Outcome::Answered,
            note: "Kept on purpose".into(),
        })
    );
    assert!(state.open.as_ref().unwrap().composer.is_none());
}

/// **Answer…** on a comment that is not Sent opens nothing; an Answer composer closes when its
/// comment leaves the entry or is reported meanwhile, so Save never targets either.
#[test]
fn an_answer_composer_closes_when_its_comment_is_gone_or_reported() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    let mut state = showing_with(&sent);
    let draft = comment("b.rs", Side::New, 3, 3, CommentState::Draft);
    push(&mut state, "feat-a", vec![sent.clone(), draft.clone()]);
    changes::update(&mut state, Msg::AnswerComment(draft.id));
    assert!(
        state.open.as_ref().unwrap().composer.is_none(),
        "a draft is not answered"
    );

    for after in [
        vec![draft.clone()],
        vec![reported(sent.clone(), Outcome::Applied, ""), draft.clone()],
    ] {
        changes::update(&mut state, Msg::AnswerComment(sent.id));
        assert!(state.open.as_ref().unwrap().composer.is_some());
        push(&mut state, "feat-a", after);
        assert!(state.open.as_ref().unwrap().composer.is_none());
        push(&mut state, "feat-a", vec![sent.clone(), draft.clone()]);
    }
    changes::update(&mut state, Msg::AnswerComment(sent.id));
    push(&mut state, "feat-b", vec![]);
    assert!(
        state.open.as_ref().unwrap().composer.is_some(),
        "another entry's push leaves it"
    );
}

/// FR-006. **Show all** opens the whole reply, made displayable; closing clears it, and so does
/// the comment leaving the entry.
#[test]
fn show_all_opens_the_full_reply_and_closing_or_losing_the_comment_clears_it() {
    let sent = comment("b.rs", Side::New, 2, 2, CommentState::Sent { at: 5 });
    let long = format!("{}\tend\u{1b}", "x".repeat(CARD_CHARS + 50));
    let answered = reported(sent.clone(), Outcome::Answered, &long);
    let mut state = showing_with(&answered);
    assert_eq!(reply_dialog(&state), None);
    changes::update(&mut state, Msg::ShowReply(answered.id));
    let full = reply_dialog(&state).expect("the dialog is open");
    assert_eq!(full, reply::display(&long));
    assert!(full.chars().count() > CARD_CHARS, "not shortened");
    changes::update(&mut state, Msg::ReplyClosed);
    assert_eq!(reply_dialog(&state), None);

    changes::update(&mut state, Msg::ShowReply(answered.id));
    push(&mut state, "feat-b", vec![]);
    assert!(
        reply_dialog(&state).is_some(),
        "another entry's push leaves it"
    );
    push(&mut state, "feat-a", vec![]);
    assert_eq!(reply_dialog(&state), None, "its comment left the entry");
    assert_eq!(state.open.as_ref().unwrap().reply_open, None);

    changes::update(&mut state, Msg::ShowReply(CommentId::new()));
    assert_eq!(
        state.open.as_ref().unwrap().reply_open,
        None,
        "no such comment"
    );
}

// --- Feature 946, M4: the review panel (FR-016 to FR-019, US3) ---

use micold_client::features::changes::{
    jump_target, panel_rows, review_toolbar_label, status_counts, Jump, Why, PANEL_EMPTY_TEXT,
};

fn panel(state: &State) -> &changes::Panel {
    &state.open.as_ref().unwrap().panel
}

/// FR-016. Rows are ordered by file (bytewise), first line, side (current first), created, and
/// carry their lines label, text start, status and reply start.
#[test]
fn panel_rows_are_ordered_and_describe_their_comments() {
    let mut state = showing(commentable());
    let mut a = comment("b.rs", Side::Old, 2, 2, CommentState::Draft);
    a.created = 1;
    let mut b = comment("b.rs", Side::New, 2, 4, CommentState::Draft);
    b.created = 2;
    let c = comment("a.rs", Side::New, 9, 9, CommentState::Draft);
    let sent = comment("b.rs", Side::New, 1, 1, CommentState::Sent { at: 5 });
    let done = reported(
        comment("b.rs", Side::New, 7, 7, CommentState::Sent { at: 5 }),
        Outcome::Answered,
        "because",
    );
    push(
        &mut state,
        "feat-a",
        vec![a.clone(), b.clone(), c.clone(), sent.clone(), done.clone()],
    );
    let rows = panel_rows(&state);
    let ids: Vec<_> = rows.iter().map(|r| r.id).collect();
    assert_eq!(ids, vec![c.id, sent.id, b.id, a.id, done.id]);
    let labels: Vec<_> = rows.iter().map(|r| r.lines.as_str()).collect();
    assert_eq!(
        labels,
        ["Line 9", "Line 1", "Lines 2–4", "Old line 2", "Line 7"]
    );
    assert_eq!(rows[0].text, "about a.rs:9");
    assert_eq!(rows[1].status, Status::Sent);
    assert_eq!(rows[4].status, Status::Answered);
    assert_eq!(rows[4].reply.as_deref(), Some("because"));
    assert!(rows[2].editable && !rows[1].editable && !rows[4].editable);
    assert!(rows[4].has_full_reply && !rows[0].has_full_reply);
}

/// Edge "file gone". A row whose file is not listed is marked and keeps its status.
#[test]
fn a_row_of_a_file_not_in_the_diff_is_marked_and_keeps_its_status() {
    let mut state = showing(commentable());
    let gone = comment("gone.rs", Side::New, 1, 1, CommentState::Sent { at: 5 });
    let here = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    push(&mut state, "feat-a", vec![gone.clone(), here.clone()]);
    let rows = panel_rows(&state);
    let row = |id| rows.iter().find(|r| r.id == id).unwrap();
    assert!(row(gone.id).not_in_diff);
    assert_eq!(row(gone.id).status, Status::Sent);
    assert!(!row(here.id).not_in_diff);
}

/// FR-016, US3 s5. Counts per status; no comments gives the empty state.
#[test]
fn status_counts_and_the_empty_state() {
    let mut state = showing(commentable());
    assert!(panel_rows(&state).is_empty());
    assert!(PANEL_EMPTY_TEXT.contains("No review comments"));
    assert_eq!(review_toolbar_label(&state), "Review (0)");
    let sent = comment("b.rs", Side::New, 1, 1, CommentState::Sent { at: 5 });
    let draft = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    let draft2 = comment("b.rs", Side::New, 3, 3, CommentState::Draft);
    let applied = reported(sent.clone(), Outcome::Applied, "done");
    push(&mut state, "feat-a", vec![sent, draft, draft2, applied]);
    assert_eq!(
        status_counts(&state),
        [
            (Status::Draft, 2),
            (Status::Sent, 1),
            (Status::Applied, 1),
            (Status::Answered, 0)
        ]
    );
    assert_eq!(review_toolbar_label(&state), "Review (4)");
}

/// FR-016. The panel is open by default; the toolbar toggle closes and reopens it.
#[test]
fn the_toolbar_toggle_closes_and_reopens_the_panel() {
    let mut state = showing(commentable());
    assert!(panel(&state).open);
    changes::update(&mut state, Msg::PanelToggled);
    assert!(!panel(&state).open);
    changes::update(&mut state, Msg::PanelToggled);
    assert!(panel(&state).open);
    changes::update(
        &mut state,
        Msg::PanelScrolled {
            offset: 40,
            viewport: 300,
        },
    );
    assert_eq!((panel(&state).offset, panel(&state).viewport), (40, 300));
}

/// FR-017, US3 s2. Pressing a row selects its file, scrolls its lines into view and focuses it.
#[test]
fn pressing_a_row_selects_the_file_scrolls_and_focuses() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let target = comment("b.rs", Side::New, 4, 4, CommentState::Draft);
    push(&mut state, "feat-a", vec![target.clone()]);
    assert_eq!(
        jump_target(&state, target.id),
        Some(Jump::Shown {
            path: path("b.rs"),
            line: 4
        })
    );
    let effect = changes::update(&mut state, Msg::PanelRowSelected(target.id));
    let Effect::ReadDiff {
        seq, path: asked, ..
    } = effect
    else {
        panic!("the file's diff is read first, got {effect:?}");
    };
    assert_eq!(asked, path("b.rs"));
    assert_eq!(panel(&state).focus, Some(target.id));
    let effect = changes::update(
        &mut state,
        Msg::DiffRead {
            seq,
            result: Ok(commentable()),
        },
    );
    // Unified rows: header, a, -b, +B, c, -x, +d: line 4 is row 6; two rows above it stay.
    assert_eq!(effect, Effect::ScrollDiff { offset: 4 * 20 });
    assert_eq!(state.open.as_ref().unwrap().diff_offset, 80);
    assert!(!panel(&state).to_show);
    assert!(panel_rows(&state)[0].selected);
    // With the file already shown, the press scrolls at once.
    changes::update(
        &mut state,
        Msg::DiffScrolled {
            offset: 0,
            viewport: 100,
        },
    );
    let effect = changes::update(&mut state, Msg::PanelRowSelected(target.id));
    assert_eq!(effect, Effect::ScrollDiff { offset: 80 });
}

/// FR-017. A refused jump says why and moves nothing.
#[test]
fn a_hidden_comment_gives_a_reason_and_no_jump() {
    let mut state = ready(&["a.rs", "b.rs"]);
    let gone = comment("gone.rs", Side::New, 1, 1, CommentState::Draft);
    push(&mut state, "feat-a", vec![gone.clone()]);
    assert_eq!(
        jump_target(&state, gone.id),
        Some(Jump::Hidden(Why::NotListed))
    );
    assert_eq!(
        changes::update(&mut state, Msg::PanelRowSelected(gone.id)),
        Effect::None
    );
    assert_eq!(panel(&state).hidden, Some(Why::NotListed));
    assert_eq!(panel(&state).focus, None);
    assert!(state.open.as_ref().unwrap().selected.is_none());
    changes::update(&mut state, Msg::CommittedToggled);
    assert_eq!(
        jump_target(&state, gone.id),
        Some(Jump::Hidden(Why::KindToggledOff))
    );
    assert_ne!(Why::KindToggledOff.message(), Why::NotListed.message());
}

/// FR-018, US3 s4. Edit on a draft row opens the composer, Delete sends the delete; settled rows
/// offer neither (the reducer ignores them too), and a row with a reply opens it.
#[test]
fn rows_edit_and_delete_drafts_and_open_replies() {
    let mut state = showing(commentable());
    let draft = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    let sent = comment("b.rs", Side::New, 3, 3, CommentState::Sent { at: 5 });
    let done = reported(
        comment("b.rs", Side::New, 4, 4, CommentState::Sent { at: 5 }),
        Outcome::Applied,
        "fixed",
    );
    push(
        &mut state,
        "feat-a",
        vec![draft.clone(), sent.clone(), done.clone()],
    );
    changes::update(&mut state, Msg::EditComment(draft.id));
    let composer = state.open.as_ref().unwrap().composer.clone().unwrap();
    assert_eq!(composer.text, draft.text);
    changes::update(&mut state, Msg::ComposerCancelled);
    changes::update(&mut state, Msg::EditComment(sent.id));
    assert!(state.open.as_ref().unwrap().composer.is_none());
    assert_eq!(
        edit_of(changes::update(&mut state, Msg::DeleteComment(draft.id))),
        Some(ReviewEditOp::Delete { id: draft.id })
    );
    changes::update(&mut state, Msg::ShowReply(done.id));
    assert_eq!(state.open.as_ref().unwrap().reply_open, Some(done.id));
}

/// FR-019. The panel's buttons are the toolbar's, so they share its availability; a push from
/// another window changes the rows.
#[test]
fn a_push_changes_the_rows_and_the_buttons_follow_the_toolbar() {
    let mut state = showing(commentable());
    assert!(!changes::send_action(&state).unwrap().enabled);
    let draft = comment("b.rs", Side::New, 2, 2, CommentState::Draft);
    push(&mut state, "feat-a", vec![draft.clone()]);
    assert_eq!(panel_rows(&state).len(), 1);
    assert!(changes::send_action(&state).unwrap().enabled);
    assert!(changes::tidy_actions(&state).unwrap().discard_pending);
    push(&mut state, "feat-a", vec![]);
    assert!(panel_rows(&state).is_empty());
}

/// SC-006. Rows and counts for 200 comments take well under 100 ms.
#[test]
fn rows_and_counts_for_200_comments_stay_within_the_budget() {
    let mut state = showing(commentable());
    let many: Vec<_> = (1..=200)
        .map(|n| comment("b.rs", Side::New, n, n, CommentState::Draft))
        .collect();
    push(&mut state, "feat-a", many);
    let started = std::time::Instant::now();
    let rows = panel_rows(&state);
    let counts = status_counts(&state);
    assert!(started.elapsed() < std::time::Duration::from_millis(100));
    assert_eq!(rows.len(), 200);
    assert_eq!(counts[0], (Status::Draft, 200));
}
