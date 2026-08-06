//! Fixed, invented content for components that would otherwise need real data (feature 020, T010 —
//! FR-006, FR-022).
//!
//! Some components cannot be rendered without content the showcase has no access to: a terminal grid,
//! a list of worktrees, a set of known projects. FR-006 says those must be rendered with fixed sample
//! content rather than omitted — a catalogue that skipped everything data-shaped would omit exactly the
//! components hardest to see in the running application.
//!
//! Everything here is a constant or a pure function of no arguments. Nothing reads the clock, a random
//! source, the environment or the filesystem, which is what makes two launches identical (SC-010) and
//! is enforced by `tests/showcase_determinism.rs`.
//!
//! **Plain data only.** This module holds strings, numbers and one grid — never component values. A
//! `TreeItem` or a `MenuItem` needs the active `Roles` and the gallery's `Message`, so those are built
//! in [`super::sections`] where both are in hand. Keeping the boundary here means sample content
//! belongs to the gallery, and nothing about a component's appearance leaks into its sample data.

use micold_core::protocol::grid::{
    GridFrame, LineId, StyleRun, WireColor, WireCursor, WireCursorShape, WireLine, WireStyle,
};
use micold_core::session::SessionId;

use crate::grid::GridCache;

// ---------------------------------------------------------------------------------------------
// Text
// ---------------------------------------------------------------------------------------------

/// A label short enough to sit in a row of siblings without dominating it.
pub const LABEL: &str = "Rebuild index";

/// A second label, so a row of two instances is visibly two things rather than one repeated.
pub const OTHER_LABEL: &str = "Discard changes";

/// A label long enough to be truncated, for the components whose job is truncating one.
pub const LONG_LABEL: &str =
    "feat/an-unusually-long-worktree-name-that-will-not-fit-in-the-sidebar";

/// A sentence, for the components that show prose rather than a label.
pub const BODY: &str =
    "Sessions keep running when you close the window. Reopen it and they are still there.";

/// What a text field is *called* — its label, which sits inside the container.
///
/// Split from [`PLACEHOLDER`] deliberately. This used to be the placeholder, which is the
/// arrangement §7.7's migration table exists to undo: the field's name belongs in the label, and
/// the placeholder holds a genuine example or nothing at all. A gallery that demonstrated the old
/// bundling would be teaching call sites the thing the contract removed.
pub const FIELD_LABEL: &str = "Branch name";

/// A genuine example of what a text field would contain — never its name.
pub const PLACEHOLDER: &str = "e.g. feat/login-page";

/// What a filled text field contains.
pub const FILLED: &str = "feat/material-showcase";

/// Invented worktree rows: `(display name, depth)`. Two depths, so a tree reads as a tree.
pub const WORKTREES: &[(&str, u16)] = &[
    ("main", 0),
    ("feat/material-showcase", 1),
    ("feat/layout-snapshots", 1),
    ("fix/scrollback-clamp", 1),
];

/// Invented known projects: `(display name, running session count, available)`.
pub const PROJECTS: &[(&str, usize, bool)] = &[
    ("micold-ai-ide", 2, true),
    ("session-daemon-notes", 0, true),
    ("archived-experiment", 0, false),
];

/// The options a dropdown offers.
pub const CHOICES: &[&str] = &["Follow the system", "Always light", "Always dark"];

/// What the type-ahead searches over: `(label, available)`.
///
/// Deliberately mixed, so one query exercises three different rows: one the query hits literally,
/// one long enough that the row has to truncate around its match, and one present but unavailable —
/// whose reason is part of its label, because the component has no second text slot for it.
pub const SEARCH_RESULTS: &[(&str, bool)] = &[
    ("feat/login-page", true),
    (
        "feat/experimental-rendering-pipeline-for-the-changelog-viewer",
        true,
    ),
    ("fix/logout-redirect — in use in ../review", false),
    ("main", true),
];

/// A tag's text, for the chip-shaped components.
pub const TAG: &str = "feat";

/// A stage name, for the progress indicator that pairs a bar with the step in flight.
pub const STAGE: &str = "Fetching submodules…";

/// A live output line under that stage — what a long stage reports while it runs (BUG-009, T123).
/// Deliberately a real-shaped git line, so the pose shows the width the component actually meets.
pub const STAGE_DETAIL: &str = "Receiving objects:  47% (470/1000), 1.21 MiB | 2.30 MiB/s";

