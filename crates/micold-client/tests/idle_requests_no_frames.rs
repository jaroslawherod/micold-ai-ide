//! At rest, nothing asks for a frame (feature 017, T054 — FR-025, SC-008).
//!
//! `quickstart.md` §B6 asks a person to press every interactive element, idle, and confirm no
//! animation state is held. That pass was never run, and it is the weaker check anyway: it proves
//! nothing about the elements nobody thought to press.
//!
//! The property underneath it is much stronger and is checkable. There is exactly one call to
//! `Shell::request_redraw` in the entire rendering layer — inside [`Progress::on_event`], behind
//! `if self.animating()` — and, since feature 038, exactly one to `Shell::request_redraw_at`, the
//! whole body of `motion::wake_at`, which one named component may call. So "the application asks for a frame only while something is moving" is
//! not a behaviour to be spot-checked but a structural fact: a component *cannot* hold the render
//! loop awake, because it has no way to ask.
//!
//! That fact currently holds by accident. Nothing stopped a component calling
//! `shell.request_redraw()` directly — it would pass the boundary, builder-API and opacity gates
//! untouched, and the application would spin at 60fps forever with every existing test green. This
//! file is what makes it hold on purpose.
//!
//! Two halves, because either alone would be misleading. The behavioural half drives a real
//! `Shell` and proves the guard does what it claims; the structural half proves the guard is the
//! only door.

#[path = "support/mod.rs"]
mod support;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::advanced::Shell;
use iced::window::RedrawRequest;
use iced::{window, Event};

use micold_client::ui::cdk::motion::{Progress, FRAME};

use support::tooltip::{tooltip, Driven, DELAY};

/// The duration a component would state in its own motion spec. Any value works; this one is a
/// realistic transition rather than a degenerate one.
const OVER: Duration = Duration::from_millis(100);

/// The redraw event for frame `n` of a run that started at `origin`.
///
/// Numbered rather than `Instant::now()`, because a track advances by the wall-clock time between
/// the frame it is handed and the one before it (007 BUG-001). Two `Instant::now()`s taken inside a
/// test loop are microseconds apart, so a thousand of them would not add up to one frame of a
/// 100 ms transition and nothing here would ever arrive.
fn frame_at(origin: Instant, n: u32) -> Event {
    Event::Window(window::Event::RedrawRequested(origin + FRAME * n))
}

// ---------------------------------------------------------------------------------------------
// The behavioural half: the guard does what it says
// ---------------------------------------------------------------------------------------------

/// A track that has nowhere to go asks for nothing. This is the idle case — the one that decides
/// whether a backgrounded window burns CPU.
#[test]
fn a_resting_track_asks_for_no_frame() {
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut track = Progress::new(0.0);

    let origin = Instant::now();
    for n in 1..=10 {
        track.on_frame(&frame_at(origin, n), 0.0, OVER, &mut shell);
    }

    assert!(!track.animating(), "precondition: the track is at rest");
    assert_eq!(
        shell.redraw_request(),
        RedrawRequest::Wait,
        "a track resting at its target asked for a frame — at rest the application must ask for \
         nothing at all (FR-025)"
    );
}

/// While moving it does ask, otherwise the transition would stall halfway.
#[test]
fn a_moving_track_asks_for_the_next_frame() {
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut track = Progress::new(0.0);

    track.on_frame(&frame_at(Instant::now(), 1), 1.0, OVER, &mut shell);

    assert!(track.animating(), "precondition: the track is in flight");
    assert_eq!(
        shell.redraw_request(),
        RedrawRequest::NextFrame,
        "a moving track must ask for the next frame or the transition stalls partway"
    );
}

/// The join between the two: it stops asking the moment it arrives, without needing to be told.
///
/// A fresh `Shell` per frame is what makes this observable — a `Shell` accumulates the most urgent
/// request across a whole pass, so reusing one would remember the request from while it was still
/// moving and this could never fail.
#[test]
fn it_stops_asking_the_moment_it_arrives() {
    let mut track = Progress::new(0.0);
    let mut asked_after_arrival = 0;
    let mut frames = 0;

    let origin = Instant::now();
    for n in 1..1_000 {
        let mut messages: Vec<()> = Vec::new();
        let mut shell = Shell::new(&mut messages);
        track.on_frame(&frame_at(origin, n), 1.0, OVER, &mut shell);
        frames += 1;

        if !track.animating() {
            // One more frame, now that it has arrived. This is the frame that matters.
            let mut messages: Vec<()> = Vec::new();
            let mut shell = Shell::new(&mut messages);
            track.on_frame(&frame_at(origin, n + 1), 1.0, OVER, &mut shell);
            if shell.redraw_request() != RedrawRequest::Wait {
                asked_after_arrival += 1;
            }
            break;
        }
    }

    assert!(track.value() == 1.0, "it must actually arrive");
    assert!(
        frames > 1,
        "arriving in one frame would not exercise anything"
    );
    assert_eq!(
        asked_after_arrival, 0,
        "the track kept asking for frames after reaching its target — this is the shape of an \
         animation that holds the render loop awake for good"
    );
}

