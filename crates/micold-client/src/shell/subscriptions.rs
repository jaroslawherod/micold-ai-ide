//! What the runtime is asked to wake us for (feature 021, T053 — FR-019a).
//!
//! The external system here is **the iced runtime and the OS events it carries**: window focus,
//! pointer moves, the clocks. Every other shell module performs an effect the application asked
//! for; this one declares what the application wants to hear about, and [`subscription`] is
//! re-read by the runtime after each update, so its `if`s are live decisions rather than setup.
//!
//! # Everything here is a wake-up the idle window has to justify
//!
//! Three of the six are conditional, and the conditions are the whole point (FR-025, FR-032a,
//! FR-039a, SC-017): the snackbar's clock is subscribed only while a notification is on screen,
//! the pointer only while the project switcher is open, and the per-frame clock only during a
//! measurement run. A timer subscribed unconditionally holds the loop awake for the life of the
//! process, which is the failure this feature's idle guarantees are about — and a `Subscription`
//! cannot be inspected for what it contains, so a conditional that quietly became unconditional
//! is invisible to every behavioural test in the workspace. `tests/idle_subscriptions.rs` is what
//! makes those three conditions checkable; `tests/frame_probe_glue.rs` already did it for the
//! third and keeps doing so.
//!
//! # The theme poll is the one that is deliberately never off
//!
//! [`os_theme_poll_interval`] coarsens with focus and never suspends. An unfocused window is
//! usually still on screen, and changing the OS theme is exactly what unfocuses the app, so a
//! poll that stopped on blur left a fully visible window showing the wrong theme indefinitely
//! (003 FR-006). Both cadences stay inside SC-003's one second.
//!
//! # `detect_system_scheme` belongs to a different external system
//!
//! It, `SystemThemeProbe` and `map_system_scheme` are the OS-theme *probe*, and T054 moved them
//! to `shell/os_theme.rs`. What lives here is the clock that drives it. The two split cleanly
//! along FR-019a's line — the runtime schedules, the probe asks the operating system — so
//! [`os_theme_poll`] imports the one function it calls and owns none of it.

use std::time::Duration;

use iced::time::every;
use iced::Subscription;

use micold_client::app::Message;
use micold_client::features::window::Msg as WindowMsg;

use crate::shell::os_theme::detect_system_scheme;
use crate::{probe_config, App};

/// How often the OS light/dark preference is polled while the window has input focus
/// (research R4).
const OS_THEME_POLL: Duration = Duration::from_millis(500);

/// How often the OS theme is polled while the window is unfocused. Coarser than
/// [`OS_THEME_POLL`], but never suspended: `window_focused` is *input* focus, and an unfocused
/// window is usually still on screen (second monitor, side-by-side), so suspending the poll
/// left a fully visible window showing the wrong theme indefinitely. Changing the OS theme also
/// means leaving the app, which is exactly what unfocuses it. Kept at 1s so SC-003's
/// "within 1 second" holds whether or not the window happens to hold focus.
const BACKGROUND_OS_THEME_POLL: Duration = Duration::from_secs(1);

/// How often the snackbar's countdown ticks while one is visible.
///
/// Coarse on purpose: the durations it serves are 4s and 10s, so a quarter-second tick is
/// imperceptible in the dismissal and costs four wake-ups a second instead of sixty. It runs only
/// while a notification is on screen.
const SNACKBAR_TICK: Duration = Duration::from_millis(250);

pub fn subscription(app: &App) -> Subscription<Message> {
    // Event-driven (not a poll): reports actual OS focus changes, so it costs nothing while
    // the window sits idle either focused or not (idle-CPU fix).
    // Resize events are rare, so this costs nothing at idle; it keeps `window_size` current for
    // context-menu clamping (feature 015).
    let mut subs = vec![
        micold_client::ui::subscription(&app.core),
        window_focus_events(),
        // The daemon connection: one long-lived socket to the session host (feature 010, T041).
        micold_client::daemon::connection(),
        iced::window::resize_events().map(|(_id, size)| {
            Message::Window(WindowMsg::Resized {
                width: size.width.max(0.0) as u16,
                height: size.height.max(0.0) as u16,
            })
        }),
    ];
    // Always polled — see [`BACKGROUND_OS_THEME_POLL`]. Only the cadence follows focus.
    subs.push(os_theme_poll(os_theme_poll_interval(app.window_focused)));
    // The snackbar's clock, subscribed **only while something is on screen** (FR-032a, SC-017).
    // A timer that ran at rest would hold the loop awake for the life of the process to count down
    // a notification that does not exist; `Queue::is_active` is what keeps it off.
    if app.core.notify.is_active() {
        subs.push(
            every(SNACKBAR_TICK)
                .map(|_| Message::NotificationsAdvanced(SNACKBAR_TICK.as_millis() as u32)),
        );
    }
    // The terminal output poll is gone — the daemon streams grid frames over the connection. Worktree
    // create now runs on the daemon too, so there is no local progress buffer to drain (T055).
    // No animation clock. Every transition is played by the widget that owns it, and a widget
    // that is moving asks the runtime for the next frame itself — so the idle window schedules
    // nothing at all, rather than ticking 60 times a second to advance tracks that have all
    // arrived (FR-014, FR-025).
    // No pointer subscription. Feature 015 tracked the cursor here — only while the switcher was
    // open, so the idle window stayed free of per-mouse-move redraws (FR-010) — purely so that a
    // right-click on a row could anchor its menu somewhere. It existed because the row handed over
    // a bare message with no press point in it; since BUG-008 the point rides on the message, and
    // the side channel has no caller. FR-010 is satisfied more strongly than it asks: no window
    // state can now make this application listen to mouse moves at all.
    // A measurement run, and only a measurement run, drives the window continuously (FR-039b): the
    // scene has to be re-composed for there to be anything to time. `window::frames()` yields once
    // per presented frame, and the `NoOp` it maps to is enough to make the runtime compose the next
    // one — so this needs no `request_redraw` of its own, and 017's single sanctioned frame-request
    // path (`ui/cdk/motion.rs`) stays the only one.
    //
    // Idle quiescence (SC-017, FR-039a) is unaffected because the branch is unreachable without the
    // environment variable; `tests/frame_probe_glue.rs` is what keeps that true.
    if probe_config().is_some() {
        subs.push(iced::window::frames().map(|_| Message::NoOp));
    }
    Subscription::batch(subs)
}

