//! The scoped git credential of a sandboxed daemon (#1002): what git inside the sandbox may use,
//! and what it may never reach.
//!
//! This part (FR-004) keeps the host's git configuration from carrying a credential into the
//! sandbox: [`gitconfig::sanitize_gitconfig`] strips a shared `~/.gitconfig`, and
//! [`git_env::git_env`] resets every credential helper that is left.

pub mod git_env;
pub mod gitconfig;
pub mod helper;

pub use git_env::git_env;
pub use gitconfig::{parse_config_list, sanitize_gitconfig, sanitize_gitconfig_text};

/// Seconds since the Unix epoch, as the credential files and the protocol carry them.
pub type UnixTime = u64;

/// The longest a scoped credential may live (FR-009): the hour GitHub gives an installation token.
pub const MAX_LIFETIME_SECS: u64 = 3600;

/// The forge the scoped credential is for, as git names it in `credential.<base>.*`.
pub const GITHUB_BASE: &str = "https://github.com";
