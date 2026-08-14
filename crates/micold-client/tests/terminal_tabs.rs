//! BUG-001 (feature 012, T036): the instance switcher's entries must all be tabs.
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
//! What is checked here is only that the call site still delegates its two rules — which variant a
//! tab draws in, and what colour a control nested inside it takes — to the places that test them
//! (`tab_variant` in `ui/terminal.rs`, and `tests/icon_roles.rs`'s contrast arithmetic). The
//! centred label, the trailing close and the no-reflow rule are visual and belong to the
//! `visual-pass` skill against `quickstart.md` §8 (T042/T044).

use std::fs;
use std::path::Path;

/// `ui/terminal.rs` with `//` and `/* */` comments stripped, so the doc comments on the very
/// function under test — which discuss variants and containers at length — cannot read as a match.
fn terminal_source() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui/terminal.rs");
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

/// The body of `fn instance_switcher_row`, from its signature to the closing brace at column 0.
fn switcher_row_body(src: &str) -> String {
    let start = src
        .find("fn instance_switcher_row")
        .expect("ui/terminal.rs must define instance_switcher_row");
    let rest = &src[start..];
    // Every top-level item ends at the first `\n}` — the function's own closing brace, since its
    // inner braces are all indented.
    let end = rest.find("\n}").map(|i| i + 2).unwrap_or(rest.len());
    rest[..end].to_string()
}

/// The rule itself — "every tab draws a container, active and inactive alike" — is a pure value
/// test, `tab_variant_always_draws_a_container` in `ui/terminal.rs`'s inline `mod tests`. This
/// gate only checks that the call site still *asks* that function, so the rule cannot be bypassed
/// by choosing a variant inline again.
///
/// Scanning the function body for `ButtonVariant::Text` outright does not work, and the first
/// draft of this gate proved it: a tab legitimately *contains* a `Text`-variant button — the
/// per-instance restart affordance — and the scan cannot tell a nested control's variant from the
/// tab's own. The distinction the rule cares about is which variant the *tab* draws in, and that
/// is exactly what delegating to `tab_variant` makes checkable.
#[test]
fn the_tab_variant_comes_from_the_shared_rule() {
    let body = switcher_row_body(&terminal_source());

    assert!(
        body.contains("tab_variant("),
        "each switcher entry's variant must come from `tab_variant(..)` (feature 012 FR-004a, \
         BUG-001), whose own test asserts both arms draw a container. Choosing the variant inline \
         here puts the rule back where no test can see it — which is how the inactive entries \
         ended up as `ButtonVariant::Text`, painting neither background nor outline, so the row \
         read as loose characters in the status bar instead of a tab strip."
    );

    // Whatever the emphasis choice is, every entry is still wrapped in the same builder.
    assert!(
        body.contains("Button::with_content(content, variant, r)"),
        "each entry must still be wrapped in a `Button` spanning the whole tab, taking the variant \
         from `tab_variant`, so a press anywhere on it selects that instance \
         (contracts/terminal-instance-switcher-ui.md)"
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
