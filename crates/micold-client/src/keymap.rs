//! Pure key-encoding for the embedded terminal (feature 006).
//!
//! Adapted from `iced_term 0.6.0` `bindings.rs` (MIT © Ilya Shvyryalkin) — the standard
//! xterm/alacritty key map — but decoupled from iced so it is unit-testable under
//! `cargo test --no-default-features` (Constitution Principle I). The gui widget
//! (`src/ui/components/terminal_pane.rs`) maps iced key events onto [`KeyInput`], calls
//! [`encode`], and applies the resulting [`KeyOutput`].
//!
//! NOTE: `iced_term`'s table maps `Ctrl+U` to `0x51` ('Q') — a bug. The control-byte formula
//! here yields the correct `0x15`, and `is_focused` defaults to false in the widget (do not
//! copy iced_term's `true`). Contract: `specs/006-real-terminal-emulator/contracts/key-encoding.md`.

/// A keyboard modifier set (platform-neutral; `logo` = Super/Command).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub logo: bool,
}

impl Mods {
    /// No modifiers held.
    pub const NONE: Mods = Mods {
        shift: false,
        ctrl: false,
        alt: false,
        logo: false,
    };

    /// True when no modifier is held.
    pub const fn is_empty(self) -> bool {
        !self.shift && !self.ctrl && !self.alt && !self.logo
    }

    /// Only `ctrl` (no shift/alt/logo).
    pub const fn is_ctrl_only(self) -> bool {
        self.ctrl && !self.shift && !self.alt && !self.logo
    }
}

/// A named (non-character) key. `F(n)` covers function keys F1..=F20.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamedKey {
    Enter,
    Backspace,
    Tab,
    Escape,
    Space,
    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    F(u8),
}

/// A key, normalized from the iced key event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Key {
    /// A character key (already lowercased for chord matching by the caller is NOT required;
    /// [`encode`] lowercases where needed).
    Char(char),
    /// A named key.
    Named(NamedKey),
}

/// One normalized key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
    pub key: Key,
    pub mods: Mods,
    /// The printable text iced resolved for the press (used when no binding matches).
    pub text: Option<String>,
    /// The key's position on a US layout, whatever the active layout types (feature 803, FR-016).
    pub physical: PhysicalKey,
}

/// The physical keys the navigation chords sit on (feature 803, FR-016): the key's position, not
/// what the active layout types. `Other` for every other key, and when the platform gave no code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalKey {
    Comma,
    Period,
    BracketLeft,
    BracketRight,
    PageUp,
    PageDown,
    /// The `P` key (feature 968, the quick switcher chord).
    KeyP,
    Other,
}

/// The level a navigation chord steps through (feature 803, FR-001).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavLevel {
    /// The sessions of the current location.
    Session,
    /// The active project's locations (Default and its worktrees).
    Location,
    /// The open projects.
    Project,
}

/// A navigation chord's meaning: which level, which direction (feature 803).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavAction {
    pub level: NavLevel,
    pub dir: micold_core::navigation::Dir,
}

/// The navigation chords, the one place a chord is chosen (contracts/keys.md K1): each is its
/// physical key with the platform primary modifier plus Shift.
pub const NAV_CHORDS: [(NavAction, PhysicalKey); 6] = [
    (
        NavAction {
            level: NavLevel::Location,
            dir: micold_core::navigation::Dir::Previous,
        },
        PhysicalKey::Comma,
    ),
    (
        NavAction {
            level: NavLevel::Location,
            dir: micold_core::navigation::Dir::Next,
        },
        PhysicalKey::Period,
    ),
    (
        NavAction {
            level: NavLevel::Session,
            dir: micold_core::navigation::Dir::Previous,
        },
        PhysicalKey::BracketLeft,
    ),
    (
        NavAction {
            level: NavLevel::Session,
            dir: micold_core::navigation::Dir::Next,
        },
        PhysicalKey::BracketRight,
    ),
    // Provisional (ledger decision D4): the project chords sit on PageUp / PageDown.
    (
        NavAction {
            level: NavLevel::Project,
            dir: micold_core::navigation::Dir::Previous,
        },
        PhysicalKey::PageUp,
    ),
    (
        NavAction {
            level: NavLevel::Project,
            dir: micold_core::navigation::Dir::Next,
        },
        PhysicalKey::PageDown,
    ),
];

/// The chord of `action` as Linux/Windows and macOS write it (contracts/keys.md K4).
pub fn nav_chord_labels(action: NavAction) -> (String, String) {
    let key = NAV_CHORDS
        .iter()
        .find(|(a, _)| *a == action)
        .map_or("", |(_, physical)| physical_label(*physical));
    (format!("Ctrl+Shift+{key}"), format!("Cmd+Shift+{key}"))
}

