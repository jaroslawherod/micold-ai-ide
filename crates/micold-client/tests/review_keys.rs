//! The Review tab's keyboard (#1274 M3, M4): who has the keyboard (research R7, FR-002, SC-002),
//! what `j k n p v g` (US1, US2), `] [ c` (US3), the send chord (US4) and `?` (US5) do to the shown
//! view (contract K7), and the single-key switch (FR-022).
//!
//! Builds `features::changes::State` (and the root `app::State` for the holder and the switch) and
//! drives them with the messages the listener and the shell send.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use micold_client::app::{Message, State as App};
use micold_client::features::changes::{
    self, composer_press, file_reviews, key_pressed, review_holder, shortcut_sheet, with_key_hint,
    Anchor, ComposerTarget, DiffRow, Effect, Msg, OpenView, Pick, State, NEXT_HUNK, SEND_TOOLTIP,
};
use micold_client::features::window::FieldId;
use micold_core::protocol::messages::ReviewEditOp;
use micold_core::review::changes::{ChangedFile, Content};
use micold_core::review::comment::{CommentId, CommentState, ReviewComment};
use micold_core::review::diff::{parse_unified, LoadedDiff, SideLines};
use micold_core::review::drawer::DrawerTab;
use micold_core::review::keys::{
    sheet_rows, Holder, Press, PressKey, ReviewAction, SINGLE_KEYS_OFF_NOTE,
};
use micold_core::review::marks::ReviewState;
use micold_core::review::navigate::{Edge, ALL_FILES_REVIEWED, PICK_OR_HUNK};
use micold_core::review::{LineRange, Side};
use micold_core::session::SessionLocation;

// --- fixtures -------------------------------------------------------------------------------------

mod support;
use support::review_fixture::{draft, file, id, path};

/// Open `entry`'s Review tab in `state` and answer its first read with `files`.
fn open_into(state: &mut State, entry: SessionLocation, files: &[ChangedFile]) {
    support::review_fixture::open_into(state, project(), entry, files);
}

/// The service pushes `feat-a`'s reviewed marks: each path on its current id.
fn marked(state: &mut State, files: &[&str], all: &[&str], comments: Vec<ReviewComment>) {
    support::review_fixture::marked(state, project(), "feat-a", files, all, comments);
}

/// The empty project: the one the root shows with no active project.
fn project() -> PathBuf {
    PathBuf::new()
}

fn worktree() -> SessionLocation {
    SessionLocation::Worktree("feat-a".into())
}

fn ready(paths: &[&str]) -> State {
    let files: Vec<ChangedFile> = paths.iter().map(|p| file(p)).collect();
    let mut state = State::default();
    open_into(&mut state, worktree(), &files);
    state
}

/// A diff of two hunks, lines 1 and 10.
fn two_hunks() -> LoadedDiff {
    let lines: Vec<String> = (1..=12).map(|n| format!("l{n}")).collect();
    let text = lines.join("\n") + "\n";
    LoadedDiff {
        diff: parse_unified(b"@@ -1,1 +1,1 @@\n-l1\n+L1\n@@ -10,1 +10,1 @@\n-l10\n+L10\n"),
        old: SideLines::from_bytes(text.as_bytes()),
        new: SideLines::from_bytes(text.replace("l1\n", "L1\n").as_bytes()),
        spans: Default::default(),
    }
}

fn view(state: &State) -> &OpenView {
    state.tabs.shown_view().expect("the review is shown")
}

fn selected(state: &State) -> Option<String> {
    view(state).selected.as_ref().map(|p| p.as_str().to_owned())
}

/// Every effect of `effect`, batches flattened.
fn effects(effect: Effect) -> Vec<Effect> {
    match effect {
        Effect::Batch(list) => list.into_iter().flat_map(effects).collect(),
        other => vec![other],
    }
}

fn diff_read(effect: &[Effect]) -> Option<u64> {
    effect.iter().find_map(|e| match e {
        Effect::ReadDiff { seq, .. } => Some(*seq),
        _ => None,
    })
}

fn scroll_diff(effect: &[Effect]) -> Option<u32> {
    effect.iter().find_map(|e| match e {
        Effect::ScrollDiff { offset } => Some(*offset),
        _ => None,
    })
}

fn key(state: &mut State, action: ReviewAction) -> Vec<Effect> {
    effects(changes::update(state, Msg::ReviewKey(action)))
}

/// Answer the selected file's diff read `seq` with `diff`.
fn answer(state: &mut State, seq: u64, diff: LoadedDiff) -> Vec<Effect> {
    effects(changes::update(
        state,
        Msg::DiffRead {
            seq,
            result: Ok(diff),
            row: DiffRow::NotRead,
        },
    ))
}

/// Select `p` by a tree click and show `diff` for it.
fn shown_with(state: &mut State, p: &str, diff: LoadedDiff) {
    let effect = effects(changes::update(state, Msg::FileSelected(path(p))));
    let seq = diff_read(&effect).expect("a diff read");
    answer(state, seq, diff);
}

fn press(c: char) -> Press {
    Press {
        key: PressKey::Char(c),
        primary: false,
        ctrl: false,
        alt: false,
        logo: false,
        shift: false,
        repeat: false,
    }
}

