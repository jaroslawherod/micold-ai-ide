//! The instance switcher's entries must read as a tab strip (feature 012, BUG-001 then BUG-002).
//!
//! # Why a source-level test
//!
//! Same reasoning as `tests/terminal_bar_stability.rs`: this guards a precondition of how the row
//! is *built*, and the defect it describes is invisible to every behavioural test. The switcher
//! worked perfectly — the right instance was active, the right message dispatched — while reading
//! as loose characters in the status bar, because only the active entry was given a container and
//! every other entry was bare text with a close glyph floating beside it. `SC-004` was satisfied
//! (you could tell which instance was active) and the row still looked wrong, which is exactly the
//! gap `FR-004a` was added to close.
//!
//! BUG-001 fixed that by giving every entry a container. BUG-002 corrected the idiom again: a tab
//! strip carries **no** container and marks its active member with an indicator, so the rule these
//! tests pin has changed once already. That is the argument for pinning it at all — each time the
//! decision moved, the test that held the old one failed loudly instead of the row quietly drifting.
//!
//! What is checked here is only that the call site still delegates its two rules — which tab is
//! marked, and what colour a control nested inside it takes — to the places that test them
//! (`tab_indicator_colour` in `ui/terminal.rs`, and `tests/icon_roles.rs`'s contrast arithmetic),
//! plus the one structural trap a value test cannot see: that both arms reserve the indicator's
//! height. The centred label, the trailing close and the indicator's *appearance* are visual and
//! belong to the `visual-pass` skill against `quickstart.md` §8.

use std::fs;
use std::path::Path;

/// `ui/terminal.rs`, comment-stripped — the call site half.
fn terminal_source() -> String {
    source_of("src/ui/terminal.rs")
}

/// `ui/material/tab.rs`, comment-stripped — the component half.
///
/// Feature 026 promoted the tab out of the call site (FR-013), so the indicator rule this file
/// pins now lives here. The property is unchanged and so is the argument for pinning it; only its
/// address moved, which is the same move `tests/inventory/mod.rs` made for the same reason.
fn tab_source() -> String {
    source_of("src/ui/material/tab.rs")
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

/// The body of `fn tab_strip_row`, from its signature to the closing brace at column 0.
///
/// It was `instance_switcher_row` until feature 026 FR-003 made the strip unconditional and FR-001
/// put the AI process in it — a strip of every displayable pane, not a switcher between instances.
fn switcher_row_body(src: &str) -> String {
    body_from(src, "fn tab_strip_row")
}

/// The body of the tab component's conversion — where the indicator is chosen and reserved.
fn tab_conversion_body(src: &str) -> String {
    body_from(src, "impl<'a, M: Clone + 'a> From<Tab<'a, M>> for Element<'a, M>")
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
    let body = switcher_row_body(&terminal_source());
    assert!(
        body.contains(".tint("),
        "the close control nested inside a tab must take that tab's foreground explicitly \
         (feature 012 FR-011a, BUG-001). `IconButton::new` defaults its tint to `on_surface`, \
         which is near tone-on-tone against the `primary` fill the active tab paints — the close \
         glyph all but disappears on the one tab a user is most likely to want to close. Pass \
         `.tint(icon_role(..))` for the tab's own state."
    );
}

// ---- feature 026: the AI process is a tab -------------------------------------------------------

/// FR-001/FR-002/FR-003: the strip holds one tab per instance **plus** the AI tab, the AI tab is
/// last, and the strip exists at zero and one instance too.
///
/// Source-level for the reason this file's module doc gives, and for one more: what is being
/// asserted is the *membership rule*, and a membership rule read off a rendered element tree is
/// read through whatever the layout did to it. The two facts that matter — that the AI tab is
/// pushed after the loop over instances, and that nothing gates the strip on how many there are —
/// are both statements about how the row is built.
#[test]
fn the_strip_holds_every_instance_plus_the_ai_tab_last() {
    let body = switcher_row_body(&terminal_source());

    assert!(
        !body.contains("shells.len() <= 1") && !body.contains("shells.is_empty()"),
        "the strip must be drawn whenever a session is displayed, including at zero and one \
         instance (FR-003, superseding feature 012 FR-005). An early return on the instance count \
         is what hid it, and it is also a bar child that comes and goes — feature 023 FR-008a."
    );

    let loop_at = body
        .find("for instance in")
        .expect("the strip must still build one tab per open instance (FR-001)");
    let ai_at = body
        .find("Icon::AiCli")
        .expect("the strip must carry a tab for the session's AI CLI process (FR-001, FR-009)");
    assert!(
        ai_at > loop_at,
        "the AI tab must be pushed **after** every terminal tab, so it holds the strip's \
         right-hand end as instances are opened and closed (FR-002, SC-006). Built before them it \
         would lead the strip, and the position FR-002 fixes would depend on iteration order."
    );
}

/// FR-004/FR-005: the AI tab has no close control, and the mark comes from the shared rule.
#[test]
fn the_ai_tab_is_unclosable_and_marked_from_one_source() {
    let body = switcher_row_body(&terminal_source());

    let ai_at = body.find("Icon::AiCli").expect("the AI tab exists");
    let after = &body[ai_at..];
    assert!(
        !after.contains("ShellInstanceCloseRequested"),
        "the AI tab must not offer a close control (FR-004, SC-005). A session has exactly one AI \
         CLI process and terminating it is not an action offered from this control — by any press."
    );

    assert!(
        body.contains("marked_tab("),
        "which tab is marked must come from `marked_tab(..)` (FR-005), whose own test proves it \
         total. Comparing against `active_shell` here instead reintroduces the state the AI pane \
         showed with nothing marked, which is the defect this feature exists to remove — and it \
         would let the mode toggle and the strip disagree, which FR-008 forbids."
    );
}
