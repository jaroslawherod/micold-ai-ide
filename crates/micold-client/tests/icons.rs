//! Foundational icon-vocabulary invariants (FR-002, FR-003, SC-006). Pure — no iced, runs
//! under `cargo test --no-default-features`. The codepoints are pinned from the shipped font
//! (assets/fonts/PROVENANCE.md); this test regression-locks the name→glyph mapping so a font
//! swap that moves a glyph is caught.

use micold_client::icons::Icon;

/// The pinned codepoint for every icon (from assets/fonts/PROVENANCE.md).
fn expected(icon: Icon) -> char {
    match icon {
        Icon::Help => '\u{e8fd}',
        Icon::About => '\u{e88e}',
        Icon::OpenProject => '\u{e2c8}',
        Icon::Rename => '\u{f097}',
        Icon::Git => '\u{eaf5}',
        Icon::ActiveMarker => '\u{f0be}',
        Icon::Unavailable => '\u{f8b6}',
        Icon::NavigateUp => '\u{e5d8}',
        Icon::AddSession => '\u{e145}',
        Icon::AddWorktree => '\u{e97a}',
        Icon::Menu => '\u{e5d4}',
        Icon::LightMode => '\u{e518}',
        Icon::DarkMode => '\u{e51c}',
        Icon::AutoMode => '\u{e1ab}',
        Icon::HideSidebar => '\u{f717}',
        Icon::ShowSidebar => '\u{f716}',
        Icon::Settings => '\u{e8b8}',
        Icon::Delete => '\u{e872}',
        Icon::Copy => '\u{e14d}',
        Icon::Filter => '\u{e152}',
        Icon::Search => '\u{e8b6}',
        Icon::AiCli => '\u{e65f}',
        Icon::RegularTerminal => '\u{eb8e}',
        Icon::ReleaseFocus => '\u{e31a}',
        Icon::ProjectRoot => '\u{e88a}',
        Icon::AddTerminalInstance => '\u{e146}',
        Icon::SplitVertical => '\u{e949}',
        Icon::SplitHorizontal => '\u{e947}',
        Icon::Close => '\u{e5cd}',
        Icon::ActivityWorking => '\u{e837}',
        Icon::ActivityEnded => '\u{e836}',
        Icon::SelectChevron => '\u{e5cf}',
        Icon::Daemons => '\u{eb2f}',
        Icon::Refresh => '\u{e5d5}',
        Icon::IssueMapping => '\u{e892}',
        Icon::JiraIssues => '\u{e85d}',
        Icon::MoveUp => '\u{e316}',
        Icon::MoveDown => '\u{e313}',
        Icon::NeedsPermission => '\u{e925}',
        Icon::SessionError => '\u{e000}',
        Icon::LongTaskFinished => '\u{e2e6}',
        Icon::TurnFinished => '\u{e0cb}',
        Icon::PrOpen => '\u{e0b6}',
        Icon::PrDraft => '\u{e745}',
        Icon::PrMerged => '\u{e0b3}',
        Icon::PrClosed => '\u{e14b}',
        Icon::ChecksPassing => '\u{e5ca}',
        Icon::ChecksPending => '\u{e8b5}',
        Icon::ChecksFailing => '\u{e5c9}',
        Icon::OpenInBrowser => '\u{e89e}',
        Icon::PlanUsage => '\u{e1af}',
        Icon::UsageWarning => '\u{e002}',
        Icon::StateRunning => '\u{e037}',
        Icon::StateBlocked => '\u{f052}',
        Icon::StateIdle => '\u{e034}',
        Icon::Overview => '\u{e871}',
        Icon::CardNeedsAction => '\u{e645}',
        Icon::CardNeedsAttention => '\u{e8f4}',
        Icon::CardDone => '\u{e86c}',
        Icon::TypeFeature => '\u{e0f0}',
        Icon::TypeBugfix => '\u{e868}',
        Icon::TypeChore => '\u{e869}',
        Icon::TypeOther => '\u{e2c7}',
        Icon::TypeIncluded => '\u{e157}',
        Icon::TypeOrphan => '\u{eb83}',
        Icon::Open => '\u{ea77}',
        Icon::ChevronLeft => '\u{e5cb}',
        Icon::ChevronRight => '\u{e5cc}',
        Icon::FullScreen => '\u{e5d0}',
        Icon::FullScreenExit => '\u{e5d1}',
        Icon::Thinking => '\u{ea4a}',
        Icon::ToolCall => '\u{f8cd}',
        Icon::ToolRunning => '\u{e9d0}',
        Icon::ToolDone => '\u{e92d}',
        Icon::ToolFailed => '\u{e001}',
        Icon::ToolNoResult => '\u{f08f}',
        Icon::JumpToLatest => '\u{e5db}',
        Icon::Subagent => '\u{f06c}',
        Icon::BackgroundTask => '\u{f1bb}',
        Icon::TaskDone => '\u{e834}',
        Icon::TaskOpen => '\u{e835}',
        Icon::MoreHorizontal => '\u{e5d3}',
    }
}