/// The root with `feat-a`'s Review tab shown, listing `paths`.
fn app_with(paths: &[&str]) -> App {
    let files: Vec<ChangedFile> = paths.iter().map(|p| file(p)).collect();
    let mut app = App::default();
    open_into(&mut app.changes, worktree(), &files);
    assert!(
        app.review_shown().is_some(),
        "precondition: the review is shown"
    );
    app
}

// --- T015: who has the keyboard --------------------------------------------------------------------

/// R7: the review holds the keyboard while its tab is shown and nothing else does (a Tab focus on a
/// button reports no field, so it is the same case); nothing shown, it does not.
#[test]
fn the_review_has_the_keyboard_while_its_tab_is_shown_and_nothing_else_has_it() {
    assert_eq!(review_holder(&App::default()), Holder::Other);
    let app = app_with(&["a.rs"]);
    assert_eq!(review_holder(&app), Holder::Review);
}

/// US1 s6: the comment box has the keyboard while it is open and focused; a closed box that never
/// reported its blur gives it back.
#[test]
fn the_open_focused_comment_box_holds_the_keyboard() {
    let mut app = app_with(&["a.rs"]);
    shown_with(&mut app.changes, "a.rs", two_hunks());
    changes::update(
        &mut app.changes,
        Msg::GutterPressed {
            old: None,
            new: Some(1),
            extend: false,
        },
    );
    changes::update(&mut app.changes, Msg::AddComment);
    assert!(view(&app.changes).composer.is_some(), "precondition");
    app.window.focused_field = Some(FieldId::ReviewComposer);
    assert_eq!(review_holder(&app), Holder::Composer);
    assert_eq!(key_pressed(&app, &press('j')), None, "j is typed text");
    changes::update(&mut app.changes, Msg::ComposerCancelled);
    assert_eq!(review_holder(&app), Holder::Review);
}

/// FR-002, SC-002: a focused field, Settings, the Overview, the quick switcher or a dialog on top
/// keep the keys.
#[test]
fn a_field_settings_the_overview_or_a_surface_on_top_has_the_keyboard() {
    let mut app = app_with(&["a.rs"]);
    app.window.focused_field = Some(FieldId::ReviewTreeFilter);
    assert_eq!(review_holder(&app), Holder::Other, "the tree filter");
    app.window.focused_field = Some(FieldId::SettingsReduceMotion);
    assert_eq!(review_holder(&app), Holder::Other, "any field");
    app.window.focused_field = None;

    let mut settings = app.clone();
    settings.settings.settings_draft = Some(Default::default());
    assert_eq!(review_holder(&settings), Holder::Other, "Settings");

    let mut overview = app.clone();
    overview.overview.open = true;
    assert_eq!(review_holder(&overview), Holder::Other, "the Overview");

    let mut switcher = app.clone();
    switcher.update(Message::QuickSwitch(
        micold_client::features::quick_switch::Msg::Toggled {
            terminal_had_focus: false,
        },
    ));
    assert!(micold_client::overlay::registry::topmost(&switcher).is_some());
    assert_eq!(
        review_holder(&switcher),
        Holder::Other,
        "the quick switcher"
    );

    let mut dialog = app.clone();
    dialog.update(Message::Help(
        micold_client::features::help::Msg::AboutOpened,
    ));
    assert_eq!(review_holder(&dialog), Holder::Other, "a dialog");
    assert_eq!(key_pressed(&dialog, &press('j')), None);
    assert_eq!(
        key_pressed(&app, &press('j')),
        Some(Msg::ReviewKey(ReviewAction::NextFile))
    );
}

/// Quickstart §C, FR-002: with the terminal shown and focused (the review hidden behind it), no
/// review key or chord resolves, so each one reaches the terminal as it does today.
#[test]
fn a_focused_terminal_keeps_every_review_key() {
    use micold_client::features::session::Msg as SessionMsg;
    use micold_core::review::keys::{KeyName, REVIEW_KEYS};
    use micold_core::session::{AiCli, Session};
    let mut app = app_with(&["a.rs"]);
    app.update(Message::Session(SessionMsg::Started(Session::start_new(
        SessionLocation::Worktree("feat-b".to_string()),
        AiCli::ClaudeCode,
    ))));
    app.update(Message::Session(SessionMsg::TerminalFocused));
    assert!(
        app.terminal_focused(),
        "precondition: the terminal has the keyboard"
    );
    assert_eq!(review_holder(&app), Holder::Other);
    for row in &REVIEW_KEYS {
        let held = match row.key {
            KeyName::Char(c) => press(c),
            KeyName::Enter => Press {
                shift: row.mods.shift,
                ..chord()
            },
        };
        assert_eq!(
            key_pressed(&app, &held),
            None,
            "`{}` goes to the terminal",
            row.labels().0
        );
    }
}

// --- T017: j k n p ----------------------------------------------------------------------------------

/// US1 s1–s2, FR-003: `j`/`k` select through the tree's select, wrap, show the diff from the top.
#[test]
fn j_and_k_select_the_neighbour_file_and_wrap() {
    let mut state = ready(&["a.rs", "b.rs", "c.rs"]);
    let first = key(&mut state, ReviewAction::NextFile);
    assert_eq!(
        selected(&state).as_deref(),
        Some("a.rs"),
        "no selection: first"
    );
    assert!(diff_read(&first).is_some(), "its diff is read");
    assert_eq!(scroll_diff(&first), Some(0), "shown from the top");
    key(&mut state, ReviewAction::NextFile);
    assert_eq!(selected(&state).as_deref(), Some("b.rs"));
    key(&mut state, ReviewAction::NextFile);
    key(&mut state, ReviewAction::NextFile);
    assert_eq!(
        selected(&state).as_deref(),
        Some("a.rs"),
        "wraps at the end"
    );
    key(&mut state, ReviewAction::PrevFile);
    assert_eq!(
        selected(&state).as_deref(),
        Some("c.rs"),
        "and at the start"
    );
}