// ---------------------------------------------------------------------------------------------
// The timed wake: a tooltip waiting for a cursor at rest (feature 038, FR-018)
// ---------------------------------------------------------------------------------------------

/// Waiting for a rest delay costs one timed wake, not a frame loop.
///
/// A tooltip that opens after the cursor has rested for three seconds has to be looked at again
/// when the three seconds are up, though nothing else happens meanwhile. It asks for exactly that:
/// a redraw *at* the instant the wait ends. It never asks for the next frame while it waits, and
/// once open it asks for nothing.
#[test]
fn a_waiting_rest_tooltip_asks_for_one_timed_wake_and_none_once_open() {
    let mut tip = Driven::new(tooltip(Some(DELAY), None));
    let start = Instant::now();
    let cursor = tip.over(20.0);

    let arrived = tip.frame(start, cursor);
    assert_eq!(
        arrived.redraw,
        RedrawRequest::At(start + DELAY),
        "the cursor came to rest: one wake, when the delay has run",
    );

    let mid_wait = tip.frame(start + Duration::from_secs(1), cursor);
    assert_eq!(
        mid_wait.redraw,
        RedrawRequest::At(start + DELAY),
        "a frame during the wait asks for the same instant again, and for no frame sooner",
    );

    let opening = tip.frame(start + DELAY, cursor);
    assert!(tip.is_open(), "precondition: the delay opened it");
    assert_eq!(
        opening.redraw,
        RedrawRequest::Wait,
        "it opens on the frame it was woken for: that frame paints it, and nothing more is due",
    );

    for n in 1..=10 {
        let settled = tip.frame(start + DELAY + FRAME * n, cursor);
        assert_eq!(
            settled.redraw,
            RedrawRequest::Wait,
            "an open tooltip asked for a frame {n} frames after opening",
        );
    }
}

/// Away and spent, there is no wait to be woken from.
#[test]
fn a_rest_tooltip_that_is_away_or_spent_asks_for_no_timed_wake() {
    let mut tip = Driven::new(tooltip(Some(DELAY), None));
    let start = Instant::now();

    let away = tip.frame(start, iced::mouse::Cursor::Available(iced::Point::ORIGIN));
    assert_eq!(away.redraw, RedrawRequest::Wait, "the cursor is elsewhere");

    let opened = tip.rest_until_open(start);
    let cursor = tip.over(20.0);
    let press = tip.pressed(cursor);
    assert_eq!(
        press.redraw,
        RedrawRequest::NextFrame,
        "the press closes the panel: the one frame that paints the close, through `Progress`",
    );
    for n in 1..=10 {
        let spent = tip.frame(opened + FRAME * n, cursor);
        assert_eq!(
            spent.redraw,
            RedrawRequest::Wait,
            "a spent tooltip asked for something {n} frames after the press",
        );
    }
}

/// A tooltip that was never given a rest delay never asks for a timed wake: the twenty call sites
/// that predate the rest mode keep the frame behaviour they had.
#[test]
fn a_tooltip_without_a_rest_delay_asks_for_no_timed_wake() {
    let mut tip = Driven::new(tooltip(None, None));
    let start = Instant::now();
    let cursor = tip.over(20.0);

    for n in 0..10 {
        let seen = tip.frame(start + FRAME * n, cursor);
        assert!(
            !matches!(seen.redraw, RedrawRequest::At(_)),
            "a tooltip without `after_rest` asked for a timed wake: {:?}",
            seen.redraw,
        );
    }
    assert!(tip.is_open(), "it opened at once, as it always has");
    assert_eq!(
        tip.frame(start + FRAME * 10, cursor).redraw,
        RedrawRequest::Wait,
        "and settled",
    );
}

// ---------------------------------------------------------------------------------------------
// The structural half: the guard is the only door
// ---------------------------------------------------------------------------------------------

fn ui_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui")
}

/// Every directory holding render glue, relative to `src/`.
///
/// `showcase/` joined this list with feature 020: the component showcase is a second binary with its
/// own view, and FR-023 states plainly that it is not exempt from the guarantees the library already
/// carries. A directory outside this scan would be exempt in practice, whatever the spec said.
const RENDER_DIRS: &[&str] = &["ui", "showcase"];

