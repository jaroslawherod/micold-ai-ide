//! A track whose value feeds `Widget::layout` must ask for a re-layout (BUG-001).
//!
//! `Progress::on_frame` asks the shell for a redraw and nothing else. A redraw re-runs `draw`
//! against the layout computed by the *last* layout pass — iced only re-lays-out when a widget
//! sets `Shell::invalidate_layout`, and nothing in this codebase ever did:
//!
//! ```text
//! // iced_winit-0.14.0/src/lib.rs, the redraw tick
//! let (state, _) = interface.update(slice::from_ref(&redraw_event), ...);
//! if message_count == messages.len() && !state.has_layout_changed() { break state; }
//! ```
//!
//! So `Expand`, whose `layout()` reports `full.height * progress`, animated a height nobody
//! recomputed. Its reveal did not move, and the `with_layer` clip it relies on was handed stale
//! bounds — which is why the revealed content painted over the widgets below it.
//!
//! Two halves, matching `idle_requests_no_frames`: the behavioural half proves the primitive does
//! what it claims, and the structural half proves no future widget can animate its layout without
//! it.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use iced::advanced::Shell;
use iced::{window, Event};

use micold_client::ui::cdk::motion::Progress;

/// A realistic transition rather than a degenerate one.
const OVER: Duration = Duration::from_millis(100);

fn redraw_at(now: Instant) -> Event {
    Event::Window(window::Event::RedrawRequested(now))
}

// ---------------------------------------------------------------------------------------------
// The behavioural half
// ---------------------------------------------------------------------------------------------

/// The defect itself. A track that feeds a widget's *size* must make the shell recompute layout,
/// or the animated size is never read and the widget draws against last frame's bounds.
#[test]
fn a_track_that_feeds_layout_asks_for_a_relayout() {
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut track = Progress::new(0.0);

    track.on_layout_frame(&redraw_at(Instant::now()), 1.0, OVER, &mut shell);

    assert!(
        track.animating(),
        "precondition: one frame of a 100ms transition leaves it still moving"
    );
    assert!(
        shell.is_layout_invalid(),
        "a track feeding Widget::layout advanced without asking for a re-layout — the new value \
         would never be laid out, so the element animates against stale bounds (BUG-001)"
    );
}

/// The contrast that makes the first test mean something: a track that only feeds `draw` must
/// *not* force a re-layout. Relayout is the expensive path; `Fade`/`Scale`/`Scrim` do not need it.
#[test]
fn a_track_that_only_feeds_drawing_asks_for_no_relayout() {
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut track = Progress::new(0.0);

    track.on_frame(&redraw_at(Instant::now()), 1.0, OVER, &mut shell);

    assert!(
        !shell.is_layout_invalid(),
        "a purely visual transition forced a re-layout — every frame of every fade would then \
         re-lay-out the whole window"
    );
}

/// Quiescence, the same property `idle_requests_no_frames` protects for redraws: a track that has
/// arrived stops asking. A track that invalidated layout for ever would relayout for ever.
#[test]
fn a_settled_layout_track_asks_for_no_relayout() {
    let mut messages: Vec<()> = Vec::new();
    let mut track = Progress::new(1.0);

    let mut shell = Shell::new(&mut messages);
    for _ in 0..50 {
        track.on_layout_frame(&redraw_at(Instant::now()), 1.0, OVER, &mut shell);
    }

    assert!(!track.animating(), "precondition: the track is at rest");
    assert!(
        !shell.is_layout_invalid(),
        "a track resting at its target kept asking for a re-layout — that relayouts the window \
         for ever behind an element that has stopped moving"
    );
}

/// The reason the fix needs a frame guard at all.
///
/// iced re-runs `update` with the *same* redraw event when that update invalidated the layout —
/// up to three times, then it logs a warning. Without this, inviting a re-layout would make every
/// layout-animated transition run at several times its stated speed and spam that warning.
#[test]
fn a_track_advances_at_most_once_per_frame() {
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut track = Progress::new(0.0);

    // One frame — one `Instant`, delivered repeatedly, exactly as the runtime's redraw loop does.
    let frame = redraw_at(Instant::now());
    track.on_frame(&frame, 1.0, OVER, &mut shell);
    let after_one_update = track.value();
    for _ in 0..3 {
        track.on_frame(&frame, 1.0, OVER, &mut shell);
    }

    assert_eq!(
        track.value(),
        after_one_update,
        "the same frame advanced the track more than once — a transition re-entered within one \
         frame runs at a multiple of its stated duration"
    );
}

