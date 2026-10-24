//! Structural gates that keep the session area's presses landing (feature 1126 S8, FR-011; carried
//! over from feature 023's `terminal_bar_stability.rs`, whose bar this area replaced).
//!
//! # Why a source-level test at all
//!
//! These guard preconditions rather than behaviour, and none is reachable from an ordinary test.
//! `iced_widget::button` publishes `on_press` on `ButtonReleased`, and only if the `is_pressed` flag
//! stored in its node of the widget tree is set. `Tree::diff_children` zips old and new children
//! *by position*. So a strip or header child that is pushed only while the terminal is focused, or
//! only for some shape of session, shifts every sibling after it the moment that changes — and a
//! press that changes focus does exactly that between its own press and release. The pressed
//! control is then diffed against its neighbour's node, `is_pressed` is gone, and the click
//! vanishes. The user presses again, it works, and it reads as a slow application rather than a bug.
//!
//! The strip's *members* do come and go — a shell tab per instance — and that is what the keyed row
//! is for (R10; `ui/cdk/keyed.rs`'s slot test): a member's slot follows its key. What these gates
//! hold is the rest: the strip's and the header's own rows keep a child list that never varies.

#[path = "support/source_scan.rs"]
mod source_scan;
#[path = "support/mod.rs"]
mod support;

use source_scan::{read_rs_under, strip_comments};

use std::fs;
use std::path::{Path, PathBuf};

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `src/`, as `(path relative to src/, code with comments stripped)`.
fn crate_sources() -> Vec<(String, String)> {
    let root = src_dir();
    let out: Vec<(String, String)> = read_rs_under(std::slice::from_ref(&root), &root)
        .into_iter()
        .map(|(name, src)| (name, strip_comments(&src)))
        .collect();
    assert!(!out.is_empty(), "found no sources under {}", root.display());
    out
}

/// A file under `src/`, comments stripped.
fn source(rel: &str) -> String {
    let path = src_dir().join(rel);
    strip_comments(&fs::read_to_string(&path).unwrap_or_else(|e| panic!("{rel}: {e}")))
}

/// The fewest bytes an extracted item may have: shorter means the extraction stopped early, and
/// every `!contains` guard over it would pass without having looked at the item.
const MIN_BODY: usize = 150;