/// How the user guide writes a navigation chord's key.
fn physical_label(physical: PhysicalKey) -> &'static str {
    match physical {
        PhysicalKey::Comma => ",",
        PhysicalKey::Period => ".",
        PhysicalKey::BracketLeft => "[",
        PhysicalKey::BracketRight => "]",
        PhysicalKey::PageUp => "PageUp",
        PhysicalKey::PageDown => "PageDown",
        PhysicalKey::KeyP => "P",
        PhysicalKey::Other => "",
    }
}

/// The physical key a logical key sits on in a US layout, for a platform that reported no code
/// (contracts/keys.md K2): the key unshifted or shifted.
fn logical_physical(key: &Key) -> PhysicalKey {
    match key {
        Key::Char('p' | 'P') => PhysicalKey::KeyP,
        Key::Char(',' | '<') => PhysicalKey::Comma,
        Key::Char('.' | '>') => PhysicalKey::Period,
        Key::Char('[' | '{') => PhysicalKey::BracketLeft,
        Key::Char(']' | '}') => PhysicalKey::BracketRight,
        Key::Named(NamedKey::PageUp) => PhysicalKey::PageUp,
        Key::Named(NamedKey::PageDown) => PhysicalKey::PageDown,
        _ => PhysicalKey::Other,
    }
}

/// The navigation chord the press is, if any (contracts/keys.md K1, K2): the platform primary
/// modifier plus Shift on a `NAV_CHORDS` key, by physical key, else by the logical fallback.
pub fn nav_chord(input: &KeyInput) -> Option<NavAction> {
    if !is_primary_shift(input.mods) {
        return None;
    }
    let physical = match input.physical {
        PhysicalKey::Other => logical_physical(&input.key),
        known => known,
    };
    NAV_CHORDS
        .iter()
        .find(|(_, key)| *key == physical && physical != PhysicalKey::Other)
        .map(|(action, _)| *action)
}

/// Whether the press is the quick switcher chord (feature 968, contracts/keys.md K5): the platform
/// primary modifier plus Shift on the physical `P` key, else the logical fallback. The one place
/// the chord is chosen.
pub fn is_switcher_chord(input: &KeyInput) -> bool {
    let physical = match input.physical {
        PhysicalKey::Other => logical_physical(&input.key),
        known => known,
    };
    is_primary_shift(input.mods) && physical == PhysicalKey::KeyP
}

/// The switcher chord as Linux/Windows and macOS write it (contracts/keys.md K5), for tooltips and
/// docs.
pub fn switcher_chord_labels() -> (String, String) {
    let key = physical_label(PhysicalKey::KeyP);
    (format!("Ctrl+Shift+{key}"), format!("Cmd+Shift+{key}"))
}

/// The subset of terminal mode that affects key encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TermMode {
    pub app_cursor: bool,
    pub alt_screen: bool,
}

/// A pane shortcut (feature 484, FR-009).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneAction {
    /// Split the focused pane side by side.
    SplitVertical,
    /// Split the focused pane stacked.
    SplitHorizontal,
    /// Close the focused pane (its session keeps running).
    Close,
    /// Move focus to the neighbouring pane.
    Focus(micold_core::pane_layout::Direction),
}

/// The decoded action for a key press.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOutput {
    /// Write these bytes to the PTY.
    Bytes(Vec<u8>),
    /// Copy the current selection to the clipboard.
    Copy,
    /// Paste clipboard text into the PTY.
    Paste,
    /// The reserved focus-out chord was pressed (never forwarded to the PTY).
    ReleaseFocus,
    /// The reserved "open a new Regular Terminal instance" chord was pressed (feature 011,
    /// FR-019; never forwarded to the PTY).
    NewTerminalInstance,
    /// The reserved Overview chord was pressed (feature 813, FR-011; never forwarded to the PTY).
    ToggleOverview,
    /// The quick switcher chord (feature 968; never forwarded to the PTY).
    ToggleSwitcher,
    /// A pane shortcut (feature 484; never forwarded to the PTY).
    Pane(PaneAction),
    /// A navigation chord (feature 803; never forwarded to the PTY).
    Navigate(NavAction),
    /// Not handled here — let the app/subscription see it.
    Ignore,
}