/// The one file allowed to ask for a frame, relative to `src/`.
const SANCTIONED: &str = "ui/cdk/motion.rs";

/// Every `.rs` file under the render directories, recursively, as `(path relative to src/, source)`.
fn ui_sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<(String, String)>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
        for entry in entries {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let name = path
                    .strip_prefix(Path::new(env!("CARGO_MANIFEST_DIR")).join("src"))
                    .unwrap_or(&path)
                    .display()
                    .to_string()
                    .replace('\\', "/");
                out.push((name, fs::read_to_string(&path).expect("read source")));
            }
        }
    }
    let mut out = Vec::new();
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for dir in RENDER_DIRS {
        walk(&src.join(dir), &mut out);
    }
    out.sort();
    out
}

/// Strips `//` line comments and `/* */` blocks, so prose *about* the rule cannot trip it — this
/// file's own subject matter appears in several module docs.
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

/// Lines of real code naming `request_redraw`, as `(file, line number, text)`.
fn redraw_call_sites() -> Vec<(String, usize, String)> {
    let mut sites = Vec::new();
    for (path, src) in ui_sources() {
        for (i, line) in code_only(&src).lines().enumerate() {
            if line.contains("request_redraw") {
                sites.push((path.clone(), i + 1, line.trim().to_string()));
            }
        }
    }
    sites
}

/// The headline structural claim: only the motion primitive may ask for a frame.
///
/// A component wanting a redraw must go through `Progress`, which will only ask while it is
/// actually moving. That is what makes idle quiescence impossible to get wrong rather than merely
/// currently right.
#[test]
fn only_the_motion_primitive_asks_for_frames() {
    let strays: Vec<_> = redraw_call_sites()
        .into_iter()
        .filter(|(path, _, _)| path != SANCTIONED)
        .map(|(path, line, text)| format!("  {path}:{line}  {text}"))
        .collect();

    assert!(
        strays.is_empty(),
        "a module outside `{SANCTIONED}` asks the runtime for a frame:\n{}\n\nIdle quiescence \
         (FR-025, SC-008) holds because every redraw request goes through `Progress`, which asks \
         only while it is moving. A direct call bypasses that: the render loop stays awake at \
         60fps with every other test still green. Own a `Progress` and let it ask.",
        strays.join("\n")
    );
}

/// …and inside the sanctioned file there are exactly two doors, each held shut its own way.
///
/// Without this, `only_the_motion_primitive_asks_for_frames` would still pass if the guard were
/// deleted — the call would be in the right file and would spin forever.
///
/// 1. `request_redraw()`, the next frame: behind the `animating()` guard. The check is deliberately
///    literal: the nearest preceding line of code must open an `animating()` test. That is brittle
///    against reformatting, and that is the correct trade — a guard this load-bearing should not
///    be able to change shape unnoticed.
/// 2. `request_redraw_at(..)`, one frame at a stated instant (feature 038, FR-018): the body of
///    `wake_at`, and nowhere else. A timed wake is not a loop — it asks once and is answered once —
///    so what holds it is who may call it: `wake_at_has_one_caller` below.
#[test]
fn the_frame_requests_are_the_guarded_one_and_the_timed_one() {
    let src = code_only(
        &fs::read_to_string(ui_dir().join("cdk/motion.rs")).expect("read the motion primitive"),
    );
    let lines: Vec<&str> = src.lines().collect();

    let calls: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.contains("request_redraw"))
        .map(|(i, _)| i)
        .collect();
    let (timed, immediate): (Vec<usize>, Vec<usize>) = calls
        .iter()
        .partition(|&&i| lines[i].contains("request_redraw_at"));

    assert_eq!(
        (immediate.len(), timed.len()),
        (1, 1),
        "expected exactly one next-frame request and one timed request in the motion primitive, \
         found {} and {}",
        immediate.len(),
        timed.len()
    );

    let at = immediate[0];
    let guard = lines[..at]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("a frame request cannot be the first line of the file");

    assert!(
        guard.contains("animating()"),
        "the frame request at cdk/motion.rs:{} is not guarded by `animating()` — the line before \
         it reads `{}`. Unguarded, it asks for a frame on every event forever, which is precisely \
         the failure FR-025 forbids.",
        at + 1,
        guard.trim()
    );

    let at = timed[0];
    let opener = lines[..at]
        .iter()
        .rev()
        .find(|l| !l.trim().is_empty())
        .expect("a timed request cannot be the first line of the file");
    assert!(
        opener.contains("fn wake_at") && lines[at].contains("RedrawRequest::At("),
        "the timed request at cdk/motion.rs:{} is not the whole body of `wake_at` — the line \
         before it reads `{}`. It must ask for one frame at a stated instant and do nothing else.",
        at + 1,
        opener.trim()
    );
}

