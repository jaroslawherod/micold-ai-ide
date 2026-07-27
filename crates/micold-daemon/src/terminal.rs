//! Daemon-owned VT emulation (plan W3, tasks T030/T034/T006).
//!
//! The daemon owns every session's [`Term`] — the alacritty VT/ANSI emulator — so the interpreted
//! grid lives with the process, not the UI (FR-001). The client is a thin renderer that receives
//! grid frames; it never holds a `Term`. This module provides:
//!
//! - [`DaemonListener`] — the [`EventListener`] that answers VT queries (`PtyWrite`, `ColorRequest`,
//!   `TextAreaSizeRequest`) **daemon-side**, never routing them to the client (protocol.md §8,
//!   T034); it also captures the OSC-0 title (`Event::Title`, T047) and raises the framer's dirty
//!   flag on new content.
//! - [`LineIdSource`] — the stable per-line identity the shadow-diff framer keys on (Decision 2,
//!   T006). The default [`ApproxLineIds`] is the no-fork approximation; the vendored VT patch can
//!   swap in behind the same trait without touching the framer.
//! - [`SharedTerm`] — the `Arc<FairMutex<Term<DaemonListener>>>` handle shared between the PTY
//!   reader thread (which advances the parser) and the framer (which reads the grid).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener, WindowSize};
use alacritty_terminal::sync::FairMutex;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::Rgb;
use micold_core::protocol::grid::LineId;

use std::io::Write;

/// The VT emulator behind a fair mutex, shared between the reader thread and the framer. `Term` is
/// not `Sync` for concurrent mutation, so every access takes the lock; `FairMutex` (alacritty's own)
/// keeps the reader thread from starving the framer under a firehose of output.
pub type SharedTerm = Arc<FairMutex<Term<DaemonListener>>>;

/// A boxed PTY writer, shared between the user-input path and the [`DaemonListener`]'s VT replies —
/// both write to the one PTY master, so both hold this.
pub type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

/// Signals the framer and supervisor read out-of-band from the VT stream: a dirty flag raised on
/// every content change (depth-1, cleared by the framer each tick — T032), the latest OSC-0 title
/// (T047), a rung-bell edge, and the child's reported exit code (`Event::ChildExit`).
#[derive(Clone, Default)]
pub struct VtSignals {
    dirty: Arc<AtomicBool>,
    bell: Arc<AtomicBool>,
    /// A braille-spinner glyph was seen on an OSC-0 title since the last drain — positive evidence
    /// of `Working` (activity invariant H1a, T046). Detected from the **raw** title before the glyph
    /// is stripped for display, so it is captured here as an edge rather than recovered from the
    /// (stripped) [`Self::title`].
    spinner: Arc<AtomicBool>,
    title: Arc<Mutex<Option<String>>>,
    child_exit: Arc<Mutex<Option<i32>>>,
}

impl VtSignals {
    /// Take and clear the dirty flag — true when content changed since the last call (framer tick).
    pub fn take_dirty(&self) -> bool {
        self.dirty.swap(false, Ordering::AcqRel)
    }

    /// Raise the dirty flag (used on attach to force the next tick to snapshot).
    pub fn mark_dirty(&self) {
        self.dirty.store(true, Ordering::Release);
    }

    /// Take and clear the bell edge.
    pub fn take_bell(&self) -> bool {
        self.bell.swap(false, Ordering::AcqRel)
    }

    /// Take and clear the spinner edge — true when a braille-spinner title was seen since the last
    /// call. The activity drain feeds this to the FSM as `SpinnerObserved` (Working-only, H1a).
    pub fn take_spinner(&self) -> bool {
        self.spinner.swap(false, Ordering::AcqRel)
    }

    /// The most recent OSC-0 title reported by the process, if any (already glyph-stripped — T047).
    pub fn title(&self) -> Option<String> {
        self.title.lock().expect("title lock poisoned").clone()
    }

    /// The child's VT-reported exit code, if the emulator saw `ChildExit` **and** the child exited
    /// with a code. Authoritative child liveness is the PTY `wait()` in the supervisor; this is the
    /// in-band signal.
    ///
    /// `None` is therefore ambiguous — no `ChildExit` seen, *or* seen but signal-terminated
    /// (`ExitStatus::code()` is `None` on Unix when a signal killed the process). That ambiguity is
    /// acceptable precisely because this value is informational: the supervisor's `wait()` decides
    /// restart policy (FR-005/FR-022), never this.
    pub fn child_exit(&self) -> Option<i32> {
        *self.child_exit.lock().expect("exit lock poisoned")
    }
}

/// The [`EventListener`] installed in every daemon `Term`. VT control replies are answered here and
/// written straight back to the PTY — they are terminal-protocol plumbing, never client concerns
/// (T034). Terminal *content* reaches the client only as grid frames.
#[derive(Clone)]
pub struct DaemonListener {
    writer: SharedWriter,
    size: Arc<Mutex<WindowSize>>,
    palette: StandardPalette,
    signals: VtSignals,
}