/// Decode a key press into a terminal action (total; never panics). See the module docs and the
/// key-encoding contract for the rules and precedence.
pub fn encode(input: &KeyInput, mode: TermMode) -> KeyOutput {
    // 1. Reserved focus-out chord (highest precedence; never forwarded).
    if is_release_chord(&input.key, input.mods) {
        return KeyOutput::ReleaseFocus;
    }
    // 1a. Reserved open-new-terminal-instance chord (feature 011; same precedence tier as the
    // focus-out chord above — never forwarded to the PTY either).
    if is_new_terminal_chord(&input.key, input.mods) {
        return KeyOutput::NewTerminalInstance;
    }
    // 1a''. Navigation chords (feature 803, contracts/keys.md K2; the same tier, never forwarded).
    if let Some(action) = nav_chord(input) {
        return KeyOutput::Navigate(action);
    }
    // 1a'''. The quick switcher chord (feature 968, contracts/keys.md K5; the same tier).
    if is_switcher_chord(input) {
        return KeyOutput::ToggleSwitcher;
    }
    // 1a'. Reserved Overview chord (feature 813, FR-011; the same tier, never forwarded).
    if is_overview_chord(&input.key, input.mods) {
        return KeyOutput::ToggleOverview;
    }
    // 1b. Pane chords (feature 484, FR-009; the same tier: never forwarded either).
    if let Some(action) = pane_action(&input.key, input.mods) {
        return KeyOutput::Pane(action);
    }
    // 2. Copy / paste chords.
    if let Some(action) = copy_paste_action(&input.key, input.mods) {
        return action;
    }
    // 3 & 4. Named keys and control chords.
    match &input.key {
        Key::Named(named) => match named_bytes(*named, input.mods, mode) {
            Some(bytes) => KeyOutput::Bytes(bytes),
            None => printable_or_ignore(input),
        },
        Key::Char(c) => {
            // Control chords on letters/symbols (ctrl held, not alt/logo).
            if input.mods.ctrl && !input.mods.alt && !input.mods.logo {
                if let Some(byte) = ctrl_byte(*c) {
                    return KeyOutput::Bytes(vec![byte]);
                }
            }
            printable_or_ignore(input)
        }
    }
}

/// Printable text when provided and no disqualifying modifier is held; else `Ignore`.
fn printable_or_ignore(input: &KeyInput) -> KeyOutput {
    // A logo/ctrl combo with no mapping is not printable input.
    if input.mods.ctrl || input.mods.logo {
        return KeyOutput::Ignore;
    }
    if let Some(text) = &input.text {
        if !text.is_empty() {
            return KeyOutput::Bytes(text.clone().into_bytes());
        }
    }
    if let Key::Char(c) = &input.key {
        // Alt+char → ESC-prefixed (meta); plain char → itself.
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        if input.mods.alt {
            let mut out = vec![0x1b];
            out.extend_from_slice(s.as_bytes());
            return KeyOutput::Bytes(out);
        }
        if input.mods.is_empty() || input.mods.shift {
            return KeyOutput::Bytes(s.as_bytes().to_vec());
        }
    }
    KeyOutput::Ignore
}

/// Whether the press is `letter` with the platform primary modifier and Shift (macOS `Cmd+Shift`,
/// else `Ctrl+Shift`): the shape of the reserved focus-out (`E`) and new-terminal (`T`) chords.
fn is_primary_shift_chord(key: &Key, mods: Mods, letter: char) -> bool {
    let is_letter = matches!(key, Key::Char(c) if c.eq_ignore_ascii_case(&letter));
    is_letter && is_primary_shift(mods)
}

/// Whether exactly the platform primary modifier and Shift are held (macOS `Cmd+Shift`, else
/// `Ctrl+Shift`; Alt and the other of Ctrl/logo disqualify): the shape of every reserved chord.
fn is_primary_shift(mods: Mods) -> bool {
    if !mods.shift {
        return false;
    }
    #[cfg(target_os = "macos")]
    {
        mods.logo && !mods.ctrl && !mods.alt
    }
    #[cfg(not(target_os = "macos"))]
    {
        mods.ctrl && !mods.logo && !mods.alt
    }
}

/// Whether the press is the reserved focus-out chord (macOS `Cmd+Shift+E`, else `Ctrl+Shift+E`).
fn is_release_chord(key: &Key, mods: Mods) -> bool {
    is_primary_shift_chord(key, mods, 'e')
}

/// Whether the press is the reserved "open a new Regular Terminal instance" chord (feature 011,
/// FR-019; macOS `Cmd+Shift+T`, else `Ctrl+Shift+T`; contracts/keyboard-shortcut.md).
fn is_new_terminal_chord(key: &Key, mods: Mods) -> bool {
    is_primary_shift_chord(key, mods, 't')
}

