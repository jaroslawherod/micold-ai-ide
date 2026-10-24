//! The daemon's `ssh_real` integration tests, linked as one test binary (#1175): one module per former test file.
#[path = "../ssh_real_support/mod.rs"]
mod ssh_real_support;

mod ssh_real_concurrent;
mod ssh_real_endtoend;
mod ssh_real_hostkey_auth;
mod ssh_real_mismatch;
mod ssh_real_reconnect;