/// A top-level item's text, from `marker` to the brace that closes the item's own block, which
/// must hold every one of `expected` and be at least [`MIN_BODY`] bytes.
fn body_from(src: &str, marker: &str, expected: &[&str]) -> String {
    let start = src
        .find(marker)
        .unwrap_or_else(|| panic!("the source must contain `{marker}`"));
    let rest = &src[start..];
    let open = rest
        .find('{')
        .unwrap_or_else(|| panic!("`{marker}` must open a block"));
    let mut depth = 0usize;
    let mut end = None;
    for (i, c) in rest[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(open + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    let body = rest[..end.unwrap_or_else(|| panic!("`{marker}`'s block must close"))].to_string();
    assert!(
        body.len() >= MIN_BODY,
        "`{marker}` extracted only {} bytes, so a guard over it would pass vacuously: {body}",
        body.len()
    );
    for token in expected {
        assert!(
            body.contains(token),
            "`{marker}`'s extracted body lacks `{token}`, so it is not the item the guard means"
        );
    }
    body
}

/// The strip's and the header's builders, and the area's own view (S8).
fn builders() -> Vec<(&'static str, String)> {
    let area = source("ui/session_area.rs");
    vec![
        (
            "session_area::view",
            body_from(&area, "pub fn view<'a>(", &["column![", "terminal::pane("]),
        ),
        (
            "session_area::strip",
            body_from(
                &area,
                "pub(crate) fn strip<'a>(",
                &["let strip = row![", "Keyed::row("],
            ),
        ),
        (
            "session_area::status_cluster",
            body_from(
                &area,
                "fn status_cluster<'a>(",
                &["model.restart", ".on_press(restart)"],
            ),
        ),
        (
            "session_area::member",
            body_from(&area, "fn member<'a>(", &["Member::Plus", "Member::Shell"]),
        ),
        (
            "session_area::header",
            body_from(&area, "fn header<'a>(", &["changes_button(", "chips"]),
        ),
    ]
}

/// Adding or removing a child from a row.
const PUSHES_A_CHILD: &[&str] = &[".push(", "row!", "column!"];

/// `if` guards in `body` whose line or the two after it push a child, when the guard's line asks
/// one of `questions`.
fn guarded_pushes(name: &str, body: &str, questions: &[&str]) -> Vec<String> {
    let lines: Vec<&str> = body.lines().collect();
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let is_guard = line.contains("if ") && questions.iter().any(|q| line.contains(q));
        if !is_guard {
            continue;
        }
        let window = lines[i..lines.len().min(i + 3)].join(" ");
        if PUSHES_A_CHILD.iter().any(|p| window.contains(p)) {
            found.push(format!("{name}: {}", line.trim()));
        }
    }
    found
}

/// S8, FR-011 (feature 023 FR-008a): the strip and the header read no focus at all — neither to
/// add a child nor for anything else; where focus ends is the reducers' business.
#[test]
fn the_strip_and_the_header_read_no_focus() {
    let found: Vec<String> = builders()
        .into_iter()
        .filter(|(_, body)| body.contains("terminal_focused"))
        .map(|(name, _)| name.to_string())
        .collect();
    assert!(
        found.is_empty(),
        "the session area's strip and header must not read focus (S8). A focus-conditional child \
         shifts every sibling after it and iced's positional tree diff drops the press: {found:?}"
    );
}

/// S8, feature 026 FR-003: no child of the strip's or the header's own rows is pushed under an `if`
/// on the session's shape — how many instances or members there are, or whether the session has
/// any. The members themselves come and go inside the keyed row, which keeps each one's slot.
#[test]
fn no_child_is_pushed_under_an_if_on_session_shape() {
    const SHAPE_QUESTION: &[&str] = &[
        "shells.len()",
        "shells.is_empty()",
        "members.len()",
        "members.is_empty()",
        "active_shell",
        "mode ==",
    ];
    let found: Vec<String> = builders()
        .iter()
        .flat_map(|(name, body)| guarded_pushes(name, body, SHAPE_QUESTION))
        .collect();
    assert!(
        found.is_empty(),
        "a strip or header child pushed under an `if` on the session's shape renumbers every \
         sibling after it under a press in flight (feature 023 FR-008a, FR-011):\n{}",
        found.join("\n")
    );
    let strip = &builders()[1].1;
    assert!(
        strip.contains("let strip = row![") && !strip.contains("strip.push("),
        "the strip's own row is one `row!` with a fixed child list (S7)"
    );
}

/// A1, SC-005: nothing is drawn under the terminal. The geometry gate cannot see a bar that comes
/// back *inside* `terminal::pane` (the pane area fills the session area either way), so the pane's
/// builder is held to drawing no bar: no row of controls, no toolbar surface, no split buttons, and
/// its last act is to wrap the body it built, not to stack anything under it. (`column!` and
/// `container(` themselves stay allowed: the pane stacks the switch strip and a refusal *above* the
/// body and centres the empty state.)
#[test]
fn the_pane_area_draws_no_bar_under_the_terminal() {
    let src = source("ui/terminal.rs");
    let pane = body_from(
        &src,
        "pub fn pane<'a>(",
        &["GridSizeReporter", "terminal_switch_strip("],
    );
    for bar_part in [
        "row![",
        "Row::",
        "Surface::new",
        "SurfaceKind::Toolbar",
        "split_buttons",
        "restart",
    ] {
        assert!(
            !pane.contains(bar_part),
            "`terminal::pane` builds `{bar_part}`: the bar under the terminal is back (A1, \
             SC-005); its controls belong to the strip above the panes"
        );
    }
    let tail = pane.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        tail.ends_with("container(body) .height(Length::Fill) .width(Length::Fill) .into() }"),
        "`terminal::pane` must end by wrapping the body it built and nothing else, so nothing is \
         laid out under the terminal (A1): {tail}"
    );
}

/// Feature 023 FR-021b, carried to the strip: no release-focus control. The reserved
/// Ctrl+Shift+E / Cmd+Shift+E chord is the only explicit release.
#[test]
fn the_session_area_has_no_release_focus_control() {
    for (name, body) in builders() {
        assert!(
            !body.contains("Icon::ReleaseFocus"),
            "{name} carries a release-focus control (feature 023 FR-021b, BUG-001)"
        );
    }
    assert!(
        !source("ui/terminal.rs").contains("Icon::ReleaseFocus"),
        "nor the pane area"
    );
}

