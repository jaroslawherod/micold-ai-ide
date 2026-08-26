//! A timer the idle window did not ask for (feature 021, T053 — FR-025, FR-032a, SC-017).
//!
//! `shell/subscriptions.rs` declares six things the runtime may wake the application for. Three of
//! them are conditional, and each condition is the only thing standing between the idle window and
//! a process that never sleeps:
//!
//! * the snackbar's 250 ms clock, subscribed only while a notification is on screen (FR-032a);
//! * the pointer stream, only while the project switcher is open (feature 015);
//! * the per-frame clock, only during a measurement run (FR-039b).
//!
//! Dropping any of those `if`s compiles, changes no behaviour a test can observe, and leaves the
//! application waking four (or sixty) times a second forever. A `Subscription` is opaque — it
//! cannot be inspected for what it contains, and `subscription` is private to the binary — so
//! there is no behavioural check available; `tests/frame_probe_glue.rs` made the same argument for
//! the third of them and reached the same conclusion. This file covers the other two, which had
//! nothing at all.
//!
//! The check reads structure rather than the preceding line. `frame_probe_glue` asserts that the
//! nearest line of code above the frame clock opens a `probe_config()` test, which is exact but
//! only survives while that push stays a one-liner; the snackbar's spans four lines, so the same
//! trick would read `subs.push(` and conclude it was unguarded. Tracking which block a line sits
//! inside costs thirty lines and does not care how the argument is wrapped.

use std::fs;
use std::path::{Path, PathBuf};

fn subscriptions_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/shell/subscriptions.rs")
}