/// FR-003: a file in a collapsed folder is visited, its folder expanded and its row brought into
/// view.
#[test]
fn k_reveals_a_file_in_a_collapsed_folder_and_brings_its_row_into_view() {
    let names: Vec<String> = (0..30).map(|i| format!("a{i:02}.rs")).collect();
    let paths: Vec<&str> = names
        .iter()
        .map(String::as_str)
        .chain(["src/x.rs"])
        .collect();
    let mut state = ready(&paths);
    changes::update(&mut state, Msg::FileSelected(path("a00.rs")));
    let shown = state.tabs.shown_view_mut().unwrap();
    shown.tree.collapsed.insert("src".into());
    shown.list_offset = 960;
    shown.list_viewport = 480;
    // Folders come first, so the folded src/x.rs is the file before a00.rs.
    let effect = key(&mut state, ReviewAction::PrevFile);
    assert_eq!(selected(&state).as_deref(), Some("src/x.rs"));
    assert!(!view(&state).tree.collapsed.contains("src"), "expanded");
    let scrolled = effect.iter().find_map(|e| match e {
        Effect::ScrollList { offset } => Some(*offset),
        _ => None,
    });
    assert!(
        scrolled.is_some_and(|o| o < 960),
        "row brought into view: {scrolled:?}"
    );
    assert_eq!(scrolled, Some(view(&state).list_offset));
}

/// US1 s5: a filtered tree visits only the files it shows.
#[test]
fn a_filtered_tree_visits_only_the_files_it_shows() {
    let mut state = ready(&["a.txt", "b.rs", "c.rs"]);
    changes::update(&mut state, Msg::TreeFilterChanged(".rs".into()));
    let mut seen = Vec::new();
    for _ in 0..4 {
        key(&mut state, ReviewAction::NextFile);
        seen.push(selected(&state).unwrap());
    }
    assert_eq!(seen, ["b.rs", "c.rs", "b.rs", "c.rs"]);
    key(&mut state, ReviewAction::PrevFile);
    assert_eq!(selected(&state).as_deref(), Some("b.rs"));
}

