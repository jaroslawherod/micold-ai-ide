//! Feature 802, FR-001 (T005): the confirmation draws generic text when this window's catalog does
//! not list the occupant (a refusal can open it first), and names the session when it does.

use micold_client::ui::confirm_additional_session::body_text;

#[test]
fn the_text_is_generic_when_the_occupant_is_unknown() {
    let text = body_text("The worktree \u{201c}feat-a\u{201d}", None);
    assert!(text.contains("already has a session"), "{text}");
    assert!(
        !text.contains("session \u{201c}"),
        "no session is named: {text}"
    );
}

#[test]
fn the_text_names_the_occupant_when_known() {
    let text = body_text("The worktree \u{201c}feat-a\u{201d}", Some("Fix login"));
    assert!(text.contains("\u{201c}Fix login\u{201d}"), "{text}");
}
