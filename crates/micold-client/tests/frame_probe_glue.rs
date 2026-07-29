//! The measurement run cannot leak into an ordinary launch (feature 018, T000z — FR-039a/b, SC-017).
//!
//! The frame-time probe exists to make T000z capturable, and it works by driving the window
//! continuously: a `window::frames()` subscription yields once per presented frame, each mapped to a
//! `NoOp` that makes the runtime compose the next one. That is a render loop that never sleeps —
//! precisely the failure `tests/idle_requests_no_frames.rs` exists to forbid, deliberately committed
//! for the length of one measurement.
//!
//! Two things therefore have to stay true, and neither is checked anywhere else.
//!
//! **It must be unreachable without being asked for.** 017's gate scans `src/ui/` and
//! `src/showcase/` for `request_redraw`; `src/main.rs` is outside its scope, and this subscription
//! would not trip it in any case — it holds the loop awake without ever calling `request_redraw`.
//! So an ungated `window::frames()` here would spin the application at full rate forever, on every
//! launch, with all 017's gates and every other test still green. SC-017 would be false and nothing
//! would say so.
//!
//! **The frame-request path must stay singular.** FR-039a's guarantee is structural, not
//! behavioural: there is one way to ask for a frame. This adds a second mechanism, so it has to be
//! visibly conditional or that guarantee quietly becomes a claim about the default configuration.
//!
//! The check is structural because the behavioural version is not available: `subscription` is
//! private to the binary and a `Subscription` cannot be inspected for its contents.

use std::fs;
use std::path::{Path, PathBuf};

fn main_rs() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs")
}

fn source() -> String {
    fs::read_to_string(main_rs()).expect("read src/main.rs")
}

/// Strips `//` line comments and `/* */` blocks, so this file's subject matter — which `main.rs`
/// discusses at length around the very code being checked — cannot read as a violation.
fn code_only(src: &str) -> String {
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

/// Real lines of code subscribing to the per-frame clock, as `(line number, text)`.
fn frame_clock_sites(src: &str) -> Vec<(usize, String)> {
    code_only(src)
        .lines()
        .enumerate()
        .filter(|(_, l)| l.contains("window::frames()"))
        .map(|(i, l)| (i + 1, l.trim().to_string()))
        .collect()
}

/// The continuous-frame clock is subscribed in exactly one place.
///
/// A second site would be a second thing capable of holding the loop awake, each needing its own
/// gate, and the guarantee stops being checkable by reading one branch.
#[test]
fn the_frame_clock_is_subscribed_in_exactly_one_place() {
    let sites = frame_clock_sites(&source());
    assert_eq!(
        sites.len(),
        1,
        "expected exactly one `window::frames()` subscription in src/main.rs, found: {sites:?}"
    );
}

/// …and that one place is gated on a measurement run having been asked for.
///
/// Deliberately literal: the nearest preceding line of code must open a `probe_config()` test. That
/// is brittle against reformatting, and it is the right trade — an ungated frame clock is invisible
/// in every other test and turns the idle application into one that never sleeps.
#[test]
fn the_frame_clock_is_gated_on_a_measurement_run() {
    let src = code_only(&source());
    let lines: Vec<&str> = src.lines().collect();

    let at = lines
        .iter()
        .position(|l| l.contains("window::frames()"))
        .expect("the frame clock subscription");

    let guard = lines[..at]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("the subscription cannot be the first line of the file");

    assert!(
        guard.contains("probe_config()"),
        "the `window::frames()` subscription at src/main.rs:{} is not gated on a measurement run \
         — the line before it reads `{}`. Ungated, it drives the window continuously on every \
         ordinary launch, which is exactly what FR-039a and SC-017 forbid, and no other test in \
         this workspace would notice.",
        at + 1,
        guard.trim()
    );
}

/// The probe's own state is optional, so an ordinary launch carries no probe to record into and
/// `view` never reads the clock. Without this the timing would run on every frame the application
/// ever composes, for no one.
#[test]
fn the_probe_is_absent_unless_a_run_was_configured() {
    let src = code_only(&source());
    assert!(
        src.contains("probe: Option<"),
        "the probe must be optional state — an always-present probe means `view` times every frame \
         of every ordinary launch"
    );
    assert!(
        src.contains("probe_config().map("),
        "the probe must be constructed from the parsed configuration, so that no configuration \
         means no probe"
    );
}

/// A scan that reads nothing passes trivially. If `main.rs` moves, this fails rather than certifying
/// a clean bill of health over an empty file.
#[test]
fn the_scan_actually_reads_the_binary() {
    let src = source();
    assert!(
        src.len() > 10_000,
        "src/main.rs is suspiciously short ({} bytes) — if the binary moved, this file's path must \
         move with it, or the gate above goes unchecked",
        src.len()
    );
}

/// The synthetic Red: an ungated subscription really is caught, rather than the guard check passing
/// because it never finds anything to look at.
#[test]
fn an_ungated_frame_clock_would_be_caught() {
    let planted =
        "let mut subs = vec![];\nsubs.push(iced::window::frames().map(|_| Message::NoOp));\n";
    let src = code_only(planted);
    let lines: Vec<&str> = src.lines().collect();
    let at = lines
        .iter()
        .position(|l| l.contains("window::frames()"))
        .expect("planted subscription");
    let guard = lines[..at]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("a preceding line");

    assert!(
        !guard.contains("probe_config()"),
        "the planted ungated subscription must not look guarded — otherwise the real check proves \
         nothing"
    );
    assert_eq!(frame_clock_sites(planted).len(), 1);
}

/// Prose about the frame clock is not a frame clock. `main.rs` explains this mechanism in the
/// comment directly above it, so without stripping, the documentation would fail the check it
/// documents — and the single-site assertion would count two.
#[test]
fn prose_about_the_frame_clock_is_not_a_subscription() {
    let planted =
        "// window::frames() is only for the probe.\n/* nor window::frames() here */\nlet x = 1;\n";
    assert!(
        frame_clock_sites(planted).is_empty(),
        "comments survived stripping"
    );
}