/// The timed door has one caller: the tooltip's rest mode.
///
/// `wake_at` cannot spin by itself, but a component that called it on every frame with an instant
/// a frame away would be a frame loop in two steps. So a new caller is a decision, made here: add
/// the file to this list with the reason it stops asking.
#[test]
fn wake_at_has_one_caller() {
    /// Who may ask for a timed wake, and why it ends.
    const CALLERS: &[(&str, &str)] = &[(
        "ui/cdk/tooltip.rs",
        "asks only while its `RestTimer` is waiting, for the instant the wait ends",
    )];

    let mut sites = Vec::new();
    for (path, src) in ui_sources() {
        if path == SANCTIONED {
            continue;
        }
        for (i, line) in code_only(&src).lines().enumerate() {
            if line.contains("wake_at(") {
                sites.push((path.clone(), i + 1, line.trim().to_string()));
            }
        }
    }

    let strays: Vec<String> = sites
        .iter()
        .filter(|(path, _, _)| !CALLERS.iter().any(|(allowed, _)| allowed == path))
        .map(|(path, line, text)| format!("  {path}:{line}  {text}"))
        .collect();
    assert!(
        strays.is_empty(),
        "a module outside {CALLERS:?} asks for a timed wake:\n{}",
        strays.join("\n")
    );
    for (caller, _) in CALLERS {
        let calls = sites.iter().filter(|(path, _, _)| path == caller).count();
        assert_eq!(
            calls, 1,
            "`{caller}` is listed as the caller of `wake_at` and calls it {calls} times: one \
             call, on the one path that waits",
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The second door: a widget that throws its own children away
// ---------------------------------------------------------------------------------------------

/// The brace-matched block starting at the first `{` at or after `at`, plus the byte index just
/// past it. `None` if there is no `{` left or the braces do not balance.
fn block_at(code: &str, at: usize) -> Option<(&str, usize)> {
    let open = at + code[at..].find('{')?;
    let mut depth = 0usize;
    for (i, c) in code[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = open + i + c.len_utf8();
                    return Some((&code[open..end], end));
                }
            }
            _ => {}
        }
    }
    None
}

/// Every `impl` block in a source file, as `(header, body)`.
///
/// Impl granularity is the point. A file granular version of this check passes on `select.rs` for
/// the wrong reason: the same file also holds `ListWatch`, whose `diff` is present and correct, and
/// that was enough to hide the missing one next to it.
fn impl_blocks(code: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut from = 0usize;
    while let Some(rel) = code[from..].find("\nimpl") {
        let at = from + rel + 1;
        let Some((body, end)) = block_at(code, at) else {
            break;
        };
        let header = code[at..code[at..].find('{').map_or(at, |o| at + o)]
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        out.push((header, body.to_string()));
        from = end;
    }
    out
}

/// Whether an impl body assembles its children inside `fn layout`.
fn assembles_in_layout(body: &str) -> bool {
    let mut from = 0usize;
    while let Some(rel) = body[from..].find("fn layout") {
        let at = from + rel;
        let Some((layout, end)) = block_at(body, at) else {
            return false;
        };
        if layout.contains("diff_children") {
            return true;
        }
        from = end;
    }
    false
}

/// Widget impls whose `layout` assembles their children, as `(path relative to src/, impl header)`.
fn assemblers() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (path, src) in ui_sources() {
        for (header, body) in impl_blocks(&code_only(&src)) {
            if assembles_in_layout(&body) {
                out.push((path.clone(), header));
            }
        }
    }
    out
}

