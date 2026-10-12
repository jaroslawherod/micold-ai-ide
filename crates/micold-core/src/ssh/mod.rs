//! Reaching a session daemon on a remote host over the user's own `ssh` (feature 489).
//!
//! Everything decidable without I/O is a pure function here, so it is testable with no `ssh`, no
//! network and no runtime (Constitution Principle I): the target and its validation, the command
//! lines, the classification of a failure. The one I/O sequence (probe, copy, start) runs over an
//! injected [`provision::SshRunner`].

pub mod argv;
pub mod classify;
pub mod provision;
pub mod stdio;
pub mod system;
pub mod target;

pub use argv::{ClientPlatform, SshCommand};
pub use classify::{ReasonContext, SshFailure};
pub use provision::{Artefacts, ProvisionProgress, SshHost, SshOutput, SshRunner};
pub use stdio::SshStdio;
pub use system::SystemSsh;
pub use target::{SshSettings, SshTarget};
