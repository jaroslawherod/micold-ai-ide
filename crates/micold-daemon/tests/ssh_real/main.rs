//! The daemon's `ssh_real` integration tests, linked as one test binary (#1175): one module per former test file.
#[cfg(all(feature = "ssh-real-runtime", unix))]
#[path = "../ssh_real_support/mod.rs"]
mod ssh_real_support;

#[cfg(all(feature = "ssh-real-runtime", unix))]
mod ssh_real_concurrent;
#[cfg(all(feature = "ssh-real-runtime", unix))]
mod ssh_real_endtoend;
#[cfg(all(feature = "ssh-real-runtime", unix))]
mod ssh_real_hostkey_auth;
#[cfg(all(feature = "ssh-real-runtime", unix))]
mod ssh_real_mismatch;
#[cfg(all(feature = "ssh-real-runtime", unix))]
mod ssh_real_reconnect;
