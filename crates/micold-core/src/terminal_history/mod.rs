//! What a session's terminal held, kept so it can be shown again after the session's process is
//! started anew (feature 041).
//!
//! Render-free and VT-free: these types use their own numbering, independent of
//! `alacritty_terminal`, so the daemon captures into them and seeds from them while this crate
//! keeps its "no PTY/VT crate" boundary.