impl DaemonListener {
    /// Build a listener sharing `writer` with the input path and `size` with the resize path.
    pub fn new(writer: SharedWriter, size: Arc<Mutex<WindowSize>>, signals: VtSignals) -> Self {
        Self {
            writer,
            size,
            palette: StandardPalette::xterm(),
            signals,
        }
    }

    fn reply(&self, bytes: &[u8]) {
        if let Ok(mut w) = self.writer.lock() {
            // A failed VT reply is not fatal — the child may have exited mid-query.
            let _ = w.write_all(bytes);
            let _ = w.flush();
        }
    }
}

impl EventListener for DaemonListener {
    fn send_event(&self, event: Event) {
        match event {
            // VT control replies — answered here, never forwarded (protocol.md §8, T034).
            Event::PtyWrite(text) => self.reply(text.as_bytes()),
            Event::ColorRequest(index, format) => {
                let rgb = self.palette.color(index);
                self.reply(format(rgb).as_bytes());
            }
            Event::TextAreaSizeRequest(format) => {
                let size = *self.size.lock().expect("size lock poisoned");
                self.reply(format(size).as_bytes());
            }
            // OSC-0 title (T047): captured for the catalog, stripped of a leading status glyph, and
            // treated as untrusted text. `ResetTitle` clears it. A braille-spinner glyph in the *raw*
            // title is Working-only activity evidence (T046, H1a) — captured as an edge before it is
            // stripped, since the stored (stripped) title no longer carries the glyph.
            Event::Title(title) => {
                if crate::activity::is_spinner_title(&title) {
                    self.signals.spinner.store(true, Ordering::Release);
                }
                *self.signals.title.lock().expect("title lock poisoned") =
                    Some(strip_status_glyph(&title));
            }
            Event::ResetTitle => {
                *self.signals.title.lock().expect("title lock poisoned") = None;
            }
            // In-band child exit; the supervisor's `wait()` is authoritative for liveness.
            Event::ChildExit(code) => {
                // `alacritty_terminal` 0.26 widened this from a bare `i32` to `ExitStatus` (T105).
                // We keep storing the code: it is informational only, and `wait()` in the
                // supervisor — not this — is what decides restart policy. A signal-terminated child
                // has no code and stores `None`; see `VtSignals::child_exit`.
                *self.signals.child_exit.lock().expect("exit lock poisoned") = code.code();
            }
            Event::Bell => self.signals.bell.store(true, Ordering::Release),
            // Any new content / cursor movement dirties the frame (depth-1 — T032).
            Event::Wakeup | Event::MouseCursorDirty | Event::CursorBlinkingChange => {
                self.signals.dirty.store(true, Ordering::Release);
            }
            // Clipboard, exit-request and the rest are not daemon concerns here.
            _ => {}
        }
    }
}

/// Drop a single leading terminal status glyph from an OSC-0 title (T047). Agents prefix the title
/// with a spinner/activity glyph (a Private-Use-Area or symbol codepoint); it is presentation, not
/// name, and — per invariant H1a — must never be read as an activity signal. Only a *leading*
/// non-alphanumeric symbol followed by a space is stripped, so ordinary titles are untouched.
pub fn strip_status_glyph(title: &str) -> String {
    let mut chars = title.chars();
    if let Some(first) = chars.clone().next() {
        let is_symbol =
            !first.is_alphanumeric() && !first.is_ascii_punctuation() && !first.is_whitespace();
        if is_symbol {
            chars.next();
            return chars.as_str().trim_start().to_string();
        }
    }
    title.to_string()
}

/// Stable per-line identity for the shadow-diff framer (Decision 2, T006). A logical line keeps its
/// id as it scrolls up into history, so the diff can say "this id is unchanged" instead of
/// re-sending every line on every scroll — the load-bearing ~11× reduction (T035).
pub trait LineIdSource {
    /// The stable id of the buffer line `offset_from_top` rows below the oldest retained line
    /// (0 = oldest retained line in the whole buffer, history included).
    fn line_id(&self, offset_from_top: usize) -> LineId;

    /// The id of the oldest line the daemon still retains — the `oldest_available` watermark the
    /// client needs to know how far back scrollback goes (sent on every frame, T032).
    fn oldest_available(&self) -> LineId;
}

/// The no-fork approximation (plan Decision 2 mitigation). Without the vendored VT patch, a line's
/// identity is derived from a monotonic count of how many lines have ever been produced: the oldest
/// retained line is `total_lines - retained`, and each line below it is one greater. The framer
/// advances [`Self::observe`] every tick with the current total so ids stay stable as content
/// scrolls off the top.
#[derive(Debug, Clone, Default)]
pub struct ApproxLineIds {
    /// Total logical lines ever produced (monotonic; only grows).
    total_lines: i64,
    /// How many of those lines are still retained in the buffer (history + screen).
    retained: i64,
}

impl ApproxLineIds {
    /// Record the current buffer shape: `total_lines` ever produced and how many are `retained`.
    /// `total_lines` is a monotonic watermark — a smaller value (which should never happen) is
    /// clamped up rather than allowed to regress ids, so a miscounting caller degrades to a stale
    /// watermark instead of corrupting line identity.
    pub fn observe(&mut self, total_lines: i64, retained: i64) {
        self.total_lines = total_lines.max(self.total_lines);
        self.retained = retained.clamp(0, self.total_lines);
    }
}