/// Feature 023 T004: `terminal_released` has exactly two writers.
const PERMITTED: &[&str] = &[
    "pub(crate) fn focus_terminal",
    "pub(crate) fn release_terminal",
    "pub terminal_released",
];

#[test]
fn no_scattered_release_writes() {
    let mut found = Vec::new();
    for (rel, src) in crate_sources() {
        let lines: Vec<&str> = src.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let assigns =
                line.contains("terminal_released =") || line.contains("terminal_released:");
            if !assigns {
                continue;
            }
            let start = i.saturating_sub(3);
            let window = lines[start..=i].join(" ");
            if PERMITTED.iter().any(|p| window.contains(p)) {
                continue;
            }
            found.push(format!("{rel}:{}  {}", i + 1, line.trim()));
        }
    }
    assert!(
        found.is_empty(),
        "`terminal_released` may be written only by `focus_terminal()` and `release_terminal()` \
         (feature 023). Call the helper that names the intent instead:\n{}",
        found.join("\n")
    );
}

/// Feature 026-multi-provider-sessions FR-016a, carried to the session tab (R8): its label names the
/// session's CLI by its **command** name (`command_caption`), never `display_name()`.
#[test]
fn a_session_tab_names_the_sessions_cli_by_command() {
    let rules = source("features/session_strip.rs");
    let tab = body_from(
        &rules,
        "fn session_member(",
        &["command_caption(", "Member::Session"],
    );
    assert!(
        tab.contains("command_caption("),
        "a session tab must label itself from the session's provider, by `command()` (FR-016a)"
    );
    assert!(
        !tab.contains("display_name()"),
        "`display_name()` is the menus-and-sentences register; a tab in a width budget takes \
         `command()`"
    );
}

// ---- T040: a press in flight follows its member (US4 scenario 3, FR-011, R10) ----

mod press_in_flight {
    use super::support::covered_states::{strip_with, SESSION_STRIP_TABS};
    use super::support::layout::{self as lay, StateUnderTest, WINDOW};
    use iced::advanced::widget::Tree;
    use iced::advanced::{clipboard, layout, mouse, Layout, Shell};
    use iced::{Event, Point, Rectangle, Size};
    use micold_client::app::{Message, State};
    use micold_client::features::session::Msg as SessionMsg;
    use micold_client::features::session_strip::{MemberKey, StripModel};
    use micold_core::pane_layout::PaneLayout;

    fn under(state: State) -> StateUnderTest {
        StateUnderTest::new(state).panes(PaneLayout::single())
    }

    /// The centre of member `key` in `under`'s laid-out strip.
    fn centre_of(node: &layout::Node, state: &State, key: MemberKey) -> Point {
        let model = StripModel::of(state).expect("a session is shown");
        let index = model
            .members
            .iter()
            .position(|m| m.key() == key)
            .expect("the member is in the strip");
        let path = [SESSION_STRIP_TABS, &[index]].concat();
        let record = lay::walk(Layout::new(node), lay::Layer::Base)
            .into_iter()
            .find(|r| r.path == path)
            .expect("the member is laid out");
        Point::new(
            record.x + record.width / 2.0,
            record.y + record.height / 2.0,
        )
    }

    fn dispatch(
        element: &mut iced::Element<'_, Message>,
        tree: &mut Tree,
        node: &layout::Node,
        renderer: &iced::Renderer,
        event: mouse::Event,
        at: Point,
        out: &mut Vec<Message>,
    ) {
        let mut shell = Shell::new(out);
        element.as_widget_mut().update(
            tree,
            &Event::Mouse(event),
            Layout::new(node),
            mouse::Cursor::Available(at),
            renderer,
            &mut clipboard::Null,
            &mut shell,
            &Rectangle::with_size(WINDOW),
        );
    }