/// The OS theme poll interval for this tick. Unlike the terminal poll this is never `None`:
/// suspending it while unfocused is what let a visible window keep the wrong theme (003 FR-006 /
/// SC-003). Both cadences satisfy SC-003's one-second bound.
fn os_theme_poll_interval(window_focused: bool) -> Duration {
    if window_focused {
        OS_THEME_POLL
    } else {
        BACKGROUND_OS_THEME_POLL
    }
}

/// Subscribes to raw OS window events and keeps only focus changes, translating them into
/// [`Message::WindowFocusChanged`]. Every other window event (resize, move, redraw, ...) is
/// discarded before it ever reaches `update`.
fn window_focus_events() -> Subscription<Message> {
    iced::event::listen_with(window_focus_message)
}

/// The `listen_with` callback backing [`window_focus_events`]; a free function (rather than a
/// closure) so it can be unit-tested directly.
fn window_focus_message(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::Focused) => {
            Some(Message::WindowFocusChanged(true))
        }
        iced::Event::Window(iced::window::Event::Unfocused) => {
            Some(Message::WindowFocusChanged(false))
        }
        _ => None,
    }
}

fn os_theme_poll(interval: Duration) -> Subscription<Message> {
    every(interval).map(|_instant| Message::SystemThemeChanged(detect_system_scheme()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 003 FR-006 / SC-003: the theme poll must keep running while unfocused. It used to be
    /// dropped entirely, so a visible-but-unfocused window kept the wrong theme indefinitely —
    /// and leaving the app to change the OS theme is what unfocuses it in the first place.
    #[test]
    fn fr_006_os_theme_poll_never_stops_while_unfocused() {
        assert_eq!(os_theme_poll_interval(true), OS_THEME_POLL);
        assert_eq!(os_theme_poll_interval(false), BACKGROUND_OS_THEME_POLL);
    }

    /// SC-003 bounds the update at one second whether or not the window holds focus.
    #[test]
    fn sc_003_both_theme_poll_cadences_stay_within_one_second() {
        assert!(os_theme_poll_interval(true) <= Duration::from_secs(1));
        assert!(os_theme_poll_interval(false) <= Duration::from_secs(1));
    }

    /// Regression: `Subscription::map` requires a zero-sized (non-capturing) closure. Threading
    /// `last_known` through the closure captured it and crashed the app on startup under iced
    /// 0.13's `debug_assert!`; since 0.14 the same mistake is a `const {}` compile error, so this
    /// test now only pins the construction path — the capture itself can no longer reach runtime.
    #[test]
    fn os_theme_poll_builds_with_a_non_capturing_closure() {
        let _ = os_theme_poll(OS_THEME_POLL);
    }

    fn dummy_status() -> iced::event::Status {
        iced::event::Status::Ignored
    }

    #[test]
    fn window_focus_message_maps_focused_and_unfocused() {
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::Focused),
                dummy_status(),
                iced::window::Id::unique()
            ),
            Some(Message::WindowFocusChanged(true))
        );
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::Unfocused),
                dummy_status(),
                iced::window::Id::unique()
            ),
            Some(Message::WindowFocusChanged(false))
        );
    }

    #[test]
    fn window_focus_message_ignores_other_window_events() {
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::Closed),
                dummy_status(),
                iced::window::Id::unique()
            ),
            None
        );
        assert_eq!(
            window_focus_message(
                iced::Event::Window(iced::window::Event::RedrawRequested(
                    iced::time::Instant::now()
                )),
                dummy_status(),
                iced::window::Id::unique()
            ),
            None
        );
    }
}