#[test]
fn glyph_maps_every_variant_to_its_pinned_codepoint() {
    for &icon in Icon::ALL {
        assert_eq!(
            icon.glyph(),
            expected(icon),
            "{icon:?} must map to its pinned codepoint"
        );
    }
}

#[test]
fn all_covers_every_variant_without_duplicates() {
    // Curated icon set (research R5 / PROVENANCE.md); +1 for Settings (feature 006),
    // +1 for Delete (feature 008), +1 for Copy (cross-app clipboard), +1 for Filter
    // (feature 009), +3 for AiCli/RegularTerminal/ReleaseFocus, +1 for ProjectRoot (T012)
    // (all feature 010), +1 for Close (session row's trailing action, shared with feature 011's
    // terminal-instance close action rather than double-counted), +1 for AddTerminalInstance
    // (feature 011), +2 for ActivityWorking/ActivityEnded (feature 010's session activity dots,
    // moved into the vocabulary by BUG-004), +1 for Search (feature 021's type-ahead field —
    // a magnifier, distinct from Filter's funnel), +1 for SelectChevron (feature 022's select
    // trailing affordance — `expand_more`, the glyph §7.7 gives the control and the one thing that
    // tells a select from a text field at a glance), +1 for Refresh (feature 029's sidebar-header
    // control — Material's `refresh`, chosen over `sync` and `autorenew` because those two say
    // "there is a remote" and "this happens by itself", and neither is true here), +3 for
    // IssueMapping/MoveUp/MoveDown (feature 034's GitHub issues settings section — `label`,
    // `keyboard_arrow_up`, `keyboard_arrow_down`; `arrow_upward` is already `NavigateUp`), +4 for the notification kinds' icons (feature 613, I1 —
    // `pan_tool`, `error`, `task_alt`, `chat_bubble`), +7 for feature 040's pull request indicator (`call_split`, `edit_note`, `call_merge`,
    // `block`, `check`, `schedule`, `cancel`; `edit` is `Rename`'s and `close` is `Close`'s), +1 for `OpenInBrowser`
    // (feature 040's **Open pull request** menu entry — `open_in_new`), +2 for feature 484's pane
    // split buttons (`vertical_split`, `horizontal_split`), +1 for feature 490's plan-usage
    // indicator (`data_usage`) and +1 for its warning look (`warning`), +1 for feature 533's Jira
    // settings section (`assignment`; no other icon shares its codepoint), -1 for the Session service
    // section icon (`dns`) removed with the section (feature 804), +3 for feature 905's worktree
    // state icon (`play_arrow`, `report`, `pause`), +10 for feature 813's Overview (`dashboard`), its
    // card states (`priority_high`, `visibility`, `check_circle`) and location types (`lightbulb`,
    // `bug_report`, `build`, `folder`, `link`, `folder_off`), +1 for feature 972's card menu *Open*
    // entry (`login`), +2 for feature 1003's pull request header's previous/next (`chevron_left`,
    // `chevron_right`), +2 for feature 970's Overview full-screen toggle (`fullscreen`,
    // `fullscreen_exit`), +7 for feature 939's transcript cards (`psychology`, `build`,
    // `progress_activity`, `check_circle`, `error`, `do_not_disturb_on`, `arrow_downward`), +2 for
    // feature 944's subagent and background-task icons (`smart_toy`, `pending_actions`), +2 for
    // feature 940's markdown task-list marks (`check_box`, `check_box_outline_blank`), +1 for
    // feature 1125's sidebar more menu (`more_horiz`; `more_vert` is `Menu`'s).
    assert_eq!(Icon::ALL.len(), 82, "curated set size");

    // No duplicate variants.
    for (i, &a) in Icon::ALL.iter().enumerate() {
        for &b in &Icon::ALL[i + 1..] {
            assert_ne!(a, b, "Icon::ALL must not contain duplicates");
        }
    }

    // No duplicate glyphs (two icons must never collide on one codepoint).
    for (i, &a) in Icon::ALL.iter().enumerate() {
        for &b in &Icon::ALL[i + 1..] {
            assert_ne!(
                a.glyph(),
                b.glyph(),
                "{a:?} and {b:?} must not share a codepoint"
            );
        }
    }
}

/// Feature 613, I1: the four notification-kind icons are in the vocabulary, on the Material Symbols
/// codepoints for `pan_tool`, `error`, `task_alt` and `chat_bubble`, and are four different glyphs.
#[test]
fn the_four_notification_kind_icons_are_in_the_vocabulary_and_distinct() {
    let kinds = [
        (Icon::NeedsPermission, '\u{e925}'),
        (Icon::SessionError, '\u{e000}'),
        (Icon::LongTaskFinished, '\u{e2e6}'),
        (Icon::TurnFinished, '\u{e0cb}'),
    ];
    for (icon, glyph) in kinds {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert_eq!(icon.glyph(), glyph, "{icon:?} codepoint");
    }
    for (i, (a, _)) in kinds.iter().enumerate() {
        for (b, _) in &kinds[i + 1..] {
            assert_ne!(a.glyph(), b.glyph(), "{a:?} and {b:?} must differ");
        }
    }
}