    /// Press member `key` in `before`, let the window change to `after` between the press and its
    /// release, and release over the member where it now is: what the strip published.
    fn press_across(before: State, after: State, key: MemberKey) -> Vec<Message> {
        let renderer = lay::renderer();
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        let origin = std::time::Instant::now();
        let mut published = Vec::new();

        let first = under(before.clone());
        let mut element = lay::view_of(&first);
        let mut tree = Tree::new(element.as_widget());
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        lay::settle(
            &mut element,
            &mut tree,
            &node,
            &renderer,
            origin,
            0..lay::SETTLE_FRAMES,
            WINDOW,
        );
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let at = centre_of(&node, &before, key);
        dispatch(
            &mut element,
            &mut tree,
            &node,
            &renderer,
            mouse::Event::ButtonPressed(mouse::Button::Left),
            at,
            &mut published,
        );
        drop(element);

        let second = under(after.clone());
        let mut element = lay::view_of(&second);
        tree.diff(element.as_widget());
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let at = centre_of(&node, &after, key);
        dispatch(
            &mut element,
            &mut tree,
            &node,
            &renderer,
            mouse::Event::CursorMoved { position: at },
            at,
            &mut published,
        );
        dispatch(
            &mut element,
            &mut tree,
            &node,
            &renderer,
            mouse::Event::ButtonReleased(mouse::Button::Left),
            at,
            &mut published,
        );
        published
            .into_iter()
            .filter(|m| matches!(m, Message::Session(_)))
            .filter(|m| {
                !matches!(
                    m,
                    Message::Session(
                        SessionMsg::TabStripScrolled { .. }
                            | SessionMsg::TabStripViewportResized { .. }
                    )
                )
            })
            .collect()
    }

    fn ids(
        state: &State,
    ) -> (
        micold_core::session::SessionId,
        micold_core::session::SessionId,
    ) {
        let sessions = state.active_sessions();
        (sessions[0].id, sessions[1].id)
    }

    /// How many chips the laid-out header's chips row holds in `state`.
    fn header_chips(state: &State) -> usize {
        let renderer = lay::renderer();
        let under = under(state.clone());
        let mut element = lay::view_of(&under);
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, WINDOW);
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let row = [
            &super::support::covered_states::SESSION_HEADER_CHANGES[..5],
            &[0, 1],
        ]
        .concat();
        lay::walk(Layout::new(&node), lay::Layer::Base)
            .into_iter()
            .filter(|r| r.path.len() == row.len() + 1 && r.path.starts_with(&row))
            .count()
    }

    /// A shell instance opens before the pressed member: the helper's tab still gets the press.
    #[test]
    fn a_press_survives_a_shell_instance_opening_before_it() {
        let before = strip_with(1, 1).state;
        let (main, helper) = ids(&before);
        let mut after = before.clone();
        after
            .workspace
            .find_session_mut(main)
            .expect("main")
            .1
            .open_shell_instance();
        assert_eq!(
            press_across(before, after, MemberKey::Session(helper)),
            vec![Message::Session(SessionMsg::Selected(helper))],
            "the press in flight on the helper's tab must reach it, not its new neighbour"
        );
    }

    /// The pressed shell's process stops (its mark and the restart appear): the press still lands.
    #[test]
    fn a_press_survives_its_process_stopping() {
        let before = strip_with(2, 1).state;
        let (main, _) = ids(&before);
        let shell = before.active_sessions()[0].shells[0].id;
        let mut after = before.clone();
        let (_, session) = after.workspace.find_session_mut(main).expect("main");
        session.mark_shell_exited(shell);
        assert_eq!(
            press_across(before, after, MemberKey::Shell(main, shell)),
            vec![Message::Session(SessionMsg::ShellInstanceSelected(
                main, shell
            ))]
        );
    }

    /// A chip appears in the header above (the pull request): the press on the session tab lands.
    /// The header is not a sibling of the strip's keyed row, so this is a no-regression pin; the
    /// assertion that the header gained a chip is what keeps the scenario from being a no-op.
    #[test]
    fn a_press_survives_a_header_chip_appearing() {
        use micold_core::pull_request::{CheckStatus, PrState, PullRequestStatus, ReviewState};
        let before = strip_with(1, 1).state;
        let (main, _) = ids(&before);
        let mut after = before.clone();
        after.pr_status.enabled = true;
        after.pr_status.statuses.insert(
            "feat/short".into(),
            PullRequestStatus {
                number: 7,
                title: "A pull request".into(),
                url: "https://github.com/o/r/pull/7".into(),
                state: PrState::Open {
                    checks: CheckStatus::Failing,
                },
                review: ReviewState::None,
                head: "abc".into(),
                conflict: false,
            },
        );
        assert_eq!(
            header_chips(&after),
            header_chips(&before) + 1,
            "the pull request chip appeared in the header"
        );
        assert_eq!(
            press_across(before, after, MemberKey::Session(main)),
            vec![Message::Session(SessionMsg::TerminalAiCliSelected(main))]
        );
    }
}