/// US1 s3, FR-004, FR-005: `n` makes the first hunk current and scrolls its header to the top; the
/// next `n` the second; a tree click clears it.
#[test]
fn n_marks_the_next_hunk_and_a_tree_click_clears_it() {
    let mut state = ready(&["a.rs", "b.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    let first = key(&mut state, ReviewAction::NextHunk);
    assert_eq!(view(&state).current_hunk, Some(0));
    assert_eq!(scroll_diff(&first), Some(0));
    let second = key(&mut state, ReviewAction::NextHunk);
    assert_eq!(view(&state).current_hunk, Some(1));
    let offset = scroll_diff(&second).expect("scrolled to it");
    assert!(offset > 0, "the second header is lower");
    assert_eq!(view(&state).diff_offset, offset);
    assert_eq!(
        changes::current_header_row(view(&state), state.layout),
        Some(micold_core::review::diff::header_rows(&view(&state).shaped, state.layout)[1])
    );
    key(&mut state, ReviewAction::PrevHunk);
    assert_eq!(view(&state).current_hunk, Some(0));
    changes::update(&mut state, Msg::FileSelected(path("b.rs")));
    assert_eq!(view(&state).current_hunk, None, "another file clears it");
}

/// US1 s4, research R5: past the last hunk `n` selects the next file and makes its first hunk
/// current once its diff is read — only that read; `p` before the first goes to the previous file's
/// last hunk.
#[test]
fn n_and_p_cross_files_with_a_pending_edge() {
    let mut state = ready(&["a.rs", "b.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    key(&mut state, ReviewAction::NextHunk);
    key(&mut state, ReviewAction::NextHunk);
    let crossed = key(&mut state, ReviewAction::NextHunk);
    assert_eq!(selected(&state).as_deref(), Some("b.rs"));
    assert_eq!(view(&state).current_hunk, None);
    assert_eq!(view(&state).pending_edge, Some((path("b.rs"), Edge::First)));
    let seq = diff_read(&crossed).expect("its diff is read");
    // A read that is not the current one leaves the edge pending.
    answer(&mut state, seq + 100, two_hunks());
    assert_eq!(view(&state).pending_edge, Some((path("b.rs"), Edge::First)));
    let shown = answer(&mut state, seq, two_hunks());
    assert_eq!(view(&state).current_hunk, Some(0));
    assert_eq!(view(&state).pending_edge, None);
    assert_eq!(scroll_diff(&shown), Some(0));

    let back = key(&mut state, ReviewAction::PrevHunk);
    assert_eq!(selected(&state).as_deref(), Some("a.rs"));
    let seq = diff_read(&back).expect("read again");
    answer(&mut state, seq, two_hunks());
    assert_eq!(view(&state).current_hunk, Some(1), "its last hunk");
}

/// FR-004: with no hunk anywhere `n` does nothing; moving keeps picks, layout and whole-file
/// choices.
#[test]
fn moving_changes_nothing_but_the_selection_and_the_current_hunk() {
    let mut state = ready(&["a.rs"]);
    let binary = {
        let mut f = file("bin.dat");
        f.content = Content::Binary;
        f
    };
    let mut none = State::default();
    open_into(&mut none, worktree(), &[binary]);
    let before = none.clone();
    assert_eq!(key(&mut none, ReviewAction::NextHunk), vec![Effect::None]);
    assert_eq!(none, before, "Nothing changes nothing");

    shown_with(&mut state, "a.rs", two_hunks());
    changes::update(
        &mut state,
        Msg::GutterPressed {
            old: None,
            new: Some(1),
            extend: false,
        },
    );
    let pick = view(&state).pick;
    assert!(pick.is_some(), "precondition");
    let layout = state.layout;
    let whole = changes::whole_file(view(&state));
    key(&mut state, ReviewAction::NextHunk);
    key(&mut state, ReviewAction::NextHunk);
    assert_eq!(view(&state).pick, pick, "the pick stays");
    assert_eq!(
        state.layout, layout,
        "the unified/side-by-side choice stays"
    );
    assert_eq!(
        changes::whole_file(view(&state)),
        whole,
        "the whole-file choice stays"
    );
}

/// FR-014: a press acts on the shown entry's tab only.
#[test]
fn a_press_leaves_a_hidden_tab_alone() {
    let mut state = ready(&["a.rs", "b.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    let hidden_key = (project(), worktree());
    open_into(
        &mut state,
        SessionLocation::Worktree("feat-b".into()),
        &[file("z.rs")],
    );
    let before = state.tabs.get(&hidden_key).cloned();
    for action in [
        ReviewAction::NextFile,
        ReviewAction::NextHunk,
        ReviewAction::ToggleReviewed,
        ReviewAction::NextUnreviewed,
    ] {
        key(&mut state, action);
    }
    assert_eq!(state.tabs.get(&hidden_key).cloned(), before);
    assert_eq!(selected(&state).as_deref(), Some("z.rs"));
}

/// SC-003: on a 200-file, 500-comment review each arm answers in under 100 ms.
#[test]
fn every_arm_is_fast_on_a_large_review() {
    let paths: Vec<String> = (0..200).map(|i| format!("d{}/f{i:03}.rs", i % 7)).collect();
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    let mut state = ready(&paths);
    let comments = (0..500)
        .map(|i| ReviewComment {
            id: CommentId::new(),
            path: path(paths[i % 200]),
            side: Side::New,
            range: LineRange::new(1, 1).expect("a range"),
            quote: vec!["l1".into()],
            text: "x".into(),
            state: CommentState::Draft,
            created: 1,
            pr: None,
        })
        .collect();
    marked(&mut state, &paths[..100], &paths, comments);
    shown_with(&mut state, paths[0], two_hunks());
    // M4 (T033, T041): `] [ c ?` and the send chord too.
    for action in [
        ReviewAction::NextFile,
        ReviewAction::PrevFile,
        ReviewAction::NextHunk,
        ReviewAction::PrevHunk,
        ReviewAction::ToggleReviewed,
        ReviewAction::NextUnreviewed,
        ReviewAction::NextComment,
        ReviewAction::PrevComment,
        ReviewAction::Comment,
        ReviewAction::ToggleSheet,
        ReviewAction::ToggleSheet,
        ReviewAction::Send,
    ] {
        let start = Instant::now();
        key(&mut state, action);
        assert!(
            start.elapsed() < Duration::from_millis(100),
            "{action:?} took {:?}",
            start.elapsed()
        );
    }
}

// --- T024: v g ------------------------------------------------------------------------------------

/// US2 s1–s2, FR-007: `v` sends **Mark reviewed**'s edit for the selected file, nothing without one.
#[test]
fn v_sends_the_mark_reviewed_edit_for_the_selected_file_only() {
    let all = ["a.rs", "b.rs"];
    let mut state = ready(&all);
    assert_eq!(
        key(&mut state, ReviewAction::ToggleReviewed),
        vec![Effect::None]
    );
    changes::update(&mut state, Msg::FileSelected(path("b.rs")));
    let pressed = changes::update(&mut state.clone(), Msg::MarkToggled);
    assert_eq!(
        key(&mut state, ReviewAction::ToggleReviewed),
        vec![pressed.clone()]
    );
    let Effect::ReviewEdit {
        edit: ReviewEditOp::SetReviewed {
            path: p, change, ..
        },
        ..
    } = pressed
    else {
        panic!("a mark edit, got {pressed:?}");
    };
    assert_eq!((p.as_str(), change), ("b.rs", Some(id(2))));
    marked(&mut state, &["b.rs"], &all, Vec::new());
    let unmark = key(&mut state, ReviewAction::ToggleReviewed);
    assert!(matches!(
        unmark.as_slice(),
        [Effect::ReviewEdit {
            edit: ReviewEditOp::SetReviewed { change: None, .. },
            ..
        }]
    ));
}

/// US2 s3–s5, FR-008: `g` visits the files not reviewed in file order, wrapping, skips one another
/// window marked, and with every file reviewed keeps the selection and says so.
#[test]
fn g_selects_the_next_file_not_reviewed() {
    let all = ["f1.rs", "f2.rs", "f3.rs", "f4.rs", "f5.rs"];
    let mut state = ready(&all);
    marked(&mut state, &["f1.rs", "f3.rs", "f5.rs"], &all, Vec::new());
    changes::update(&mut state, Msg::FileSelected(path("f1.rs")));
    let mut seen = Vec::new();
    for _ in 0..3 {
        key(&mut state, ReviewAction::NextUnreviewed);
        seen.push(selected(&state).unwrap());
    }
    assert_eq!(seen, ["f2.rs", "f4.rs", "f2.rs"]);
    // Another window marks f4.rs.
    marked(
        &mut state,
        &["f1.rs", "f3.rs", "f4.rs", "f5.rs"],
        &all,
        Vec::new(),
    );
    key(&mut state, ReviewAction::NextUnreviewed);
    assert_eq!(
        selected(&state).as_deref(),
        Some("f2.rs"),
        "f4.rs is skipped"
    );
    marked(&mut state, &all, &all, Vec::new());
    key(&mut state, ReviewAction::NextUnreviewed);
    assert_eq!(
        selected(&state).as_deref(),
        Some("f2.rs"),
        "the selection stays"
    );
    assert_eq!(view(&state).notice, Some(ALL_FILES_REVIEWED));
}

/// SC-001: on a 30-file review, `j` then `v` per file leaves every file marked reviewed.
#[test]
fn j_then_v_per_file_marks_a_thirty_file_review() {
    let names: Vec<String> = (0..30).map(|i| format!("f{i:02}.rs")).collect();
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut state = ready(&all);
    let mut done: Vec<&str> = Vec::new();
    for _ in 0..30 {
        key(&mut state, ReviewAction::NextFile);
        let edits = key(&mut state, ReviewAction::ToggleReviewed);
        let [Effect::ReviewEdit {
            edit: ReviewEditOp::SetReviewed { path: p, .. },
            ..
        }] = edits.as_slice()
        else {
            panic!("a mark edit, got {edits:?}");
        };
        done.push(all.iter().find(|q| **q == p.as_str()).expect("listed"));
        // The service's push of the new mark.
        marked(&mut state, &done, &all, Vec::new());
    }
    let reviews = file_reviews(&state, view(&state), true);
    assert_eq!(reviews.len(), 30);
    assert!(reviews.values().all(|r| r.state == ReviewState::Reviewed));
}

// --- T063: the single-key switch --------------------------------------------------------------------

/// US1 s7, FR-022: with the window's switch off, `j`, `n` and `v` change nothing, and the hunk and
/// **Mark reviewed** tooltips name no key.
#[test]
fn with_the_switch_off_the_single_keys_do_nothing_and_tooltips_name_none() {
    let mut app = app_with(&["a.rs", "b.rs"]);
    shown_with(&mut app.changes, "a.rs", two_hunks());
    app.settings.review_single_keys = false;
    let before = app.changes.clone();
    for c in ['j', 'n', 'v', 'k', 'p', 'g'] {
        assert_eq!(
            app.update_changes(Msg::KeyPressed(press(c))),
            Effect::None,
            "{c}"
        );
    }
    assert_eq!(
        app.changes, before,
        "selection, current hunk and marks unchanged"
    );
    assert_eq!(
        with_key_hint(NEXT_HUNK, ReviewAction::NextHunk, false),
        NEXT_HUNK
    );
    let mark = changes::MARK_TOOLTIP;
    assert_eq!(
        with_key_hint(mark, ReviewAction::ToggleReviewed, false),
        mark
    );

    app.settings.review_single_keys = true;
    app.update_changes(Msg::KeyPressed(press('j')));
    assert_eq!(
        selected(&app.changes).as_deref(),
        Some("b.rs"),
        "on, `j` acts"
    );
    assert_eq!(
        with_key_hint(NEXT_HUNK, ReviewAction::NextHunk, true),
        "Next change (n)"
    );
    assert_eq!(
        with_key_hint(mark, ReviewAction::ToggleReviewed, true),
        format!("{mark} (v)")
    );
}

// --- T033: ] [ c ------------------------------------------------------------------------------------

/// US3 s1–s2, FR-006: `]` shows each comment's card in comment order, selecting `c.rs` for the
/// third and wrapping; `[` steps back. The card is reached through the drawer row's pending jump.
/// (Line 5 stands for the scenario's line 20: the fixture's file has 12 lines, and a comment whose
/// quote is not in the file is outdated, which the next test covers.)
#[test]
fn close_bracket_and_open_bracket_step_through_the_comments_and_wrap() {
    let all = ["a.rs", "b.rs", "c.rs"];
    let mut state = ready(&all);
    let (a3, a5, c7) = (
        draft("a.rs", 3, 1),
        draft("a.rs", 5, 2),
        draft("c.rs", 7, 3),
    );
    marked(
        &mut state,
        &[],
        &all,
        vec![c7.clone(), a5.clone(), a3.clone()],
    );
    shown_with(&mut state, "a.rs", two_hunks());

    let first = key(&mut state, ReviewAction::NextComment);
    assert_eq!(view(&state).last_comment, Some(a3.id));
    assert_eq!(
        view(&state).drawer.focus,
        Some(a3.id),
        "its card is the jump's"
    );
    assert!(
        scroll_diff(&first).is_some(),
        "a shown file scrolls at once"
    );
    key(&mut state, ReviewAction::NextComment);
    assert_eq!(view(&state).last_comment, Some(a5.id));
    assert_eq!(selected(&state).as_deref(), Some("a.rs"));

    let third = key(&mut state, ReviewAction::NextComment);
    assert_eq!(
        selected(&state).as_deref(),
        Some("c.rs"),
        "the third selects c.rs"
    );
    assert_eq!(view(&state).last_comment, Some(c7.id));
    assert!(
        view(&state).drawer.to_show,
        "the jump waits for c.rs's diff"
    );
    let seq = diff_read(&third).expect("c.rs's diff is read");
    assert!(
        scroll_diff(&answer(&mut state, seq, two_hunks())).is_some(),
        "its card is scrolled to once the diff is shown"
    );
    assert!(
        !view(&state).drawer.state.open,
        "a card in the diff opens no drawer"
    );

    key(&mut state, ReviewAction::NextComment);
    assert_eq!(selected(&state).as_deref(), Some("a.rs"), "a fourth wraps");
    assert_eq!(view(&state).last_comment, Some(a3.id));
    key(&mut state, ReviewAction::PrevComment);
    assert_eq!(view(&state).last_comment, Some(c7.id), "`[` wraps back");
    assert_eq!(selected(&state).as_deref(), Some("c.rs"));
}

/// US3 s3, FR-006: a comment not in the diff (outdated) selects its file and opens the drawer on
/// it; one whose file the list does not show opens the drawer without selecting.
#[test]
fn a_comment_not_in_the_diff_opens_the_drawer_on_it() {
    let all = ["a.rs", "b.rs"];
    let mut state = ready(&all);
    let (gone, unlisted) = (draft("b.rs", 4, 1), draft("z.rs", 1, 2));
    marked(&mut state, &[], &all, vec![gone.clone(), unlisted.clone()]);
    shown_with(&mut state, "a.rs", two_hunks());
    state.tabs.shown_view_mut().expect("shown").anchors.insert(
        gone.id,
        Anchor {
            shown: gone.range,
            outdated: true,
        },
    );

    key(&mut state, ReviewAction::NextComment);
    assert_eq!(
        selected(&state).as_deref(),
        Some("b.rs"),
        "its file is selected"
    );
    let drawer = &view(&state).drawer;
    assert!(drawer.state.open, "the drawer opens");
    assert_eq!(drawer.state.tab, DrawerTab::Drafts);
    assert_eq!(drawer.focus, Some(gone.id), "on that comment");
    assert!(!drawer.to_show, "no jump into the diff");

    key(&mut state, ReviewAction::NextComment);
    assert_eq!(view(&state).last_comment, Some(unlisted.id));
    assert_eq!(
        selected(&state).as_deref(),
        Some("b.rs"),
        "an unlisted file is not selected"
    );
    assert_eq!(view(&state).drawer.focus, Some(unlisted.id));
}

/// FR-006: no comments, nothing changes.
#[test]
fn close_bracket_without_comments_does_nothing() {
    let mut state = ready(&["a.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    let before = state.clone();
    assert_eq!(
        key(&mut state, ReviewAction::NextComment),
        vec![Effect::None]
    );
    assert_eq!(state, before);
}

/// US3 s4, FR-009: with picked lines `c` opens **Add comment**'s composer on them, with the
/// keyboard in it.
#[test]
fn c_opens_the_composer_on_the_picked_lines() {
    let mut state = ready(&["a.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    changes::update(
        &mut state,
        Msg::GutterPressed {
            old: None,
            new: Some(10),
            extend: false,
        },
    );
    let mut pressed = state.clone();
    changes::update(&mut pressed, Msg::AddComment);
    let effect = key(&mut state, ReviewAction::Comment);
    assert!(effect.contains(&Effect::FocusComposer), "{effect:?}");
    assert_eq!(view(&state).composer, view(&pressed).composer);
    assert!(view(&state).composer.is_some());
}

/// US3 s5, FR-009: with no picked lines `c` picks the current hunk's changed lines and opens the
/// composer on them.
#[test]
fn c_opens_the_composer_on_the_current_hunks_changed_lines() {
    let mut state = ready(&["a.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    key(&mut state, ReviewAction::NextHunk);
    key(&mut state, ReviewAction::NextHunk);
    let effect = key(&mut state, ReviewAction::Comment);
    assert!(effect.contains(&Effect::FocusComposer), "{effect:?}");
    let pick = Pick {
        side: Side::New,
        anchor: 10,
        head: 10,
    };
    assert_eq!(view(&state).pick, Some(pick));
    assert_eq!(
        view(&state).composer.as_ref().map(|c| c.target),
        Some(ComposerTarget::New(pick))
    );
}

/// US3 s6, FR-009: with neither, nothing opens and the note says what to do; with a composer
/// already open, `c` gives it the keyboard and keeps its text.
#[test]
fn c_with_nothing_to_comment_on_says_so_and_an_open_composer_is_focused() {
    let mut state = ready(&["a.rs"]);
    shown_with(&mut state, "a.rs", two_hunks());
    assert_eq!(key(&mut state, ReviewAction::Comment), vec![Effect::None]);
    assert_eq!(view(&state).notice, Some(PICK_OR_HUNK));
    assert!(view(&state).composer.is_none());

    changes::update(
        &mut state,
        Msg::GutterPressed {
            old: None,
            new: Some(1),
            extend: false,
        },
    );
    changes::update(&mut state, Msg::AddComment);
    changes::update(&mut state, Msg::ComposerEdited("half a thought".into()));
    let before = view(&state).composer.clone();
    assert_eq!(
        key(&mut state, ReviewAction::Comment),
        vec![Effect::FocusComposer]
    );
    assert_eq!(view(&state).composer, before, "its text is kept");
}

// --- T038: the send chord ---------------------------------------------------------------------------

/// The send chord as the platform sends it: primary + Shift + Enter.
fn chord() -> Press {
    Press {
        key: PressKey::Enter,
        primary: true,
        ctrl: !cfg!(target_os = "macos"),
        logo: cfg!(target_os = "macos"),
        shift: true,
        ..press('x')
    }
}

/// US4 s1, SC-006: the chord under the review has exactly **Send review (n)**'s effect.
#[test]
fn the_send_chord_sends_as_send_review_does() {
    let all = ["a.rs"];
    let mut app = app_with(&all);
    marked(
        &mut app.changes,
        &[],
        &all,
        vec![draft("a.rs", 1, 1), draft("a.rs", 10, 2)],
    );
    assert_eq!(
        key_pressed(&app, &chord()),
        Some(Msg::ReviewKey(ReviewAction::Send))
    );
    let pressed = changes::update(&mut app.changes.clone(), Msg::SendPressed);
    assert!(
        matches!(pressed, Effect::ReviewSend { count: 2, .. }),
        "{pressed:?}"
    );
    assert_eq!(app.update_changes(Msg::KeyPressed(chord())), pressed);
}

/// US4 s2–s3: no drafts, or a send under way, and the chord does nothing.
#[test]
fn the_send_chord_does_nothing_without_drafts_or_while_sending() {
    let all = ["a.rs"];
    let mut state = ready(&all);
    assert_eq!(key(&mut state, ReviewAction::Send), vec![Effect::None]);
    changes::update(
        &mut state,
        Msg::ReviewChanged {
            project: project(),
            worktree_dir: "feat-a".into(),
            comments: vec![draft("a.rs", 1, 1)],
            sending: true,
            marks: Vec::new(),
        },
    );
    assert_eq!(key(&mut state, ReviewAction::Send), vec![Effect::None]);
}

/// US4 s4, SC-006: in the comment box the chord sends only the saved drafts, as **Send review
/// (n)** does, and the box stays open, focused, with its text; other presses stay the box's.
#[test]
fn the_send_chord_in_the_comment_box_sends_the_saved_drafts_and_keeps_the_box() {
    let all = ["a.rs"];
    let mut app = app_with(&all);
    marked(&mut app.changes, &[], &all, vec![draft("a.rs", 10, 1)]);
    shown_with(&mut app.changes, "a.rs", two_hunks());
    changes::update(
        &mut app.changes,
        Msg::GutterPressed {
            old: None,
            new: Some(1),
            extend: false,
        },
    );
    changes::update(&mut app.changes, Msg::AddComment);
    changes::update(&mut app.changes, Msg::ComposerEdited("j unsaved".into()));
    app.window.focused_field = Some(FieldId::ReviewComposer);
    assert_eq!(review_holder(&app), Holder::Composer);

    for other in [
        press('j'),
        Press {
            primary: true,
            ctrl: !cfg!(target_os = "macos"),
            logo: cfg!(target_os = "macos"),
            key: PressKey::Enter,
            ..press('x')
        },
    ] {
        assert_eq!(
            composer_press(&other, true),
            None,
            "{other:?} stays the box's"
        );
    }
    let msg = composer_press(&chord(), true).expect("the chord is the review's");
    let composer = view(&app.changes).composer.clone();
    let pressed = changes::update(&mut app.changes.clone(), Msg::SendPressed);
    let effect = app.update_changes(msg);
    assert!(
        matches!(effect, Effect::ReviewSend { count: 1, .. }),
        "{effect:?}"
    );
    assert_eq!(effect, pressed);
    assert_eq!(view(&app.changes).composer, composer, "open, with its text");
    assert_eq!(
        app.window.focused_field,
        Some(FieldId::ReviewComposer),
        "and the keyboard"
    );
}

/// SC-001: a 30-file review opened and marked with `j` and `v`, then one chord sends it.
#[test]
fn a_thirty_file_review_walked_by_key_is_sent_by_one_chord() {
    let names: Vec<String> = (0..30).map(|i| format!("f{i:02}.rs")).collect();
    let all: Vec<&str> = names.iter().map(String::as_str).collect();
    let mut app = app_with(&all);
    let notes = vec![draft("f00.rs", 1, 1)];
    marked(&mut app.changes, &[], &all, notes.clone());
    let mut done: Vec<&str> = Vec::new();
    for _ in 0..30 {
        app.update_changes(Msg::KeyPressed(press('j')));
        let edit = app.update_changes(Msg::KeyPressed(press('v')));
        let Effect::ReviewEdit {
            edit: ReviewEditOp::SetReviewed { path: p, .. },
            ..
        } = edit
        else {
            panic!("a mark edit, got {edit:?}");
        };
        done.push(all.iter().find(|q| **q == p.as_str()).expect("listed"));
        marked(&mut app.changes, &done, &all, notes.clone());
    }
    let sent = app.update_changes(Msg::KeyPressed(chord()));
    assert!(
        matches!(sent, Effect::ReviewSend { count: 1, .. }),
        "{sent:?}"
    );
}

/// FR-022: with the single-key switch off the chord still sends, under the review and in the box,
/// and **Send review (n)**'s tooltip still names it.
#[test]
fn with_the_switch_off_the_send_chord_still_sends_and_is_named() {
    let all = ["a.rs"];
    let mut app = app_with(&all);
    marked(&mut app.changes, &[], &all, vec![draft("a.rs", 1, 1)]);
    app.settings.review_single_keys = false;
    assert!(matches!(
        app.update_changes(Msg::KeyPressed(chord())),
        Effect::ReviewSend { count: 1, .. }
    ));
    assert_eq!(composer_press(&chord(), false), Some(Msg::SendPressed));
    let chord_label = if cfg!(target_os = "macos") {
        "Cmd+Shift+Enter"
    } else {
        "Ctrl+Shift+Enter"
    };
    for on in [true, false] {
        assert_eq!(
            with_key_hint(SEND_TOOLTIP, ReviewAction::Send, on),
            format!("{SEND_TOOLTIP} ({chord_label})")
        );
    }
}

// --- T041: the shortcut sheet -----------------------------------------------------------------------

fn sheet_open(app: &App) -> bool {
    view(&app.changes).sheet_open
}

/// US5 s1–s3, FR-012: `?` opens the sheet, which then has the keyboard: `j` changes nothing and `?`
/// or Escape closes it, giving the keyboard back to the review.
#[test]
fn question_mark_opens_the_sheet_which_holds_the_keyboard_until_closed() {
    let mut app = app_with(&["a.rs", "b.rs"]);
    app.update_changes(Msg::KeyPressed(press('?')));
    assert!(sheet_open(&app));
    let top = micold_client::overlay::registry::topmost(&app).expect("on top");
    assert_eq!(top.id().as_str(), "review_shortcut_sheet");
    assert_eq!(review_holder(&app), Holder::Sheet);
    let before = app.changes.clone();
    assert_eq!(
        app.update_changes(Msg::KeyPressed(press('j'))),
        Effect::None
    );
    assert_eq!(app.changes, before, "`j` does not move the selection");
    let model = shortcut_sheet(&app).expect("drawn while open");
    assert_eq!(
        model.rows.len(),
        14,
        "every row of the table, then Back and Forward"
    );
    assert_eq!(model.note, None);

    app.update_changes(Msg::KeyPressed(press('?')));
    assert!(!sheet_open(&app), "`?` closes it");
    assert_eq!(review_holder(&app), Holder::Review);

    app.update_changes(Msg::KeyPressed(press('?')));
    let escape = micold_client::app::on_escape(&app).expect("Escape closes it");
    assert_eq!(escape, Message::Changes(Msg::ShortcutsClosed));
    app.update(escape);
    assert!(!sheet_open(&app));
    assert_eq!(review_holder(&app), Holder::Review);
}

/// Back and Forward as the sheet ends with them (FR-014, contracts/ui.md U6).
fn history_rows() -> [(String, &'static str); 2] {
    use micold_client::keymap::history_key_label;
    use micold_core::history::Dir;
    [
        (
            history_key_label(Dir::Back),
            "Back (also the mouse back button)",
        ),
        (
            history_key_label(Dir::Forward),
            "Forward (also the mouse forward button)",
        ),
    ]
}

/// US5 s1–s2, FR-014: the sheet ends with Back and Forward, with the single keys on and off.
#[test]
fn the_sheet_ends_with_back_and_forward_with_single_keys_on_and_off() {
    for single_keys in [true, false] {
        let rows = changes::sheet_model(single_keys).rows;
        let tail = &rows[rows.len() - 2..];
        assert_eq!(tail, history_rows(), "single keys {single_keys}");
    }
}

/// US5 s4: the overflow menu's **Keyboard shortcuts** opens the same sheet, closing the menu.
#[test]
fn the_overflow_item_opens_the_same_sheet() {
    let mut app = app_with(&["a.rs"]);
    app.update_changes(Msg::OverflowToggled);
    app.update_changes(Msg::ShortcutsOpened);
    assert!(sheet_open(&app));
    assert!(!view(&app.changes).overflow_open);
    assert_eq!(review_holder(&app), Holder::Sheet);
}

/// US5 s6, FR-022: with the switch off `?` does nothing, the overflow item still opens the sheet,
/// which lists only the chords and the note, and Escape still closes it.
#[test]
fn with_the_switch_off_the_sheet_lists_only_the_chords() {
    let mut app = app_with(&["a.rs"]);
    app.settings.review_single_keys = false;
    assert_eq!(
        app.update_changes(Msg::KeyPressed(press('?'))),
        Effect::None
    );
    assert!(!sheet_open(&app), "`?` does nothing");
    app.update_changes(Msg::ShortcutsOpened);
    let model = shortcut_sheet(&app).expect("open");
    let mac = cfg!(target_os = "macos");
    let mut want: Vec<(String, &str)> = sheet_rows(false)
        .map(|row| {
            let (other, macos) = row.labels();
            (if mac { macos } else { other }, row.sheet)
        })
        .collect();
    want.extend(history_rows());
    assert_eq!(model.rows, want);
    assert_eq!(
        model.rows.len(),
        4,
        "the send and save-comment chords, Back and Forward"
    );
    assert_eq!(model.note, Some(SINGLE_KEYS_OFF_NOTE));
    assert_eq!(
        app.update_changes(Msg::KeyPressed(press('?'))),
        Effect::None
    );
    assert!(sheet_open(&app), "nor does `?` close it");
    let escape = micold_client::app::on_escape(&app).expect("Escape closes it");
    app.update(escape);
    assert!(!sheet_open(&app));
}