/// And the guard must not stall the animation: a *different* frame still advances it.
#[test]
fn a_later_frame_still_advances_the_track() {
    let mut messages: Vec<()> = Vec::new();
    let mut shell = Shell::new(&mut messages);
    let mut track = Progress::new(0.0);

    let first = Instant::now();
    track.on_frame(&redraw_at(first), 1.0, OVER, &mut shell);
    let after_first = track.value();
    track.on_frame(
        &redraw_at(first + Duration::from_millis(16)),
        1.0,
        OVER,
        &mut shell,
    );

    assert!(
        track.value() > after_first,
        "a new frame did not advance the track — the per-frame guard stalled the animation"
    );
}

// ---------------------------------------------------------------------------------------------
// The structural half: the property cannot be lost again
// ---------------------------------------------------------------------------------------------

/// Every widget whose `layout` reads an animated progress must advance it with the layout-aware
/// call. This is the half that survives the next widget: the behavioural tests above prove the
/// primitive works, but nothing would stop a new reveal reaching for `on_frame` and reintroducing
/// BUG-001 with every existing gate green — the boundary, builder-API, opacity and idle-frame
/// gates all pass on the defective code, which is how it shipped in the first place.
#[test]
fn a_widget_that_lays_out_from_progress_advances_it_as_layout() {
    let mut offenders = Vec::new();
    let mut checked = 0;

    for (path, source) in rendering_sources() {
        // Per `impl ... Widget ... for X` block, not per file: `animation.rs` holds five wrappers,
        // and a file-wide search would let one of them regress behind another's correct call.
        for block in widget_impl_blocks(&source) {
            let Some(layout_body) = method_body(block, "fn layout(") else {
                continue;
            };
            // The widget animates its own size if its `layout` reads an animated progress.
            if !(layout_body.contains("progress") || layout_body.contains("::value(tree)")) {
                continue;
            }
            checked += 1;

            let Some(update_body) = method_body(block, "fn update(") else {
                continue;
            };
            let advances_as_layout =
                update_body.contains("on_layout_frame(") || update_body.contains("advance_layout(");
            if !advances_as_layout {
                offenders.push(format!("{}: {}", path.display(), widget_name(block)));
            }
        }
    }

    assert!(
        checked >= 2,
        "expected to find at least the two known layout-animating widgets (Expand, \
         NavigationDrawer), found {checked} — the scan stopped matching and this gate is now \
         passing vacuously"
    );

    assert!(
        offenders.is_empty(),
        "these widgets size themselves from an animated progress but advance it with `on_frame`, \
         which asks only for a redraw. iced re-lays-out only on `Shell::invalidate_layout`, so \
         their animated size is never read: they sit still and clip against stale bounds \
         (BUG-001). Use `on_layout_frame`/`advance_layout` instead.\n  {}",
        offenders.join("\n  ")
    );
}

/// Each `impl ... Widget<...> for X` block in `source`, as a source slice.
///
/// Split on top-level `impl`, which is enough here: every widget in this layer writes its `Widget`
/// implementation as one unindented `impl` block.
fn widget_impl_blocks(source: &str) -> Vec<&str> {
    let mut blocks = Vec::new();
    let mut starts: Vec<usize> = source.match_indices("\nimpl").map(|(i, _)| i + 1).collect();
    starts.push(source.len());

    for pair in starts.windows(2) {
        let block = &source[pair[0]..pair[1]];
        // The `Widget` trait implementation, not the inherent `impl` or the `From` conversion.
        if block.contains("Widget<") && block.contains("fn layout(") {
            blocks.push(block);
        }
    }
    blocks
}

/// The body of `method` within `block`, up to the next method at the same indentation.
fn method_body<'a>(block: &'a str, method: &str) -> Option<&'a str> {
    let start = block.find(method)?;
    let rest = &block[start..];
    let end = rest[method.len()..]
        .find("\n    fn ")
        .map_or(rest.len(), |i| i + method.len());
    Some(&rest[..end])
}

/// The implementing type's name, for the failure message.
fn widget_name(block: &str) -> &str {
    block
        .find(" for ")
        .map(|i| &block[i + 5..])
        .and_then(|rest| rest.split(['<', ' ', '\n']).next())
        .unwrap_or("<unknown>")
}

/// Every `.rs` file under the rendering layer, as `(path, contents)`.
fn rendering_sources() -> Vec<(PathBuf, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
    let mut out = Vec::new();
    collect(&root, &mut out);
    assert!(
        !out.is_empty(),
        "found no sources under {} — this gate would pass vacuously",
        root.display()
    );
    out
}

fn collect(dir: &Path, out: &mut Vec<(PathBuf, String)>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("read {}: {e}", dir.display()));
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let source = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            out.push((path, source));
        }
    }
}