// ---- T063, T065: pressing the rendered controls (US3 scenario 2, US2 scenario 5, FR-007) ----

mod rendered_presses {
    use super::support::covered_states::{
        changes_open_under_header, strip_with, SESSION_HEADER_CHANGES, SESSION_STRIP_ROW,
    };
    use super::support::layout::{self as lay, StateUnderTest};
    use iced::advanced::widget::Tree;
    use iced::advanced::{layout, Layout};
    use iced::{Point, Size};
    use micold_client::app::Message;
    use micold_client::features::changes::Msg as ChangesMsg;
    use micold_client::features::session::Msg as SessionMsg;
    use micold_client::features::sidebar::Msg as SidebarMsg;
    use micold_core::session::SessionLocation;

    /// What a click on the centre of the laid-out element at `path` publishes.
    fn click(under: StateUnderTest, path: &[usize]) -> Vec<Message> {
        let renderer = lay::renderer();
        let mut element = lay::view_of(&under);
        let mut tree = Tree::new(element.as_widget());
        let limits = layout::Limits::new(Size::ZERO, lay::WINDOW);
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let target = lay::walk(Layout::new(&node), lay::Layer::Base)
            .into_iter()
            .find(|r| r.path == path)
            .unwrap_or_else(|| panic!("{path:?} is laid out"));
        assert!(
            target.width > 0.0 && target.height > 0.0,
            "the control has an area to press: {target:?}"
        );
        let centre = Point::new(
            target.x + target.width / 2.0,
            target.y + target.height / 2.0,
        );
        drop(element);
        lay::clicks_at(lay::view_of(&under), &renderer, [centre])
    }

    #[test]
    fn pressing_changes_with_the_view_closed_opens_the_shown_location() {
        let pressed: Vec<Message> = click(strip_with(1, 1), SESSION_HEADER_CHANGES)
            .into_iter()
            .filter(|m| matches!(m, Message::Sidebar(_) | Message::Changes(_)))
            .collect();
        assert_eq!(
            pressed,
            vec![Message::Sidebar(SidebarMsg::ReviewChangesRequested(
                SessionLocation::Worktree("feat-short".to_string())
            ))]
        );
    }

    #[test]
    fn pressing_changes_with_the_view_open_on_the_shown_location_closes_it() {
        let state = changes_open_under_header();
        assert!(
            state.changes.open.is_some(),
            "the premise: the view is open"
        );
        let pressed: Vec<Message> = click(StateUnderTest::new(state), SESSION_HEADER_CHANGES)
            .into_iter()
            .filter(|m| matches!(m, Message::Sidebar(_) | Message::Changes(_)))
            .collect();
        assert_eq!(pressed, vec![Message::Changes(ChangesMsg::Closed)]);
    }

    /// `012` BUG-004 / FR-010, carried to the strip: with a shell instance shown and its process
    /// stopped, the restart control restarts **that instance**; a bare session-level restart is a
    /// no-op in Regular mode, where the session's AI CLI primary is still running.
    #[test]
    fn pressing_restart_restarts_the_shown_instance_not_the_session() {
        let mut under = strip_with(2, 0);
        let main = under.state.active_sessions()[0].id;
        let shell = under.state.active_sessions()[0].shells[1].id;
        under
            .state
            .workspace
            .find_session_mut(main)
            .expect("the shown session")
            .1
            .mark_shell_exited(shell);
        // The status cluster is the strip row's second child, the restart control its last.
        let path = [SESSION_STRIP_ROW, &[1, 1]].concat();
        let pressed: Vec<Message> = click(under, &path)
            .into_iter()
            .filter(|m| {
                matches!(
                    m,
                    Message::Session(
                        SessionMsg::TerminalRestartRequested
                            | SessionMsg::ShellInstanceRestartRequested(..)
                    )
                )
            })
            .collect();
        assert_eq!(
            pressed,
            vec![Message::Session(SessionMsg::ShellInstanceRestartRequested(
                main, shell
            ))]
        );
    }
}
