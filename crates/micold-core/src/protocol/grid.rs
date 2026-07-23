//! Wire grid types (contracts/messages.md §Grid stream, data-model.md §LineId).
//!
//! These carry the ~15× size win of the streaming design, and the representation rules below are
//! load-bearing, not polish:
//!
//! 1. Text is one `String`, style is RLE [`StyleRun`]s. Per-cell structs are ~15× larger.
//! 2. Styles are interned **per frame** (`u16` index into [`GridFrame::styles`]). Typically < 8
//!    distinct styles per frame. Never intern across frames — that couples both ends' state and
//!    breaks resnapshot-on-attach.
//! 3. Rare per-cell data (`zerowidth`, `underline_color`, `hyperlink`) is hoisted into sparse side
//!    tables so the common case pays nothing.
//! 4. `Flags::bits()` and `TermMode::bits()` ship as raw integers, no translation.
//!
//! **Postcard note:** `#[serde(skip_serializing_if)]` is deliberately *not* used here even though the
//! contract mentions it for the JSON view. The same type serializes under both JSON and `postcard`
//! (`MICOLD_WIRE=json`), and `postcard` is not self-describing — a skipped field would desynchronise
//! the decoder. The structural sparseness (empty `Vec`s cost one length byte, `None` costs one tag
//! byte) already delivers the win while keeping the byte-identical round-trip the contract requires.

use serde::{Deserialize, Serialize};

use crate::session::SessionId;

/// Absolute, stable line identity: `scrolled_total + history_size + line` (data-model §LineId).
///
/// Monotonic over a session's lifetime and never reused. A line's ID never changes once assigned,
/// even as the viewport moves (I1), and history lines are immutable once scrolled off (I2) — which
/// is what makes client-side caching sound and scrollback requests idempotent.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct LineId(pub i64);

/// A color as alacritty models it — named (16-color + specials), 256-indexed, or true color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WireColor {
    /// A named ANSI color, by alacritty `NamedColor` discriminant.
    Named(u8),
    /// A 256-color palette index.
    Indexed(u8),
    /// 24-bit true color.
    Rgb(u8, u8, u8),
}

/// One interned style (contracts/messages.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WireStyle {
    /// Foreground color.
    pub fg: WireColor,
    /// Background color.
    pub bg: WireColor,
    /// `alacritty_terminal::term::cell::Flags::bits()` verbatim — no translation.
    pub flags: u16,
    /// Underline color, when it differs from `fg`.
    pub underline_color: Option<WireColor>,
}

/// A run-length-encoded span of cells sharing one style. `sum(len)` over a line's runs equals the
/// cell count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleRun {
    /// Number of consecutive cells this run covers.
    pub len: u16,
    /// Index into the frame's [`GridFrame::styles`] palette.
    pub style: u16,
}

/// Sparse per-cell data hoisted out of the hot path (mirrors alacritty's `Cell::extra`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellExtras {
    /// The cell column this applies to.
    pub col: u16,
    /// Combining marks / ZWJ sequences attached to the cell. Dropping these silently mangles emoji.
    pub zerowidth: Vec<char>,
    /// Index into [`GridFrame::hyperlinks`], when the cell is part of a hyperlink.
    pub hyperlink: Option<u16>,
}

/// One line of the grid, keyed by its stable [`LineId`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireLine {
    /// Stable absolute identity.
    pub id: LineId,
    /// One `char` per **cell**. A wide char's trailing cell keeps a spacer sentinel so the column
    /// alignment the daemon already computed survives the wire (messages.md §Wide characters).
    pub text: String,
    /// RLE style runs; `sum(len) == ` cell count.
    pub runs: Vec<StyleRun>,
    /// Usually empty — zerowidth marks / hyperlinks only.
    pub extras: Vec<CellExtras>,
    /// `WRAPLINE` set on the last cell (the line soft-wraps into the next).
    pub wrapped: bool,
}

/// Cursor shape. Own enum because vte's `CursorShape` does not derive `Serialize`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WireCursorShape {
    /// Filled block.
    Block,
    /// Underline bar.
    Underline,
    /// Vertical beam.
    Beam,
    /// Hollow block (unfocused).
    HollowBlock,
    /// No cursor drawn.
    Hidden,
}

/// The cursor, anchored to a [`LineId`] so it survives scrolling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireCursor {
    /// The line the cursor sits on.
    pub line: LineId,
    /// The cursor column.
    pub col: u16,
    /// Its shape.
    pub shape: WireCursorShape,
    /// Whether it is currently drawn.
    pub visible: bool,
    /// Whether it blinks.
    pub blinking: bool,
}

/// A grid frame — a full snapshot (`full = true`) or a stable-`LineId`-keyed delta (`full = false`).
/// Sent on the transport under envelope `kind = 1`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GridFrame {
    /// Which session this frame belongs to.
    pub session: SessionId,
    /// Monotonic per-session frame sequence.
    pub seq: u64,
    /// Bumped on resize / alt-screen enter-exit / reset — a change forces a resnapshot because
    /// line identities change and diffing is meaningless (data-model F4).
    pub generation: u64,
    /// `true` = full snapshot; `false` = delta carrying only changed lines.
    pub full: bool,
    /// The `LineId` at the top of the viewport.
    pub viewport_top: LineId,
    /// The trim watermark, present on **every** frame so the client can evict and clamp without
    /// asking (data-model I3).
    pub oldest_available: LineId,
    /// Viewport width in cells.
    pub cols: u16,
    /// Viewport height in cells.
    pub rows: u16,
    /// The cursor.
    pub cursor: WireCursor,
    /// The per-frame interned style palette (rule 2).
    pub styles: Vec<WireStyle>,
    /// The per-frame interned hyperlink URIs.
    pub hyperlinks: Vec<String>,
    /// All lines if `full`, changed lines only if a delta.
    pub lines: Vec<WireLine>,
    /// `TermMode::bits()` — raw, no translation.
    pub mode: u32,
    /// Echo of the last applied input serial, for local-echo correlation.
    pub input_serial: Option<u64>,
}