// ---------------------------------------------------------------------------------------------
// The terminal grid
// ---------------------------------------------------------------------------------------------

/// The session the fabricated frame claims to belong to.
///
/// The nil UUID rather than a fresh one: `SessionId::new()` is random, and a gallery whose content
/// differed between launches would fail SC-010 (and the determinism gate, which forbids `new_v4`).
/// Nothing here talks to a daemon, so the value only has to be stable.
fn sample_session() -> SessionId {
    SessionId::from_uuid(uuid::Uuid::nil())
}

/// Width of the fabricated screen, in cells. Wide enough for the longest line below.
const COLS: u16 = 64;

/// `alacritty_terminal::vte::ansi::NamedColor::Foreground`'s discriminant, carried on the wire
/// verbatim. Named here rather than imported so this module stays free of the terminal stack: the
/// numbers are part of the wire contract (`WireColor::Named`'s doc records why they need `u16`).
const FG: u16 = 256;
/// `NamedColor::Background`.
const BG: u16 = 257;
/// `NamedColor::Green`, for the lines a test run prints in green.
const GREEN: u16 = 2;

/// The lines the fabricated terminal shows. Plausible session output, so the pane reads as a
/// terminal rather than as a placeholder — and fixed, so it reads the same tomorrow.
const SCREEN: &[&str] = &[
    "$ cargo test --workspace",
    "   Compiling micold-core v0.5.0",
    "   Compiling micold-client v0.5.0",
    "    Finished `test` profile [unoptimized + debuginfo]",
    "",
    "running 12 tests",
    "test the_gallery_lists_every_component ... ok",
    "test every_variant_has_an_instance ... ok",
    "",
    "test result: ok. 12 passed; 0 failed",
    "$ ",
];

/// The default cell style: the terminal's own foreground on its own background, no flags.
const DEFAULT_STYLE: WireStyle = WireStyle {
    fg: WireColor::Named(FG),
    bg: WireColor::Named(BG),
    flags: 0,
    underline_color: None,
};

/// A green style, for the lines a test run would print in green.
const ACCENT_STYLE: WireStyle = WireStyle {
    fg: WireColor::Named(GREEN),
    bg: WireColor::Named(BG),
    flags: 0,
    underline_color: None,
};

/// Which style index a line is drawn in: the passing-test lines in green, the rest default.
fn style_for(line: &str) -> u16 {
    if line.ends_with("ok") || line.contains("passed") {
        1
    } else {
        0
    }
}

/// The fabricated terminal grid, built by applying one hand-written full snapshot.
///
/// A `GridCache` is what `TerminalPane` renders from, and it is the one piece of sample content that
/// cannot be a `const` — the cache resolves interned styles at apply time. It is still entirely
/// determined by the constants above: same frame in, same cache out, every launch.
pub fn grid() -> GridCache {
    let mut cache = GridCache::new();
    let lines: Vec<WireLine> = SCREEN
        .iter()
        .enumerate()
        .map(|(row, text)| {
            // Padded to the full width so every cell in the row is covered by a style run; the
            // resolver expects the run lengths to sum to the cell count.
            let padded = format!("{text:<width$}", width = COLS as usize);
            WireLine {
                id: LineId(row as i64),
                text: padded,
                runs: vec![StyleRun {
                    len: COLS,
                    style: style_for(text),
                }],
                extras: Vec::new(),
                wrapped: false,
            }
        })
        .collect();

    cache.apply(&GridFrame {
        session: sample_session(),
        seq: 1,
        generation: 0,
        full: true,
        viewport_top: LineId(0),
        oldest_available: LineId(0),
        cols: COLS,
        rows: SCREEN.len() as u16,
        // Parked at the prompt on the last line, visible and not blinking — a blinking cursor would
        // be an animation nobody asked for, and FR-023 says the page is inert at rest.
        cursor: WireCursor {
            line: LineId(SCREEN.len() as i64 - 1),
            col: 2,
            shape: WireCursorShape::Block,
            visible: true,
            blinking: false,
        },
        styles: vec![DEFAULT_STYLE, ACCENT_STYLE],
        hyperlinks: Vec::new(),
        lines,
        mode: 0,
        input_serial: None,
    });
    cache
}