/// A widget that builds its children in `layout` must override `diff`, or it discards them.
///
/// This is the *other* way to hold the render loop awake, and it leaves no fingerprint the checks
/// above can see: no component calls `request_redraw`, every `Progress` is guarded and correct, and
/// the application still spins at 60fps. iced's default `Widget::diff` is `tree.children.clear()`,
/// which is right for a widget that hands its children over in `children()` — and destructive for
/// one that assembles them a step later, in `layout`. The cleared subtree is rebuilt from scratch,
/// its transitions restart from zero, restarting asks for a frame, the frame re-runs `view()`, and
/// the rebuild clears it again. That loop is BUG-001: the select's list blinked for as long as it
/// was open.
///
/// Declaring `diff` is the fix, and an empty body is the right one when `layout` already diffs the
/// child in — so the rule costs one line in a widget that genuinely wants the default.
#[test]
fn a_widget_that_assembles_in_layout_keeps_its_subtree() {
    let found = assemblers();

    assert!(
        found.iter().any(|(p, _)| p == "ui/material/select.rs"),
        "no widget was found to assemble its children in `layout`, so this check is vacuous. \
         `ui/material/select.rs` is the widget that does it and the reason the rule exists; if it \
         changed shape, this test must be re-aimed rather than left to pass on an empty set. \
         Found: {found:?}"
    );

    let sources = ui_sources();
    let offenders: Vec<_> = found
        .iter()
        .filter(|(path, header)| {
            let src = code_only(
                &sources
                    .iter()
                    .find(|(p, _)| p == path)
                    .expect("path came from the same scan")
                    .1,
            );
            impl_blocks(&src)
                .iter()
                .find(|(h, _)| h == header)
                .is_some_and(|(_, body)| !body.contains("fn diff("))
        })
        .map(|(path, header)| format!("  {path}  {header}"))
        .collect();

    assert!(
        offenders.is_empty(),
        "these widgets assemble their children in `layout` but leave `Widget::diff` at its \
         default, which is `tree.children.clear()`:\n{}\n\nEvery rebuild of the view throws the \
         assembled subtree away — scroll positions, ripples, and any transition in flight. A \
         restarted transition asks for a frame, that frame rebuilds the view, and the widget never \
         settles (BUG-001, feature 022). Override `diff`; an empty body is the right one when \
         `layout` already diffs the child in.",
        offenders.join("\n")
    );
}

/// A scan that scans nothing passes trivially. If `src/ui/` moves or the sanctioned file is
/// renamed, this fails rather than reporting a clean bill of health for an empty set.
#[test]
fn the_scan_actually_finds_the_rendering_layer() {
    let sources = ui_sources();
    assert!(
        sources.len() > 20,
        "found only {} sources under {} — the checks above would be near-vacuous",
        sources.len(),
        ui_dir().display()
    );
    assert!(
        sources.iter().any(|(p, _)| p == SANCTIONED),
        "`{SANCTIONED}` not found; if the motion primitive moved, this file's constant must move \
         with it — otherwise the sanctioned call becomes a stray and the real guard goes unchecked"
    );
    // Feature 020: the component showcase is a second binary with its own render glue. FR-023 says
    // it is not exempt from the single sanctioned frame-request path, and a directory this scan does
    // not know about would be exempt in fact — it could call `shell.request_redraw()` and spin at
    // 60fps forever with every other test green, which is the failure this file's own module doc
    // describes.
    assert!(
        sources.iter().any(|(p, _)| p.starts_with("showcase/")),
        "the showcase's sources are not being scanned — FR-023 holds it to the same frame-request \
         path as the application, and this scan is what makes that true. Found: {:?}",
        sources.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
    // Two since feature 038: the guarded next-frame request and the timed wake.
    assert_eq!(
        redraw_call_sites().len(),
        2,
        "expected exactly two frame requests across the whole rendering layer, found: {:?}",
        redraw_call_sites()
    );
}

/// Feature 027, T064: the settings surface is inside the scan.
///
/// It is the largest thing this feature adds to the rendering layer, and a view rewrite is the
/// change most likely to leave a component asking for frames at rest — a rail that animates its
/// selection, a section that fades as it swaps. `ui_sources()` walks `src/ui` recursively, so
/// these files are covered the moment they exist; what is *not* automatic is that they stay
/// there. Naming them makes moving the settings surface out from under the walk a failure here
/// rather than a silent loss of coverage, which is the same reason the showcase is named above.
#[test]
fn the_scan_reaches_the_settings_surface() {
    let sources = ui_sources();
    let found: Vec<&String> = sources
        .iter()
        .map(|(p, _)| p)
        .filter(|p| p.as_str() == "ui/settings_view.rs" || p.starts_with("ui/settings/"))
        .collect();

    assert!(
        found.iter().any(|p| p.as_str() == "ui/settings_view.rs"),
        "`settings_view.rs` is not being scanned for frame requests. Found under {}: {:?}",
        ui_dir().display(),
        sources.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
    assert!(
        found.len() >= 5,
        "expected the settings view and a module per section inside the scan, found {found:?}"
    );
    assert!(
        sources
            .iter()
            .any(|(p, _)| p == "ui/material/section_list.rs"),
        "the section rail is not being scanned — it is the one new component here that renders a \
         selection, which is exactly the shape that grows an animation later"
    );
}