/// Whether the press is the reserved Overview chord (feature 813, FR-011, research R3; macOS
/// `Cmd+Shift+O`, else `Ctrl+Shift+O`). Public so the focused and unfocused paths share it.
pub fn is_overview_chord(key: &Key, mods: Mods) -> bool {
    is_primary_shift_chord(key, mods, 'o')
}

/// The Overview chord as the platform writes it, for tooltips and docs.
pub fn overview_chord_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd+Shift+O"
    } else {
        "Ctrl+Shift+O"
    }
}

/// Whether the press is the Overview's full-screen chord (feature 970, FR-001, research R7; macOS
/// `Cmd+Shift+F`, else `Ctrl+Shift+F`). Not in the terminal's reserved tier: the app hears it only
/// while the terminal is unfocused, which the open Overview always is.
pub fn is_overview_full_screen_chord(key: &Key, mods: Mods) -> bool {
    is_primary_shift_chord(key, mods, 'f')
}

/// The full-screen chord as the platform writes it, for the header's tooltip and docs.
pub fn overview_full_screen_chord_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "Cmd+Shift+F"
    } else {
        "Ctrl+Shift+F"
    }
}

/// Pane chords (macOS `Cmd+Shift+D/H/W/Arrow`, else `Ctrl+Shift+D/H/W/Arrow`), built like
/// [`is_release_chord`] (contracts/keybindings.md).
fn pane_action(key: &Key, mods: Mods) -> Option<PaneAction> {
    use micold_core::pane_layout::Direction;
    if !mods.shift {
        return None;
    }
    #[cfg(target_os = "macos")]
    let platform = mods.logo && !mods.ctrl && !mods.alt;
    #[cfg(not(target_os = "macos"))]
    let platform = mods.ctrl && !mods.logo && !mods.alt;
    if !platform {
        return None;
    }
    match key {
        Key::Char(c) => match c.to_ascii_lowercase() {
            'd' => Some(PaneAction::SplitVertical),
            'h' => Some(PaneAction::SplitHorizontal),
            'w' => Some(PaneAction::Close),
            _ => None,
        },
        Key::Named(NamedKey::ArrowLeft) => Some(PaneAction::Focus(Direction::Left)),
        Key::Named(NamedKey::ArrowRight) => Some(PaneAction::Focus(Direction::Right)),
        Key::Named(NamedKey::ArrowUp) => Some(PaneAction::Focus(Direction::Up)),
        Key::Named(NamedKey::ArrowDown) => Some(PaneAction::Focus(Direction::Down)),
        Key::Named(_) => None,
    }
}

/// Copy/paste chords (macOS `Cmd+C`/`Cmd+V`, else `Ctrl+Shift+C`/`Ctrl+Shift+V`).
fn copy_paste_action(key: &Key, mods: Mods) -> Option<KeyOutput> {
    let ch = match key {
        Key::Char(c) => c.to_ascii_lowercase(),
        _ => return None,
    };
    #[cfg(target_os = "macos")]
    let matches = mods.logo && !mods.ctrl && !mods.alt && !mods.shift;
    #[cfg(not(target_os = "macos"))]
    let matches = mods.ctrl && mods.shift && !mods.alt && !mods.logo;
    if !matches {
        return None;
    }
    match ch {
        'c' => Some(KeyOutput::Copy),
        'v' => Some(KeyOutput::Paste),
        _ => None,
    }
}

/// Opens a bracketed paste (`DECSET 2004`).
const PASTE_START: &str = "\x1b[200~";
/// Closes a bracketed paste.
const PASTE_END: &str = "\x1b[201~";

/// The bytes a paste of `text` sends to the process (FR-013d) — shared by every paste gesture, so
/// the chord, middle-click and the context menu cannot disagree.
///
/// When the process has enabled bracketed paste, the text goes out as one `ESC[200~ … ESC[201~`
/// block, which a shell or editor inserts rather than executes line by line. Any end marker inside
/// the text is removed first — repeatedly, since removing one can join its neighbours into another —
/// or pasted text could close the block early and run the rest as keystrokes.
///
/// Only the end marker is removed, not every `ESC`: the block is the process's to interpret, and a
/// narrower rule keeps the text the user copied intact. When bracketing is off the text is sent
/// exactly as pasted; there is no block for a marker to close.
pub fn paste_bytes(text: &str, bracketed: bool) -> Vec<u8> {
    if !bracketed {
        return text.as_bytes().to_vec();
    }
    let mut body = text.to_string();
    while body.contains(PASTE_END) {
        body = body.replace(PASTE_END, "");
    }
    [PASTE_START, &body, PASTE_END].concat().into_bytes()
}

