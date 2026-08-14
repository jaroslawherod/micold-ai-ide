//! The shell: everything that talks to something outside the process (feature 021, Tier "shell
//! split").
//!
//! The render-free core decides; the shell performs. Concretely, this is the one part of the
//! client allowed to name a real implementation of a service capability (FR-017) — the guard in
//! `tests/no_concrete_implementations.rs` has known about `shell/` since T041, so that rule was in
//! force before this directory existed.
//!
//! Declared from `main.rs` rather than `lib.rs`: the modules that follow (T050–T054) move `boot`,
//! `main`, persistence, daemon synchronisation and the subscriptions out of the binary, and those
//! operate on `App`, which is the binary's type.

pub mod capabilities;
pub mod daemon_sync;
pub mod persist;
pub mod startup;
pub mod subscriptions;