/// Strips `//` line comments, so this file's subject matter — which `subscriptions.rs` discusses
/// at length directly above each of the pushes being checked — cannot be read as code.
fn code_only(src: &str) -> String {
    src.lines()
        .map(|l| match l.find("//") {
            Some(at) => &l[..at],
            None => l,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The line that opened the innermost block `needle` sits inside, or `None` if it sits at the top
/// level of the file.
///
/// A brace-depth walk with a stack of openers: whatever line last increased the depth is the
/// enclosing construct. `{` inside string literals would confuse it and there are none here; a
/// line that both opens and closes (`|| { … }`) nets to zero and is correctly ignored.
fn enclosing_block(src: &str, needle: &str) -> Option<String> {
    let mut stack: Vec<String> = Vec::new();
    for line in code_only(src).lines() {
        if line.contains(needle) {
            return stack.last().cloned();
        }
        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        for _ in 0..closes.min(stack.len()) {
            stack.pop();
        }
        for _ in 0..opens.saturating_sub(closes) {
            stack.push(line.trim().to_string());
        }
    }
    panic!("{needle:?} is not in src/shell/subscriptions.rs — has it been renamed or removed?");
}

/// The snackbar's clock runs only while a notification is on screen.
///
/// Four wake-ups a second, held for the life of the process, to count down a notification that
/// does not exist. `Queue::is_active` is unit-tested in `tests/notifications.rs`; nothing checked
/// that the subscription consults it, and nothing would have noticed if it stopped.
#[test]
fn the_snackbar_clock_is_subscribed_only_while_a_notification_is_showing() {
    let src = fs::read_to_string(subscriptions_rs()).expect("read src/shell/subscriptions.rs");
    let guard = enclosing_block(&src, "every(SNACKBAR_TICK)")
        .expect("the snackbar tick is subscribed unconditionally — it must be inside an `if`");
    assert!(
        guard.contains("notify.is_active()"),
        "the snackbar clock is inside `{guard}`, which does not test whether a notification is \
         showing. Subscribed at rest it wakes the process four times a second for the life of the \
         application, and no behavioural test in this workspace can see it (FR-032a, SC-017)."
    );
}

/// The pointer is not tracked **at all**.
///
/// This test used to say "only while the project switcher is open" (015 FR-010), and the
/// subscription it guarded existed for one reason: a switcher row handed over a bare message with
/// no press point in it, so the point had to be collected on the side. Since 018's BUG-008 the
/// point rides on the message — every context menu's, not just the switcher's — and the side
/// channel has no caller left.
///
/// So the requirement is satisfied by construction rather than by a guard: there is no window
/// state that can make this application listen to mouse moves, which is the strongest form of
/// "the idle window schedules nothing" (FR-025, SC-017). Asserting the absence, rather than
/// deleting the test with the code, is what stops the subscription quietly coming back the next
/// time something wants a pointer position.
#[test]
fn the_pointer_is_not_subscribed_at_all() {
    let src = fs::read_to_string(subscriptions_rs()).expect("read src/shell/subscriptions.rs");
    let code = code_only(&src);
    for banned in ["cursor_move", "CursorMoved"] {
        assert!(
            !code.contains(banned),
            "`{banned}` is back in src/shell/subscriptions.rs. A pointer subscription makes every \
             mouse move through the window an `update` — the busiest subscription there is, on an \
             application whose idle guarantee is that it schedules nothing. If a surface needs a \
             press point, take it from the gesture that raised it (`cdk::ContextArea`), which is \
             what BUG-008 established (018 FR-029d, 015 FR-010)."
        );
    }
}

/// The frame clock, checked here too — and this one is the control.
///
/// `frame_probe_glue` already gates it by an independent mechanism, so if `enclosing_block` were
/// subtly wrong, this is the assertion that says so: the two files must agree about the one push
/// whose guard is verified twice. The other two tests above have no such second opinion.
#[test]
fn the_frame_clock_agrees_with_the_gate_frame_probe_glue_checks_it_by() {
    let src = fs::read_to_string(subscriptions_rs()).expect("read src/shell/subscriptions.rs");
    let guard = enclosing_block(&src, "window::frames()")
        .expect("the frame clock is subscribed unconditionally");
    assert!(
        guard.contains("probe_config()"),
        "the frame clock is inside `{guard}` — `frame_probe_glue` reads the same gate a different \
         way, so a disagreement here means one of the two is misreading the file"
    );
}

/// The OS theme poll is deliberately *not* conditional, and that is the assertion.
///
/// This is the inverse failure and it has already happened once (003 FR-006 / SC-003): the poll
/// used to be dropped while the window was unfocused, so a visible-but-unfocused window kept the
/// wrong theme indefinitely — and leaving the app to change the OS theme is precisely what
/// unfocuses it. Only the cadence may follow focus. It doubles as this file's vacuity control: a
/// walk that reported "inside an `if`" for everything would fail here.
#[test]
fn the_theme_poll_is_never_conditional() {
    let src = fs::read_to_string(subscriptions_rs()).expect("read src/shell/subscriptions.rs");
    let guard = enclosing_block(&src, "subs.push(os_theme_poll(");
    assert_eq!(
        guard.as_deref().map(str::trim),
        Some("pub fn subscription(app: &App) -> Subscription<Message> {"),
        "the OS theme poll has been made conditional. Suspending it is the 003 FR-006 bug: an \
         unfocused window is usually still on screen. Only `os_theme_poll_interval` may vary."
    );
}

/// The walk finds a guard when there is one and reports none when there is not.
///
/// Without this the three tests above could be passing because `enclosing_block` returns whatever
/// it likes; the multi-line case is the one that matters, since it is what ruled out
/// `frame_probe_glue`'s simpler line-before rule.
#[test]
fn the_walk_distinguishes_a_guarded_push_from_a_bare_one() {
    let planted = "fn subscription() {\n    subs.push(bare());\n    if condition {\n        \
                   subs.push(\n            wrapped_over(\n                several_lines(),\n     \
                   ),\n        );\n    }\n}\n";
    assert_eq!(
        enclosing_block(planted, "subs.push(bare())").as_deref(),
        Some("fn subscription() {"),
        "a push at the function's own level must report the function, not an `if`"
    );
    assert_eq!(
        enclosing_block(planted, "several_lines()").as_deref(),
        Some("if condition {"),
        "a push wrapped over several lines is still inside its `if` — this is the case the \
         line-before rule gets wrong"
    );
}

/// Prose about a subscription is not a subscription.
///
/// `subscriptions.rs` explains each gate in the comment directly above it, so without stripping,
/// the documentation would be read as the code it documents — and `enclosing_block` returns at the
/// *first* line containing its needle, which would be a comment every time.
#[test]
fn prose_about_a_subscription_is_not_one() {
    let planted =
        "fn f() {\n    // subs.push(cursor_move_events()) is only for the switcher.\n    if real \
         {\n        subs.push(cursor_move_events());\n    }\n}\n";
    assert_eq!(
        enclosing_block(planted, "subs.push(cursor_move_events())").as_deref(),
        Some("if real {"),
        "the comment was read as the subscription, so every gate here would be judged by prose"
    );
}
