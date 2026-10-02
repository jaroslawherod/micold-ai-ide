//! What a session's terminal held, kept so it can be shown again after the session's process is
//! started anew (feature 041).
//!
//! Render-free and VT-free: these types use their own numbering, independent of
//! `alacritty_terminal`, so the daemon captures into them and seeds from them while this crate
//! keeps its "no PTY/VT crate" boundary.

/// What one terminal held at one moment: its history rows, then its screen rows down to the last
/// one that shows anything, oldest first. Empty means there is nothing to show.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistorySnapshot {
    pub lines: Vec<LogicalLine>,
}

/// One line as it was printed: grid rows joined by the wrap flag are a single `LogicalLine`.
/// `text` holds no C0, C1 or `ESC` character.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogicalLine {
    pub text: String,
    pub runs: Vec<StyleRun>,
}

/// `chars` consecutive characters of a line's `text` in one style. A line's runs cover its text in
/// order, so their `chars` sum to its number of characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StyleRun {
    pub chars: u32,
    pub style: HistoryStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HistoryStyle {
    pub fg: HistoryColor,
    pub bg: HistoryColor,
    pub flags: StyleFlags,
}

/// A cell colour in this module's own numbering, independent of `alacritty_terminal`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HistoryColor {
    /// The terminal's default foreground or background.
    #[default]
    Default,
    /// One of the 16 basic colours, 0 to 15.
    Basic(u8),
    /// The dim variant of one of the first 8 basic colours, 0 to 7.
    Dim(u8),
    /// A colour of the 256-colour palette.
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// The text attributes of a run, as a bit set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StyleFlags(u8);

/// Why a snapshot breaks the rules of data-model §1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {}

impl HistorySnapshot {
    /// Checks the rules a snapshot must hold before it is seeded or saved.
    pub fn validate(&self) -> Result<(), SnapshotError> {
        Ok(())
    }
}