/// The control byte for `Ctrl+<char>`, or `None` if the char has no control encoding.
/// `Ctrl+A`..`Ctrl+Z` → `0x01`..`0x1a` (so `Ctrl+U` == `0x15`, fixing iced_term's `0x51` bug).
fn ctrl_byte(c: char) -> Option<u8> {
    let lc = c.to_ascii_lowercase();
    if lc.is_ascii_alphabetic() {
        return Some(0x01 + (lc as u8 - b'a'));
    }
    match c {
        ' ' | '2' => Some(0x00),
        '[' => Some(0x1b),
        '\\' => Some(0x1c),
        ']' => Some(0x1d),
        '6' => Some(0x1e),
        '-' | '_' | '/' => Some(0x1f),
        _ => None,
    }
}

/// Escape/byte sequence for a named key, honoring modifiers and cursor-key mode. `None` means
/// the key has no encoding for the given modifiers (caller falls back to printable/ignore).
fn named_bytes(named: NamedKey, mods: Mods, mode: TermMode) -> Option<Vec<u8>> {
    let seq: Option<String> = match named {
        NamedKey::Enter => Some("\r".into()),
        NamedKey::Backspace => {
            if mods.alt {
                Some("\x1b\x7f".into())
            } else {
                Some("\x7f".into())
            }
        }
        NamedKey::Tab => {
            if mods.shift {
                Some("\x1b[Z".into())
            } else {
                Some("\t".into())
            }
        }
        NamedKey::Escape => Some("\x1b".into()),
        NamedKey::Space => Some(" ".into()),
        NamedKey::Insert => Some("\x1b[2~".into()),
        NamedKey::Delete => Some(csi_tilde(3, mods)),
        NamedKey::PageUp => Some(csi_tilde(5, mods)),
        NamedKey::PageDown => Some(csi_tilde(6, mods)),
        NamedKey::Home => Some(csi_letter('H', mods, mode)),
        NamedKey::End => Some(csi_letter('F', mods, mode)),
        NamedKey::ArrowUp => Some(csi_letter('A', mods, mode)),
        NamedKey::ArrowDown => Some(csi_letter('B', mods, mode)),
        NamedKey::ArrowRight => Some(csi_letter('C', mods, mode)),
        NamedKey::ArrowLeft => Some(csi_letter('D', mods, mode)),
        NamedKey::F(n) => function_key(n, mods),
    };
    seq.map(String::into_bytes)
}

/// The xterm modifier parameter (`1;<n>`): 2=Shift, 3=Alt, 5=Ctrl, and combinations.
fn modifier_param(mods: Mods) -> Option<u8> {
    let mut n = 1u8;
    if mods.shift {
        n += 1;
    }
    if mods.alt {
        n += 2;
    }
    if mods.ctrl {
        n += 4;
    }
    if n == 1 {
        None
    } else {
        Some(n)
    }
}

/// CSI sequences ending in a letter (arrows/Home/End). Uses `SS3` (`ESC O`) in cursor-key mode
/// for the unmodified form, `CSI 1;<mod> <letter>` when modified, else `CSI <letter>`.
fn csi_letter(letter: char, mods: Mods, mode: TermMode) -> String {
    if let Some(m) = modifier_param(mods) {
        return format!("\x1b[1;{m}{letter}");
    }
    if mode.app_cursor {
        format!("\x1bO{letter}")
    } else {
        format!("\x1b[{letter}")
    }
}

/// CSI `~` sequences (Insert/Delete/PageUp/PageDown): `CSI <n>~` or `CSI <n>;<mod>~`.
fn csi_tilde(n: u8, mods: Mods) -> String {
    match modifier_param(mods) {
        Some(m) => format!("\x1b[{n};{m}~"),
        None => format!("\x1b[{n}~"),
    }
}

/// Base function-key sequences (unmodified). F1..F4 use SS3; F5.. use CSI `~`.
fn function_key(n: u8, _mods: Mods) -> Option<String> {
    let s = match n {
        1 => "\x1bOP",
        2 => "\x1bOQ",
        3 => "\x1bOR",
        4 => "\x1bOS",
        5 => "\x1b[15~",
        6 => "\x1b[17~",
        7 => "\x1b[18~",
        8 => "\x1b[19~",
        9 => "\x1b[20~",
        10 => "\x1b[21~",
        11 => "\x1b[23~",
        12 => "\x1b[24~",
        13 => "\x1b[25~",
        14 => "\x1b[26~",
        15 => "\x1b[28~",
        16 => "\x1b[29~",
        17 => "\x1b[31~",
        18 => "\x1b[32~",
        19 => "\x1b[33~",
        20 => "\x1b[34~",
        _ => return None,
    };
    Some(s.to_string())
}
