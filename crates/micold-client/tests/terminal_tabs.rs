//! The strip's tabs must read as a tab strip (feature 012, BUG-001 then BUG-002), and since feature
//! 1126 they are the strip **above** the terminal: session tabs, the shown session's shell tabs and
//! the "+" (contracts S5, S7).
//!
//! # Why a source-level test
//!
//! Same reasoning as `tests/session_area_stability.rs`: this guards a precondition of how the row
//! is *built*, and the defect it describes is invisible to every behavioural test. The switcher
//! worked perfectly — the right instance was active, the right message dispatched — while reading
//! as loose characters in the status bar, because only the active entry was given a container and
//! every other entry was bare text with a close glyph floating beside it.
//!
//! What is checked here is that the call site delegates its rules — which member is marked and
//! what each press sends (`features::session_strip`), what colour a control nested inside a tab
//! takes (`tests/icon_roles.rs`) — plus the structural traps a value test cannot see: that every
//! tab reserves the indicator's height and its mark's slot. The behavioural half (each member sends
//! its S5 message) is asserted from a `State` below.

use micold_client::app::{Message, State};
use micold_client::features::session::Msg as SessionMsg;
use micold_client::features::session_strip::{Member, MemberKey, StripModel, StripTab};
use micold_core::project::{Availability, Project};
use micold_core::session::{AiCli, Session, SessionLocation};
use std::fs;
use std::path::Path;

/// `ui/session_area.rs`, comment-stripped — the call site half.
fn area_source() -> String {
    source_of("src/ui/session_area.rs")
}

/// `features/session_strip.rs`, comment-stripped — the rules half.
fn strip_rules_source() -> String {
    source_of("src/features/session_strip.rs")
}

/// `ui/material/tab.rs`, comment-stripped — the component half.
fn tab_source() -> String {
    source_of("../micold-material/src/tab.rs")
}

/// A source file with `//` and `/* */` comments stripped, so the doc comments on the very code
/// under test — which discuss variants and containers at length — cannot read as a match.
fn source_of(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_block = false;
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            continue;
        }
        match (c, chars.peek()) {
            ('/', Some('/')) => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            ('/', Some('*')) => {
                chars.next();
                in_block = true;
            }
            _ => out.push(c),
        }
    }
    out
}

/// The body of `fn member` — one strip member's widget (S7).
fn member_body(src: &str) -> String {
    body_from(src, "fn member<'a>(")
}

/// The body of the tab component's conversion — where the indicator is chosen and reserved.
fn tab_conversion_body(src: &str) -> String {
    body_from(
        src,
        "impl<'a, M: Clone + 'a> From<Tab<'a, M>> for Element<'a, M>",
    )
}

