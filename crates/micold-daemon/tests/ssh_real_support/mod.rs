//! Shared harness for the `ssh_real_*` targets (feature 489): a rootless `sshd` on loopback that
//! logs in as the current user, so the client's own `ssh` reaches a stand-in remote host.
//!
//! Each target is its own test binary, so this is included by `mod`; `dead_code` is allowed because
//! no single binary uses all of it.

#![allow(dead_code)]

pub mod proxy;
pub mod remote;
pub mod sshd;

pub use sshd::Sshd;

/// Start the stand-in host, or print why this machine cannot and return `None` (a skip, never a
/// failure: a machine without `sshd` cannot run this suite).
pub fn host_or_skip(test: &str) -> Option<Sshd> {
    match Sshd::start() {
        Ok(host) => Some(host),
        Err(reason) => {
            eprintln!("SKIP {test}: {reason}");
            None
        }
    }
}