/// Feature 905 (FR-007, FR-008, contract A5): the three worktree-state glyphs are `play_arrow`,
/// `report` and `pause` (names checked against the upstream `.codepoints` manifest), and none of
/// them is a glyph another mark on the same row already draws: the Default entry's house, the pull
/// request indicator's seven, or the session activity dots.
#[test]
fn the_three_state_icons_are_pinned_and_share_no_glyph_with_a_rows_other_marks() {
    let states = [
        (Icon::StateRunning, '\u{e037}'),
        (Icon::StateBlocked, '\u{f052}'),
        (Icon::StateIdle, '\u{e034}'),
    ];
    let row_marks = [
        Icon::ProjectRoot,
        Icon::PrOpen,
        Icon::PrDraft,
        Icon::PrMerged,
        Icon::PrClosed,
        Icon::ChecksPassing,
        Icon::ChecksPending,
        Icon::ChecksFailing,
        Icon::ActivityWorking,
        Icon::ActivityEnded,
    ];
    for (icon, glyph) in states {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert_eq!(icon.glyph(), glyph, "{icon:?} codepoint");
        for other in row_marks {
            assert_ne!(
                icon.glyph(),
                other.glyph(),
                "{icon:?} must not share a glyph with {other:?}"
            );
        }
    }
}

/// Feature 813 (research R11): the Overview's ten icons are pinned, in the vocabulary, and share no
/// glyph with any other icon, so a card's type and state are told apart by shape.
#[test]
fn the_overview_icons_are_pinned_and_distinct_from_every_other_icon() {
    let overview = [
        (Icon::Overview, '\u{e871}'),
        (Icon::CardNeedsAction, '\u{e645}'),
        (Icon::CardNeedsAttention, '\u{e8f4}'),
        (Icon::CardDone, '\u{e86c}'),
        (Icon::TypeFeature, '\u{e0f0}'),
        (Icon::TypeBugfix, '\u{e868}'),
        (Icon::TypeChore, '\u{e869}'),
        (Icon::TypeOther, '\u{e2c7}'),
        (Icon::TypeIncluded, '\u{e157}'),
        (Icon::TypeOrphan, '\u{eb83}'),
        // Feature 970 (FR-012): the header's full-screen toggle, one glyph per state.
        (Icon::FullScreen, '\u{e5d0}'),
        (Icon::FullScreenExit, '\u{e5d1}'),
    ];
    for (icon, glyph) in overview {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert_eq!(icon.glyph(), glyph, "{icon:?} codepoint");
        for &other in Icon::ALL.iter().filter(|&&other| other != icon) {
            assert_ne!(
                icon.glyph(),
                other.glyph(),
                "{icon:?} must not share a glyph with {other:?}"
            );
        }
    }
}

/// Feature 939 (research R11): the transcript's seven icons are pinned and distinct from each
/// other. `build`, `check_circle` and `error` are already `TypeChore`'s, `CardDone`'s and
/// `SessionError`'s codepoints, so these three take the font's alias codepoints for the same glyph
/// (`U+F8CD`, `U+E92D` `check_circle_outline`, `U+E001` `error_outline`), as `ActiveMarker` and
/// `Unavailable` do: one glyph, one meaning on screen, and no two variants on one codepoint.
#[test]
fn the_transcript_icons_are_pinned_and_distinct_from_each_other() {
    let transcript = [
        (Icon::Thinking, '\u{ea4a}'),
        (Icon::ToolCall, '\u{f8cd}'),
        (Icon::ToolRunning, '\u{e9d0}'),
        (Icon::ToolDone, '\u{e92d}'),
        (Icon::ToolFailed, '\u{e001}'),
        (Icon::ToolNoResult, '\u{f08f}'),
        (Icon::JumpToLatest, '\u{e5db}'),
    ];
    for (icon, glyph) in transcript {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert_eq!(icon.glyph(), glyph, "{icon:?} codepoint");
    }
    for (i, (a, _)) in transcript.iter().enumerate() {
        for (b, _) in &transcript[i + 1..] {
            assert_ne!(a.glyph(), b.glyph(), "{a:?} and {b:?} must differ");
        }
    }
}

/// Feature 944 (research R12): the subagent and background-task icons are pinned, in `Icon::ALL`,
/// and share a codepoint with no other icon.
#[test]
fn the_subagent_and_background_task_icons_are_pinned_and_unique() {
    for (icon, glyph) in [
        (Icon::Subagent, '\u{f06c}'),
        (Icon::BackgroundTask, '\u{f1bb}'),
    ] {
        assert!(Icon::ALL.contains(&icon), "{icon:?} must be in Icon::ALL");
        assert_eq!(icon.glyph(), glyph, "{icon:?} codepoint");
        for &other in Icon::ALL.iter().filter(|&&other| other != icon) {
            assert_ne!(
                icon.glyph(),
                other.glyph(),
                "{icon:?} and {other:?} share a codepoint"
            );
        }
    }
}