/// A top-level item's text, from `marker` to the closing brace at column 0.
fn body_from(src: &str, marker: &str) -> String {
    let start = src
        .find(marker)
        .unwrap_or_else(|| panic!("the source must contain `{marker}`"));
    let rest = &src[start..];
    // Every top-level item ends at the first `\n}` — the function's own closing brace, since its
    // inner braces are all indented.
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// BUG-002, FR-004b/SC-008: every tab reserves the indicator's height, and the mark comes from the
/// shared rule.
///
/// Replaces `the_tab_variant_comes_from_the_shared_rule`, which checked delegation to
/// `tab_variant` — a function that chose between two container variants and no longer exists. The
/// delegation half is kept because its reasoning still holds: choosing inline puts the rule back
/// where no test can see it. What changed is the rule.
///
/// The reserved-height half is the SC-008 trap specific to this design. An indicator drawn only on
/// the active tab grows that tab by its own thickness, pushing every tab after it — and it does so
/// between a press and its release, which is the same shape as the swallowed-press bug feature 023
/// spent a whole feature removing. Both arms of the match must produce something of the
/// indicator's height.
#[test]
fn every_tab_reserves_the_indicators_height() {
    let body = tab_conversion_body(&tab_source());

    assert!(
        body.contains("indicator_colour("),
        "the active/inactive mark must come from `indicator_colour(..)` (feature 012 FR-004b, \
         BUG-002), whose own tests assert exactly the active tab carries one. Choosing inline here \
         puts the rule back where no test can see it."
    );

    let indicator_arms = body.matches("anatomy::tab::INDICATOR").count();
    assert!(
        indicator_arms >= 2,
        "every tab must reserve the indicator's height, drawn or not — found \
         `anatomy::tab::INDICATOR` {indicator_arms} time(s), so one arm of the active/inactive \
         choice is not sizing itself by it (feature 012 SC-008). An indicator that appears only on \
         activation grows its tab by that thickness and pushes every tab after it, under the \
         pointer, between a press and its release."
    );
}

/// The nested close control must not fall through to `IconButton`'s `on_surface` default, which is
/// invisible on the active tab's fill (FR-011a, SC-007). `tests/icon_roles.rs` proves the contrast
/// arithmetic; this proves the call site actually asks for a tint.
#[test]
fn the_nested_close_control_is_tinted_from_its_tab() {
    let body = member_body(&area_source());
    assert!(
        body.contains(".tint(tint)") && body.contains("tab_content_colour("),
        "the close control nested inside a tab must take that tab's foreground explicitly \
         (feature 012 FR-011a, BUG-001). `IconButton::new` defaults its tint to `on_surface`, \
         which is near tone-on-tone against the active tab's accent."
    );
}

/// S1, S7: the strip lays out the model's members, every one of them, in a keyed row; nothing
/// gates a member on how many there are, and each member's press, close and menu come from the
/// member (S5), never written at the call site.
#[test]
fn the_strip_lays_out_the_models_members_and_their_presses() {
    let src = area_source();
    let strip = body_from(&src, "pub(crate) fn strip<'a>(");
    assert!(
        strip.contains("Keyed::row(model.members.iter()"),
        "the strip's members must be the model's, in a keyed row (R10)"
    );
    assert!(
        !strip.contains("shells.len()") && !strip.contains("shells.is_empty()"),
        "a member gated on the instance count is a child that comes and goes (feature 023 \
         FR-008a)"
    );
    let body = member_body(&src);
    for call in ["m.press()", "m.close()", "m.menu_tab()"] {
        assert!(
            body.contains(call),
            "a member's widget must send what `{call}` says (S5)"
        );
    }
    assert!(
        body.contains("tab::SESSION_WIDTH"),
        "a session tab takes the session tab's one derived width (R8)"
    );
}

/// FR-004, R8: a session tab has no close control; its trailing slot stays reserved and empty.
#[test]
fn a_session_tab_is_unclosable() {
    let body = member_body(&area_source());
    let session_arm = body
        .split_once("Member::Session {")
        .and_then(|(_, rest)| rest.split_once("Member::Shell {"))
        .expect("`member` matches a session tab before a shell tab")
        .0;
    assert!(
        !session_arm.contains(".trailing(") && !session_arm.contains("CloseRequested"),
        "a session tab must not offer a close control (feature 026 FR-004): its trailing slot \
         stays reserved and empty, so it is no narrower than its neighbours"
    );
}

/// Research R4 / feature 023 FR-008a: the mark's slot is **reserved**, never pushed-or-not.
///
/// A tab is a pressable control whose press is the whole feature, and a child that comes and goes
/// inside one shifts every sibling after it — iced's positional `Tree::diff_children` then hands
/// the pressed control its neighbour's node and the press is dropped. A mark that appears when a
/// process exits is exactly such a child.
#[test]
fn the_marks_slot_is_reserved_rather_than_pushed_when_stopped() {
    let body = member_body(&area_source());
    assert_eq!(
        body.matches(".leading(mark(").count(),
        2,
        "both kinds of tab must always build their leading slot (FR-012c, research R4)"
    );
    for line in body.lines() {
        let trimmed = line.trim_start();
        assert!(
            !(trimmed.starts_with("if ") && line.contains(".leading(")),
            "a tab builds its leading slot conditionally: `{trimmed}`. Pass the badge \
             unconditionally and let its *emphasis* be `None` when there is nothing to draw."
        );
    }
}

/// Feature 029, T033 (FR-010, SC-004a), carried to the session tab (R8): the label reads the
/// session's CLI from `command()`, so a Pi session's tab says `pi` without the strip knowing Pi
/// exists.
#[test]
fn a_session_tab_names_the_cli_from_the_provider_and_never_a_literal() {
    let src = strip_rules_source();
    let tab = body_from(&src, "fn session_member(");
    assert!(
        tab.contains("command_caption(provider, kind)") && tab.contains("s.provider"),
        "a session tab's label must come from the session's provider, in the command register"
    );
    assert!(
        !tab.contains("display_name()"),
        "a tab is a label in a width budget; the menu register does not belong in it"
    );
    for literal in ["\"pi\"", "\"claude\"", "\"copilot\""] {
        assert!(
            !tab.contains(literal),
            "a session tab names a CLI by literal ({literal}), so every other CLI's session wears it"
        );
    }
}

/// A project at `/repo` with two sessions at its root, the second shown; the first holds one shell.
fn two_sessions() -> (State, [micold_core::session::SessionId; 2]) {
    let mut s = State::default();
    let path = std::path::PathBuf::from("/repo");
    s.workspace.projects.push(Project {
        path: path.clone(),
        display_name: "repo".to_string(),
        is_git_repo: true,
        availability: Availability::Available,
    });
    s.workspace.active = Some(path);
    let mut ids = Vec::new();
    for _ in 0..2 {
        let session = Session::start_new(SessionLocation::Default, AiCli::ClaudeCode);
        ids.push(session.id);
        s.update(Message::Session(SessionMsg::Started(session)));
    }
    s.update(Message::Session(SessionMsg::Selected(ids[0])));
    let (_, main) = s.workspace.find_session_mut(ids[0]).expect("started");
    main.open_shell_instance();
    (s, [ids[0], ids[1]])
}

/// US2 sc. 2 and 3, S5: pressing a session, shell, "+" or another session's member sends what the
/// bar's control did (or the sidebar row does, for another session).
#[test]
fn each_strip_member_sends_its_s5_message() {
    let (state, [main, other]) = two_sessions();
    let model = StripModel::of(&state).expect("a session is shown");
    let shell = state.active_sessions()[0].shells[0].id;
    let presses: Vec<(MemberKey, Message)> = model
        .members
        .iter()
        .map(|m: &Member| (m.key(), m.press()))
        .collect();
    assert_eq!(
        presses,
        vec![
            (
                MemberKey::Session(main),
                Message::Session(SessionMsg::TerminalAiCliSelected(main))
            ),
            (
                MemberKey::Shell(main, shell),
                Message::Session(SessionMsg::ShellInstanceSelected(main, shell))
            ),
            (
                MemberKey::Plus,
                Message::Session(SessionMsg::ShellInstanceOpenRequested)
            ),
            (
                MemberKey::Session(other),
                Message::Session(SessionMsg::Selected(other))
            ),
        ]
    );
    assert_eq!(model.members[1].menu_tab(), Some(StripTab::Instance(shell)));
    assert_eq!(
        model.location,
        Some(SessionLocation::Default),
        "New session starts another session here"
    );
    let src = area_source();
    let strip = body_from(&src, "pub(crate) fn strip<'a>(");
    assert!(
        strip.contains("SessionMsg::AdditionalRequested(at)") && strip.contains("Icon::AddSession"),
        "New session runs **Start another session…** for the shown location (US2 sc. 3)"
    );
}
