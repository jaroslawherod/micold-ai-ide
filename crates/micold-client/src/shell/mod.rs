//! The shell: everything that talks to something outside the process (feature 021, Tier "shell
//! split").
//!
//! The render-free core decides; the shell performs. Concretely, this is the one part of the
//! client allowed to name a real implementation of a service capability (FR-017) — the guard in
//! `tests/no_concrete_implementations.rs` has known about `shell/` since T041, so that rule was in
//! force before this directory existed.
//!
//! Declared from `main.rs` rather than `lib.rs`: the modules here (T050–T054) moved `boot`,
//! `main`, persistence, daemon synchronisation, the subscriptions, the environment-include
//! sourcing and the OS-theme probe out of the binary, and those operate on `App`, which is the
//! binary's type.
//!
//! One module per external system (FR-019a), which is why the theme is split across two of them:
//! `subscriptions` owns the clock that decides *when* to ask, `os_theme` owns the asking. They
//! are different systems — the iced runtime and the desktop environment — that happen to meet at
//! one message.

pub mod capabilities;
pub mod clipboard;
pub mod connection;
pub mod daemon_sync;
pub mod env_include;
pub mod legacy_units;
pub mod os_theme;
pub mod persist;
pub mod sandbox;
pub mod service_control;
pub mod settings;
pub mod startup;
pub mod subscriptions;
pub mod workspace;