impl LineIdSource for ApproxLineIds {
    fn line_id(&self, offset_from_top: usize) -> LineId {
        LineId(self.oldest_available().0 + offset_from_top as i64)
    }

    fn oldest_available(&self) -> LineId {
        LineId(self.total_lines - self.retained)
    }
}

/// A fixed xterm-compatible 256-colour palette used to answer VT `ColorRequest` queries daemon-side
/// (T034). The daemon does not render, but a program may *ask* the terminal for a palette entry; a
/// stable, conventional answer keeps VT programs correct without involving the client's theme.
#[derive(Debug, Clone)]
pub struct StandardPalette {
    entries: [Rgb; 256],
}

impl StandardPalette {
    /// Build the standard xterm 256-colour cube: 16 system colours, a 6×6×6 colour cube, and a
    /// 24-step grey ramp (the universally-assumed layout).
    pub fn xterm() -> Self {
        let mut entries = [Rgb { r: 0, g: 0, b: 0 }; 256];

        // 0..16 — the classic system palette.
        const SYSTEM: [(u8, u8, u8); 16] = [
            (0, 0, 0),
            (205, 0, 0),
            (0, 205, 0),
            (205, 205, 0),
            (0, 0, 238),
            (205, 0, 205),
            (0, 205, 205),
            (229, 229, 229),
            (127, 127, 127),
            (255, 0, 0),
            (0, 255, 0),
            (255, 255, 0),
            (92, 92, 255),
            (255, 0, 255),
            (0, 255, 255),
            (255, 255, 255),
        ];
        for (i, (r, g, b)) in SYSTEM.iter().enumerate() {
            entries[i] = Rgb {
                r: *r,
                g: *g,
                b: *b,
            };
        }

        // 16..232 — 6×6×6 colour cube.
        let level = |v: u8| -> u8 {
            if v == 0 {
                0
            } else {
                55 + v * 40
            }
        };
        let mut idx = 16;
        for r in 0..6u8 {
            for g in 0..6u8 {
                for b in 0..6u8 {
                    entries[idx] = Rgb {
                        r: level(r),
                        g: level(g),
                        b: level(b),
                    };
                    idx += 1;
                }
            }
        }

        // 232..256 — 24-step grey ramp.
        for i in 0..24u8 {
            let v = 8 + i * 10;
            entries[232 + i as usize] = Rgb { r: v, g: v, b: v };
        }

        Self { entries }
    }

    /// The colour for a palette index; indices ≥ 256 fold back to the default foreground (index 7),
    /// so an out-of-range query still gets a sane answer rather than a panic.
    pub fn color(&self, index: usize) -> Rgb {
        *self.entries.get(index).unwrap_or(&self.entries[7])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_ids_are_stable_as_lines_scroll_off() {
        // A buffer of 100 retained lines out of 100 produced: oldest id is 0.
        let mut ids = ApproxLineIds::default();
        ids.observe(100, 100);
        assert_eq!(ids.oldest_available(), LineId(0));
        let id_of_row_10 = ids.line_id(10);
        assert_eq!(id_of_row_10, LineId(10));

        // 50 more lines produced, buffer still caps at 100 retained: the first 50 scrolled off.
        ids.observe(150, 100);
        assert_eq!(ids.oldest_available(), LineId(50));
        // The line that was row 60 (id 60) is now row 10 — SAME id. That stability is the point.
        assert_eq!(ids.line_id(10), LineId(60));
    }

    #[test]
    fn total_lines_is_monotonic() {
        let mut ids = ApproxLineIds::default();
        ids.observe(100, 100);
        // A smaller total (should never happen) is clamped up, never regressing ids.
        ids.observe(80, 80);
        assert_eq!(ids.oldest_available().0 + ids.retained, 100);
    }

    #[test]
    fn strips_only_a_leading_status_glyph() {
        // A leading PUA/symbol glyph + space is stripped.
        assert_eq!(strip_status_glyph("\u{2726} Fixing login"), "Fixing login");
        // An ordinary title is untouched.
        assert_eq!(strip_status_glyph("Fixing login"), "Fixing login");
        // Punctuation-led titles (e.g. a path) are not treated as status glyphs.
        assert_eq!(strip_status_glyph("~/code: build"), "~/code: build");
        // Empty stays empty.
        assert_eq!(strip_status_glyph(""), "");
    }

    #[test]
    fn palette_has_conventional_anchors() {
        let p = StandardPalette::xterm();
        assert_eq!(p.color(0), Rgb { r: 0, g: 0, b: 0 }, "index 0 is black");
        assert_eq!(
            p.color(15),
            Rgb {
                r: 255,
                g: 255,
                b: 255
            },
            "index 15 is bright white"
        );
        // Out-of-range folds back to a sane default rather than panicking.
        let _ = p.color(999);
    }
}
