//! What a Kubernetes daemon may take from this machine (feature 686, FR-008). Each allowed entry
//! becomes exactly one secret and one mount at start (US3); nothing is copied unless listed.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::session::AiCli;

/// One credential the user allowed a Kubernetes daemon to receive.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KubeCredential {
    /// `~/.gitconfig`.
    GitConfig,
    /// `~/.git-credentials`.
    GitCredentials,
    /// One SSH private key.
    GitSshKey {
        /// The key file on this machine.
        key: PathBuf,
    },
    /// One AI CLI's sign-in.
    SignIn {
        /// Which CLI.
        cli: AiCli,
    },
    /// A registry `auths` file for pulling a private image.
    RegistryPull {
        /// The docker-config JSON on this machine.
        auths: PathBuf,
    },
}
