//! The app bar elevates when content scrolls beneath it (feature 018, T043 — FR-025a).
//!
//! Material's small top app bar is flat at rest and gains elevation 2 — a tonal shift and a shadow
//! — once content is scrolled under it. The bar's job is to say "there is more above"; a bar that
//! is always raised says it while the list is at the top, and one that is never raised never says
//! it at all.
//!
//! # The signal is the sidebar, and that is a decision worth pinning
//!
//! The worktree sidebar is the only scroll region beneath the bar (contract §7.1), so its offset is
//! what the flag derives from. That is why this is testable without a renderer: "is the bar raised"
//! is a question about a number the application already holds, not about pixels.
//!
//! What must not creep in is a *second* source. A bar that also raised itself for the terminal's
//! scrollback, or for a dialog's overflow, would flicker between states that have nothing to do
//! with what is under it — so the derivation is asserted here as a pure function of one offset.

use micold_client::app::{scroll_offset_px, Message, State};
use micold_client::features::sidebar::Msg as SidebarMsg;

/// A state with the sidebar scrolled to `offset`.
fn scrolled_to(reported: f32) -> State {
    let mut state = State::default();
    state.update(Message::Sidebar(SidebarMsg::Scrolled(scroll_offset_px(
        reported,
    ))));
    state
}

/// At the top, the bar is flat.
#[test]
fn the_bar_is_flat_while_the_sidebar_is_at_the_top() {
    assert!(
        !State::default().app_bar_elevated(),
        "a freshly built application already raises its app bar, so the raise says nothing"
    );
    assert!(!scrolled_to(0.0).app_bar_elevated());
}

/// Any scroll at all raises it.
///
/// Material's rule is "content is scrolled under the bar", not "scrolled far" — the shadow exists
/// to separate the bar from what is passing beneath it, and one pixel of overlap is already that.
#[test]
fn any_scroll_beneath_the_bar_raises_it() {
    assert!(scrolled_to(1.0).app_bar_elevated());
    assert!(scrolled_to(240.0).app_bar_elevated());
}

/// Scrolling back to the top lowers it again. The flag follows the offset rather than latching —
/// a bar that stayed raised after the list returned to the top would be reporting history.
#[test]
fn returning_to_the_top_lowers_the_bar_again() {
    let mut state = scrolled_to(120.0);
    assert!(state.app_bar_elevated());

    state.update(Message::Sidebar(SidebarMsg::Scrolled(scroll_offset_px(
        0.0,
    ))));
    assert!(
        !state.app_bar_elevated(),
        "the bar stayed raised after the sidebar returned to the top"
    );
}

/// A negative offset — an overscroll bounce, or a renderer reporting past the origin — is still
/// "at the top". Treating it as scrolled would raise the bar for a gesture that moved content the
/// wrong way.
#[test]
fn an_overscroll_past_the_top_does_not_raise_the_bar() {
    assert_eq!(scroll_offset_px(-8.0), 0);
    assert!(!scrolled_to(-8.0).app_bar_elevated());
}

/// A viewport that has not settled reports a non-finite offset. That is "no reading yet", not a
/// scroll, and rounding it would produce a garbage pixel count.
#[test]
fn an_unsettled_viewport_reads_as_the_top() {
    assert_eq!(scroll_offset_px(f32::NAN), 0);
    assert_eq!(scroll_offset_px(f32::INFINITY), 0);
}

/// The flag is derived, not stored twice.
///
/// The failure this rules out is a second field that some other message forgets to update: the bar
/// would then be raised or flat according to whichever write happened last. Asserted by driving an
/// unrelated message between two scrolls and requiring the flag still to follow the offset.
#[test]
fn the_flag_follows_the_offset_and_not_the_last_unrelated_message() {
    let mut state = scrolled_to(80.0);
    state.update(Message::Sidebar(SidebarMsg::Toggled));
    assert!(
        state.app_bar_elevated(),
        "an unrelated message cleared the app bar's elevation"
    );

    state.update(Message::Sidebar(SidebarMsg::Scrolled(scroll_offset_px(
        0.0,
    ))));
    state.update(Message::Sidebar(SidebarMsg::Toggled));
    assert!(
        !state.app_bar_elevated(),
        "an unrelated message raised the app bar"
    );
}
